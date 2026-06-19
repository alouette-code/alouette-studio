use tauri::{State, Manager};
use crate::state::AppState;
use core_engine::vm_engine::VmConfig;

#[tauri::command]
pub async fn save_virtual_machine(
    state: State<'_, AppState>,
    config: VmConfig,
) -> Result<(), String> {
    state.vm_manager.save_vm(config)
}

#[tauri::command]
pub async fn delete_virtual_machine(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.vm_manager.delete_vm(&id)
}

#[derive(serde::Serialize)]
pub struct VmStatusInfo {
    pub config: VmConfig,
    pub status: String,
}

#[tauri::command]
pub async fn list_virtual_machines(
    state: State<'_, AppState>,
) -> Result<Vec<VmStatusInfo>, String> {
    let list = state.vm_manager.list_vms()?;
    let status_list = list
        .into_iter()
        .map(|(config, status)| VmStatusInfo { config, status })
        .collect();
    Ok(status_list)
}

#[tauri::command]
pub async fn start_virtual_machine(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    // Resolve optional sidecar binaries under packaged resources
    let qemu_path = app_handle.path().resolve("bin/qemu-system-x86_64", tauri::path::BaseDirectory::Resource)
        .map(|p| p.to_string_lossy().into_owned())
        .ok();
        
    let qemu_img_path = app_handle.path().resolve("bin/qemu-img", tauri::path::BaseDirectory::Resource)
        .map(|p| p.to_string_lossy().into_owned())
        .ok();

    let final_qemu = qemu_path.as_ref().filter(|p| std::path::Path::new(p).exists()).map(|s| s.as_str());
    let final_qemu_img = qemu_img_path.as_ref().filter(|p| std::path::Path::new(p).exists()).map(|s| s.as_str());

    state.vm_manager.start_vm(&id, final_qemu, final_qemu_img)
}

#[tauri::command]
pub async fn stop_virtual_machine(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.vm_manager.stop_vm(&id)
}

#[tauri::command]
pub async fn get_virtual_machine_logs(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    state.vm_manager.get_vm_logs(&id)
}
