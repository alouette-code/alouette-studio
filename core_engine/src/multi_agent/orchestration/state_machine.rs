#[derive(Debug, Clone, PartialEq)]
pub enum OrchestratorState {
    Init,
    Planning,
    Coding,
    Verifying,
    Finished(String),
    Error(String),
    Halted(String), // Tạm dừng chờ User duyệt
}

impl OrchestratorState {
    pub fn next(&self) -> Self {
        match self {
            Self::Init => Self::Planning,
            Self::Planning => Self::Coding,
            Self::Coding => Self::Verifying,
            Self::Verifying => Self::Finished("Completed workflow".to_string()),
            _ => self.clone(),
        }
    }
}
