pub mod injector;

pub trait MockManager {
    fn inject_mocks(&self, env_vars: &mut std::collections::HashMap<String, String>);
    fn cleanup(&self);
}
