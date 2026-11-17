use core_engine::docker_engine::client::DockerClient;
use core_engine::docker_engine::container::{
    create_container, list_containers, remove_container, restart_container, start_container,
    stop_container, DockerContainerConfig,
};
use core_engine::docker_engine::logs::stream_container_logs;
use core_engine::docker_engine::stats::stream_container_stats;
use core_engine::docker_engine::terminal::{
    close_terminal, resize_terminal, spawn_terminal, write_to_terminal, TerminalOutput,
};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::AbortHandle;
use lazy_static::lazy_static;

lazy_static! {
    static ref DOCKER_CLIENT: Arc<Mutex<Option<DockerClient>>> = Arc::new(Mutex::new(None));
    static ref LOGS_STREAM_HANDLES: Mutex<Vec<AbortHandle>> = Mutex::new(Vec::new());
    static ref STATS_STREAM_HANDLES: Mutex<Vec<AbortHandle>> = Mutex::new(Vec::new());
}

async fn replace_stream_handles(store: &Mutex<Vec<AbortHandle>>, new_handles: Vec<AbortHandle>) {
    let mut guard = store.lock().await;
    for handle in guard.drain(..) {
        handle.abort();
    }
    *guard = new_handles;
}

async fn get_client() -> Result<DockerClient, String> {
    let mut lock = DOCKER_CLIENT.lock().await;
    if let Some(client) = lock.as_ref() {
        return Ok(DockerClient { docker: client.docker.clone() });
    }

    let new_client = DockerClient::new()?;
    *lock = Some(DockerClient { docker: new_client.docker.clone() });
    Ok(new_client)
}

#[tauri::command]
pub async fn docker_ensure_started() -> Result<(), String> {
    let client = get_client().await?;
    client.ensure_started().await
}

#[tauri::command]
pub async fn docker_list_containers(all: bool) -> Result<Vec<bollard::models::ContainerSummary>, String> {
    let client = get_client().await?;
    list_containers(&client.docker, all).await
}

#[tauri::command]
pub async fn docker_list_images() -> Result<Vec<String>, String> {
    let client = get_client().await?;
    let images = client.docker.list_images(Some(bollard::image::ListImagesOptions::<String> {
        all: true,
        ..Default::default()
    })).await.map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for img in images {
        for tag in img.repo_tags {
            if tag != "<none>:<none>" {
                result.push(tag);
            }
        }
    }
    
    result.sort();
    result.dedup();
    Ok(result)
}

#[tauri::command]
pub async fn docker_create_container(config: DockerContainerConfig) -> Result<String, String> {
    let client = get_client().await?;
    create_container(&client.docker, config).await
}

#[tauri::command]
pub async fn docker_start_container(id: String) -> Result<(), String> {
    let client = get_client().await?;
    start_container(&client.docker, &id).await
}

#[tauri::command]
pub async fn docker_stop_container(id: String) -> Result<(), String> {
    let client = get_client().await?;
    stop_container(&client.docker, &id).await
}

#[tauri::command]
pub async fn docker_remove_container(id: String, force: bool) -> Result<(), String> {
    let client = get_client().await?;
    remove_container(&client.docker, &id, force).await
}

#[tauri::command]
pub async fn docker_restart_container(id: String) -> Result<(), String> {
    let client = get_client().await?;
    restart_container(&client.docker, &id).await
}

#[derive(serde::Serialize, Clone)]
struct LogPayload {
    id: String,
    message: String,
}

#[tauri::command]
pub async fn docker_stream_logs(app_handle: AppHandle, id: String, since: Option<i64>) -> Result<(), String> {
    let client = get_client().await?;
    let (tx, mut rx) = mpsc::channel(100);

    let container_id = id.clone();
    let emit_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let _ = app_handle.emit(
                "docker_log",
                LogPayload {
                    id: container_id.clone(),
                    message: msg,
                },
            );
        }
    });

    let stream_handle = tokio::spawn(async move {
        let _ = stream_container_logs(client.docker, id, tx, since.unwrap_or(0)).await;
    });

    replace_stream_handles(
        &LOGS_STREAM_HANDLES,
        vec![emit_handle.abort_handle(), stream_handle.abort_handle()],
    )
    .await;

    Ok(())
}

#[tauri::command]
pub async fn docker_stop_logs_stream() -> Result<(), String> {
    replace_stream_handles(&LOGS_STREAM_HANDLES, Vec::new()).await;
    Ok(())
}

#[derive(serde::Serialize, Clone)]
struct StatsPayload {
    id: String,
    stats: core_engine::docker_engine::stats::ContainerStats,
}

#[tauri::command]
pub async fn docker_stream_stats(app_handle: AppHandle, id: String) -> Result<(), String> {
    let client = get_client().await?;
    let (tx, mut rx) = mpsc::channel(100);

    let container_id = id.clone();
    let emit_handle = tokio::spawn(async move {
        while let Some(stats) = rx.recv().await {
            let _ = app_handle.emit(
                "docker_stats",
                StatsPayload {
                    id: container_id.clone(),
                    stats,
                },
            );
        }
    });

    let stream_handle = tokio::spawn(async move {
        let _ = stream_container_stats(client.docker, id, tx).await;
    });

    replace_stream_handles(
        &STATS_STREAM_HANDLES,
        vec![emit_handle.abort_handle(), stream_handle.abort_handle()],
    )
    .await;

    Ok(())
}

#[tauri::command]
pub async fn docker_stop_stats_stream() -> Result<(), String> {
    replace_stream_handles(&STATS_STREAM_HANDLES, Vec::new()).await;
    Ok(())
}

#[tauri::command]
pub async fn docker_exec_terminal(app_handle: AppHandle, id: String) -> Result<String, String> {
    let client = get_client().await?;

    let exec_id = spawn_terminal(&client.docker, &id, move |output: TerminalOutput| {
        let _ = app_handle.emit("docker_terminal_out", output);
    })
    .await?;

    Ok(exec_id)
}

#[tauri::command]
pub async fn docker_write_terminal(exec_id: String, data: String) -> Result<(), String> {
    write_to_terminal(&exec_id, data).await
}

#[tauri::command]
pub async fn docker_resize_terminal(exec_id: String, cols: u16, rows: u16) -> Result<(), String> {
    let client = get_client().await?;
    resize_terminal(&client.docker, &exec_id, cols, rows).await
}

#[tauri::command]
pub async fn docker_close_terminal(exec_id: String) -> Result<(), String> {
    close_terminal(&exec_id).await;
    Ok(())
}

#[tauri::command]
pub async fn docker_cleanup() -> Result<String, String> {
    let output = tokio::process::Command::new("docker")
        .args(["system", "prune", "-a", "-f"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
