#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTier {
    Auto,
    Tiny,
    Constrained,
    Balanced,
    Accelerated,
    Distributed,
}

impl DeviceTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Tiny => "tiny",
            Self::Constrained => "constrained",
            Self::Balanced => "balanced",
            Self::Accelerated => "accelerated",
            Self::Distributed => "distributed",
        }
    }

    pub fn compute_headroom(self) -> f32 {
        match self {
            Self::Auto => 0.45,
            Self::Tiny => 0.08,
            Self::Constrained => 0.22,
            Self::Balanced => 0.50,
            Self::Accelerated => 0.78,
            Self::Distributed => 1.0,
        }
    }
}
