use crate::state::AppState;
use core_engine::vm_engine::{VmConfig, VmRuntimeInfo};
use tauri::{Manager, State};

pub(crate) fn resolve_qemu_binaries(app_handle: &tauri::AppHandle) -> (Option<String>, Option<String>) {
    let qemu_path = app_handle
        .path()
        .resolve("bin/qemu-system-x86_64", tauri::path::BaseDirectory::Resource)
        .ok()
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().into_owned());

    let qemu_img_path = app_handle
        .path()
        .resolve("bin/qemu-img", tauri::path::BaseDirectory::Resource)
        .ok()
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().into_owned());

    (qemu_path, qemu_img_path)
}

#[tauri::command]
pub async fn save_virtual_machine(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    config: VmConfig,
) -> Result<VmConfig, String> {
    let vm_manager = state.vm_manager.clone();
    let (_, qemu_img_path) = resolve_qemu_binaries(&app_handle);
    tokio::task::spawn_blocking(move || vm_manager.save_vm(config, qemu_img_path.as_deref()))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn delete_virtual_machine(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let vm_manager = state.vm_manager.clone();
    tokio::task::spawn_blocking(move || vm_manager.delete_vm(&id))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn list_virtual_machines(
    state: State<'_, AppState>,
) -> Result<Vec<VmRuntimeInfo>, String> {
    let vm_manager = state.vm_manager.clone();
    tokio::task::spawn_blocking(move || vm_manager.list_vms_with_runtime())
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn start_virtual_machine(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let vm_manager = state.vm_manager.clone();
    let (qemu_path, qemu_img_path) = resolve_qemu_binaries(&app_handle);
    tokio::task::spawn_blocking(move || {
        vm_manager.start_vm(&id, qemu_path.as_deref(), qemu_img_path.as_deref())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn stop_virtual_machine(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let vm_manager = state.vm_manager.clone();
    tokio::task::spawn_blocking(move || vm_manager.stop_vm(&id))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn shutdown_virtual_machine(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let vm_manager = state.vm_manager.clone();
    tokio::task::spawn_blocking(move || vm_manager.shutdown_vm(&id))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn get_virtual_machine_logs(
    id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let vm_manager = state.vm_manager.clone();
    tokio::task::spawn_blocking(move || vm_manager.get_vm_logs(&id))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn create_vm_snapshot(
    id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let vm_manager = state.vm_manager.clone();
    tokio::task::spawn_blocking(move || vm_manager.create_snapshot(&id, &name))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn restore_vm_snapshot(
    id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let vm_manager = state.vm_manager.clone();
    tokio::task::spawn_blocking(move || vm_manager.restore_snapshot(&id, &name))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn delete_vm_snapshot(
    id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let vm_manager = state.vm_manager.clone();
    tokio::task::spawn_blocking(move || vm_manager.delete_snapshot(&id, &name))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn list_vm_snapshots(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let vm_manager = state.vm_manager.clone();
    tokio::task::spawn_blocking(move || vm_manager.list_snapshots(&id))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn inject_guest_file(
    id: String,
    host_path: String,
    guest_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let vm_manager = state.vm_manager.clone();
    tokio::task::spawn_blocking(move || vm_manager.inject_file(&id, &host_path, &guest_path))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}
