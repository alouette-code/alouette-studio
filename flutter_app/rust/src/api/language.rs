/// Get available language runtimes
pub fn get_language_runtimes() -> Vec<RuntimeInfo> {
    vec![
        RuntimeInfo { name: "node".into(), version: "latest".into() },
        RuntimeInfo { name: "go".into(), version: "latest".into() },
        RuntimeInfo { name: "python".into(), version: "latest".into() },
        RuntimeInfo { name: "rust".into(), version: "latest".into() },
    ]
}

#[derive(serde::Serialize)]
pub struct RuntimeInfo {
    pub name: String,
    pub version: String,
}
