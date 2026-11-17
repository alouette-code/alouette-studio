use crate::vm_engine::config::VmConfig;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub const SOCKET_DIR: &str = "/tmp/alouette_vms";

pub struct QemuInstance {
    child: Child,
    swtpm_child: Option<Child>,
    pub vnc_display: u16,
    pub ws_port: u16,
    qmp_socket_path: PathBuf,
    qga_socket_path: PathBuf,
    tpm_socket_path: Option<PathBuf>,
}

impl QemuInstance {
    pub fn qmp_socket_path(id: &str) -> PathBuf {
        Path::new(SOCKET_DIR).join(format!("{}_qmp.sock", id))
    }

    pub fn qga_socket_path(id: &str) -> PathBuf {
        Path::new(SOCKET_DIR).join(format!("{}_qga.sock", id))
    }

    pub fn tpm_socket_path(id: &str) -> PathBuf {
        Path::new(SOCKET_DIR).join(format!("{}_swtpm.sock", id))
    }

    fn is_restricted_system_path(path: &str) -> bool {
        path.starts_with("/dev/") || path.starts_with("/sys/") || path.starts_with("/proc/")
    }

    fn is_kvm_available() -> bool {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/kvm")
            .is_ok()
    }

    fn is_windows_os(os_type: Option<&str>) -> bool {
        os_type
            .map(|o| o.to_lowercase().starts_with("win"))
            .unwrap_or(false)
    }

    fn find_ovmf_pair(prefer_ms_secboot: bool) -> Option<(&'static str, &'static str)> {
        let ms_pairs = [
            ("/usr/share/OVMF/OVMF_CODE_4M.ms.fd", "/usr/share/OVMF/OVMF_VARS_4M.ms.fd"),
            ("/usr/share/OVMF/OVMF_CODE_4M.secboot.fd", "/usr/share/OVMF/OVMF_VARS_4M.fd"),
            ("/usr/share/OVMF/OVMF_CODE_4M.fd", "/usr/share/OVMF/OVMF_VARS_4M.fd"),
            ("/usr/share/edk2-ovmf/x64/OVMF_CODE.fd", "/usr/share/edk2-ovmf/x64/OVMF_VARS.fd"),
            ("/usr/share/edk2/x64/OVMF_CODE.fd", "/usr/share/edk2/x64/OVMF_VARS.fd"),
        ];
        let std_pairs = [
            ("/usr/share/OVMF/OVMF_CODE_4M.fd", "/usr/share/OVMF/OVMF_VARS_4M.fd"),
            ("/usr/share/OVMF/OVMF_CODE_4M.ms.fd", "/usr/share/OVMF/OVMF_VARS_4M.ms.fd"),
            ("/usr/share/edk2-ovmf/x64/OVMF_CODE.fd", "/usr/share/edk2-ovmf/x64/OVMF_VARS.fd"),
            ("/usr/share/edk2/x64/OVMF_CODE.fd", "/usr/share/edk2/x64/OVMF_VARS.fd"),
        ];
        let list: &[(&str, &str)] = if prefer_ms_secboot {
            &ms_pairs
        } else {
            &std_pairs
        };
        for &(code, vars) in list {
            if Path::new(code).exists() && Path::new(vars).exists() {
                return Some((code, vars));
            }
        }
        None
    }

    fn find_ovmf_code_only() -> Option<&'static str> {
        let candidates = [
            "/usr/share/OVMF/OVMF_CODE.fd",
            "/usr/share/OVMF/OVMF_CODE_4M.fd",
            "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd",
            "/usr/share/edk2/x64/OVMF_CODE.fd",
        ];
        candidates.into_iter().find(|p| Path::new(p).exists())
    }

    fn find_aavmf_code() -> Option<&'static str> {
        let candidates = [
            "/usr/share/AAVMF/AAVMF_CODE.fd",
            "/usr/share/qemu-efi-aarch64/QEMU_EFI.fd",
            "/usr/share/edk2/aarch64/QEMU_EFI.fd",
        ];
        candidates.into_iter().find(|p| Path::new(p).exists())
    }

    /// Spawns a QEMU virtual machine process based on the config.
    pub fn spawn(
        config: &VmConfig,
        qemu_path: Option<&str>,
        qemu_img_path: Option<&str>,
        reserved_vnc: &HashSet<u16>,
        reserved_ws: &HashSet<u16>,
    ) -> Result<Self, String> {
        let is_arm64 = config.arch.as_deref() == Some("aarch64");
        let qemu_bin = if is_arm64 {
            "qemu-system-aarch64"
        } else {
            qemu_path.unwrap_or("qemu-system-x86_64")
        };
        let qemu_img_bin = qemu_img_path.unwrap_or("qemu-img");

        if !Self::is_qemu_installed(qemu_bin) {
            if is_arm64 {
                return Err(
                    "ARM64 emulator 'qemu-system-aarch64' not found on host. Install it with: sudo apt install qemu-system-arm qemu-efi-aarch64"
                        .to_string(),
                );
            }
            return Err(format!(
                "QEMU binary not found or not executable at '{}'. Please ensure QEMU is installed.",
                qemu_bin
            ));
        }

        // Prepare Virtual Disk
        let disk_path_str = config
            .disk_path
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                let safe_name = VmConfig::sanitize_name(&config.name);
                Path::new(&config.vm_dir)
                    .join(format!("{}.qcow2", safe_name))
                    .to_string_lossy()
                    .into_owned()
            });

        if Self::is_restricted_system_path(&disk_path_str) {
            return Err(format!(
                "Security Violation: Disk path '{}' points to a host system device.",
                disk_path_str
            ));
        }

        let disk_path = Path::new(&disk_path_str);
        if !disk_path.exists() {
            let disk_size_gb = config.disk_size_gb.unwrap_or(20).clamp(1, 2000);
            Self::create_qcow2_disk(qemu_img_bin, &disk_path_str, disk_size_gb)?;
        }

        let ws_port = Self::get_free_ws_port(reserved_ws)?;
        let vnc_display = Self::get_free_vnc_display(reserved_vnc)?;

        let safe_cpu_cores = config.cpu_cores.clamp(1, 64);
        let safe_ram_size = config.ram_size_mb.clamp(128, 65536);
        let is_windows = Self::is_windows_os(config.os_type.as_deref());
        let is_uefi = config.firmware.as_deref() == Some("uefi");

        let vm_dir_path = Path::new(&config.vm_dir);
        let _ = std::fs::create_dir_all(vm_dir_path);

        let socket_dir = Path::new(SOCKET_DIR);
        if !socket_dir.exists() {
            let _ = std::fs::create_dir_all(socket_dir);
        }

        let mut args = Vec::new();

        if is_arm64 {
            args.extend([
                "-M".to_string(),
                "virt".to_string(),
                "-cpu".to_string(),
                "cortex-a72".to_string(),
                "-smp".to_string(),
                safe_cpu_cores.to_string(),
                "-m".to_string(),
                safe_ram_size.to_string(),
                "-device".to_string(),
                "virtio-gpu-pci".to_string(),
                "-device".to_string(),
                "qemu-xhci".to_string(),
                "-device".to_string(),
                "usb-kbd".to_string(),
                "-device".to_string(),
                "usb-tablet".to_string(),
            ]);

            if let Some(aavmf) = Self::find_aavmf_code() {
                args.push("-bios".to_string());
                args.push(aavmf.to_string());
            } else {
                return Err(
                    "ARM64 UEFI firmware (AAVMF_CODE.fd / QEMU_EFI.fd) not found. Install 'qemu-efi-aarch64'."
                        .to_string(),
                );
            }
        } else {
            if is_uefi {
                args.push("-machine".to_string());
                args.push("q35".to_string());
            }

            if Self::is_kvm_available() {
                args.push("-enable-kvm".to_string());
                args.push("-cpu".to_string());
                if is_windows {
                    args.push("host,hv_relaxed,hv_spinlocks=0x1fff,hv_vapic,hv_time".to_string());
                } else {
                    args.push("host".to_string());
                }
            } else {
                args.push("-cpu".to_string());
                args.push("max".to_string());
            }

            args.extend([
                "-smp".to_string(),
                safe_cpu_cores.to_string(),
                "-m".to_string(),
                safe_ram_size.to_string(),
                "-device".to_string(),
                "virtio-vga".to_string(),
                // Input Devices (tablet provides absolute [0..32767] coords for AI precision)
                "-device".to_string(),
                "virtio-keyboard-pci".to_string(),
                "-device".to_string(),
                "virtio-mouse-pci".to_string(),
                "-device".to_string(),
                "virtio-tablet-pci".to_string(),
            ]);

            // UEFI Firmware (OVMF pflash pair or fallback -bios)
            if is_uefi {
                if let Some((code_path, vars_template)) = Self::find_ovmf_pair(is_windows) {
                    let vm_vars_path = vm_dir_path.join("OVMF_VARS.fd");
                    if !vm_vars_path.exists() {
                        std::fs::copy(vars_template, &vm_vars_path).map_err(|e| {
                            format!("Failed to initialize per-VM OVMF_VARS.fd: {}", e)
                        })?;
                    }
                    args.push("-drive".to_string());
                    args.push(format!("if=pflash,format=raw,readonly=on,file={}", code_path));
                    args.push("-drive".to_string());
                    args.push(format!(
                        "if=pflash,format=raw,file={}",
                        vm_vars_path.to_string_lossy()
                    ));
                } else if let Some(code_only) = Self::find_ovmf_code_only() {
                    args.push("-bios".to_string());
                    args.push(code_only.to_string());
                } else {
                    return Err(
                        "UEFI firmware (OVMF_CODE.fd) not found on host. Install 'ovmf' package or switch firmware to BIOS."
                            .to_string(),
                    );
                }
            }
        }

        // Display & VNC WebSocket for embedded noVNC + shared AI framebuffer stream
        args.extend([
            "-display".to_string(),
            "none".to_string(),
            "-vnc".to_string(),
            format!("127.0.0.1:{},websocket={},share=ignore", vnc_display, ws_port),
            // Primary Storage
            "-drive".to_string(),
            format!("file={},format=qcow2,if=virtio", disk_path_str),
        ]);

        // Optional TPM 2.0 Emulator (required for Windows 11)
        let mut swtpm_child: Option<Child> = None;
        let mut tpm_socket_path_opt: Option<PathBuf> = None;
        if config.advanced.tpm_enabled {
            if !Self::is_binary_available("swtpm") {
                return Err(
                    "TPM 2.0 is enabled for this VM, but 'swtpm' is not installed on the host. Install it (e.g. `sudo apt install swtpm`) or disable TPM 2.0 in Hardware settings."
                        .to_string(),
                );
            }
            let tpm_state_dir = vm_dir_path.join("tpm_state");
            std::fs::create_dir_all(&tpm_state_dir)
                .map_err(|e| format!("Failed to create TPM state dir: {}", e))?;
            let tpm_sock = Self::tpm_socket_path(&config.id);
            let _ = std::fs::remove_file(&tpm_sock);

            let mut tpm_cmd = Command::new("swtpm");
            tpm_cmd.args([
                "socket",
                "--tpmstate",
                &format!("dir={}", tpm_state_dir.to_string_lossy()),
                "--ctrl",
                &format!("type=unixio,path={}", tpm_sock.to_string_lossy()),
                "--tpm2",
            ]);
            #[cfg(unix)]
            unsafe {
                tpm_cmd.pre_exec(|| {
                    libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0);
                    Ok(())
                });
            }
            let child_tpm = tpm_cmd
                .spawn()
                .map_err(|e| format!("Failed to start swtpm daemon: {}", e))?;

            // Wait up to 1s for swtpm socket creation
            for _ in 0..20 {
                if tpm_sock.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            if !tpm_sock.exists() {
                return Err("swtpm daemon started but did not create control socket in time".to_string());
            }

            args.push("-chardev".to_string());
            args.push(format!("socket,id=chrtpm,path={}", tpm_sock.to_string_lossy()));
            args.push("-tpmdev".to_string());
            args.push("emulator,id=tpm0,chardev=chrtpm".to_string());
            args.push("-device".to_string());
            args.push(if is_arm64 {
                "tpm-tis-device,tpmdev=tpm0".to_string()
            } else {
                "tpm-tis,tpmdev=tpm0".to_string()
            });

            swtpm_child = Some(child_tpm);
            tpm_socket_path_opt = Some(tpm_sock);
        }

        // Network configuration
        let net_mode = VmConfig::normalize_network_mode(&config.network_mode);
        match net_mode.as_str() {
            "bridge" => {
                args.push("-netdev".to_string());
                args.push("bridge,id=net0,br=br0".to_string());
            }
            "tap" => {
                args.push("-netdev".to_string());
                args.push("tap,id=net0,ifname=tap0,script=no,downscript=no".to_string());
            }
            "host-only" => {
                let mut netdev = "user,id=net0,restrict=on".to_string();
                for (hport, gport) in &config.advanced.host_port_forwards {
                    if *hport > 0 && *gport > 0 {
                        netdev.push_str(&format!(",hostfwd=tcp:127.0.0.1:{}-:{}", hport, gport));
                    }
                }
                args.push("-netdev".to_string());
                args.push(netdev);
            }
            _ => {
                let mut netdev = "user,id=net0".to_string();
                for (hport, gport) in &config.advanced.host_port_forwards {
                    if *hport > 0 && *gport > 0 {
                        netdev.push_str(&format!(",hostfwd=tcp:127.0.0.1:{}-:{}", hport, gport));
                    }
                }
                args.push("-netdev".to_string());
                args.push(netdev);
            }
        }
        args.push("-device".to_string());
        args.push("virtio-net-pci,netdev=net0".to_string());

        // Optional Audio
        if config.advanced.audio_enabled {
            args.push("-audiodev".to_string());
            args.push("pa,id=snd0".to_string());
            args.push("-device".to_string());
            args.push("intel-hda".to_string());
            args.push("-device".to_string());
            args.push("hda-output,audiodev=snd0".to_string());
        }

        // Additional Disks
        for (i, extra_disk) in config.advanced.additional_disks.iter().enumerate() {
            if Self::is_restricted_system_path(extra_disk) {
                return Err(format!(
                    "Security Violation: Additional disk path '{}' points to a host system device.",
                    extra_disk
                ));
            }
            if Path::new(extra_disk).exists() {
                args.push("-drive".to_string());
                args.push(format!(
                    "file={},format=qcow2,if=virtio,id=drive{}",
                    extra_disk,
                    i + 1
                ));
            }
        }

        // Optional ISO image
        if let Some(iso) = &config.iso_path {
            if Self::is_restricted_system_path(iso) {
                return Err(format!(
                    "Security Violation: ISO path '{}' points to a host system device.",
                    iso
                ));
            }
            if !Path::new(iso).exists() {
                return Err(format!("ISO file not found at '{}'", iso));
            }
            args.push("-cdrom".to_string());
            args.push(iso.clone());
            if !is_arm64 {
                args.push("-boot".to_string());
                args.push("order=dc".to_string());
            }
        }

        let qmp_socket_path = Self::qmp_socket_path(&config.id);
        let qga_socket_path = Self::qga_socket_path(&config.id);
        let _ = std::fs::remove_file(&qmp_socket_path);
        let _ = std::fs::remove_file(&qga_socket_path);

        // QMP over Unix Socket
        args.push("-qmp".to_string());
        args.push(format!(
            "unix:{},server=on,wait=off",
            qmp_socket_path.to_string_lossy()
        ));

        // QGA over virtio-serial
        args.push("-chardev".to_string());
        args.push(format!(
            "socket,path={},server=on,wait=off,id=qga0",
            qga_socket_path.to_string_lossy()
        ));
        args.push("-device".to_string());
        args.push("virtio-serial".to_string());
        args.push("-device".to_string());
        args.push("virtserialport,chardev=qga0,name=org.qemu.guest_agent.0".to_string());

        // Capture real guest serial output to serial.log inside vm_dir
        let serial_log_path = vm_dir_path.join("serial.log");
        args.push("-chardev".to_string());
        args.push(format!(
            "file,id=serial0,path={}",
            serial_log_path.to_string_lossy()
        ));
        args.push("-serial".to_string());
        args.push("chardev:serial0".to_string());

        // Start QEMU process
        let log_file_path = vm_dir_path.join("qemu_error.log");
        let log_file_out = std::fs::File::create(&log_file_path).ok();
        let log_file_err = log_file_out.as_ref().and_then(|f| f.try_clone().ok());

        let mut cmd = Command::new(qemu_bin);
        cmd.args(&args);

        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0);
                Ok(())
            });
        }

        if let Some(f) = log_file_out {
            cmd.stdout(std::process::Stdio::from(f));
        }
        if let Some(f) = log_file_err {
            cmd.stderr(std::process::Stdio::from(f));
        }

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                if let Some(mut tpm) = swtpm_child {
                    let _ = tpm.kill();
                    let _ = tpm.wait();
                }
                if let Some(sock) = tpm_socket_path_opt {
                    let _ = std::fs::remove_file(sock);
                }
                return Err(format!("Failed to spawn QEMU process ({}): {}", qemu_bin, e));
            }
        };

        Ok(Self {
            child,
            swtpm_child,
            vnc_display,
            ws_port,
            qmp_socket_path,
            qga_socket_path,
            tpm_socket_path: tpm_socket_path_opt,
        })
    }

    /// Kills the running QEMU process (and swtpm if active) and cleans up sockets.
    pub fn kill(&mut self) -> Result<(), String> {
        if self.is_running() {
            self.child
                .kill()
                .map_err(|e| format!("Failed to kill QEMU VM process: {}", e))?;
            let _ = self.child.wait();
        }
        if let Some(mut tpm) = self.swtpm_child.take() {
            let _ = tpm.kill();
            let _ = tpm.wait();
        }
        let _ = std::fs::remove_file(&self.qmp_socket_path);
        let _ = std::fs::remove_file(&self.qga_socket_path);
        if let Some(tpm_sock) = &self.tpm_socket_path {
            let _ = std::fs::remove_file(tpm_sock);
        }
        Ok(())
    }

    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn is_binary_available(bin: &str) -> bool {
        Command::new(bin)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn is_qemu_installed(qemu_bin: &str) -> bool {
        Self::is_binary_available(qemu_bin)
    }

    pub fn create_qcow2_disk(qemu_img_bin: &str, path: &str, size_gb: u32) -> Result<(), String> {
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create disk parent directory: {}", e))?;
        }

        let output = Command::new(qemu_img_bin)
            .args(["create", "-f", "qcow2", path, &format!("{}G", size_gb)])
            .output()
            .map_err(|e| format!("Failed to invoke qemu-img ({}): {}", qemu_img_bin, e))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("qemu-img disk creation failed: {}", err_msg));
        }

        Ok(())
    }

    fn get_free_vnc_display(reserved: &HashSet<u16>) -> Result<u16, String> {
        for display in 0..100u16 {
            if reserved.contains(&display) {
                continue;
            }
            let port = 5900 + display;
            if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
                return Ok(display);
            }
        }
        Err("No free VNC display ports available (5900-5999)".to_string())
    }

    fn get_free_ws_port(reserved: &HashSet<u16>) -> Result<u16, String> {
        for port in 5700..5800u16 {
            if reserved.contains(&port) {
                continue;
            }
            if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
                return Ok(port);
            }
        }
        Err("No free VNC WebSocket ports available (5700-5799)".to_string())
    }
}

impl Drop for QemuInstance {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}
