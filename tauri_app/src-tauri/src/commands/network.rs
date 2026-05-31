use tauri::Manager;

// ============================================================================
// Commands
// ============================================================================

#[tauri::command]
pub async fn check_port_status(port: u16) -> Option<u32> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("netstat").args(["-ano", "-p", "tcp"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let port_suffix_colon = format!(":{}", port);

            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let local_addr = parts[1];
                    let state = parts[3];
                    let pid_str = parts[4];

                    if (local_addr.ends_with(&port_suffix_colon)
                        || local_addr.ends_with(&format!("]{}", port_suffix_colon)))
                        && state == "LISTENING"
                    {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            if pid > 0 {
                                return Some(pid);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("lsof")
            .args(["-t", &format!("-i:{}", port)])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = stdout.lines().next() {
                if let Ok(pid) = first_line.trim().parse::<u32>() {
                    return Some(pid);
                }
            }
        }
    }

    None
}

#[tauri::command]
pub async fn force_kill_process(pid: u32) -> Result<(), String> {
    core_engine::terminate_process_tree(pid).await;
    Ok(())
}

#[tauri::command]
pub async fn open_ping_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("ping_window") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let _window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "ping_window",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Mini Postman - Connection Diagnostics")
    .inner_size(1100.0, 780.0)
    .resizable(true)
    .decorations(false)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Admin Window
// ============================================================================

/// Open (or focus) a standalone Admin / Settings window with left dock navigation.
#[tauri::command]
pub async fn open_admin_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("admin_window") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let _window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "admin_window",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Admin - Settings & Management")
    .inner_size(1100.0, 750.0)
    .min_inner_size(800.0, 500.0)
    .resizable(true)
    .decorations(false)
    .build()
    .map_err(|e: tauri::Error| e.to_string())?;

    Ok(())
}
