use kvm_ioctls::{Kvm, VmFd};
use kvm_bindings::kvm_userspace_memory_region;
use std::ptr;
use crate::vm_engine::config::VmConfig;


pub struct KvmVm {
    pub kvm: Kvm,
    pub vm_fd: VmFd,
    pub ram_ptr: *mut libc::c_void,
    pub ram_size: usize,
}

// KvmVm contains raw pointers but we control its lifetime and access.
unsafe impl Send for KvmVm {}
unsafe impl Sync for KvmVm {}

impl KvmVm {
    pub fn new(config: &VmConfig) -> Result<Self, String> {
        // 1. Open /dev/kvm
        let kvm = Kvm::new().map_err(|e| format!("Failed to open /dev/kvm: {}. Make sure you have KVM permissions.", e))?;

        // 2. Create the VM
        let vm_fd = kvm.create_vm().map_err(|e| format!("Failed to create KVM VM: {}", e))?;

        // 3. Allocate page-aligned guest physical memory (RAM)
        let ram_size = (config.ram_size_mb * 1024 * 1024) as usize;
        let ram_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                ram_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_PRIVATE | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };

        if ram_ptr == libc::MAP_FAILED {
            return Err("Failed to mmap guest memory region".to_string());
        }

        // 4. Register the memory slot with KVM
        // Slot 0 is guest memory starting at address 0x0
        let mem_region = kvm_userspace_memory_region {
            slot: 0,
            guest_phys_addr: 0x0,
            memory_size: ram_size as u64,
            userspace_addr: ram_ptr as u64,
            flags: 0,
        };

        unsafe {
            vm_fd.set_user_memory_region(mem_region)
                .map_err(|e| format!("Failed to set user memory region: {}", e))?;
        }

        Ok(Self {
            kvm,
            vm_fd,
            ram_ptr,
            ram_size,
        })
    }

    /// Loads a simple bootloader / mock binary or flat kernel into memory.
    pub fn load_kernel(&self, kernel_path: Option<&str>) -> Result<u64, String> {
        let load_addr = 0x100000; // Load kernel at 1MB boundary
        if let Some(path) = kernel_path {
            if std::path::Path::new(path).exists() {
                let kernel_data = std::fs::read(path)
                    .map_err(|e| format!("Failed to read kernel file: {}", e))?;
                
                if kernel_data.len() + load_addr as usize > self.ram_size {
                    return Err("Kernel file is larger than allocated guest RAM".to_string());
                }

                unsafe {
                    let dest = (self.ram_ptr as usize + load_addr as usize) as *mut u8;
                    ptr::copy_nonoverlapping(kernel_data.as_ptr(), dest, kernel_data.len());
                }
                return Ok(load_addr);
            }
        }

        // Fallback: load a tiny mock loop program if no kernel file is specified/found.
        // This program performs simple I/O writes to the COM1 port (0x3f8) in a loop.
        // Assembly equivalent:
        //   mov al, 'H'
        //   out 0xf8, al  (port 0x3f8, using offset/dx configuration)
        //   jmp $
        let mock_code: [u8; 11] = [
            0xb0, 0x48,             // mov al, 0x48 ('H')
            0xba, 0xf8, 0x03,       // mov dx, 0x3f8
            0xee,                   // out dx, al
            0xb0, 0x0a,             // mov al, 0x0a ('\n')
            0xee,                   // out dx, al
            0xeb, 0xfe,             // jmp -2 (infinite loop)
        ];

        unsafe {
            let dest = (self.ram_ptr as usize + load_addr as usize) as *mut u8;
            ptr::copy_nonoverlapping(mock_code.as_ptr(), dest, mock_code.len());
        }

        Ok(load_addr)
    }
}

impl Drop for KvmVm {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ram_ptr, self.ram_size);
        }
    }
}
