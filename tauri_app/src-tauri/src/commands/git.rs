use std::process::Command;
use crate::state::log_to_app_file;

#[derive(serde::Serialize, Clone)]
pub struct GitFile {
    pub path: String,
    pub status: String,
}

#[derive(serde::Serialize, Clone)]
pub struct GitStatus {
    pub branch: String,
    pub remote: String,
    pub staged: Vec<GitFile>,
    pub unstaged: Vec<GitFile>,
}

#[derive(serde::Serialize, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
}

fn run_git_cmd(cwd: &str, args: &[&str]) -> Result<String, String> {
    log_to_app_file(&format!("Running git command in {}: {:?}", cwd, args));
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err_msg.is_empty() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(err_msg)
        }
    }
}

#[tauri::command]
pub async fn git_get_status(cwd: String) -> Result<GitStatus, String> {
    // 1. Get current branch
    let branch = run_git_cmd(&cwd, &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_else(|_| "HEAD (detached)".to_string());

    // 2. Get remote URL
    let remote = run_git_cmd(&cwd, &["remote", "get-url", "origin"])
        .unwrap_or_else(|_| "Local Repository".to_string());

    // 3. Get status porcelain
    let status_output = run_git_cmd(&cwd, &["status", "--porcelain"])?;
    
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();

    for line in status_output.lines() {
        if line.len() < 4 {
            continue;
        }
        let (xy, path) = line.split_at(2);
        let path = path.trim().to_string();
        let chars: Vec<char> = xy.chars().collect();
        let x = chars.get(0).copied().unwrap_or(' ');
        let y = chars.get(1).copied().unwrap_or(' ');

        // X status (staged changes status index)
        if x != ' ' && x != '?' && x != '!' {
            let status = match x {
                'M' => "modified",
                'A' => "added",
                'D' => "deleted",
                'R' => "renamed",
                'C' => "copied",
                _ => "staged",
            };
            staged.push(GitFile { path: path.clone(), status: status.to_string() });
        }

        // Y status (unstaged changes status working tree)
        if y != ' ' {
            let status = match y {
                'M' => "modified",
                'D' => "deleted",
                '?' => "untracked",
                _ => "modified",
            };
            unstaged.push(GitFile { path, status: status.to_string() });
        } else if x == '?' && y == '?' {
            unstaged.push(GitFile { path, status: "untracked".to_string() });
        }
    }

    Ok(GitStatus {
        branch,
        remote,
        staged,
        unstaged,
    })
}

#[tauri::command]
pub async fn git_stage_file(cwd: String, file: String) -> Result<(), String> {
    run_git_cmd(&cwd, &["add", &file])?;
    Ok(())
}

#[tauri::command]
pub async fn git_stage_all(cwd: String) -> Result<(), String> {
    run_git_cmd(&cwd, &["add", "."])?;
    Ok(())
}

#[tauri::command]
pub async fn git_unstage_file(cwd: String, file: String) -> Result<(), String> {
    run_git_cmd(&cwd, &["reset", "HEAD", &file])?;
    Ok(())
}

#[tauri::command]
pub async fn git_unstage_all(cwd: String) -> Result<(), String> {
    run_git_cmd(&cwd, &["reset", "HEAD"])?;
    Ok(())
}

#[tauri::command]
pub async fn git_discard_file(cwd: String, file: String, status: String) -> Result<(), String> {
    if status == "untracked" {
        let path = std::path::Path::new(&cwd).join(&file);
        if path.is_dir() {
            std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
        } else if path.is_file() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    } else {
        run_git_cmd(&cwd, &["checkout", "--", &file])?;
    }
    Ok(())
}

#[tauri::command]
pub async fn git_commit(cwd: String, message: String) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("Commit message cannot be empty".to_string());
    }
    run_git_cmd(&cwd, &["commit", "-m", &message])?;
    Ok(())
}

#[tauri::command]
pub async fn git_push(cwd: String) -> Result<(), String> {
    run_git_cmd(&cwd, &["push"])?;
    Ok(())
}

#[tauri::command]
pub async fn git_pull(cwd: String) -> Result<(), String> {
    run_git_cmd(&cwd, &["pull"])?;
    Ok(())
}

#[tauri::command]
pub async fn git_get_log(cwd: String) -> Result<Vec<CommitInfo>, String> {
    let log_format = "%h|%an|%ad|%s";
    let output = run_git_cmd(&cwd, &["log", "--oneline", "-n", "30", &format!("--pretty=format:{}", log_format)])?;
    
    let mut commits = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            commits.push(CommitInfo {
                hash: parts[0].to_string(),
                author: parts[1].to_string(),
                date: parts[2].to_string(),
                subject: parts[3..].join("|"),
            });
        }
    }
    Ok(commits)
}

#[tauri::command]
pub async fn git_get_commit_files(cwd: String, hash: String) -> Result<Vec<GitFile>, String> {
    let output = run_git_cmd(&cwd, &["show", "--name-status", "--pretty=format:", &hash])?;
    
    let mut files = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let status_char = parts[0];
            let path = parts[1..].join(" ");
            let status = match status_char {
                "M" => "modified",
                "A" => "added",
                "D" => "deleted",
                _ => "modified",
            };
            files.push(GitFile { path, status: status.to_string() });
        }
    }
    Ok(files)
}
