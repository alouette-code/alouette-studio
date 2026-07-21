use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use serde_json::{json, Value};
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub struct QgaClient {
    stream: UnixStream,
    buffer: Vec<u8>,
}

impl QgaClient {
    /// Connects to a QGA Unix socket.
    pub fn connect<P: AsRef<Path>>(socket_path: P) -> Result<Self, String> {
        let stream = UnixStream::connect(socket_path)
            .map_err(|e| format!("Failed to connect to QGA socket: {}", e))?;
        
        stream.set_read_timeout(Some(Duration::from_secs(10))).map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(Duration::from_secs(10))).map_err(|e| e.to_string())?;

        Ok(Self { stream, buffer: Vec::new() })
    }

    /// Reads a single JSON object response from the socket.
    fn read_response(&mut self) -> Result<Value, String> {
        let mut read_buf = [0; 8192];

        loop {
            // Check if we already have a newline in the buffer
            if let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
                let line = self.buffer[..pos].to_vec();
                self.buffer.drain(..=pos); // Consume line + newline
                
                if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                    return Ok(value);
                } else {
                    continue; // Ignore invalid lines and look for next
                }
            }

            let bytes_read = self.stream.read(&mut read_buf)
                .map_err(|e| format!("Failed to read from QGA socket: {}", e))?;
            
            if bytes_read == 0 {
                return Err("QGA socket closed unexpectedly".to_string());
            }

            self.buffer.extend_from_slice(&read_buf[..bytes_read]);
        }
    }

    /// Executes a QGA command and waits for the result or error.
    pub fn execute(&mut self, command: &str, arguments: Option<Value>) -> Result<Value, String> {
        let mut req = json!({
            "execute": command
        });
        
        if let Some(args) = arguments {
            req.as_object_mut().unwrap().insert("arguments".to_string(), args);
        }

        let mut req_str = req.to_string();
        req_str.push('\n');

        self.stream.write_all(req_str.as_bytes())
            .map_err(|e| format!("Failed to send QGA command: {}", e))?;

        loop {
            let response = self.read_response()?;
            if let Some(err) = response.get("error") {
                let desc = err.get("desc").and_then(|d| d.as_str()).unwrap_or("Unknown error");
                return Err(format!("QGA Error: {}", desc));
            }
            if let Some(res) = response.get("return") {
                return Ok(res.clone());
            }
        }
    }

    // --- High Level File Transfer API ---

    pub fn guest_file_open(&mut self, path: &str, mode: &str) -> Result<i64, String> {
        let res = self.execute("guest-file-open", Some(json!({
            "path": path,
            "mode": mode
        })))?;
        res.as_i64().ok_or_else(|| "Invalid handle returned from guest-file-open".to_string())
    }

    pub fn guest_file_write(&mut self, handle: i64, data: &[u8]) -> Result<usize, String> {
        let encoded_data = BASE64.encode(data);
        let res = self.execute("guest-file-write", Some(json!({
            "handle": handle,
            "buf-b64": encoded_data
        })))?;
        
        res.get("count")
           .and_then(|c| c.as_u64())
           .map(|c| c as usize)
           .ok_or_else(|| "Failed to parse write count".to_string())
    }

    pub fn guest_file_close(&mut self, handle: i64) -> Result<(), String> {
        self.execute("guest-file-close", Some(json!({
            "handle": handle
        })))?;
        Ok(())
    }
}
