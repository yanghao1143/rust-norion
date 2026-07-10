mod experiences;
mod metrics;

use std::collections::BTreeSet;

use crate::engine::NoironEngine;
use crate::experience::{ExperienceRecord, ExperienceStore};
use crate::tenant_scope::TenantScope;

use super::{
    StateExperienceHygieneFinding, StateExperienceIndexFinding, StateExperienceSummary,
    StateInspectionReport, is_runtime_kv_memory_key, memory_vector_dimensions_for_entries,
    runtime_kv_vector_dimensions_for_entries, top_memory_summaries_for_entries,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StateInspectionBuildMode {
    Full,
    Online,
}

#[derive(Debug, Clone)]
struct ExperienceInspectionSnapshot {
    top_experiences: Vec<StateExperienceSummary>,
    hygiene_finding_count: usize,
    hygiene_watch_count: usize,
    hygiene_quarantine_candidate_count: usize,
    hygiene_legacy_metadata_lesson_count: usize,
    hygiene_legacy_metadata_without_clean_gist_count: usize,
    repairable_legacy_metadata_lesson_count: usize,
    repairable_index_record_count: usize,
    repair_projected_hygiene_finding_count: usize,
    repair_projected_hygiene_watch_count: usize,
    repair_projected_hygiene_quarantine_candidate_count: usize,
    repair_projected_legacy_metadata_lesson_count: usize,
    repair_projected_legacy_metadata_without_clean_gist_count: usize,
    repair_skipped_quarantine_candidate_count: usize,
    repair_skipped_missing_clean_gist_count: usize,
    index_compacted_record_count: usize,
    index_overlong_record_count: usize,
    index_overlong_without_clean_gist_count: usize,
    index_max_record_chars: usize,
    index_noisy_record_count: usize,
    index_duplicate_output_count: usize,
    index_max_noise_penalty: f32,
    index_quality_score: f32,
    index_retrieval_ready: bool,
    index_risk_level: String,
    hygiene_findings: Vec<StateExperienceHygieneFinding>,
    index_findings: Vec<StateExperienceIndexFinding>,
}

impl StateInspectionReport {
    pub fn from_engine(engine: &NoironEngine, limit: usize) -> Self {
        Self::from_engine_with_mode(engine, limit, StateInspectionBuildMode::Full, None)
    }

    pub fn from_engine_scoped(engine: &NoironEngine, limit: usize, scope: &TenantScope) -> Self {
        Self::from_engine_with_mode(engine, limit, StateInspectionBuildMode::Full, Some(scope))
    }

    pub fn from_engine_online(engine: &NoironEngine, limit: usize) -> Self {
        Self::from_engine_with_mode(engine, limit, StateInspectionBuildMode::Online, None)
    }

    pub fn from_engine_online_scoped(
        engine: &NoironEngine,
        limit: usize,
        scope: &TenantScope,
    ) -> Self {
        Self::from_engine_with_mode(engine, limit, StateInspectionBuildMode::Online, Some(scope))
    }

    fn from_engine_with_mode(
        engine: &NoironEngine,
        limit: usize,
        mode: StateInspectionBuildMode,
        scope: Option<&TenantScope>,
    ) -> Self {
        let limit = limit.max(1);
        let adaptive_state = engine.adaptive_state();
        let scoped_memory_entries = scope.map(|scope| engine.cache.entries_scoped(scope));
        let memory_entries = scoped_memory_entries
            .as_deref()
            .unwrap_or_else(|| engine.cache.entries());
        let scoped_experience_store = scope.map(|_| {
            let visible_memory_ids = memory_entries
                .iter()
                .map(|entry| entry.id)
                .collect::<BTreeSet<_>>();
            let records = engine
                .experience
                .records()
                .iter()
                .filter(|record| experience_record_has_visible_memory(record, &visible_memory_ids))
                .cloned()
                .collect::<Vec<_>>();
            ExperienceStore::from_records_for_inspection(records)
        });
        let experience_store = scoped_experience_store
            .as_ref()
            .unwrap_or(&engine.experience);
        let top_memories = top_memory_summaries_for_entries(memory_entries, limit, |_| true);
        let top_runtime_kv_memories =
            top_memory_summaries_for_entries(memory_entries, limit, is_runtime_kv_memory_key);
        let counts = metrics::aggregate_counts(engine, experience_store.records());
        let experience = experience_inspection_snapshot(experience_store, limit, mode);

        Self {
            memory_count: memory_entries.len(),
            runtime_kv_memory_count: memory_entries
                .iter()
                .filter(|entry| is_runtime_kv_memory_key(&entry.key))
                .count(),
            experience_count: experience_store.len(),
            experience_hygiene_finding_count: experience.hygiene_finding_count,
            experience_hygiene_watch_count: experience.hygiene_watch_count,
            experience_hygiene_quarantine_candidate_count: experience
                .hygiene_quarantine_candidate_count,
            experience_hygiene_legacy_metadata_lesson_count: experience
                .hygiene_legacy_metadata_lesson_count,
            experience_hygiene_legacy_metadata_without_clean_gist_count: experience
                .hygiene_legacy_metadata_without_clean_gist_count,
            experience_repairable_legacy_metadata_lesson_count: experience
                .repairable_legacy_metadata_lesson_count,
            experience_repairable_index_record_count: experience.repairable_index_record_count,
            experience_repair_projected_hygiene_finding_count: experience
                .repair_projected_hygiene_finding_count,
            experience_repair_projected_hygiene_watch_count: experience
                .repair_projected_hygiene_watch_count,
            experience_repair_projected_hygiene_quarantine_candidate_count: experience
                .repair_projected_hygiene_quarantine_candidate_count,
            experience_repair_projected_legacy_metadata_lesson_count: experience
                .repair_projected_legacy_metadata_lesson_count,
            experience_repair_projected_legacy_metadata_without_clean_gist_count: experience
                .repair_projected_legacy_metadata_without_clean_gist_count,
            experience_repair_skipped_quarantine_candidate_count: experience
                .repair_skipped_quarantine_candidate_count,
            experience_repair_skipped_missing_clean_gist_count: experience
                .repair_skipped_missing_clean_gist_count,
            experience_index_compacted_record_count: experience.index_compacted_record_count,
            experience_index_overlong_record_count: experience.index_overlong_record_count,
            experience_index_overlong_without_clean_gist_count: experience
                .index_overlong_without_clean_gist_count,
            experience_index_max_record_chars: experience.index_max_record_chars,
            experience_index_noisy_record_count: experience.index_noisy_record_count,
            experience_index_duplicate_output_count: experience.index_duplicate_output_count,
            experience_index_max_noise_penalty: experience.index_max_noise_penalty,
            experience_index_quality_score: experience.index_quality_score,
            experience_index_retrieval_ready: experience.index_retrieval_ready,
            experience_index_risk_level: experience.index_risk_level,
            runtime_model_experience_count: counts.runtime_model_experience_count,
            runtime_adapter_experience_count: counts.runtime_adapter_experience_count,
            runtime_adapter_selection_mismatch_count: counts
                .runtime_adapter_selection_mismatch_count,
            runtime_forward_energy_experience_count: counts.runtime_forward_energy_experience_count,
            runtime_kv_influence_experience_count: counts.runtime_kv_influence_experience_count,
            runtime_token_count: counts.runtime_token_count,
            runtime_uncertainty_experience_count: counts.runtime_uncertainty_experience_count,
            runtime_uncertainty_token_count: counts.runtime_uncertainty_token_count,
            runtime_architecture_experience_count: counts.runtime_architecture_experience_count,
            runtime_kv_precision_experience_count: counts.runtime_kv_precision_experience_count,
            runtime_kv_precision_mismatch_count: counts.runtime_kv_precision_mismatch_count,
            runtime_device_execution_experience_count: counts
                .runtime_device_execution_experience_count,
            runtime_layer_mode_experience_count: counts.runtime_layer_mode_experience_count,
            runtime_all_layer_mode_experience_count: counts.runtime_all_layer_mode_experience_count,
            runtime_global_layers: counts.runtime_global_layers,
            runtime_local_window_layers: counts.runtime_local_window_layers,
            runtime_convolutional_fusion_layers: counts.runtime_convolutional_fusion_layers,
            runtime_kv_import_experience_count: counts.runtime_kv_import_experience_count,
            runtime_kv_weak_import_skip_experience_count: counts
                .runtime_kv_weak_import_skip_experience_count,
            weak_runtime_kv_imports_skipped: counts.weak_runtime_kv_imports_skipped,
            runtime_kv_weak_import_pressure_experience_count: counts
                .runtime_kv_weak_import_pressure_experience_count,
            runtime_kv_weak_import_pressure_avg: counts.runtime_kv_weak_import_pressure_avg,
            runtime_kv_weak_import_pressure_max: counts.runtime_kv_weak_import_pressure_max,
            runtime_kv_budget_import_skip_experience_count: counts
                .runtime_kv_budget_import_skip_experience_count,
            budget_limited_runtime_kv_imports_skipped: counts
                .budget_limited_runtime_kv_imports_skipped,
            runtime_kv_budget_pressure_experience_count: counts
                .runtime_kv_budget_pressure_experience_count,
            runtime_kv_budget_pressure_avg: counts.runtime_kv_budget_pressure_avg,
            runtime_kv_budget_pressure_max: counts.runtime_kv_budget_pressure_max,
            runtime_kv_export_experience_count: counts.runtime_kv_export_experience_count,
            runtime_kv_segment_experience_count: counts.runtime_kv_segment_experience_count,
            runtime_kv_segments_included: counts.runtime_kv_segments_included,
            runtime_kv_segments_skipped: counts.runtime_kv_segments_skipped,
            runtime_kv_segments_rejected: counts.runtime_kv_segments_rejected,
            runtime_kv_hold_experience_count: counts.runtime_kv_hold_experience_count,
            runtime_kv_held_blocks: counts.runtime_kv_held_blocks,
            runtime_error_experience_count: counts.runtime_error_experience_count,
            runtime_error_count: counts.runtime_error_count,
            runtime_timeout_experience_count: counts.runtime_timeout_experience_count,
            runtime_timeout_count: counts.runtime_timeout_count,
            runtime_error_message_chars: counts.runtime_error_message_chars,
            reflection_issue_experience_count: counts.reflection_issue_experience_count,
            critical_reflection_issue_experience_count: counts
                .critical_reflection_issue_experience_count,
            revision_action_experience_count: counts.revision_action_experience_count,
            live_memory_feedback_experience_count: counts.live_memory_feedback_experience_count,
            live_memory_feedback_update_count: counts.live_memory_feedback_update_count,
            live_memory_feedback_detail_experience_count: counts
                .live_memory_feedback_detail_experience_count,
            live_memory_feedback_applied_count: counts.live_memory_feedback_applied_count,
            live_memory_feedback_removed_count: counts.live_memory_feedback_removed_count,
            live_memory_feedback_missing_count: counts.live_memory_feedback_missing_count,
            live_memory_feedback_strength_delta: counts.live_memory_feedback_strength_delta,
            rust_check_experience_count: counts.rust_check_experience_count,
            rust_check_passed_count: counts.rust_check_passed_count,
            rust_check_failed_count: counts.rust_check_failed_count,
            rust_check_diagnostic_chars: counts.rust_check_diagnostic_chars,
            business_contract_experience_count: counts.business_contract_experience_count,
            business_contract_passed_count: counts.business_contract_passed_count,
            business_contract_failed_count: counts.business_contract_failed_count,
            business_contract_required_signals: counts.business_contract_required_signals,
            business_contract_matched_signals: counts.business_contract_matched_signals,
            business_contract_missing_signals: counts.business_contract_missing_signals,
            business_contract_protocol_leaks: counts.business_contract_protocol_leaks,
            business_contract_substitutions: counts.business_contract_substitutions,
            business_contract_evasive_denials: counts.business_contract_evasive_denials,
            business_contract_missing_handling_signals: counts
                .business_contract_missing_handling_signals,
            business_contract_raw_passed_count: counts.business_contract_raw_passed_count,
            business_contract_raw_failed_count: counts.business_contract_raw_failed_count,
            business_contract_response_normalized_count: counts
                .business_contract_response_normalized_count,
            business_contract_sanitized_count: counts.business_contract_sanitized_count,
            business_contract_canonical_fallback_count: counts
                .business_contract_canonical_fallback_count,
            pool_dispatch_experience_count: counts.pool_dispatch_experience_count,
            pool_dispatch_item_count: counts.pool_dispatch_item_count,
            pool_dispatch_forwarded_count: counts.pool_dispatch_forwarded_count,
            pool_dispatch_clamped_count: counts.pool_dispatch_clamped_count,
            pool_dispatch_low_priority_count: counts.pool_dispatch_low_priority_count,
            router_threshold: adaptive_state.router.threshold,
            router_observations: adaptive_state.router.observations,
            profile_thresholds: adaptive_state.router.profile_thresholds,
            profile_observations: adaptive_state.router.profile_observations,
            hierarchy: adaptive_state.hierarchy.current,
            profile_hierarchy_weights: adaptive_state.hierarchy.profile_weights,
            profile_hierarchy_observations: adaptive_state.hierarchy.profile_observations,
            tier_counts: adaptive_state.tier_plan.counts(),
            memory_retention_policy: engine.memory_retention_policy,
            memory_compaction_policy: engine.memory_compaction_policy.clone(),
            evolution_ledger: adaptive_state.evolution_ledger,
            genome_profiles: adaptive_state
                .genome_runtime
                .profiles
                .iter()
                .map(|profile| super::StateGenomeProfileSummary {
                    profile: profile.profile,
                    generation: profile.generation,
                    active_genome_id: profile.active.id.clone(),
                    previous_genome_id: profile.previous.as_ref().map(|genome| genome.id.clone()),
                    active_gene_count: profile.active.genes.len(),
                    express_chain_record_count: profile.active_chain.express_chain.len(),
                    memory_chain_record_count: profile.active_chain.memory_chain.len(),
                    dual_chain_consistent: profile.active_chain.genome_id == profile.active.id
                        && profile.active_chain.express_chain.len() == profile.active.genes.len(),
                    journal_record_count: profile.journal_lines.len(),
                })
                .collect(),
            memory_vector_dimensions: memory_vector_dimensions_for_entries(memory_entries),
            runtime_kv_vector_dimensions: runtime_kv_vector_dimensions_for_entries(memory_entries),
            top_memories,
            top_runtime_kv_memories,
            top_experiences: experience.top_experiences,
            experience_hygiene_findings: experience.hygiene_findings,
            experience_index_findings: experience.index_findings,
        }
    }
}

fn experience_inspection_snapshot(
    experience_store: &ExperienceStore,
    limit: usize,
    mode: StateInspectionBuildMode,
) -> ExperienceInspectionSnapshot {
    match mode {
        StateInspectionBuildMode::Full => {
            full_experience_inspection_snapshot(experience_store, limit)
        }
        StateInspectionBuildMode::Online => online_experience_inspection_snapshot(),
    }
}

fn full_experience_inspection_snapshot(
    experience_store: &ExperienceStore,
    limit: usize,
) -> ExperienceInspectionSnapshot {
    let top_experiences = experiences::top_experience_summaries(experience_store.records(), limit);
    let hygiene_report = experience_store.hygiene_report(limit);
    let repair_plan = experience_store.legacy_metadata_repair_plan(limit);
    let index_report = experience_store.index_report(limit);
    let hygiene_findings = hygiene_report
        .findings
        .into_iter()
        .map(|finding| StateExperienceHygieneFinding {
            experience_id: finding.experience_id,
            severity: finding.severity,
            reason: finding.reason,
            markers: finding.markers,
            prompt_preview: finding.prompt_preview,
            lesson_preview: finding.lesson_preview,
        })
        .collect::<Vec<_>>();
    let index_findings = index_report
        .findings
        .into_iter()
        .map(|finding| StateExperienceIndexFinding {
            experience_id: finding.experience_id,
            reason: finding.reason,
            compacted: finding.compacted,
            noise_penalty: finding.noise_penalty,
            duplicate_of: finding.duplicate_of,
            prompt_chars: finding.prompt_chars,
            lesson_chars: finding.lesson_chars,
            prompt_preview: finding.prompt_preview,
            lesson_preview: finding.lesson_preview,
        })
        .collect::<Vec<_>>();

    ExperienceInspectionSnapshot {
        top_experiences,
        hygiene_finding_count: hygiene_report.finding_count,
        hygiene_watch_count: hygiene_report.watch_count,
        hygiene_quarantine_candidate_count: hygiene_report.quarantine_candidate_count,
        hygiene_legacy_metadata_lesson_count: hygiene_report.legacy_metadata_lesson_count,
        hygiene_legacy_metadata_without_clean_gist_count: hygiene_report
            .legacy_metadata_without_clean_gist_count,
        repairable_legacy_metadata_lesson_count: repair_plan
            .repairable_legacy_metadata_lesson_count,
        repairable_index_record_count: repair_plan.repairable_index_record_count,
        repair_projected_hygiene_finding_count: repair_plan
            .projected_after_repair
            .hygiene_finding_count,
        repair_projected_hygiene_watch_count: repair_plan
            .projected_after_repair
            .hygiene_watch_count,
        repair_projected_hygiene_quarantine_candidate_count: repair_plan
            .projected_after_repair
            .hygiene_quarantine_candidate_count,
        repair_projected_legacy_metadata_lesson_count: repair_plan
            .projected_after_repair
            .legacy_metadata_lesson_count,
        repair_projected_legacy_metadata_without_clean_gist_count: repair_plan
            .projected_after_repair
            .legacy_metadata_without_clean_gist_count,
        repair_skipped_quarantine_candidate_count: repair_plan.skipped_quarantine_candidate_count,
        repair_skipped_missing_clean_gist_count: repair_plan.skipped_missing_clean_gist_count,
        index_compacted_record_count: index_report.compacted_record_count,
        index_overlong_record_count: index_report.overlong_record_count,
        index_overlong_without_clean_gist_count: index_report.overlong_without_clean_gist_count,
        index_max_record_chars: index_report.max_record_chars,
        index_noisy_record_count: index_report.noisy_record_count,
        index_duplicate_output_count: index_report.duplicate_output_count,
        index_max_noise_penalty: index_report.max_noise_penalty,
        index_quality_score: index_report.quality_score,
        index_retrieval_ready: index_report.retrieval_ready,
        index_risk_level: index_report.risk_level,
        hygiene_findings,
        index_findings,
    }
}

fn experience_record_has_visible_memory(
    record: &ExperienceRecord,
    visible_memory_ids: &BTreeSet<u64>,
) -> bool {
    record
        .stored_memory_id
        .is_some_and(|id| visible_memory_ids.contains(&id))
        || record
            .used_memory_ids
            .iter()
            .chain(record.gist_memory_ids.iter())
            .chain(record.stored_runtime_kv_memory_ids.iter())
            .any(|id| visible_memory_ids.contains(id))
}

fn online_experience_inspection_snapshot() -> ExperienceInspectionSnapshot {
    ExperienceInspectionSnapshot {
        top_experiences: Vec::new(),
        hygiene_finding_count: 0,
        hygiene_watch_count: 0,
        hygiene_quarantine_candidate_count: 0,
        hygiene_legacy_metadata_lesson_count: 0,
        hygiene_legacy_metadata_without_clean_gist_count: 0,
        repairable_legacy_metadata_lesson_count: 0,
        repairable_index_record_count: 0,
        repair_projected_hygiene_finding_count: 0,
        repair_projected_hygiene_watch_count: 0,
        repair_projected_hygiene_quarantine_candidate_count: 0,
        repair_projected_legacy_metadata_lesson_count: 0,
        repair_projected_legacy_metadata_without_clean_gist_count: 0,
        repair_skipped_quarantine_candidate_count: 0,
        repair_skipped_missing_clean_gist_count: 0,
        index_compacted_record_count: 0,
        index_overlong_record_count: 0,
        index_overlong_without_clean_gist_count: 0,
        index_max_record_chars: 0,
        index_noisy_record_count: 0,
        index_duplicate_output_count: 0,
        index_max_noise_penalty: 0.0,
        index_quality_score: 1.0,
        index_retrieval_ready: true,
        index_risk_level: "online_deferred".to_owned(),
        hygiene_findings: Vec::new(),
        index_findings: Vec::new(),
    }
}
