use rust_norion::{InferenceOutcome, NoironOrchestrationStageStatus, TaskProfile};

use super::super::json::{
    option_f32_service_json, option_str_service_json, option_u64_service_json,
    option_usize_service_json, service_json_string, service_json_string_array, service_u64_array,
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
    requested_max_tokens: Option<usize>,
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
    let runtime_adapter_metadata = model_service_runtime_adapter_metadata_json(outcome);
    let runtime_kv_metadata = model_service_runtime_kv_metadata_json(outcome);
    let runtime_closed_loop_counters = model_service_runtime_closed_loop_counters_json(outcome);
    let dna_closed_loop = model_service_dna_closed_loop_json(outcome);
    format!(
        "{{\"ok\":true,\"request_id\":{},\"profile\":\"{}\",{},{},\"requested_max_tokens\":{},\"elapsed_ms\":{},\"output_mode\":\"{}\",\"answer\":{},\"raw_answer\":{},\"enhanced_answer\":{},\"quality\":{:.6},\"process_reward\":{:.6},\"action\":\"{}\",\"memory_stored\":{},\"stored_memory_id\":{},\"used_memory_count\":{},\"used_memory_ids\":{},\"stored_gist_memory_ids\":{},\"stored_runtime_kv_memory_ids\":{},\"feedback_memory_ids\":{},\"experience_id\":{},\"runtime_model\":{},{},\"runtime_token_count\":{},\"runtime_entropy_count\":{},\"runtime_logprob_count\":{},\"runtime_uncertainty_token_count\":{},\"runtime_uncertainty_signal\":{},\"runtime_average_entropy\":{},\"runtime_average_neg_logprob\":{},\"runtime_uncertainty_perplexity\":{},\"runtime_architecture_signal\":{},\"runtime_kv_precision_signal\":{},\"runtime_device_execution_source\":{}, {},{},{},\"traceable\":{}}}",
        request_id,
        profile_name(profile),
        task_metadata,
        route_metadata,
        option_usize_service_json(requested_max_tokens),
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
        runtime_adapter_metadata,
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
        runtime_closed_loop_counters,
        dna_closed_loop,
        traceable,
    )
}

pub(crate) fn openai_chat_completion_response_json(
    request_id: usize,
    endpoint: &str,
    profile: TaskProfile,
    model_hint: Option<&str>,
    output_mode: ModelServiceOutputMode,
    requested_max_tokens: Option<usize>,
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
        "{{\"id\":\"chatcmpl-norion-{}\",\"object\":\"chat.completion\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"message\":{{\"role\":\"assistant\",\"content\":{}}},\"finish_reason\":\"stop\"}}],\"usage\":{{\"prompt_tokens\":0,\"completion_tokens\":{},\"total_tokens\":{}}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"profile\":\"{}\",{},\"requested_max_tokens\":{},\"cancelled\":false,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"elapsed_ms\":{},\"output_mode\":\"{}\",\"quality\":{:.6},\"experience_id\":{},\"memory_stored\":{}, {},\"persistent_writes\":true,\"memory_write_allowed\":true,\"genome_write_allowed\":true,\"self_evolution_write_allowed\":true}}}}",
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
        option_usize_service_json(requested_max_tokens),
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
    requested_max_tokens: Option<usize>,
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
        "{{\"id\":\"cmpl-norion-{}\",\"object\":\"text_completion\",\"created\":{},\"model\":{},\"choices\":[{{\"index\":0,\"text\":{},\"finish_reason\":\"stop\"}}],\"usage\":{{\"prompt_tokens\":0,\"completion_tokens\":{},\"total_tokens\":{}}},\"norion\":{{\"request_id\":{},\"endpoint\":{},\"model\":{},\"profile\":\"{}\",{},\"requested_max_tokens\":{},\"cancelled\":false,\"timeout\":false,\"retryable\":false,\"runtime_error_note\":null,\"elapsed_ms\":{},\"output_mode\":\"{}\",\"quality\":{:.6},\"experience_id\":{},\"memory_stored\":{}, {},\"persistent_writes\":true,\"memory_write_allowed\":true,\"genome_write_allowed\":true,\"self_evolution_write_allowed\":true}}}}",
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
        option_usize_service_json(requested_max_tokens),
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
    let runtime_adapter_metadata = model_service_runtime_adapter_metadata_json(outcome);
    let runtime_kv_metadata = model_service_runtime_kv_metadata_json(outcome);
    let runtime_closed_loop_counters = model_service_runtime_closed_loop_counters_json(outcome);
    let dna_closed_loop = model_service_dna_closed_loop_json(outcome);
    let used_memory_ids = outcome
        .used_memories
        .iter()
        .map(|memory| memory.id)
        .collect::<Vec<_>>();
    format!(
        "\"used_memory_count\":{},\"used_memory_ids\":{},\"reflection_issue_codes\":{},\"revision_actions\":{},\"stored_runtime_kv_memory_ids\":{}, {},\"runtime_model\":{},{},\"runtime_token_count\":{},\"runtime_entropy_count\":{},\"runtime_logprob_count\":{},\"runtime_uncertainty_token_count\":{},\"runtime_uncertainty_signal\":{},\"runtime_average_entropy\":{},\"runtime_average_neg_logprob\":{},\"runtime_uncertainty_perplexity\":{},\"runtime_architecture_signal\":{},\"runtime_kv_precision_signal\":{},\"runtime_device_execution_source\":{}, {},{},{}",
        outcome.used_memories.len(),
        service_u64_array(&used_memory_ids),
        service_json_string_array(&outcome.report.issue_codes()),
        service_json_string_array(&outcome.report.revision_actions),
        service_u64_array(&outcome.stored_runtime_kv_memory_ids),
        route_metadata,
        option_str_service_json(outcome.runtime_diagnostics.model_id.as_deref()),
        runtime_adapter_metadata,
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
        runtime_closed_loop_counters,
        dna_closed_loop
    )
}

pub(crate) fn model_service_dna_closed_loop_json(outcome: &InferenceOutcome) -> String {
    let receipt = &outcome.dna_apply_receipt;
    format!(
        "\"dna_closed_loop\":{{\"strategy\":{},\"strategy_genome_id\":{},\"strategy_gene_count\":{},\"generation_before\":{},\"generation_after\":{},\"active_genome_id_after\":{},\"reasoning_frame_id\":{},\"reasoning_frame_valid\":{},\"reasoning_frame_vm_executed\":{},\"reasoning_frame_opcode_count\":{},\"task_gene_decision\":{},\"task_skill_decision\":{},\"writer_gate_decision\":{},\"apply_plan_decision\":{},\"mutation_count\":{},\"dual_chain_committed\":{},\"express_chain_records\":{},\"memory_chain_records\":{},\"mutation_applied\":{},\"rollback_applied\":{},\"receipt_reason\":{}}}",
        service_json_string(outcome.genome_strategy.as_str()),
        service_json_string(&outcome.strategy_genome.genome_id),
        outcome.strategy_genome.active_gene_count(),
        receipt.generation_before,
        receipt.generation_after,
        service_json_string(&receipt.genome_id_after),
        service_json_string(&outcome.reasoning_frame.frame_id),
        outcome.reasoning_frame_valid,
        outcome.reasoning_frame.executed_opcodes == outcome.reasoning_frame.genome_isa.opcodes,
        outcome.reasoning_frame.executed_opcodes.len(),
        service_json_string(outcome.task_gene_review.decision.as_str()),
        service_json_string(outcome.task_skill_gene.decision.as_str()),
        service_json_string(outcome.dna_writer_gate.decision.as_str()),
        service_json_string(outcome.dna_apply_plan.decision.as_str()),
        receipt.mutation_count,
        receipt.dual_chain_committed,
        receipt.express_chain_records,
        receipt.memory_chain_records,
        receipt.applied,
        receipt.rolled_back,
        service_json_string(&receipt.reason)
    )
}

fn model_service_runtime_adapter_metadata_json(outcome: &InferenceOutcome) -> String {
    let diagnostics = &outcome.runtime_diagnostics;
    format!(
        "\"runtime_adapter\":{},\"runtime_device\":{},\"runtime_primary_lane\":{},\"runtime_fallback_lane\":{},\"runtime_memory_mode\":{},\"runtime_forward_energy\":{},\"runtime_hot_kv_precision_bits\":{},\"runtime_cold_kv_precision_bits\":{}",
        option_str_service_json(diagnostics.selected_adapter.as_deref()),
        option_str_service_json(diagnostics.device_profile.as_deref()),
        option_str_service_json(diagnostics.primary_lane.as_deref()),
        option_str_service_json(diagnostics.fallback_lane.as_deref()),
        option_str_service_json(diagnostics.memory_mode.as_deref()),
        option_f32_service_json(diagnostics.forward_energy),
        option_usize_service_json(diagnostics.hot_kv_precision_bits.map(usize::from)),
        option_usize_service_json(diagnostics.cold_kv_precision_bits.map(usize::from))
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
        diagnostics.runtime_kv_budget_pressure(),
        diagnostics.exported_kv_blocks,
        diagnostics.runtime_kv_segments_included,
        diagnostics.runtime_kv_segments_skipped,
        diagnostics.runtime_kv_segments_rejected,
        option_f32_service_json(diagnostics.runtime_kv_segment_yield())
    )
}

pub(crate) fn model_service_runtime_closed_loop_counters_json(
    outcome: &InferenceOutcome,
) -> String {
    let budget = &outcome.compute_budget_schedule;
    let admission = &outcome.memory_admission;
    let fusion = &admission.fusion_plan;
    let orchestration = outcome.orchestration_trace();
    let control_expression = &orchestration.control_expression;
    format!(
        "\"runtime_closed_loop_counters\":{{\"adaptive_routing_candidates\":{},\"adaptive_routing_saved_tokens\":{},\"adaptive_routing_threshold_delta_milli\":{},\"task_hierarchy_mutation_records\":{},\"task_hierarchy_compute_reduction_milli\":{},\"task_hierarchy_weight_delta_milli\":{},\"compute_budget_selected_candidates\":{},\"compute_budget_kv_lookups_skipped\":{},\"compute_budget_saved_tokens\":{},\"compute_budget_avoided_tokens\":{},\"compute_budget_write_allowed\":{},\"compute_budget_applied\":{},\"memory_admission_candidates\":{},\"memory_admission_ready\":{},\"memory_admission_blocked\":{},\"memory_admission_ledger_records\":{},\"memory_admission_ledger_preview_only\":{},\"memory_admission_ledger_authorized\":{},\"memory_admission_ledger_applied\":{},\"memory_admission_write_allowed\":{},\"memory_admission_applied\":{},\"kv_fusion_candidates\":{},\"kv_fusion_fused\":{},\"kv_fusion_compressed\":{},\"kv_fusion_skipped\":{},\"kv_fusion_held\":{},\"kv_fusion_rejected\":{},\"kv_fusion_approval_blocked\":{},\"kv_fusion_input_tokens\":{},\"kv_fusion_retained_tokens\":{},\"kv_fusion_saved_tokens\":{},\"kv_fusion_write_allowed\":{},\"kv_fusion_applied\":{},\"self_evolving_memory_store_updates\":{},\"self_evolving_memory_store_primary_applied\":{},\"self_evolving_memory_store_gist_applied\":{},\"self_evolving_memory_store_runtime_kv_applied\":{},\"memory_residency_retention_decayed\":{},\"memory_residency_retention_removed\":{},\"memory_residency_compaction_merged\":{},\"memory_residency_compaction_removed\":{},\"reflection_issues\":{},\"reflection_critical_issues\":{},\"reflection_revision_actions\":{},\"online_reward_feedbacks\":{},\"online_reward_reinforcements\":{},\"online_reward_penalties\":{},\"online_reward_strength_milli\":{},\"online_reward_reinforcement_strength_milli\":{},\"online_reward_penalty_strength_milli\":{},\"memory_feedback_updates\":{},\"memory_feedback_reinforcements\":{},\"memory_feedback_penalties\":{},\"noiron_orchestration_stages\":{},\"noiron_orchestration_completed_stages\":{},\"noiron_orchestration_failed_stages\":{},\"noiron_orchestration_preview_only_stages\":{},\"noiron_orchestration_gated_stages\":{},\"noiron_orchestration_rolled_back_stages\":{},\"noiron_orchestration_rollback_records\":{},\"noiron_orchestration_writes_gated\":{},\"noiron_orchestration_live_feedback_closed\":{},\"noiron_orchestration_durable_memory_ledger_authorized\":{},\"noiron_orchestration_durable_memory_ledger_applied\":{},\"control_expression_profile_selected\":{},\"control_expression_context_anchor_promoted\":{},\"control_expression_suppression_gate_triggered\":{},\"control_expression_checkpoint_repair_requested\":{},\"control_expression_checkpoint_rejected\":{},\"control_expression_memory_refresh_candidate\":{},\"control_expression_memory_tombstone_candidate\":{},\"control_expression_preview_admission\":{},\"control_expression_write_allowed\":{},\"control_expression_applied\":{},\"control_expression_operator_approval_required\":{},\"control_expression_ready\":{}}}",
        outcome.adaptive_route_plan.candidates,
        outcome.adaptive_route_plan.saved_tokens,
        nonnegative_milli(outcome.live_evolution.router_threshold_delta),
        outcome.task_hierarchy_plan.mutation_count(),
        nonnegative_milli(outcome.task_hierarchy_plan.compute_reduction),
        nonnegative_milli(outcome.live_evolution.hierarchy_weight_delta),
        budget.selected_candidates,
        budget.kv_lookups_skipped,
        budget.saved_tokens,
        budget.wasted_compute_avoided_tokens,
        budget.write_allowed,
        budget.applied,
        admission.candidate_count(),
        admission.ready_count(),
        admission.blocked_count(),
        admission.ledger_record_count(),
        admission.ledger_preview_only_count(),
        admission.ledger_authorized_count(),
        admission.ledger_applied_count(),
        admission.write_allowed,
        admission.applied,
        fusion.candidates,
        fusion.fused,
        fusion.compressed,
        fusion.skipped,
        fusion.held,
        fusion.rejected,
        fusion.approval_blocked,
        fusion.input_tokens,
        fusion.retained_tokens,
        fusion.saved_tokens,
        fusion.write_allowed,
        fusion.applied,
        outcome.live_evolution.stored_memory_updates(),
        outcome.live_evolution.stored_memory,
        outcome.live_evolution.stored_gist_memories,
        outcome.live_evolution.stored_runtime_kv_memories,
        outcome.retention_report.decayed,
        outcome.retention_report.removed.len(),
        outcome.memory_compaction_report.merged.len(),
        outcome.memory_compaction_report.removed.len(),
        outcome.live_evolution.reflection_issues,
        outcome.live_evolution.critical_reflection_issues,
        outcome.live_evolution.revision_actions,
        outcome.live_evolution.online_reward_feedbacks,
        outcome.live_evolution.online_reward_reinforcements,
        outcome.live_evolution.online_reward_penalties,
        nonnegative_milli(outcome.live_evolution.online_reward_strength),
        nonnegative_milli(outcome.live_evolution.online_reward_reinforcement_strength),
        nonnegative_milli(outcome.live_evolution.online_reward_penalty_strength),
        outcome.live_evolution.memory_updates(),
        outcome.live_evolution.memory_reinforcements,
        outcome.live_evolution.memory_penalties,
        orchestration.stages.len(),
        orchestration_stage_status_count(&orchestration, NoironOrchestrationStageStatus::Completed),
        orchestration.failed_stages().len(),
        orchestration_stage_status_count(
            &orchestration,
            NoironOrchestrationStageStatus::PreviewOnly
        ),
        orchestration_stage_status_count(&orchestration, NoironOrchestrationStageStatus::Gated),
        orchestration_stage_status_count(
            &orchestration,
            NoironOrchestrationStageStatus::RolledBack
        ),
        orchestration_rollback_record_count(&orchestration),
        orchestration.all_writes_gated(),
        orchestration.live_feedback_closed(),
        orchestration.gates.durable_memory_ledger_authorized,
        orchestration.gates.durable_memory_ledger_applied,
        control_expression.control_expression_profile_selected,
        control_expression.context_anchor_promoted,
        control_expression.suppression_gate_triggered,
        control_expression.checkpoint_repair_requested,
        control_expression.checkpoint_rejected,
        control_expression.memory_refresh_candidate,
        control_expression.memory_tombstone_candidate,
        control_expression.control_expression_preview_admission,
        control_expression.write_allowed,
        control_expression.applied,
        control_expression.operator_approval_required,
        control_expression.ready()
    )
}

fn orchestration_stage_status_count(
    trace: &rust_norion::NoironOrchestrationTrace,
    status: NoironOrchestrationStageStatus,
) -> usize {
    trace
        .stages
        .iter()
        .filter(|stage| stage.status == status)
        .count()
}

fn orchestration_rollback_record_count(trace: &rust_norion::NoironOrchestrationTrace) -> usize {
    trace.rollback_records.len()
        + trace
            .stages
            .iter()
            .map(|stage| stage.rollback_records.len())
            .sum::<usize>()
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

fn nonnegative_milli(value: f32) -> usize {
    (value.max(0.0) * 1000.0).round() as usize
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::{HeuristicBackend, InferenceRequest, NoironEngine};

    fn timed_for(prompt: &str, profile: TaskProfile) -> TimedOutcome {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        TimedOutcome {
            outcome: engine.infer(InferenceRequest::new(prompt, profile), &mut backend),
            elapsed_ms: 3,
        }
    }

    #[test]
    fn issue5_service_response_proves_prompt_intent_max_tokens_and_diagnostics() {
        let cases = [
            (
                "Explain the Noiron runtime",
                TaskProfile::General,
                "\"language_mode\":\"english\"",
                "\"coding_language\":\"none\"",
                "\"rust_coding\":false",
            ),
            (
                "请解释 Noiron 运行时",
                TaskProfile::General,
                "\"language_mode\":\"chinese\"",
                "\"coding_language\":\"none\"",
                "\"rust_coding\":false",
            ),
            (
                "Write a Rust function using ownership safely",
                TaskProfile::Coding,
                "\"language_mode\":\"english\"",
                "\"coding_language\":\"rust\"",
                "\"rust_coding\":true",
            ),
        ];

        for (index, (prompt, profile, language, coding_language, rust_coding)) in
            cases.into_iter().enumerate()
        {
            let timed = timed_for(prompt, profile);
            let intent = model_service_task_intent_metadata(prompt, profile);
            let body = model_service_response_json(
                index + 1,
                profile,
                false,
                ModelServiceOutputMode::Enhanced,
                Some(256),
                intent,
                &timed,
            );

            assert!(body.contains(language), "{body}");
            assert!(body.contains(coding_language), "{body}");
            assert!(body.contains(rust_coding), "{body}");
            assert!(body.contains("\"requested_max_tokens\":256"), "{body}");
            assert!(body.contains("\"runtime_model\":"), "{body}");
            assert!(
                body.contains("\"runtime_closed_loop_counters\":{"),
                "{body}"
            );
            assert!(body.contains("\"dna_closed_loop\":{"), "{body}");
            assert!(body.contains("\"generation_before\":0"), "{body}");
            assert!(
                body.contains("\"receipt_reason\":\"explicit_authorization_missing\""),
                "{body}"
            );
            assert!(body.contains("\"runtime_architecture_signal\":"), "{body}");
        }
    }

    #[test]
    fn issue5_openai_response_proves_max_tokens_and_norion_diagnostics() {
        let prompt = "Fix this Rust lifetime error";
        let profile = TaskProfile::Coding;
        let timed = timed_for(prompt, profile);
        let intent = model_service_task_intent_metadata(prompt, profile);

        let body = openai_chat_completion_response_json(
            9,
            "chat-completions",
            profile,
            Some("rust-norion-local"),
            ModelServiceOutputMode::Enhanced,
            Some(128),
            intent,
            &timed,
        );

        assert!(body.contains("\"norion\":{"), "{body}");
        assert!(body.contains("\"requested_max_tokens\":128"), "{body}");
        assert!(body.contains("\"language_mode\":\"english\""), "{body}");
        assert!(body.contains("\"coding_language\":\"rust\""), "{body}");
        assert!(
            body.contains("\"runtime_closed_loop_counters\":{"),
            "{body}"
        );
        assert!(body.contains("\"dna_closed_loop\":{"), "{body}");
        assert!(body.contains("\"runtime_token_count\":"), "{body}");
        assert!(body.contains("\"used_memory_ids\":["), "{body}");
        assert!(body.contains("\"reflection_issue_codes\":["), "{body}");
        assert!(body.contains("\"revision_actions\":["), "{body}");

        let completion = openai_completion_response_json(
            10,
            "completions",
            profile,
            Some("rust-norion-local"),
            ModelServiceOutputMode::Enhanced,
            Some(96),
            intent,
            &timed,
        );
        assert!(
            completion.contains("\"requested_max_tokens\":96"),
            "{completion}"
        );
    }
}
