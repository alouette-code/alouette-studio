use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::io::AsyncWriteExt;
use anyhow::{anyhow, Result, Context};
use super::protocol::JsonRpcRequest;
use super::manifest::ExtensionManifest;
use super::wasm_engine::WasmExtensionEngine;
use std::path::PathBuf;
use std::fs;

pub struct ExtensionProcess {
    pub id: String,
    child: Child,
}

impl ExtensionProcess {
    /// Khởi tạo một extension process mới (Legacy Process Fallback)
    pub fn spawn(id: &str, command: &str, args: &[&str]) -> Result<Self> {
        let child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to spawn extension process")?;

        Ok(Self {
            id: id.to_string(),
            child,
        })
    }

    /// Gửi một request JSON-RPC đến extension (ví dụ non-blocking)
    pub async fn send_request(&mut self, request: &JsonRpcRequest) -> Result<()> {
        let stdin = self.child.stdin.as_mut().context("Failed to get stdin")?;
        let mut msg = serde_json::to_string(request)?;
        msg.push('\n');
        stdin.write_all(msg.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }
}

pub struct ExtensionRegistry {
    pub extensions_dir: PathBuf,
    pub wasm_engine: WasmExtensionEngine,
}

impl ExtensionRegistry {
    pub fn new(extensions_dir: PathBuf) -> Self {
        if !extensions_dir.exists() {
            let _ = fs::create_dir_all(&extensions_dir);
        }
        let wasm_engine = WasmExtensionEngine::new().expect("Failed to initialize Wasm engine");
        Self { extensions_dir, wasm_engine }
    }

    pub fn scan_extensions(&self) -> Vec<ExtensionManifest> {
        let mut extensions = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.extensions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest_path = path.join("proto-extension.json");
                    if manifest_path.exists() {
                        if let Ok(content) = fs::read_to_string(manifest_path) {
                            if let Ok(manifest) = serde_json::from_str::<ExtensionManifest>(&content) {
                                extensions.push(manifest);
                            }
                        }
                    }
                }
            }
        }
        extensions
    }

    /// Khởi chạy Wasm extension từ thư mục cài đặt local
    pub async fn run_wasm_extension(
        &self,
        extension_id: &str,
        function_name: &str,
        param_json: &str,
    ) -> Result<String> {
        let ext_dir = self.extensions_dir.join(extension_id);
        let manifest_path = ext_dir.join("proto-extension.json");

        if !manifest_path.exists() {
            return Err(anyhow!("Extension manifest not found for ID: {}", extension_id));
        }

        let manifest_content = fs::read_to_string(&manifest_path)?;
        let manifest: ExtensionManifest = serde_json::from_str(&manifest_content)?;

        let wasm_file_name = manifest.runtime.wasm_entry.clone()
            .unwrap_or_else(|| manifest.runtime.entry.clone());
        let wasm_path = ext_dir.join(&wasm_file_name);

        if !wasm_path.exists() {
            return Err(anyhow!("Wasm binary missing at: {:?}", wasm_path));
        }

        // Checksum validation nếu manifest yêu cầu
        if let Some(expected_sha256) = &manifest.sha256 {
            let is_valid = WasmExtensionEngine::verify_sha256(&wasm_path, expected_sha256)?;
            if !is_valid {
                return Err(anyhow!("SHA-256 checksum mismatch for Wasm binary of {}", extension_id));
            }
        }

        let wasm_bytes = fs::read(&wasm_path)?;
        let permissions = manifest.capabilities.map(|c| c.permissions).unwrap_or_default();

        self.wasm_engine.execute_plugin(extension_id, &wasm_bytes, &permissions, function_name, param_json).await
    }
}
