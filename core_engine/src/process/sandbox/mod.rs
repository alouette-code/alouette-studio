//! # Sandbox Module — 3 tầng bảo vệ
//!
//! ## Tầng 1 — interceptor (thuật toán nội suy)
//! Phân tích câu lệnh ở mức ngữ nghĩa, trích xuất paths đích.
//!
//! ## Tầng 1b — engine (cross-platform fallback)
//! Tokenize + path checking cơ bản.
//!
//! ## Tầng 2 — OS-specific (pre-spawn isolation)
//! - `windows.rs`: Windows Job Object + AppContainer kernel-level sandbox
//! - `linux.rs`: pre_exec với prctl(NO_NEW_PRIVS) + setrlimit(RLIMIT_AS) + unshare(CLONE_NEWNET)
//! - `macos.rs`: (placeholder)
//!
//! ## Luồng xử lý sandbox
//! 1. Tier 1 (interceptor) + Tier 1b (engine) chạy trước trên command string → block/reject
//! 2. Tier 2 (OS-level) chạy NGAY TRƯỚC khi process spawn (pre_exec hook hoặc wrapper script)
//!
//! ## Nguyên tắc thiết kế
//! - Tier 2 Linux KHÔNG cần root: dùng prctl/setrlimit/unshare mà user thường được phép gọi
//! - pre_exec chạy trong child process sau fork() nhưng trước execve() → không có race condition

pub mod engine;
pub mod interceptor;
pub mod linux;
pub mod macos;

pub mod windows;

pub use engine::Verdict;

/// Kiểm tra toàn bộ câu lệnh trước khi gửi đến shell.
///
/// Sử dụng interceptor (thuật toán nội suy) trước,
/// fallback về engine nếu interceptor không phát hiện.
pub fn check_command(
    input: &str,
    cwd: &std::path::Path,
    workspace_root: &std::path::Path,
) -> Verdict {
    // Tầng 1a: Interceptor — nội suy ngữ nghĩa
    let v = interceptor::intercept(input, cwd, workspace_root);
    if v != Verdict::Allow {
        return v;
    }

    // Tầng 1b: Engine — tokenize + path checking
    let v = engine::check(input, cwd, workspace_root);
    if v != Verdict::Allow {
        return v;
    }

    Verdict::Allow
}

/// Kiểm tra OS có support OS-level sandbox không.
pub fn is_os_sandbox_supported() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::is_supported()
    }
    #[cfg(target_os = "linux")]
    {
        linux::is_supported()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}
