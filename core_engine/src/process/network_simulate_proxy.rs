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
    let mut is_first_chunk = true;
    let mut last_io = tokio::time::Instant::now();
    let mut consecutive_losses: u32 = 0;

    loop {
        let n = match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };

        let idle_ms = last_io.elapsed().as_millis();

        // 1. Simulate Latency & Jitter per RTT burst (first chunk or after idle > 50ms), not per 8KB chunk
        if params.weak_network_enabled
            && (params.latency_ms > 0 || params.jitter_ms > 0)
            && (is_first_chunk || idle_ms > 50)
        {
            let jitter = if params.jitter_ms > 0 {
                let mut rng = rand::thread_rng();
                rng.gen_range(-(params.jitter_ms as i32)..=(params.jitter_ms as i32))
            } else {
                0
            };
            let rtt_delay = ((params.latency_ms as i32 + jitter).max(0) as u64) / 2;
            if rtt_delay > 0 {
                sleep(Duration::from_millis(rtt_delay)).await;
            }
        }
        is_first_chunk = false;

        // 2. Simulate Packet Loss (TCP retransmission stall; drop only on extreme/repeated loss)
        if params.weak_network_enabled && params.loss_rate > 0.0 {
            if params.loss_rate >= 99.9 {
                log_debug(format!("Simulating 100% packet loss drop for: {}", direction_desc));
                break;
            }
            let (hit, stall_ms) = {
                let mut rng = rand::thread_rng();
                (rng.gen_range(0.0..100.0) < params.loss_rate, rng.gen_range(150..=450))
            };
            if hit {
                consecutive_losses += 1;
                if params.loss_rate >= 30.0 && consecutive_losses >= 3 {
                    log_debug(format!("Simulating connection drop after {} consecutive packet losses ({})", consecutive_losses, direction_desc));
                    break;
                }
                sleep(Duration::from_millis(stall_ms)).await;
            } else {
                consecutive_losses = 0;
            }
        }

        let write_start = tokio::time::Instant::now();
        if writer.write_all(&buf[..n]).await.is_err() {
            break;
        }

        // 3. Simulate Bandwidth Throttling (1 Kbps = 1000 bits/s)
        if limit_rate {
            let bits = (n as f64) * 8.0;
            let speed_bps = (bw as f64) * 1000.0;
            let target_secs = bits / speed_bps;
            let elapsed_secs = write_start.elapsed().as_secs_f64();
            if target_secs > elapsed_secs {
                sleep(Duration::from_secs_f64(target_secs - elapsed_secs)).await;
            }
        }

        last_io = tokio::time::Instant::now();
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
    if params.firewall_enabled && is_host_blocked(&host, &params.firewall_rules) {
        log_debug(format!("[ENV_SIM] Blocked by firewall: {}", host));
        let response = b"HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n\r\nBlocked by simulated firewall.\r\n";
        let _ = client_stream.write_all(response).await;
        return Ok(());
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
        let payload = inject_http_headers(&buffer[..n], &params, "outbound");
        server_stream.write_all(&payload).await
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
    _initial_buf: &[u8],
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
            let octets: [u8; 16] = req_buf[4..20].try_into().unwrap_or([0; 16]);
            let ipv6 = std::net::Ipv6Addr::from(octets).to_string();
            let p = u16::from_be_bytes([req_buf[20], req_buf[21]]);
            (ipv6, p)
        }
        _ => return Ok(()),
    };

    let host = host.trim().to_lowercase();

    // Firewall check
    if params.firewall_enabled && is_host_blocked(&host, &params.firewall_rules) {
        log_debug(format!("[ENV_SIM] SOCKS5 Blocked by firewall: {}", host));
        // Reply Connection not allowed by ruleset (0x02)
        let _ = client_stream.write_all(&[0x05, 0x02, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await;
        return Ok(());
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
        if parts.len() >= 2 && matches!(parts[0], "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "OPTIONS" | "HEAD") {
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

    // Forward the initial client read buffer to the internal server (with inbound HTTP header injection)
    if n > 0 {
        let payload = inject_http_headers(&buffer[..n], &params, "inbound");
        server_stream.write_all(&payload).await
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

    if rule.is_empty() {
        return false;
    }

    if rule == "*" || rule == "*.*" {
        return true;
    }

    if rule.starts_with("*.") {
        let suffix = &rule[2..];
        return host == suffix || host.ends_with(&format!(".{}", suffix));
    }

    if rule.ends_with(".*") {
        let prefix = &rule[..rule.len() - 1];
        return host.starts_with(prefix);
    }

    host == rule
}

/// Evaluates firewall rules with allow-exceptions (`!pattern`) taking precedence over block rules.
pub fn is_host_blocked(host: &str, rules: &[String]) -> bool {
    for rule in rules {
        let trimmed = rule.trim();
        if let Some(allow_pattern) = trimmed.strip_prefix('!') {
            if matches_rule(host, allow_pattern) {
                return false;
            }
        }
    }
    for rule in rules {
        let trimmed = rule.trim();
        if !trimmed.starts_with('!') && matches_rule(host, trimmed) {
            return true;
        }
    }
    false
}

/// Injects enabled `hidden` gateway marker headers matching `direction` ("inbound" | "outbound")
/// into an HTTP/1.x request head if `\r\n\r\n` is present.
pub fn inject_http_headers(raw_request: &[u8], params: &SimulationParams, direction: &str) -> Vec<u8> {
    if !params.env_injection_enabled || params.custom_headers.is_empty() {
        return raw_request.to_vec();
    }

    let applicable: Vec<(&str, &str)> = params
        .custom_headers
        .iter()
        .filter(|h| {
            h.visibility == "hidden"
                && !h.key.trim().is_empty()
                && (h.scope == "both" || h.scope == direction)
        })
        .map(|h| (h.key.trim(), h.value.trim()))
        .collect();

    if applicable.is_empty() {
        return raw_request.to_vec();
    }

    // Find end of HTTP headers
    let header_end = match raw_request.windows(4).position(|w| w == b"\r\n\r\n") {
        Some(pos) => pos,
        None => return raw_request.to_vec(),
    };

    let head_str = String::from_utf8_lossy(&raw_request[..header_end]);
    let first_line = head_str.lines().next().unwrap_or("");
    let first_word = first_line.split_whitespace().next().unwrap_or("");
    if !matches!(first_word, "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "OPTIONS" | "HEAD") {
        return raw_request.to_vec();
    }

    let existing_lower = head_str.to_lowercase();
    let mut injected_block = String::new();
    for (k, v) in applicable {
        let clean_k = k.replace(['\r', '\n', ':'], "");
        let clean_v = v.replace(['\r', '\n'], "");
        if clean_k.is_empty() {
            continue;
        }
        let needle = format!("\n{}:", clean_k.to_lowercase());
        if !existing_lower.contains(&needle) {
            injected_block.push_str(&format!("\r\n{}: {}", clean_k, clean_v));
        }
    }

    if injected_block.is_empty() {
        return raw_request.to_vec();
    }

    let mut out = Vec::with_capacity(raw_request.len() + injected_block.len());
    out.extend_from_slice(&raw_request[..header_end]);
    out.extend_from_slice(injected_block.as_bytes());
    out.extend_from_slice(&raw_request[header_end..]);
    out
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
        assert!(matches_rule("192.168.1.50", "192.168.*"));
        assert!(!matches_rule("10.0.0.1", "192.168.*"));
    }

    #[test]
    fn test_is_host_blocked_local_only() {
        let rules: Vec<String> = vec![
            "*", "!localhost", "!127.0.0.1", "!::1", "!*.local", "!192.168.*", "!10.*",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert!(!is_host_blocked("localhost", &rules));
        assert!(!is_host_blocked("127.0.0.1", &rules));
        assert!(!is_host_blocked("192.168.1.10", &rules));
        assert!(!is_host_blocked("mybox.local", &rules));
        assert!(is_host_blocked("google.com", &rules));
        assert!(is_host_blocked("example.vn", &rules));
        assert!(is_host_blocked("1.1.1.1", &rules));
    }

    #[test]
    fn test_inject_http_headers() {
        let params = SimulationParams {
            firewall_enabled: false,
            firewall_rules: vec![],
            weak_network_enabled: false,
            latency_ms: 0,
            jitter_ms: 0,
            loss_rate: 0.0,
            bandwidth_kbps: 0,
            unstable_server_enabled: false,
            unstable_server_drop_rate: 0.0,
            unstable_server_error_rate: 0.0,
            unstable_server_error_codes: vec![],
            env_injection_enabled: true,
            custom_headers: vec![
                HeaderInjection {
                    key: "X-Gateway-Trace-Id".to_string(),
                    value: "trace_9981".to_string(),
                    visibility: "hidden".to_string(),
                    scope: "both".to_string(),
                },
                HeaderInjection {
                    key: "X-Inbound-Only".to_string(),
                    value: "in_secret".to_string(),
                    visibility: "hidden".to_string(),
                    scope: "inbound".to_string(),
                },
                HeaderInjection {
                    key: "NODE_ENV".to_string(),
                    value: "simulation".to_string(),
                    visibility: "exposed".to_string(),
                    scope: "both".to_string(),
                },
            ],
        };

        let raw = b"GET /api/test HTTP/1.1\r\nHost: localhost:3000\r\n\r\n";
        let outbound = String::from_utf8(inject_http_headers(raw, &params, "outbound")).unwrap();
        assert!(outbound.contains("X-Gateway-Trace-Id: trace_9981"));
        assert!(!outbound.contains("X-Inbound-Only"));
        assert!(!outbound.contains("NODE_ENV"));

        let inbound = String::from_utf8(inject_http_headers(raw, &params, "inbound")).unwrap();
        assert!(inbound.contains("X-Gateway-Trace-Id: trace_9981"));
        assert!(inbound.contains("X-Inbound-Only: in_secret"));

        // Disabled env_injection_enabled -> untouched
        let mut disabled = params.clone();
        disabled.env_injection_enabled = false;
        assert_eq!(inject_http_headers(raw, &disabled, "inbound"), raw);

        // Duplicate header already present -> not duplicated; CRLF stripped
        let raw_with_dup = b"POST /submit HTTP/1.1\r\nHost: localhost\r\nX-Gateway-Trace-Id: existing\r\n\r\n";
        let deduped = String::from_utf8(inject_http_headers(raw_with_dup, &params, "outbound")).unwrap();
        assert_eq!(deduped.matches("X-Gateway-Trace-Id").count(), 1);
        assert!(deduped.contains("X-Gateway-Trace-Id: existing"));
    }

    #[tokio::test]
    async fn test_live_forward_proxy_firewall_http_and_connect() {
        let params = SimulationParams {
            firewall_enabled: true,
            firewall_rules: vec!["*.blocked.test".to_string(), "1.1.1.1".to_string()],
            weak_network_enabled: false,
            latency_ms: 0,
            jitter_ms: 0,
            loss_rate: 0.0,
            bandwidth_kbps: 0,
            unstable_server_enabled: false,
            unstable_server_drop_rate: 0.0,
            unstable_server_error_rate: 0.0,
            unstable_server_error_codes: vec![],
            env_injection_enabled: false,
            custom_headers: vec![],
        };

        let mut handle = start_proxy(params).await.expect("start_proxy");
        let addr = format!("127.0.0.1:{}", handle.port);

        // 1. Plain HTTP to blocked host -> 403 Forbidden
        let mut stream = TcpStream::connect(&addr).await.unwrap();
        stream
            .write_all(b"GET http://api.blocked.test/v1 HTTP/1.1\r\nHost: api.blocked.test\r\n\r\n")
            .await
            .unwrap();
        let mut buf = [0u8; 512];
        let n = stream.read(&mut buf).await.unwrap();
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert!(resp.starts_with("HTTP/1.1 403 Forbidden"), "Got: {}", resp);

        // 2. HTTPS CONNECT to blocked IP -> 403 Forbidden
        let mut stream2 = TcpStream::connect(&addr).await.unwrap();
        stream2
            .write_all(b"CONNECT 1.1.1.1:443 HTTP/1.1\r\nHost: 1.1.1.1:443\r\n\r\n")
            .await
            .unwrap();
        let mut buf2 = [0u8; 512];
        let n2 = stream2.read(&mut buf2).await.unwrap();
        let resp2 = String::from_utf8_lossy(&buf2[..n2]);
        assert!(resp2.starts_with("HTTP/1.1 403 Forbidden"), "Got: {}", resp2);

        handle.stop();
    }

    #[tokio::test]
    async fn test_live_reverse_proxy_header_injection_and_error_injection() {
        // Spawn a mock backend server on ephemeral port
        let backend_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let backend_port = backend_listener.local_addr().unwrap().port();

        let backend_task = tokio::spawn(async move {
            if let Ok((mut sock, _)) = backend_listener.accept().await {
                let mut buf = [0u8; 2048];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let received = String::from_utf8_lossy(&buf[..n]).to_string();
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    received.len(),
                    received
                );
                let _ = sock.write_all(reply.as_bytes()).await;
            }
        });

        // Pick a free public port for reverse proxy
        let tmp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let public_port = tmp_listener.local_addr().unwrap().port();
        drop(tmp_listener);

        let params = SimulationParams {
            firewall_enabled: false,
            firewall_rules: vec![],
            weak_network_enabled: false,
            latency_ms: 0,
            jitter_ms: 0,
            loss_rate: 0.0,
            bandwidth_kbps: 0,
            unstable_server_enabled: false,
            unstable_server_drop_rate: 0.0,
            unstable_server_error_rate: 0.0,
            unstable_server_error_codes: vec![503],
            env_injection_enabled: true,
            custom_headers: vec![HeaderInjection {
                key: "X-Sim-Auth-Marker".to_string(),
                value: "secret_token_123".to_string(),
                visibility: "hidden".to_string(),
                scope: "inbound".to_string(),
            }],
        };

        let mut gw = start_reverse_proxy_gateway(public_port, backend_port, params)
            .await
            .expect("start_reverse_proxy_gateway");

        let mut client = TcpStream::connect(format!("127.0.0.1:{}", public_port))
            .await
            .unwrap();
        client
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();

        let mut resp_buf = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            match client.read(&mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(n) => resp_buf.extend_from_slice(&chunk[..n]),
            }
        }
        let resp_str = String::from_utf8_lossy(&resp_buf);
        assert!(
            resp_str.contains("X-Sim-Auth-Marker: secret_token_123"),
            "Backend did not receive injected header! Response: {}",
            resp_str
        );

        gw.stop();
        let _ = backend_task.await;
    }

    #[tokio::test]
    async fn test_live_reverse_proxy_100_percent_error_injection() {
        let tmp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let public_port = tmp_listener.local_addr().unwrap().port();
        drop(tmp_listener);

        let params = SimulationParams {
            firewall_enabled: false,
            firewall_rules: vec![],
            weak_network_enabled: false,
            latency_ms: 0,
            jitter_ms: 0,
            loss_rate: 0.0,
            bandwidth_kbps: 0,
            unstable_server_enabled: true,
            unstable_server_drop_rate: 0.0,
            unstable_server_error_rate: 100.0,
            unstable_server_error_codes: vec![503],
            env_injection_enabled: false,
            custom_headers: vec![],
        };

        let mut gw = start_reverse_proxy_gateway(public_port, 59999, params)
            .await
            .expect("start_reverse_proxy_gateway");

        let mut client = TcpStream::connect(format!("127.0.0.1:{}", public_port))
            .await
            .unwrap();
        client
            .write_all(b"HEAD /status HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();

        let mut buf = [0u8; 512];
        let n = client.read(&mut buf).await.unwrap();
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert!(resp.starts_with("HTTP/1.1 503 Service Unavailable"), "Got: {}", resp);

        gw.stop();
    }
}
