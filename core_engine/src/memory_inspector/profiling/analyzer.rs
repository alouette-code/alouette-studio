use crate::memory_inspector::models::{TelemetryData, Diagnosis, ProfilingResult};

pub struct SmartAnalyzer {
    history: Vec<TelemetryData>,
}

impl SmartAnalyzer {
    pub fn new() -> Self {
        Self { history: Vec::new() }
    }

    pub fn is_crash_imminent(&self) -> bool {
        if self.history.len() < 5 {
            return false;
        }

        // Calculate derivatives
        let mut d1 = Vec::new();
        for i in 1..self.history.len() {
            let dy = self.history[i].memory_usage_mb - self.history[i-1].memory_usage_mb;
            let dt = (self.history[i].timestamp - self.history[i-1].timestamp) as f64;
            if dt > 0.0 {
                d1.push(dy / dt);
            }
        }

        if d1.len() < 3 {
            return false;
        }

        // Check second derivative (is the rate of growth increasing? -> Exponential)
        let mut d2_positive_count = 0;
        for i in 1..d1.len() {
            if d1[i] > d1[i-1] {
                d2_positive_count += 1;
            }
        }

        // If most points show increasing rate of growth, crash is imminent
        d2_positive_count > d1.len() / 2 && d1.last().unwrap() > &0.0
    }
}

impl super::Profiler for SmartAnalyzer {
    fn record_telemetry(&mut self, data: TelemetryData) {
        self.history.push(data);
        if self.history.len() > 1000 {
            // Lossy compression for stability
            self.history.remove(0);
        }
    }

    fn analyze(&self) -> Diagnosis {
        if self.history.len() < 10 {
            return Diagnosis::Unknown;
        }

        let mut drops = 0;
        let mut increases = 0;

        for i in 1..self.history.len() {
            if self.history[i].memory_usage_mb < self.history[i-1].memory_usage_mb {
                drops += 1;
            } else if self.history[i].memory_usage_mb > self.history[i-1].memory_usage_mb {
                increases += 1;
            }
        }

        if drops > 3 {
            // Saw tooth pattern -> Cache Eviction
            Diagnosis::CacheEviction
        } else if increases > self.history.len() / 2 && drops == 0 {
            // Straight line up -> Stubborn Leak
            Diagnosis::StubbornLeak
        } else {
            Diagnosis::Unknown
        }
    }

    fn get_baseline(&self) -> Option<ProfilingResult> {
        if self.history.is_empty() {
            return None;
        }

        let min = self.history.iter().map(|d| d.memory_usage_mb).fold(f64::INFINITY, f64::min);
        let sum: f64 = self.history.iter().map(|d| d.memory_usage_mb).sum();
        let avg = sum / self.history.len() as f64;

        Some(ProfilingResult {
            min_ram_mb: min,
            avg_ram_mb: avg,
            behavior_fingerprint: "baseline_sig_1".to_string()
        })
    }
}
