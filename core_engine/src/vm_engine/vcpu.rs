use kvm_ioctls::{VcpuFd, VcpuExit};
use kvm_bindings::kvm_regs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::vm_engine::device::SerialDevice;

pub struct Vcpu {
    pub vcpu_fd: VcpuFd,
    pub serial_device: SerialDevice,
}

impl Vcpu {
    pub fn new(vcpu_fd: VcpuFd, serial_device: SerialDevice) -> Self {
        Self {
            vcpu_fd,
            serial_device,
        }
    }

    /// Sets up vCPU registers for booting in 16-bit Real Mode or flat 32-bit mode
    pub fn setup_registers(&self, entry_point: u64) -> Result<(), String> {
        // Configure special registers (sregs)
        let mut sregs = self.vcpu_fd.get_sregs()
            .map_err(|e| format!("Failed to get sregs: {}", e))?;

        // Initialize CS register to point to 0
        sregs.cs.base = 0;
        sregs.cs.selector = 0;

        self.vcpu_fd.set_sregs(&sregs)
            .map_err(|e| format!("Failed to set sregs: {}", e))?;

        // Configure general purpose registers (regs)
        let regs = kvm_regs {
            rip: entry_point,
            rflags: 2, // Bit 1 is always set to 1 in rflags
            ..Default::default()
        };

        self.vcpu_fd.set_regs(&regs)
            .map_err(|e| format!("Failed to set regs: {}", e))?;

        Ok(())
    }

    /// Starts the vCPU execution loop in the current thread.
    /// Runs until `should_stop` is set to true, or the vCPU halts/errors.
    pub fn run_loop(&mut self, should_stop: Arc<AtomicBool>) -> Result<(), String> {
        while !should_stop.load(Ordering::Relaxed) {
            match self.vcpu_fd.run() {
                Ok(exit_reason) => match exit_reason {
                    VcpuExit::IoIn(addr, data) => {
                        // Handle serial read at COM1 (0x3f8 - 0x3ff)
                        if (0x3f8..=0x3ff).contains(&addr) {
                            let offset = (addr - 0x3f8) as u64;
                            self.serial_device.handle_read(offset, data);
                        }
                    }
                    VcpuExit::IoOut(addr, data) => {
                        // Handle serial write at COM1 (0x3f8 - 0x3ff)
                        if (0x3f8..=0x3ff).contains(&addr) {
                            let offset = (addr - 0x3f8) as u64;
                            self.serial_device.handle_write(offset, data);
                        }
                    }
                    VcpuExit::Hlt => {
                        let _ = self.serial_device.handle_write(0, b"\n[VM Halted]\n");
                        break;
                    }
                    VcpuExit::InternalError => {
                        return Err("KVM internal emulation error".to_string());
                    }
                    _ => {
                        // Handle other exits (e.g. MMIO, hypercall, reboot, etc.)
                    }
                },
                Err(e) => {
                    // EINTR is returned when a signal is received (e.g. thread interruption).
                    // We should check should_stop and continue.
                    if e.errno() == libc::EINTR {
                        continue;
                    }
                    return Err(format!("vCPU execution failed: {}", e));
                }
            }
        }
        Ok(())
    }
}
