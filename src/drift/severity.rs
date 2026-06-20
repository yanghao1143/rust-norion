#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftSeverity {
    Stable,
    Watch,
    Block,
    Rollback,
}

impl DriftSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Block => "block",
            Self::Rollback => "rollback",
        }
    }
}
