use super::evaluation::evaluate_draft;
use super::model::{InferenceDraft, ReasoningStep};
use super::repair::{merged_actions, repair_answer, should_attempt_repair};
use super::report::ReflectionReport;

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
        let initial = evaluate_draft(prompt, draft, self.min_answer_chars, 0);
        if !should_attempt_repair(&initial) {
            return initial;
        }

        let repaired_answer = repair_answer(prompt, draft.answer.trim(), &initial);
        let mut repaired_trace = draft.trace.clone();
        repaired_trace.push(ReasoningStep::new(
            "reflection_repair",
            format!(
                "applied revision actions: {}",
                initial.revision_actions.join(",")
            ),
            0.74,
        ));
        let repaired_draft =
            InferenceDraft::new(repaired_answer, repaired_trace).with_tokens(draft.tokens.clone());
        let mut repaired = evaluate_draft(prompt, &repaired_draft, self.min_answer_chars, 1);

        if repaired.quality >= initial.quality {
            repaired.revision_actions =
                merged_actions(&initial.revision_actions, &repaired.revision_actions);
            repaired
                .revision_actions
                .push("reflection_repair_applied".to_owned());
            repaired.lesson = format!(
                "{} revision_passes={} initial_quality={:.3} initial_issues={}",
                repaired.lesson,
                repaired.revision_passes,
                initial.quality,
                initial.issues.len()
            );
            repaired
        } else {
            let mut rejected = initial;
            rejected
                .revision_actions
                .push("reflection_repair_rejected".to_owned());
            rejected
        }
    }
}
