use std::collections::HashSet;

use crate::kv_exchange::RuntimeKvBlock;

#[derive(Debug, Clone)]
pub struct ReasoningStep {
    pub label: String,
    pub content: String,
    pub confidence: f32,
}

impl ReasoningStep {
    pub fn new(label: impl Into<String>, content: impl Into<String>, confidence: f32) -> Self {
        Self {
            label: label.into(),
            content: content.into(),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DraftToken {
    pub text: String,
    pub logprob: Option<f32>,
    pub entropy: Option<f32>,
}

impl DraftToken {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            logprob: None,
            entropy: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InferenceDraft {
    pub answer: String,
    pub trace: Vec<ReasoningStep>,
    pub tokens: Vec<DraftToken>,
    pub exported_kv_blocks: Vec<RuntimeKvBlock>,
}

impl InferenceDraft {
    pub fn new(answer: impl Into<String>, trace: Vec<ReasoningStep>) -> Self {
        Self {
            answer: answer.into(),
            trace,
            tokens: Vec::new(),
            exported_kv_blocks: Vec::new(),
        }
    }

    pub fn with_tokens(mut self, tokens: Vec<DraftToken>) -> Self {
        self.tokens = tokens;
        self
    }

    pub fn with_exported_kv_blocks(mut self, blocks: Vec<RuntimeKvBlock>) -> Self {
        self.exported_kv_blocks = blocks;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ReflectionReport {
    pub quality: f32,
    pub contradictions: Vec<String>,
    pub issues: Vec<ReflectionIssue>,
    pub revision_actions: Vec<String>,
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

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    fn penalty(self) -> f32 {
        match self {
            Self::Info => 0.04,
            Self::Warning => 0.11,
            Self::Critical => 0.24,
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Critical => 2,
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

#[derive(Debug, Clone)]
pub struct Reflector {
    min_answer_chars: usize,
}

impl Default for Reflector {
    fn default() -> Self {
        Self {
            min_answer_chars: 48,
        }
    }
}

impl Reflector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reflect(&self, prompt: &str, draft: &InferenceDraft) -> ReflectionReport {
        let mut issues = Vec::new();
        let mut revision_actions = Vec::new();
        let answer = draft.answer.trim();

        if answer.is_empty() {
            add_issue(
                &mut issues,
                &mut revision_actions,
                "empty_answer",
                ReflectionSeverity::Critical,
                "draft answer is empty",
                "reject_empty_answer",
            );
        }
        if answer.chars().count() < self.min_answer_chars {
            add_issue(
                &mut issues,
                &mut revision_actions,
                "answer_too_short",
                ReflectionSeverity::Warning,
                "draft answer is shorter than the configured minimum",
                "expand_short_answer",
            );
        }
        if contains_conflicting_markers(answer) {
            add_issue(
                &mut issues,
                &mut revision_actions,
                "conflicting_certainty_markers",
                ReflectionSeverity::Critical,
                "answer mixes certainty and uncertainty markers",
                "mark_tentative_for_conflicting_certainty",
            );
        }

        for step in &draft.trace {
            if step.confidence < 0.28 {
                add_issue(
                    &mut issues,
                    &mut revision_actions,
                    format!("low_confidence_step:{}", step.label),
                    if step.confidence < 0.14 {
                        ReflectionSeverity::Critical
                    } else {
                        ReflectionSeverity::Warning
                    },
                    format!("reasoning step confidence {:.3}", step.confidence),
                    format!("review_low_confidence_step:{}", step.label),
                );
            }
        }

        let average_confidence = if draft.trace.is_empty() {
            0.5
        } else {
            draft.trace.iter().map(|step| step.confidence).sum::<f32>() / draft.trace.len() as f32
        };
        let overlap = lexical_overlap(prompt, answer);
        if !answer.is_empty() && overlap < 0.12 {
            add_issue(
                &mut issues,
                &mut revision_actions,
                "low_prompt_overlap",
                ReflectionSeverity::Warning,
                format!("lexical overlap {:.3} below grounding threshold", overlap),
                "increase_prompt_grounding",
            );
        }
        let repetition = repetition_ratio(answer);
        if repetition >= 0.58 {
            add_issue(
                &mut issues,
                &mut revision_actions,
                "repetitive_answer",
                ReflectionSeverity::Warning,
                format!("unique-token repetition ratio {:.3}", repetition),
                "deduplicate_repeated_phrases",
            );
        }
        if let Some(uncertainty) = token_uncertainty(&draft.tokens) {
            if uncertainty >= 0.72 {
                add_issue(
                    &mut issues,
                    &mut revision_actions,
                    "high_token_uncertainty",
                    ReflectionSeverity::Warning,
                    format!("token uncertainty {:.3}", uncertainty),
                    "increase_attention_or_resample",
                );
            }
        }

        let length_score = (answer.chars().count() as f32 / 240.0).min(1.0);
        let penalty = issues
            .iter()
            .map(|issue| issue.severity.penalty())
            .sum::<f32>()
            .min(0.72);
        let quality = (overlap * 0.42 + average_confidence * 0.38 + length_score * 0.20 - penalty)
            .clamp(0.0, 1.0);

        let contradictions = issues
            .iter()
            .filter(|issue| issue.severity == ReflectionSeverity::Critical)
            .map(|issue| issue.code.clone())
            .collect::<Vec<_>>();
        let revised_answer = if issues.is_empty() {
            answer.to_owned()
        } else {
            let answer_for_note = if answer.is_empty() {
                "[empty draft]"
            } else {
                answer
            };
            format!(
                "{answer_for_note}\n\nReflection note: flagged {}; actions {}; keep this response as tentative.",
                contradictions.join(","),
                revision_actions.join(",")
            )
        };
        let critical_issue_count = issues
            .iter()
            .filter(|issue| issue.severity == ReflectionSeverity::Critical)
            .count();
        let store_as_memory =
            quality >= 0.46 && critical_issue_count == 0 && issues.len() <= 2 && answer.len() >= 24;
        let max_severity = issues
            .iter()
            .map(|issue| issue.severity)
            .max_by_key(|severity| severity.rank())
            .unwrap_or(ReflectionSeverity::Info);
        let lesson = if store_as_memory {
            format!(
                "accepted_pattern quality={quality:.3} overlap={overlap:.3} issues={} max_severity={}",
                issues.len(),
                max_severity.as_str()
            )
        } else {
            format!(
                "rejected_pattern quality={quality:.3} issues={} critical={} max_severity={} actions={}",
                issues.len(),
                critical_issue_count,
                max_severity.as_str(),
                revision_actions.join(",")
            )
        };

        ReflectionReport {
            quality,
            contradictions,
            issues,
            revision_actions,
            revised_answer,
            store_as_memory,
            lesson,
        }
    }
}

fn add_issue(
    issues: &mut Vec<ReflectionIssue>,
    revision_actions: &mut Vec<String>,
    code: impl Into<String>,
    severity: ReflectionSeverity,
    detail: impl Into<String>,
    action: impl Into<String>,
) {
    issues.push(ReflectionIssue::new(code, severity, detail));
    revision_actions.push(action.into());
}

fn contains_conflicting_markers(answer: &str) -> bool {
    let lower = answer.to_ascii_lowercase();
    (lower.contains("certain") || lower.contains("guaranteed"))
        && (lower.contains("uncertain") || lower.contains("unknown") || lower.contains("maybe"))
}

fn lexical_overlap(prompt: &str, answer: &str) -> f32 {
    let prompt_chars = prompt
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .collect::<HashSet<_>>();
    let answer_chars = answer
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .collect::<HashSet<_>>();

    if prompt_chars.is_empty() || answer_chars.is_empty() {
        return 0.0;
    }

    let shared = prompt_chars.intersection(&answer_chars).count() as f32;
    let denom = prompt_chars.len().min(answer_chars.len()) as f32;
    (shared / denom).clamp(0.0, 1.0)
}

fn repetition_ratio(answer: &str) -> f32 {
    let words = answer
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|ch| !ch.is_ascii_punctuation())
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    if words.len() < 6 {
        return 0.0;
    }

    let unique = words.iter().collect::<HashSet<_>>().len();
    (1.0 - unique as f32 / words.len() as f32).clamp(0.0, 1.0)
}

fn token_uncertainty(tokens: &[DraftToken]) -> Option<f32> {
    let mut scores = Vec::new();

    for token in tokens {
        if let Some(entropy) = token.entropy {
            scores.push((entropy / 4.0).clamp(0.0, 1.0));
        }
        if let Some(logprob) = token.logprob {
            scores.push((-logprob / 4.0).clamp(0.0, 1.0));
        }
    }

    if scores.is_empty() {
        None
    } else {
        Some(scores.iter().sum::<f32>() / scores.len() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_answer_is_rejected() {
        let report = Reflector::new().reflect("build a router", &InferenceDraft::new("", vec![]));

        assert!(!report.store_as_memory);
        assert!(
            report
                .contradictions
                .iter()
                .any(|item| item == "empty_answer")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "empty_answer"
                    && issue.severity == ReflectionSeverity::Critical)
        );
        assert!(
            report
                .revision_actions
                .iter()
                .any(|action| action == "reject_empty_answer")
        );
    }

    #[test]
    fn useful_answer_can_be_stored() {
        let draft = InferenceDraft::new(
            "Build a Rust router that observes quality metrics and adjusts the entropy threshold.",
            vec![ReasoningStep::new("plan", "route by entropy", 0.9)],
        );

        let report = Reflector::new().reflect("Rust router quality metrics", &draft);

        assert!(report.quality > 0.46);
        assert!(report.store_as_memory);
    }

    #[test]
    fn conflicting_and_uncertain_draft_gets_structured_actions() {
        let mut token = DraftToken::new("maybe");
        token.entropy = Some(3.6);
        token.logprob = Some(-3.4);
        let draft = InferenceDraft::new(
            "The result is certain and guaranteed, but maybe unknown. repeat repeat repeat repeat repeat repeat.",
            vec![ReasoningStep::new("verify", "weak evidence", 0.10)],
        )
        .with_tokens(vec![token]);

        let report = Reflector::new().reflect("verify result carefully", &draft);

        assert!(!report.store_as_memory);
        assert!(report.critical_issue_count() >= 2);
        assert!(
            report
                .issue_codes()
                .iter()
                .any(|code| code == "conflicting_certainty_markers")
        );
        assert!(
            report
                .revision_actions
                .iter()
                .any(|action| action == "increase_attention_or_resample")
        );
        assert!(report.revised_answer.contains("Reflection note"));
    }
}
