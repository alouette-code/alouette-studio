use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    #[serde(default)]
    pub audio_enabled: bool,
    #[serde(default)]
    pub tpm_enabled: bool,
    #[serde(default = "default_shutdown_timeout")]
    pub graceful_shutdown_timeout: u32,
    #[serde(default)]
    pub host_port_forwards: Vec<(u16, u16)>, // (host_port, guest_port)
    #[serde(default)]
    pub additional_disks: Vec<String>,
}

fn default_shutdown_timeout() -> u32 {
    30
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            audio_enabled: false,
            tpm_enabled: false,
            graceful_shutdown_timeout: default_shutdown_timeout(),
            host_port_forwards: Vec::new(),
            additional_disks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub os_type: Option<String>,
    #[serde(default)]
    pub arch: Option<String>, // "x86_64" (default) or "aarch64"
    pub cpu_cores: u32,
    pub ram_size_mb: u64,
    #[serde(default)]
    pub vm_dir: String,
    #[serde(default)]
    pub iso_path: Option<String>,
    #[serde(default)]
    pub disk_path: Option<String>,
    #[serde(default = "default_disk_size_gb")]
    pub disk_size_gb: Option<u32>,
    #[serde(default = "default_network_mode")]
    pub network_mode: String,
    #[serde(default = "default_firmware")]
    pub firmware: Option<String>, // "bios" or "uefi"
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

fn default_disk_size_gb() -> Option<u32> {
    Some(20)
}

fn default_network_mode() -> String {
    "nat".to_string()
}

fn default_firmware() -> Option<String> {
    Some("bios".to_string())
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Default-VM".to_string(),
            cpu_cores: 1,
            ram_size_mb: 512,
            vm_dir: String::new(),
            iso_path: None,
            disk_path: None,
            disk_size_gb: default_disk_size_gb(),
            network_mode: default_network_mode(),
            os_type: None,
            arch: Some("x86_64".to_string()),
            firmware: default_firmware(),
            advanced: AdvancedConfig::default(),
        }
    }
}

impl VmConfig {
    /// Sanitizes a VM name for safe filesystem paths.
    pub fn sanitize_name(name: &str) -> String {
        name.trim()
            .chars()
            .map(|c| if c == ' ' { '_' } else { c })
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect()
    }

    /// Sanitizes a VM ID for safe filenames and socket names.
    pub fn sanitize_id(id: &str) -> String {
        id.chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect()
    }

    /// Normalizes network mode strings from UI/legacy configs into canonical values.
    pub fn normalize_network_mode(mode: &str) -> String {
        match mode.trim().to_lowercase().as_str() {
            "bridge" | "bridged" => "bridge".to_string(),
            "host-only" | "host_only" | "isolated" => "host-only".to_string(),
            "tap" => "tap".to_string(),
            _ => "nat".to_string(),
        }
    }

    /// Validates and clamps numeric/config bounds in place.
    pub fn normalize(&mut self) {
        self.cpu_cores = self.cpu_cores.clamp(1, 64);
        self.ram_size_mb = self.ram_size_mb.clamp(128, 65536);
        if let Some(ds) = self.disk_size_gb {
            self.disk_size_gb = Some(ds.clamp(1, 2000));
        } else {
            self.disk_size_gb = Some(20);
        }
        self.network_mode = Self::normalize_network_mode(&self.network_mode);
        self.firmware = Some(match self.firmware.as_deref() {
            Some(fw) if fw.eq_ignore_ascii_case("uefi") => "uefi".to_string(),
            _ => "bios".to_string(),
        });
        self.arch = Some(match self.arch.as_deref() {
            Some(a) if a.eq_ignore_ascii_case("aarch64") || a.eq_ignore_ascii_case("arm64") => {
                "aarch64".to_string()
            }
            _ => "x86_64".to_string(),
        });
        if self.iso_path.as_deref() == Some("") {
            self.iso_path = None;
        }
        if self.disk_path.as_deref() == Some("") {
            self.disk_path = None;
        }
    }

    /// Parses a legacy VMware-like .vmx format into VmConfig for backward compatibility.
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
                if key.starts_with("advanced.") {
                    parse_legacy_advanced_line(&mut config.advanced, key, val);
                } else {
                    map.insert(key, val);
                }
            }
        }

        if let Some(&uuid) = map.get("uuid.bios") {
            config.id = uuid.to_string();
        }
        if let Some(&name) = map.get("displayName") {
            config.name = name.to_string();
        }
        if let Some(&cpus) = map.get("numvcpus") {
            config.cpu_cores = cpus.parse().unwrap_or(1);
        }
        if let Some(&mem) = map.get("memsize") {
            config.ram_size_mb = mem.parse().unwrap_or(512);
        }
        if let Some(&dir) = map.get("vm.dir") {
            config.vm_dir = dir.to_string();
        }
        if let Some(&iso) = map.get("ide1:0.fileName") {
            config.iso_path = Some(iso.to_string());
        }
        if let Some(&disk) = map.get("scsi0:0.fileName") {
            config.disk_path = Some(disk.to_string());
        }
        if let Some(&net) = map.get("ethernet0.connectionType") {
            config.network_mode = net.to_string();
        }
        if let Some(&os) = map.get("guestOS") {
            config.os_type = Some(os.to_string());
        }
        if let Some(&fw) = map.get("firmware") {
            config.firmware = Some(fw.to_string());
        }
        if let Some(&ds) = map.get("disk.sizeGB") {
            config.disk_size_gb = ds.parse().ok();
        }

        config.normalize();
        config
    }
}

fn parse_legacy_advanced_line(adv: &mut AdvancedConfig, key: &str, value: &str) {
    if key == "advanced.audio" {
        adv.audio_enabled = value.eq_ignore_ascii_case("true");
    } else if key == "advanced.shutdown_timeout" {
        if let Ok(val) = value.parse() {
            adv.graceful_shutdown_timeout = val;
        }
    } else if let Some(rest) = key.strip_prefix("advanced.forward") {
        if let Some((idx_str, field)) = rest.split_once('.') {
            if let Ok(idx) = idx_str.parse::<usize>() {
                while adv.host_port_forwards.len() <= idx {
                    adv.host_port_forwards.push((0, 0));
                }
                if let Ok(port) = value.parse::<u16>() {
                    if field == "host" {
                        adv.host_port_forwards[idx].0 = port;
                    } else if field == "guest" {
                        adv.host_port_forwards[idx].1 = port;
                    }
                }
            }
        }
    } else if let Some(rest) = key.strip_prefix("advanced.disk") {
        if let Some((idx_str, field)) = rest.split_once('.') {
            if field == "path" {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    while adv.additional_disks.len() <= idx {
                        adv.additional_disks.push(String::new());
                    }
                    adv.additional_disks[idx] = value.to_string();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_vmx_parsing_and_normalization() {
        let vmx = r#"
config.version = "8"
uuid.bios = "2339926108475324171"
displayName = "New-VM-1"
numvcpus = "2"
memsize = "1024"
vm.dir = "/tmp/New-VM-1"
firmware = "uefi"
disk.sizeGB = "25"
ethernet0.connectionType = "bridged"
advanced.audio = "true"
advanced.forward0.host = "2222"
advanced.forward0.guest = "22"
"#;
        let cfg = VmConfig::from_vmx(vmx);
        assert_eq!(cfg.id, "2339926108475324171");
        assert_eq!(cfg.name, "New-VM-1");
        assert_eq!(cfg.cpu_cores, 2);
        assert_eq!(cfg.ram_size_mb, 1024);
        assert_eq!(cfg.firmware.as_deref(), Some("uefi"));
        assert_eq!(cfg.disk_size_gb, Some(25));
        assert_eq!(cfg.network_mode, "bridge");
        assert!(cfg.advanced.audio_enabled);
        assert_eq!(cfg.advanced.host_port_forwards, vec![(2222, 22)]);
    }

    #[test]
    fn test_sanitize_and_network_normalization() {
        assert_eq!(VmConfig::sanitize_name(" My VM /../ 1! "), "My_VM__1");
        assert_eq!(VmConfig::normalize_network_mode("bridged"), "bridge");
        assert_eq!(VmConfig::normalize_network_mode("host-only"), "host-only");
        assert_eq!(VmConfig::normalize_network_mode("unknown"), "nat");
    }
}
