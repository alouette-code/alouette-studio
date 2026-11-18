use crate::commands::files::open_vm_window;
use crate::commands::vm::resolve_qemu_binaries;
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use core_engine::vm_engine::{AdvancedConfig, VmConfig, VmInputAction, VmManager};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::oneshot;

#[derive(Clone)]
struct McpState {
    vm_manager: Arc<VmManager>,
    app_handle: Option<tauri::AppHandle>,
    auth_token: String,
    session_id: String,
}

pub struct VmMcpServerHandle {
    pub url: String,
    pub auth_token: String,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Drop for VmMcpServerHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Start the embedded Alouette VM MCP server (Streamable HTTP) on localhost.
pub async fn start_vm_mcp_server(
    vm_manager: Arc<VmManager>,
    app_handle: Option<tauri::AppHandle>,
) -> Result<VmMcpServerHandle, String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind Alouette VM MCP listener: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to read MCP port: {}", e))?
        .port();

    let auth_token = uuid::Uuid::new_v4().to_string();
    let session_id = uuid::Uuid::new_v4().to_string();
    let state = McpState {
        vm_manager,
        app_handle,
        auth_token: auth_token.clone(),
        session_id,
    };

    let app = Router::new()
        .route(
            "/mcp",
            post(handle_mcp_post)
                .get(|| async { StatusCode::METHOD_NOT_ALLOWED })
                .delete(|| async { StatusCode::NO_CONTENT }),
        )
        .with_state(state);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    Ok(VmMcpServerHandle {
        url: format!("http://127.0.0.1:{}/mcp", port),
        auth_token,
        shutdown_tx: Some(shutdown_tx),
    })
}

fn is_authorized(headers: &HeaderMap, expected_token: &str) -> bool {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|tok| tok.trim() == expected_token)
        .unwrap_or(false)
}

async fn handle_mcp_post(
    State(state): State<McpState>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> Response {
    if !is_authorized(&headers, &state.auth_token) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

    // JSON-RPC notifications have no `id`
    if id.is_none() || method.starts_with("notifications/") {
        return StatusCode::ACCEPTED.into_response();
    }

    let req_id = id.unwrap();
    let response_body = match method {
        "initialize" => {
            let protocol = params
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or("2024-11-05");
            json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": protocol,
                    "capabilities": {
                        "tools": { "listChanged": false }
                    },
                    "serverInfo": {
                        "name": "alouette-vm-mcp",
                        "version": "1.0.0"
                    }
                }
            })
        }
        "ping" => json!({ "jsonrpc": "2.0", "id": req_id, "result": {} }),
        "resources/list" => json!({ "jsonrpc": "2.0", "id": req_id, "result": { "resources": [] } }),
        "prompts/list" => json!({ "jsonrpc": "2.0", "id": req_id, "result": { "prompts": [] } }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": { "tools": mcp_tool_definitions() }
        }),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let result = execute_mcp_tool(&state, name, args).await;
            json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": result
            })
        }
        other => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {}", other)
            }
        }),
    };

    let mut resp = Json(response_body).into_response();
    if let Ok(val) = HeaderValue::from_str(&state.session_id) {
        resp.headers_mut().insert("Mcp-Session-Id", val);
    }
    resp
}

fn text_result(text: String, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error
    })
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Missing required '{}' argument", key))
}

async fn execute_mcp_tool(state: &McpState, name: &str, args: Value) -> Value {
    let vm_manager = state.vm_manager.clone();
    match name {
        "vm_list" => {
            let res = tokio::task::spawn_blocking(move || vm_manager.list_vms_with_runtime())
                .await
                .map_err(|e| format!("Task join error: {}", e))
                .and_then(|r| r);
            match res {
                Ok(vms) => text_result(
                    serde_json::to_string_pretty(&vms).unwrap_or_else(|_| "[]".to_string()),
                    false,
                ),
                Err(e) => text_result(e, true),
            }
        }
        "vm_create" => {
            let vm_name = match require_str(&args, "name") {
                Ok(n) => n.to_string(),
                Err(e) => return text_result(e, true),
            };
            let cpu_cores = args.get("cpu_cores").and_then(|v| v.as_u64()).unwrap_or(2) as u32;
            let ram_size_mb = args.get("ram_size_mb").and_then(|v| v.as_u64()).unwrap_or(2048);
            let disk_size_gb = args
                .get("disk_size_gb")
                .and_then(|v| v.as_u64())
                .map(|g| g as u32)
                .or(Some(20));
            let os_type = args.get("os_type").and_then(|v| v.as_str()).map(ToString::to_string);
            let arch = args
                .get("arch")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
                .or_else(|| Some("x86_64".to_string()));
            let iso_path = args
                .get("iso_path")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string);
            let disk_path = args
                .get("disk_path")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string);
            let network_mode = args
                .get("network_mode")
                .and_then(|v| v.as_str())
                .unwrap_or("nat")
                .to_string();
            let firmware = args
                .get("firmware")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
                .or_else(|| Some("bios".to_string()));
            let host_port_forwards: Vec<(u16, u16)> = args
                .get("host_port_forwards")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|pair| {
                            let p = pair.as_array()?;
                            if p.len() == 2 {
                                Some((p[0].as_u64()? as u16, p[1].as_u64()? as u16))
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            let mut advanced = AdvancedConfig::default();
            advanced.host_port_forwards = host_port_forwards;

            let cfg = VmConfig {
                id: String::new(),
                name: vm_name,
                os_type,
                arch,
                cpu_cores,
                ram_size_mb,
                vm_dir: String::new(),
                iso_path,
                disk_path,
                disk_size_gb,
                network_mode,
                firmware,
                advanced,
            };

            let (_, qemu_img_path) = state
                .app_handle
                .as_ref()
                .map(resolve_qemu_binaries)
                .unwrap_or((None, None));

            let res = tokio::task::spawn_blocking(move || {
                vm_manager.save_vm(cfg, qemu_img_path.as_deref())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);

            match res {
                Ok(created) => text_result(
                    serde_json::to_string_pretty(&created).unwrap_or_default(),
                    false,
                ),
                Err(e) => text_result(e, true),
            }
        }
        "vm_start" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let open_window = args
                .get("open_window")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let (qemu_path, qemu_img_path) = state
                .app_handle
                .as_ref()
                .map(resolve_qemu_binaries)
                .unwrap_or((None, None));

            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                let start_res =
                    vm_manager.start_vm(&vm_id, qemu_path.as_deref(), qemu_img_path.as_deref());
                let already_running = match &start_res {
                    Err(e) if e.contains("already running") || e.contains("Failed to get \"write\" lock") => true,
                    Err(e) => return Err(e.clone()),
                    Ok(_) => false,
                };
                let info = vm_manager
                    .list_vms_with_runtime()?
                    .into_iter()
                    .find(|v| v.config.id == vm_id);
                Ok(json!({
                    "vm_id": vm_id,
                    "already_running": already_running,
                    "status": "running",
                    "runtime": info
                }))
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);

            match res {
                Ok(payload) => {
                    if open_window {
                        if let Some(ref handle) = state.app_handle {
                            let _ = open_vm_window(handle.clone()).await;
                        }
                    }
                    text_result(serde_json::to_string_pretty(&payload).unwrap_or_default(), false)
                }
                Err(e) => text_result(e, true),
            }
        }
        "vm_stop" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                vm_manager.stop_vm(&vm_id)?;
                Ok(format!("Force stopped VM '{}'.", vm_id))
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);
            match res {
                Ok(msg) => text_result(msg, false),
                Err(e) => text_result(e, true),
            }
        }
        "vm_shutdown" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                vm_manager.shutdown_vm(&vm_id)?;
                Ok(format!("Sent ACPI system_powerdown signal via QMP to VM '{}'.", vm_id))
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);
            match res {
                Ok(msg) => text_result(msg, false),
                Err(e) => text_result(e, true),
            }
        }
        "vm_logs" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                vm_manager.get_vm_logs(&vm_id)
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);
            match res {
                Ok(logs) => text_result(logs, false),
                Err(e) => text_result(e, true),
            }
        }
        "vm_execute_command" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let command = match require_str(&args, "command") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let cmd_args: Vec<String> = args
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(ToString::to_string))
                        .collect()
                })
                .unwrap_or_default();

            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                vm_manager.exec_guest_command(&vm_id, &command, &cmd_args, 12, 45)
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);
            match res {
                Ok(out) => text_result(out, false),
                Err(e) => text_result(e, true),
            }
        }
        "vm_file_io" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let operation = match require_str(&args, "operation") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let path = match require_str(&args, "path") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let content = args.get("content").and_then(|v| v.as_str()).map(ToString::to_string);
            let max_bytes = args
                .get("max_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(131_072)
                .clamp(1, 2_097_152) as usize;

            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                match operation.as_str() {
                    "read" => vm_manager.read_guest_file(&vm_id, &path, max_bytes),
                    "write" => {
                        let text = content
                            .ok_or_else(|| "Missing 'content' for write operation".to_string())?;
                        let written = vm_manager.write_guest_file(&vm_id, &path, &text)?;
                        Ok(format!("Wrote {} bytes to '{}' inside VM '{}'.", written, path, vm_id))
                    }
                    other => Err(format!(
                        "Unsupported operation '{}'. Use 'read' or 'write'.",
                        other
                    )),
                }
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);
            match res {
                Ok(out) => text_result(out, false),
                Err(e) => text_result(e, true),
            }
        }
        "vm_inject_file" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let host_path = match require_str(&args, "host_path") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let guest_path = match require_str(&args, "guest_path") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };

            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                vm_manager.inject_file(&vm_id, &host_path, &guest_path)?;
                Ok(format!(
                    "Injected '{}' into guest '{}' on VM '{}'.",
                    host_path, guest_path, vm_id
                ))
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);
            match res {
                Ok(out) => text_result(out, false),
                Err(e) => text_result(e, true),
            }
        }
        "vm_checkpoint" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let action = match require_str(&args, "action") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let snap_name = args.get("name").and_then(|v| v.as_str()).map(ToString::to_string);

            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                match action.as_str() {
                    "save" => {
                        let snap = snap_name
                            .ok_or_else(|| "Missing 'name' for checkpoint save".to_string())?;
                        vm_manager.create_snapshot(&vm_id, &snap)?;
                        Ok(format!("Saved checkpoint '{}' for VM '{}'.", snap, vm_id))
                    }
                    "restore" => {
                        let snap = snap_name
                            .ok_or_else(|| "Missing 'name' for checkpoint restore".to_string())?;
                        vm_manager.restore_snapshot(&vm_id, &snap)?;
                        Ok(format!("Restored checkpoint '{}' for VM '{}'.", snap, vm_id))
                    }
                    "delete" => {
                        let snap = snap_name
                            .ok_or_else(|| "Missing 'name' for checkpoint delete".to_string())?;
                        vm_manager.delete_snapshot(&vm_id, &snap)?;
                        Ok(format!("Deleted checkpoint '{}' for VM '{}'.", snap, vm_id))
                    }
                    "list" => {
                        let list = vm_manager.list_snapshots(&vm_id)?;
                        Ok(serde_json::to_string_pretty(&list).unwrap_or_else(|_| "[]".to_string()))
                    }
                    other => Err(format!(
                        "Unsupported action '{}'. Use 'save', 'restore', 'delete', or 'list'.",
                        other
                    )),
                }
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);
            match res {
                Ok(out) => text_result(out, false),
                Err(e) => text_result(e, true),
            }
        }
        "vm_observe" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let since_seq = args.get("since_seq").and_then(|v| v.as_u64());
            let wait_change_ms = args
                .get("wait_change_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(300);
            let include_ocr = args
                .get("include_ocr")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let include_image_base64 = args
                .get("include_image_base64")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let wait_idle_ms = args.get("wait_idle_ms").and_then(|v| v.as_u64());
            let max_image_dim = args
                .get("max_image_dim")
                .and_then(|v| v.as_u64())
                .unwrap_or(1024) as u32;

            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                if let Some(idle_ms) = wait_idle_ms {
                    let stream = vm_manager.get_vnc_stream(&vm_id)?;
                    stream.wait_for_idle(idle_ms, wait_change_ms.max(idle_ms).max(1500));
                }
                vm_manager.observe_vm(
                    &vm_id,
                    since_seq,
                    if wait_idle_ms.is_some() { 0 } else { wait_change_ms },
                    include_ocr,
                    include_image_base64,
                    max_image_dim,
                )
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);

            match res {
                Ok(mut obs) => {
                    let image_b64 = obs.image_base64_png.take();
                    let json_text = serde_json::to_string_pretty(&obs).unwrap_or_default();
                    let mut content = vec![json!({ "type": "text", "text": json_text })];
                    if let Some(b64) = image_b64 {
                        content.push(json!({
                            "type": "image",
                            "data": b64,
                            "mimeType": "image/png"
                        }));
                    }
                    json!({ "content": content, "isError": false })
                }
                Err(e) => text_result(e, true),
            }
        }
        "vm_act" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let actions_val = match args.get("actions") {
                Some(v) => v.clone(),
                None => return text_result("Missing 'actions' array".to_string(), true),
            };
            let actions: Vec<VmInputAction> = match serde_json::from_value(actions_val) {
                Ok(a) => a,
                Err(e) => return text_result(format!("Invalid actions payload: {}", e), true),
            };
            if actions.is_empty() {
                return text_result("'actions' array cannot be empty".to_string(), true);
            }
            let wait_after_ms = args
                .get("wait_after_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(350);

            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                let (count, (w, h), pre_seq) = vm_manager.act_on_vm(&vm_id, &actions)?;
                let post_obs =
                    vm_manager.observe_vm(&vm_id, Some(pre_seq), wait_after_ms, true, false, 1024)?;
                Ok(json!({
                    "vm_id": vm_id,
                    "executed_actions": count,
                    "screen_resolution": [w, h],
                    "post_observation": post_obs
                }))
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);

            match res {
                Ok(payload) => {
                    text_result(serde_json::to_string_pretty(&payload).unwrap_or_default(), false)
                }
                Err(e) => text_result(e, true),
            }
        }
        "vm_fast_dispatch" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let instruction = match require_str(&args, "instruction") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let endpoint = args.get("endpoint").and_then(|v| v.as_str()).map(ToString::to_string);
            let model = args.get("model").and_then(|v| v.as_str()).map(ToString::to_string);

            let vm_mgr_obs = vm_manager.clone();
            let id_obs = id_or_name.clone();
            let obs_res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_mgr_obs.resolve_vm_id(&id_obs)?;
                let obs = vm_mgr_obs.observe_vm(&vm_id, None, 250, true, false, 1024)?;
                Ok::<_, String>((vm_id, obs))
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);

            let (vm_id, obs) = match obs_res {
                Ok(v) => v,
                Err(e) => return text_result(e, true),
            };

            let compiled = match core_engine::vm_engine::action_compiler::compile_instruction_with_small_model(
                &instruction,
                &obs,
                endpoint.as_deref(),
                model.as_deref(),
            )
            .await
            {
                Ok(c) => c,
                Err(e) => return text_result(e, true),
            };

            let actions_clone = compiled.actions.clone();
            let post_res = tokio::task::spawn_blocking(move || {
                let (count, (w, h), pre_seq) = vm_manager.act_on_vm(&vm_id, &actions_clone)?;
                let post_obs =
                    vm_manager.observe_vm(&vm_id, Some(pre_seq), 400, true, false, 1024)?;
                Ok::<_, String>((count, w, h, post_obs))
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);

            match post_res {
                Ok((count, w, h, post_obs)) => text_result(
                    serde_json::to_string_pretty(&json!({
                        "model_used": compiled.model_used,
                        "endpoint_used": compiled.endpoint_used,
                        "compiled_actions": compiled.actions,
                        "executed_actions": count,
                        "screen_resolution": [w, h],
                        "post_observation": post_obs
                    }))
                    .unwrap_or_default(),
                    false,
                ),
                Err(e) => text_result(e, true),
            }
        }
        "vm_qmp" => {
            let id_or_name = match require_str(&args, "vm_id") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let command = match require_str(&args, "command") {
                Ok(v) => v.to_string(),
                Err(e) => return text_result(e, true),
            };
            let qmp_args = args.get("arguments").cloned();

            let res = tokio::task::spawn_blocking(move || {
                let vm_id = vm_manager.resolve_vm_id(&id_or_name)?;
                vm_manager.execute_qmp_command(&vm_id, &command, qmp_args)
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))
            .and_then(|r| r);
            match res {
                Ok(val) => {
                    text_result(serde_json::to_string_pretty(&val).unwrap_or_default(), false)
                }
                Err(e) => text_result(e, true),
            }
        }
        "vm_open_window" => {
            if let Some(ref handle) = state.app_handle {
                match open_vm_window(handle.clone()).await {
                    Ok(_) => text_result("Opened Alouette VM Manager window.".to_string(), false),
                    Err(e) => text_result(e, true),
                }
            } else {
                text_result("AppHandle not available to open VM window.".to_string(), true)
            }
        }
        other => text_result(format!("Unknown Alouette VM tool '{}'", other), true),
    }
}

fn mcp_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "vm_list",
            "description": "List all virtual machines managed by Alouette Studio with their IDs, names, status (running/stopped), VNC/WebSocket ports, and hardware config.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "vm_create",
            "description": "Create and register a new QEMU virtual machine inside Alouette Studio (stored in app_data/vms with an auto-created qcow2 disk if disk_path is omitted).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Human-readable VM name (e.g. 'Alpine-Dev')." },
                    "os_type": { "type": "string", "description": "Optional OS hint (e.g. 'linux', 'alpine', 'ubuntu', 'windows')." },
                    "arch": { "type": "string", "enum": ["x86_64", "aarch64"], "description": "CPU architecture (default 'x86_64')." },
                    "cpu_cores": { "type": "integer", "description": "CPU core count (default 2)." },
                    "ram_size_mb": { "type": "integer", "description": "Memory in MB (default 2048)." },
                    "disk_size_gb": { "type": "integer", "description": "Disk size in GB when creating a new qcow2 disk (default 20)." },
                    "iso_path": { "type": "string", "description": "Optional path to installer/live ISO file." },
                    "disk_path": { "type": "string", "description": "Optional path to an existing qcow2/raw disk image." },
                    "network_mode": { "type": "string", "enum": ["nat", "bridge", "host-only"], "description": "Network mode (default 'nat')." },
                    "firmware": { "type": "string", "enum": ["bios", "uefi"], "description": "Firmware type (default 'bios')." },
                    "host_port_forwards": {
                        "type": "array",
                        "description": "Optional NAT port forwards as [[host_port, guest_port], ...].",
                        "items": { "type": "array", "items": { "type": "integer" } }
                    }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "vm_start",
            "description": "Start an Alouette-managed VM (with integrated VNC, WebSocket, QMP, QGA, and serial console) and optionally open the Alouette VM Manager window.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name from vm_list." },
                    "open_window": { "type": "boolean", "description": "Open/focus the built-in Alouette VM Manager window (default true)." }
                },
                "required": ["vm_id"]
            }
        }),
        json!({
            "name": "vm_stop",
            "description": "Force-stop (kill) a running Alouette VM.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name." }
                },
                "required": ["vm_id"]
            }
        }),
        json!({
            "name": "vm_shutdown",
            "description": "Gracefully shut down a running Alouette VM via ACPI QMP system_powerdown.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name." }
                },
                "required": ["vm_id"]
            }
        }),
        json!({
            "name": "vm_logs",
            "description": "Read runtime VNC/WebSocket endpoints, COM1 serial console output, and QEMU error logs for an Alouette VM.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name." }
                },
                "required": ["vm_id"]
            }
        }),
        json!({
            "name": "vm_execute_command",
            "description": "Tier-1 deep guest control: execute a command directly inside the running VM via QEMU Guest Agent (QGA guest-exec). If QGA is unavailable, use Tier-2 tools (vm_observe + vm_act + vm_logs).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name." },
                    "command": { "type": "string", "description": "Executable or shell binary path inside the guest (e.g. '/bin/sh')." },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Command arguments (e.g. ['-c', 'uname -a'])." }
                },
                "required": ["vm_id", "command"]
            }
        }),
        json!({
            "name": "vm_file_io",
            "description": "Tier-1 deep guest control: read or write files directly inside the running VM via QEMU Guest Agent.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name." },
                    "operation": { "type": "string", "enum": ["read", "write"] },
                    "path": { "type": "string", "description": "Absolute path inside the guest OS." },
                    "content": { "type": "string", "description": "Text content for 'write'." },
                    "max_bytes": { "type": "integer", "description": "Max bytes for 'read' (default 131072, max 2097152)." }
                },
                "required": ["vm_id", "operation", "path"]
            }
        }),
        json!({
            "name": "vm_inject_file",
            "description": "Inject a file (up to 10MB, text or binary) from the host filesystem directly into the running guest via QGA.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name." },
                    "host_path": { "type": "string", "description": "Source file path on the host." },
                    "guest_path": { "type": "string", "description": "Destination path inside the guest OS." }
                },
                "required": ["vm_id", "host_path", "guest_path"]
            }
        }),
        json!({
            "name": "vm_checkpoint",
            "description": "Manage VM snapshots ('save', 'restore', 'delete', 'list') using live QMP savevm/loadvm or offline qemu-img.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name." },
                    "action": { "type": "string", "enum": ["save", "restore", "delete", "list"] },
                    "name": { "type": "string", "description": "Snapshot name (required for save/restore/delete)." }
                },
                "required": ["vm_id", "action"]
            }
        }),
        json!({
            "name": "vm_observe",
            "description": "Tier-2 deep display control: inspect the live in-memory VNC framebuffer of a running VM (zero disk overhead), returning resolution, frame_seq, dirty regions, OCR text bounding boxes (cx, cy), and optional PNG screenshot.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name." },
                    "since_seq": { "type": "integer", "description": "Previous frame_seq to wait for visual diffs." },
                    "wait_change_ms": { "type": "integer", "description": "Ms to wait for screen change (default 300)." },
                    "wait_idle_ms": { "type": "integer", "description": "Optional ms to wait until screen stops changing (debounce) before capturing OCR/image." },
                    "include_ocr": { "type": "boolean", "description": "Extract clickable OCR text coordinates (default true)." },
                    "include_image_base64": { "type": "boolean", "description": "Attach PNG image of the VM display (default false)." },
                    "max_image_dim": { "type": "integer", "description": "Max width/height for PNG (default 1024)." }
                },
                "required": ["vm_id"]
            }
        }),
        json!({
            "name": "vm_act",
            "description": "Tier-2 deep input control: inject isolated mouse/keyboard actions directly into the VM via QMP input-send-event + virtio-tablet-pci (does not touch host mouse/keyboard), with deterministic OCR text targeting (`click_text`, `move_text`) and frame-aware waits (`wait_for_text`, `wait_for_idle`).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID, VM name, unique prefix, or 'active'." },
                    "actions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["move", "click", "click_text", "move_text", "scroll", "key", "type", "wait", "wait_for_text", "wait_for_idle"] },
                                "x": { "type": "integer" },
                                "y": { "type": "integer" },
                                "button": { "type": "string", "enum": ["left", "right", "middle"] },
                                "double": { "type": "boolean" },
                                "occurrence": { "type": "integer", "description": "0-based match index for click_text/move_text." },
                                "direction": { "type": "string", "enum": ["up", "down"] },
                                "amount": { "type": "integer" },
                                "key": { "type": "string", "description": "e.g. 'ret', 'esc', 'tab', 'ctrl+c', 'alt+tab', 'super', 'f1'..'f12'" },
                                "text": { "type": "string", "description": "Text to type, OCR target for click_text/move_text, or expected text for wait_for_text." },
                                "ms": { "type": "integer" },
                                "idle_ms": { "type": "integer", "description": "Idle duration in ms for wait_for_idle (default 250)." },
                                "timeout_ms": { "type": "integer", "description": "Max wait ms for wait_for_text/wait_for_idle (default 5000)." }
                            },
                            "required": ["type"]
                        }
                    },
                    "wait_after_ms": { "type": "integer", "description": "Ms to wait before capturing post_observation (default 350)." }
                },
                "required": ["vm_id", "actions"]
            }
        }),
        json!({
            "name": "vm_fast_dispatch",
            "description": "Tier-2 accelerated UI action compiler: delegates a natural-language UI instruction + live VNC OCR elements to a local small model (Ollama/llama-server) which compiles and executes QMP input actions inside the VM.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name." },
                    "instruction": { "type": "string", "description": "Natural language UI step (e.g. 'Type root, press Enter, then run uname -a')." },
                    "endpoint": { "type": "string", "description": "Optional local OpenAI-compatible URL." },
                    "model": { "type": "string", "description": "Optional local small model name." }
                },
                "required": ["vm_id", "instruction"]
            }
        }),
        json!({
            "name": "vm_qmp",
            "description": "Low-level QEMU Machine Protocol (QMP) command execution directly on the VM's QMP socket (e.g. 'query-status', 'query-block', 'system_reset', 'stop', 'cont', 'human-monitor-command').",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vm_id": { "type": "string", "description": "VM ID or exact VM name." },
                    "command": { "type": "string", "description": "QMP command name." },
                    "arguments": { "type": "object", "description": "Optional QMP arguments object." }
                },
                "required": ["vm_id", "command"]
            }
        }),
        json!({
            "name": "vm_open_window",
            "description": "Open or focus the built-in Alouette VM Manager window (showing VM list and live VNC display).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vm_mcp_streamable_http_lifecycle() {
        let tmp_dir = std::env::temp_dir().join(format!("alouette_vm_mcp_test_{}", uuid::Uuid::new_v4()));
        let vm_mgr = Arc::new(VmManager::new(&tmp_dir));
        let srv = start_vm_mcp_server(vm_mgr, None).await.expect("start MCP server");
        let client = reqwest::Client::new();

        // 1. Unauthorized request must return 401
        let unauth = client
            .post(&srv.url)
            .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }))
            .send()
            .await
            .expect("send unauth");
        assert_eq!(unauth.status(), reqwest::StatusCode::UNAUTHORIZED);

        let auth = format!("Bearer {}", srv.auth_token);

        // 2. Initialize returns protocolVersion + Mcp-Session-Id header
        let init_resp = client
            .post(&srv.url)
            .header("Authorization", &auth)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": { "protocolVersion": "2025-03-26" }
            }))
            .send()
            .await
            .expect("send initialize");
        assert_eq!(init_resp.status(), reqwest::StatusCode::OK);
        assert!(init_resp.headers().contains_key("Mcp-Session-Id"));
        let init_json: Value = init_resp.json().await.expect("parse initialize");
        assert_eq!(init_json["result"]["serverInfo"]["name"], "alouette-vm-mcp");

        // 3. tools/list returns all 15 VM tools including deep-level controls
        let list_resp: Value = client
            .post(&srv.url)
            .header("Authorization", &auth)
            .json(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }))
            .send()
            .await
            .expect("send tools/list")
            .json()
            .await
            .expect("parse tools/list");
        let tools = list_resp["result"]["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), 15);
        assert!(tools.iter().any(|t| t["name"] == "vm_start"));
        assert!(tools.iter().any(|t| t["name"] == "vm_observe"));
        assert!(tools.iter().any(|t| t["name"] == "vm_act"));
        assert!(tools.iter().any(|t| t["name"] == "vm_qmp"));

        // 4. tools/call vm_list succeeds
        let call_resp: Value = client
            .post(&srv.url)
            .header("Authorization", &auth)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": { "name": "vm_list", "arguments": {} }
            }))
            .send()
            .await
            .expect("send tools/call")
            .json()
            .await
            .expect("parse tools/call");
        assert_eq!(call_resp["result"]["isError"], false);
        assert_eq!(call_resp["result"]["content"][0]["text"], "[]");

        drop(srv);
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
