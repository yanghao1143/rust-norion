use crate::engine::NoironEngine;
use crate::experience_replay::{LiveMemoryFeedbackStats, PoolDispatchReplayStats};
use crate::process_reward::RewardAction;

use super::super::super::{
    BusinessContractInspectionStats, ExternalSemanticContextInspectionStats,
    RuntimeErrorInspectionStats, RustCheckInspectionStats,
    SelfEvolvingMemoryWritebackInspectionStats,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct QualitySignalCounts {
    pub(super) runtime_error_experience_count: usize,
    pub(super) runtime_error_count: usize,
    pub(super) runtime_timeout_experience_count: usize,
    pub(super) runtime_timeout_count: usize,
    pub(super) runtime_error_message_chars: usize,
    pub(super) reflection_issue_experience_count: usize,
    pub(super) critical_reflection_issue_experience_count: usize,
    pub(super) revision_action_experience_count: usize,
    pub(super) live_memory_feedback_experience_count: usize,
    pub(super) live_memory_feedback_update_count: usize,
    pub(super) live_memory_feedback_reinforced_count: usize,
    pub(super) live_memory_feedback_penalized_count: usize,
    pub(super) live_memory_feedback_detail_experience_count: usize,
    pub(super) live_memory_feedback_applied_count: usize,
    pub(super) live_memory_feedback_removed_count: usize,
    pub(super) live_memory_feedback_missing_count: usize,
    pub(super) live_memory_feedback_strength_delta: f32,
    pub(super) process_reward_experience_count: usize,
    pub(super) process_reward_positive_count: usize,
    pub(super) process_reward_reinforce_count: usize,
    pub(super) process_reward_hold_count: usize,
    pub(super) process_reward_penalize_count: usize,
    pub(super) process_reward_total: f32,
    pub(super) external_semantic_context_experience_count: usize,
    pub(super) external_semantic_context_count: usize,
    pub(super) self_evolving_memory_writeback_experience_count: usize,
    pub(super) self_evolving_memory_writeback_attempted_records: usize,
    pub(super) self_evolving_memory_writeback_accepted_records: usize,
    pub(super) self_evolving_memory_writeback_records_before: usize,
    pub(super) self_evolving_memory_writeback_records_after: usize,
    pub(super) self_evolving_memory_writeback_tool_reliability_after: usize,
    pub(super) self_evolving_memory_writeback_tool_observations_after: usize,
    pub(super) self_evolving_memory_writeback_maintenance_actions: usize,
    pub(super) self_evolving_memory_writeback_merged_duplicate_episodes: usize,
    pub(super) self_evolving_memory_writeback_write_allowed: usize,
    pub(super) self_evolving_memory_writeback_durable_write_allowed: usize,
    pub(super) self_evolving_memory_writeback_applied: usize,
    pub(super) self_evolving_memory_writeback_applied_to_disk: usize,
    pub(super) self_evolving_memory_writeback_snapshot_changes: usize,
    pub(super) rust_check_experience_count: usize,
    pub(super) rust_check_passed_count: usize,
    pub(super) rust_check_failed_count: usize,
    pub(super) rust_check_diagnostic_chars: usize,
    pub(super) business_contract_experience_count: usize,
    pub(super) business_contract_passed_count: usize,
    pub(super) business_contract_failed_count: usize,
    pub(super) business_contract_required_signals: usize,
    pub(super) business_contract_matched_signals: usize,
    pub(super) business_contract_missing_signals: usize,
    pub(super) business_contract_protocol_leaks: usize,
    pub(super) business_contract_substitutions: usize,
    pub(super) business_contract_evasive_denials: usize,
    pub(super) business_contract_missing_handling_signals: usize,
    pub(super) business_contract_raw_passed_count: usize,
    pub(super) business_contract_raw_failed_count: usize,
    pub(super) business_contract_response_normalized_count: usize,
    pub(super) business_contract_sanitized_count: usize,
    pub(super) business_contract_canonical_fallback_count: usize,
    pub(super) pool_dispatch_experience_count: usize,
    pub(super) pool_dispatch_item_count: usize,
    pub(super) pool_dispatch_forwarded_count: usize,
    pub(super) pool_dispatch_clamped_count: usize,
    pub(super) pool_dispatch_low_priority_count: usize,
}

pub(super) fn quality_signal_counts(engine: &NoironEngine) -> QualitySignalCounts {
    let runtime_error_stats = engine
        .experience
        .records()
        .iter()
        .filter_map(|record| RuntimeErrorInspectionStats::from_notes(&record.process_reward.notes))
        .collect::<Vec<_>>();
    let runtime_error_experience_count = runtime_error_stats.len();
    let runtime_error_count = runtime_error_stats
        .iter()
        .map(|stats| stats.errors)
        .sum::<usize>();
    let runtime_timeout_experience_count = runtime_error_stats
        .iter()
        .filter(|stats| stats.timeouts > 0)
        .count();
    let runtime_timeout_count = runtime_error_stats
        .iter()
        .map(|stats| stats.timeouts)
        .sum::<usize>();
    let runtime_error_message_chars = runtime_error_stats
        .iter()
        .map(|stats| stats.message_chars)
        .sum::<usize>();
    let reflection_issue_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| !record.reflection_issues.is_empty())
        .count();
    let critical_reflection_issue_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| {
            record
                .reflection_issues
                .iter()
                .any(|issue| issue.severity == crate::reflection::ReflectionSeverity::Critical)
        })
        .count();
    let revision_action_experience_count = engine
        .experience
        .records()
        .iter()
        .filter(|record| !record.revision_actions.is_empty())
        .count();
    let live_memory_feedback_stats = engine
        .experience
        .records()
        .iter()
        .filter_map(|record| LiveMemoryFeedbackStats::from_notes(&record.process_reward.notes))
        .collect::<Vec<_>>();
    let live_memory_feedback_experience_count = live_memory_feedback_stats.len();
    let live_memory_feedback_update_count = live_memory_feedback_stats
        .iter()
        .map(LiveMemoryFeedbackStats::updates)
        .sum::<usize>();
    let live_memory_feedback_reinforced_count = live_memory_feedback_stats
        .iter()
        .map(|stats| stats.reinforced)
        .sum::<usize>();
    let live_memory_feedback_penalized_count = live_memory_feedback_stats
        .iter()
        .map(|stats| stats.penalized)
        .sum::<usize>();
    let live_memory_feedback_detail_experience_count = live_memory_feedback_stats
        .iter()
        .filter(|stats| stats.has_detailed_update_evidence())
        .count();
    let live_memory_feedback_applied_count = live_memory_feedback_stats
        .iter()
        .map(|stats| stats.applied)
        .sum::<usize>();
    let live_memory_feedback_removed_count = live_memory_feedback_stats
        .iter()
        .map(|stats| stats.removed)
        .sum::<usize>();
    let live_memory_feedback_missing_count = live_memory_feedback_stats
        .iter()
        .map(|stats| stats.missing)
        .sum::<usize>();
    let live_memory_feedback_strength_delta = live_memory_feedback_stats
        .iter()
        .map(|stats| stats.strength_delta)
        .sum::<f32>();
    let mut process_reward_experience_count = 0;
    let mut process_reward_positive_count = 0;
    let mut process_reward_reinforce_count = 0;
    let mut process_reward_hold_count = 0;
    let mut process_reward_penalize_count = 0;
    let mut process_reward_total = 0.0;
    for record in engine.experience.records() {
        process_reward_experience_count += 1;
        process_reward_total += record.process_reward.total;
        process_reward_positive_count += usize::from(record.process_reward.total > 0.0);
        match record.process_reward.action {
            RewardAction::Reinforce => process_reward_reinforce_count += 1,
            RewardAction::Hold => process_reward_hold_count += 1,
            RewardAction::Penalize => process_reward_penalize_count += 1,
        }
    }
    let external_semantic_context_stats = engine
        .experience
        .records()
        .iter()
        .filter_map(|record| {
            ExternalSemanticContextInspectionStats::from_notes(&record.process_reward.notes)
        })
        .collect::<Vec<_>>();
    let external_semantic_context_experience_count = external_semantic_context_stats.len();
    let external_semantic_context_count = external_semantic_context_stats
        .iter()
        .map(|stats| stats.contexts)
        .sum::<usize>();
    let self_evolving_memory_writeback_stats = engine
        .experience
        .records()
        .iter()
        .filter_map(|record| {
            SelfEvolvingMemoryWritebackInspectionStats::from_notes(&record.process_reward.notes)
        })
        .collect::<Vec<_>>();
    let self_evolving_memory_writeback_experience_count =
        self_evolving_memory_writeback_stats.len();
    let self_evolving_memory_writeback_attempted_records = self_evolving_memory_writeback_stats
        .iter()
        .map(|stats| stats.attempted_records)
        .sum::<usize>();
    let self_evolving_memory_writeback_accepted_records = self_evolving_memory_writeback_stats
        .iter()
        .map(|stats| stats.accepted_records)
        .sum::<usize>();
    let self_evolving_memory_writeback_records_before = self_evolving_memory_writeback_stats
        .iter()
        .map(|stats| stats.records_before)
        .sum::<usize>();
    let self_evolving_memory_writeback_records_after = self_evolving_memory_writeback_stats
        .iter()
        .map(|stats| stats.records_after)
        .sum::<usize>();
    let self_evolving_memory_writeback_tool_reliability_after =
        self_evolving_memory_writeback_stats
            .iter()
            .map(|stats| stats.tool_reliability_after)
            .sum::<usize>();
    let self_evolving_memory_writeback_tool_observations_after =
        self_evolving_memory_writeback_stats
            .iter()
            .map(|stats| stats.tool_observations_after)
            .sum::<usize>();
    let self_evolving_memory_writeback_maintenance_actions = self_evolving_memory_writeback_stats
        .iter()
        .map(|stats| stats.maintenance_actions)
        .sum::<usize>();
    let self_evolving_memory_writeback_merged_duplicate_episodes =
        self_evolving_memory_writeback_stats
            .iter()
            .map(|stats| stats.merged_duplicate_episodes)
            .sum::<usize>();
    let self_evolving_memory_writeback_write_allowed = self_evolving_memory_writeback_stats
        .iter()
        .map(|stats| stats.write_allowed)
        .sum::<usize>();
    let self_evolving_memory_writeback_durable_write_allowed = self_evolving_memory_writeback_stats
        .iter()
        .map(|stats| stats.durable_write_allowed)
        .sum::<usize>();
    let self_evolving_memory_writeback_applied = self_evolving_memory_writeback_stats
        .iter()
        .map(|stats| stats.applied)
        .sum::<usize>();
    let self_evolving_memory_writeback_applied_to_disk = self_evolving_memory_writeback_stats
        .iter()
        .map(|stats| stats.applied_to_disk)
        .sum::<usize>();
    let self_evolving_memory_writeback_snapshot_changes = self_evolving_memory_writeback_stats
        .iter()
        .map(|stats| stats.snapshot_changes)
        .sum::<usize>();
    let rust_check_stats = engine
        .experience
        .records()
        .iter()
        .filter_map(|record| RustCheckInspectionStats::from_notes(&record.process_reward.notes))
        .collect::<Vec<_>>();
    let rust_check_experience_count = rust_check_stats.len();
    let rust_check_passed_count = rust_check_stats
        .iter()
        .map(|stats| stats.passed)
        .sum::<usize>();
    let rust_check_failed_count = rust_check_stats
        .iter()
        .map(|stats| stats.failed)
        .sum::<usize>();
    let rust_check_diagnostic_chars = rust_check_stats
        .iter()
        .map(|stats| stats.diagnostic_chars)
        .sum::<usize>();
    let business_contract_stats = engine
        .experience
        .records()
        .iter()
        .filter_map(|record| {
            BusinessContractInspectionStats::from_notes(&record.process_reward.notes)
        })
        .collect::<Vec<_>>();
    let business_contract_experience_count = business_contract_stats.len();
    let business_contract_passed_count = business_contract_stats
        .iter()
        .map(|stats| stats.passed)
        .sum::<usize>();
    let business_contract_failed_count = business_contract_stats
        .iter()
        .map(|stats| stats.failed)
        .sum::<usize>();
    let business_contract_required_signals = business_contract_stats
        .iter()
        .map(|stats| stats.required_signals)
        .sum::<usize>();
    let business_contract_matched_signals = business_contract_stats
        .iter()
        .map(|stats| stats.matched_signals)
        .sum::<usize>();
    let business_contract_missing_signals = business_contract_stats
        .iter()
        .map(|stats| stats.missing_signals)
        .sum::<usize>();
    let business_contract_protocol_leaks = business_contract_stats
        .iter()
        .map(|stats| stats.protocol_leaks)
        .sum::<usize>();
    let business_contract_substitutions = business_contract_stats
        .iter()
        .map(|stats| stats.substitutions)
        .sum::<usize>();
    let business_contract_evasive_denials = business_contract_stats
        .iter()
        .map(|stats| stats.evasive_denials)
        .sum::<usize>();
    let business_contract_missing_handling_signals = business_contract_stats
        .iter()
        .map(|stats| stats.missing_handling_signals)
        .sum::<usize>();
    let business_contract_raw_passed_count = business_contract_stats
        .iter()
        .map(|stats| stats.raw_passed)
        .sum::<usize>();
    let business_contract_raw_failed_count = business_contract_stats
        .iter()
        .map(|stats| stats.raw_failed)
        .sum::<usize>();
    let business_contract_response_normalized_count = business_contract_stats
        .iter()
        .map(|stats| stats.response_normalized)
        .sum::<usize>();
    let business_contract_sanitized_count = business_contract_stats
        .iter()
        .map(|stats| stats.sanitized)
        .sum::<usize>();
    let business_contract_canonical_fallback_count = business_contract_stats
        .iter()
        .map(|stats| stats.canonical_fallbacks)
        .sum::<usize>();
    let pool_dispatch_stats = engine
        .experience
        .records()
        .iter()
        .filter_map(|record| PoolDispatchReplayStats::from_notes(&record.process_reward.notes))
        .collect::<Vec<_>>();
    let pool_dispatch_experience_count = pool_dispatch_stats.len();
    let pool_dispatch_item_count = pool_dispatch_stats
        .iter()
        .map(|stats| stats.items)
        .sum::<usize>();
    let pool_dispatch_forwarded_count = pool_dispatch_stats
        .iter()
        .map(|stats| stats.forwarded)
        .sum::<usize>();
    let pool_dispatch_clamped_count = pool_dispatch_stats
        .iter()
        .map(|stats| stats.clamped)
        .sum::<usize>();
    let pool_dispatch_low_priority_count = pool_dispatch_stats
        .iter()
        .map(|stats| stats.low_priority)
        .sum::<usize>();

    QualitySignalCounts {
        runtime_error_experience_count,
        runtime_error_count,
        runtime_timeout_experience_count,
        runtime_timeout_count,
        runtime_error_message_chars,
        reflection_issue_experience_count,
        critical_reflection_issue_experience_count,
        revision_action_experience_count,
        live_memory_feedback_experience_count,
        live_memory_feedback_update_count,
        live_memory_feedback_reinforced_count,
        live_memory_feedback_penalized_count,
        live_memory_feedback_detail_experience_count,
        live_memory_feedback_applied_count,
        live_memory_feedback_removed_count,
        live_memory_feedback_missing_count,
        live_memory_feedback_strength_delta,
        process_reward_experience_count,
        process_reward_positive_count,
        process_reward_reinforce_count,
        process_reward_hold_count,
        process_reward_penalize_count,
        process_reward_total,
        external_semantic_context_experience_count,
        external_semantic_context_count,
        self_evolving_memory_writeback_experience_count,
        self_evolving_memory_writeback_attempted_records,
        self_evolving_memory_writeback_accepted_records,
        self_evolving_memory_writeback_records_before,
        self_evolving_memory_writeback_records_after,
        self_evolving_memory_writeback_tool_reliability_after,
        self_evolving_memory_writeback_tool_observations_after,
        self_evolving_memory_writeback_maintenance_actions,
        self_evolving_memory_writeback_merged_duplicate_episodes,
        self_evolving_memory_writeback_write_allowed,
        self_evolving_memory_writeback_durable_write_allowed,
        self_evolving_memory_writeback_applied,
        self_evolving_memory_writeback_applied_to_disk,
        self_evolving_memory_writeback_snapshot_changes,
        rust_check_experience_count,
        rust_check_passed_count,
        rust_check_failed_count,
        rust_check_diagnostic_chars,
        business_contract_experience_count,
        business_contract_passed_count,
        business_contract_failed_count,
        business_contract_required_signals,
        business_contract_matched_signals,
        business_contract_missing_signals,
        business_contract_protocol_leaks,
        business_contract_substitutions,
        business_contract_evasive_denials,
        business_contract_missing_handling_signals,
        business_contract_raw_passed_count,
        business_contract_raw_failed_count,
        business_contract_response_normalized_count,
        business_contract_sanitized_count,
        business_contract_canonical_fallback_count,
        pool_dispatch_experience_count,
        pool_dispatch_item_count,
        pool_dispatch_forwarded_count,
        pool_dispatch_clamped_count,
        pool_dispatch_low_priority_count,
    }
}
