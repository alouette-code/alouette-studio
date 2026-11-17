use crate::vm_engine::config::VmConfig;
use crate::vm_engine::qemu_wrapper::{QemuInstance, SOCKET_DIR};
use crate::vm_engine::qga_client::QgaClient;
use crate::vm_engine::qmp_client::{QmpClient, VmInputAction};
use crate::vm_engine::vnc_stream::{VmFrameStream, VmObservation};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sysinfo::System;

const CONFIG_FILENAME: &str = "vm.json";
const MAX_LOG_TAIL_BYTES: u64 = 16 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct VmRuntimeInfo {
    pub config: VmConfig,
    pub status: String,
    pub ws_port: Option<u16>,
    pub vnc_display: Option<u16>,
}

pub struct ActiveVm {
    pub config: VmConfig,
    pub qemu_instance: Arc<Mutex<QemuInstance>>,
    pub ws_port: u16,
    pub vnc_display: u16,
    pub vnc_stream: Arc<VmFrameStream>,
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

        let manager = Self {
            storage_dir: storage_path,
            active_vms: Arc::new(Mutex::new(HashMap::new())),
        };
        manager.cleanup_orphaned_qemu();
        manager
    }

    fn cleanup_orphaned_qemu(&self) {
        let known_ids: HashSet<String> = self.scan_vms().into_iter().map(|c| c.id).collect();
        let mut sys = System::new_all();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        for (_pid, process) in sys.processes() {
            let cmd_str = format!("{:?}", process.cmd());
            let is_qemu = cmd_str.contains("qemu-system-");
            let is_alouette_socket = cmd_str.contains(SOCKET_DIR);

            if is_qemu && is_alouette_socket {
                let belongs_to_known_vm = known_ids
                    .iter()
                    .any(|id| cmd_str.contains(&format!("{}_qmp.sock", id)));
                if belongs_to_known_vm || known_ids.is_empty() {
                    let _ = process.kill();
                }
            }
        }
    }

    fn load_config_from_dir(dir: &Path) -> Option<VmConfig> {
        let json_path = dir.join(CONFIG_FILENAME);
        if json_path.exists() {
            if let Ok(content) = fs::read_to_string(&json_path) {
                if let Ok(mut config) = serde_json::from_str::<VmConfig>(&content) {
                    if config.vm_dir.is_empty() {
                        config.vm_dir = dir.to_string_lossy().into_owned();
                    }
                    config.normalize();
                    return Some(config);
                }
            }
        }

        // Backward compatibility: fall back to legacy .vmx files
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("vmx") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let mut config = VmConfig::from_vmx(&content);
                        if config.vm_dir.is_empty() {
                            config.vm_dir = dir.to_string_lossy().into_owned();
                        }
                        if !config.id.is_empty() {
                            return Some(config);
                        }
                    }
                }
            }
        }

        None
    }

    fn scan_vms(&self) -> Vec<VmConfig> {
        let mut vms = Vec::new();
        let mut seen_ids = HashSet::new();

        if let Ok(entries) = fs::read_dir(&self.storage_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_dir_or_symlink = path.is_dir()
                    || path
                        .symlink_metadata()
                        .map(|m| m.file_type().is_symlink())
                        .unwrap_or(false);

                if is_dir_or_symlink {
                    if let Some(config) = Self::load_config_from_dir(&path) {
                        if !config.id.is_empty() && seen_ids.insert(config.id.clone()) {
                            vms.push(config);
                        }
                    }
                }
            }
        }

        vms.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        vms
    }

    fn get_vm_config(&self, id: &str) -> Result<VmConfig, String> {
        self.scan_vms()
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| format!("VM with ID '{}' not found", id))
    }

    /// Creates or updates a virtual machine configuration (stored as vm.json).
    pub fn save_vm(
        &self,
        mut config: VmConfig,
        qemu_img_path: Option<&str>,
    ) -> Result<VmConfig, String> {
        if config.id.is_empty() {
            config.id = rand::random::<u64>().to_string();
        }

        let safe_name = VmConfig::sanitize_name(&config.name);
        let safe_id = VmConfig::sanitize_id(&config.id);
        if safe_name.is_empty() {
            return Err("Invalid VM name (must contain alphanumeric characters)".to_string());
        }
        if safe_id.is_empty() {
            return Err("Invalid VM ID".to_string());
        }
        config.id = safe_id;
        config.normalize();

        // Setup vm_dir: include ID suffix for new VMs to guarantee uniqueness across same-named VMs
        if config.vm_dir.trim().is_empty() {
            let folder_name = format!("{}_{}", safe_name, config.id);
            config.vm_dir = self
                .storage_dir
                .join(&folder_name)
                .to_string_lossy()
                .into_owned();
        }

        let vm_dir_path = Path::new(&config.vm_dir);
        if !vm_dir_path.exists() {
            fs::create_dir_all(vm_dir_path)
                .map_err(|e| format!("Failed to create VM directory: {}", e))?;
        }

        // Create symlink inside storage_dir when vm_dir is stored externally
        let link_name = format!("{}_{}", safe_name, config.id);
        let link_path = self.storage_dir.join(&link_name);
        if vm_dir_path.parent() != Some(&self.storage_dir) && !link_path.exists() {
            #[cfg(unix)]
            let _ = std::os::unix::fs::symlink(vm_dir_path, &link_path);
        }

        // Auto-configure and create qcow2 disk if empty
        if config
            .disk_path
            .as_deref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            let disk_name = format!("{}.qcow2", safe_name);
            let target_disk_path = vm_dir_path.join(&disk_name);
            let target_str = target_disk_path.to_string_lossy().into_owned();
            if !target_disk_path.exists() {
                let size_gb = config.disk_size_gb.unwrap_or(20);
                let qemu_img_bin = qemu_img_path.unwrap_or("qemu-img");
                QemuInstance::create_qcow2_disk(qemu_img_bin, &target_str, size_gb)?;
            }
            config.disk_path = Some(target_str);
        }

        // Write canonical vm.json
        let json_path = vm_dir_path.join(CONFIG_FILENAME);
        let content = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize VM config: {}", e))?;
        fs::write(&json_path, content)
            .map_err(|e| format!("Failed to write VM config ({}): {}", CONFIG_FILENAME, e))?;

        // Clean up legacy .vmx in the same directory once migrated to vm.json
        if let Ok(entries) = fs::read_dir(vm_dir_path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("vmx") {
                    let _ = fs::remove_file(p);
                }
            }
        }

        Ok(config)
    }

    /// Deletes a virtual machine configuration and its storage directory (if managed).
    pub fn delete_vm(&self, id: &str) -> Result<(), String> {
        let _ = self.stop_vm(id);

        if let Ok(config) = self.get_vm_config(id) {
            let vm_dir = Path::new(&config.vm_dir);
            if vm_dir.exists() && !config.vm_dir.is_empty() {
                if let (Ok(canon_vm_dir), Ok(canon_storage_dir)) =
                    (vm_dir.canonicalize(), self.storage_dir.canonicalize())
                {
                    if canon_vm_dir.starts_with(&canon_storage_dir)
                        && canon_vm_dir != canon_storage_dir
                    {
                        let _ = fs::remove_dir_all(&canon_vm_dir);
                    } else {
                        let _ = fs::remove_file(vm_dir.join(CONFIG_FILENAME));
                        let safe_name = VmConfig::sanitize_name(&config.name);
                        let _ = fs::remove_file(vm_dir.join(format!("{}.vmx", safe_name)));
                        if let Some(disk) = config.disk_path {
                            let _ = fs::remove_file(Path::new(&disk));
                        }
                    }
                }
            }

            let safe_name = VmConfig::sanitize_name(&config.name);
            let link_name = format!("{}_{}", safe_name, config.id);
            let link_path = self.storage_dir.join(&link_name);
            if link_path.exists() || link_path.symlink_metadata().is_ok() {
                let _ = fs::remove_file(link_path);
            }
        }

        Ok(())
    }

    /// Lists all virtual machines along with runtime VNC/WebSocket metadata.
    pub fn list_vms_with_runtime(&self) -> Result<Vec<VmRuntimeInfo>, String> {
        let mut active = self.active_vms.lock();
        active.retain(|_, vm| {
            let alive = vm.qemu_instance.lock().is_running();
            if !alive {
                vm.vnc_stream.stop();
            }
            alive
        });

        let mut results = Vec::new();
        for config in self.scan_vms() {
            if let Some(running) = active.get(&config.id) {
                results.push(VmRuntimeInfo {
                    config,
                    status: "running".to_string(),
                    ws_port: Some(running.ws_port),
                    vnc_display: Some(running.vnc_display),
                });
            } else {
                results.push(VmRuntimeInfo {
                    config,
                    status: "stopped".to_string(),
                    ws_port: None,
                    vnc_display: None,
                });
            }
        }

        Ok(results)
    }

    /// Legacy compatibility wrapper used by agent_harness.
    pub fn list_vms(&self) -> Result<Vec<(VmConfig, String)>, String> {
        Ok(self
            .list_vms_with_runtime()?
            .into_iter()
            .map(|info| (info.config, info.status))
            .collect())
    }

    /// Starts a virtual machine without holding the active_vms lock during spawn/sleep.
    pub fn start_vm(
        &self,
        id: &str,
        qemu_path: Option<&str>,
        qemu_img_path: Option<&str>,
    ) -> Result<(), String> {
        let (reserved_vnc, reserved_ws) = {
            let mut active = self.active_vms.lock();
            active.retain(|_, vm| vm.qemu_instance.lock().is_running());

            if active.contains_key(id) {
                return Err("VM is already running".to_string());
            }

            let vnc = active.values().map(|v| v.vnc_display).collect::<HashSet<_>>();
            let ws = active.values().map(|v| v.ws_port).collect::<HashSet<_>>();
            (vnc, ws)
        };

        let config = self.get_vm_config(id)?;
        let mut qemu_instance =
            QemuInstance::spawn(&config, qemu_path, qemu_img_path, &reserved_vnc, &reserved_ws)?;

        // Catch immediate QEMU startup crashes without holding the global mutex
        std::thread::sleep(std::time::Duration::from_millis(500));
        if !qemu_instance.is_running() {
            let log_file_path = Path::new(&config.vm_dir).join("qemu_error.log");
            let error_log = fs::read_to_string(&log_file_path)
                .unwrap_or_else(|_| "No log available".to_string());
            return Err(format!(
                "QEMU process crashed immediately upon startup.\nLog:\n{}",
                error_log
            ));
        }

        let ws_port = qemu_instance.ws_port;
        let vnc_display = qemu_instance.vnc_display;

        let mut active = self.active_vms.lock();
        if active.contains_key(id) {
            let _ = qemu_instance.kill();
            return Err("VM is already running".to_string());
        }

        let vnc_stream = VmFrameStream::start(5900 + vnc_display);

        active.insert(
            id.to_string(),
            ActiveVm {
                config,
                qemu_instance: Arc::new(Mutex::new(qemu_instance)),
                ws_port,
                vnc_display,
                vnc_stream,
            },
        );

        Ok(())
    }

    /// Force stops a running virtual machine (SIGKILL).
    pub fn stop_vm(&self, id: &str) -> Result<(), String> {
        let removed = {
            let mut active = self.active_vms.lock();
            active.remove(id)
        };
        if let Some(active_vm) = removed {
            active_vm.vnc_stream.stop();
            active_vm.qemu_instance.lock().kill()?;
            Ok(())
        } else {
            Err("VM is not running".to_string())
        }
    }

    /// Gracefully shuts down a running virtual machine using QMP system_powerdown (ACPI).
    pub fn shutdown_vm(&self, id: &str) -> Result<(), String> {
        if !self.is_vm_running(id) {
            return Err("VM is not running".to_string());
        }

        let qmp_socket_path = QemuInstance::qmp_socket_path(id);
        let mut client = QmpClient::connect(qmp_socket_path)?;
        client.system_powerdown()
    }

    fn is_vm_running(&self, id: &str) -> bool {
        let mut active = self.active_vms.lock();
        if let Some(vm) = active.get_mut(id) {
            if vm.qemu_instance.lock().is_running() {
                return true;
            }
            vm.vnc_stream.stop();
            active.remove(id);
        }
        false
    }

    fn read_log_tail(path: &Path) -> Option<String> {
        let mut file = fs::File::open(path).ok()?;
        let len = file.metadata().ok()?.len();
        if len == 0 {
            return None;
        }
        let start = len.saturating_sub(MAX_LOG_TAIL_BYTES);
        file.seek(SeekFrom::Start(start)).ok()?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;
        let text = String::from_utf8_lossy(&buf).into_owned();
        if start > 0 {
            // Drop partial first UTF-8/line fragment after seek
            if let Some(idx) = text.find('\n') {
                return Some(text[idx + 1..].to_string());
            }
        }
        Some(text)
    }

    /// Gets serial console and QEMU logs for a VM.
    pub fn get_vm_logs(&self, id: &str) -> Result<String, String> {
        let config = self.get_vm_config(id)?;
        let vm_dir = Path::new(&config.vm_dir);

        let runtime_header = {
            let mut active = self.active_vms.lock();
            if let Some(active_vm) = active.get_mut(id) {
                if active_vm.qemu_instance.lock().is_running() {
                    format!(
                        "[Status: Running | VNC: 127.0.0.1:{} | WebSocket: ws://127.0.0.1:{}]",
                        5900 + active_vm.vnc_display,
                        active_vm.ws_port
                    )
                } else {
                    active.remove(id);
                    "[Status: Stopped]".to_string()
                }
            } else {
                "[Status: Stopped]".to_string()
            }
        };

        let serial_tail = Self::read_log_tail(&vm_dir.join("serial.log"));
        let qemu_err_tail = Self::read_log_tail(&vm_dir.join("qemu_error.log"));

        let mut sections = vec![runtime_header];
        if let Some(serial) = serial_tail {
            sections.push(format!("--- Serial Console (COM1) ---\n{}", serial.trim_end()));
        } else {
            sections.push(
                "--- Serial Console (COM1) ---\n(No serial output yet. Add console=ttyS0 to kernel args for Linux serial logs.)"
                    .to_string(),
            );
        }

        if let Some(err) = qemu_err_tail {
            sections.push(format!("--- QEMU Log ---\n{}", err.trim_end()));
        }

        Ok(sections.join("\n\n"))
    }

    // --- Snapshot Management ---

    pub fn create_snapshot(&self, id: &str, snapshot_name: &str) -> Result<(), String> {
        let config = self.get_vm_config(id)?;
        if self.is_vm_running(id) {
            let qmp_socket_path = QemuInstance::qmp_socket_path(id);
            let mut client = QmpClient::connect(qmp_socket_path)?;
            client.save_snapshot(snapshot_name)?;
        } else {
            let disk_path = config
                .disk_path
                .as_ref()
                .ok_or("VM has no disk path configured")?;
            let output = std::process::Command::new("qemu-img")
                .args(["snapshot", "-c", snapshot_name, disk_path])
                .output()
                .map_err(|e| format!("Failed to invoke qemu-img: {}", e))?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).into_owned());
            }
        }
        Ok(())
    }

    pub fn restore_snapshot(&self, id: &str, snapshot_name: &str) -> Result<(), String> {
        let config = self.get_vm_config(id)?;
        if self.is_vm_running(id) {
            let qmp_socket_path = QemuInstance::qmp_socket_path(id);
            let mut client = QmpClient::connect(qmp_socket_path)?;
            client.load_snapshot(snapshot_name)?;
        } else {
            let disk_path = config
                .disk_path
                .as_ref()
                .ok_or("VM has no disk path configured")?;
            let output = std::process::Command::new("qemu-img")
                .args(["snapshot", "-a", snapshot_name, disk_path])
                .output()
                .map_err(|e| format!("Failed to invoke qemu-img: {}", e))?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).into_owned());
            }
        }
        Ok(())
    }

    pub fn delete_snapshot(&self, id: &str, snapshot_name: &str) -> Result<(), String> {
        let config = self.get_vm_config(id)?;
        if self.is_vm_running(id) {
            let qmp_socket_path = QemuInstance::qmp_socket_path(id);
            let mut client = QmpClient::connect(qmp_socket_path)?;
            client.delete_snapshot(snapshot_name)?;
        } else {
            let disk_path = config
                .disk_path
                .as_ref()
                .ok_or("VM has no disk path configured")?;
            let output = std::process::Command::new("qemu-img")
                .args(["snapshot", "-d", snapshot_name, disk_path])
                .output()
                .map_err(|e| format!("Failed to invoke qemu-img: {}", e))?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).into_owned());
            }
        }
        Ok(())
    }

    pub fn list_snapshots(&self, id: &str) -> Result<Vec<String>, String> {
        let config = self.get_vm_config(id)?;
        let disk_path = config
            .disk_path
            .as_ref()
            .ok_or("VM has no disk path configured")?;

        // -U allows reading metadata safely while QEMU holds a lock on the disk
        let output = std::process::Command::new("qemu-img")
            .args(["info", "-U", "--output", "json", disk_path])
            .output()
            .map_err(|e| format!("Failed to invoke qemu-img: {}", e))?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }

        let info: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Failed to parse qemu-img output: {}", e))?;

        let mut snapshots = Vec::new();
        if let Some(snaps) = info.get("snapshots").and_then(|s| s.as_array()) {
            for snap in snaps {
                if let Some(name) = snap.get("name").and_then(|n| n.as_str()) {
                    snapshots.push(name.to_string());
                }
            }
        }

        Ok(snapshots)
    }

    // --- Guest File Injection (QGA) ---

    pub fn inject_file(&self, id: &str, host_path: &str, guest_path: &str) -> Result<(), String> {
        let h_path = Path::new(host_path);
        if !h_path.is_file() {
            return Err("Host path does not exist or is not a regular file".to_string());
        }

        let _config = self.get_vm_config(id)?;
        if !self.is_vm_running(id) {
            return Err("VM must be running to inject files via QEMU Guest Agent.".to_string());
        }

        if let Ok(metadata) = fs::metadata(host_path) {
            if metadata.len() > 10 * 1024 * 1024 {
                return Err(
                    "File exceeds 10MB limit for QGA injection. Please use a secondary disk for large files."
                        .to_string(),
                );
            }
        }

        let qga_socket_path = QemuInstance::qga_socket_path(id);
        let mut client = QgaClient::connect(qga_socket_path)?;

        let data = fs::read(host_path).map_err(|e| format!("Failed to read host file: {}", e))?;
        let handle = client.guest_file_open(guest_path, "w+")?;

        let chunk_size = 48 * 1024;
        for chunk in data.chunks(chunk_size) {
            if let Err(e) = client.guest_file_write(handle, chunk) {
                let _ = client.guest_file_close(handle);
                return Err(e);
            }
        }

        client.guest_file_close(handle)?;
        Ok(())
    }

    // --- Streaming Framebuffer & Direct VM Agent Control ---

    pub fn get_vnc_stream(&self, id: &str) -> Result<Arc<VmFrameStream>, String> {
        let mut active = self.active_vms.lock();
        if let Some(vm) = active.get_mut(id) {
            if vm.qemu_instance.lock().is_running() {
                return Ok(vm.vnc_stream.clone());
            }
            vm.vnc_stream.stop();
            active.remove(id);
        }
        Err(format!("VM '{}' is not running", id))
    }

    pub fn observe_vm(
        &self,
        id: &str,
        since_seq: Option<u64>,
        wait_change_ms: u64,
        include_ocr: bool,
        include_image_base64: bool,
        max_image_dim: u32,
    ) -> Result<VmObservation, String> {
        let stream = self.get_vnc_stream(id)?;
        Ok(stream.observe(
            since_seq,
            wait_change_ms,
            include_ocr,
            include_image_base64,
            max_image_dim,
        ))
    }

    /// Executes a batch of GUI actions directly in the VM via QMP `input-send-event`.
    /// Returns `(executed_actions_count, (screen_w, screen_h), pre_action_frame_seq)`.
    pub fn act_on_vm(
        &self,
        id: &str,
        actions: &[VmInputAction],
    ) -> Result<(usize, (u32, u32), u64), String> {
        let stream = self.get_vnc_stream(id)?;
        let (w, h) = stream.current_resolution();
        let pre_seq = stream.current_seq();
        let qmp_socket_path = QemuInstance::qmp_socket_path(id);
        let mut qmp = QmpClient::connect(qmp_socket_path)?;
        let count = qmp.execute_action_batch(actions, w, h)?;
        Ok((count, (w, h), pre_seq))
    }

    pub fn read_guest_file(
        &self,
        id: &str,
        guest_path: &str,
        max_bytes: usize,
    ) -> Result<String, String> {
        if !self.is_vm_running(id) {
            return Err("VM must be running to read guest files via QGA.".to_string());
        }
        let qga_socket_path = QemuInstance::qga_socket_path(id);
        let mut client = QgaClient::connect(qga_socket_path)?;
        let bytes = client.read_file_bytes(guest_path, max_bytes.min(2 * 1024 * 1024))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn write_guest_file(
        &self,
        id: &str,
        guest_path: &str,
        content: &str,
    ) -> Result<usize, String> {
        if !self.is_vm_running(id) {
            return Err("VM must be running to write guest files via QGA.".to_string());
        }
        let bytes = content.as_bytes();
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("Content exceeds 2MB limit for direct QGA text write.".to_string());
        }
        let qga_socket_path = QemuInstance::qga_socket_path(id);
        let mut client = QgaClient::connect(qga_socket_path)?;
        client.write_file_bytes(guest_path, bytes)?;
        Ok(bytes.len())
    }
}

impl Drop for VmManager {
    fn drop(&mut self) {
        let mut active = self.active_vms.lock();
        for (_, active_vm) in active.drain() {
            active_vm.vnc_stream.stop();
            let mut qemu = active_vm.qemu_instance.lock();
            let _ = qemu.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_vm_manager_loads_legacy_vmx_and_migrates_on_save() {
        let dir = tempdir().unwrap();
        let legacy_dir = dir.path().join("Legacy-VM");
        fs::create_dir_all(&legacy_dir).unwrap();
        let disk_file = legacy_dir.join("Legacy-VM.qcow2");
        fs::write(&disk_file, b"dummy").unwrap();

        let vmx = format!(
            "uuid.bios = \"999\"\ndisplayName = \"Legacy VM!\"\nnumvcpus = \"2\"\nmemsize = \"512\"\nscsi0:0.fileName = \"{}\"\nethernet0.connectionType = \"host-only\"\n",
            disk_file.display()
        );
        fs::write(legacy_dir.join("Legacy_VM.vmx"), vmx).unwrap();

        let mgr = VmManager::new(dir.path());
        let vms = mgr.list_vms_with_runtime().unwrap();
        assert_eq!(vms.len(), 1);
        assert_eq!(vms[0].config.id, "999");
        assert_eq!(vms[0].config.network_mode, "host-only");
        assert_eq!(vms[0].status, "stopped");

        // Saving migrates to vm.json and removes legacy .vmx
        let saved = mgr.save_vm(vms[0].config.clone(), None).unwrap();
        assert!(legacy_dir.join("vm.json").exists());
        assert!(!legacy_dir.join("Legacy_VM.vmx").exists());
        assert_eq!(saved.id, "999");

        // Logs should report stopped state without failing
        let logs = mgr.get_vm_logs("999").unwrap();
        assert!(logs.contains("[Status: Stopped]"));

        // Deleting removes managed dir
        mgr.delete_vm("999").unwrap();
        assert!(!legacy_dir.exists());
    }

    #[test]
    fn test_same_name_vms_get_distinct_directories() {
        let dir = tempdir().unwrap();
        let mgr = VmManager::new(dir.path());
        let dummy_disk = dir.path().join("shared_dummy.qcow2");
        fs::write(&dummy_disk, b"dummy").unwrap();

        let mut cfg1 = VmConfig::default();
        cfg1.name = "DevBox".to_string();
        cfg1.disk_path = Some(dummy_disk.to_string_lossy().into_owned());

        let mut cfg2 = VmConfig::default();
        cfg2.name = "DevBox".to_string();
        cfg2.disk_path = Some(dummy_disk.to_string_lossy().into_owned());

        let s1 = mgr.save_vm(cfg1, None).unwrap();
        let s2 = mgr.save_vm(cfg2, None).unwrap();
        assert_ne!(s1.id, s2.id);
        assert_ne!(s1.vm_dir, s2.vm_dir);

        let list = mgr.list_vms_with_runtime().unwrap();
        assert_eq!(list.len(), 2);
    }
}
