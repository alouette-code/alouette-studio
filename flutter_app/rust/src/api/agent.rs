/// Send a message to the AI agent
pub async fn agent_send_message(msg: String) -> Result<String, String> {
    Ok(format!("AI agent received: {msg}"))
}

/// Approve or reject a tool call
pub async fn agent_approve_tool(_approve: bool, _reason: Option<String>) -> Result<(), String> {
    Ok(())
}

/// Reset the agent session
pub async fn agent_reset_session() -> Result<(), String> {
    Ok(())
}

/// Cancel the current agent operation
pub async fn agent_cancel() -> Result<(), String> {
    Ok(())
}

/// Get agent status
pub async fn agent_status() -> Result<String, String> {
    Ok("idle".to_string())
}

/// Get agent history list
pub async fn agent_get_history() -> Result<Vec<String>, String> {
    Ok(vec![])
}

/// Load a specific agent session
pub async fn load_agent_session(_session_id: String) -> Result<String, String> {
    Ok(format!("Session {_session_id}"))
}

/// Save an agent session
pub async fn save_agent_session(_session_id: String, _data: String) -> Result<(), String> {
    Ok(())
}

/// Delete an agent session
pub async fn agent_delete_session(_session_id: String) -> Result<(), String> {
    Ok(())
}

/// Get custom AI config
pub async fn get_custom_ai_config() -> Result<String, String> {
    Ok("{}".to_string())
}

/// Save custom AI config
pub async fn save_custom_ai_config(_config: String) -> Result<(), String> {
    Ok(())
}

/// Switch active project for the agent
pub async fn switch_agent_project(_project_id: String) -> Result<(), String> {
    Ok(())
}

/// Load history page
pub async fn load_history_page(_page: i32, _page_size: i32) -> Result<Vec<String>, String> {
    Ok(vec![])
}
