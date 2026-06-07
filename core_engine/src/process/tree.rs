use sysinfo::{Pid, System};
use tokio::sync::broadcast;

use super::models::ProcessState;



pub(crate) struct StateUpdater {
    pub project_id: String,
    pub sender: broadcast::Sender<(String, ProcessState)>,
}

impl StateUpdater {
    pub fn update(&mut self, state: ProcessState) {
        let _ = self.sender.send((self.project_id.clone(), state));
    }
}

fn is_safe_to_kill(sys: &System, target_pid: Pid) -> Result<(), String> {
    let current_pid = Pid::from(std::process::id() as usize);

    // Get target process
    let target_proc = match sys.process(target_pid) {
        Some(p) => p,
        None => return Ok(()), // Already dead
    };

    let target_name = target_proc.name().to_string_lossy().to_lowercase();

    // 1. System Process Blocklist
    let blocklist = [
        "csrss.exe", "svchost.exe", "lsass.exe", "systemd", "kernel_task", "smss.exe",
        "explorer.exe", "wininit.exe", "winlogon.exe", "services.exe", "init", "launchd"
    ];
    if blocklist.iter().any(|&b| target_name.contains(b)) {
        return Err(format!("Security Block: Attempted to kill system process '{}'", target_name));
    }

    // 2. Session / UID Isolation
    #[cfg(not(target_os = "windows"))]
    {
        let current_uid = sys.process(current_pid).and_then(|p| p.user_id());
        let target_uid = target_proc.user_id();
        if let (Some(cur_uid), Some(t_uid)) = (current_uid, target_uid) {
            if cur_uid != t_uid {
                return Err("Security Block: Attempted to kill process owned by another user".to_string());
            }
        }
    }

    // 3. Parent check & Allowlist
    let mut is_descendant = false;
    let mut temp_pid = target_pid;
    while let Some(proc) = sys.process(temp_pid) {
        if let Some(ppid) = proc.parent() {
            if ppid == current_pid {
                is_descendant = true;
                break;
            }
            temp_pid = ppid;
        } else {
            break;
        }
    }

    if is_descendant {
        return Ok(());
    }

    let allowlist = [
        "node", "python", "cargo", "npm", "docker", "sh", "bash", "powershell", "cmd",
        "git", "python3", "pip", "yarn", "pnpm", "rustc"
    ];
    if allowlist.iter().any(|&a| target_name.contains(a)) {
        return Ok(());
    }

    Err(format!("Security Block: Process '{}' is not spawned by this application and not in the allowlist", target_name))
}

fn kill_tree_sysinfo(root_pid: u32) -> Result<(), String> {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let target_pid = Pid::from(root_pid as usize);
    is_safe_to_kill(&sys, target_pid)?;

    let mut pids_to_kill = Vec::new();
    let mut queue = vec![target_pid];
    let mut index = 0;

    while index < queue.len() {
        let parent = queue[index];
        index += 1;
        pids_to_kill.push(parent);

        for (&pid, process) in sys.processes() {
            if let Some(ppid) = process.parent() {
                if ppid == parent && !queue.contains(&pid) {
                    if is_safe_to_kill(&sys, pid).is_ok() {
                        queue.push(pid);
                    }
                }
            }
        }
    }

    // Kill processes bottom-up
    for pid in pids_to_kill.into_iter().rev() {
        if let Some(process) = sys.process(pid) {
            process.kill();
        }
    }

    Ok(())
}

pub async fn terminate_process_tree(pid: u32) {
    // First try the native Rust recursive system-crawling teardown
    if let Err(e) = kill_tree_sysinfo(pid) {
        eprintln!("Failed to kill tree natively: {}", e);
        return; // Security validation failed, do not proceed with OS taskkill
    }

    // On Windows, also run `taskkill` to guarantee that nested shell child wrapper environments are fully purged.
    #[cfg(target_os = "windows")]
    {
        let _ = tokio::process::Command::new("taskkill")
            .args(&["/F", "/T", "/PID", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
    }
}
