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

    /// Calculates Linear Regression over time series telemetry data.
    /// Returns Option<(drift_rate_kb_per_sec, r2_correlation_coefficient)>
    pub fn calculate_linear_regression(&self) -> Option<(f64, f64)> {
        if self.history.len() < 8 {
            return None;
        }

        let n = self.history.len() as f64;
        let first_t = self.history[0].timestamp as f64;

        let mut sum_t = 0.0;
        let mut sum_y = 0.0;
        let mut sum_tt = 0.0;
        let mut sum_yy = 0.0;
        let mut sum_ty = 0.0;

        for point in &self.history {
            let t = (point.timestamp as f64) - first_t;
            let y = point.memory_usage_mb;
            sum_t += t;
            sum_y += y;
            sum_tt += t * t;
            sum_yy += y * y;
            sum_ty += t * y;
        }

        let mean_t = sum_t / n;
        let mean_y = sum_y / n;

        let var_t = sum_tt - n * mean_t * mean_t;
        let var_y = sum_yy - n * mean_y * mean_y;
        let cov_ty = sum_ty - n * mean_t * mean_y;

        if var_t <= 0.0001 {
            return None;
        }

        let slope_mb_per_sec = cov_ty / var_t;
        let drift_rate_kb_per_sec = slope_mb_per_sec * 1024.0;

        let r2 = if var_y > 0.000001 {
            let r = cov_ty / (var_t.sqrt() * var_y.sqrt());
            (r * r).min(1.0).max(0.0)
        } else {
            0.0
        };

        Some((drift_rate_kb_per_sec, r2))
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
        if self.history.len() < 8 {
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

        let reg = self.calculate_linear_regression();

        if drops > 3 && increases > 3 {
            // Saw-tooth pattern
            Diagnosis::CacheEviction
        } else if increases > self.history.len() / 2 && drops == 0 {
            // Monotonic increase
            Diagnosis::StubbornLeak
        } else if let Some((drift_kb_s, r2)) = reg {
            // Stealthy Drift: Strong positive linear correlation (R^2 >= 0.70) and positive drift (> 0.005 KB/s)
            if r2 >= 0.70 && drift_kb_s > 0.005 {
                Diagnosis::StealthyDrift
            } else {
                Diagnosis::Unknown
            }
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
