use crate::ai_manager::EngineConfig;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;
use tauri::AppHandle;

lazy_static::lazy_static! {
    static ref OLLAMA_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
}

pub async fn start_ollama(_app: AppHandle, config: EngineConfig) -> Result<(), String> {
    println!("[Ollama] Starting engine with config: {:?}", config);
    
    let url = format!("http://{}:{}/api/tags", config.api_host, config.port);
    let client = reqwest::Client::new();
    
    // Check if it's already running
    match client.get(&url).send().await {
        Ok(res) if res.status().is_success() => {
            println!("[Ollama] Server is already running natively. Attaching to it.");
            return Ok(());
        }
        _ => {
            println!("[Ollama] Server not running. Attempting to spawn `ollama serve`...");
        }
    }

    let host_env = format!("{}:{}", config.api_host, config.port);
    let child = Command::new("ollama")
        .arg("serve")
        .env("OLLAMA_HOST", &host_env)
        .spawn()
        .map_err(|e| format!("Failed to spawn Ollama process. Make sure Ollama is installed on your system. Error: {}", e))?;

    {
        let mut process_guard = OLLAMA_PROCESS.lock().unwrap();
        *process_guard = Some(child);
    }
    
    // Wait for it to become ready
    let mut ready = false;
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Ok(res) = client.get(&url).send().await {
            if res.status().is_success() {
                ready = true;
                break;
            }
        }
    }

    if !ready {
        stop_ollama();
        return Err("Ollama process spawned but API is not responding after 5 seconds.".to_string());
    }

    println!("[Ollama] Successfully started and verified.");
    Ok(())
}

pub fn stop_ollama() {
    let mut process_guard = OLLAMA_PROCESS.lock().unwrap();
    if let Some(mut child) = process_guard.take() {
        println!("[Ollama] Stopping child process...");
        let _ = child.kill();
        let _ = child.wait();
    }
}
