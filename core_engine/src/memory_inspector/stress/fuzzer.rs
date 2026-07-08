use super::StressController;
use rand::Rng;

pub struct ChaosFuzzer {
    _base_limit_mb: f64,
    current_limit_mb: f64,
    min_limit_mb: f64,
    iteration: u32,
    max_iterations: u32,
    chaos_probability: f64, // 0.0 to 1.0
}

impl ChaosFuzzer {
    pub fn new(base_limit_mb: f64, max_iterations: u32, chaos_probability: f64) -> Self {
        Self {
            _base_limit_mb: base_limit_mb,
            current_limit_mb: base_limit_mb,
            min_limit_mb: base_limit_mb * 0.1, // Don't go below 10% of base
            iteration: 0,
            max_iterations,
            chaos_probability,
        }
    }
    
    pub fn should_inject_chaos(&self) -> bool {
        let mut rng = rand::thread_rng();
        rng.gen_bool(self.chaos_probability)
    }
}

impl StressController for ChaosFuzzer {
    fn calculate_next_limit(&mut self, current_usage_mb: f64) -> f64 {
        self.iteration += 1;
        
        // Slowly ramp down like ExponentialRampDown, but occasionally jump
        let reduction_factor = 0.95;
        self.current_limit_mb = (self.current_limit_mb * reduction_factor).max(current_usage_mb + 20.0).max(50.0);
        
        self.current_limit_mb.max(self.min_limit_mb)
    }

    fn is_finished(&self) -> bool {
        self.iteration >= self.max_iterations
    }
}
