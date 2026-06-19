use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub id: String,
    pub name: String,
    pub cpu_cores: u32,
    pub ram_size_mb: u64,
    pub kernel_path: Option<String>,
    pub initrd_path: Option<String>,
    pub boot_args: Option<String>,
    pub disk_path: Option<String>,
    pub network_mode: String,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            name: "Default-VM".to_string(),
            cpu_cores: 1,
            ram_size_mb: 512,
            kernel_path: None,
            initrd_path: None,
            boot_args: Some("console=ttyS0 quiet panic=1".to_string()),
            disk_path: None,
            network_mode: "nat".to_string(),
        }
    }
}
