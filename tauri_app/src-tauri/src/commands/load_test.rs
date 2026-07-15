use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use reqwest::Client;
use std::time::{Instant, Duration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoadTestInput {
    pub url: String,
    pub method: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub vus: usize,
    pub duration_sec: u64,
}

#[derive(Clone, Serialize)]
pub struct LoadTestProgress {
    pub current_sec: u64,
    pub total_sec: u64,
    pub total_requests: usize,
    pub rps: f64,
    pub p95_latency: f64,
    pub error_count: usize,
}

#[derive(Clone, Serialize)]
pub struct LoadTestResult {
    pub total_requests: usize,
    pub success_count: usize,
    pub error_count: usize,
    pub total_time_ms: u64,
    pub rps: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
}

#[tauri::command]
pub async fn run_load_test(
    input: LoadTestInput,
    app: AppHandle,
) -> Result<LoadTestResult, String> {
    let mut client_builder = Client::builder()
        .timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(input.vus);

    let mut header_map = reqwest::header::HeaderMap::new();
    for (k, v) in &input.headers {
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(k.as_bytes()) {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(v) {
                header_map.insert(name, val);
            }
        }
    }
    client_builder = client_builder.default_headers(header_map);
    let client = client_builder.build().map_err(|e| e.to_string())?;

    let vus = if input.vus == 0 { 1 } else { input.vus };
    let duration = Duration::from_secs(input.duration_sec);
    
    let (tx, mut rx) = mpsc::channel::<(bool, u64)>(100000); // success, latency_ms

    let start_time = Instant::now();
    
    // Spawn VU workers
    for _ in 0..vus {
        let client = client.clone();
        let method_str = input.method.clone();
        let url = input.url.clone();
        let body_opt = input.body.clone();
        let tx = tx.clone();
        
        let method = match method_str.as_str() {
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "OPTIONS" => reqwest::Method::OPTIONS,
            "HEAD" => reqwest::Method::HEAD,
            _ => reqwest::Method::GET,
        };

        tokio::spawn(async move {
            while start_time.elapsed() < duration {
                let req_start = Instant::now();
                let mut builder = client.request(method.clone(), &url);
                if let Some(ref b) = body_opt {
                    builder = builder.body(b.clone());
                }
                
                let success = match builder.send().await {
                    Ok(resp) => resp.status().is_success(),
                    Err(_) => false,
                };
                
                let elapsed = req_start.elapsed().as_millis() as u64;
                if tx.send((success, elapsed)).await.is_err() {
                    break;
                }
            }
        });
    }

    drop(tx); // drop original tx so rx stream can end when all workers finish or time is up

    let mut latencies = Vec::with_capacity(10000);
    let mut success_count = 0;
    let mut error_count = 0;
    
    let mut last_progress_time = Instant::now();
    let mut requests_in_window = 0;
    let mut current_sec = 0;

    // Collect results
    while let Some((success, lat)) = rx.recv().await {
        if success {
            success_count += 1;
        } else {
            error_count += 1;
        }
        latencies.push(lat as f64);
        requests_in_window += 1;

        if last_progress_time.elapsed() >= Duration::from_secs(1) {
            current_sec += 1;
            let mut window_latencies = latencies[latencies.len().saturating_sub(requests_in_window)..].to_vec();
            window_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let p95 = if !window_latencies.is_empty() {
                window_latencies[(window_latencies.len() as f64 * 0.95) as usize]
            } else {
                0.0
            };

            let _ = app.emit("load-test-progress", LoadTestProgress {
                current_sec,
                total_sec: input.duration_sec,
                total_requests: success_count + error_count,
                rps: requests_in_window as f64,
                p95_latency: p95,
                error_count,
            });

            requests_in_window = 0;
            last_progress_time = Instant::now();
        }
    }

    let total_time_ms = start_time.elapsed().as_millis() as u64;
    let total_requests = success_count + error_count;
    
    if total_requests == 0 {
        return Ok(LoadTestResult {
            total_requests: 0,
            success_count: 0,
            error_count: 0,
            total_time_ms,
            rps: 0.0,
            min_latency_ms: 0.0,
            max_latency_ms: 0.0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
        });
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let sum: f64 = latencies.iter().sum();
    let avg = sum / latencies.len() as f64;
    let min = latencies[0];
    let max = latencies[latencies.len() - 1];
    let p95 = latencies[(latencies.len() as f64 * 0.95) as usize];
    let p99 = latencies[(latencies.len() as f64 * 0.99) as usize];
    let rps = total_requests as f64 / (total_time_ms as f64 / 1000.0);

    Ok(LoadTestResult {
        total_requests,
        success_count,
        error_count,
        total_time_ms,
        rps,
        min_latency_ms: min,
        max_latency_ms: max,
        avg_latency_ms: avg,
        p95_latency_ms: p95,
        p99_latency_ms: p99,
    })
}
