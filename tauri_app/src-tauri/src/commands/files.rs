use crate::state::log_to_app_file;
use base64::{Engine as _, engine::general_purpose};
use std::fs;
use std::path::Path;

#[derive(serde::Serialize, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
}

#[tauri::command]
pub fn get_project_files(dir_path: Option<String>) -> Result<Vec<FileNode>, String> {
    let path_str = dir_path.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    get_directory_contents(path_str)
}

#[tauri::command]
pub async fn read_file_content(path: String) -> Result<String, String> {
    log_to_app_file(&format!("Reading file: {}", path));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;

    if bytes.len() > 10 * 1024 * 1024 {
        return Err("File quá lớn để mở trong trình soạn thảo. Vui lòng sử dụng terminal.".to_string());
    }

    Ok(general_purpose::STANDARD.encode(bytes))
}

#[tauri::command]
pub async fn write_file_content(path: String, content: String) -> Result<(), String> {
    log_to_app_file(&format!("Writing file: {}", path));
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_file(path: String) -> Result<(), String> {
    log_to_app_file(&format!("Creating file: {}", path));
    let path = Path::new(&path);
    if path.exists() {
        return Err("File already exists".to_string());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, "").map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_folder(path: String) -> Result<(), String> {
    log_to_app_file(&format!("Creating folder: {}", path));
    let path = Path::new(&path);
    if path.exists() {
        return Err("Folder already exists".to_string());
    }
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_directory_contents(dir_path: String) -> Result<Vec<FileNode>, String> {
    let path = Path::new(&dir_path);
    if !path.exists() {
        return Err("Directory does not exist".to_string());
    }

    let mut entries = Vec::new();
    let read_entries = fs::read_dir(path).map_err(|e| e.to_string())?;

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
pub fn get_all_files_and_folders(dir_path: Option<String>) -> Result<Vec<SearchFileItem>, String> {
    let path_str = dir_path.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let root_path = Path::new(&path_str);
    if !root_path.exists() {
        return Err("Directory does not exist".to_string());
    }

    let mut items = Vec::new();
    collect_files_and_folders_recursive(root_path, root_path, &mut items)?;

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



