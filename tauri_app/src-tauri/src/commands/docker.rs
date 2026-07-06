use core_engine::docker_engine::client::DockerClient;
use core_engine::docker_engine::container::{
    create_container, list_containers, remove_container, restart_container, start_container,
    stop_container, DockerContainerConfig,
};
use core_engine::docker_engine::logs::stream_container_logs;
use core_engine::docker_engine::stats::stream_container_stats;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref DOCKER_CLIENT: Arc<Mutex<Option<DockerClient>>> = Arc::new(Mutex::new(None));
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
pub async fn docker_stream_logs(app_handle: AppHandle, id: String) -> Result<(), String> {
    let client = get_client().await?;
    let (tx, mut rx) = mpsc::channel(100);
    
    let container_id = id.clone();
    tokio::spawn(async move {
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

    tokio::spawn(async move {
        let _ = stream_container_logs(client.docker, id, tx).await;
    });

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
    tokio::spawn(async move {
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

    tokio::spawn(async move {
        let _ = stream_container_stats(client.docker, id, tx).await;
    });

    Ok(())
}
