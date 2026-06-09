use std::cell::RefCell;
use std::path::Path;

use crate::state::log_to_app_file;

#[derive(serde::Serialize, Clone, Debug)]
pub struct GitDiffLine {
    /// 1-based line number in the current file (for "added" | "modified").
    /// For "deleted_context": the context line immediately after the deletion block.
    pub line_number: usize,
    /// "added" | "modified" | "deleted_context"
    pub change_type: String,
    /// Number of deleted lines (only meaningful for "deleted_context")
    pub deleted_count: usize,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct GitFileDiff {
    pub lines: Vec<GitDiffLine>,
    /// true if the file is untracked (never committed)
    pub untracked: bool,
}

/// Compute a diff between HEAD and the working tree for a given file.
///
/// Uses `git2` (libgit2) directly to avoid the pitfalls of:
///   - Shelling out to the git CLI
///   - Manually parsing unified diff format text
///
/// `cwd` is optional: if not provided, we try to find the git repo
/// by walking up from the file's parent directory.
#[tauri::command]
pub async fn git_get_file_diff(cwd: Option<String>, file: String) -> Result<GitFileDiff, String> {
    log_to_app_file(&format!("git_get_file_diff: cwd={:?}, file={}", cwd, file));

    let file_path = Path::new(&file);

    // Determine the git repository root
    let repo: git2::Repository = if let Some(ref cwd_str) = cwd {
        git2::Repository::open(cwd_str)
            .map_err(|e| format!("Failed to open git repo at '{}': {}", cwd_str, e.message()))?
    } else {
        // Auto-detect: walk up from the file's directory
        let mut dir = file_path.parent().unwrap_or(Path::new("."));
        // Try up to 10 levels up
        let mut found: Option<git2::Repository> = None;
        for _ in 0..10 {
            if let Ok(r) = git2::Repository::open(dir) {
                found = Some(r);
                break;
            }
            if let Some(parent) = dir.parent() {
                dir = parent;
            } else {
                break;
            }
        }
        found.ok_or_else(|| {
            format!(
                "No git repository found in any parent directory of '{}'",
                file
            )
        })?
    };

    // Get workdir path to compute relative paths
    let workdir = repo
        .workdir()
        .ok_or_else(|| "Repository has no workdir (bare repository)".to_string())?
        .to_path_buf();

    // Convert absolute file path to relative from workdir (if possible)
    let rel_path = if file_path.is_absolute() {
        pathdiff::diff_paths(file_path, &workdir).unwrap_or_else(|| file_path.to_path_buf())
    } else {
        file_path.to_path_buf()
    };

    // ── 1. Check if file is untracked ──
    let statuses = repo
        .statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .recurse_untracked_dirs(false)
                .pathspec(&rel_path),
        ))
        .map_err(|e| format!("Failed to get file status: {}", e.message()))?;

    let is_untracked = statuses
        .iter()
        .next()
        .map(|s| {
            let flags = s.status();
            flags.contains(git2::Status::WT_NEW) || flags.contains(git2::Status::IGNORED)
        })
        .unwrap_or(false);

    if is_untracked {
        log_to_app_file(&format!(
            "git_get_file_diff: '{}' is untracked — marking all lines as added",
            file
        ));
        return Ok(GitFileDiff {
            lines: vec![],
            untracked: true,
        });
    }

    // ── 2. Get HEAD tree ──
    let head = repo
        .head()
        .map_err(|e| format!("Failed to get HEAD: {}", e.message()))?;
    let head_tree = head
        .peel_to_tree()
        .map_err(|e| format!("Failed to peel HEAD to tree: {}", e.message()))?;

    // ── 3. Compute diff: HEAD tree → working directory ──
    let mut diff_opts = git2::DiffOptions::new();
    diff_opts
        .pathspec(&rel_path)
        .context_lines(0)
        .ignore_whitespace(false);

    let diff = repo
        .diff_tree_to_workdir(Some(&head_tree), Some(&mut diff_opts))
        .map_err(|e| format!("Failed to diff file: {}", e.message()))?;

    // ── 4. Parse diff → GitDiffLine vector ──
    // Use RefCell for interior mutability since git2 foreach closures are FnMut
    let result: RefCell<Vec<GitDiffLine>> = RefCell::new(Vec::new());
    let deleted_in_hunk: RefCell<Vec<usize>> = RefCell::new(Vec::new());

    diff.foreach(
        &mut |_delta, _progress| true,
        None,
        Some(&mut |_delta, _hunk| {
            // If no lines in new file (pure deletion), deleted_in_hunk will
            // be populated via the line callback. We don't need special handling here
            // since the line callback already accumulates '-'.
            true
        }),
        Some(&mut |_delta, _hunk, line| {
            let line_origin = line.origin();
            let new_lineno = line.new_lineno();

            match line_origin {
                '+' => {
                    if let Some(nl) = new_lineno {
                        let mut deleted = deleted_in_hunk.borrow_mut();
                        if !deleted.is_empty() {
                            let count = deleted.len();
                            result.borrow_mut().push(GitDiffLine {
                                line_number: nl as usize,
                                change_type: "modified".to_string(),
                                deleted_count: count,
                            });
                            deleted.clear();
                        } else {
                            result.borrow_mut().push(GitDiffLine {
                                line_number: nl as usize,
                                change_type: "added".to_string(),
                                deleted_count: 0,
                            });
                        }
                    }
                }
                '-' => {
                    if let Some(_ol) = line.old_lineno() {
                        deleted_in_hunk.borrow_mut().push(1);
                    }
                }
                ' ' => {
                    let mut deleted = deleted_in_hunk.borrow_mut();
                    if !deleted.is_empty() {
                        if let Some(nl) = new_lineno {
                            let count = deleted.len();
                            result.borrow_mut().push(GitDiffLine {
                                line_number: nl as usize,
                                change_type: "deleted_context".to_string(),
                                deleted_count: count,
                            });
                        }
                        deleted.clear();
                    }
                }
                _ => {}
            }
            true
        }),
    )
    .map_err(|e| format!("Failed to iterate diff: {}", e))?;

    // ── 5. Handle trailing deletions at end of file ──
    {
        let mut deleted = deleted_in_hunk.borrow_mut();
        if !deleted.is_empty() {
            result.borrow_mut().push(GitDiffLine {
                line_number: usize::MAX,
                change_type: "deleted_context".to_string(),
                deleted_count: deleted.len(),
            });
            deleted.clear();
        }
    }

    let final_lines = result.into_inner();

    log_to_app_file(&format!(
        "git_get_file_diff: '{}' → {} diff lines",
        file,
        final_lines.len()
    ));

    Ok(GitFileDiff {
        lines: final_lines,
        untracked: false,
    })
}
