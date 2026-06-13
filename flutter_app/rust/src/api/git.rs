/// Get git status for a working directory
pub async fn git_get_status(cwd: String) -> Result<String, String> {
    let output = tokio::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

/// Stage a file
pub async fn git_stage_file(cwd: String, file: String) -> Result<(), String> {
    let output = tokio::process::Command::new("git")
        .args(["add", &file])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

/// Stage all files
pub async fn git_stage_all(cwd: String) -> Result<(), String> {
    let output = tokio::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

/// Commit with a message
pub async fn git_commit(cwd: String, message: String) -> Result<(), String> {
    let output = tokio::process::Command::new("git")
        .args(["commit", "-m", &message])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

/// Push commits
pub async fn git_push(cwd: String) -> Result<(), String> {
    let output = tokio::process::Command::new("git")
        .args(["push"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

/// Pull changes
pub async fn git_pull(cwd: String) -> Result<(), String> {
    let output = tokio::process::Command::new("git")
        .args(["pull"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

/// Get git log
pub async fn git_get_log(cwd: String, limit: Option<i32>) -> Result<Vec<GitCommit>, String> {
    let limit = limit.unwrap_or(20);
    let output = tokio::process::Command::new("git")
        .args([
            "log",
            &format!("--max-count={limit}"),
            "--format=%H|%an|%ae|%at|%s",
        ])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(5, '|').collect();
        if parts.len() >= 5 {
            commits.push(GitCommit {
                hash: parts[0].to_string(),
                author: parts[1].to_string(),
                email: parts[2].to_string(),
                timestamp: parts[3].parse().unwrap_or(0),
                message: parts[4].to_string(),
            });
        }
    }
    Ok(commits)
}

#[derive(serde::Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub message: String,
}
