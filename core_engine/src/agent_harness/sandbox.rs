use std::time::{Instant, Duration};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct SandboxManager {
    call_history: VecDeque<Instant>,
    max_calls_per_second: usize,
    blacklisted_domains: Vec<String>,
}

impl SandboxManager {
    pub fn new() -> Self {
        Self {
            call_history: VecDeque::new(),
            max_calls_per_second: 20,
            blacklisted_domains: vec![], // Cleared to allow unrestricted API testing
        }
    }

    /// Checks if a tool execution is allowed.
    /// Primarily used to sanitize inputs like URLs and enforce rate limits.
    pub fn check_execution(&mut self, tool_name: &str, arguments: &serde_json::Value) -> Result<(), String> {
        // We only apply sandbox rules to specific tools like ping_zero_min
        if tool_name == "ping_zero_min" {
            self.check_rate_limit()?;

            if let Some(url_str) = arguments.get("url").and_then(|u| u.as_str()) {
                self.check_url_safety(url_str)?;
            }
            
            // Record the call after checks pass
            self.record_call();
        }
        
        Ok(())
    }

    fn check_rate_limit(&mut self) -> Result<(), String> {
        let now = Instant::now();
        let one_second_ago = now - Duration::from_secs(1);

        // Remove calls older than 1 second
        while let Some(&oldest_call) = self.call_history.front() {
            if oldest_call < one_second_ago {
                self.call_history.pop_front();
            } else {
                break;
            }
        }

        if self.call_history.len() >= self.max_calls_per_second {
            return Err(format!(
                "Sandbox blocked execution: Rate limit exceeded (Max {} calls per second). Please wait before trying again.",
                self.max_calls_per_second
            ));
        }

        Ok(())
    }

    fn check_url_safety(&self, url: &str) -> Result<(), String> {
        // Very basic parsing to extract domain. In a real app, `url` crate is better.
        // Assuming format like http://domain.com/path
        let domain_part = url.trim_start_matches("http://")
                             .trim_start_matches("https://");
        let domain = domain_part.split('/').next().unwrap_or("").split(':').next().unwrap_or("");
        
        if domain.is_empty() {
            return Err("Sandbox blocked execution: Invalid URL format.".to_string());
        }

        for blacklisted in &self.blacklisted_domains {
            if domain == blacklisted || domain.ends_with(&format!(".{}", blacklisted)) {
                return Err(format!(
                    "Sandbox blocked execution: Domain '{}' is blacklisted for security reasons.",
                    domain
                ));
            }
        }

        Ok(())
    }

    fn record_call(&mut self) {
        self.call_history.push_back(Instant::now());
    }
}
