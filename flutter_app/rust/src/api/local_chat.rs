/// Send a message to local chat
pub async fn local_chat_send(message: String) -> Result<String, String> {
    // Placeholder: in production this connects to a local LLM
    Ok(format!("Echo: {message}"))
}

/// Stop local chat generation
pub async fn local_chat_stop() -> Result<(), String> {
    Ok(())
}
