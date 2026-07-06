use tauri::Manager;

#[tauri::command]
pub async fn open_browser_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    // Kiem tra xem window da mo chua
    if let Some(window) = app_handle.get_webview_window("mini_browser") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let _window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "mini_browser",
        tauri::WebviewUrl::App("index.html?window=mini-browser".into())
    )
    .title("Alouette Browser")
    .inner_size(1024.0, 768.0)
    .resizable(true)
    .decorations(true)
    .build()
    .map_err(|e| e.to_string())?;

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
