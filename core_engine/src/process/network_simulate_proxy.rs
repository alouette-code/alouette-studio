use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, Duration};
use rand::Rng;

#[derive(Clone, Debug)]
pub struct SimulationParams {
    pub firewall_enabled: bool,
    pub firewall_rules: Vec<String>, // List of lowercase blocked patterns, e.g. "github.com", "*.google.com"
    pub weak_network_enabled: bool,
    pub latency_ms: u32,
    pub jitter_ms: u32,
    pub loss_rate: f32, // 0.0 - 100.0
    pub bandwidth_kbps: u32, // 0 = unlimited
    pub unstable_server_enabled: bool,
    pub unstable_server_drop_rate: f32,
    pub unstable_server_error_rate: f32,
    pub unstable_server_error_codes: Vec<u16>,
}

/// Spawns the simulation proxy on localhost. Returns the local port it is listening on
/// and a sender to stop the server.
pub async fn start_proxy(params: SimulationParams) -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind simulation proxy listener: {}", e))?;
    
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let params = Arc::new(params);

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    let params_clone = Arc::clone(&params);
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(socket, addr, params_clone).await {
                            // Silence normal connection errors to prevent console spam
                            log_debug(format!("Proxy connection handler error for {}: {}", addr, e));
                        }
                    });
                }
                Err(e) => {
                    eprintln!("[simulation_proxy] Error accepting socket: {}", e);
                    break;
                }
            }
        }
    });

    Ok(port)
}

fn log_debug(msg: String) {
    #[cfg(debug_assertions)]
    eprintln!("[simulation_proxy] {}", msg);
    let _ = msg;
}

async fn handle_client(
    mut client_stream: TcpStream,
    _addr: SocketAddr,
    params: Arc<SimulationParams>,
) -> Result<(), String> {
    // Read the initial request headers to parse the destination host
    let mut buffer = [0u8; 8192];
    let n = client_stream.read(&mut buffer).await
        .map_err(|e| format!("Failed to read request: {}", e))?;

    if n == 0 {
        return Ok(());
    }

    let request_str = String::from_utf8_lossy(&buffer[..n]);
    let first_line = request_str.lines().next().unwrap_or("");
    log_debug(format!("Incoming request line: {}", first_line));

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Malformed HTTP request line".to_string());
    }

    let method = parts[0].to_uppercase();
    let target = parts[1];

    let mut is_connect = false;
    let (mut host, port) = if method == "CONNECT" {
        is_connect = true;
        // CONNECT target is usually host:port
        parse_host_port(target, 443)
    } else {
        // Plain HTTP proxy request (e.g. GET http://example.com/path)
        if target.starts_with("http://") {
            let without_proto = &target["http://".len()..];
            let path_start = without_proto.find('/').unwrap_or(without_proto.len());
            parse_host_port(&without_proto[..path_start], 80)
        } else if target.starts_with("https://") {
            let without_proto = &target["https://".len()..];
            let path_start = without_proto.find('/').unwrap_or(without_proto.len());
            parse_host_port(&without_proto[..path_start], 443)
        } else {
            // Check Host header
            let mut host_header = None;
            for line in request_str.lines() {
                let trimmed = line.trim();
                if trimmed.to_lowercase().starts_with("host:") {
                    let val = trimmed["host:".len()..].trim();
                    host_header = Some(val.to_string());
                    break;
                }
            }
            if let Some(ref h) = host_header {
                parse_host_port(h, 80)
            } else {
                parse_host_port(target, 80)
            }
        }
    };

    // Sanitize host
    host = host.trim().to_lowercase();

    // 1. Check Firewall Rules
    if params.firewall_enabled {
        for rule in &params.firewall_rules {
            if matches_rule(&host, rule) {
                log_debug(format!("Blocked by firewall: {} (rule: {})", host, rule));
                if is_connect {
                    // Abruptly drop for HTTPS CONNECT
                    return Ok(());
                } else {
                    // Return 403 Forbidden for HTTP
                    let response = b"HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n\r\nBlocked by simulated firewall.\r\n";
                    let _ = client_stream.write_all(response).await;
                    return Ok(());
                }
            }
        }
    }

    // 2. Unstable Server - Error Injection (Only for plain HTTP proxy requests)
    if params.unstable_server_enabled && !is_connect && !params.unstable_server_error_codes.is_empty() {
        let inject = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..100.0) < params.unstable_server_error_rate
        };
        if inject {
            let code = {
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0..params.unstable_server_error_codes.len());
                params.unstable_server_error_codes[idx]
            };
            let (status_text, body) = get_http_status_details(code);
            let response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{}",
                code, status_text, body
            );
            log_debug(format!("Injecting simulated server error {} for host {}", code, host));
            let _ = client_stream.write_all(response.as_bytes()).await;
            return Ok(());
        }
    }

    // 3. Unstable Server - Random Connection Drop
    if params.unstable_server_enabled {
        let drop_conn = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..100.0) < params.unstable_server_drop_rate
        };
        if drop_conn {
            log_debug(format!("Simulated server random connection drop for host {}", host));
            return Ok(());
        }
    }

    // 4. Weak Network - Packet Loss Drop
    if params.weak_network_enabled {
        let loss = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..100.0) < params.loss_rate
        };
        if loss {
            log_debug(format!("Simulated network packet loss drop for host {}", host));
            return Ok(());
        }
    }

    // 5. Weak Network - Latency & Jitter
    if params.weak_network_enabled && (params.latency_ms > 0 || params.jitter_ms > 0) {
        let jitter = if params.jitter_ms > 0 {
            let mut rng = rand::thread_rng();
            rng.gen_range(-(params.jitter_ms as i32)..=(params.jitter_ms as i32))
        } else {
            0
        };
        let total_latency = (params.latency_ms as i32 + jitter).max(0) as u64;
        if total_latency > 0 {
            log_debug(format!("Simulated network latency sleep of {}ms for {}", total_latency, host));
            sleep(Duration::from_millis(total_latency)).await;
        }
    }

    // Connect to destination server
    let dest_addr = format!("{}:{}", host, port);
    log_debug(format!("Connecting to destination: {}", dest_addr));
    let mut server_stream = match TcpStream::connect(&dest_addr).await {
        Ok(s) => s,
        Err(e) => {
            log_debug(format!("Failed to connect to destination {}: {}", dest_addr, e));
            if !is_connect {
                let response = format!(
                    "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nFailed to connect to proxy target: {}\r\n",
                    e
                );
                let _ = client_stream.write_all(response.as_bytes()).await;
            }
            return Ok(());
        }
    };

    if is_connect {
        // Send HTTP 200 Established back to client to establish tunnel
        let established = b"HTTP/1.1 200 Connection Established\r\n\r\n";
        client_stream.write_all(established).await
            .map_err(|e| format!("Failed to write CONNECT response: {}", e))?;
    } else {
        // Forward the initial HTTP request headers we already read to the server
        server_stream.write_all(&buffer[..n]).await
            .map_err(|e| format!("Failed to forward initial request to server: {}", e))?;
    }

    // Bidirectional forwarding
    let (mut client_reader, mut client_writer) = client_stream.into_split();
    let (mut server_reader, mut server_writer) = server_stream.into_split();

    let bw = params.bandwidth_kbps;
    let limit_rate = params.weak_network_enabled && bw > 0;

    let client_to_server = async move {
        let mut buf = [0u8; 4096];
        loop {
            let n = match client_reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            if server_writer.write_all(&buf[..n]).await.is_err() {
                break;
            }

            if limit_rate {
                // Throttle writing: 4KB block size. To maintain `bw` kbps:
                // Time needed for `n` bytes = (n * 8) / (bw * 1000) seconds.
                let bits = (n as f64) * 8.0;
                let speed_bps = (bw as f64) * 1024.0;
                let sleep_secs = bits / speed_bps;
                sleep(Duration::from_secs_f64(sleep_secs)).await;
            }
        }
    };

    let server_to_client = async move {
        let mut buf = [0u8; 4096];
        loop {
            let n = match server_reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            if client_writer.write_all(&buf[..n]).await.is_err() {
                break;
            }

            if limit_rate {
                let bits = (n as f64) * 8.0;
                let speed_bps = (bw as f64) * 1024.0;
                let sleep_secs = bits / speed_bps;
                sleep(Duration::from_secs_f64(sleep_secs)).await;
            }
        }
    };

    // Wait until either direction closes
    tokio::select! {
        _ = client_to_server => {}
        _ = server_to_client => {}
    }

    Ok(())
}

fn parse_host_port(target: &str, default_port: u16) -> (String, u16) {
    if let Some(colon_idx) = target.rfind(':') {
        let host = &target[..colon_idx];
        let port_str = &target[colon_idx + 1..];
        if let Ok(port) = port_str.parse::<u16>() {
            return (host.to_string(), port);
        }
    }
    (target.to_string(), default_port)
}

fn matches_rule(host: &str, rule: &str) -> bool {
    let host = host.trim().to_lowercase();
    let rule = rule.trim().to_lowercase();

    if rule == "*" {
        return true;
    }

    if rule.starts_with("*.") {
        let suffix = &rule[2..];
        return host.ends_with(suffix);
    }

    host == rule
}

fn get_http_status_details(code: u16) -> (&'static str, &'static str) {
    match code {
        400 => ("Bad Request", "Simulated error: Bad Request"),
        401 => ("Unauthorized", "Simulated error: Unauthorized"),
        403 => ("Forbidden", "Simulated error: Forbidden"),
        404 => ("Not Found", "Simulated error: Not Found"),
        500 => ("Internal Server Error", "Simulated server failure: Internal Server Error"),
        502 => ("Bad Gateway", "Simulated server failure: Bad Gateway"),
        503 => ("Service Unavailable", "Simulated server failure: Service Unavailable"),
        504 => ("Gateway Timeout", "Simulated server failure: Gateway Timeout"),
        _ => ("Error", "Simulated Server Error"),
    }
}
