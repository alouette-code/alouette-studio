/// Get application settings
pub async fn get_settings() -> Result<String, String> {
    let path = app_data_dir().join("settings.toml");
    let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
    Ok(content)
}

/// Save application settings
pub async fn save_settings(content: String) -> Result<(), String> {
    let path = app_data_dir().join("settings.toml");
    if let Some(parent) = std::path::Path::new(&path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }
    tokio::fs::write(&path, &content)
        .await
        .map_err(|e| e.to_string())
}

/// Reset settings to default
pub async fn reset_settings() -> Result<(), String> {
    let path = app_data_dir().join("settings.toml");
    let _ = tokio::fs::remove_file(&path).await;
    Ok(())
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
