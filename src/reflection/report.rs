#[derive(Debug, Clone)]
pub struct ReflectionReport {
    pub quality: f32,
    pub contradictions: Vec<String>,
    pub issues: Vec<ReflectionIssue>,
    pub revision_actions: Vec<String>,
    pub revision_passes: usize,
    pub revised_answer: String,
    pub store_as_memory: bool,
    pub lesson: String,
}

impl ReflectionReport {
    pub fn issue_codes(&self) -> Vec<String> {
        self.issues.iter().map(|issue| issue.code.clone()).collect()
    }

    pub fn critical_issue_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == ReflectionSeverity::Critical)
            .count()
    }

    pub fn max_severity(&self) -> ReflectionSeverity {
        self.issues
            .iter()
            .map(|issue| issue.severity)
            .max_by_key(|severity| severity.rank())
            .unwrap_or(ReflectionSeverity::Info)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectionSeverity {
    Info,
    Warning,
    Critical,
}

impl ReflectionSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    pub(super) fn penalty(self) -> f32 {
        match self {
            Self::Info => 0.04,
            Self::Warning => 0.11,
            Self::Critical => 0.24,
        }
    }

    pub(super) fn rank(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Critical => 2,
        }
    }
}

impl std::str::FromStr for ReflectionSeverity {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "critical" => Ok(Self::Critical),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReflectionIssue {
    pub code: String,
    pub severity: ReflectionSeverity,
    pub detail: String,
}

impl ReflectionIssue {
    pub fn new(
        code: impl Into<String>,
        severity: ReflectionSeverity,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            detail: detail.into(),
        }
    }
}
