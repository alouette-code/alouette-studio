use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Clone)]
pub struct SerialDevice {
    buffer: Arc<Mutex<Vec<u8>>>,
    callback: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl SerialDevice {
    pub fn new(callback: Option<Arc<dyn Fn(&str) + Send + Sync>>) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            callback,
        }
    }

    pub fn handle_write(&self, offset: u64, data: &[u8]) {
        // Serial TX is at offset 0 (COM1 base is 0x3f8)
        if offset == 0 {
            for &byte in data {
                // Buffer the byte
                let mut buf = self.buffer.lock();
                buf.push(byte);
                
                // If it's a newline or buffer is large, flush to callback
                if byte == b'\n' || buf.len() >= 80 {
                    if let Ok(s) = std::str::from_utf8(&buf) {
                        if let Some(cb) = &self.callback {
                            cb(s);
                        }
                    }
                    buf.clear();
                }
            }
        }
    }

    pub fn handle_read(&self, offset: u64, data: &mut [u8]) {
        // Offset 5 is Line Status Register (LSR)
        // We return LSR empty (0x20 / 0x60) so KVM kernel thinks UART can accept write
        if offset == 5 {
            if !data.is_empty() {
                data[0] = 0x60; // Transmitter empty + Transmitter holding register empty
            }
        } else if !data.is_empty() {
            data[0] = 0;
        }
    }

    pub fn get_buffered_logs(&self) -> String {
        let buf = self.buffer.lock();
        String::from_utf8_lossy(&buf).into_owned()
    }
}
