/// List files in a directory (non-recursive)
pub fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = std::path::Path::new(&path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        entries.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            is_folder: ft.is_dir(),
        });
    }
    entries.sort_by(|a, b| {
        if a.is_folder != b.is_folder {
            b.is_folder.cmp(&a.is_folder)
        } else {
            a.name.cmp(&b.name)
        }
    });
    Ok(entries)
}

/// Read file content as string
pub fn read_file_content(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Write content to file
pub fn write_file_content(path: String, content: String) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}

/// Create a new file
pub fn create_file(path: String) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::File::create(&path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Create a new folder
pub fn create_folder(path: String) -> Result<(), String> {
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())
}

/// Get directory contents (recursive)
pub fn get_directory_contents(path: String) -> Result<Vec<FileEntry>, String> {
    let mut result = Vec::new();
    let dir = std::path::Path::new(&path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }
    collect_entries(dir, &mut result, 0, 5)?; // max depth 5
    Ok(result)
}

fn collect_entries(
    dir: &std::path::Path,
    result: &mut Vec<FileEntry>,
    depth: usize,
    max_depth: usize,
) -> Result<(), String> {
    if depth > max_depth {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        let is_dir = ft.is_dir();
        result.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            is_folder: is_dir,
        });
        if is_dir {
            collect_entries(&entry.path(), result, depth + 1, max_depth)?;
        }
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_folder: bool,
}
