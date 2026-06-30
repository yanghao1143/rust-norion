use rust_norion::{InferenceOutcome, TaskProfile};

use super::super::json::{
    option_f32_service_json, option_str_service_json, option_u64_service_json, service_json_string,
    service_u64_array,
};
use super::super::request::ModelServiceOutputMode;
use super::super::types::{TimedOutcome, profile_name};

#[derive(Clone, Copy)]
pub(crate) struct ModelServiceTaskIntentMetadata {
    language_mode: &'static str,
    coding_language: &'static str,
    rust_coding: bool,
}

pub(crate) fn model_service_task_intent_metadata(
    prompt: &str,
    profile: TaskProfile,
) -> ModelServiceTaskIntentMetadata {
    let language_mode = if prompt.chars().any(is_cjk_unified_ideograph) {
        "chinese"
    } else if prompt
        .chars()
        .any(|character| character.is_ascii_alphabetic())
    {
        "english"
    } else {
        "auto"
    };
    let rust_coding = profile == TaskProfile::Coding && prompt_mentions_rust(prompt);
    let coding_language = match (profile, rust_coding) {
        (TaskProfile::Coding, true) => "rust",
        (TaskProfile::Coding, false) => "unspecified",
        _ => "none",
    };
    ModelServiceTaskIntentMetadata {
        language_mode,
        coding_language,
        rust_coding,
    }
}

pub(crate) fn model_service_response_json(
    request_id: usize,
    profile: TaskProfile,
    traceable: bool,
    output_mode: ModelServiceOutputMode,
    task_intent: ModelServiceTaskIntentMetadata,
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
    let task_metadata = model_service_task_metadata_json(outcome, task_intent);
    let route_metadata = model_service_route_budget_metadata_json(outcome);
    let runtime_kv_metadata = model_service_runtime_kv_metadata_json(outcome);
    format!(
        "{{\"ok\":true,\"request_id\":{},\"profile\":\"{}\",{},{},\"elapsed_ms\":{},\"output_mode\":\"{}\",\"answer\":{},\"raw_answer\":{},\"enhanced_answer\":{},\"quality\":{:.6},\"process_reward\":{:.6},\"action\":\"{}\",\"memory_stored\":{},\"stored_memory_id\":{},\"used_memory_count\":{},\"used_memory_ids\":{},\"stored_gist_memory_ids\":{},\"stored_runtime_kv_memory_ids\":{},\"feedback_memory_ids\":{},\"experience_id\":{},\"runtime_model\":{},\"runtime_token_count\":{},\"runtime_entropy_count\":{},\"runtime_logprob_count\":{},\"runtime_uncertainty_token_count\":{},\"runtime_uncertainty_signal\":{},\"runtime_average_entropy\":{},\"runtime_average_neg_logprob\":{},\"runtime_uncertainty_perplexity\":{},\"runtime_architecture_signal\":{},\"runtime_kv_precision_signal\":{},\"runtime_device_execution_source\":{}, {},\"traceable\":{}}}",
        request_id,
        profile_name(profile),
        task_metadata,
        route_metadata,
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
        used_memory_ids.len(),
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
        runtime_kv_metadata,
        traceable,
    )
}

pub(crate) fn openai_chat_completion_response_json(
    request_id: usize,
    endpoint: &str,
    profile: TaskProfile,
    model_hint: Option<&str>,
    output_mode: ModelServiceOutputMode,
    task_intent: ModelServiceTaskIntentMetadata,
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
    let runtime_metadata = openai_norion_runtime_metadata_json(outcome);
    let task_metadata = model_service_task_metadata_json(outcome, task_intent);
    format!(
        "{{\"id\":\"chatcmpl-norion-{}\",\"object\":\"chat.completion\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"message\":{{\"role\":\"assistant\",\"content\":{}}},\"finish_reason\":\"stop\"}}],\"usage\":{{\"prompt_tokens\":0,\"completion_tokens\":{},\"total_tokens\":{}}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"profile\":\"{}\",{},\"cancelled\":false,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"elapsed_ms\":{},\"output_mode\":\"{}\",\"quality\":{:.6},\"experience_id\":{},\"memory_stored\":{}, {},\"persistent_writes\":true,\"memory_write_allowed\":true,\"genome_write_allowed\":true,\"self_evolution_write_allowed\":true}}}}",
        request_id,
        unix_timestamp_seconds(),
        service_json_string(model),
        service_json_string(answer),
        completion_tokens,
        completion_tokens,
        request_id,
        service_json_string(endpoint),
        service_json_string(model),
        profile_name(profile),
        task_metadata,
        timed.elapsed_ms,
        output_mode.as_str(),
        outcome.report.quality,
        outcome.experience_id,
        option_u64_service_json(outcome.stored_memory_id),
        runtime_metadata
    )
}

pub(crate) fn openai_completion_response_json(
    request_id: usize,
    endpoint: &str,
    profile: TaskProfile,
    model_hint: Option<&str>,
    output_mode: ModelServiceOutputMode,
    task_intent: ModelServiceTaskIntentMetadata,
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
    let runtime_metadata = openai_norion_runtime_metadata_json(outcome);
    let task_metadata = model_service_task_metadata_json(outcome, task_intent);
    format!(
        "{{\"id\":\"cmpl-norion-{}\",\"object\":\"text_completion\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"text\":{},\"finish_reason\":\"stop\"}}],\"usage\":{{\"prompt_tokens\":0,\"completion_tokens\":{},\"total_tokens\":{}}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"profile\":\"{}\",{},\"cancelled\":false,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"elapsed_ms\":{},\"output_mode\":\"{}\",\"quality\":{:.6},\"experience_id\":{},\"memory_stored\":{}, {},\"persistent_writes\":true,\"memory_write_allowed\":true,\"genome_write_allowed\":true,\"self_evolution_write_allowed\":true}}}}",
        request_id,
        unix_timestamp_seconds(),
        service_json_string(model),
        service_json_string(answer),
        completion_tokens,
        completion_tokens,
        request_id,
        service_json_string(endpoint),
        service_json_string(model),
        profile_name(profile),
        task_metadata,
        timed.elapsed_ms,
        output_mode.as_str(),
        outcome.report.quality,
        outcome.experience_id,
        option_u64_service_json(outcome.stored_memory_id),
        runtime_metadata
    )
}

pub(crate) fn model_service_task_metadata_json(
    outcome: &InferenceOutcome,
    task_intent: ModelServiceTaskIntentMetadata,
) -> String {
    let plan = &outcome.task_hierarchy_plan;
    let signals = &plan.signals;
    let compute_budget = &outcome.compute_budget_schedule;
    let fanout_reduction = compute_budget
        .route_fanout_before
        .saturating_sub(compute_budget.route_fanout_after);
    format!(
        "\"language_mode\":{},\"coding_language\":{},\"rust_coding\":{},\"task_mode\":{},\"task_language\":{},\"coding_intent\":{},\"validation_mode\":{},\"memory_need\":{:.6},\"compute_budget\":{},\"compute_budget_summary\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_fanout_reduction\":{},\"compute_budget_read_only\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{}",
        service_json_string(task_intent.language_mode),
        service_json_string(task_intent.coding_language),
        task_intent.rust_coding,
        service_json_string(plan.mode.as_str()),
        service_json_string(signals.language.as_str()),
        signals.coding_intent,
        signals.validation_mode,
        signals.memory_need,
        service_json_string(signals.compute_budget.as_str()),
        service_json_string(&compute_budget.summary_line()),
        compute_budget.saved_tokens,
        compute_budget.wasted_compute_avoided_tokens,
        compute_budget.kv_lookups_skipped,
        fanout_reduction,
        compute_budget.read_only,
        compute_budget.write_allowed,
        compute_budget.applied
    )
}

pub(crate) fn openai_norion_runtime_metadata_json(outcome: &InferenceOutcome) -> String {
    let runtime_uncertainty_token_count = outcome
        .runtime_token_metrics
        .entropy_count
        .saturating_add(outcome.runtime_token_metrics.logprob_count);
    let route_metadata = model_service_route_budget_metadata_json(outcome);
    let runtime_kv_metadata = model_service_runtime_kv_metadata_json(outcome);
    format!(
        "\"used_memory_count\":{}, {},\"runtime_model\":{},\"runtime_token_count\":{},\"runtime_entropy_count\":{},\"runtime_logprob_count\":{},\"runtime_uncertainty_token_count\":{},\"runtime_uncertainty_signal\":{},\"runtime_average_entropy\":{},\"runtime_average_neg_logprob\":{},\"runtime_uncertainty_perplexity\":{},\"runtime_architecture_signal\":{},\"runtime_kv_precision_signal\":{},\"runtime_device_execution_source\":{}, {}",
        outcome.used_memories.len(),
        route_metadata,
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
        runtime_kv_metadata
    )
}

fn model_service_runtime_kv_metadata_json(outcome: &InferenceOutcome) -> String {
    let diagnostics = &outcome.runtime_diagnostics;
    format!(
        "\"runtime_kv_influence\":{},\"runtime_imported_kv_blocks\":{},\"runtime_weak_kv_imports_skipped\":{},\"runtime_budget_limited_kv_imports_skipped\":{},\"runtime_kv_budget_pressure\":{:.6},\"runtime_exported_kv_blocks\":{},\"runtime_kv_segments_included\":{},\"runtime_kv_segments_skipped\":{},\"runtime_kv_segments_rejected\":{},\"runtime_kv_segment_yield\":{}",
        option_f32_service_json(diagnostics.kv_influence),
        diagnostics.imported_kv_blocks,
        diagnostics.weak_runtime_kv_imports_skipped,
        diagnostics.budget_limited_runtime_kv_imports_skipped,
        runtime_kv_budget_pressure(
            diagnostics.exported_kv_blocks,
            diagnostics.budget_limited_runtime_kv_imports_skipped
        ),
        diagnostics.exported_kv_blocks,
        diagnostics.runtime_kv_segments_included,
        diagnostics.runtime_kv_segments_skipped,
        diagnostics.runtime_kv_segments_rejected,
        option_f32_service_json(diagnostics.runtime_kv_segment_yield())
    )
}

fn runtime_kv_budget_pressure(exported_kv_blocks: usize, budget_limited_skipped: usize) -> f32 {
    let total = exported_kv_blocks.saturating_add(budget_limited_skipped);
    if total == 0 {
        return 0.0;
    }

    (budget_limited_skipped as f32 / total as f32).clamp(0.0, 1.0)
}

pub(crate) fn model_service_route_budget_metadata_json(outcome: &InferenceOutcome) -> String {
    format!(
        "\"route_threshold\":{:.6},\"route_attention_tokens\":{},\"route_fast_tokens\":{},\"route_attention_fraction\":{:.6}",
        outcome.route_budget.threshold,
        outcome.route_budget.attention_tokens,
        outcome.route_budget.fast_tokens,
        outcome.route_budget.attention_fraction
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

fn prompt_mentions_rust(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    contains_any(
        &lower,
        &[
            "rust",
            "cargo",
            "crate",
            "borrow",
            "ownership",
            "lifetime",
            "trait",
            "impl",
            "tokio",
            "axum",
            "clippy",
        ],
    ) || contains_any(
        prompt,
        &["所有权", "借用", "生命周期", "结构体", "特征", "编译"],
    )
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

fn is_cjk_unified_ideograph(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff
    )
}
