use crate::task::AgentRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentMessageKind {
    Task,
    Finding,
    Risk,
    Gate,
    Decision,
    Critique,
    Revision,
    MemoryNote,
    Status,
}

impl AgentMessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Finding => "finding",
            Self::Risk => "risk",
            Self::Gate => "gate",
            Self::Decision => "decision",
            Self::Critique => "critique",
            Self::Revision => "revision",
            Self::MemoryNote => "memory_note",
            Self::Status => "status",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMessage {
    pub id: String,
    pub role: AgentRole,
    pub kind: AgentMessageKind,
    pub topic: String,
    pub content: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub conflict: bool,
    pub conflict_topic: Option<String>,
}

impl AgentMessage {
    pub fn new(
        id: impl Into<String>,
        role: AgentRole,
        kind: AgentMessageKind,
        topic: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            role,
            kind,
            topic: topic.into(),
            content: content.into(),
            confidence: 0.5,
            evidence: Vec::new(),
            conflict: false,
            conflict_topic: None,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }

    pub fn mark_conflict(&mut self, topic: impl Into<String>) {
        self.conflict = true;
        self.conflict_topic = Some(topic.into());
    }

    pub fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.role.as_str(),
            self.kind.as_str(),
            normalize(&self.topic),
            normalize(&self.content)
        )
    }

    pub fn summary(&self) -> String {
        format!(
            "{}:{}:{} confidence={:.2} conflict={}",
            self.role.as_str(),
            self.kind.as_str(),
            compact(&self.content, 96),
            self.confidence,
            self.conflict
        )
    }
}

pub(crate) fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_ascii_lowercase()
}

pub(crate) fn compact(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut output = normalized
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    output.push_str("...");
    output
}
