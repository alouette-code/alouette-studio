use tokio::process::Command;
use crate::memory_inspector::models::{InspectionConfig, TelemetryData, TargetType};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DockerDriver;

impl super::ExecutionProvider for DockerDriver {
    async fn check_health(&self) -> Result<(), String> {
        let status = Command::new("docker")
            .arg("info")
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if !status.status.success() {
            return Err("Docker daemon is not running or accessible.".to_string());
        }
        Ok(())
    }

    async fn create_sandbox(&self, config: &InspectionConfig, name: &str) -> Result<(), String> {
        let mem_str = format!("{}m", config.initial_ram_mb);
        
        let mut args = vec!["run".to_string(), "-d".to_string(), "--name".to_string(), name.to_string(), "-m".to_string(), mem_str];
        
        for (key, value) in &config.env_vars {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }
        
        for port in &config.ports {
            args.push("-p".to_string());
            args.push(port.clone());
        }
        
        for vol in &config.volumes {
            args.push("-v".to_string());
            args.push(vol.clone());
        }
        
        if let Some(net) = &config.network {
            args.push("--network".to_string());
            args.push(net.clone());
        }
        
        // Capabilities for debugging
        args.push("--cap-add".to_string());
        args.push("SYS_PTRACE".to_string());
        
        let mut actual_image = config.image.clone();
        let mut actual_cmd = config.cmd.clone();

        if let Some(target) = &config.target_type {
            match target {
                TargetType::DockerImage => {}
                TargetType::CodeSnippet { language, code } => {
                    let tmp_dir = std::env::temp_dir().join("alouette_snippets");
                    let _ = std::fs::create_dir_all(&tmp_dir);
                    let ext = match language.as_str() {
                        "python" => "py",
                        "node" | "javascript" => "js",
                        "bash" => "sh",
                        "c" => "c",
                        "cpp" => "cpp",
                        "rust" => "rs",
                        "go" => "go",
                        "java" => "java",
                        "php" => "php",
                        "ruby" => "rb",
                        _ => "txt"
                    };
                    let file_name = if language.as_str() == "java" { "Main".to_string() } else { name.to_string() };
                    let file_path = tmp_dir.join(format!("{}.{}", file_name, ext));
                    let _ = std::fs::write(&file_path, code);
                    
                    let is_compiled = match language.as_str() {
                        "c" | "cpp" | "rust" | "go" | "java" => true,
                        _ => false,
                    };

                    let dest_name = if language.as_str() == "java" { "Main".to_string() } else { "snippet".to_string() };

                    if is_compiled {
                        // Synchronously compile in a throwaway container
                        let compile_cmd = match language.as_str() {
                            "c" => format!("gcc /app/{}.c -o /app/run", file_name),
                            "cpp" => format!("g++ /app/{}.cpp -o /app/run", file_name),
                            "rust" => format!("rustc /app/{}.rs -o /app/run", file_name),
                            "go" => format!("cd /app && go build -o run {}.go", file_name),
                            "java" => format!("javac -d /app /app/{}.java", file_name),
                            _ => "".to_string(),
                        };
                        
                        let compile_output = Command::new("docker")
                            .args(["run", "--rm", "-v", &format!("{}:/app", tmp_dir.display()), &config.image, "sh", "-c", &compile_cmd])
                            .output()
                            .await
                            .map_err(|e| e.to_string())?;
                            
                        if !compile_output.status.success() {
                            return Err(format!("Compilation failed: {}", String::from_utf8_lossy(&compile_output.stderr)));
                        }

                        args.push("-v".to_string());
                        args.push(format!("{}:/app", tmp_dir.display()));
                        
                        actual_cmd = Some(match language.as_str() {
                            "java" => format!("java -cp /app {}", dest_name),
                            _ => "/app/run".to_string(),
                        });
                    } else {
                        args.push("-v".to_string());
                        args.push(format!("{}:/app/{}.{}", file_path.display(), dest_name, ext));
                        
                        actual_cmd = Some(match language.as_str() {
                            "python" => format!("python /app/{}.{}", dest_name, ext),
                            "node" | "javascript" => format!("node /app/{}.{}", dest_name, ext),
                            "bash" => format!("bash /app/{}.{}", dest_name, ext),
                            "php" => format!("php /app/{}.{}", dest_name, ext),
                            "ruby" => format!("ruby /app/{}.{}", dest_name, ext),
                            _ => format!("cat /app/{}.{}", dest_name, ext)
                        });
                    }
                }
                TargetType::ExecutableFile { host_path } => {
                    args.push("-v".to_string());
                    args.push(format!("{}:/app/run_me", host_path));
                    
                    if config.image.trim().is_empty() {
                        actual_image = "ubuntu:22.04".to_string();
                    }
                    actual_cmd = Some("/app/run_me".to_string());
                }
            }
        }
        
        args.push(actual_image);
        
        if let Some(cmd_str) = &actual_cmd {
            if !cmd_str.trim().is_empty() {
                args.push("sh".to_string());
                args.push("-c".to_string());
                args.push(cmd_str.clone());
            }
        }

        let output = Command::new("docker")
            .args(&args)
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to create docker sandbox: {}", err_msg));
        }

        // Launch APM Sidecar
        let apm_name = format!("{}-apm", name);
        let apm_args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(), apm_name.clone(),
            format!("--pid=container:{}", name),
            "--cap-add".to_string(), "SYS_PTRACE".to_string(),
            "alpine".to_string(),
            "sh".to_string(), "-c".to_string(),
            "apk add --no-cache strace && strace -p 1 -f -e trace=memory,network,file -s 256 2>&1".to_string(),
        ];
        
        let apm_output = Command::new("docker")
            .args(&apm_args)
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if !apm_output.status.success() {
            println!("Warning: APM sidecar failed to start. Tracing might be limited.");
        }

        Ok(())
    }

    async fn update_memory_limit(&self, name: &str, memory_mb: f64) -> Result<(), String> {
        let mem_str = format!("{}m", memory_mb.round() as u64);
        let status = Command::new("docker")
            .args(["update", "-m", &mem_str, name])
            .status()
            .await
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err("Failed to update docker memory limit".to_string());
        }
        Ok(())
    }

    async fn inject_chaos(&self, name: &str) -> Result<(), String> {
        use rand::Rng;
        let scenario = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..3)
        };
        
        match scenario {
            0 => {
                // Memory Spike: Extremely low limit for a short duration
                let _ = Command::new("docker").args(["update", "-m", "80m", name]).status().await;
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                // It will be restored by the StressController in the next tick
            }
            1 => {
                // Network Blackout
                let _ = Command::new("docker")
                    .args(["exec", name, "tc", "qdisc", "add", "dev", "eth0", "root", "netem", "loss", "100%"])
                    .status()
                    .await;
                
                tokio::spawn({
                    let n = name.to_string();
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        let _ = Command::new("docker")
                            .args(["exec", &n, "tc", "qdisc", "del", "dev", "eth0", "root", "netem"])
                            .status()
                            .await;
                    }
                });
            }
            2 => {
                // Thread Panic / Freeze
                // Freeze process 1 for 1 second
                let _ = Command::new("docker").args(["exec", name, "kill", "-STOP", "1"]).status().await;
                tokio::spawn({
                    let n = name.to_string();
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        let _ = Command::new("docker").args(["exec", &n, "kill", "-CONT", "1"]).status().await;
                    }
                });
            }
            _ => {}
        }
        
        Ok(())
    }

    async fn get_stats(&self, name: &str) -> Result<TelemetryData, String> {
        let output = Command::new("docker")
            .args(["stats", "--no-stream", "--format", "{{.MemUsage}}", name])
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err("Failed to get docker stats".to_string());
        }

        let stat_str = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stat_str.trim().split(" / ").collect();
        
        let mut usage_mb = 0.0;
        let mut limit_mb = 0.0;
        
        if parts.len() == 2 {
            usage_mb = parse_docker_mem(parts[0]);
            limit_mb = parse_docker_mem(parts[1]);
        }

        let top_output = Command::new("docker")
            .args(["top", name])
            .output()
            .await
            .map_err(|e| e.to_string())?;
            
        let mut thread_count = 0;
        let mut activities = Vec::new();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        if top_output.status.success() {
            let top_str = String::from_utf8_lossy(&top_output.stdout);
            let lines: Vec<&str> = top_str.lines().collect();
            if lines.len() > 1 {
                thread_count = (lines.len() - 1) as u32;
                // Generate activities from top
                for line in lines.iter().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 8 {
                        let pid = parts[1].to_string();
                        let cmd = parts[7..].join(" ");
                        activities.push(crate::memory_inspector::models::ProcessActivity {
                            timestamp: now,
                            event_type: "Process".to_string(),
                            pid,
                            details: format!("Running: {}", cmd),
                        });
                    }
                }
            }
        }

        // Capture recent container logs
        let logs_output = Command::new("docker")
            .args(["logs", "--tail", "5", name])
            .output()
            .await;
            
        if let Ok(logs) = logs_output {
            if logs.status.success() {
                let log_str = String::from_utf8_lossy(&logs.stdout);
                for line in log_str.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        activities.push(crate::memory_inspector::models::ProcessActivity {
                            timestamp: now,
                            event_type: "Log".to_string(),
                            pid: "Container".to_string(),
                            details: trimmed.to_string(),
                        });
                    }
                }
            }
        }

        // Capture APM Syscalls
        let apm_name = format!("{}-apm", name);
        let apm_logs = Command::new("docker")
            .args(["logs", "--tail", "15", &apm_name])
            .output()
            .await;

        if let Ok(logs) = apm_logs {
            if logs.status.success() {
                let log_str = String::from_utf8_lossy(&logs.stdout);
                for line in log_str.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with("fetch") || trimmed.starts_with("OK:") { continue; } // ignore apk output

                    let mut event_type = "Syscall";
                    if trimmed.contains("mmap") || trimmed.contains("brk") || trimmed.contains("munmap") {
                        event_type = "Memory Syscall";
                    } else if trimmed.contains("socket") || trimmed.contains("connect") || trimmed.contains("accept") || trimmed.contains("bind") {
                        event_type = "Network Syscall";
                    } else if trimmed.contains("open") || trimmed.contains("read") || trimmed.contains("write") || trimmed.contains("unlink") {
                        event_type = "File Syscall";
                    }

                    activities.push(crate::memory_inspector::models::ProcessActivity {
                        timestamp: now,
                        event_type: event_type.to_string(),
                        pid: "Sidecar".to_string(),
                        details: trimmed.to_string(),
                    });
                }
            }
        }

        Ok(TelemetryData {
            timestamp: now,
            memory_usage_mb: usage_mb,
            memory_limit_mb: limit_mb,
            thread_count,
            gc_events_detected: 0,
            crash_imminent: false,
            status: "Running".to_string(),
            activities,
        })
    }

    async fn destroy_sandbox(&self, name: &str) -> Result<(), String> {
        let apm_name = format!("{}-apm", name);
        
        // --- DEBUG CODE: Save logs before destroying ---
        let log_file = format!("/home/nhatanh/projet/alouette_studio/tauri_app/logs/{}_crash.log", name);
        let _ = Command::new("sh")
            .args(["-c", &format!("docker logs {} > {} 2>&1", name, log_file)])
            .status()
            .await;
        // -----------------------------------------------

        let _ = Command::new("docker").args(["rm", "-f", name, &apm_name]).status().await;
        Ok(())
    }
}

fn parse_docker_mem(mem_str: &str) -> f64 {
    let s = mem_str.to_lowercase();
    if s.ends_with("mib") {
        s.replace("mib", "").trim().parse().unwrap_or(0.0)
    } else if s.ends_with("gib") {
        s.replace("gib", "").trim().parse::<f64>().unwrap_or(0.0) * 1024.0
    } else if s.ends_with("kib") {
        s.replace("kib", "").trim().parse::<f64>().unwrap_or(0.0) / 1024.0
    } else if s.ends_with("b") {
        s.replace("b", "").trim().parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
    } else {
        0.0
    }
}
