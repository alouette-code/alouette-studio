pub mod ramp_down;

pub trait StressController {
    fn calculate_next_limit(&mut self, current_usage: f64) -> f64;
    fn is_finished(&self) -> bool;
}
