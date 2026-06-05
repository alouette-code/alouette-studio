use crate::state::AppState;
use core_engine::{SandboxConfig, EnvSimulationConfig};
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn load_env_simulation_configs(
    state: State<'_, AppState>,
) -> Result<HashMap<String, EnvSimulationConfig>, String> {
    let pm = state.process_manager.lock().await;
    let path = pm.app_data_dir.join("env_simulation.yml");
    EnvSimulationConfig::load_all_from_file(path)
}

#[tauri::command]
pub async fn save_env_simulation_config(
    state: State<'_, AppState>,
    config: EnvSimulationConfig,
) -> Result<(), String> {
    let pm = state.process_manager.lock().await;
    let path = pm.app_data_dir.join("env_simulation.yml");
    let mut configs = EnvSimulationConfig::load_all_from_file(&path)?;
    configs.insert(config.project_id.clone(), config);
    EnvSimulationConfig::save_all_to_file(&configs, &path)
}

#[tauri::command]
pub async fn load_sandbox_configs(
    state: State<'_, AppState>,
) -> Result<HashMap<String, SandboxConfig>, String> {
    let pm = state.process_manager.lock().await;
    let db = &pm.db_manager;

    let configs = db.load_all_sandbox_configs()?;
    let mut map = HashMap::new();
    for cfg in configs {
        map.insert(cfg.project_id.clone(), cfg);
    }
    Ok(map)
}

#[tauri::command]
pub async fn save_sandbox_config(
    state: State<'_, AppState>,
    config: SandboxConfig,
) -> Result<(), String> {
    let pm = state.process_manager.lock().await;
    let db = &pm.db_manager;
    db.save_sandbox_config(&config)?;
    Ok(())
}

#[tauri::command]
pub async fn save_all_sandbox_configs(
    state: State<'_, AppState>,
    configs: Vec<SandboxConfig>,
) -> Result<(), String> {
    let pm = state.process_manager.lock().await;
    let db = &pm.db_manager;
    for config in &configs {
        db.save_sandbox_config(config)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_sandbox_config(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<(), String> {
    let pm = state.process_manager.lock().await;
    let db = &pm.db_manager;
    db.delete_sandbox_config(&project_id)?;
    Ok(())
}
