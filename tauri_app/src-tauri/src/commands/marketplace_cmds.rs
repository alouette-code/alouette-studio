use core_engine::extension_manager::marketplace::{
    RegistryIndexItem, ServerlessMarketplaceClient,
};
use core_engine::extension_manager::manifest::ExtensionManifest;
use std::path::PathBuf;
use std::fs;

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
pub async fn fetch_marketplace_extensions() -> Result<Vec<RegistryIndexItem>, String> {
    let client = ServerlessMarketplaceClient::new();
    client.fetch_registry_index().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_wasm_extension(item: RegistryIndexItem) -> Result<String, String> {
    let client = ServerlessMarketplaceClient::new();
    let target_dir = get_extensions_dir();
    
    client.download_and_install_wasm(&item, &target_dir)
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!("Extension {} (v{}) installed successfully!", item.name, item.version))
}

#[tauri::command]
pub async fn calculate_wasm_sha256(file_path: String) -> Result<String, String> {
    let path = PathBuf::from(file_path);
    ServerlessMarketplaceClient::calculate_file_sha256(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_extension_icon_uuid(file_path: String) -> Result<String, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err("Image file does not exist".to_string());
    }

    // 1. Kiểm tra kích thước độ phân giải tối đa 500x500
    if let Ok((width, height)) = image::image_dimensions(&path) {
        if width > 500 || height > 500 {
            return Err(format!(
                "Image resolution exceeds 500x500 limit! Your selected image is {}x{}px. Please choose an icon up to 500x500 pixels.",
                width, height
            ));
        }
    } else {
        return Err("Failed to inspect image file dimensions. Please select a valid PNG/JPG/WebP/SVG image.".to_string());
    }

    // 2. Sinh mã UUID 36 ký tự duy nhất
    let uuid_str = uuid::Uuid::new_v4().to_string();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_else(|| "png".to_string());

    let icon_filename = format!("{}.{}", uuid_str, ext);
    let cdn_url = format!(
        "https://raw.githubusercontent.com/alouette-code/alouette-extension-registry/main/icons/{}",
        icon_filename
    );

    Ok(cdn_url)
}

#[tauri::command]
pub async fn publish_extension_github(
    folder_path: String,
    _github_token: String,
) -> Result<String, String> {
    let path = PathBuf::from(&folder_path);
    let manifest_path = path.join("proto-extension.json");
    
    if !manifest_path.exists() {
        return Err("proto-extension.json not found in folder".to_string());
    }

    let manifest_content = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: ExtensionManifest = serde_json::from_str(&manifest_content).map_err(|e| e.to_string())?;

    let wasm_file = path.join(manifest.runtime.wasm_entry.as_deref().unwrap_or("plugin.wasm"));
    if !wasm_file.exists() {
        return Err("Wasm binary (plugin.wasm) not found in folder".to_string());
    }

    let sha256 = ServerlessMarketplaceClient::calculate_file_sha256(&wasm_file).map_err(|e| e.to_string())?;

    Ok(format!(
        "Ready to create PR for extension '{}' (v{}). Checksum SHA-256: {}",
        manifest.name, manifest.version, sha256
    ))
}
