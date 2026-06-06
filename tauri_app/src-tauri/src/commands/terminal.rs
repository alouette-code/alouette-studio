use crate::state::AppState;
use core_engine::TerminalOutput;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct TerminalSessionInfo {
    pub exists: bool,
    pub pid: Option<u32>,
}

#[tauri::command]
pub async fn spawn_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
    cwd: Option<String>,
    block_internet: Option<bool>,
) -> Result<(), String> {
    let mut pm = state.process_manager.lock().await;
    pm.spawn_terminal(&session_id, cwd.as_deref(), block_internet.unwrap_or(false))
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn sync_terminal_input_buf(
    state: State<'_, AppState>,
    session_id: String,
    current_input: String,
) -> Result<(), String> {
    let sanitized_input = current_input.replace('\u{00a0}', " ");
    let mut pm = state.process_manager.lock().await;
    pm.input_buf.insert(session_id, sanitized_input);
    Ok(())
}

#[tauri::command]
pub async fn write_to_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
    input: String,
) -> Result<(), String> {
    let input = input.replace('\u{00a0}', " ");
    eprintln!("[term-input] Received input: {:?}", input);
    // 1. Immediately handle Ctrl+C to interrupt executing commands safely
    if input.contains('\x03') {
        let mut pm = state.process_manager.lock().await;
        pm.clear_input_buf(&session_id);
        if let Ok(ctx) = pm.get_terminal_write_context(&session_id) {
            let stdin_tx = ctx.stdin_sender.clone();
            drop(pm);
            let _ = stdin_tx.send("\x03".to_string()).await;
        }
        return Ok(());
    }


    let is_enter = input.contains('\r') || input.contains('\n');
    let mut allowed_cmd = String::new();
    let mut blocked_reason = String::new();
    let out_tx;
    let stdin_tx;

    {
        let mut pm = state.process_manager.lock().await;

        if is_enter {
            // Normalize cd shortcuts in the input buffer first
            if let Some(buf) = pm.input_buf.get_mut(&session_id) {
                let trimmed = buf.trim();
                let lower = trimmed.to_lowercase();
                if lower.starts_with("cd..") {
                    *buf = format!("cd ..{}", &trimmed[4..]);
                } else if lower.starts_with("cd/") {
                    *buf = format!("cd /{}", &trimmed[3..]);
                } else if lower.starts_with("cd\\") {
                    *buf = format!("cd \\{}", &trimmed[3..]);
                }
            }

            match pm.check_input_sandbox(&session_id) {
                Ok(None) => {
                    allowed_cmd = pm.get_input_buf(&session_id).cloned().unwrap_or_default();
                    pm.clear_input_buf(&session_id);

                    // Add command to history on successful enter
                    if !allowed_cmd.trim().is_empty() {
                        let len = {
                            let hist = pm.terminal_history.entry(session_id.clone()).or_default();
                            if hist.last() != Some(&allowed_cmd) {
                                hist.push(allowed_cmd.clone());
                            }
                            hist.len()
                        };
                        pm.terminal_history_index.insert(session_id.clone(), len);
                    }
                }
                Ok(Some(reason)) => {
                    blocked_reason = reason;
                    pm.clear_input_buf(&session_id);
                }
                Err(e) => {
                    eprintln!("[sandbox] error: {e}");
                    pm.clear_input_buf(&session_id);
                }
            }
        }

        let ctx = match pm.get_terminal_write_context(&session_id) {
            Ok(c) => c,
            Err(_) => return Ok(()), // Return gracefully if session no longer exists
        };
        out_tx = ctx.terminal_sender.clone();
        stdin_tx = ctx.stdin_sender.clone();
    }

    if !blocked_reason.is_empty() {
        // Send Ctrl+C to PowerShell/Bash to cancel input line
        let _ = stdin_tx.send("\x03".to_string()).await;
        // Send blocked warning
        let warning = format!("\r[Sandbox] Blocked: {}\r\n", blocked_reason);
        let _ = out_tx.send(TerminalOutput {
            session_id: session_id.clone(),
            text: warning,
        });
        return Ok(());
    }

    if is_enter {
        {
            let mut pm = state.process_manager.lock().await;
            if !allowed_cmd.is_empty() {
                pm.update_cwd_for_cd(&session_id, &allowed_cmd);
            }
        }
        let _ = stdin_tx.send("\r".to_string()).await;
        return Ok(());
    }

    // Direct forward to PTY for typing/backspacing
    let _ = stdin_tx.send(input).await;

    Ok(())
}

#[tauri::command]
pub async fn kill_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let mut pm = state.process_manager.lock().await;
    pm.kill_terminal(&session_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn check_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<TerminalSessionInfo, String> {
    let pm = state.process_manager.lock().await;
    if let Some(session) = pm.terminal_sessions.get(&session_id) {
        Ok(TerminalSessionInfo {
            exists: true,
            pid: Some(session.pid),
        })
    } else {
        Ok(TerminalSessionInfo {
            exists: false,
            pid: None,
        })
    }
}

#[tauri::command]
pub async fn resize_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let pm = state.process_manager.lock().await;
    pm.resize_terminal(&session_id, rows, cols)?;
    Ok(())
}
