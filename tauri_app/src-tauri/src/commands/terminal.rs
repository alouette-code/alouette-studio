use crate::state::AppState;
use serde::Serialize;
use tauri::State;
use core_engine::TerminalOutput;

#[derive(Serialize)]
pub struct TerminalSessionInfo {
    pub exists: bool,
    pub pid: Option<u32>,
}

#[tauri::command]
pub async fn spawn_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
    cwd: Option<String>,
    block_internet: Option<bool>,
) -> Result<(), String> {
    let mut pm = state.process_manager.lock().await;
    pm.spawn_terminal(&session_id, cwd.as_deref(), block_internet.unwrap_or(false))
        .await?;
    Ok(())
}

fn vietnamese_to_telex(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 2);
    for c in input.chars() {
        let mapped = match c {
            'đ' => "dd", 'Đ' => "DD",
            'â' => "aa", 'Â' => "AA",
            'ê' => "ee", 'Ê' => "EE",
            'ô' => "oo", 'Ô' => "OO",
            'ă' => "aw", 'Ă' => "AW",
            'ơ' => "ow", 'Ơ' => "OW",
            'ư' => "uw", 'Ư' => "UW",
            'á' => "as", 'Á' => "AS",
            'à' => "af", 'À' => "AF",
            'ả' => "ar", 'Ả' => "AR",
            'ã' => "ax", 'Ã' => "AX",
            'ạ' => "aj", 'Ạ' => "AJ",
            'é' => "es", 'É' => "ES",
            'è' => "ef", 'È' => "EF",
            'ẻ' => "er", 'Ẻ' => "ER",
            'ẽ' => "ex", 'Ẽ' => "EX",
            'ẹ' => "ej", 'Ẹ' => "EJ",
            'í' => "is", 'Í' => "IS",
            'ì' => "if", 'Ì' => "IF",
            'ỉ' => "ir", 'Ỉ' => "IR",
            'ĩ' => "ix", 'Ĩ' => "IX",
            'ị' => "ij", 'Ị' => "IJ",
            'ó' => "os", 'Ó' => "OS",
            'ò' => "of", 'Ò' => "OF",
            'ỏ' => "or", 'Ỏ' => "OR",
            'õ' => "ox", 'Õ' => "OX",
            'ọ' => "oj", 'Ọ' => "OJ",
            'ú' => "us", 'Ú' => "US",
            'ù' => "uf", 'Ù' => "UF",
            'ủ' => "ur", 'Ủ' => "UR",
            'ũ' => "ux", 'Ũ' => "UX",
            'ụ' => "uj", 'Ụ' => "UJ",
            'ý' => "ys", 'Ý' => "YS",
            'ỳ' => "yf", 'Ỳ' => "YF",
            'ỷ' => "yr", 'Ỷ' => "YR",
            'ỹ' => "yx", 'Ỹ' => "YX",
            'ỵ' => "yj", 'Ỵ' => "YJ",
            'ấ' => "aas", 'Ấ' => "AAS",
            'ầ' => "aaf", 'Ầ' => "AAF",
            'ẩ' => "aar", 'Ẩ' => "AAR",
            'ẫ' => "aax", 'Ẫ' => "AAX",
            'ậ' => "aaj", 'Ậ' => "AAJ",
            'ắ' => "aws", 'Ắ' => "AWS",
            'ằ' => "awf", 'Ằ' => "AWF",
            'ẳ' => "awr", 'Ẳ' => "AWR",
            'ẵ' => "awx", 'Ẵ' => "AWX",
            'ặ' => "awj", 'Ặ' => "AWJ",
            'ế' => "ees", 'Ế' => "EES",
            'ề' => "eef", 'Ề' => "EEF",
            'ể' => "eer", 'Ể' => "EER",
            'ễ' => "eex", 'Ễ' => "EEX",
            'ệ' => "eej", 'Ệ' => "EEJ",
            'ố' => "oos", 'Ố' => "OOS",
            'ồ' => "oof", 'Ồ' => "OOF",
            'ổ' => "oor", 'Ổ' => "OOR",
            'ỗ' => "oox", 'Ỗ' => "OOX",
            'ộ' => "ooj", 'Ộ' => "OOJ",
            'ớ' => "ows", 'Ớ' => "OWS",
            'ờ' => "owf", 'Ờ' => "OWF",
            'ở' => "owr", 'Ở' => "OWR",
            'ỡ' => "owx", 'Ỡ' => "OWX",
            'ợ' => "owj", 'Ợ' => "OWJ",
            'ứ' => "uws", 'Ứ' => "UWS",
            'ừ' => "uwf", 'Ừ' => "UWF",
            'ử' => "uwr", 'Ử' => "UWR",
            'ữ' => "uwx", 'Ữ' => "UWX",
            'ự' => "uwj", 'Ự' => "UWJ",
            '\u{0300}' | '\u{0301}' | '\u{0309}' | '\u{0303}' | '\u{0323}' | '\u{0302}' | '\u{0306}' | '\u{031b}' => "",
            other => {
                result.push(other);
                continue;
            }
        };
        result.push_str(mapped);
    }
    result
}

#[tauri::command]
pub async fn sync_terminal_input_buf(
    state: State<'_, AppState>,
    session_id: String,
    current_input: String,
) -> Result<(), String> {
    let mut pm = state.process_manager.lock().await;
    let filtered_input = vietnamese_to_telex(&current_input);
    pm.input_buf.insert(session_id, filtered_input);
    Ok(())
}

#[tauri::command]
pub async fn write_to_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
    input: String,
) -> Result<(), String> {
    let input = vietnamese_to_telex(&input);
    eprintln!("[term-input] Received input: {:?}", input);
    // 1. Immediately handle Ctrl+C to interrupt executing commands safely
    if input.contains('\x03') {
        let mut pm = state.process_manager.lock().await;
        pm.clear_input_buf(&session_id);
        if let Ok(ctx) = pm.get_terminal_write_context(&session_id) {
            let stdin_tx = ctx.stdin_sender.clone();
            drop(pm);
            let _ = stdin_tx.send("\x03".to_string()).await;
        }
        return Ok(());
    }

    // 2. Handle Up/Down arrow keys for command history navigation
    if input == "\x1b[A" || input == "\x1b[B" {
        let mut pm = state.process_manager.lock().await;
        if let Ok(ctx) = pm.get_terminal_write_context(&session_id) {
            let out_tx = ctx.terminal_sender.clone();
            let history = pm.terminal_history.entry(session_id.clone()).or_default().clone();
            let mut idx = *pm.terminal_history_index.entry(session_id.clone()).or_insert(history.len());

            if !history.is_empty() {
                if input == "\x1b[A" {
                    // Up Arrow
                    if idx > 0 {
                        idx -= 1;
                    }
                } else {
                    // Down Arrow
                    if idx < history.len() {
                        idx += 1;
                    }
                }
                pm.terminal_history_index.insert(session_id.clone(), idx);

                let new_cmd = if idx < history.len() {
                    history[idx].clone()
                } else {
                    String::new()
                };

                let cur_len = pm.input_buf.get(&session_id).map(|s| s.len()).unwrap_or(0);
                let erase = "\x08 \x08".repeat(cur_len);

                pm.input_buf.insert(session_id.clone(), new_cmd.clone());

                let _ = out_tx.send(core_engine::TerminalOutput {
                    session_id,
                    text: format!("{}{}", erase, new_cmd),
                });
            }
        }
        return Ok(());
    }

    let is_enter = input.contains('\r') || input.contains('\n');
    let mut allowed_cmd = String::new();
    let mut blocked_reason = String::new();
    let mut buf_len: usize = 0;
    let out_tx;
    let stdin_tx;

    {
        let mut pm = state.process_manager.lock().await;

        if is_enter {
            // Normalize cd shortcuts in the input buffer first
            if let Some(buf) = pm.input_buf.get_mut(&session_id) {
                let trimmed = buf.trim();
                let lower = trimmed.to_lowercase();
                if lower.starts_with("cd..") {
                    *buf = format!("cd ..{}", &trimmed[4..]);
                } else if lower.starts_with("cd/") {
                    *buf = format!("cd /{}", &trimmed[3..]);
                } else if lower.starts_with("cd\\") {
                    *buf = format!("cd \\{}", &trimmed[3..]);
                }
            }

            match pm.check_input_sandbox(&session_id) {
                Ok(None) => {
                    allowed_cmd = pm.get_input_buf(&session_id).cloned().unwrap_or_default();
                    pm.clear_input_buf(&session_id);

                    // Add command to history on successful enter
                    if !allowed_cmd.trim().is_empty() {
                        let len = {
                            let hist = pm.terminal_history.entry(session_id.clone()).or_default();
                            if hist.last() != Some(&allowed_cmd) {
                                hist.push(allowed_cmd.clone());
                            }
                            hist.len()
                        };
                        pm.terminal_history_index.insert(session_id.clone(), len);
                    }
                }
                Ok(Some(reason)) => {
                    blocked_reason = reason;
                    buf_len = pm.get_input_buf(&session_id).map(|s| s.len()).unwrap_or(0);
                    pm.clear_input_buf(&session_id);
                }
                Err(e) => {
                    eprintln!("[sandbox] error: {e}");
                    pm.clear_input_buf(&session_id);
                }
            }
        }

        let ctx = match pm.get_terminal_write_context(&session_id) {
            Ok(c) => c,
            Err(_) => return Ok(()), // Return gracefully if session no longer exists
        };
        out_tx = ctx.terminal_sender.clone();
        stdin_tx = ctx.stdin_sender.clone();
    }

    if !blocked_reason.is_empty() {
        // Send Ctrl+C to PowerShell/Bash to cancel input line
        let _ = stdin_tx.send("\x03".to_string()).await;
        // Send blocked warning
        let warning = format!("\r[Sandbox] Blocked: {}\r\n", blocked_reason);
        let _ = out_tx.send(TerminalOutput {
            session_id: session_id.clone(),
            text: warning,
        });
        return Ok(());
    }

    if is_enter {
        {
            let mut pm = state.process_manager.lock().await;
            if !allowed_cmd.is_empty() {
                pm.update_cwd_for_cd(&session_id, &allowed_cmd);
            }
        }
        let _ = stdin_tx.send("\r".to_string()).await;
        return Ok(());
    }

    // Direct forward to PTY for typing/backspacing
    let _ = stdin_tx.send(input).await;

    Ok(())
}

#[tauri::command]
pub async fn kill_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let mut pm = state.process_manager.lock().await;
    pm.kill_terminal(&session_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn check_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<TerminalSessionInfo, String> {
    let pm = state.process_manager.lock().await;
    if let Some(session) = pm.terminal_sessions.get(&session_id) {
        Ok(TerminalSessionInfo {
            exists: true,
            pid: Some(session.pid),
        })
    } else {
        Ok(TerminalSessionInfo {
            exists: false,
            pid: None,
        })
    }
}

#[tauri::command]
pub async fn resize_terminal_session(
    state: State<'_, AppState>,
    session_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let pm = state.process_manager.lock().await;
    pm.resize_terminal(&session_id, rows, cols)?;
    Ok(())
}
