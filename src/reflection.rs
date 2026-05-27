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
    pub revised_answer: String,
    pub store_as_memory: bool,
    pub lesson: String,
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
        let mut contradictions = Vec::new();
        let answer = draft.answer.trim();

        if answer.is_empty() {
            contradictions.push("empty_answer".to_owned());
        }
        if answer.chars().count() < self.min_answer_chars {
            contradictions.push("answer_too_short".to_owned());
        }
        if contains_conflicting_markers(answer) {
            contradictions.push("conflicting_certainty_markers".to_owned());
        }

        for step in &draft.trace {
            if step.confidence < 0.28 {
                contradictions.push(format!("low_confidence_step:{}", step.label));
            }
        }

        let average_confidence = if draft.trace.is_empty() {
            0.5
        } else {
            draft.trace.iter().map(|step| step.confidence).sum::<f32>() / draft.trace.len() as f32
        };
        let overlap = lexical_overlap(prompt, answer);
        let length_score = (answer.chars().count() as f32 / 240.0).min(1.0);
        let penalty = (contradictions.len() as f32 * 0.13).min(0.65);
        let quality = (overlap * 0.42 + average_confidence * 0.38 + length_score * 0.20 - penalty)
            .clamp(0.0, 1.0);

        let revised_answer = if contradictions.is_empty() {
            answer.to_owned()
        } else {
            format!(
                "{answer}\n\nReflection note: flagged {}; keep this response as tentative.",
                contradictions.join(",")
            )
        };
        let store_as_memory = quality >= 0.46 && contradictions.len() <= 1 && answer.len() >= 24;
        let lesson = if store_as_memory {
            format!("accepted_pattern quality={quality:.3} overlap={overlap:.3}")
        } else {
            format!(
                "rejected_pattern quality={quality:.3} issues={}",
                contradictions.len()
            )
        };

        ReflectionReport {
            quality,
            contradictions,
            revised_answer,
            store_as_memory,
            lesson,
        }
    }
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
}
