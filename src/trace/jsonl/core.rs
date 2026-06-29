mod auto_replay;

use std::collections::BTreeSet;

use crate::engine::InferenceOutcome;
use crate::hardware::RuntimeAdapterHint;
use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::stable_redaction_digest;
use crate::reasoning_genome::DnaGeneChain;

use super::super::fields::json_escape;
use super::json::{
    option_bool_json, option_f32_json, option_owned_string_json, option_string_json,
    option_u8_json, option_u64_json, string_array_json,
};
use super::summary::memory_feedback_summaries;
use auto_replay::AutoReplayTraceFields;

pub fn trace_json_line(
    prompt: &str,
    profile: TaskProfile,
    elapsed_ms: u128,
    outcome: &InferenceOutcome,
) -> String {
    trace_json_line_with_case(None, prompt, profile, elapsed_ms, outcome)
}

pub fn trace_json_line_with_case(
    case_name: Option<&str>,
    prompt: &str,
    profile: TaskProfile,
    elapsed_ms: u128,
    outcome: &InferenceOutcome,
) -> String {
    let tier_counts = outcome.tier_plan.counts();
    let infini_counts = outcome.infini_memory_plan.counts();
    let transformer_counts = outcome.transformer_plan.counts();
    let adapter_hints = outcome
        .hardware_plan
        .execution
        .adapter_hints
        .iter()
        .map(|adapter| adapter.as_str().to_owned())
        .collect::<Vec<_>>();
    let runtime_budget = runtime_budget_json(&outcome.hardware_plan);
    let reflection_issue_codes = outcome.report.issue_codes();
    let auto_replay = AutoReplayTraceFields::from(outcome.auto_replay_report.as_ref());
    let best_adapter_observation = outcome.runtime_adapter_observations.first();
    let runtime_selected_adapter = outcome
        .runtime_diagnostics
        .selected_adapter
        .as_deref()
        .and_then(RuntimeAdapterHint::canonical_name);
    let runtime_adapter_selection_mismatch = match (
        best_adapter_observation.map(|observation| observation.adapter.as_str()),
        runtime_selected_adapter,
    ) {
        (Some(best_adapter), Some(selected_adapter)) => best_adapter != selected_adapter,
        _ => false,
    };
    let toolsmith_blueprints = outcome
        .toolsmith_plan
        .blueprints
        .iter()
        .map(|blueprint| blueprint.summary())
        .collect::<Vec<_>>();
    let agent_team_messages = outcome.agent_team_plan.message_summaries(16);
    let agent_team_conflicts = outcome.agent_team_plan.conflict_summaries(8);
    let agent_team_evolution = outcome.agent_team_plan.evolution_summaries(8);
    let agent_team_aggregation = &outcome.agent_team_plan.aggregation;
    let reasoning_genome_mutation_intents = outcome.reasoning_genome.mutation_intents();
    let reasoning_genome_proposal_ids = outcome.reasoning_genome.proposal_ids();
    let reasoning_genome_lifecycle_actions = outcome.reasoning_genome.lifecycle_action_summaries();
    let reasoning_genome_lifecycle_summaries = outcome.reasoning_genome.lifecycle_summaries(8);
    let reasoning_genome_lineage_scope_digests =
        reasoning_genome_lineage_scope_digests(&outcome.reasoning_genome_chain);
    let reasoning_genome_mixed_lineage = reasoning_genome_lineage_scope_digests.len() > 1;
    let reasoning_genome_splice_finding_kinds = outcome.reasoning_genome_splice.finding_kinds();
    let reasoning_genome_splice_mutation_intents =
        outcome.reasoning_genome_splice.mutation_intents();
    let reasoning_genome_splice_proposal_ids = outcome.reasoning_genome_splice.proposal_ids();
    let reasoning_genome_splice_dispositions =
        outcome.reasoning_genome_splice.disposition_summaries();
    let reasoning_genome_splice_reason_summaries =
        outcome.reasoning_genome_splice.segment_reason_summaries(8);
    let reasoning_genome_splice_lifecycle_states =
        outcome.reasoning_genome_splice.lifecycle_state_summaries();
    let reasoning_genome_splice_lifecycle_summaries =
        outcome.reasoning_genome_splice.lifecycle_summaries(8);
    let runtime_kv_stored = outcome.stored_runtime_kv_memory_ids.len();
    let runtime_kv_held = outcome
        .exported_runtime_kv_blocks
        .saturating_sub(runtime_kv_stored);
    let runtime_kv_hold = runtime_kv_held > 0;
    let memory_admission_kinds = outcome.memory_admission.kind_summaries();
    let memory_admission_decisions = outcome.memory_admission.decision_summaries();
    let memory_admission_candidates = outcome.memory_admission.candidate_summaries();
    let memory_admission_review_packets = outcome.memory_admission.review_packet_summaries();
    let memory_admission_ledger_summaries = outcome.memory_admission.ledger_summaries();
    let kv_fusion_score_summaries = outcome.memory_admission.fusion_score_summaries(12);
    let trace_segment_replay_prior_summaries = outcome
        .memory_admission
        .trace_segment_replay_prior_summaries();
    let adaptive_route_actions = outcome.adaptive_route_plan.action_summaries();
    let adaptive_route_selected_routes = outcome.adaptive_route_plan.selected_route_summaries();
    let adaptive_route_score_summaries = outcome.adaptive_route_plan.score_summaries(12);
    let compute_budget_notes = outcome.compute_budget_schedule.notes.clone();
    let task_hierarchy_mutation_summaries = outcome.task_hierarchy_plan.mutation_summaries(8);
    let runtime_kv_segment_lifecycle_summaries =
        runtime_kv_segment_lifecycle_summaries(prompt, &outcome.runtime_diagnostics);
    let prompt_digest = stable_redaction_digest(["trace_prompt", prompt]);
    let agent_team_goal_digest = stable_redaction_digest([
        "agent_team_main_thread_goal",
        &outcome.agent_team_plan.main_thread_goal,
    ]);

    format!(
        "{{\
         \"schema\":\"rust-norion-trace-v1\",\
         \"case\":{},\
         \"profile\":\"{:?}\",\
         \"prompt_chars\":{},\
         \"prompt_preview\":\"{}\",\
         \"elapsed_ms\":{},\
         \"quality\":{:.6},\
         \"perplexity\":{:.6},\
         \"reflection\":{{\"issues\":{},\"critical_issues\":{},\"max_severity\":\"{}\",\"issue_codes\":{},\"revision_actions\":{},\"revision_passes\":{}}},\
         \"router_threshold_after\":{:.6},\
         \"route\":{{\"threshold\":{:.6},\"attention_fraction\":{:.6},\"attention_tokens\":{},\"fast_tokens\":{}}},\
         \"adaptive_routing\":{{\"candidates\":{},\"include\":{},\"compress\":{},\"defer\":{},\"skip\":{},\"input_tokens\":{},\"retained_tokens\":{},\"saved_tokens\":{},\"min_score\":{:.6},\"max_score\":{:.6},\"average_score\":{:.6},\"actions\":{},\"selected_routes\":{},\"score_summaries\":{},\"read_only\":{},\"write_allowed\":{},\"applied\":{}}},\
         \"compute_budget\":{{\"budget\":\"{}\",\"base_threshold\":{:.6},\"threshold_after\":{:.6},\"threshold_delta\":{:.6},\"route_fanout_before\":{},\"route_fanout_after\":{},\"candidate_count\":{},\"selected_candidates\":{},\"anchor_count\":{},\"anchors_preserved\":{},\"anchors_preserved_count\":{},\"low_value_skipped\":{},\"kv_lookup_budget\":{},\"kv_lookups_planned\":{},\"kv_lookups_skipped\":{},\"reflection_pass_budget\":{},\"validation_run_budget\":{},\"validation_cost_tokens\":{},\"input_tokens\":{},\"retained_tokens\":{},\"saved_tokens\":{},\"estimated_budget_tokens\":{},\"estimated_spent_tokens\":{},\"wasted_compute_avoided_tokens\":{},\"fallback_triggered\":{},\"notes\":{},\"read_only\":{},\"write_allowed\":{},\"applied\":{}}},\
         \"task_hierarchy\":{{\"mode\":\"{}\",\"language\":\"{}\",\"coding_intent\":{},\"validation_mode\":{},\"memory_need\":{:.6},\"compute_budget\":\"{}\",\"hierarchy_depth\":{},\"route_fanout\":{},\"route_pressure\":{:.6},\"compute_reduction\":{:.6},\"threshold_before\":{:.6},\"threshold_after\":{:.6},\"threshold_delta\":{:.6},\"hierarchy_before\":\"{}\",\"hierarchy_after\":\"{}\",\"selected_lanes\":{},\"skipped_lanes\":{},\"memory_lanes\":{},\"skipped_memory_lanes\":{},\"mutation_records\":{},\"mutation_summaries\":{},\"rollback_anchor_id\":\"{}\",\"replayable\":{},\"reverted\":{},\"runtime_applied\":{},\"state_write_allowed\":{},\"adaptive_state_write_allowed\":{},\"ndkv_write_allowed\":{}}},\
         \"runtime_tokens\":{{\"token_count\":{},\"entropy_count\":{},\"logprob_count\":{},\"average_entropy\":{},\"average_neg_logprob\":{},\"uncertainty_perplexity\":{},\"has_uncertainty_signal\":{}}},\
         \"embedding\":{{\"query_source\":\"{}\",\"query_dimensions\":{},\"memory_write_source\":{},\"memory_write_dimensions\":{},\"gist_writes\":{},\"gist_write_runtime_calls\":{},\"gist_write_fallback_calls\":{},\"runtime_embedding_calls\":{},\"fallback_embedding_calls\":{},\"runtime_embedding_available\":{},\"fallback_used\":{}}},\
         \"runtime_diagnostics\":{{\"model_id\":{},\"selected_adapter\":{},\"adapter_cache_mode\":{},\"adapter_stream_trace_id\":{},\"adapter_stream_gate_summary_digest\":{},\"adapter_stream_read_only\":{},\"adapter_stream_write_allowed\":{},\"adapter_stream_applied\":{},\"device_profile\":{},\"primary_lane\":{},\"fallback_lane\":{},\"memory_mode\":{},\"device_execution_source\":{},\"hot_kv_precision_bits\":{},\"cold_kv_precision_bits\":{},\"layer_count\":{},\"global_layers\":{},\"local_window_layers\":{},\"convolutional_fusion_layers\":{},\"hidden_size\":{},\"local_window_tokens\":{},\"forward_energy\":{},\"kv_influence\":{},\"imported_kv_blocks\":{},\"exported_kv_blocks\":{},\"weak_runtime_kv_imports_skipped\":{},\"budget_limited_runtime_kv_imports_skipped\":{},\"runtime_kv_segments_included\":{},\"runtime_kv_segments_skipped\":{},\"runtime_kv_segments_rejected\":{},\"runtime_kv_segment_count\":{},\"runtime_kv_segment_yield\":{},\"runtime_kv_segment_lifecycle_records\":{},\"runtime_kv_segment_lifecycle_summaries\":{},\"has_runtime_kv_activity_signal\":{},\"has_runtime_kv_segment_signal\":{},\"has_runtime_architecture_signal\":{},\"has_forward_signal\":{},\"has_all_layer_modes\":{},\"has_kv_precision_signal\":{}}},\
         \"runtime_adapter_observations\":{{\"observation_count\":{},\"best_adapter\":{},\"selection_mismatch\":{},\"best_score\":{},\"best_reward\":{},\"best_quality\":{},\"best_forward_energy\":{},\"best_kv_influence\":{},\"best_experience_id\":{}}},\
         \"hierarchy\":{{\"global\":{:.6},\"local\":{:.6},\"convolution\":{:.6}}},\
         \"hardware\":{{\"device\":\"{}\",\"tier\":\"{}\",\"pressure\":{:.6},\"runtime_device_contract\":\"{}\",\"latency_budget_ms\":{},\"local_kv_token_budget\":{},\"global_kv_token_budget\":{},\"runtime_budget\":{},\"execution\":{{\"primary_lane\":\"{}\",\"fallback_lane\":\"{}\",\"memory_mode\":\"{}\",\"max_parallel_chunks\":{},\"kv_prefetch_blocks\":{},\"hot_kv_bits\":{},\"cold_kv_bits\":{},\"disk_spill\":{},\"adapter_hints\":{}}}}},\
         \"recursive\":{{\"required\":{},\"prompt_tokens\":{},\"native_window\":{},\"chunks\":{},\"merge_rounds\":{},\"execution_waves\":{},\"max_parallel_chunks\":{},\"chunk_tokens\":{},\"overlap_tokens\":{},\"runtime_calls\":{}}},\
         \"tiers\":{{\"hot_gpu\":{},\"warm_ram\":{},\"cold_disk\":{}}},\
         \"infini_memory\":{{\"local_window\":{},\"global_memory\":{},\"sparse_skipped\":{},\"local_tokens\":{},\"global_tokens\":{},\"skipped_tokens\":{}}},\
         \"transformer\":{{\"template\":\"{}\",\"global\":{},\"local\":{},\"convolution\":{}}},\
         \"toolsmith\":{{\"rust_only\":{},\"exploration_required\":{},\"blueprints\":{},\"ready\":{},\"held\":{},\"rejected\":{},\"gate_passed\":{},\"notes\":{},\"rejected_requests\":{},\"blueprint_summaries\":{}}},\
         \"agent_team\":{{\"enabled\":{},\"summary\":\"{}\",\"run_id\":\"{}\",\"main_thread_goal\":\"{}\",\"agents\":{},\"messages\":{},\"conflicts\":{},\"unresolved_conflicts\":{},\"evolution_signals\":{},\"collision_free\":{},\"isolation\":{{\"single_writer\":{},\"read_only_subagents\":{},\"namespace\":\"{}\",\"allowed_outputs\":{},\"denied_capabilities\":{}}},\"aggregation\":{{\"lane_count\":{},\"message_summaries\":{},\"conflict_topics\":{},\"unresolved_conflict_topics\":{},\"budget_scope\":\"{}\",\"max_parallel_lanes\":{},\"attention_fraction\":{:.6},\"main_thread_writer\":\"{}\"}},\"message_summaries\":{},\"conflict_summaries\":{},\"evolution_summaries\":{}}},\
         \"reasoning_genome\":{{\"genome_id\":\"{}\",\"stable_anchor_id\":\"{}\",\"chain_records\":{},\"lineage_scope_digests\":{},\"mixed_lineage\":{},\"gene_count\":{},\"active_genes\":{},\"aged_genes\":{},\"malignant_genes\":{},\"relabel_candidates\":{},\"regeneration_candidates\":{},\"gene_scissors_proposals\":{},\"repair_payloads\":{},\"regeneration_payloads\":{},\"mutation_intents\":{},\"proposal_ids\":{},\"read_only\":{},\"write_allowed\":{},\"mutation_applied\":{},\"youth_pressure\":{:.6},\"lifecycle_records\":{},\"lifecycle_actions\":{},\"lifecycle_summaries\":{},\"lifecycle_tombstone_candidates\":{},\"lifecycle_pending_validations\":{},\"lifecycle_source_evidence\":{},\"splice_segments\":{},\"splice_exons\":{},\"splice_introns\":{},\"splice_variants\":{},\"splice_retained\":{},\"splice_skipped\":{},\"splice_quarantined\":{},\"splice_repair_candidates\":{},\"splice_dispositions\":{},\"splice_reason_summaries\":{},\"splice_lifecycle_records\":{},\"splice_lifecycle_states\":{},\"splice_lifecycle_summaries\":{},\"splice_findings\":{},\"splice_finding_kinds\":{},\"splice_mutation_intents\":{},\"splice_proposals\":{},\"splice_proposal_ids\":{},\"splice_read_only\":{},\"splice_write_allowed\":{},\"splice_applied\":{}}},\
         \"stream_windows\":{},\
         \"memory\":{{\"used\":{},\"stored\":{},\"gist_records\":{},\"gist_stored\":{},\"runtime_kv_exported\":{},\"runtime_kv_stored\":{},\"runtime_kv_hold\":{},\"runtime_kv_held\":{},\"feedback_reinforced\":{},\"feedback_penalized\":{},\"feedback_reinforcement_amount\":{:.6},\"feedback_penalty_amount\":{:.6},\"feedback_updates\":{},\"feedback_applied\":{},\"feedback_removed\":{},\"feedback_missing\":{},\"feedback_strength_delta\":{:.6},\"feedback_update_summaries\":{}}},\
         \"drift\":{{\"severity\":\"{}\",\"memory_write\":{},\"runtime_kv_write\":{},\"penalize_used_memory\":{},\"rollback_adaptive\":{},\"notes\":{}}},\
         \"process_reward\":{{\"total\":{:.6},\"action\":\"{}\",\"route\":{:.6},\"memory\":{:.6},\"hierarchy\":{:.6},\"reflection\":{:.6},\"latency\":{:.6},\"admission\":{:.6},\"notes\":{}}},\
         \"auto_replay\":{{\"applied\":{},\"router_updates\":{},\"hierarchy_updates\":{},\"router_threshold_mutations\":{},\"hierarchy_weight_mutations\":{},\"router_threshold_delta\":{:.6},\"hierarchy_weight_delta\":{:.6},\"reinforced\":{},\"penalized\":{},\"touched_memories\":{},\"memory_reinforcements\":{},\"memory_penalties\":{},\"live_memory_feedback_items\":{},\"live_memory_feedback_updates\":{},\"live_memory_feedback_reinforcements\":{},\"live_memory_feedback_penalties\":{},\"live_memory_feedback_detail_items\":{},\"live_memory_feedback_applied\":{},\"live_memory_feedback_removed\":{},\"live_memory_feedback_missing\":{},\"live_memory_feedback_strength_delta\":{:.6},\"business_contract_items\":{},\"business_contract_passed\":{},\"business_contract_failed\":{},\"business_contract_raw_passed\":{},\"business_contract_raw_failed\":{},\"business_contract_response_normalized\":{},\"business_contract_sanitized\":{},\"business_contract_canonical_fallbacks\":{},\"live_evolution_items\":{},\"live_evolution_router_threshold_mutations\":{},\"live_evolution_hierarchy_weight_mutations\":{},\"live_evolution_router_threshold_delta\":{:.6},\"live_evolution_hierarchy_weight_delta\":{:.6},\"live_evolution_online_reward_feedbacks\":{},\"live_evolution_online_reward_reinforcements\":{},\"live_evolution_online_reward_penalties\":{},\"live_evolution_online_reward_strength\":{:.6},\"live_evolution_online_reward_reinforcement_strength\":{:.6},\"live_evolution_online_reward_penalty_strength\":{:.6},\"live_evolution_memory_updates\":{},\"live_evolution_stored_memory_updates\":{},\"live_evolution_reflection_issues\":{},\"live_evolution_critical_reflection_issues\":{},\"live_evolution_revision_actions\":{},\"runtime_kv_budget_pressure_items\":{},\"avg_runtime_kv_budget_pressure\":{:.6},\"max_runtime_kv_budget_pressure\":{:.6},\"runtime_kv_weak_import_pressure_items\":{},\"avg_runtime_kv_weak_import_pressure\":{:.6},\"max_runtime_kv_weak_import_pressure\":{:.6},\"recursive_runtime_items\":{},\"recursive_runtime_calls\":{},\"avg_recursive_call_pressure\":{:.6},\"max_recursive_call_pressure\":{:.6}}},\
         \"memory_admission\":{{\"candidates\":{},\"ready\":{},\"blocked\":{},\"admitted\":{},\"hold\":{},\"reject\":{},\"quarantine\":{},\"kinds\":{},\"decisions\":{},\"candidate_summaries\":{},\"review_packets\":{},\"review_packet_summaries\":{},\"trace_segment_priors\":{},\"trace_segment_prior_summaries\":{},\"ledger_records\":{},\"ledger_authorized\":{},\"ledger_applied\":{},\"ledger_preview_only\":{},\"ledger_held\":{},\"ledger_rejected\":{},\"ledger_duplicate\":{},\"ledger_decayed\":{},\"ledger_merged\":{},\"ledger_rollback\":{},\"ledger_summaries\":{},\"read_only\":{},\"write_allowed\":{},\"applied\":{}}},\
         \"kv_fusion\":{{\"candidates\":{},\"fused\":{},\"compressed\":{},\"skipped\":{},\"held\":{},\"rejected\":{},\"approval_blocked\":{},\"input_tokens\":{},\"retained_tokens\":{},\"saved_tokens\":{},\"min_score\":{:.6},\"max_score\":{:.6},\"average_score\":{:.6},\"score_summaries\":{},\"read_only\":{},\"write_allowed\":{},\"applied\":{}}},\
         \"live_evolution\":{{\"live_inference_recorded\":true,\"live_router_threshold_delta\":{:.6},\"live_hierarchy_weight_delta\":{:.6},\"live_online_reward_feedbacks\":{},\"live_online_reward_reinforcements\":{},\"live_online_reward_penalties\":{},\"live_online_reward_strength\":{:.6},\"live_online_reward_reinforcement_strength\":{:.6},\"live_online_reward_penalty_strength\":{:.6},\"live_memory_reinforcements\":{},\"live_memory_penalties\":{},\"live_memory_updates\":{},\"live_stored_memory\":{},\"live_stored_gist_memories\":{},\"live_stored_runtime_kv_memories\":{},\"live_stored_memory_updates\":{},\"live_reflection_issues\":{},\"live_critical_reflection_issues\":{},\"live_revision_actions\":{}}},\
         \"evolution_ledger\":{{\"live_inference_runs\":{},\"cumulative_live_router_threshold_mutations\":{},\"cumulative_live_hierarchy_weight_mutations\":{},\"cumulative_live_router_threshold_delta\":{:.6},\"cumulative_live_hierarchy_weight_delta\":{:.6},\"cumulative_live_online_reward_feedbacks\":{},\"cumulative_live_online_reward_reinforcements\":{},\"cumulative_live_online_reward_penalties\":{},\"cumulative_live_online_reward_strength\":{:.6},\"cumulative_live_online_reward_reinforcement_strength\":{:.6},\"cumulative_live_online_reward_penalty_strength\":{:.6},\"cumulative_live_memory_reinforcements\":{},\"cumulative_live_memory_penalties\":{},\"cumulative_live_memory_updates\":{},\"cumulative_live_stored_memories\":{},\"cumulative_live_stored_gist_memories\":{},\"cumulative_live_stored_runtime_kv_memories\":{},\"cumulative_live_stored_memory_updates\":{},\"cumulative_live_reflection_issues\":{},\"cumulative_live_critical_reflection_issues\":{},\"cumulative_live_revision_actions\":{},\"replay_runs\":{},\"replay_items\":{},\"cumulative_router_threshold_mutations\":{},\"cumulative_hierarchy_weight_mutations\":{},\"cumulative_router_threshold_delta\":{:.6},\"cumulative_hierarchy_weight_delta\":{:.6},\"cumulative_memory_reinforcements\":{},\"cumulative_memory_penalties\":{},\"cumulative_memory_updates\":{},\"cumulative_replay_live_memory_feedback_items\":{},\"cumulative_replay_live_memory_feedback_updates\":{},\"cumulative_replay_live_memory_feedback_reinforcements\":{},\"cumulative_replay_live_memory_feedback_penalties\":{},\"cumulative_replay_live_memory_feedback_detail_items\":{},\"cumulative_replay_live_memory_feedback_applied\":{},\"cumulative_replay_live_memory_feedback_removed\":{},\"cumulative_replay_live_memory_feedback_missing\":{},\"cumulative_replay_live_memory_feedback_strength_delta\":{:.6},\"cumulative_replay_business_contract_items\":{},\"cumulative_replay_business_contract_passed\":{},\"cumulative_replay_business_contract_failed\":{},\"cumulative_replay_business_contract_raw_passed\":{},\"cumulative_replay_business_contract_raw_failed\":{},\"cumulative_replay_business_contract_response_normalized\":{},\"cumulative_replay_business_contract_sanitized\":{},\"cumulative_replay_business_contract_canonical_fallbacks\":{},\"cumulative_replay_live_evolution_items\":{},\"cumulative_replay_live_evolution_router_threshold_mutations\":{},\"cumulative_replay_live_evolution_hierarchy_weight_mutations\":{},\"cumulative_replay_live_evolution_router_threshold_delta\":{:.6},\"cumulative_replay_live_evolution_hierarchy_weight_delta\":{:.6},\"cumulative_replay_live_evolution_online_reward_feedbacks\":{},\"cumulative_replay_live_evolution_online_reward_reinforcements\":{},\"cumulative_replay_live_evolution_online_reward_penalties\":{},\"cumulative_replay_live_evolution_online_reward_strength\":{:.6},\"cumulative_replay_live_evolution_online_reward_reinforcement_strength\":{:.6},\"cumulative_replay_live_evolution_online_reward_penalty_strength\":{:.6},\"cumulative_replay_live_evolution_memory_updates\":{},\"cumulative_replay_live_evolution_stored_memory_updates\":{},\"cumulative_replay_live_evolution_reflection_issues\":{},\"cumulative_replay_live_evolution_critical_reflection_issues\":{},\"cumulative_replay_live_evolution_revision_actions\":{},\"cumulative_recursive_replay_items\":{},\"cumulative_recursive_runtime_calls\":{},\"cumulative_drift_rollbacks\":{},\"cumulative_rollback_router_threshold_delta\":{:.6},\"cumulative_rollback_hierarchy_weight_delta\":{:.6}}},\
         \"retention\":{{\"stale_after\":{},\"decay_rate\":{:.6},\"remove_below_strength\":{:.6},\"remove_after_failures\":{},\"before\":{},\"after\":{},\"decayed\":{},\"removed\":{}}},\
         \"memory_compaction\":{{\"similarity_threshold\":{:.6},\"max_candidates\":{},\"max_merges\":{},\"before\":{},\"after\":{},\"merged\":{},\"removed\":{},\"pairs\":{}}},\
         \"experience_id\":{}\
         }}",
        option_string_json(case_name),
        profile,
        prompt.chars().count(),
        json_escape(&prompt_digest),
        elapsed_ms,
        outcome.report.quality,
        outcome.metrics.perplexity,
        outcome.report.issues.len(),
        outcome.report.critical_issue_count(),
        outcome.report.max_severity().as_str(),
        string_array_json(&reflection_issue_codes),
        string_array_json(&outcome.report.revision_actions),
        outcome.report.revision_passes,
        outcome.router_threshold_after,
        outcome.route_budget.threshold,
        outcome.route_budget.attention_fraction,
        outcome.route_budget.attention_tokens,
        outcome.route_budget.fast_tokens,
        outcome.adaptive_route_plan.candidates,
        outcome.adaptive_route_plan.include,
        outcome.adaptive_route_plan.compress,
        outcome.adaptive_route_plan.defer,
        outcome.adaptive_route_plan.skip,
        outcome.adaptive_route_plan.input_tokens,
        outcome.adaptive_route_plan.retained_tokens,
        outcome.adaptive_route_plan.saved_tokens,
        outcome.adaptive_route_plan.min_score,
        outcome.adaptive_route_plan.max_score,
        outcome.adaptive_route_plan.average_score,
        string_array_json(&adaptive_route_actions),
        string_array_json(&adaptive_route_selected_routes),
        string_array_json(&adaptive_route_score_summaries),
        outcome.adaptive_route_plan.read_only,
        outcome.adaptive_route_plan.write_allowed,
        outcome.adaptive_route_plan.applied,
        outcome.compute_budget_schedule.compute_budget.as_str(),
        outcome.compute_budget_schedule.base_threshold,
        outcome.compute_budget_schedule.threshold_after,
        outcome.compute_budget_schedule.threshold_delta,
        outcome.compute_budget_schedule.route_fanout_before,
        outcome.compute_budget_schedule.route_fanout_after,
        outcome.compute_budget_schedule.candidate_count,
        outcome.compute_budget_schedule.selected_candidates,
        outcome.compute_budget_schedule.anchor_count,
        outcome.compute_budget_schedule.anchors_preserved(),
        outcome.compute_budget_schedule.anchors_preserved,
        outcome.compute_budget_schedule.low_value_skipped,
        outcome.compute_budget_schedule.kv_lookup_budget,
        outcome.compute_budget_schedule.kv_lookups_planned,
        outcome.compute_budget_schedule.kv_lookups_skipped,
        outcome.compute_budget_schedule.reflection_pass_budget,
        outcome.compute_budget_schedule.validation_run_budget,
        outcome.compute_budget_schedule.validation_cost_tokens,
        outcome.compute_budget_schedule.input_tokens,
        outcome.compute_budget_schedule.retained_tokens,
        outcome.compute_budget_schedule.saved_tokens,
        outcome.compute_budget_schedule.estimated_budget_tokens,
        outcome.compute_budget_schedule.estimated_spent_tokens,
        outcome
            .compute_budget_schedule
            .wasted_compute_avoided_tokens,
        outcome.compute_budget_schedule.fallback_triggered,
        string_array_json(&compute_budget_notes),
        outcome.compute_budget_schedule.read_only,
        outcome.compute_budget_schedule.write_allowed,
        outcome.compute_budget_schedule.applied,
        outcome.task_hierarchy_plan.mode.as_str(),
        outcome.task_hierarchy_plan.signals.language.as_str(),
        outcome.task_hierarchy_plan.signals.coding_intent,
        outcome.task_hierarchy_plan.signals.validation_mode,
        outcome.task_hierarchy_plan.signals.memory_need,
        outcome.task_hierarchy_plan.signals.compute_budget.as_str(),
        outcome.task_hierarchy_plan.hierarchy_depth,
        outcome.task_hierarchy_plan.route_fanout,
        outcome.task_hierarchy_plan.route_pressure,
        outcome.task_hierarchy_plan.compute_reduction,
        outcome.task_hierarchy_plan.threshold_before,
        outcome.task_hierarchy_plan.threshold_after,
        outcome.task_hierarchy_plan.threshold_after - outcome.task_hierarchy_plan.threshold_before,
        json_escape(&hierarchy_summary(
            outcome.task_hierarchy_plan.hierarchy_before
        )),
        json_escape(&hierarchy_summary(
            outcome.task_hierarchy_plan.hierarchy_after
        )),
        string_array_json(&outcome.task_hierarchy_plan.selected_lanes),
        string_array_json(&outcome.task_hierarchy_plan.skipped_lanes),
        string_array_json(&outcome.task_hierarchy_plan.memory_lanes),
        string_array_json(&outcome.task_hierarchy_plan.skipped_memory_lanes),
        outcome.task_hierarchy_plan.mutation_count(),
        string_array_json(&task_hierarchy_mutation_summaries),
        json_escape(&outcome.task_hierarchy_plan.rollback_anchor_id),
        outcome.task_hierarchy_plan.mutation_history_replayable(),
        false,
        outcome.task_hierarchy_plan.runtime_applied,
        outcome.task_hierarchy_plan.state_write_allowed,
        outcome.task_hierarchy_plan.adaptive_state_write_allowed,
        outcome.task_hierarchy_plan.ndkv_write_allowed,
        outcome.runtime_token_metrics.token_count,
        outcome.runtime_token_metrics.entropy_count,
        outcome.runtime_token_metrics.logprob_count,
        option_f32_json(outcome.runtime_token_metrics.average_entropy),
        option_f32_json(outcome.runtime_token_metrics.average_neg_logprob),
        option_f32_json(outcome.runtime_token_metrics.uncertainty_perplexity),
        outcome.runtime_token_metrics.has_uncertainty_signal(),
        outcome.embedding_diagnostics.query.source.as_str(),
        outcome.embedding_diagnostics.query.dimensions,
        option_owned_string_json(
            outcome
                .embedding_diagnostics
                .memory_write
                .map(|call| call.source.as_str())
        ),
        outcome
            .embedding_diagnostics
            .memory_write
            .map(|call| call.dimensions)
            .unwrap_or(0),
        outcome.embedding_diagnostics.gist_writes.len(),
        outcome.embedding_diagnostics.gist_write_runtime_calls(),
        outcome.embedding_diagnostics.gist_write_fallback_calls(),
        outcome.embedding_diagnostics.runtime_calls,
        outcome.embedding_diagnostics.fallback_calls,
        outcome.embedding_diagnostics.runtime_embedding_available(),
        outcome.embedding_diagnostics.fallback_embedding_used(),
        option_owned_string_json(outcome.runtime_diagnostics.model_id.as_deref()),
        option_owned_string_json(runtime_selected_adapter),
        option_owned_string_json(outcome.runtime_diagnostics.adapter_cache_mode.as_deref()),
        option_owned_string_json(
            outcome
                .runtime_diagnostics
                .adapter_stream_trace_id
                .as_deref()
        ),
        option_owned_string_json(
            outcome
                .runtime_diagnostics
                .adapter_stream_gate_summary_digest
                .as_deref()
        ),
        option_bool_json(outcome.runtime_diagnostics.adapter_stream_read_only),
        option_bool_json(outcome.runtime_diagnostics.adapter_stream_write_allowed),
        option_bool_json(outcome.runtime_diagnostics.adapter_stream_applied),
        option_owned_string_json(outcome.runtime_diagnostics.device_profile.as_deref()),
        option_owned_string_json(outcome.runtime_diagnostics.primary_lane.as_deref()),
        option_owned_string_json(outcome.runtime_diagnostics.fallback_lane.as_deref()),
        option_owned_string_json(outcome.runtime_diagnostics.memory_mode.as_deref()),
        option_owned_string_json(
            outcome
                .runtime_diagnostics
                .device_execution_source
                .as_deref()
        ),
        option_u8_json(outcome.runtime_diagnostics.hot_kv_precision_bits),
        option_u8_json(outcome.runtime_diagnostics.cold_kv_precision_bits),
        outcome.runtime_diagnostics.layer_count,
        outcome.runtime_diagnostics.global_layers,
        outcome.runtime_diagnostics.local_window_layers,
        outcome.runtime_diagnostics.convolutional_fusion_layers,
        outcome.runtime_diagnostics.hidden_size,
        outcome.runtime_diagnostics.local_window_tokens,
        option_f32_json(outcome.runtime_diagnostics.forward_energy),
        option_f32_json(outcome.runtime_diagnostics.kv_influence),
        outcome.runtime_diagnostics.imported_kv_blocks,
        outcome.runtime_diagnostics.exported_kv_blocks,
        outcome.runtime_diagnostics.weak_runtime_kv_imports_skipped,
        outcome
            .runtime_diagnostics
            .budget_limited_runtime_kv_imports_skipped,
        outcome.runtime_diagnostics.runtime_kv_segments_included,
        outcome.runtime_diagnostics.runtime_kv_segments_skipped,
        outcome.runtime_diagnostics.runtime_kv_segments_rejected,
        outcome.runtime_diagnostics.runtime_kv_segment_count(),
        option_f32_json(outcome.runtime_diagnostics.runtime_kv_segment_yield()),
        outcome.runtime_diagnostics.runtime_kv_segment_count(),
        string_array_json(&runtime_kv_segment_lifecycle_summaries),
        outcome.runtime_diagnostics.has_runtime_kv_activity_signal(),
        outcome.runtime_diagnostics.has_runtime_kv_segment_signal(),
        outcome
            .runtime_diagnostics
            .has_runtime_architecture_signal(),
        outcome.runtime_diagnostics.has_forward_signal(),
        outcome.runtime_diagnostics.has_all_layer_modes(),
        outcome.runtime_diagnostics.has_valid_kv_precision_signal(),
        outcome.runtime_adapter_observations.len(),
        option_owned_string_json(
            best_adapter_observation.map(|observation| observation.adapter.as_str())
        ),
        runtime_adapter_selection_mismatch,
        option_f32_json(best_adapter_observation.map(|observation| observation.score)),
        option_f32_json(best_adapter_observation.map(|observation| observation.reward)),
        option_f32_json(best_adapter_observation.map(|observation| observation.quality)),
        option_f32_json(
            best_adapter_observation.and_then(|observation| observation.forward_energy)
        ),
        option_f32_json(best_adapter_observation.and_then(|observation| observation.kv_influence)),
        option_u64_json(best_adapter_observation.map(|observation| observation.experience_id)),
        outcome.hierarchy.global,
        outcome.hierarchy.local,
        outcome.hierarchy.convolution,
        outcome.hardware_plan.device.as_str(),
        outcome.hardware_plan.tier.as_str(),
        outcome.hardware_plan.pressure,
        json_escape(&outcome.hardware_plan.runtime_contract_summary()),
        option_u64_json(outcome.hardware_plan.latency_budget_ms),
        outcome.hardware_plan.local_kv_token_budget,
        outcome.hardware_plan.global_kv_token_budget,
        runtime_budget,
        outcome.hardware_plan.execution.primary_lane.as_str(),
        outcome.hardware_plan.execution.fallback_lane.as_str(),
        outcome.hardware_plan.execution.memory_mode.as_str(),
        outcome.hardware_plan.execution.max_parallel_chunks,
        outcome.hardware_plan.execution.kv_prefetch_blocks,
        outcome.hardware_plan.execution.hot_kv_precision_bits,
        outcome.hardware_plan.execution.cold_kv_precision_bits,
        outcome.hardware_plan.execution.allow_disk_spill,
        string_array_json(&adapter_hints),
        outcome.recursive_schedule.requires_recursion,
        outcome.recursive_schedule.prompt_tokens,
        outcome.recursive_schedule.native_window_tokens,
        outcome.recursive_schedule.chunk_count(),
        outcome.recursive_schedule.merge_round_count(),
        outcome.recursive_schedule.execution_wave_count(),
        outcome.recursive_schedule.max_parallel_chunks,
        outcome.recursive_schedule.chunk_tokens,
        outcome.recursive_schedule.overlap_tokens,
        outcome.recursive_runtime_calls,
        tier_counts.hot_gpu,
        tier_counts.warm_ram,
        tier_counts.cold_disk,
        infini_counts.local_window,
        infini_counts.global_memory,
        infini_counts.skipped,
        infini_counts.local_tokens,
        infini_counts.global_tokens,
        infini_counts.skipped_tokens,
        json_escape(outcome.transformer_plan.template_name()),
        transformer_counts.global,
        transformer_counts.local,
        transformer_counts.convolution,
        outcome.toolsmith_plan.rust_only,
        outcome.toolsmith_plan.exploration_required,
        outcome.toolsmith_plan.blueprint_count(),
        outcome.toolsmith_plan.ready_count(),
        outcome.toolsmith_plan.held_count(),
        outcome.toolsmith_plan.rejected_count(),
        outcome.toolsmith_plan.passed_rust_gate(),
        string_array_json(&outcome.toolsmith_plan.notes),
        string_array_json(&outcome.toolsmith_plan.rejected_requests),
        string_array_json(&toolsmith_blueprints),
        outcome.agent_team_plan.enabled,
        json_escape(&outcome.agent_team_plan.summary()),
        json_escape(&outcome.agent_team_plan.run_id),
        json_escape(&agent_team_goal_digest),
        outcome.agent_team_plan.active_agent_count(),
        outcome.agent_team_plan.message_count(),
        outcome.agent_team_plan.conflict_count(),
        outcome.agent_team_plan.unresolved_conflict_count(),
        outcome.agent_team_plan.evolution_signal_count(),
        outcome.agent_team_plan.collision_free(),
        outcome.agent_team_plan.isolation.single_writer,
        outcome.agent_team_plan.isolation.read_only_subagents,
        json_escape(&outcome.agent_team_plan.isolation.namespace),
        string_array_json(&outcome.agent_team_plan.isolation.allowed_outputs),
        string_array_json(&outcome.agent_team_plan.isolation.denied_capabilities),
        agent_team_aggregation.lane_count,
        string_array_json(&agent_team_aggregation.message_summaries),
        string_array_json(&agent_team_aggregation.conflict_topics),
        string_array_json(&agent_team_aggregation.unresolved_conflict_topics),
        json_escape(&agent_team_aggregation.budget_scope),
        agent_team_aggregation.max_parallel_lanes,
        agent_team_aggregation.attention_fraction,
        json_escape(&agent_team_aggregation.main_thread_writer),
        string_array_json(&agent_team_messages),
        string_array_json(&agent_team_conflicts),
        string_array_json(&agent_team_evolution),
        json_escape(&outcome.reasoning_genome.genome_id),
        json_escape(&outcome.reasoning_genome.stable_anchor_id),
        outcome.reasoning_genome_chain.total_gene_count(),
        string_array_json(&reasoning_genome_lineage_scope_digests),
        reasoning_genome_mixed_lineage,
        outcome.reasoning_genome.expression_gene_count,
        outcome.reasoning_genome.active_gene_count(),
        outcome.reasoning_genome.aged_gene_count(),
        outcome.reasoning_genome.malignant_gene_count(),
        outcome.reasoning_genome.relabel_candidate_count(),
        outcome.reasoning_genome.regeneration_candidate_count(),
        outcome.reasoning_genome.scissors_proposal_count(),
        outcome.reasoning_genome.repair_payload_count(),
        outcome.reasoning_genome.regeneration_payload_count(),
        string_array_json(&reasoning_genome_mutation_intents),
        string_array_json(&reasoning_genome_proposal_ids),
        outcome.reasoning_genome.read_only,
        outcome.reasoning_genome.write_allowed,
        outcome.reasoning_genome.applied,
        outcome.reasoning_genome.youth_pressure,
        outcome.reasoning_genome.lifecycle_record_count(),
        string_array_json(&reasoning_genome_lifecycle_actions),
        string_array_json(&reasoning_genome_lifecycle_summaries),
        outcome.reasoning_genome.tombstone_candidate_count(),
        outcome
            .reasoning_genome
            .pending_lifecycle_validation_count(),
        outcome.reasoning_genome.lifecycle_source_evidence_count(),
        outcome.reasoning_genome_splice.segments.len(),
        outcome.reasoning_genome_splice.exon_count(),
        outcome.reasoning_genome_splice.intron_count(),
        outcome.reasoning_genome_splice.variant_count(),
        outcome.reasoning_genome_splice.retained_count(),
        outcome.reasoning_genome_splice.skipped_count(),
        outcome.reasoning_genome_splice.quarantined_count(),
        outcome.reasoning_genome_splice.repair_candidate_count(),
        string_array_json(&reasoning_genome_splice_dispositions),
        string_array_json(&reasoning_genome_splice_reason_summaries),
        outcome.reasoning_genome_splice.lifecycle_record_count(),
        string_array_json(&reasoning_genome_splice_lifecycle_states),
        string_array_json(&reasoning_genome_splice_lifecycle_summaries),
        outcome.reasoning_genome_splice.findings.len(),
        string_array_json(&reasoning_genome_splice_finding_kinds),
        string_array_json(&reasoning_genome_splice_mutation_intents),
        outcome.reasoning_genome_splice.mutation_plans.len(),
        string_array_json(&reasoning_genome_splice_proposal_ids),
        outcome.reasoning_genome_splice.read_only,
        outcome.reasoning_genome_splice.write_allowed,
        outcome.reasoning_genome_splice.applied,
        outcome.stream_reports.len(),
        outcome.used_memories.len(),
        option_u64_json(outcome.stored_memory_id),
        outcome.gist_records.len(),
        outcome.stored_gist_memory_ids.len(),
        outcome.exported_runtime_kv_blocks,
        runtime_kv_stored,
        runtime_kv_hold,
        runtime_kv_held,
        outcome.memory_feedback.reinforced,
        outcome.memory_feedback.penalized,
        outcome.memory_feedback.reinforcement_amount,
        outcome.memory_feedback.penalty_amount,
        outcome.memory_feedback.total_updates(),
        outcome.memory_feedback.applied_updates(),
        outcome.memory_feedback.removed_updates(),
        outcome.memory_feedback.missing_updates(),
        outcome.memory_feedback.strength_delta(),
        string_array_json(&memory_feedback_summaries(outcome)),
        outcome.drift_report.severity.as_str(),
        outcome.drift_report.allow_memory_write,
        outcome.drift_report.allow_runtime_kv_write,
        outcome.drift_report.penalize_used_memory,
        outcome.drift_report.rollback_adaptive,
        string_array_json(&outcome.drift_report.notes),
        outcome.process_reward.total,
        outcome.process_reward.action.as_str(),
        outcome.process_reward.components.route,
        outcome.process_reward.components.memory,
        outcome.process_reward.components.hierarchy,
        outcome.process_reward.components.reflection,
        outcome.process_reward.components.latency,
        outcome.process_reward.components.admission,
        string_array_json(&outcome.process_reward.notes),
        auto_replay.applied,
        auto_replay.router_updates,
        auto_replay.hierarchy_updates,
        auto_replay.router_threshold_mutations,
        auto_replay.hierarchy_weight_mutations,
        auto_replay.router_threshold_delta,
        auto_replay.hierarchy_weight_delta,
        auto_replay.reinforced,
        auto_replay.penalized,
        auto_replay.touched_memories,
        auto_replay.memory_reinforcements,
        auto_replay.memory_penalties,
        auto_replay.live_memory_feedback_items,
        auto_replay.live_memory_feedback_updates,
        auto_replay.live_memory_feedback_reinforcements,
        auto_replay.live_memory_feedback_penalties,
        auto_replay.live_memory_feedback_detail_items,
        auto_replay.live_memory_feedback_applied,
        auto_replay.live_memory_feedback_removed,
        auto_replay.live_memory_feedback_missing,
        auto_replay.live_memory_feedback_strength_delta,
        auto_replay.business_contract_items,
        auto_replay.business_contract_passed,
        auto_replay.business_contract_failed,
        auto_replay.business_contract_raw_passed,
        auto_replay.business_contract_raw_failed,
        auto_replay.business_contract_response_normalized,
        auto_replay.business_contract_sanitized,
        auto_replay.business_contract_canonical_fallbacks,
        auto_replay.live_evolution_items,
        auto_replay.live_evolution_router_threshold_mutations,
        auto_replay.live_evolution_hierarchy_weight_mutations,
        auto_replay.live_evolution_router_threshold_delta,
        auto_replay.live_evolution_hierarchy_weight_delta,
        auto_replay.live_evolution_online_reward_feedbacks,
        auto_replay.live_evolution_online_reward_reinforcements,
        auto_replay.live_evolution_online_reward_penalties,
        auto_replay.live_evolution_online_reward_strength,
        auto_replay.live_evolution_online_reward_reinforcement_strength,
        auto_replay.live_evolution_online_reward_penalty_strength,
        auto_replay.live_evolution_memory_updates,
        auto_replay.live_evolution_stored_memory_updates,
        auto_replay.live_evolution_reflection_issues,
        auto_replay.live_evolution_critical_reflection_issues,
        auto_replay.live_evolution_revision_actions,
        auto_replay.runtime_kv_budget_pressure_items,
        auto_replay.average_runtime_kv_budget_pressure,
        auto_replay.max_runtime_kv_budget_pressure,
        auto_replay.runtime_kv_weak_import_pressure_items,
        auto_replay.average_runtime_kv_weak_import_pressure,
        auto_replay.max_runtime_kv_weak_import_pressure,
        auto_replay.recursive_runtime_items,
        auto_replay.recursive_runtime_calls,
        auto_replay.average_recursive_call_pressure,
        auto_replay.max_recursive_call_pressure,
        outcome.memory_admission.candidate_count(),
        outcome.memory_admission.ready_count(),
        outcome.memory_admission.blocked_count(),
        outcome.memory_admission.admitted_count(),
        outcome.memory_admission.hold_count(),
        outcome.memory_admission.reject_count(),
        outcome.memory_admission.quarantine_count(),
        string_array_json(&memory_admission_kinds),
        string_array_json(&memory_admission_decisions),
        string_array_json(&memory_admission_candidates),
        outcome.memory_admission.review_packet_count(),
        string_array_json(&memory_admission_review_packets),
        outcome.memory_admission.trace_segment_replay_prior_count(),
        string_array_json(&trace_segment_replay_prior_summaries),
        outcome.memory_admission.ledger_record_count(),
        outcome.memory_admission.ledger_authorized_count(),
        outcome.memory_admission.ledger_applied_count(),
        outcome.memory_admission.ledger_preview_only_count(),
        outcome.memory_admission.ledger_held_count(),
        outcome.memory_admission.ledger_rejected_count(),
        outcome.memory_admission.ledger_duplicate_count(),
        outcome.memory_admission.ledger_decayed_count(),
        outcome.memory_admission.ledger_merged_count(),
        outcome.memory_admission.ledger_rollback_count(),
        string_array_json(&memory_admission_ledger_summaries),
        outcome.memory_admission.read_only,
        outcome.memory_admission.write_allowed,
        outcome.memory_admission.applied,
        outcome.memory_admission.fusion_plan.candidates,
        outcome.memory_admission.fusion_plan.fused,
        outcome.memory_admission.fusion_plan.compressed,
        outcome.memory_admission.fusion_plan.skipped,
        outcome.memory_admission.fusion_plan.held,
        outcome.memory_admission.fusion_plan.rejected,
        outcome.memory_admission.fusion_plan.approval_blocked,
        outcome.memory_admission.fusion_plan.input_tokens,
        outcome.memory_admission.fusion_plan.retained_tokens,
        outcome.memory_admission.fusion_plan.saved_tokens,
        outcome.memory_admission.fusion_plan.min_score,
        outcome.memory_admission.fusion_plan.max_score,
        outcome.memory_admission.fusion_plan.average_score,
        string_array_json(&kv_fusion_score_summaries),
        outcome.memory_admission.fusion_plan.read_only,
        outcome.memory_admission.fusion_plan.write_allowed,
        outcome.memory_admission.fusion_plan.applied,
        outcome.live_evolution.router_threshold_delta,
        outcome.live_evolution.hierarchy_weight_delta,
        outcome.live_evolution.online_reward_feedbacks,
        outcome.live_evolution.online_reward_reinforcements,
        outcome.live_evolution.online_reward_penalties,
        outcome.live_evolution.online_reward_strength,
        outcome.live_evolution.online_reward_reinforcement_strength,
        outcome.live_evolution.online_reward_penalty_strength,
        outcome.live_evolution.memory_reinforcements,
        outcome.live_evolution.memory_penalties,
        outcome
            .live_evolution
            .memory_reinforcements
            .saturating_add(outcome.live_evolution.memory_penalties),
        outcome.live_evolution.stored_memory,
        outcome.live_evolution.stored_gist_memories,
        outcome.live_evolution.stored_runtime_kv_memories,
        usize::from(outcome.live_evolution.stored_memory)
            .saturating_add(outcome.live_evolution.stored_gist_memories)
            .saturating_add(outcome.live_evolution.stored_runtime_kv_memories),
        outcome.live_evolution.reflection_issues,
        outcome.live_evolution.critical_reflection_issues,
        outcome.live_evolution.revision_actions,
        outcome.evolution_ledger.live_inference_runs,
        outcome.evolution_ledger.live_router_threshold_mutations,
        outcome.evolution_ledger.live_hierarchy_weight_mutations,
        outcome.evolution_ledger.live_router_threshold_delta,
        outcome.evolution_ledger.live_hierarchy_weight_delta,
        outcome.evolution_ledger.live_online_reward_feedbacks,
        outcome.evolution_ledger.live_online_reward_reinforcements,
        outcome.evolution_ledger.live_online_reward_penalties,
        outcome.evolution_ledger.live_online_reward_strength,
        outcome
            .evolution_ledger
            .live_online_reward_reinforcement_strength,
        outcome.evolution_ledger.live_online_reward_penalty_strength,
        outcome.evolution_ledger.live_memory_reinforcements,
        outcome.evolution_ledger.live_memory_penalties,
        outcome.evolution_ledger.live_memory_updates(),
        outcome.evolution_ledger.live_stored_memories,
        outcome.evolution_ledger.live_stored_gist_memories,
        outcome.evolution_ledger.live_stored_runtime_kv_memories,
        outcome.evolution_ledger.live_stored_memory_updates(),
        outcome.evolution_ledger.live_reflection_issues,
        outcome.evolution_ledger.live_critical_reflection_issues,
        outcome.evolution_ledger.live_revision_actions,
        outcome.evolution_ledger.replay_runs,
        outcome.evolution_ledger.replay_items,
        outcome.evolution_ledger.router_threshold_mutations,
        outcome.evolution_ledger.hierarchy_weight_mutations,
        outcome.evolution_ledger.router_threshold_delta,
        outcome.evolution_ledger.hierarchy_weight_delta,
        outcome.evolution_ledger.memory_reinforcements,
        outcome.evolution_ledger.memory_penalties,
        outcome.evolution_ledger.memory_updates(),
        outcome.evolution_ledger.replay_live_memory_feedback_items,
        outcome
            .evolution_ledger
            .replay_live_memory_feedback_updates(),
        outcome
            .evolution_ledger
            .replay_live_memory_feedback_reinforcements,
        outcome
            .evolution_ledger
            .replay_live_memory_feedback_penalties,
        outcome
            .evolution_ledger
            .replay_live_memory_feedback_detail_items,
        outcome.evolution_ledger.replay_live_memory_feedback_applied,
        outcome.evolution_ledger.replay_live_memory_feedback_removed,
        outcome.evolution_ledger.replay_live_memory_feedback_missing,
        outcome
            .evolution_ledger
            .replay_live_memory_feedback_strength_delta,
        outcome.evolution_ledger.replay_business_contract_items,
        outcome.evolution_ledger.replay_business_contract_passed,
        outcome.evolution_ledger.replay_business_contract_failed,
        outcome.evolution_ledger.replay_business_contract_raw_passed,
        outcome.evolution_ledger.replay_business_contract_raw_failed,
        outcome
            .evolution_ledger
            .replay_business_contract_response_normalized,
        outcome.evolution_ledger.replay_business_contract_sanitized,
        outcome
            .evolution_ledger
            .replay_business_contract_canonical_fallbacks,
        outcome.evolution_ledger.replay_live_evolution_items,
        outcome
            .evolution_ledger
            .replay_live_evolution_router_threshold_mutations,
        outcome
            .evolution_ledger
            .replay_live_evolution_hierarchy_weight_mutations,
        outcome
            .evolution_ledger
            .replay_live_evolution_router_threshold_delta,
        outcome
            .evolution_ledger
            .replay_live_evolution_hierarchy_weight_delta,
        outcome
            .evolution_ledger
            .replay_live_evolution_online_reward_feedbacks,
        outcome
            .evolution_ledger
            .replay_live_evolution_online_reward_reinforcements,
        outcome
            .evolution_ledger
            .replay_live_evolution_online_reward_penalties,
        outcome
            .evolution_ledger
            .replay_live_evolution_online_reward_strength,
        outcome
            .evolution_ledger
            .replay_live_evolution_online_reward_reinforcement_strength,
        outcome
            .evolution_ledger
            .replay_live_evolution_online_reward_penalty_strength,
        outcome
            .evolution_ledger
            .replay_live_evolution_memory_updates,
        outcome
            .evolution_ledger
            .replay_live_evolution_stored_memory_updates,
        outcome
            .evolution_ledger
            .replay_live_evolution_reflection_issues,
        outcome
            .evolution_ledger
            .replay_live_evolution_critical_reflection_issues,
        outcome
            .evolution_ledger
            .replay_live_evolution_revision_actions,
        outcome.evolution_ledger.recursive_replay_items,
        outcome.evolution_ledger.recursive_runtime_calls,
        outcome.evolution_ledger.drift_rollbacks,
        outcome.evolution_ledger.rollback_router_threshold_delta,
        outcome.evolution_ledger.rollback_hierarchy_weight_delta,
        outcome.memory_retention_policy.stale_after,
        outcome.memory_retention_policy.decay_rate,
        outcome.memory_retention_policy.remove_below_strength,
        outcome.memory_retention_policy.remove_after_failures,
        outcome.retention_report.before,
        outcome.retention_report.after,
        outcome.retention_report.decayed,
        outcome.retention_report.removed.len(),
        outcome.memory_compaction_policy.similarity_threshold,
        outcome.memory_compaction_policy.max_candidates,
        outcome.memory_compaction_policy.max_merges,
        outcome.memory_compaction_report.before,
        outcome.memory_compaction_report.after,
        outcome.memory_compaction_report.merged.len(),
        outcome.memory_compaction_report.removed.len(),
        memory_compaction_pairs_json(&outcome.memory_compaction_report.merged),
        outcome.experience_id
    )
}

fn runtime_kv_segment_lifecycle_summaries(
    prompt: &str,
    diagnostics: &crate::reflection::RuntimeDiagnostics,
) -> Vec<String> {
    let source_digest = stable_redaction_digest([
        "runtime_kv_segment",
        prompt,
        &diagnostics.runtime_kv_segment_count().to_string(),
    ]);
    let mut summaries = Vec::new();
    push_runtime_kv_segment_lifecycle_summary(
        &mut summaries,
        diagnostics.runtime_kv_segments_included,
        "active",
        "runtime_kv_segment_included",
        &source_digest,
        "none",
        false,
    );
    push_runtime_kv_segment_lifecycle_summary(
        &mut summaries,
        diagnostics.runtime_kv_segments_skipped,
        "recycle_candidate",
        "runtime_kv_segment_skipped_by_budget_or_value",
        &source_digest,
        "hold_until_budget_and_verifier_pass",
        true,
    );
    push_runtime_kv_segment_lifecycle_summary(
        &mut summaries,
        diagnostics.runtime_kv_segments_rejected,
        "rejected_final",
        "runtime_kv_segment_rejected_by_safety_or_contract",
        &source_digest,
        "operator_approval_required_for_reentry",
        true,
    );
    summaries
}

fn push_runtime_kv_segment_lifecycle_summary(
    summaries: &mut Vec<String>,
    count: usize,
    lifecycle: &str,
    reason_code: &str,
    source_digest: &str,
    readmission_gate: &str,
    operator_approval_required: bool,
) {
    if count == 0 {
        return;
    }
    let (shadow_state, drift_state, domain_state, expires_after_steps, score_milli) =
        match lifecycle {
            "active" => (
                "ready_for_explicit_apply",
                "drift_passed",
                "pass",
                168,
                1000,
            ),
            "rejected_final" => ("quarantined", "drift_failed", "reject", 0, 0),
            _ => ("benchmark_pending", "benchmark_pending", "pending", 72, 450),
        };
    let drift_gate_domains = format!(
        "golden_fixture:{domain_state}|routing_behavior:{domain_state}|memory_hygiene:{domain_state}|privacy:{domain_state}|trace_schema:{domain_state}"
    );
    let rollback = stable_redaction_digest([
        "runtime_kv_segment_rollback",
        lifecycle,
        reason_code,
        source_digest,
    ]);
    summaries.push(format!(
        "lifecycle={lifecycle} count={count} reason_code={reason_code} source_digest={source_digest} shadow_state={shadow_state} drift_state={drift_state} source_ids={count} expires_after_steps={expires_after_steps} score_milli={score_milli} drift_gate_domains={drift_gate_domains} rollback={rollback} parent_lineage=runtime_kv_segment:trace_runtime_diagnostics rollback_anchor=runtime_kv_segment_preview_only_no_apply affected_scope=runtime_kv_segment_candidate readmission_gate={readmission_gate} operator_approval_required={operator_approval_required}"
    ));
}

fn memory_compaction_pairs_json(pairs: &[crate::kv_cache::MemoryCompactionMerge]) -> String {
    let values = pairs
        .iter()
        .map(|pair| {
            format!(
                "{{\"primary_id\":{},\"removed_id\":{},\"similarity\":{:.6},\"namespace\":\"{}\",\"primary_vector_dimensions\":{},\"removed_vector_dimensions\":{},\"primary_protected\":{},\"removed_protected\":{}}}",
                pair.primary_id,
                pair.removed_id,
                pair.similarity,
                json_escape(&pair.namespace),
                pair.primary_vector_dimensions,
                pair.removed_vector_dimensions,
                pair.primary_protected,
                pair.removed_protected
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn reasoning_genome_lineage_scope_digests(chain: &DnaGeneChain) -> Vec<String> {
    chain
        .express_chain
        .iter()
        .chain(chain.memory_chain.iter())
        .map(|record| {
            stable_redaction_digest([
                "reasoning_genome_lineage",
                &record.lineage.tenant_scope,
                &record.lineage.session_id,
            ])
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn runtime_budget_json(plan: &crate::hardware::HardwarePlan) -> String {
    let budget = &plan.runtime_budget;
    format!(
        "{{\"requested_device\":\"{}\",\"selected_device\":\"{}\",\"selected_adapter\":\"{}\",\"backend_family\":\"{}\",\"quantization_profile\":\"{}\",\"weight_quantization_bits\":{},\"kv_cache_quantization_bits\":{},\"gene_cache_quantization_bits\":{},\"model_weight_bytes\":{},\"kv_cache_bytes\":{},\"gene_segment_cache_bytes\":{},\"routing_reflection_overhead_bytes\":{},\"total_required_bytes\":{},\"available_budget_bytes\":{},\"memory_pressure\":{:.6},\"fallback_reason\":\"{}\",\"fail_closed_cpu_stub\":{},\"read_only\":{},\"write_allowed\":{},\"applied\":{}}}",
        budget.requested_device.as_str(),
        budget.selected_device.as_str(),
        budget.selected_adapter.as_str(),
        json_escape(budget.backend_family),
        budget.quantization_profile.as_str(),
        budget.weight_quantization_bits,
        budget.kv_cache_quantization_bits,
        budget.gene_cache_quantization_bits,
        budget.model_weight_bytes,
        budget.kv_cache_bytes,
        budget.gene_segment_cache_bytes,
        budget.routing_reflection_overhead_bytes,
        budget.total_required_bytes,
        budget.available_budget_bytes,
        budget.memory_pressure,
        budget.fallback_reason.as_str(),
        budget.fail_closed_cpu_stub,
        budget.read_only,
        budget.write_allowed,
        budget.applied
    )
}

fn hierarchy_summary(weights: crate::hierarchy::HierarchyWeights) -> String {
    format!(
        "g:{:.3}|l:{:.3}|c:{:.3}",
        weights.global, weights.local, weights.convolution
    )
}
