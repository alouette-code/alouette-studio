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
    pub full_hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
    pub is_pushed: bool,
}

#[derive(serde::Serialize, Clone)]
pub struct CommitFileStat {
    pub path: String,
    pub status: String,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(serde::Serialize, Clone)]
pub struct CommitDetails {
    pub hash: String,
    pub total_files: usize,
    pub added_files: usize,
    pub modified_files: usize,
    pub deleted_files: usize,
    pub total_insertions: usize,
    pub total_deletions: usize,
    pub files: Vec<CommitFileStat>,
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

fn resolve_remote_tracking_ref(cwd: &str) -> Option<(String, String, String)> {
    let branch = run_git_cmd(cwd, &["rev-parse", "--abbrev-ref", "HEAD"]).ok()?;
    if branch == "HEAD" || branch.is_empty() {
        return None;
    }

    if let Ok(upstream) = run_git_cmd(cwd, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]) {
        if !upstream.is_empty() {
            let remote_name = upstream.split('/').next().unwrap_or("origin").to_string();
            return Some((remote_name, branch, upstream));
        }
    }

    let fallback_ref = format!("origin/{}", branch);
    if run_git_cmd(cwd, &["rev-parse", "--verify", &fallback_ref]).is_ok() {
        return Some(("origin".to_string(), branch, fallback_ref));
    }

    None
}

fn get_unpushed_commits_ordered(cwd: &str, remote_ref: Option<&str>) -> Vec<String> {
    let range = remote_ref.map(|r| format!("{}..HEAD", r));
    let output = match &range {
        Some(rng) => run_git_cmd(cwd, &["rev-list", "--reverse", rng]),
        None => run_git_cmd(cwd, &["rev-list", "--reverse", "HEAD"]),
    };
    match output {
        Ok(out) => out
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
pub async fn git_get_log(cwd: String) -> Result<Vec<CommitInfo>, String> {
    let remote_info = resolve_remote_tracking_ref(&cwd);
    let remote_ref = remote_info.as_ref().map(|(_, _, r)| r.as_str());
    let unpushed_set: std::collections::HashSet<String> =
        get_unpushed_commits_ordered(&cwd, remote_ref)
            .into_iter()
            .collect();

    let log_format = "%h|%H|%an|%cd|%s";
    let output = run_git_cmd(
        &cwd,
        &[
            "log",
            "-n",
            "30",
            "--date=format:%d/%m %H:%M",
            &format!("--pretty=format:{}", log_format),
        ],
    )?;

    let mut commits = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 5 {
            let full_hash = parts[1].to_string();
            let is_pushed = remote_ref.is_some() && !unpushed_set.contains(&full_hash);
            commits.push(CommitInfo {
                hash: parts[0].to_string(),
                full_hash,
                author: parts[2].to_string(),
                date: parts[3].to_string(),
                subject: parts[4..].join("|"),
                is_pushed,
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

#[tauri::command]
pub async fn git_get_commit_details(cwd: String, hash: String) -> Result<CommitDetails, String> {
    let status_output = run_git_cmd(&cwd, &["show", "--name-status", "--pretty=format:", &hash])?;
    let numstat_output = run_git_cmd(&cwd, &["show", "--numstat", "--pretty=format:", &hash])?;

    let mut stats_map: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();
    for line in numstat_output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('\t').collect();
        if parts.len() >= 3 {
            let ins = parts[0].parse::<usize>().unwrap_or(0);
            let del = parts[1].parse::<usize>().unwrap_or(0);
            let path = parts[parts.len() - 1].to_string();
            stats_map.insert(path, (ins, del));
        }
    }

    let mut files = Vec::new();
    let mut added_files = 0;
    let mut modified_files = 0;
    let mut deleted_files = 0;
    let mut total_insertions = 0;
    let mut total_deletions = 0;

    for line in status_output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('\t').collect();
        if parts.len() >= 2 {
            let status_code = parts[0];
            let (status, path) = if status_code.starts_with('R') && parts.len() >= 3 {
                modified_files += 1;
                ("renamed".to_string(), parts[2].to_string())
            } else {
                let p = parts[1].to_string();
                let st = match status_code {
                    "A" => {
                        added_files += 1;
                        "added"
                    }
                    "D" => {
                        deleted_files += 1;
                        "deleted"
                    }
                    _ => {
                        modified_files += 1;
                        "modified"
                    }
                };
                (st.to_string(), p)
            };

            let (insertions, deletions) = stats_map.get(&path).copied().unwrap_or((0, 0));
            total_insertions += insertions;
            total_deletions += deletions;

            files.push(CommitFileStat {
                path,
                status,
                insertions,
                deletions,
            });
        }
    }

    Ok(CommitDetails {
        hash,
        total_files: files.len(),
        added_files,
        modified_files,
        deleted_files,
        total_insertions,
        total_deletions,
        files,
    })
}

#[tauri::command]
pub async fn git_get_commit_file_diff(cwd: String, hash: String, file: String) -> Result<String, String> {
    let output = run_git_cmd(&cwd, &["show", "--pretty=format:", "--unified=3", &hash, "--", &file])?;
    Ok(output)
}

fn run_git_with_env(cwd: &str, args: &[&str], envs: &[(&str, &str)]) -> Result<String, String> {
    log_to_app_file(&format!("Running git command with env in {}: {:?}", cwd, args));
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(cwd);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let output = cmd.output().map_err(|e| format!("Failed to execute git: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err_msg.is_empty() {
            Err(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(err_msg)
        }
    }
}

#[tauri::command]
pub async fn git_push_single_commit(cwd: String, hash: String) -> Result<String, String> {
    let (remote_name, current_branch, remote_ref) = resolve_remote_tracking_ref(&cwd)
        .ok_or_else(|| "Chưa có nhánh remote upstream (origin). Hãy Push toàn bộ lần đầu trước.".to_string())?;

    let target_full_hash = run_git_cmd(&cwd, &["rev-parse", &hash])?;
    let unpushed = get_unpushed_commits_ordered(&cwd, Some(&remote_ref));

    let pos = unpushed
        .iter()
        .position(|h| h == &target_full_hash)
        .ok_or_else(|| "Commit này đã được push lên remote trước đó.".to_string())?;

    let push_hash = if pos == 0 {
        target_full_hash
    } else {
        // Ensure working tree is clean before reordering commits
        let dirty = run_git_cmd(&cwd, &["status", "--porcelain"])?;
        if !dirty.trim().is_empty() {
            return Err("Có thay đổi chưa commit ở thư mục làm việc. Vui lòng commit hoặc stash trước khi push riêng commit giữa.".to_string());
        }

        // Reorder: move target commit to the front (right after remote_ref)
        let mut reordered = Vec::with_capacity(unpushed.len());
        reordered.push(unpushed[pos].clone());
        for (i, h) in unpushed.iter().enumerate() {
            if i != pos {
                reordered.push(h.clone());
            }
        }

        let todo_content: String = reordered
            .iter()
            .map(|h| format!("pick {}", h))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";

        let temp_path = std::env::temp_dir().join(format!("alouette_rebase_todo_{}", std::process::id()));
        std::fs::write(&temp_path, &todo_content).map_err(|e| format!("Không thể tạo file tạm: {}", e))?;

        let editor_cmd = format!("sh -c 'cp \"{}\" \"$1\"' _", temp_path.to_string_lossy());
        let rebase_res = run_git_with_env(
            &cwd,
            &["rebase", "-i", &remote_ref],
            &[("GIT_SEQUENCE_EDITOR", &editor_cmd)],
        );
        let _ = std::fs::remove_file(&temp_path);

        if let Err(e) = rebase_res {
            let _ = run_git_cmd(&cwd, &["rebase", "--abort"]);
            return Err(format!(
                "Không thể tách riêng commit này do xung đột code (conflict) với commit đứng trước nó. Chi tiết: {}",
                e
            ));
        }

        let new_unpushed = get_unpushed_commits_ordered(&cwd, Some(&remote_ref));
        new_unpushed
            .first()
            .cloned()
            .ok_or_else(|| "Không tìm thấy commit sau khi sắp xếp lại.".to_string())?
    };

    let refspec = format!("{}:refs/heads/{}", push_hash, current_branch);
    run_git_cmd(&cwd, &["push", &remote_name, &refspec])?;

    Ok(format!("Đã push riêng commit {} lên {}/{}", &hash, remote_name, current_branch))
}

#[tauri::command]
pub async fn git_reword_commit(cwd: String, hash: String, new_message: String) -> Result<(), String> {
    let trimmed_msg = new_message.trim();
    if trimmed_msg.is_empty() {
        return Err("Nội dung commit message không được để trống.".to_string());
    }

    let remote_info = resolve_remote_tracking_ref(&cwd);
    let remote_ref = remote_info.as_ref().map(|(_, _, r)| r.as_str());
    let unpushed = get_unpushed_commits_ordered(&cwd, remote_ref);
    let target_full_hash = run_git_cmd(&cwd, &["rev-parse", &hash])?;

    if !unpushed.contains(&target_full_hash) {
        return Err("Chỉ có thể sửa message của commit chưa push lên remote.".to_string());
    }

    let head_hash = run_git_cmd(&cwd, &["rev-parse", "HEAD"])?;
    if target_full_hash == head_hash {
        run_git_cmd(&cwd, &["commit", "--amend", "--only", "-m", trimmed_msg])?;
        return Ok(());
    }

    let dirty = run_git_cmd(&cwd, &["status", "--porcelain"])?;
    if !dirty.trim().is_empty() {
        return Err("Vui lòng commit hoặc stash thay đổi hiện tại trước khi sửa message của commit cũ.".to_string());
    }

    let todo_content: String = unpushed
        .iter()
        .map(|h| {
            if h == &target_full_hash {
                format!("reword {}", h)
            } else {
                format!("pick {}", h)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    let pid = std::process::id();
    let todo_path = std::env::temp_dir().join(format!("alouette_reword_todo_{}", pid));
    let msg_path = std::env::temp_dir().join(format!("alouette_reword_msg_{}", pid));

    std::fs::write(&todo_path, &todo_content).map_err(|e| format!("Lỗi ghi file tạm: {}", e))?;
    std::fs::write(&msg_path, format!("{}\n", trimmed_msg)).map_err(|e| format!("Lỗi ghi file message: {}", e))?;

    let seq_editor = format!("sh -c 'cp \"{}\" \"$1\"' _", todo_path.to_string_lossy());
    let msg_editor = format!("sh -c 'cp \"{}\" \"$1\"' _", msg_path.to_string_lossy());

    let base_arg = match remote_ref {
        Some(r) => r.to_string(),
        None => "--root".to_string(),
    };

    let res = run_git_with_env(
        &cwd,
        &["rebase", "-i", &base_arg],
        &[
            ("GIT_SEQUENCE_EDITOR", &seq_editor),
            ("GIT_EDITOR", &msg_editor),
        ],
    );

    let _ = std::fs::remove_file(&todo_path);
    let _ = std::fs::remove_file(&msg_path);

    if let Err(e) = res {
        let _ = run_git_cmd(&cwd, &["rebase", "--abort"]);
        return Err(format!("Không thể sửa message commit: {}", e));
    }

    Ok(())
}
