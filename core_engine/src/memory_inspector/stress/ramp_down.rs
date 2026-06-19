pub struct ExponentialRampDown {
    steps: Vec<f64>,
    current_step: usize,
}

impl ExponentialRampDown {
    pub fn new(baseline_limit: f64) -> Self {
        // Reduce gradually: 100% -> 70% -> 50% -> 30% -> 15%
        let factors = vec![1.0, 0.7, 0.5, 0.3, 0.15];
        let steps = factors.into_iter().map(|f| baseline_limit * f).collect();
        Self {
            steps,
            current_step: 0,
        }
    }

    pub fn advance_step(&mut self) {
        if self.current_step < self.steps.len() - 1 {
            self.current_step += 1;
        }
    }
}

impl super::StressController for ExponentialRampDown {
    fn calculate_next_limit(&mut self, _current_usage: f64) -> f64 {
        self.steps[self.current_step]
    }

    fn is_finished(&self) -> bool {
        self.current_step >= self.steps.len() - 1
    }
}
