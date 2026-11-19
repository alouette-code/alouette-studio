use tauri::Manager;

fn resolve_chrome_binary(app_handle: &tauri::AppHandle) -> Result<String, String> {
    if let Ok(env_path) = std::env::var("ALOUETTE_CHROME_PATH") {
        let p = std::path::PathBuf::from(&env_path);
        if p.exists() {
            return Ok(env_path);
        }
    }

    let bin_name = if cfg!(target_os = "windows") {
        "chrome.exe"
    } else {
        "chrome"
    };

    if let Ok(res_dir) = app_handle.path().resource_dir() {
        let bundled = res_dir.join("chrome").join(bin_name);
        if bundled.exists() {
            return Ok(bundled.to_string_lossy().to_string());
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let sibling = exe_dir.join("chrome").join(bin_name);
            if sibling.exists() {
                return Ok(sibling.to_string_lossy().to_string());
            }
        }
    }

    let dev_local = std::path::PathBuf::from("chrome").join(bin_name);
    if dev_local.exists() {
        return Ok(dev_local.to_string_lossy().to_string());
    }

    #[cfg(target_os = "linux")]
    let candidates = ["google-chrome", "chromium", "chromium-browser"];
    #[cfg(target_os = "macos")]
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ];
    #[cfg(target_os = "windows")]
    let candidates = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];

    for candidate in candidates {
        if std::path::Path::new(candidate).exists() {
            return Ok(candidate.to_string());
        }
        #[cfg(not(target_os = "windows"))]
        if let Ok(out) = std::process::Command::new("which").arg(candidate).output() {
            if out.status.success() {
                let resolved = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !resolved.is_empty() {
                    return Ok(resolved);
                }
            }
        }
    }

    Err("Không tìm thấy trình duyệt Chrome/Chromium. Vui lòng cài Chrome hoặc đặt biến ALOUETTE_CHROME_PATH.".to_string())
}

#[tauri::command]
pub async fn open_browser_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    let chrome_path = resolve_chrome_binary(&app_handle)?;
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("alouette_studio"));

    let profile_path = app_data_dir.join("chrome_profile");
    let log_path = app_data_dir.join("chrome_err.log");

    // Xóa sạch dữ liệu trình duyệt (Incognito/Fresh) mỗi lần mở
    let _ = std::fs::remove_dir_all(&profile_path);
    std::fs::create_dir_all(&profile_path)
        .map_err(|e| format!("Không thể tạo profile directory: {}", e))?;

    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| format!("Không thể tạo file log Chrome: {}", e))?;
    let log_stdout = log_file
        .try_clone()
        .map_err(|e| format!("Không thể clone file log Chrome: {}", e))?;

    let mut cmd = std::process::Command::new(&chrome_path);
    cmd.env_clear();

    let keep_envs = [
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XDG_RUNTIME_DIR",
        "HOME",
        "USER",
        "XAUTHORITY",
        "DBUS_SESSION_BUS_ADDRESS",
        "PATH",
    ];
    for key in keep_envs.iter() {
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }

    cmd.arg(format!("--user-data-dir={}", profile_path.display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--no-sandbox")
        .arg("--disable-dev-shm-usage")
        .arg("--remote-debugging-port=9222")
        .stdout(log_stdout)
        .stderr(log_file)
        .spawn()
        .map_err(|e| format!("Failed to launch Google Chrome ({}): {}", chrome_path, e))?;

    Ok(())
}
