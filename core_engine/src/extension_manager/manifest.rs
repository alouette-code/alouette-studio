use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublisherInfo {
    pub id: String, // e.g. "nhatanh"
    pub name: String, // e.g. "Nhất Anh"
    pub public_key: Option<String>, // Ed25519 hex encoded public key
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtensionManifest {
    pub id: String, // e.g. "nhatanh.code-formatter"
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub publisher: Option<PublisherInfo>,
    pub repository: Option<String>,
    pub icon: Option<String>,
    pub readme_url: Option<String>,
    pub sha256: Option<String>,
    pub signature: Option<String>, // Ed25519 digital signature of SHA-256 hash
    pub runtime: ExtensionRuntime,
    pub capabilities: Option<ExtensionCapabilities>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtensionRuntime {
    pub r#type: String, // "wasm" or "legacy_process"
    pub entry: String, // e.g. "plugin.wasm"
    pub wasm_entry: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ExtensionCapabilities {
    #[serde(default)]
    pub permissions: Vec<String>, // e.g. ["fs:read", "fs:write", "net:http", "terminal:exec"]
}
