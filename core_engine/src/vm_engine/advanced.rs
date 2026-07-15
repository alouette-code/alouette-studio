use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    pub audio_enabled: bool,
    pub graceful_shutdown_timeout: u32,
    pub host_port_forwards: Vec<(u16, u16)>, // (host_port, guest_port)
    pub additional_disks: Vec<String>,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            audio_enabled: false,
            graceful_shutdown_timeout: 30, // seconds
            host_port_forwards: Vec::new(),
            additional_disks: Vec::new(),
        }
    }
}

impl AdvancedConfig {
    pub fn to_vmx_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("advanced.audio = \"{}\"", self.audio_enabled));
        lines.push(format!("advanced.shutdown_timeout = \"{}\"", self.graceful_shutdown_timeout));
        
        for (i, (h, g)) in self.host_port_forwards.iter().enumerate() {
            lines.push(format!("advanced.forward{}.host = \"{}\"", i, h));
            lines.push(format!("advanced.forward{}.guest = \"{}\"", i, g));
        }
        
        for (i, disk) in self.additional_disks.iter().enumerate() {
            lines.push(format!("advanced.disk{}.path = \"{}\"", i, disk));
        }
        
        lines
    }

    pub fn parse_vmx_line(&mut self, key: &str, value: &str) {
        if key == "advanced.audio" {
            self.audio_enabled = value.to_lowercase() == "true";
        } else if key == "advanced.shutdown_timeout" {
            if let Ok(val) = value.parse() {
                self.graceful_shutdown_timeout = val;
            }
        } else if key.starts_with("advanced.forward") {
            // advanced.forward0.host = "5555"
            // advanced.forward0.guest = "5555"
            let parts: Vec<&str> = key.split('.').collect();
            if parts.len() == 3 {
                let index_str = &parts[1][7..]; // after "forward"
                if let Ok(idx) = index_str.parse::<usize>() {
                    while self.host_port_forwards.len() <= idx {
                        self.host_port_forwards.push((0, 0));
                    }
                    if let Ok(port) = value.parse::<u16>() {
                        if parts[2] == "host" {
                            self.host_port_forwards[idx].0 = port;
                        } else if parts[2] == "guest" {
                            self.host_port_forwards[idx].1 = port;
                        }
                    }
                }
            }
        } else if key.starts_with("advanced.disk") {
            // advanced.disk0.path = "path/to/disk"
            let parts: Vec<&str> = key.split('.').collect();
            if parts.len() == 3 {
                let index_str = &parts[1][4..]; // after "disk"
                if let Ok(idx) = index_str.parse::<usize>() {
                    while self.additional_disks.len() <= idx {
                        self.additional_disks.push(String::new());
                    }
                    if parts[2] == "path" {
                        self.additional_disks[idx] = value.to_string();
                    }
                }
            }
        }
    }
}
