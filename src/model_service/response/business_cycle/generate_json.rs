use super::super::super::json::{option_str_service_json, service_json_string, service_u64_array};
use super::super::super::types::{ModelServiceBusinessCycleReport, profile_name};

pub(super) fn business_cycle_generate_json(report: &ModelServiceBusinessCycleReport) -> String {
    let outcome = &report.timed.outcome;
    generate_json(GenerateJsonInput {
        profile: profile_name(report.profile),
        elapsed_ms: report.timed.elapsed_ms,
        answer: &outcome.answer,
        quality: outcome.report.quality,
        process_reward: outcome.process_reward.total,
        action: outcome.process_reward.action.as_str(),
        experience_id: outcome.experience_id,
        feedback_memory_ids: &report.feedback_memory_ids,
        runtime_model: outcome.runtime_diagnostics.model_id.as_deref(),
        runtime_token_count: outcome.runtime_token_metrics.token_count,
        runtime_uncertainty_signal: outcome.runtime_token_metrics.has_uncertainty_signal(),
        traceable: report.traceable,
    })
}

fn generate_json(input: GenerateJsonInput<'_>) -> String {
    format!(
        "{{\"profile\":\"{}\",\"elapsed_ms\":{},\"answer\":{},\"quality\":{:.6},\"process_reward\":{:.6},\"action\":\"{}\",\"experience_id\":{},\"feedback_memory_ids\":{},\"runtime_model\":{},\"runtime_token_count\":{},\"runtime_uncertainty_signal\":{},\"traceable\":{}}}",
        input.profile,
        input.elapsed_ms,
        service_json_string(input.answer),
        input.quality,
        input.process_reward,
        input.action,
        input.experience_id,
        service_u64_array(input.feedback_memory_ids),
        option_str_service_json(input.runtime_model),
        input.runtime_token_count,
        input.runtime_uncertainty_signal,
        input.traceable
    )
}

struct GenerateJsonInput<'a> {
    profile: &'a str,
    elapsed_ms: u128,
    answer: &'a str,
    quality: f32,
    process_reward: f32,
    action: &'a str,
    experience_id: u64,
    feedback_memory_ids: &'a [u64],
    runtime_model: Option<&'a str>,
    runtime_token_count: usize,
    runtime_uncertainty_signal: bool,
    traceable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_json_renders_runtime_and_feedback_evidence() {
        let json = generate_json(GenerateJsonInput {
            profile: "coding",
            elapsed_ms: 123,
            answer: "hello \"Forge\"",
            quality: 0.8125,
            process_reward: 0.625,
            action: "reinforce",
            experience_id: 77,
            feedback_memory_ids: &[3, 5, 8],
            runtime_model: Some("gemma-12b-it-q4"),
            runtime_token_count: 42,
            runtime_uncertainty_signal: true,
            traceable: true,
        });

        assert!(json.contains("\"profile\":\"coding\""));
        assert!(json.contains("\"elapsed_ms\":123"));
        assert!(json.contains("\"answer\":\"hello \\\"Forge\\\"\""));
        assert!(json.contains("\"quality\":0.812500"));
        assert!(json.contains("\"process_reward\":0.625000"));
        assert!(json.contains("\"action\":\"reinforce\""));
        assert!(json.contains("\"experience_id\":77"));
        assert!(json.contains("\"feedback_memory_ids\":[3,5,8]"));
        assert!(json.contains("\"runtime_model\":\"gemma-12b-it-q4\""));
        assert!(json.contains("\"runtime_token_count\":42"));
        assert!(json.contains("\"runtime_uncertainty_signal\":true"));
        assert!(json.contains("\"traceable\":true"));
    }

    #[test]
    fn generate_json_renders_missing_runtime_model_as_null() {
        let json = generate_json(GenerateJsonInput {
            profile: "general",
            elapsed_ms: 0,
            answer: "",
            quality: 0.0,
            process_reward: 0.0,
            action: "hold",
            experience_id: 1,
            feedback_memory_ids: &[],
            runtime_model: None,
            runtime_token_count: 0,
            runtime_uncertainty_signal: false,
            traceable: false,
        });

        assert!(json.contains("\"runtime_model\":null"));
        assert!(json.contains("\"feedback_memory_ids\":[]"));
        assert!(json.contains("\"runtime_uncertainty_signal\":false"));
        assert!(json.contains("\"traceable\":false"));
    }
}
