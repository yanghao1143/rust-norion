use crate::hierarchy::HierarchyWeights;
use crate::reflection::{InferenceDraft, ReasoningStep, ReflectionReport};
use crate::router::{GenerationMetrics, RouteBudget};

use super::text::{approximate_token_count, compact};
use super::types::RuntimeTokenMetrics;

pub(super) fn metrics_from_report(
    draft: &InferenceDraft,
    report: &ReflectionReport,
    route_budget: RouteBudget,
    runtime_token_metrics: RuntimeTokenMetrics,
) -> GenerationMetrics {
    let token_count =
        approximate_token_count(&report.revised_answer).max(approximate_token_count(&draft.answer));
    let route_pressure = (1.0 - route_budget.attention_fraction).max(0.0) * 2.5;
    let baseline_perplexity = 4.0
        + (1.0 - report.quality) * 24.0
        + route_pressure
        + report.contradictions.len() as f32 * 3.5;
    let perplexity = runtime_token_metrics
        .uncertainty_perplexity
        .map(|runtime_perplexity| baseline_perplexity * 0.55 + runtime_perplexity * 0.45)
        .unwrap_or(baseline_perplexity);

    GenerationMetrics {
        perplexity,
        semantic_consistency: report.quality,
        contradiction_count: report.contradictions.len(),
        token_count,
    }
}

pub(super) fn average(total: f32, count: usize) -> Option<f32> {
    if count == 0 {
        None
    } else {
        Some(total / count as f32)
    }
}

pub(super) fn runtime_error_note_from_trace(trace: &[ReasoningStep]) -> Option<String> {
    trace
        .iter()
        .find(|step| step.label.starts_with("runtime") && step.label.contains("error"))
        .map(|step| {
            let timeout = step.content.to_ascii_lowercase().contains("timed out");
            format!(
                "runtime_error:label={}:timeout={}:message_chars={}",
                compact_note_value(&step.label, 48),
                timeout,
                step.content.chars().count()
            )
        })
}

fn compact_note_value(value: &str, max_chars: usize) -> String {
    compact(value, max_chars)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn hierarchy_weight_delta(before: HierarchyWeights, after: HierarchyWeights) -> f32 {
    ((before.global - after.global).abs()
        + (before.local - after.local).abs()
        + (before.convolution - after.convolution).abs())
        / 3.0
}
