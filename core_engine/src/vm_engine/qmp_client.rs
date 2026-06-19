use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use serde_json::{json, Value};
use std::time::Duration;

pub struct QmpClient {
    stream: UnixStream,
}

impl QmpClient {
    /// Connects to a QMP Unix socket and performs the initial handshake.
    pub fn connect<P: AsRef<Path>>(socket_path: P) -> Result<Self, String> {
        let mut stream = UnixStream::connect(socket_path)
            .map_err(|e| format!("Failed to connect to QMP socket: {}", e))?;
        
        stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;

        let mut client = Self { stream };

        // Read the greeting message
        let _greeting = client.read_response()?;
        
        // Send qmp_capabilities to exit capabilities negotiation mode
        client.execute("qmp_capabilities", None)?;

        Ok(client)
    }

    /// Reads a single JSON object response from the socket.
    fn read_response(&mut self) -> Result<Value, String> {
        let mut buffer = [0; 4096];
        let mut raw_data = Vec::new();

        loop {
            let bytes_read = self.stream.read(&mut buffer)
                .map_err(|e| format!("Failed to read from QMP socket: {}", e))?;
            
            if bytes_read == 0 {
                return Err("QMP socket closed unexpectedly".to_string());
            }

            raw_data.extend_from_slice(&buffer[..bytes_read]);

            // Try to parse what we have so far
            if let Ok(value) = serde_json::from_slice::<Value>(&raw_data) {
                return Ok(value);
            }
            
            // Note: In a robust implementation, we would split by newlines as QMP sends JSON-lines.
            // For simple commands, this works because the response fits in the buffer.
            if let Some(pos) = raw_data.iter().position(|&b| b == b'\n') {
                let line = &raw_data[..pos];
                if let Ok(value) = serde_json::from_slice::<Value>(line) {
                    return Ok(value);
                }
            }
        }
    }

    /// Executes a QMP command and waits for the result or error.
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
            .map_err(|e| format!("Failed to send QMP command: {}", e))?;

        // Read responses until we get the result or an error (ignoring asynchronous events)
        loop {
            let response = self.read_response()?;
            if let Some(err) = response.get("error") {
                let desc = err.get("desc").and_then(|d| d.as_str()).unwrap_or("Unknown error");
                return Err(format!("QMP Error: {}", desc));
            }
            if let Some(res) = response.get("return") {
                return Ok(res.clone());
            }
            // Ignore events like {"event": "..."}
        }
    }

    // --- High Level API ---

    pub fn save_snapshot(&mut self, name: &str) -> Result<(), String> {
        // HMP command savevm via QMP human-monitor-command
        self.execute("human-monitor-command", Some(json!({"command-line": format!("savevm {}", name)})))?;
        Ok(())
    }

    pub fn load_snapshot(&mut self, name: &str) -> Result<(), String> {
        // HMP command loadvm
        self.execute("human-monitor-command", Some(json!({"command-line": format!("loadvm {}", name)})))?;
        Ok(())
    }

    pub fn delete_snapshot(&mut self, name: &str) -> Result<(), String> {
        // HMP command delvm
        self.execute("human-monitor-command", Some(json!({"command-line": format!("delvm {}", name)})))?;
        Ok(())
    }
}
