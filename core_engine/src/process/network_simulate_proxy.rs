use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::time::{sleep, Duration};
use rand::Rng;

#[derive(Clone, Debug)]
pub struct HeaderInjection {
    pub key: String,
    pub value: String,
    pub visibility: String, // "exposed" | "hidden"
    pub scope: String,      // "inbound" | "outbound" | "both"
}

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
    pub env_injection_enabled: bool,
    pub custom_headers: Vec<HeaderInjection>,
}

/// Handle for controlling lifecycle & graceful shutdown of simulation proxies.
pub struct ProxyHandle {
    pub port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl ProxyHandle {
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for ProxyHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Spawns the SOCKS5/HTTP Forward Proxy on localhost. Returns ProxyHandle containing the port and shutdown trigger.
pub async fn start_proxy(params: SimulationParams) -> Result<ProxyHandle, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind simulation forward proxy listener: {}", e))?;
    
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let params = Arc::new(params);
    let (tx, mut rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut rx => {
                    log_debug(format!("Forward proxy listener on port {} shut down", port));
                    break;
                }
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((socket, addr)) => {
                            let params_clone = Arc::clone(&params);
                            tokio::spawn(async move {
                                if let Err(e) = handle_forward_client(socket, addr, params_clone).await {
                                    log_debug(format!("Forward proxy connection error for {}: {}", addr, e));
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("[simulation_proxy] Error accepting socket: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    });

    Ok(ProxyHandle {
        port,
        shutdown_tx: Some(tx),
    })
}

/// Spawns the Inbound Reverse Proxy Gateway on public_port, forwarding allowed traffic to internal_port.
pub async fn start_reverse_proxy_gateway(
    public_port: u16,
    internal_port: u16,
    params: SimulationParams,
) -> Result<ProxyHandle, String> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", public_port))
        .await
        .map_err(|e| format!("Failed to bind reverse proxy gateway on port {}: {}", public_port, e))?;

    let params = Arc::new(params);
    let target_addr = format!("127.0.0.1:{}", internal_port);
    let (tx, mut rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut rx => {
                    log_debug(format!("Reverse proxy gateway listener on port {} shut down", public_port));
                    break;
                }
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((socket, addr)) => {
                            let params_clone = Arc::clone(&params);
                            let target_addr_clone = target_addr.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_reverse_client(socket, addr, target_addr_clone, params_clone).await {
                                    log_debug(format!("Reverse proxy gateway error for {}: {}", addr, e));
                                }
                            });
                        }
                        Err(e) => {
                            log_debug(format!("Reverse proxy gateway accept error: {}", e));
                            break;
                        }
                    }
                }
            }
        }
    });

    Ok(ProxyHandle {
        port: public_port,
        shutdown_tx: Some(tx),
    })
}

fn log_debug(msg: String) {
    #[cfg(debug_assertions)]
    eprintln!("[simulation_proxy] {}", msg);
    let _ = msg;
}

/// Helper function to perform data pipe with weak network emulation (latency, jitter, packet loss, bandwidth limit)
async fn pipe_with_emulation<R, W>(
    mut reader: R,
    mut writer: W,
    params: Arc<SimulationParams>,
    direction_desc: &'static str,
) where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let mut buf = [0u8; 8192];
    let bw = params.bandwidth_kbps;
    let limit_rate = params.weak_network_enabled && bw > 0;

    loop {
        // 1. Simulate mid-stream Packet Loss
        if params.weak_network_enabled && params.loss_rate > 0.0 {
            let mut rng = rand::thread_rng();
            if rng.gen_range(0.0..100.0) < (params.loss_rate / 15.0) { // Mid-stream chunk drop simulation
                log_debug(format!("Simulating packet loss drop mid-stream for direction: {}", direction_desc));
                break;
            }
        }

        let n = match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };

        // 2. Simulate Latency & Jitter on data transmission
        if params.weak_network_enabled && (params.latency_ms > 0 || params.jitter_ms > 0) {
            let jitter = if params.jitter_ms > 0 {
                let mut rng = rand::thread_rng();
                rng.gen_range(-(params.jitter_ms as i32)..=(params.jitter_ms as i32))
            } else {
                0
            };
            let chunk_latency = ((params.latency_ms as i32 + jitter).max(0) as u64) / 2;
            if chunk_latency > 0 {
                sleep(Duration::from_millis(chunk_latency)).await;
            }
        }

        if writer.write_all(&buf[..n]).await.is_err() {
            break;
        }

        // 3. Simulate Bandwidth Throttling
        if limit_rate {
            let bits = (n as f64) * 8.0;
            let speed_bps = (bw as f64) * 1024.0;
            let sleep_secs = bits / speed_bps;
            sleep(Duration::from_secs_f64(sleep_secs)).await;
        }
    }
}

async fn handle_forward_client(
    mut client_stream: TcpStream,
    _addr: SocketAddr,
    params: Arc<SimulationParams>,
) -> Result<(), String> {
    let mut buffer = [0u8; 8192];
    let n = client_stream.read(&mut buffer).await
        .map_err(|e| format!("Failed to read request: {}", e))?;

    if n == 0 {
        return Ok(());
    }

    // Check if client is using SOCKS5 protocol (VER = 0x05)
    if buffer[0] == 0x05 {
        return handle_socks5_forward(&mut client_stream, &buffer[..n], params).await;
    }

    // HTTP / HTTPS CONNECT Protocol
    let request_str = String::from_utf8_lossy(&buffer[..n]);
    let first_line = request_str.lines().next().unwrap_or("");
    log_debug(format!("Incoming Forward Request: {}", first_line));

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Malformed HTTP request line".to_string());
    }

    let method = parts[0].to_uppercase();
    let target = parts[1];

    let mut is_connect = false;
    let (mut host, port) = if method == "CONNECT" {
        is_connect = true;
        parse_host_port(target, 443)
    } else {
        if target.starts_with("http://") {
            let without_proto = &target["http://".len()..];
            let path_start = without_proto.find('/').unwrap_or(without_proto.len());
            parse_host_port(&without_proto[..path_start], 80)
        } else if target.starts_with("https://") {
            let without_proto = &target["https://".len()..];
            let path_start = without_proto.find('/').unwrap_or(without_proto.len());
            parse_host_port(&without_proto[..path_start], 443)
        } else {
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

    host = host.trim().to_lowercase();

    // 1. Check Firewall Rules
    if params.firewall_enabled {
        for rule in &params.firewall_rules {
            if matches_rule(&host, rule) {
                log_debug(format!("[ENV_SIM] Blocked by firewall: {} (rule: {})", host, rule));
                if is_connect {
                    return Ok(());
                } else {
                    let response = b"HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n\r\nBlocked by simulated firewall.\r\n";
                    let _ = client_stream.write_all(response).await;
                    return Ok(());
                }
            }
        }
    }

    // 2. Unstable Server - Connection Drop & Error Injection
    if params.unstable_server_enabled {
        let drop_conn = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..100.0) < params.unstable_server_drop_rate
        };
        if drop_conn {
            log_debug(format!("[ENV_SIM] Simulated server random connection drop for host {}", host));
            return Ok(());
        }

        if !is_connect && !params.unstable_server_error_codes.is_empty() {
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
                log_debug(format!("[ENV_SIM] Injecting simulated server error {} for host {}", code, host));
                let _ = client_stream.write_all(response.as_bytes()).await;
                return Ok(());
            }
        }
    }

    // Connect to destination server
    let dest_addr = format!("{}:{}", host, port);
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
        let established = b"HTTP/1.1 200 Connection Established\r\n\r\n";
        client_stream.write_all(established).await
            .map_err(|e| format!("Failed to write CONNECT response: {}", e))?;
    } else {
        server_stream.write_all(&buffer[..n]).await
            .map_err(|e| format!("Failed to forward initial request: {}", e))?;
    }

    let (client_reader, client_writer) = client_stream.into_split();
    let (server_reader, server_writer) = server_stream.into_split();

    let client_to_server = pipe_with_emulation(client_reader, server_writer, Arc::clone(&params), "client_to_server");
    let server_to_client = pipe_with_emulation(server_reader, client_writer, Arc::clone(&params), "server_to_client");

    tokio::select! {
        _ = client_to_server => {}
        _ = server_to_client => {}
    }

    Ok(())
}

async fn handle_socks5_forward(
    client_stream: &mut TcpStream,
    initial_buf: &[u8],
    params: Arc<SimulationParams>,
) -> Result<(), String> {
    // 1. SOCKS5 Handshake / Auth Negotiation
    // Response: [VER 0x05, METHOD 0x00 (NO AUTHENTICATION)]
    client_stream.write_all(&[0x05, 0x00]).await
        .map_err(|e| format!("SOCKS5 handshake reply error: {}", e))?;

    // 2. Read Request packet
    let mut req_buf = [0u8; 512];
    let req_n = client_stream.read(&mut req_buf).await
        .map_err(|e| format!("SOCKS5 request read error: {}", e))?;

    if req_n < 7 || req_buf[0] != 0x05 || req_buf[1] != 0x01 {
        // Only CMD = 0x01 (CONNECT) is supported
        let _ = client_stream.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await; // Command not supported
        return Ok(());
    }

    let atyp = req_buf[3];
    let (host, port) = match atyp {
        0x01 => { // IPv4
            if req_n < 10 { return Ok(()); }
            let ip = format!("{}.{}.{}.{}", req_buf[4], req_buf[5], req_buf[6], req_buf[7]);
            let p = u16::from_be_bytes([req_buf[8], req_buf[9]]);
            (ip, p)
        }
        0x03 => { // Domain Name
            let len = req_buf[4] as usize;
            if req_n < 5 + len + 2 { return Ok(()); }
            let domain = String::from_utf8_lossy(&req_buf[5..5 + len]).to_string();
            let p = u16::from_be_bytes([req_buf[5 + len], req_buf[5 + len + 1]]);
            (domain, p)
        }
        0x04 => { // IPv6
            if req_n < 22 { return Ok(()); }
            let p = u16::from_be_bytes([req_buf[20], req_buf[21]]);
            ("localhost".to_string(), p) // Simplified IPv6 fallback
        }
        _ => return Ok(()),
    };

    let host = host.trim().to_lowercase();

    // Firewall check
    if params.firewall_enabled {
        for rule in &params.firewall_rules {
            if matches_rule(&host, rule) {
                log_debug(format!("[ENV_SIM] SOCKS5 Blocked by firewall: {} (rule: {})", host, rule));
                // Reply Connection not allowed by ruleset (0x02)
                let _ = client_stream.write_all(&[0x05, 0x02, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await;
                return Ok(());
            }
        }
    }

    // Unstable Server Drop
    if params.unstable_server_enabled {
        let drop_conn = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..100.0) < params.unstable_server_drop_rate
        };
        if drop_conn {
            log_debug(format!("[ENV_SIM] SOCKS5 Random Connection drop for host {}", host));
            return Ok(());
        }
    }

    // Connect to Target
    let dest_addr = format!("{}:{}", host, port);
    let server_stream = match TcpStream::connect(&dest_addr).await {
        Ok(s) => s,
        Err(e) => {
            log_debug(format!("SOCKS5 Target connect failed {}: {}", dest_addr, e));
            // Reply Host Unreachable (0x04)
            let _ = client_stream.write_all(&[0x05, 0x04, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await;
            return Ok(());
        }
    };

    // SOCKS5 Success Response
    client_stream.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await
        .map_err(|e| format!("SOCKS5 reply error: {}", e))?;

    let (client_reader, client_writer) = client_stream.split();
    let (server_reader, server_writer) = server_stream.into_split();

    let client_to_server = pipe_with_emulation(client_reader, server_writer, Arc::clone(&params), "socks5_c2s");
    let server_to_client = pipe_with_emulation(server_reader, client_writer, Arc::clone(&params), "socks5_s2c");

    tokio::select! {
        _ = client_to_server => {}
        _ = server_to_client => {}
    }

    Ok(())
}

async fn handle_reverse_client(
    mut client_stream: TcpStream,
    _addr: SocketAddr,
    target_addr: String,
    params: Arc<SimulationParams>,
) -> Result<(), String> {
    // 1. Simulate Connection Drop at gateway level
    if params.unstable_server_enabled {
        let drop_conn = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..100.0) < params.unstable_server_drop_rate
        };
        if drop_conn {
            log_debug("[ENV_SIM] Reverse Proxy Gateway: Simulating connection drop before forwarding".to_string());
            return Ok(());
        }
    }

    // Read initial bytes to see if we should inject errors for plain HTTP
    let mut buffer = [0u8; 8192];
    let n = client_stream.read(&mut buffer).await.unwrap_or(0);

    if n > 0 {
        let request_str = String::from_utf8_lossy(&buffer[..n]);
        let first_line = request_str.lines().next().unwrap_or("");
        
        // Check if it looks like a standard HTTP request line
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() >= 2 && (parts[0] == "GET" || parts[0] == "POST" || parts[0] == "PUT" || parts[0] == "DELETE" || parts[0] == "PATCH" || parts[0] == "OPTIONS") {
            // 2. HTTP Error Injection
            if params.unstable_server_enabled && !params.unstable_server_error_codes.is_empty() {
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
                    log_debug(format!("[ENV_SIM] Reverse Proxy Gateway: Injecting error code {}", code));
                    let _ = client_stream.write_all(response.as_bytes()).await;
                    return Ok(());
                }
            }
        }
    }

    // Connect to actual server on internal port
    let mut server_stream = match TcpStream::connect(&target_addr).await {
        Ok(s) => s,
        Err(e) => {
            log_debug(format!("[ENV_SIM] Reverse Proxy Gateway: Failed to connect to internal server {}: {}", target_addr, e));
            let response = format!(
                "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nReverse Proxy Gateway: Failed to connect to backend target.\r\n"
            );
            let _ = client_stream.write_all(response.as_bytes()).await;
            return Ok(());
        }
    };

    // Forward the initial client read buffer to the internal server
    if n > 0 {
        server_stream.write_all(&buffer[..n]).await
            .map_err(|e| format!("Failed to forward request to internal server: {}", e))?;
    }

    let (client_reader, client_writer) = client_stream.into_split();
    let (server_reader, server_writer) = server_stream.into_split();

    let client_to_server = pipe_with_emulation(client_reader, server_writer, Arc::clone(&params), "client_to_server");
    let server_to_client = pipe_with_emulation(server_reader, client_writer, Arc::clone(&params), "server_to_client");

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

pub fn matches_rule(host: &str, rule: &str) -> bool {
    let host = host.trim().to_lowercase();
    let rule = rule.trim().to_lowercase();

    if rule == "*" || rule == "*.*" {
        return true;
    }

    if rule.starts_with("*.") {
        let suffix = &rule[2..];
        return host == suffix || host.ends_with(&format!(".{}", suffix));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_rule_exact() {
        assert!(matches_rule("github.com", "github.com"));
        assert!(!matches_rule("gitlab.com", "github.com"));
    }

    #[test]
    fn test_matches_rule_wildcard() {
        assert!(matches_rule("google.com", "*.google.com"));
        assert!(matches_rule("api.google.com", "*.google.com"));
        assert!(matches_rule("v2.api.google.com", "*.google.com"));
        assert!(!matches_rule("notgoogle.com", "*.google.com"));
        assert!(!matches_rule("fakegoogle.com", "*.google.com"));
    }
}
