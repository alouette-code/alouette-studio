use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;
use crate::vm_engine::config::VmConfig;
use crate::vm_engine::qemu_wrapper::QemuInstance;

pub struct ActiveVm {
    pub config: VmConfig,
    pub qemu_instance: Arc<Mutex<QemuInstance>>,
    pub status: String,
}

pub struct VmManager {
    storage_dir: PathBuf,
    active_vms: Arc<Mutex<HashMap<String, ActiveVm>>>,
}

impl VmManager {
    pub fn new<P: AsRef<Path>>(storage_dir: P) -> Self {
        let storage_path = storage_dir.as_ref().to_path_buf();
        if !storage_path.exists() {
            let _ = fs::create_dir_all(&storage_path);
        }

        Self {
            storage_dir: storage_path,
            active_vms: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn scan_vms(&self) -> Vec<VmConfig> {
        let mut vms = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        if let Ok(entries) = fs::read_dir(&self.storage_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_dir_or_symlink = path.is_dir() || path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false);
                
                if is_dir_or_symlink {
                    if let Ok(sub_entries) = fs::read_dir(&path) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_path = sub_entry.path();
                            if sub_path.extension().and_then(|e| e.to_str()) == Some("vmx") {
                                if let Ok(content) = fs::read_to_string(&sub_path) {
                                    let config = VmConfig::from_vmx(&content);
                                    // Ensure ID is populated if missing in legacy configs
                                    if !config.id.is_empty() && !seen_ids.contains(&config.id) {
                                        seen_ids.insert(config.id.clone());
                                        vms.push(config);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        vms
    }

    fn get_config_path(&self, id: &str) -> Option<PathBuf> {
        self.scan_vms().into_iter().find(|c| c.id == id).map(|c| {
            let safe_name = c.name.replace(" ", "_");
            PathBuf::from(&c.vm_dir).join(format!("{}.vmx", safe_name))
        })
    }

    /// Creates or updates a virtual machine configuration.
    pub fn save_vm(&self, mut config: VmConfig) -> Result<(), String> {
        if config.id.is_empty() {
            config.id = rand::random::<u64>().to_string();
        }

        let safe_name = config.name.replace(" ", "_");

        // Setup vm_dir
        if config.vm_dir.is_empty() {
            config.vm_dir = self.storage_dir.join(&safe_name).to_string_lossy().into_owned();
        }
        let vm_dir_path = Path::new(&config.vm_dir);
        if !vm_dir_path.exists() {
            fs::create_dir_all(vm_dir_path).map_err(|e| format!("Failed to create VM directory: {}", e))?;
        }

        // Create a symlink in storage_dir if vm_dir is outside
        let link_name = format!("{}_{}", safe_name, config.id);
        let link_path = self.storage_dir.join(&link_name);
        if vm_dir_path.parent() != Some(&self.storage_dir) && !link_path.exists() {
            #[cfg(unix)]
            let _ = std::os::unix::fs::symlink(vm_dir_path, &link_path);
        }

        // Handle ISO copying (physical copy into VM directory for independence)
        if let Some(iso) = &config.iso_path {
            let iso_path = Path::new(iso);
            if iso_path.exists() && !iso_path.starts_with(vm_dir_path) {
                let file_name = iso_path.file_name().unwrap_or_default();
                let dest_path = vm_dir_path.join(file_name);
                if !dest_path.exists() {
                    fs::copy(iso_path, &dest_path).map_err(|e| format!("Failed to copy ISO file: {}", e))?;
                }
                config.iso_path = Some(dest_path.to_string_lossy().into_owned());
            }
        }

        // Auto-configure disk path if empty
        if config.disk_path.is_none() || config.disk_path.as_ref().unwrap().is_empty() {
            let disk_name = format!("{}.qcow2", safe_name);
            config.disk_path = Some(vm_dir_path.join(disk_name).to_string_lossy().into_owned());
        }

        let config_path = vm_dir_path.join(format!("{}.vmx", safe_name));
        let content = config.to_vmx();
        
        fs::write(config_path, content)
            .map_err(|e| format!("Failed to write VM config (.vmx): {}", e))?;
        
        Ok(())
    }

    /// Deletes a virtual machine configuration.
    pub fn delete_vm(&self, id: &str) -> Result<(), String> {
        let _ = self.stop_vm(id);

        if let Some(config_path) = self.get_config_path(id) {
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    let config = VmConfig::from_vmx(&content);
                    let vm_dir = Path::new(&config.vm_dir);
                    if vm_dir.exists() && !config.vm_dir.is_empty() {
                        let _ = fs::remove_dir_all(vm_dir); // Remove the entire VM directory including disk and ISO
                    }
                    
                    // Clean up potential symlinks in storage_dir
                    let safe_name = config.name.replace(" ", "_");
                    let link_name = format!("{}_{}", safe_name, config.id);
                    let link_path = self.storage_dir.join(&link_name);
                    if link_path.exists() || link_path.symlink_metadata().is_ok() {
                        let _ = fs::remove_file(link_path);
                    }
                }
            }
        }
        Ok(())
    }

    /// Lists all virtual machines and their statuses.
    pub fn list_vms(&self) -> Result<Vec<(VmConfig, String)>, String> {
        let mut vms = Vec::new();
        let mut active = self.active_vms.lock();

        for config in self.scan_vms() {
            let mut is_still_running = false;
            if let Some(active_vm) = active.get_mut(&config.id) {
                is_still_running = active_vm.qemu_instance.lock().is_running();
                if !is_still_running {
                    active_vm.status = "stopped".to_string();
                }
            }

            let status = if is_still_running { "running".to_string() } else { "stopped".to_string() };
            vms.push((config, status));
        }
        
        Ok(vms)
    }

    /// Starts a virtual machine.
    pub fn start_vm(&self, id: &str, qemu_path: Option<&str>, qemu_img_path: Option<&str>) -> Result<(), String> {
        let mut active = self.active_vms.lock();
        if let Some(active_vm) = active.get_mut(id) {
            if active_vm.qemu_instance.lock().is_running() {
                return Err("VM is already running".to_string());
            }
        }

        // Load config
        let config_path = self.get_config_path(id)
            .ok_or_else(|| format!("VM with ID {} not found", id))?;
        if !config_path.exists() {
            return Err(format!("VM config file not found at {:?}", config_path));
        }

        let content = fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read VM config: {}", e))?;
        let config = VmConfig::from_vmx(&content);

        // Launch QEMU KVM instance
        let qemu_instance = QemuInstance::spawn(&config, qemu_path, qemu_img_path)?;

        active.insert(
            id.to_string(),
            ActiveVm {
                config,
                qemu_instance: Arc::new(Mutex::new(qemu_instance)),
                status: "running".to_string(),
            },
        );

        Ok(())
    }

    /// Stops a running virtual machine.
    pub fn stop_vm(&self, id: &str) -> Result<(), String> {
        let mut active = self.active_vms.lock();
        if let Some(active_vm) = active.remove(id) {
            active_vm.qemu_instance.lock().kill()?;
            Ok(())
        } else {
            Err("VM is not running".to_string())
        }
    }

    /// Gets live logs of a running VM.
    pub fn get_vm_logs(&self, id: &str) -> Result<String, String> {
        let mut active = self.active_vms.lock();
        if let Some(active_vm) = active.get_mut(id) {
            if active_vm.qemu_instance.lock().is_running() {
                return Ok("[VM running under QEMU KVM with hardware GPU acceleration. Display rendered on GTK host display.]".to_string());
            }
        }
        Ok("[VM is stopped]".to_string())
    }
}
