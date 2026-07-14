use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub runtime: ExtensionRuntime,
    pub capabilities: Option<ExtensionCapabilities>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtensionRuntime {
    pub r#type: String,
    pub entry: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ExtensionCapabilities {
    #[serde(default)]
    pub permissions: Vec<String>,
}
