use rust_norion::{InferenceOutcome, TaskProfile};

use super::super::json::{
    option_f32_service_json, option_str_service_json, option_u64_service_json, service_json_string,
    service_u64_array,
};
use super::super::request::ModelServiceOutputMode;
use super::super::types::{TimedOutcome, profile_name};

pub(crate) fn model_service_response_json(
    request_id: usize,
    profile: TaskProfile,
    traceable: bool,
    output_mode: ModelServiceOutputMode,
    timed: &TimedOutcome,
) -> String {
    let outcome = &timed.outcome;
    let used_memory_ids = outcome
        .used_memories
        .iter()
        .map(|memory| memory.id)
        .collect::<Vec<_>>();
    let feedback_memory_ids = model_service_outcome_feedback_memory_ids(outcome);
    let runtime_uncertainty_token_count = outcome
        .runtime_token_metrics
        .entropy_count
        .saturating_add(outcome.runtime_token_metrics.logprob_count);
    let answer = match output_mode {
        ModelServiceOutputMode::Enhanced => outcome.answer.as_str(),
        ModelServiceOutputMode::Raw => outcome.raw_answer.as_str(),
    };
    format!(
        "{{\"ok\":true,\"request_id\":{},\"profile\":\"{}\",\"elapsed_ms\":{},\"output_mode\":\"{}\",\"answer\":{},\"raw_answer\":{},\"enhanced_answer\":{},\"quality\":{:.6},\"process_reward\":{:.6},\"action\":\"{}\",\"memory_stored\":{},\"stored_memory_id\":{},\"used_memory_ids\":{},\"stored_gist_memory_ids\":{},\"stored_runtime_kv_memory_ids\":{},\"feedback_memory_ids\":{},\"experience_id\":{},\"runtime_model\":{},\"runtime_token_count\":{},\"runtime_entropy_count\":{},\"runtime_logprob_count\":{},\"runtime_uncertainty_token_count\":{},\"runtime_uncertainty_signal\":{},\"runtime_average_entropy\":{},\"runtime_average_neg_logprob\":{},\"runtime_uncertainty_perplexity\":{},\"runtime_architecture_signal\":{},\"runtime_kv_precision_signal\":{},\"runtime_device_execution_source\":{},\"traceable\":{}}}",
        request_id,
        profile_name(profile),
        timed.elapsed_ms,
        output_mode.as_str(),
        service_json_string(answer),
        service_json_string(&outcome.raw_answer),
        service_json_string(&outcome.answer),
        outcome.report.quality,
        outcome.process_reward.total,
        outcome.process_reward.action.as_str(),
        option_u64_service_json(outcome.stored_memory_id),
        option_u64_service_json(outcome.stored_memory_id),
        service_u64_array(&used_memory_ids),
        service_u64_array(&outcome.stored_gist_memory_ids),
        service_u64_array(&outcome.stored_runtime_kv_memory_ids),
        service_u64_array(&feedback_memory_ids),
        outcome.experience_id,
        option_str_service_json(outcome.runtime_diagnostics.model_id.as_deref()),
        outcome.runtime_token_metrics.token_count,
        outcome.runtime_token_metrics.entropy_count,
        outcome.runtime_token_metrics.logprob_count,
        runtime_uncertainty_token_count,
        outcome.runtime_token_metrics.has_uncertainty_signal(),
        option_f32_service_json(outcome.runtime_token_metrics.average_entropy),
        option_f32_service_json(outcome.runtime_token_metrics.average_neg_logprob),
        option_f32_service_json(outcome.runtime_token_metrics.uncertainty_perplexity),
        outcome
            .runtime_diagnostics
            .has_runtime_architecture_signal(),
        outcome.runtime_diagnostics.has_valid_kv_precision_signal(),
        option_str_service_json(
            outcome
                .runtime_diagnostics
                .device_execution_source
                .as_deref()
        ),
        traceable,
    )
}

pub(crate) fn openai_chat_completion_response_json(
    request_id: usize,
    profile: TaskProfile,
    model_hint: Option<&str>,
    output_mode: ModelServiceOutputMode,
    timed: &TimedOutcome,
) -> String {
    let outcome = &timed.outcome;
    let answer = match output_mode {
        ModelServiceOutputMode::Enhanced => outcome.answer.as_str(),
        ModelServiceOutputMode::Raw => outcome.raw_answer.as_str(),
    };
    let model = model_hint
        .filter(|model| !model.trim().is_empty())
        .or(outcome.runtime_diagnostics.model_id.as_deref())
        .unwrap_or("rust-norion-local");
    let completion_tokens = outcome.runtime_token_metrics.token_count;
    format!(
        "{{\"id\":\"chatcmpl-norion-{}\",\"object\":\"chat.completion\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"message\":{{\"role\":\"assistant\",\"content\":{}}},\"finish_reason\":\"stop\"}}],\"usage\":{{\"prompt_tokens\":0,\"completion_tokens\":{},\"total_tokens\":{}}},\"norion\":{{\"request_id\":{},\"profile\":\"{}\",\"elapsed_ms\":{},\"output_mode\":\"{}\",\"quality\":{:.6},\"experience_id\":{},\"memory_stored\":{},\"runtime_token_count\":{},\"persistent_writes\":true}}}}",
        request_id,
        unix_timestamp_seconds(),
        service_json_string(model),
        service_json_string(answer),
        completion_tokens,
        completion_tokens,
        request_id,
        profile_name(profile),
        timed.elapsed_ms,
        output_mode.as_str(),
        outcome.report.quality,
        outcome.experience_id,
        option_u64_service_json(outcome.stored_memory_id),
        completion_tokens
    )
}

pub(crate) fn openai_completion_response_json(
    request_id: usize,
    profile: TaskProfile,
    model_hint: Option<&str>,
    output_mode: ModelServiceOutputMode,
    timed: &TimedOutcome,
) -> String {
    let outcome = &timed.outcome;
    let answer = match output_mode {
        ModelServiceOutputMode::Enhanced => outcome.answer.as_str(),
        ModelServiceOutputMode::Raw => outcome.raw_answer.as_str(),
    };
    let model = model_hint
        .filter(|model| !model.trim().is_empty())
        .or(outcome.runtime_diagnostics.model_id.as_deref())
        .unwrap_or("rust-norion-local");
    let completion_tokens = outcome.runtime_token_metrics.token_count;
    format!(
        "{{\"id\":\"cmpl-norion-{}\",\"object\":\"text_completion\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"text\":{},\"finish_reason\":\"stop\"}}],\"usage\":{{\"prompt_tokens\":0,\"completion_tokens\":{},\"total_tokens\":{}}},\"norion\":{{\"request_id\":{},\"profile\":\"{}\",\"elapsed_ms\":{},\"output_mode\":\"{}\",\"quality\":{:.6},\"experience_id\":{},\"memory_stored\":{},\"runtime_token_count\":{},\"persistent_writes\":true}}}}",
        request_id,
        unix_timestamp_seconds(),
        service_json_string(model),
        service_json_string(answer),
        completion_tokens,
        completion_tokens,
        request_id,
        profile_name(profile),
        timed.elapsed_ms,
        output_mode.as_str(),
        outcome.report.quality,
        outcome.experience_id,
        option_u64_service_json(outcome.stored_memory_id),
        completion_tokens
    )
}

fn unix_timestamp_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn model_service_outcome_feedback_memory_ids(outcome: &InferenceOutcome) -> Vec<u64> {
    let mut memory_ids = Vec::new();
    if let Some(memory_id) = outcome.stored_memory_id {
        push_unique_u64(&mut memory_ids, memory_id);
    }
    for memory in &outcome.used_memories {
        push_unique_u64(&mut memory_ids, memory.id);
    }
    for memory_id in &outcome.stored_gist_memory_ids {
        push_unique_u64(&mut memory_ids, *memory_id);
    }
    for memory_id in &outcome.stored_runtime_kv_memory_ids {
        push_unique_u64(&mut memory_ids, *memory_id);
    }
    memory_ids
}

fn push_unique_u64(values: &mut Vec<u64>, value: u64) {
    if !values.contains(&value) {
        values.push(value);
    }
}
