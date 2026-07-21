use std::process::{Command, Child};
use std::path::Path;
use crate::vm_engine::config::VmConfig;

pub struct QemuInstance {
    child: Child,
    pub vnc_display: u16,
    pub ws_port: u16,
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

        // 2. Prepare Virtual Disk
        let disk_path_str = config.disk_path.clone().unwrap_or_else(|| {
            format!("{}_disk.qcow2", config.name)
        });
        
        // Security Check: Prevent overwriting host devices
        if disk_path_str.starts_with("/dev/") || disk_path_str.starts_with("/sys/") || disk_path_str.starts_with("/proc/") {
            return Err(format!("Security Violation: Disk path '{}' points to a host system device or directory.", disk_path_str));
        }
        
        let disk_path = Path::new(&disk_path_str);
        if !disk_path.exists() {
            Self::create_qcow2_disk(qemu_img_bin, disk_path.to_str().unwrap(), 20)?; // Default to 20GB disk if not present
        }

        let ws_port = Self::get_free_ws_port();
        let vnc_display = Self::get_free_vnc_display();

        // 3. Construct QEMU arguments
        let safe_cpu_cores = std::cmp::min(config.cpu_cores, 64);
        let safe_ram_size = std::cmp::min(config.ram_size_mb, 65536);

        let mut args = vec![
            "-enable-kvm".to_string(),
            "-cpu".to_string(), "host".to_string(),
            "-smp".to_string(), safe_cpu_cores.to_string(),
            "-m".to_string(), safe_ram_size.to_string(),
            // Display & GPU settings: VNC Server with WebSocket for UI embedding
            "-device".to_string(), "virtio-vga".to_string(),
            "-display".to_string(), "none".to_string(),
            "-vnc".to_string(), format!("127.0.0.1:{},websocket={}", vnc_display, ws_port),
            // Input Devices (Keyboard & Mouse)
            "-device".to_string(), "virtio-keyboard-pci".to_string(),
            "-device".to_string(), "virtio-mouse-pci".to_string(),
            // Storage
            "-drive".to_string(), format!("file={},format=qcow2,if=virtio", disk_path.to_str().unwrap()),
        ];

        // Network
        if config.network_mode == "bridge" {
            args.push("-netdev".to_string());
            args.push("bridge,id=net0,br=br0".to_string());
            args.push("-device".to_string());
            args.push("virtio-net-pci,netdev=net0".to_string());
        } else if config.network_mode == "tap" {
            args.push("-netdev".to_string());
            args.push("tap,id=net0,ifname=tap0,script=no,downscript=no".to_string());
            args.push("-device".to_string());
            args.push("virtio-net-pci,netdev=net0".to_string());
        } else {
            // Default: user mode NAT
            let mut netdev = "user,id=net0".to_string();
            for (hport, gport) in &config.advanced.host_port_forwards {
                netdev.push_str(&format!(",hostfwd=tcp::{}-:{}", hport, gport));
            }
            args.push("-netdev".to_string());
            args.push(netdev);
            args.push("-device".to_string());
            args.push("virtio-net-pci,netdev=net0".to_string());
        }

        // Advanced: Audio
        if config.advanced.audio_enabled {
            args.push("-audiodev".to_string());
            args.push("pa,id=snd0".to_string()); // PulseAudio as default
            args.push("-device".to_string());
            args.push("intel-hda".to_string());
            args.push("-device".to_string());
            args.push("hda-output,audiodev=snd0".to_string());
        }

        // Advanced: Additional Disks
        for (i, disk_path_str) in config.advanced.additional_disks.iter().enumerate() {
            if disk_path_str.starts_with("/dev/") || disk_path_str.starts_with("/sys/") || disk_path_str.starts_with("/proc/") {
                return Err(format!("Security Violation: Additional disk path '{}' points to a host system device.", disk_path_str));
            }
            if Path::new(disk_path_str).exists() {
                args.push("-drive".to_string());
                args.push(format!("file={},format=qcow2,if=virtio,id=drive{}", disk_path_str, i + 1));
            }
        }

        if config.firmware.as_deref() == Some("uefi") {
            args.push("-bios".to_string());
            let bios_paths = [
                "/usr/share/OVMF/OVMF_CODE.fd",          // Ubuntu/Debian
                "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd", // Arch
                "/usr/share/OVMF/OVMF_CODE_4M.fd",       // Alpine/RHEL
            ];
            let mut found_bios = false;
            for path in bios_paths {
                if Path::new(path).exists() {
                    args.push(path.to_string());
                    found_bios = true;
                    break;
                }
            }
            if !found_bios {
                args.push("/usr/share/OVMF/OVMF_CODE.fd".to_string()); // Default fallback
            }
        }

        // Boot from ISO if provided (for OS installers)
        if let Some(iso) = &config.iso_path {
            if iso.starts_with("/dev/") || iso.starts_with("/sys/") || iso.starts_with("/proc/") {
                return Err(format!("Security Violation: ISO path '{}' points to a host system device or directory.", iso));
            }
            if Path::new(iso).exists() {
                args.push("-cdrom".to_string());
                args.push(iso.clone());
                args.push("-boot".to_string());
                args.push("d".to_string()); // Boot from CD-ROM first
            }
        }

        // Create socket directory if it doesn't exist
        let socket_dir = Path::new("/tmp/alouette_vms");
        if !socket_dir.exists() {
            let _ = std::fs::create_dir_all(socket_dir);
        }

        // QMP (QEMU Monitor Protocol) over Unix Socket
        let qmp_socket_path = socket_dir.join(format!("{}_qmp.sock", config.id));
        args.push("-qmp".to_string());
        args.push(format!("unix:{},server=on,wait=off", qmp_socket_path.to_string_lossy()));

        // QEMU Guest Agent (QGA) over virtio-serial
        let qga_socket_path = socket_dir.join(format!("{}_qga.sock", config.id));
        args.push("-chardev".to_string());
        args.push(format!("socket,path={},server=on,wait=off,id=qga0", qga_socket_path.to_string_lossy()));
        args.push("-device".to_string());
        args.push("virtio-serial".to_string());
        args.push("-device".to_string());
        args.push("virtserialport,chardev=qga0,name=org.qemu.guest_agent.0".to_string());

        // 4. Start the QEMU process
        let log_file_path = Path::new(&config.vm_dir).join("qemu_error.log");
        let log_file_out = std::fs::File::create(&log_file_path).ok();
        let log_file_err = log_file_out.as_ref().and_then(|f| f.try_clone().ok());

        let mut cmd = Command::new(qemu_bin);
        cmd.args(&args);
        
        if let Some(f) = log_file_out {
            cmd.stdout(std::process::Stdio::from(f));
        }
        if let Some(f) = log_file_err {
            cmd.stderr(std::process::Stdio::from(f));
        }

        let child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn QEMU process ({}): {}", qemu_bin, e))?;

        Ok(Self { child, vnc_display, ws_port })
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

    fn get_free_vnc_display() -> u16 {
        for port in 5900..6000 {
            if std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok() {
                return port - 5900;
            }
        }
        0 // fallback if all 100 ports are taken
    }

    fn get_free_ws_port() -> u16 {
        for port in 5700..5800 {
            if std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok() {
                return port;
            }
        }
        5700 // fallback
    }
}
