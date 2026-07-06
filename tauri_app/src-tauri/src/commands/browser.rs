use tauri::Manager;

#[tauri::command]
pub async fn open_browser_window(_app_handle: tauri::AppHandle) -> Result<(), String> {
    let zen_path = "/home/nhatanh/projet/alouette_studio/zen browser/zen-browser.AppImage";
    let profile_path = "/home/nhatanh/projet/alouette_studio/zen browser/fresh_profile";
    
    // Xóa profile cũ để mọi dữ liệu, cache, cookie bị dọn sạch
    let _ = std::fs::remove_dir_all(profile_path);
    // Tạo lại thư mục profile trống
    let _ = std::fs::create_dir_all(profile_path);
    
    std::process::Command::new(zen_path)
        .arg("--profile")
        .arg(profile_path)
        .spawn()
        .map_err(|e| format!("Failed to launch Zen Browser: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn navigate_webview(app_handle: tauri::AppHandle, url: String) -> Result<(), String> {
    if let Some(webview) = app_handle.get_webview("browser-content") {
        let js = format!("window.location.href = '{}';", url.replace("'", "\\'"));
        webview.eval(&js).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Webview browser-content not found".into())
    }
}
