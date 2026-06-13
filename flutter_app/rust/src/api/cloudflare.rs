/// Get Cloudflare tunnel configuration
pub async fn load_cloudflare_config() -> Result<String, String> {
    let path = app_data_dir().join("cloudflare_config.yml");
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| e.to_string())
}

/// Save Cloudflare tunnel configuration
pub async fn save_cloudflare_config(yaml: String) -> Result<(), String> {
    let path = app_data_dir().join("cloudflare_config.yml");
    tokio::fs::write(&path, &yaml)
        .await
        .map_err(|e| e.to_string())
}

fn app_data_dir() -> std::path::PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    let candidate = cwd.join("app_data");
    if candidate.exists() {
        candidate
    } else {
        cwd.parent()
            .map(|p| p.join("app_data"))
            .unwrap_or(candidate)
    }
}
