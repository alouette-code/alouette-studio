use crate::ai_manager::EngineConfig;
use pyo3::prelude::*;
use tauri::AppHandle;

pub async fn start_python_engine(app: AppHandle, config: EngineConfig) -> Result<(), String> {
    println!("[PythonFFI] Preparing to start Python engine: {}", config.engine_id);
    
    // 1. Ensure venv exists and packages are installed
    let venv_path = super::python_env::ensure_python_venv(&app).await?;
    
    // 2. We do NOT set PYTHONHOME to the venv because it breaks standard library imports (like 'encodings').
    // Instead, we will inject the venv's site-packages directly into sys.path inside Python.
    let site_packages = if cfg!(windows) {
        venv_path.join("Lib").join("site-packages")
    } else {
        // Dynamically find python3.x folder
        let lib_dir = venv_path.join("lib");
        let mut py_version_dir = lib_dir.join("python3.12").join("site-packages"); // Default fallback
        if let Ok(entries) = std::fs::read_dir(&lib_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("python") && entry.path().is_dir() {
                    py_version_dir = entry.path().join("site-packages");
                    break;
                }
            }
        }
        py_version_dir
    };

    // 3. Initialize PyO3 (runs in a blocking thread since PyO3 is sync)
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        // Ensure PYTHONHOME is NOT set so we don't break the base interpreter
        std::env::remove_var("PYTHONHOME");
        
        // Initialize the Python interpreter
        pyo3::prepare_freethreaded_python();
        
        Python::with_gil(|py| -> PyResult<()> {
            println!("[PythonFFI] Python GIL acquired. Testing PyTorch environment...");
            
            // Inject the venv site-packages path
            let sys = py.import_bound("sys")?;
            let path = sys.getattr("path")?;
            let site_packages_str = site_packages.to_string_lossy().to_string();
            // Insert at index 0 to prioritize our venv packages
            path.call_method1("insert", (0, site_packages_str.clone()))?;
            println!("[PythonFFI] Injected Venv path into sys.path: {}", site_packages_str);

            // Execute a simple Python script to verify the environment
            let code = r#"
import sys
import torch
print(f"[Python] Running on Python {sys.version}")
print(f"[Python] PyTorch Version: {torch.__version__}")
print(f"[Python] CUDA Available: {torch.cuda.is_available()}")
"#;
            
            let _ = py.run_bound(code, None, None)?;

            println!("[PythonFFI] Successfully verified Python Venv FFI integration.");

            // In the real system, depending on config.engine_id:
            // if config.engine_id == "vllm" {
            //    let vllm = py.import("vllm")?; ...
            // }

            Ok(())
        }).map_err(|e| format!("Python FFI error: {:?}", e))?;
        
        Ok(())
    }).await.map_err(|e| e.to_string())??;

    Ok(())
}
