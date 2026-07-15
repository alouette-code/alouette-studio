use crate::state::{log_to_app_file, AppState};
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tauri::{Manager, State};
use tokio::fs;

// ── Canonical path cache (TTL 10 giây) ──
static CANONICAL_CACHE: LazyLock<DashMap<String, (Instant, PathBuf)>> =
    LazyLock::new(|| DashMap::new());
const CACHE_TTL: Duration = Duration::from_secs(10);

fn cached_canonicalize(path: &Path) -> Result<PathBuf, String> {
    let key = path.to_string_lossy().to_string();

    // Check cache
    if let Some(entry) = CANONICAL_CACHE.get(&key) {
        if entry.0.elapsed() < CACHE_TTL {
            return Ok(entry.1.clone());
        }
    }

    // Miss — canonicalize thật
    let canonical = std::fs::canonicalize(path)
        .map_err(|e| format!("Failed to canonicalize '{}': {}", path.display(), e))?;

    CANONICAL_CACHE.insert(key, (Instant::now(), canonical.clone()));
    Ok(canonical)
}

/// Resolve workspace root một lần và cache lại.
fn workspace_root() -> PathBuf {
    static ROOT: LazyLock<PathBuf> =
        LazyLock::new(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    ROOT.clone()
}

async fn validate_path(state: &AppState, path_str: &str) -> Result<PathBuf, String> {
    let workspace_root = workspace_root();
    let canonical_root = cached_canonicalize(&workspace_root)?;

    let path = Path::new(path_str);
    let absolute_target = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    // Prevent traversal using component checks
    for component in path.components() {
        if component == std::path::Component::ParentDir {
            return Err(
                "Security Boundary Error: Parent directory traversal (..) is forbidden."
                    .to_string(),
            );
        }
    }

    // Resolve the closest existing parent/ancestor
    let mut ancestor = absolute_target.clone();
    let mut canonical_ancestor = None;

    while let Some(parent) = ancestor.parent() {
        if ancestor.exists() {
            if let Ok(canon) = cached_canonicalize(&ancestor) {
                canonical_ancestor = Some(canon);
                break;
            }
        }
        ancestor = parent.to_path_buf();
    }

    if canonical_ancestor.is_none() && ancestor.exists() {
        if let Ok(canon) = cached_canonicalize(&ancestor) {
            canonical_ancestor = Some(canon);
        }
    }

    let canonical_ancestor = canonical_ancestor.ok_or_else(|| {
        format!(
            "Security Error: Path parent does not exist or cannot be resolved: {}",
            absolute_target.display()
        )
    })?;

    // Allow access if the path is inside the main workspace root
    if canonical_ancestor.starts_with(&canonical_root) {
        return Ok(absolute_target);
    }

    // Otherwise, check if it starts with any registered project's CWD
    let pm = state.process_manager.lock().await;
    let configs = pm.get_configs();
    for config in configs {
        if let Some(cwd) = &config.cwd {
            if let Ok(canonical_cwd) = cached_canonicalize(Path::new(cwd)) {
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
pub async fn get_project_files(
    state: State<'_, AppState>,
    dir_path: Option<String>,
) -> Result<Vec<FileNode>, String> {
    let path_str = dir_path.unwrap_or_else(|| workspace_root().to_string_lossy().to_string());

    let validated = validate_path(&state, &path_str).await?;
    get_directory_contents(state, validated.to_string_lossy().to_string()).await
}

#[derive(serde::Serialize)]
pub struct FileContentResponse {
    pub encoding: String,
    pub content: String,
}

fn detect_image_mime_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.starts_with(b"RIFF") && bytes.len() > 11 && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    // Ico files
    if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        return Some("image/x-icon");
    }
    None
}

#[tauri::command]
pub async fn read_file_content(
    state: State<'_, AppState>,
    path: String,
) -> Result<FileContentResponse, String> {
    log_to_app_file(&format!("Reading file: {}", path));
    let validated = validate_path(&state, &path).await?;
    let metadata = fs::metadata(&validated).await.map_err(|e| e.to_string())?;

    if metadata.len() > 10 * 1024 * 1024 {
        return Err(
            "File quá lớn để mở trong trình soạn thảo. Vui lòng sử dụng terminal.".to_string(),
        );
    }

    let bytes = fs::read(&validated).await.map_err(|e| e.to_string())?;
    
    use base64::Engine;
    
    // Check for binary image signatures first
    if let Some(mime) = detect_image_mime_type(&bytes) {
        return Ok(FileContentResponse {
            encoding: mime.to_string(),
            content: base64::engine::general_purpose::STANDARD.encode(&bytes),
        });
    }

    match String::from_utf8(bytes.clone()) {
        Ok(text) => Ok(FileContentResponse {
            encoding: "utf8".to_string(),
            content: text,
        }),
        Err(_) => {
            Ok(FileContentResponse {
                encoding: "base64".to_string(),
                content: base64::engine::general_purpose::STANDARD.encode(bytes),
            })
        }
    }
}

#[tauri::command]
pub async fn write_file_content(
    state: State<'_, AppState>,
    path: String,
    content: String,
) -> Result<(), String> {
    log_to_app_file(&format!("Writing file: {}", path));
    let validated = validate_path(&state, &path).await?;
    fs::write(&validated, &content)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_file(state: State<'_, AppState>, path: String) -> Result<(), String> {
    log_to_app_file(&format!("Creating file: {}", path));
    let validated = validate_path(&state, &path).await?;
    if validated.exists() {
        return Err("File already exists".to_string());
    }
    if let Some(parent) = validated.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }
    fs::write(&validated, "").await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_folder(state: State<'_, AppState>, path: String) -> Result<(), String> {
    log_to_app_file(&format!("Creating folder: {}", path));
    let validated = validate_path(&state, &path).await?;
    if validated.exists() {
        return Err("Folder already exists".to_string());
    }
    fs::create_dir_all(&validated)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_item(state: State<'_, AppState>, path: String) -> Result<(), String> {
    log_to_app_file(&format!("Deleting item: {}", path));
    let validated = validate_path(&state, &path).await?;
    if !validated.exists() {
        return Err("Item does not exist".to_string());
    }
    if validated.is_dir() {
        fs::remove_dir_all(&validated)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(&validated)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn rename_item(state: State<'_, AppState>, old_path: String, new_path: String) -> Result<(), String> {
    log_to_app_file(&format!("Renaming item: {} to {}", old_path, new_path));
    let validated_old = validate_path(&state, &old_path).await?;
    let validated_new = validate_path(&state, &new_path).await?;
    
    if !validated_old.exists() {
        return Err("Source item does not exist".to_string());
    }
    if validated_new.exists() {
        return Err("Destination item already exists".to_string());
    }
    
    fs::rename(&validated_old, &validated_new)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn copy_dir_all(src: impl AsRef<std::path::Path>, dst: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn copy_item(state: State<'_, AppState>, source_path: String, dest_path: String) -> Result<(), String> {
    log_to_app_file(&format!("Copying item: {} to {}", source_path, dest_path));
    let validated_source = validate_path(&state, &source_path).await?;
    let validated_dest = validate_path(&state, &dest_path).await?;
    
    if !validated_source.exists() {
        return Err("Source item does not exist".to_string());
    }
    if validated_dest.exists() {
        return Err("Destination item already exists".to_string());
    }
    
    if validated_source.is_dir() {
        tokio::task::spawn_blocking(move || copy_dir_all(validated_source, validated_dest))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
    } else {
        fs::copy(&validated_source, &validated_dest)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_directory_contents(
    state: State<'_, AppState>,
    dir_path: String,
) -> Result<Vec<FileNode>, String> {
    let validated = validate_path(&state, &dir_path).await?;
    let mut entries = Vec::new();
    let mut read_dir = tokio::fs::read_dir(&validated)
        .await
        .map_err(|e| e.to_string())?;

    while let Some(entry) = read_dir.next_entry().await.map_err(|e| e.to_string())? {
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
pub async fn get_all_files_and_folders(
    state: State<'_, AppState>,
    dir_path: Option<String>,
) -> Result<Vec<SearchFileItem>, String> {
    let path_str = dir_path.unwrap_or_else(|| workspace_root().to_string_lossy().to_string());

    let validated = validate_path(&state, &path_str).await?;
    let mut items = Vec::new();
    collect_files_and_folders_recursive(&validated, &validated, &mut items).await?;

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

async fn collect_files_and_folders_recursive(
    root: &Path,
    dir: &Path,
    items: &mut Vec<SearchFileItem>,
) -> Result<(), String> {
    if dir.is_dir() {
        let mut read_dir = tokio::fs::read_dir(dir).await.map_err(|e| e.to_string())?;

        while let Some(entry) = read_dir.next_entry().await.map_err(|e| e.to_string())? {
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
                Box::pin(collect_files_and_folders_recursive(
                    root,
                    &entry_path,
                    items,
                ))
                .await?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn open_file_dialog() -> Result<Option<String>, String> {
    let file = rfd::FileDialog::new()
        .pick_file();
    
    Ok(file.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn open_folder_dialog() -> Result<Option<String>, String> {
    let folder = rfd::FileDialog::new()
        .pick_folder();
    
    Ok(folder.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn save_file_dialog(default_name: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(name) = default_name {
        dialog = dialog.set_file_name(&name);
    }
    let file = dialog.save_file();
    
    Ok(file.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn open_new_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    let label = format!("window_{}", chrono::Utc::now().timestamp_millis());
    let _window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        &label,
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Alouette Studio")
    .inner_size(1200.0, 800.0)
    .resizable(true)
    .decorations(true)
    .build()
    .map_err(|e: tauri::Error| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn open_vm_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("vm-manager") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let _window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "vm-manager",
        tauri::WebviewUrl::App("index.html?window=vm-manager".into()),
    )
    .title("Virtual Machine Configuration")
    .inner_size(800.0, 650.0)
    .resizable(true)
    .decorations(false)
    .build()
    .map_err(|e: tauri::Error| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn open_docker_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("docker-manager") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let _window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "docker-manager",
        tauri::WebviewUrl::App("index.html?window=docker-manager".into()),
    )
    .title("Docker Manager")
    .inner_size(800.0, 650.0)
    .resizable(true)
    .decorations(false)
    .build()
    .map_err(|e: tauri::Error| e.to_string())?;

    Ok(())
}
