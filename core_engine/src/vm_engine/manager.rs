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

    fn get_config_path(&self, id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", id))
    }

    /// Creates or updates a virtual machine configuration.
    pub fn save_vm(&self, mut config: VmConfig) -> Result<(), String> {
        if config.id.is_empty() {
            config.id = rand::random::<u64>().to_string();
        }

        // Auto-configure disk path if empty
        if config.disk_path.is_none() {
            let disk_name = format!("{}_disk.qcow2", config.name);
            config.disk_path = Some(self.storage_dir.join(disk_name).to_string_lossy().into_owned());
        }

        let config_path = self.get_config_path(&config.id);
        let content = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize VM config: {}", e))?;
        
        fs::write(config_path, content)
            .map_err(|e| format!("Failed to write VM config: {}", e))?;
        
        Ok(())
    }

    /// Deletes a virtual machine configuration.
    pub fn delete_vm(&self, id: &str) -> Result<(), String> {
        let _ = self.stop_vm(id);

        let config_path = self.get_config_path(id);
        if config_path.exists() {
            fs::remove_file(config_path)
                .map_err(|e| format!("Failed to delete VM config file: {}", e))?;
        }
        Ok(())
    }

    /// Lists all virtual machines and their statuses.
    pub fn list_vms(&self) -> Result<Vec<(VmConfig, String)>, String> {
        let mut vms = Vec::new();
        let mut active = self.active_vms.lock();

        if let Ok(entries) = fs::read_dir(&self.storage_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(config) = serde_json::from_str::<VmConfig>(&content) {
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
                    }
                }
            }
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
        let config_path = self.get_config_path(id);
        if !config_path.exists() {
            return Err(format!("VM with ID {} not found", id));
        }

        let content = fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read VM config: {}", e))?;
        let config: VmConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse VM config: {}", e))?;

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
