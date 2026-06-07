use crate::state::{AppState, log_to_app_file};
use base64::{Engine as _, engine::general_purpose};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

async fn validate_path(state: &AppState, path_str: &str) -> Result<PathBuf, String> {
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let canonical_root = fs::canonicalize(&workspace_root)
        .map_err(|e| format!("Failed to canonicalize workspace root: {}", e))?;
    
    let path = Path::new(path_str);
    let absolute_target = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    // Prevent traversal using component checks
    for component in path.components() {
        if component == std::path::Component::ParentDir {
            return Err("Security Boundary Error: Parent directory traversal (..) is forbidden.".to_string());
        }
    }

    // Resolve the closest existing parent/ancestor
    let mut ancestor = absolute_target.clone();
    let mut canonical_ancestor = None;

    while let Some(parent) = ancestor.parent() {
        if ancestor.exists() {
            if let Ok(canon) = fs::canonicalize(&ancestor) {
                canonical_ancestor = Some(canon);
                break;
            }
        }
        ancestor = parent.to_path_buf();
    }

    if canonical_ancestor.is_none() && ancestor.exists() {
        if let Ok(canon) = fs::canonicalize(&ancestor) {
            canonical_ancestor = Some(canon);
        }
    }

    let canonical_ancestor = canonical_ancestor
        .ok_or_else(|| format!("Security Error: Path parent does not exist or cannot be resolved: {}", absolute_target.display()))?;

    // Allow access if the path is inside the main workspace root
    if canonical_ancestor.starts_with(&canonical_root) {
        return Ok(absolute_target);
    }

    // Otherwise, check if it starts with any registered project's CWD
    let pm = state.process_manager.lock().await;
    let configs = pm.get_configs();
    for config in configs {
        if let Some(cwd) = &config.cwd {
            if let Ok(canonical_cwd) = fs::canonicalize(Path::new(cwd)) {
                if canonical_ancestor.starts_with(&canonical_cwd) {
                    return Ok(absolute_target);
                }
            }
        }
    }

    Err(format!(
        "Security Boundary Error: Access to '{}' is forbidden. Outside workspace and registered project paths.",
        absolute_target.display()
    ))
}

#[derive(serde::Serialize, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
}

#[tauri::command]
pub async fn get_project_files(state: State<'_, AppState>, dir_path: Option<String>) -> Result<Vec<FileNode>, String> {
    let path_str = dir_path.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let validated = validate_path(&state, &path_str).await?;
    get_directory_contents(state, validated.to_string_lossy().to_string()).await
}

#[tauri::command]
pub async fn read_file_content(state: State<'_, AppState>, path: String) -> Result<String, String> {
    log_to_app_file(&format!("Reading file: {}", path));
    let validated = validate_path(&state, &path).await?;
    let bytes = std::fs::read(&validated).map_err(|e| e.to_string())?;

    if bytes.len() > 10 * 1024 * 1024 {
        return Err("File quá lớn để mở trong trình soạn thảo. Vui lòng sử dụng terminal.".to_string());
    }

    Ok(general_purpose::STANDARD.encode(bytes))
}

#[tauri::command]
pub async fn write_file_content(state: State<'_, AppState>, path: String, content: String) -> Result<(), String> {
    log_to_app_file(&format!("Writing file: {}", path));
    let validated = validate_path(&state, &path).await?;
    std::fs::write(&validated, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_file(state: State<'_, AppState>, path: String) -> Result<(), String> {
    log_to_app_file(&format!("Creating file: {}", path));
    let validated = validate_path(&state, &path).await?;
    if validated.exists() {
        return Err("File already exists".to_string());
    }
    if let Some(parent) = validated.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(validated, "").map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_folder(state: State<'_, AppState>, path: String) -> Result<(), String> {
    log_to_app_file(&format!("Creating folder: {}", path));
    let validated = validate_path(&state, &path).await?;
    if validated.exists() {
        return Err("Folder already exists".to_string());
    }
    fs::create_dir_all(validated).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_directory_contents(state: State<'_, AppState>, dir_path: String) -> Result<Vec<FileNode>, String> {
    let validated = validate_path(&state, &dir_path).await?;
    let mut entries = Vec::new();
    let read_entries = fs::read_dir(&validated).map_err(|e| e.to_string())?;

    for entry_result in read_entries {
        if let Ok(entry) = entry_result {
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name == ".git" {
                continue;
            }

            let is_dir = entry_path.is_dir();
            entries.push(FileNode {
                name,
                path: entry_path.to_string_lossy().to_string(),
                is_dir,
                children: None,
            });
        }
    }

    entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(entries)
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct SearchFileItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[tauri::command]
pub async fn get_all_files_and_folders(state: State<'_, AppState>, dir_path: Option<String>) -> Result<Vec<SearchFileItem>, String> {
    let path_str = dir_path.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let validated = validate_path(&state, &path_str).await?;
    let mut items = Vec::new();
    collect_files_and_folders_recursive(&validated, &validated, &mut items)?;

    // Sort to have shorter paths and directories first, which usually feels cleaner
    items.sort_by(|a, b| {
        let depth_a = a.path.matches('/').count();
        let depth_b = b.path.matches('/').count();
        if depth_a != depth_b {
            depth_a.cmp(&depth_b)
        } else if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(items)
}

fn collect_files_and_folders_recursive(
    root: &Path,
    dir: &Path,
    items: &mut Vec<SearchFileItem>,
) -> Result<(), String> {
    if dir.is_dir() {
        let read_entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
        for entry_result in read_entries {
            if let Ok(entry) = entry_result {
                let entry_path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if name == ".git"
                    || name == ".idea"
                    || name == "target"
                    || name == "node_modules"
                    || name == "dist"
                    || name == "build"
                    || name == "logs"
                    || name == ".tauri"
                {
                    continue;
                }

                let is_dir = entry_path.is_dir();
                let rel_path = match entry_path.strip_prefix(root) {
                    Ok(p) => p.to_string_lossy().to_string(),
                    Err(_) => entry_path.to_string_lossy().to_string(),
                };
                let normalized_path = rel_path.replace("\\", "/");

                items.push(SearchFileItem {
                    name,
                    path: normalized_path,
                    is_dir,
                });

                if is_dir {
                    collect_files_and_folders_recursive(root, &entry_path, items)?;
                }
            }
        }
    }
    Ok(())
}




