use core_engine::extension_manager::manifest::ExtensionManifest;
use core_engine::extension_manager::manager::ExtensionRegistry;
use std::path::PathBuf;

fn get_extensions_dir() -> PathBuf {
    let mut ext_dir = PathBuf::new();
    if let Ok(home) = std::env::var("HOME") {
        ext_dir.push(home);
    } else if let Ok(user_profile) = std::env::var("USERPROFILE") {
        ext_dir.push(user_profile);
    } else {
        ext_dir.push(".");
    }
    ext_dir.push(".alouette");
    ext_dir.push("extensions");
    ext_dir
}

#[tauri::command]
pub async fn get_installed_extensions() -> Result<Vec<ExtensionManifest>, String> {
    let ext_dir = get_extensions_dir();
    let registry = ExtensionRegistry::new(ext_dir);
    Ok(registry.scan_extensions())
}

#[tauri::command]
pub async fn get_extension_details(id: String) -> Result<Option<ExtensionManifest>, String> {
    let extensions = get_installed_extensions().await?;
    Ok(extensions.into_iter().find(|ext| ext.id == id))
}

#[tauri::command]
pub async fn execute_wasm_extension(
    id: String,
    function_name: String,
    param_json: String,
) -> Result<String, String> {
    let ext_dir = get_extensions_dir();
    let registry = ExtensionRegistry::new(ext_dir);
    
    registry.run_wasm_extension(&id, &function_name, &param_json)
        .await
        .map_err(|e| e.to_string())
}
