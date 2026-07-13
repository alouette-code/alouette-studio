use core_engine::AppSettings;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use tauri::WebviewWindow;

fn settings_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("app_data")
        .join("settings.json")
}

/// Initialize all system-level configurations (autostart, background run, close interception)
pub fn init_system(window: &WebviewWindow) {
    let window_clone = window.clone();
    // Intercept close window event to hide instead of quit
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

    // Spawn background Auto-Restart Monitor Task
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

/// Configure the OS autostart for the application on Windows
pub fn configure_autostart(_enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        if _enabled {
            let _ = Command::new("reg")
                .args(&[
                    "add",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "AlouetteServer",
                    "/t",
                    "REG_SZ",
                    "/d",
                    exe_path.to_str().unwrap_or(""),
                    "/f",
                ])
                .status();
        } else {
            let _ = Command::new("reg")
                .args(&[
                    "delete",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "AlouetteServer",
                    "/f",
                ])
                .status();
        }
    }
    Ok(())
}
use std::panic;
use std::fs::OpenOptions;
use std::io::Write;

use chrono::Local;

/// Bắt mọi Panic phát sinh từ các luồng (thread) trong ứng dụng.
/// Thay vì crash thẳng, nó sẽ ghi lại stack trace vào log và có thể gọi một hàm dọn dẹp.
pub fn setup_panic_hook(log_dir: PathBuf) {
    panic::set_hook(Box::new(move |panic_info| {
        // Trích xuất thông tin lỗi (vị trí file, dòng, thông báo)
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic"
        };

        let location = panic_info.location().unwrap();
        
        let crash_message = format!(
            "[{}] CRASH DETECTED at {}:{}:{} - Panic Payload: {}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            location.file(),
            location.line(),
            location.column(),
            payload
        );

        // Ghi vào terminal
        eprintln!("\n================ FATAL ERROR ================");
        eprintln!("{}", crash_message);
        eprintln!("=============================================\n");

        // Ghi vào file log khẩn cấp
        let crash_log_path = log_dir.join("crash_report.log");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_log_path)
        {
            let _ = file.write_all(crash_message.as_bytes());
        }

        // Tùy chọn: Ở đây có thể gửi sự kiện (Event) lên Frontend để báo UI hiển thị "Ứng dụng gặp lỗi nội bộ".
    }));
}
