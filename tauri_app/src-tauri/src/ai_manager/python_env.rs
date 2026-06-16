use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Manager};

/// Ensures that a Python virtual environment exists in the app's data directory.
/// If it doesn't exist, it creates one and installs the necessary packages via pip.
pub async fn ensure_python_venv(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let venv_dir = app_dir.join("venv");

    if !venv_dir.exists() {
        println!("[PythonEnv] Venv not found. Creating new venv at: {:?}", venv_dir);
        
        // Ensure parent directory exists
        let _ = std::fs::create_dir_all(&app_dir);

        // 1. Create the virtual environment
        let python_exec = if cfg!(windows) { "python" } else { "python3" };
        let create_status = Command::new(python_exec)
            .arg("-m")
            .arg("venv")
            .arg(&venv_dir)
            .status()
            .map_err(|e| format!("Failed to execute venv command: {}", e))?;

        if !create_status.success() {
            return Err("Failed to create Python virtual environment. Please ensure Python 3 is installed on your system.".into());
        }
        
        println!("[PythonEnv] Venv created. Installing required ML packages...");

        // 2. Determine pip path inside venv
        let pip_path = if cfg!(windows) {
            venv_dir.join("Scripts").join("pip")
        } else {
            venv_dir.join("bin").join("pip")
        };

        // 3. Install packages (vllm, tensorrt_llm, exllamav2)
        // Note: In a real world scenario, these installations can be huge and take a long time.
        // We run this in the background or show progress to the user.
        let install_status = Command::new(&pip_path)
            .args(["install", "torch", "transformers", "accelerate", "fastapi", "uvicorn"])
            // We install the core PyTorch packages first as foundation. 
            // vllm and others can be very heavy so we start with the basics.
            .status()
            .map_err(|e| format!("Failed to execute pip install: {}", e))?;

        if !install_status.success() {
            return Err("Failed to install basic Python dependencies.".into());
        }

        println!("[PythonEnv] Successfully created venv and installed core packages.");
    } else {
        println!("[PythonEnv] Venv already exists at: {:?}", venv_dir);
    }

    Ok(venv_dir)
}
