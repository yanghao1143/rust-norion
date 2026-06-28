use rust_norion::{InferenceOutcome, SelfEvolvingMemoryRuntimeWritebackReport, TaskProfile};

use super::super::json::{
    option_f32_service_json, option_str_service_json, option_u64_service_json, service_json_string,
    service_u64_array,
};
use super::super::request::ModelServiceOutputMode;
use super::super::types::{profile_name, TimedOutcome};

pub(crate) fn model_service_response_json(
    request_id: usize,
    profile: TaskProfile,
    traceable: bool,
    output_mode: ModelServiceOutputMode,
    timed: &TimedOutcome,
    self_evolving_memory_writeback: Option<&SelfEvolvingMemoryRuntimeWritebackReport>,
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
        "{{\"ok\":true,\"request_id\":{},\"profile\":\"{}\",\"elapsed_ms\":{},\"output_mode\":\"{}\",\"answer\":{},\"raw_answer\":{},\"enhanced_answer\":{},\"quality\":{:.6},\"process_reward\":{:.6},\"action\":\"{}\",\"memory_stored\":{},\"stored_memory_id\":{},\"used_memory_ids\":{},\"stored_gist_memory_ids\":{},\"stored_runtime_kv_memory_ids\":{},\"feedback_memory_ids\":{},\"experience_id\":{},\"runtime_model\":{},\"runtime_token_count\":{},\"runtime_entropy_count\":{},\"runtime_logprob_count\":{},\"runtime_uncertainty_token_count\":{},\"runtime_uncertainty_signal\":{},\"runtime_average_entropy\":{},\"runtime_average_neg_logprob\":{},\"runtime_uncertainty_perplexity\":{},\"runtime_architecture_signal\":{},\"runtime_kv_precision_signal\":{},\"runtime_device_execution_source\":{},\"self_evolving_memory_writeback\":{},\"traceable\":{}}}",
        request_id,
        profile_name(profile),
        timed.elapsed_ms,
        output_mode.as_str(),
        service_json_string(answer),
        option_str_service_json(None),
        option_str_service_json(None),
        outcome.report.quality,
        outcome.process_reward.total,
        outcome.process_reward.action.as_str(),
        outcome.stored_memory_id.is_some(),
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
        option_self_evolving_memory_writeback_json(self_evolving_memory_writeback),
        traceable,
    )
}

fn option_self_evolving_memory_writeback_json(
    report: Option<&SelfEvolvingMemoryRuntimeWritebackReport>,
) -> String {
    report
        .map(self_evolving_memory_writeback_json)
        .unwrap_or_else(|| "null".to_owned())
}

fn self_evolving_memory_writeback_json(
    report: &SelfEvolvingMemoryRuntimeWritebackReport,
) -> String {
    format!(
        "{{\"schema\":\"rust-norion-self-evolving-memory-writeback-v1\",\"operation\":{},\"tool\":{},\"profile\":\"{}\",\"experience_id\":{},\"source_case_digest\":{},\"attempted_records\":{},\"accepted_records\":{},\"rejected_records\":{},\"records_before\":{},\"records_after\":{},\"episodes_after\":{},\"active_episodes_after\":{},\"heuristics_after\":{},\"tool_reliability_after\":{},\"tool_observations_after\":{},\"maintenance_actions\":{},\"merged_duplicate_episodes\":{},\"redacted\":{},\"write_allowed\":{},\"durable_write_allowed\":{},\"applied\":{},\"applied_to_disk\":{},\"snapshot_changes\":{},\"snapshot_before_digest\":{},\"snapshot_digest\":{},\"disk_snapshot_digest\":{}}}",
        service_json_string(report.operation),
        service_json_string(&report.tool_name),
        profile_name(report.profile),
        report.experience_id,
        service_json_string(&report.source_case_digest),
        report.attempted_records,
        report.accepted_records,
        report.attempted_records.saturating_sub(report.accepted_records),
        report.records_before,
        report.records_after,
        report.episodes_after,
        report.active_episodes_after,
        report.heuristics_after,
        report.tool_reliability_after,
        report.tool_observations_after,
        report.maintenance_actions,
        report.merged_duplicate_episodes,
        report.redacted,
        report.write_allowed,
        report.durable_write_allowed,
        report.applied,
        report.applied_to_disk,
        report.snapshot_before_digest != report.snapshot_digest,
        service_json_string(&report.snapshot_before_digest),
        service_json_string(&report.snapshot_digest),
        service_json_string(&report.disk_snapshot_digest)
    )
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
