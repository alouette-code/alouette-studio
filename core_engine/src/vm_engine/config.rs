use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub id: String,
    pub name: String,
    pub os_type: Option<String>,
    pub cpu_cores: u32,
    pub ram_size_mb: u64,
    pub vm_dir: String,
    pub iso_path: Option<String>,
    pub disk_path: Option<String>,
    pub disk_size_gb: Option<u32>,
    pub network_mode: String,
    pub firmware: Option<String>, // "bios" or "uefi"
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            name: "Default-VM".to_string(),
            cpu_cores: 1,
            ram_size_mb: 512,
            vm_dir: "".to_string(),
            iso_path: None,
            disk_path: None,
            disk_size_gb: Some(20), // Default 20GB
            network_mode: "nat".to_string(),
            os_type: None,
            firmware: Some("bios".to_string()),
        }
    }
}

impl VmConfig {
    /// Serializes the config to a VMware-like .vmx format
    pub fn to_vmx(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("config.version = \"8\""));
        lines.push(format!("virtualHW.version = \"19\""));
        lines.push(format!("uuid.bios = \"{}\"", self.id));
        lines.push(format!("displayName = \"{}\"", self.name));
        lines.push(format!("numvcpus = \"{}\"", self.cpu_cores));
        lines.push(format!("memsize = \"{}\"", self.ram_size_mb));
        lines.push(format!("vm.dir = \"{}\"", self.vm_dir));
        
        if let Some(os) = &self.os_type {
            lines.push(format!("guestOS = \"{}\"", os));
        }
        if let Some(fw) = &self.firmware {
            lines.push(format!("firmware = \"{}\"", fw));
        }
        if let Some(ds) = self.disk_size_gb {
            lines.push(format!("disk.sizeGB = \"{}\"", ds));
        }
        
        if let Some(iso) = &self.iso_path {
            lines.push(format!("ide1:0.deviceType = \"cdrom-image\""));
            lines.push(format!("ide1:0.fileName = \"{}\"", iso));
            lines.push(format!("ide1:0.present = \"TRUE\""));
        }
        
        if let Some(disk) = &self.disk_path {
            lines.push(format!("scsi0:0.fileName = \"{}\"", disk));
            lines.push(format!("scsi0:0.present = \"TRUE\""));
        }
        
        lines.push(format!("ethernet0.connectionType = \"{}\"", self.network_mode));
        lines.push(format!("ethernet0.present = \"TRUE\""));
        
        lines.join("\n")
    }

    /// Parses a VMware-like .vmx format back into VmConfig
    pub fn from_vmx(content: &str) -> Self {
        let mut config = VmConfig::default();
        let mut map = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let val = value.trim().trim_matches('"');
                map.insert(key, val);
            }
        }

        if let Some(&uuid) = map.get("uuid.bios") { config.id = uuid.to_string(); }
        if let Some(&name) = map.get("displayName") { config.name = name.to_string(); }
        if let Some(&cpus) = map.get("numvcpus") { config.cpu_cores = cpus.parse().unwrap_or(1); }
        if let Some(&mem) = map.get("memsize") { config.ram_size_mb = mem.parse().unwrap_or(512); }
        if let Some(&dir) = map.get("vm.dir") { config.vm_dir = dir.to_string(); }
        if let Some(&iso) = map.get("ide1:0.fileName") { config.iso_path = Some(iso.to_string()); }
        if let Some(&disk) = map.get("scsi0:0.fileName") { config.disk_path = Some(disk.to_string()); }
        if let Some(&net) = map.get("ethernet0.connectionType") { config.network_mode = net.to_string(); }
        if let Some(&os) = map.get("guestOS") { config.os_type = Some(os.to_string()); }
        if let Some(&fw) = map.get("firmware") { config.firmware = Some(fw.to_string()); }
        if let Some(&ds) = map.get("disk.sizeGB") { config.disk_size_gb = ds.parse().ok(); }

        config
    }
}
