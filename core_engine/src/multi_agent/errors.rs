use std::fmt;

#[derive(Debug, Clone)]
pub enum MultiAgentError {
    StateError(String),
    ConfigError(String),
    ToolError(String),
    TokenLimitExceeded,
    IterationLimitExceeded(String),
    Timeout,
    Cancelled,
}

impl fmt::Display for MultiAgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateError(msg) => write!(f, "State Error: {}", msg),
            Self::ConfigError(msg) => write!(f, "Config Error: {}", msg),
            Self::ToolError(msg) => write!(f, "Tool Error: {}", msg),
            Self::TokenLimitExceeded => write!(f, "Token Limit Exceeded"),
            Self::IterationLimitExceeded(agent) => write!(f, "Iteration Limit Exceeded for Agent: {}", agent),
            Self::Timeout => write!(f, "Workflow Timeout"),
            Self::Cancelled => write!(f, "Workflow Cancelled"),
        }
    }
}

impl std::error::Error for MultiAgentError {}
