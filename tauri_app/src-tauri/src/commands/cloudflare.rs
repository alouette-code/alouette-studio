use crate::state::app_data_dir;
use std::path::PathBuf;

fn cloudflare_config_path() -> PathBuf {
    app_data_dir().join("cloudflare_config.yml")
}

#[tauri::command]
pub fn load_cloudflare_config() -> Result<String, String> {
    let path = cloudflare_config_path();
    if !path.exists() {
        return Ok(String::new());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read cloudflare config: {}", e))?;
    Ok(content)
}

#[tauri::command]
pub fn save_cloudflare_config(content: String) -> Result<(), String> {
    let path = cloudflare_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create app_data directory: {}", e))?;
    }
    std::fs::write(&path, &content)
        .map_err(|e| format!("Failed to write cloudflare config: {}", e))?;
    Ok(())
}
