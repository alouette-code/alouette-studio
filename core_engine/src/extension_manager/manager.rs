use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::io::AsyncWriteExt;
use anyhow::{Result, Context};
use super::protocol::JsonRpcRequest;

pub struct ExtensionProcess {
    pub id: String,
    child: Child,
}

impl ExtensionProcess {
    /// Khởi tạo một extension process mới
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
        msg.push('\n'); // Phân tách bằng newline
        stdin.write_all(msg.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }
}

use std::path::PathBuf;
use std::fs;
use super::manifest::ExtensionManifest;

pub struct ExtensionRegistry {
    pub extensions_dir: PathBuf,
}

impl ExtensionRegistry {
    pub fn new(extensions_dir: PathBuf) -> Self {
        if !extensions_dir.exists() {
            let _ = fs::create_dir_all(&extensions_dir);
        }
        Self { extensions_dir }
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
}

