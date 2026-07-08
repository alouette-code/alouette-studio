use crate::memory_inspector::models::{TelemetryData, Diagnosis, ProfilingResult};

pub struct HeuristicEngine {
    history: Vec<TelemetryData>,
    current_sample_interval_ms: u64,
}

impl HeuristicEngine {
    pub fn new() -> Self {
        Self { 
            history: Vec::new(),
            current_sample_interval_ms: 1000, // Default 1s
        }
    }

    pub fn is_crash_imminent(&self) -> bool {
        if self.history.len() < 5 {
            return false;
        }

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

        let mut d2_positive_count = 0;
        for i in 1..d1.len() {
            if d1[i] > d1[i-1] {
                d2_positive_count += 1;
            }
        }

        // If rate of growth is increasing exponentially
        d2_positive_count > d1.len() / 2 && d1.last().unwrap() > &0.0
    }
    
    fn update_adaptive_sampling(&mut self) {
        // If memory is stable, sample less frequently. If volatile, sample more frequently.
        if self.history.len() < 10 {
            return;
        }
        
        let last_10 = &self.history[self.history.len() - 10..];
        let max_val = last_10.iter().map(|d| d.memory_usage_mb).fold(0.0_f64, f64::max);
        let min_val = last_10.iter().map(|d| d.memory_usage_mb).fold(f64::INFINITY, f64::min);
        
        let variance = max_val - min_val;
        
        if variance > 50.0 {
            // High volatility, sample every 100ms
            self.current_sample_interval_ms = 100;
        } else if variance > 10.0 {
            // Medium volatility, sample every 500ms
            self.current_sample_interval_ms = 500;
        } else {
            // Stable, sample every 2 seconds
            self.current_sample_interval_ms = 2000;
        }
    }
}

impl super::Profiler for HeuristicEngine {
    fn record_telemetry(&mut self, data: TelemetryData) {
        self.history.push(data);
        if self.history.len() > 2000 {
            self.history.remove(0);
        }
        self.update_adaptive_sampling();
    }

    fn analyze(&self) -> Diagnosis {
        if self.history.len() < 10 {
            return Diagnosis::Unknown;
        }

        let mut drops = 0;
        let mut increases = 0;

        for i in 1..self.history.len() {
            let diff = self.history[i].memory_usage_mb - self.history[i-1].memory_usage_mb;
            if diff < -1.0 {
                drops += 1;
            } else if diff > 1.0 {
                increases += 1;
            }
        }

        if drops > 3 && increases > 3 {
            // Saw-tooth pattern
            Diagnosis::CacheEviction
        } else if increases > self.history.len() / 2 && drops == 0 {
            // Monotonic increase
            Diagnosis::StubbornLeak
        } else {
            Diagnosis::Unknown
        }
    }

    fn get_baseline(&self) -> Option<ProfilingResult> {
        if self.history.len() < 5 {
            return None;
        }

        let min = self.history.iter().map(|d| d.memory_usage_mb).fold(f64::INFINITY, f64::min);
        let sum: f64 = self.history.iter().map(|d| d.memory_usage_mb).sum();
        let avg = sum / self.history.len() as f64;

        Some(ProfilingResult {
            min_ram_mb: min,
            avg_ram_mb: avg,
            behavior_fingerprint: "heuristic_sig_v2".to_string()
        })
    }
    
    fn recommended_sample_interval_ms(&self) -> u64 {
        self.current_sample_interval_ms
    }
}
