use crate::engine::NoironEngine;
use crate::experience_replay::{LiveMemoryFeedbackStats, PoolDispatchReplayStats};

use super::super::super::{
    BusinessContractInspectionStats, RuntimeErrorInspectionStats, RustCheckInspectionStats,
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
    pub(super) live_memory_feedback_detail_experience_count: usize,
    pub(super) live_memory_feedback_applied_count: usize,
    pub(super) live_memory_feedback_removed_count: usize,
    pub(super) live_memory_feedback_missing_count: usize,
    pub(super) live_memory_feedback_strength_delta: f32,
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
        live_memory_feedback_detail_experience_count,
        live_memory_feedback_applied_count,
        live_memory_feedback_removed_count,
        live_memory_feedback_missing_count,
        live_memory_feedback_strength_delta,
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
