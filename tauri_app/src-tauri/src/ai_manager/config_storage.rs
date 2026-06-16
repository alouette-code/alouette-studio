use crate::ai_manager::EngineConfig;
use std::fs;
use std::path::PathBuf;

fn get_config_path() -> PathBuf {
    // Tạm thời hardcode thư mục dự án theo yêu cầu của user, 
    // trong thực tế có thể dùng app_dir của Tauri.
    PathBuf::from("/home/nhatanh/projet/alouette_studio/core_engine/app_data/AIlocal.yml")
}

pub fn save_settings(config: EngineConfig) -> Result<(), String> {
    let path = get_config_path();
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }

    // Load existing list or create new
    let mut configs = load_all_settings().unwrap_or_else(|_| Vec::new());
    
    // Replace if exists (based on engine_id or model_name), or append
    // Here we assume unique identification by model_name + engine_id
    let mut found = false;
    for existing in configs.iter_mut() {
        if existing.engine_id == config.engine_id && existing.model_name == config.model_name {
            *existing = config.clone();
            found = true;
            break;
        }
    }
    
    if !found {
        configs.push(config);
    }

    let yaml_string = serde_yaml::to_string(&configs)
        .map_err(|e| format!("Failed to serialize config to YAML: {}", e))?;
        
    fs::write(&path, yaml_string)
        .map_err(|e| format!("Failed to write AIlocal.yml: {}", e))?;
        
    Ok(())
}

pub fn load_all_settings() -> Result<Vec<EngineConfig>, String> {
    let path = get_config_path();
    
    if !path.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read AIlocal.yml: {}", e))?;
        
    let configs: Vec<EngineConfig> = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML array: {}", e))?;
        
    Ok(configs)
}

pub fn delete_setting(engine_id: String, model_name: Option<String>) -> Result<(), String> {
    let path = get_config_path();
    if !path.exists() { return Ok(()); }
    
    let mut configs = load_all_settings()?;
    
    let target_model_name = model_name.unwrap_or_default();
    
    configs.retain(|c| !(c.engine_id == engine_id && c.model_name == target_model_name));
    
    let yaml_string = serde_yaml::to_string(&configs)
        .map_err(|e| format!("Failed to serialize config to YAML: {}", e))?;
        
    fs::write(&path, yaml_string)
        .map_err(|e| format!("Failed to write AIlocal.yml: {}", e))?;
        
    Ok(())
}
