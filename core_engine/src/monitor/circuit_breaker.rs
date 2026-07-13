use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Instant, Duration};

/// Cầu dao tự động (Circuit Breaker).
/// Nếu lỗi liên tục vượt ngưỡng (max_failures), cầu dao sẽ "ngắt" (Open state),
/// từ chối mọi yêu cầu trong `reset_timeout` để bảo vệ tài nguyên.
pub struct CircuitBreaker {
    failures: AtomicUsize,
    max_failures: usize,
    last_failure_time: std::sync::RwLock<Option<Instant>>,
    reset_timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(max_failures: usize, reset_timeout_secs: u64) -> Self {
        Self {
            failures: AtomicUsize::new(0),
            max_failures,
            last_failure_time: std::sync::RwLock::new(None),
            reset_timeout: Duration::from_secs(reset_timeout_secs),
        }
    }

    /// Kiểm tra xem cầu dao có đang ngắt hay không.
    pub fn is_open(&self) -> bool {
        let failures = self.failures.load(Ordering::Relaxed);
        if failures >= self.max_failures {
            let last = self.last_failure_time.read().unwrap();
            if let Some(time) = *last {
                if time.elapsed() < self.reset_timeout {
                    return true; // Still open
                }
            }
        }
        false
    }

    /// Đánh dấu một kết quả thành công, reset bộ đếm lỗi.
    pub fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        let mut last = self.last_failure_time.write().unwrap();
        *last = None;
    }

    /// Đánh dấu một lỗi, nếu vượt ngưỡng sẽ ngắt cầu dao.
    pub fn record_failure(&self) {
        let count = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        if count >= self.max_failures {
            let mut last = self.last_failure_time.write().unwrap();
            *last = Some(Instant::now());
        }
    }
}
