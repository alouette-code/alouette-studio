use core_engine::{AppSettings, ProcessManager};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use tauri::WebviewWindow;
use tokio::sync::Mutex;

fn settings_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("app_data")
        .join("settings.json")
}

/// Check if the application was started with the --minimized flag.
pub fn is_startup_minimized() -> bool {
    std::env::args().any(|arg| arg == "--minimized" || arg == "--tray")
}

/// Initialize all system-level configurations (autostart, background run, close interception)
pub fn init_system(window: &WebviewWindow) {
    let window_clone = window.clone();

    // 1. If started with --minimized, hide the window immediately
    if is_startup_minimized() {
        // We schedule the hide on the main thread to ensure the window is ready
        let w = window.clone();
        let _ = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let _ = w.hide();
        });
        crate::state::log_to_app_file("[AutoStart] App launched minimized — hidden to system tray");
    }

    // 2. Intercept close window event to hide instead of quit
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            let current_settings = AppSettings::load_from_file(settings_path()).unwrap_or_default();
            println!(
                "WINDOW CLOSE REQUESTED! keep_alive: {}",
                current_settings.keep_alive
            );
            if current_settings.keep_alive {
                api.prevent_close();
                println!("Prevented close! Hiding window instead.");
                let _ = window_clone.hide();
            } else {
                println!("keep_alive is false, letting window close.");
            }
        }
    });

    // 3. Spawn background Auto-Restart Monitor Task
    tauri::async_runtime::spawn(async move {
        let start_time = Instant::now();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            let current_settings = AppSettings::load_from_file(settings_path()).unwrap_or_default();
            if current_settings.auto_restart {
                let elapsed_hours = start_time.elapsed().as_secs() as f32 / 3600.0;
                if elapsed_hours >= current_settings.restart_interval_hours as f32 {
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = Command::new(exe).spawn();
                        std::process::exit(0);
                    }
                }
            }
        }
    });
}

/// Spawn background auto-start projects task.
/// Waits 5s for app to fully initialize, then starts each project with 10s interval.
pub fn spawn_auto_start_projects(pm: Arc<Mutex<ProcessManager>>) {
    tauri::async_runtime::spawn(async move {
        // 1. Wait for app to fully initialize (5s)
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let settings = AppSettings::load_from_file(settings_path()).unwrap_or_default();
        if !settings.auto_start_projects {
            return;
        }

        // 2. Get all registered projects
        let project_ids: Vec<String> = {
            let pm = pm.lock().await;
            pm.get_configs().iter().map(|c| c.id.clone()).collect()
        };

        if project_ids.is_empty() {
            crate::state::log_to_app_file(
                "[AutoStartProjects] No projects registered to auto-start.",
            );
            return;
        }

        crate::state::log_to_app_file(&format!(
            "[AutoStartProjects] Starting {} project(s) with 10s interval...",
            project_ids.len()
        ));

        // 3. Start each project with 10s delay
        for (i, project_id) in project_ids.iter().enumerate() {
            if i > 0 {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }

            let mut pm = pm.lock().await;
            match pm.start_process(project_id).await {
                Ok(_) => {
                    crate::state::log_to_app_file(&format!(
                        "[AutoStartProjects] Started project: {}",
                        project_id
                    ));
                }
                Err(e) => {
                    // Check if already running — not an error
                    if e.contains("already running") || e.contains("Already running") {
                        crate::state::log_to_app_file(&format!(
                            "[AutoStartProjects] Project already running: {}",
                            project_id
                        ));
                    } else {
                        crate::state::log_to_app_file(&format!(
                            "[AutoStartProjects] Failed to start project {}: {}",
                            project_id, e
                        ));
                    }
                }
            }
            // Drop the lock before next iteration
            drop(pm);
        }

        crate::state::log_to_app_file("[AutoStartProjects] All projects processed.");
    });
}

/// Build the command-line arguments to append for minimized startup.
fn minimized_args() -> Vec<String> {
    vec!["--minimized".to_string()]
}

/// Build the full command string for autostart entry.
/// Always appends `--minimized` so the app starts silently in the system tray.
fn autostart_command() -> Result<String, String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("Cannot get exe path: {}", e))?;
    let exe_str = exe_path.to_str().ok_or("Exe path is not valid UTF-8")?;
    // Quote the path in case it contains spaces
    let quoted = format!("\"{}\"", exe_str);
    let args = minimized_args();
    Ok(format!("{} {}", quoted, args.join(" ")))
}

/// Configure the OS autostart for the application.
///
/// Works on:
/// - Windows: via Registry HKCU\...\Run
/// - macOS:   via LaunchAgent plist ~/Library/LaunchAgents/
/// - Linux:   via .desktop file ~/.config/autostart/
pub fn configure_autostart(enabled: bool) -> Result<(), String> {
    let cmd_line = autostart_command()?;

    #[cfg(target_os = "windows")]
    {
        configure_autostart_windows(enabled, &cmd_line)?;
    }
    #[cfg(target_os = "macos")]
    {
        configure_autostart_macos(enabled, &cmd_line)?;
    }
    #[cfg(target_os = "linux")]
    {
        configure_autostart_linux(enabled, &cmd_line)?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        return Err("Autostart is not supported on this OS".to_string());
    }

    let action = if enabled { "enabled" } else { "disabled" };
    crate::state::log_to_app_file(&format!("[AutoStart] Successfully {} autostart", action));

    Ok(())
}

// ─── Windows ───────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn configure_autostart_windows(enabled: bool, cmd_line: &str) -> Result<(), String> {
    let reg_key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let value_name = "AlouetteServer";

    if enabled {
        let status = Command::new("reg")
            .args([
                "add", reg_key, "/v", value_name, "/t", "REG_SZ", "/d", cmd_line, "/f",
            ])
            .status()
            .map_err(|e| format!("Failed to execute reg.exe: {}", e))?;

        if !status.success() {
            return Err(format!(
                "reg.exe failed with exit code: {:?}",
                status.code()
            ));
        }
    } else {
        // Only attempt to delete if the registry value exists, to avoid exit code 1 (not found)
        let exists = Command::new("reg")
            .args(["query", reg_key, "/v", value_name])
            .status()
            .map_or(false, |status| status.success());

        if exists {
            let status = Command::new("reg")
                .args(["delete", reg_key, "/v", value_name, "/f"])
                .status()
                .map_err(|e| format!("Failed to execute reg.exe: {}", e))?;

            if !status.success() {
                return Err(format!(
                    "reg.exe failed with exit code: {:?}",
                    status.code()
                ));
            }
        }
    }

    Ok(())
}

// ─── macOS ─────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn configure_autostart_macos(enabled: bool, cmd_line: &str) -> Result<(), String> {
    use std::fs;

    let home = std::env::var("HOME").map_err(|_| "Cannot determine HOME directory".to_string())?;
    let launch_agents_dir = PathBuf::from(&home).join("Library/LaunchAgents");
    let plist_path = launch_agents_dir.join("com.alouette.server.plist");

    if enabled {
        // Split the command line into program and arguments
        let parts: Vec<&str> = cmd_line.split_whitespace().collect();
        let program = parts.first().ok_or("Empty command line")?.trim_matches('"');
        let args: Vec<&str> = parts.iter().skip(1).map(|s| s.trim_matches('"')).collect();

        // Build args XML
        let mut args_xml = String::new();
        for arg in args {
            args_xml.push_str(&format!("        <string>{}</string>\n", arg));
        }

        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.alouette.server</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
{}
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
    <key>ProcessType</key>
    <string>Background</string>
</dict>
</plist>"#,
            program, args_xml
        );

        fs::create_dir_all(&launch_agents_dir)
            .map_err(|e| format!("Failed to create LaunchAgents dir: {}", e))?;
        fs::write(&plist_path, plist_content.as_bytes())
            .map_err(|e| format!("Failed to write plist: {}", e))?;

        // Load the plist with launchctl
        let _ = Command::new("launchctl")
            .args(["load", &plist_path.to_string_lossy()])
            .status();
    } else {
        // Unload and remove plist
        let _ = Command::new("launchctl")
            .args(["unload", &plist_path.to_string_lossy()])
            .status();

        if plist_path.exists() {
            fs::remove_file(&plist_path).map_err(|e| format!("Failed to remove plist: {}", e))?;
        }
    }

    Ok(())
}

// ─── Linux ─────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn configure_autostart_linux(enabled: bool, cmd_line: &str) -> Result<(), String> {
    use std::fs;

    let home = std::env::var("HOME").map_err(|_| "Cannot determine HOME directory".to_string())?;
    let autostart_dir = PathBuf::from(&home).join(".config/autostart");
    let desktop_path = autostart_dir.join("alouette-server.desktop");

    if enabled {
        let desktop_content = format!(
            r#"[Desktop Entry]
Type=Application
Name=Alouette Server
Comment=Alouette Process Runner & Resource Monitor
Exec={}
Terminal=false
Categories=Utility;
X-GNOME-Autostart-enabled=true
"#,
            cmd_line
        );

        fs::create_dir_all(&autostart_dir)
            .map_err(|e| format!("Failed to create autostart dir: {}", e))?;
        fs::write(&desktop_path, desktop_content.as_bytes())
            .map_err(|e| format!("Failed to write .desktop file: {}", e))?;

        // Make executable
        use std::os::unix::fs::PermissionsExt;
        let metadata =
            fs::metadata(&desktop_path).map_err(|e| format!("Failed to get metadata: {}", e))?;
        let mut perms = metadata.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&desktop_path, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    } else {
        if desktop_path.exists() {
            fs::remove_file(&desktop_path)
                .map_err(|e| format!("Failed to remove .desktop file: {}", e))?;
        }
    }

    Ok(())
}
