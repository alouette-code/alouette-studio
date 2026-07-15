use core_engine::extension_manager::marketplace::{
    zip_extension_dir, CreateExtensionRequest, ExtensionData, MarketplaceClient,
};
use core_engine::extension_manager::manifest::ExtensionManifest;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use std::fs;

pub struct MarketplaceState(pub Mutex<MarketplaceClient>);

#[tauri::command]
pub async fn login_marketplace(
    email: String,
    password: String,
    state: State<'_, MarketplaceState>,
) -> Result<String, String> {
    let client = state.0.lock().unwrap();
    // This is synchronous lock but we need async login, actually let's just make a new client or unlock
    // Wait, holding a std::sync::Mutex across an await is an error in Rust.
    // Let's drop the lock or use tokio::sync::Mutex.
    // For simplicity of Tauri commands, we'll recreate client or use a clone of token.
    drop(client);
    
    // Create temporary client to login
    let mut temp_client = MarketplaceClient::new();
    match temp_client.login(&email, &password).await {
        Ok(token) => {
            let mut client = state.0.lock().unwrap();
            client.set_token(token.clone());
            Ok(token)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn fetch_marketplace_extensions() -> Result<Vec<ExtensionData>, String> {
    let client = MarketplaceClient::new();
    client.fetch_extensions().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn publish_extension(
    folder_path: String,
    version: String,
    changelog: String,
    state: State<'_, MarketplaceState>,
) -> Result<String, String> {
    let path = PathBuf::from(&folder_path);
    let manifest_path = path.join("proto-extension.json");
    
    if !manifest_path.exists() {
        return Err("proto-extension.json not found in the specified folder".to_string());
    }

    let manifest_content = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: ExtensionManifest = serde_json::from_str(&manifest_content).map_err(|e| e.to_string())?;

    let token = {
        let client = state.0.lock().unwrap();
        client.token.clone()
    };

    if token.is_none() {
        return Err("Not logged in".to_string());
    }

    let mut client = MarketplaceClient::new();
    client.set_token(token.unwrap());

    // 1. Create extension
    let req = CreateExtensionRequest {
        name: manifest.name.clone(),
        description: manifest.description.unwrap_or_default(),
        category: "General".to_string(),
        tech_stack: vec![manifest.runtime.r#type],
        visibility: "public".to_string(),
    };

    let ext_id = client.create_extension(&req).await.map_err(|e| e.to_string())?;

    // 2. Zip folder
    let temp_zip = std::env::temp_dir().join(format!("{}.zip", ext_id));
    zip_extension_dir(&path, &temp_zip).map_err(|e| e.to_string())?;

    // 3. Upload release
    client.upload_release(&ext_id, &version, &changelog, &temp_zip).await.map_err(|e| e.to_string())?;

    Ok(ext_id)
}
