use std::process::{Command, Child};
use std::path::Path;
use crate::vm_engine::config::VmConfig;

pub struct QemuInstance {
    child: Child,
}

impl QemuInstance {
    /// Spawns a QEMU/KVM virtual machine process based on the config.
    pub fn spawn(config: &VmConfig, qemu_path: Option<&str>, qemu_img_path: Option<&str>) -> Result<Self, String> {
        let qemu_bin = qemu_path.unwrap_or("qemu-system-x86_64");
        let qemu_img_bin = qemu_img_path.unwrap_or("qemu-img");

        // 1. Verify QEMU is installed / accessible
        if !Self::is_qemu_installed(qemu_bin) {
            return Err(format!(
                "QEMU binary not found or not executable at '{}'. Please ensure QEMU is installed.",
                qemu_bin
            ));
        }

        // 2. Prepare Virtual Disk (Auto-create a qcow2 disk if it does not exist)
        let disk_path_str = config.disk_path.clone().unwrap_or_else(|| {
            format!("{}_disk.qcow2", config.name)
        });
        let disk_path = Path::new(&disk_path_str);
        if !disk_path.exists() {
            Self::create_qcow2_disk(qemu_img_bin, disk_path.to_str().unwrap(), 20)?; // Default to 20GB disk if not present
        }

        // 3. Construct QEMU arguments
        let mut args = vec![
            "-enable-kvm".to_string(),
            "-cpu".to_string(), "host".to_string(),
            "-smp".to_string(), config.cpu_cores.to_string(),
            "-m".to_string(), config.ram_size_mb.to_string(),
            // Display & GPU settings: VNC Server with WebSocket for UI embedding
            "-device".to_string(), "virtio-vga".to_string(),
            "-display".to_string(), "none".to_string(),
            "-vnc".to_string(), "127.0.0.1:0,websocket=5700".to_string(),
            // Input Devices (Keyboard & Mouse)
            "-device".to_string(), "virtio-keyboard-pci".to_string(),
            "-device".to_string(), "virtio-mouse-pci".to_string(),
            // Storage
            "-drive".to_string(), format!("file={},format=qcow2,if=virtio", disk_path.to_str().unwrap()),
            // Network (User mode NAT with VirtIO NIC)
            "-netdev".to_string(), "user,id=net0".to_string(),
            "-device".to_string(), "virtio-net-pci,netdev=net0".to_string(),
        ];

        // Boot from ISO if provided (for OS installers)
        if let Some(iso) = &config.iso_path {
            if Path::new(iso).exists() {
                args.push("-cdrom".to_string());
                args.push(iso.clone());
                args.push("-boot".to_string());
                args.push("d".to_string()); // Boot from CD-ROM first
            }
        }

        // QMP (QEMU Monitor Protocol) over Unix Socket
        let qmp_socket_path = Path::new(&config.vm_dir).join(format!("{}_qmp.sock", config.id));
        args.push("-qmp".to_string());
        args.push(format!("unix:{},server,nowait", qmp_socket_path.to_string_lossy()));

        // 4. Start the QEMU process
        let child = Command::new(qemu_bin)
            .args(&args)
            .spawn()
            .map_err(|e| format!("Failed to spawn QEMU process ({}): {}", qemu_bin, e))?;

        Ok(Self { child })
    }

    /// Kills the running QEMU process.
    pub fn kill(&mut self) -> Result<(), String> {
        self.child.kill()
            .map_err(|e| format!("Failed to kill QEMU VM process: {}", e))?;
        let _ = self.child.wait();
        Ok(())
    }

    /// Checks if QEMU process is still running.
    pub fn is_running(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            _ => false,
        }
    }

    fn is_qemu_installed(qemu_bin: &str) -> bool {
        Command::new(qemu_bin)
            .arg("--version")
            .output()
            .is_ok()
    }

    fn create_qcow2_disk(qemu_img_bin: &str, path: &str, size_gb: u32) -> Result<(), String> {
        // Ensure directory parent exists
        if let Some(parent) = Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let output = Command::new(qemu_img_bin)
            .args(&["create", "-f", "qcow2", path, &format!("{}G", size_gb)])
            .output()
            .map_err(|e| format!("Failed to invoke qemu-img ({}): {}", qemu_img_bin, e))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("qemu-img disk creation failed: {}", err_msg));
        }

        Ok(())
    }
}
