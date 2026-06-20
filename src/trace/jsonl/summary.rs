use crate::engine::InferenceOutcome;
use crate::kv_cache::MemoryUpdateReport;

use super::json::option_f32_json;

pub(super) fn memory_feedback_summaries(outcome: &InferenceOutcome) -> Vec<String> {
    outcome
        .memory_feedback
        .updates
        .iter()
        .map(memory_update_summary)
        .collect()
}

pub(super) fn memory_update_summary(update: &MemoryUpdateReport) -> String {
    format!(
        "{}#{}:amount={:.6}:before={}:after={}:delta={:.6}:applied={}:removed={}",
        update.action.as_str(),
        update.id,
        update.requested_amount,
        option_f32_json(update.strength_before),
        option_f32_json(update.strength_after),
        update.strength_delta,
        update.was_applied(),
        update.removed
    )
}

pub(super) fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}
