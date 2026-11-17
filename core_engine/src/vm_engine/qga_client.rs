use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

pub struct QgaClient {
    stream: UnixStream,
    buffer: Vec<u8>,
}

impl QgaClient {
    /// Connects to a QGA Unix socket.
    pub fn connect<P: AsRef<Path>>(socket_path: P) -> Result<Self, String> {
        let stream = UnixStream::connect(socket_path)
            .map_err(|e| format!("Failed to connect to QGA socket: {}", e))?;

        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| e.to_string())?;

        Ok(Self {
            stream,
            buffer: Vec::new(),
        })
    }

    fn read_response(&mut self) -> Result<Value, String> {
        let mut read_buf = [0; 8192];

        loop {
            if let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
                let line = self.buffer[..pos].to_vec();
                self.buffer.drain(..=pos);

                if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                    return Ok(value);
                } else {
                    continue;
                }
            }

            let bytes_read = self
                .stream
                .read(&mut read_buf)
                .map_err(|e| format!("Failed to read from QGA socket: {}", e))?;

            if bytes_read == 0 {
                return Err("QGA socket closed unexpectedly".to_string());
            }

            self.buffer.extend_from_slice(&read_buf[..bytes_read]);
        }
    }

    pub fn execute(&mut self, command: &str, arguments: Option<Value>) -> Result<Value, String> {
        let mut req = json!({
            "execute": command
        });

        if let Some(args) = arguments {
            req.as_object_mut()
                .unwrap()
                .insert("arguments".to_string(), args);
        }

        let mut req_str = req.to_string();
        req_str.push('\n');

        self.stream
            .write_all(req_str.as_bytes())
            .map_err(|e| format!("Failed to send QGA command: {}", e))?;

        loop {
            let response = self.read_response()?;
            if let Some(err) = response.get("error") {
                let desc = err
                    .get("desc")
                    .and_then(|d| d.as_str())
                    .unwrap_or("Unknown error");
                return Err(format!("QGA Error: {}", desc));
            }
            if let Some(res) = response.get("return") {
                return Ok(res.clone());
            }
        }
    }

    // --- File Transfer API ---

    pub fn guest_file_open(&mut self, path: &str, mode: &str) -> Result<i64, String> {
        let res = self.execute(
            "guest-file-open",
            Some(json!({
                "path": path,
                "mode": mode
            })),
        )?;
        res.as_i64()
            .ok_or_else(|| "Invalid handle returned from guest-file-open".to_string())
    }

    pub fn guest_file_write(&mut self, handle: i64, data: &[u8]) -> Result<usize, String> {
        let encoded_data = BASE64.encode(data);
        let res = self.execute(
            "guest-file-write",
            Some(json!({
                "handle": handle,
                "buf-b64": encoded_data
            })),
        )?;

        res.get("count")
            .and_then(|c| c.as_u64())
            .map(|c| c as usize)
            .ok_or_else(|| "Failed to parse write count".to_string())
    }

    pub fn guest_file_read_chunk(&mut self, handle: i64, count: usize) -> Result<(Vec<u8>, bool), String> {
        let res = self.execute(
            "guest-file-read",
            Some(json!({
                "handle": handle,
                "count": count
            })),
        )?;
        let eof = res.get("eof").and_then(|v| v.as_bool()).unwrap_or(true);
        let b64 = res.get("buf-b64").and_then(|v| v.as_str()).unwrap_or("");
        let decoded = BASE64
            .decode(b64)
            .map_err(|e| format!("Failed to decode guest file base64: {}", e))?;
        Ok((decoded, eof))
    }

    pub fn guest_file_close(&mut self, handle: i64) -> Result<(), String> {
        self.execute("guest-file-close", Some(json!({ "handle": handle })))?;
        Ok(())
    }

    pub fn read_file_bytes(&mut self, guest_path: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
        let handle = self.guest_file_open(guest_path, "r")?;
        let mut out = Vec::new();
        let chunk_size = 64 * 1024;

        loop {
            let remaining = max_bytes.saturating_sub(out.len());
            if remaining == 0 {
                break;
            }
            match self.guest_file_read_chunk(handle, remaining.min(chunk_size)) {
                Ok((chunk, eof)) => {
                    out.extend_from_slice(&chunk);
                    if eof || chunk.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = self.guest_file_close(handle);
                    return Err(e);
                }
            }
        }

        let _ = self.guest_file_close(handle);
        Ok(out)
    }

    pub fn write_file_bytes(&mut self, guest_path: &str, data: &[u8]) -> Result<(), String> {
        let handle = self.guest_file_open(guest_path, "w+")?;
        for chunk in data.chunks(48 * 1024) {
            if let Err(e) = self.guest_file_write(handle, chunk) {
                let _ = self.guest_file_close(handle);
                return Err(e);
            }
        }
        self.guest_file_close(handle)
    }

    // --- Command Execution & Introspection ---

    pub fn exec_sync(
        &mut self,
        path: &str,
        args: &[String],
        timeout_secs: u64,
    ) -> Result<(i64, String, String), String> {
        let res = self.execute(
            "guest-exec",
            Some(json!({
                "path": path,
                "arg": args,
                "capture-output": true
            })),
        )?;
        let pid = res
            .get("pid")
            .and_then(|p| p.as_i64())
            .ok_or_else(|| "Failed to get pid from guest-exec".to_string())?;

        let started = Instant::now();
        loop {
            if started.elapsed().as_secs() > timeout_secs {
                return Err(format!(
                    "Guest command timed out after {} seconds",
                    timeout_secs
                ));
            }
            let status = self.execute("guest-exec-status", Some(json!({ "pid": pid })))?;
            if status
                .get("exited")
                .and_then(|e| e.as_bool())
                .unwrap_or(false)
            {
                let exitcode = status
                    .get("exitcode")
                    .and_then(|c| c.as_i64())
                    .unwrap_or(0);
                let out_str = status
                    .get("out-data")
                    .and_then(|d| d.as_str())
                    .and_then(|s| BASE64.decode(s).ok())
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
                    .unwrap_or_default();
                let err_str = status
                    .get("err-data")
                    .and_then(|d| d.as_str())
                    .and_then(|s| BASE64.decode(s).ok())
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
                    .unwrap_or_default();
                return Ok((exitcode, out_str, err_str));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    pub fn guest_os_info(&mut self) -> Result<Value, String> {
        self.execute("guest-get-osinfo", None)
    }
}
