use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use sysinfo::{Pid, System, ProcessStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmd: String,
    pub cwd: String,
    pub status: String,
    pub cpu_percentage: f32,
    pub ram_bytes: u64,
    pub thread_count: u64,
    pub ports: Vec<u16>,
    pub loaded_modules: Vec<String>,
    pub parent_pid: Option<u32>,
}

/// Helper to parse active listening TCP ports from /proc/net/tcp and /proc/net/tcp6
fn get_listening_sockets() -> HashMap<u64, u16> {
    let mut inodes = HashMap::new();
    
    #[cfg(target_os = "linux")]
    {
        for path in &["/proc/net/tcp", "/proc/net/tcp6"] {
            if let Ok(content) = fs::read_to_string(path) {
                for line in content.lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 10 {
                        let state = parts[3];
                        if state == "0A" { // TCP_LISTEN
                            let local_addr = parts[1];
                            if let Some(port_hex) = local_addr.split(':').nth(1) {
                                if let Ok(port) = u16::from_str_radix(port_hex, 16) {
                                    if let Ok(inode) = parts[9].parse::<u64>() {
                                        inodes.insert(inode, port);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    inodes
}

/// Helper to get listening ports of a PID by inspecting /proc/<pid>/fd/
fn get_pid_listening_ports(pid: u32, listening_sockets: &HashMap<u64, u16>) -> Vec<u16> {
    let mut ports = Vec::new();
    
    #[cfg(target_os = "linux")]
    {
        let fd_path = format!("/proc/{}/fd", pid);
        if let Ok(entries) = fs::read_dir(fd_path) {
            for entry in entries.flatten() {
                if let Ok(target) = fs::read_link(entry.path()) {
                    let target_str = target.to_string_lossy();
                    if target_str.starts_with("socket:[") && target_str.ends_with(']') {
                        let inode_str = &target_str[8..target_str.len() - 1];
                        if let Ok(inode) = inode_str.parse::<u64>() {
                            if let Some(&port) = listening_sockets.get(&inode) {
                                if !ports.contains(&port) {
                                    ports.push(port);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    ports.sort();
    ports
}

/// Helper to parse loaded libraries/files from /proc/<pid>/maps
fn get_loaded_modules(pid: u32) -> Vec<String> {
    let mut modules = HashSet::new();
    
    #[cfg(target_os = "linux")]
    {
        let maps_path = format!("/proc/{}/maps", pid);
        if let Ok(content) = fs::read_to_string(maps_path) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    let path_str = parts[parts.len() - 1];
                    if path_str.starts_with('/') {
                        let path = Path::new(path_str);
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy().to_lowercase();
                            if ext_str == "so" || ext_str == "js" || ext_str == "node" || ext_str == "json" || ext_str == "py" {
                                modules.insert(path_str.to_string());
                            }
                        } else if path_str.contains("node") || path_str.contains("python") || path_str.contains("alouette") {
                            modules.insert(path_str.to_string());
                        }
                    }
                }
            }
        }
    }
    
    let mut result: Vec<String> = modules.into_iter().collect();
    result.sort();
    result.truncate(15);
    result
}

/// Helper to retrieve the number of threads for a process
fn get_thread_count(pid: u32) -> u64 {
    #[cfg(target_os = "linux")]
    {
        let task_path = format!("/proc/{}/task", pid);
        if let Ok(entries) = fs::read_dir(task_path) {
            return entries.count() as u64;
        }
    }
    
    1
}

/// Collect detailed child process information recursively starting from root_pid
pub fn collect_child_processes(root_pid: u32, sys: &System, core_count: f32) -> Vec<ChildProcessInfo> {
    let mut tree_pids = HashSet::new();
    let mut queue = vec![Pid::from(root_pid as usize)];
    let mut index = 0;

    // Traverse the process tree using sysinfo
    while index < queue.len() {
        let current = queue[index];
        index += 1;
        tree_pids.insert(current);

        for (&pid, process) in sys.processes() {
            if let Some(ppid) = process.parent() {
                if ppid == current && !tree_pids.contains(&pid) {
                    queue.push(pid);
                }
            }
        }
    }

    let listening_sockets = get_listening_sockets();
    let mut processes_info = Vec::new();

    for pid in tree_pids {
        if let Some(process) = sys.process(pid) {
            let pid_val = format!("{}", pid).parse::<u32>().unwrap_or(0);
            if pid_val == 0 {
                continue;
            }

            let ram = process.memory();
            let cpu = process.cpu_usage();

            let cmd_str = process.cmd().iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect::<Vec<String>>()
                .join(" ");

            let cwd_str = process.cwd()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();

            let status_str = match process.status() {
                ProcessStatus::Run => "Running",
                ProcessStatus::Sleep => "Sleeping",
                ProcessStatus::Idle => "Idle",
                ProcessStatus::Stop => "Stopped",
                ProcessStatus::Zombie => "Zombie",
                _ => "Unknown",
            };

            let ports = get_pid_listening_ports(pid_val, &listening_sockets);
            let loaded_modules = get_loaded_modules(pid_val);
            let thread_count = get_thread_count(pid_val);

            let parent_pid_val = process.parent().map(|p| format!("{}", p).parse::<u32>().unwrap_or(0));

            processes_info.push(ChildProcessInfo {
                pid: pid_val,
                name: process.name().to_string_lossy().into_owned(),
                cmd: cmd_str,
                cwd: cwd_str,
                status: status_str.to_string(),
                cpu_percentage: cpu / core_count,
                ram_bytes: ram,
                thread_count,
                ports,
                loaded_modules,
                parent_pid: parent_pid_val,
            });
        }
    }

    // Sort by PID for stable order
    processes_info.sort_by_key(|p| p.pid);
    processes_info
}
