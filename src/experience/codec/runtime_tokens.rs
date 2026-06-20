use super::super::model::ExperienceRuntimeTokenMetrics;
use super::fields::{field_to_finite_f32, option_f32_to_field};

pub(super) fn serialize_runtime_token_metrics(metrics: ExperienceRuntimeTokenMetrics) -> String {
    [
        metrics.token_count.to_string(),
        metrics.entropy_count.to_string(),
        metrics.logprob_count.to_string(),
        option_f32_to_field(metrics.average_entropy),
        option_f32_to_field(metrics.average_neg_logprob),
        option_f32_to_field(metrics.uncertainty_perplexity),
    ]
    .join(",")
}

pub(super) fn deserialize_runtime_token_metrics(
    value: &str,
) -> Option<ExperienceRuntimeTokenMetrics> {
    if value.is_empty() {
        return Some(ExperienceRuntimeTokenMetrics::default());
    }

    let fields = value.split(',').collect::<Vec<_>>();
    if fields.len() != 6 {
        return None;
    }

    Some(ExperienceRuntimeTokenMetrics {
        token_count: fields[0].parse::<usize>().ok()?,
        entropy_count: fields[1].parse::<usize>().ok()?,
        logprob_count: fields[2].parse::<usize>().ok()?,
        average_entropy: field_to_finite_f32(fields[3]),
        average_neg_logprob: field_to_finite_f32(fields[4]),
        uncertainty_perplexity: field_to_finite_f32(fields[5]),
    })
}
