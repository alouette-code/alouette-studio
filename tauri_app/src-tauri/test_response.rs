use tauri::ipc::Response;
#[tauri::command]
fn test_resp() -> Result<Response, String> {
    Ok(Response::new(vec![1, 2, 3]))
}
