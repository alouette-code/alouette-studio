use tauri::Manager;

#[tauri::command]
pub async fn open_browser_window(_app_handle: tauri::AppHandle) -> Result<(), String> {
    let chrome_path = "/home/nhatanh/projet/alouette_studio/chrome/chrome";
    let profile_path = "/home/nhatanh/projet/alouette_studio/chrome_profile";
    
    // Tính năng: Xóa sạch dữ liệu trình duyệt (Incognito/Fresh)
    // Mỗi khi bấm mở, ta xóa hẳn thư mục profile cũ để quét sạch lịch sử, cache, cookie
    let _ = std::fs::remove_dir_all(profile_path);
    if let Err(e) = std::fs::create_dir_all(profile_path) {
        return Err(format!("Không thể tạo profile directory: {}", e));
    }
    
    let log_file = std::fs::File::create("/home/nhatanh/projet/alouette_studio/chrome_err.log").unwrap_or_else(|_| std::fs::File::create("chrome_err.log").unwrap());
    
    let mut cmd = std::process::Command::new(chrome_path);
    cmd.env_clear(); // Xóa sạch biến môi trường độc hại từ Tauri
    
    // Giữ lại các biến môi trường thiết yếu
    let keep_envs = ["DISPLAY", "WAYLAND_DISPLAY", "XDG_RUNTIME_DIR", "HOME", "USER", "XAUTHORITY", "DBUS_SESSION_BUS_ADDRESS", "PATH"];
    for key in keep_envs.iter() {
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }

    cmd.arg(format!("--user-data-dir={}", profile_path))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--no-sandbox")
        .arg("--disable-dev-shm-usage")
        .arg("--remote-debugging-port=9222") // Cho phép AI Agent dễ dàng kết nối và điều khiển qua cổng 9222
        .stdout(log_file.try_clone().unwrap())
        .stderr(log_file)
        .spawn()
        .map_err(|e| format!("Failed to launch Google Chrome: {}", e))?;

    Ok(())
}
