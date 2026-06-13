/// Read projects config from app_data
pub async fn get_projects_config_yaml() -> Result<String, String> {
    let path = app_data_dir().join("projects.toml");
    tokio::fs::read_to_string(&path)
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
