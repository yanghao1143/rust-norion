use std::cmp::Ordering;

use crate::engine::NoironEngine;
use crate::experience::recursive_runtime_calls_from_notes;
use crate::experience_replay::{LiveMemoryFeedbackStats, PoolDispatchReplayStats};
use crate::hardware::RuntimeAdapterHint;

use super::super::{
    BusinessContractInspectionStats, RuntimeErrorInspectionStats, RustCheckInspectionStats,
    StateExperienceSummary, compact,
};

pub(super) fn top_experience_summaries(
    engine: &NoironEngine,
    limit: usize,
) -> Vec<StateExperienceSummary> {
    let mut top_experiences = engine.experience.records().iter().collect::<Vec<_>>();
    top_experiences.sort_by(|left, right| {
        right
            .process_reward
            .total
            .partial_cmp(&left.process_reward.total)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                right
                    .quality
                    .partial_cmp(&left.quality)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.id.cmp(&right.id))
    });

    top_experiences
        .into_iter()
        .take(limit)
        .map(|record| {
            let live_memory_feedback =
                LiveMemoryFeedbackStats::from_notes(&record.process_reward.notes);
            let rust_check = RustCheckInspectionStats::from_notes(&record.process_reward.notes);
            let business_contract =
                BusinessContractInspectionStats::from_notes(&record.process_reward.notes);
            let runtime_error =
                RuntimeErrorInspectionStats::from_notes(&record.process_reward.notes);
            let pool_dispatch = PoolDispatchReplayStats::from_notes(&record.process_reward.notes);
            StateExperienceSummary {
                id: record.id,
                profile: record.profile,
                quality: record.quality,
                process_reward: record.process_reward.total,
                reward_action: record.process_reward.action,
                runtime_model_id: record.runtime_diagnostics.model_id.clone(),
                runtime_selected_adapter: record
                    .runtime_diagnostics
                    .selected_adapter
                    .as_deref()
                    .and_then(RuntimeAdapterHint::canonical_name)
                    .map(str::to_owned),
                runtime_device_profile: record.runtime_diagnostics.device_profile.clone(),
                runtime_primary_lane: record.runtime_diagnostics.primary_lane.clone(),
                runtime_fallback_lane: record.runtime_diagnostics.fallback_lane.clone(),
                runtime_memory_mode: record.runtime_diagnostics.memory_mode.clone(),
                runtime_layer_count: record.runtime_diagnostics.layer_count,
                runtime_global_layers: record.runtime_diagnostics.global_layers,
                runtime_local_window_layers: record.runtime_diagnostics.local_window_layers,
                runtime_convolutional_fusion_layers: record
                    .runtime_diagnostics
                    .convolutional_fusion_layers,
                runtime_hidden_size: record.runtime_diagnostics.hidden_size,
                runtime_local_window_tokens: record.runtime_diagnostics.local_window_tokens,
                runtime_forward_energy: record.runtime_diagnostics.forward_energy,
                runtime_kv_influence: record.runtime_diagnostics.kv_influence,
                runtime_token_count: record.runtime_token_metrics.token_count,
                runtime_uncertainty_token_count: record
                    .runtime_token_metrics
                    .entropy_count
                    .saturating_add(record.runtime_token_metrics.logprob_count),
                runtime_uncertainty_perplexity: record.runtime_token_metrics.uncertainty_perplexity,
                runtime_hot_kv_precision_bits: record.runtime_diagnostics.hot_kv_precision_bits,
                runtime_cold_kv_precision_bits: record.runtime_diagnostics.cold_kv_precision_bits,
                runtime_imported_kv_blocks: record.runtime_diagnostics.imported_kv_blocks,
                runtime_weak_kv_imports_skipped: record
                    .runtime_diagnostics
                    .weak_runtime_kv_imports_skipped,
                runtime_exported_kv_blocks: record.runtime_diagnostics.exported_kv_blocks,
                runtime_kv_segments_included: record
                    .runtime_diagnostics
                    .runtime_kv_segments_included,
                runtime_kv_segments_skipped: record.runtime_diagnostics.runtime_kv_segments_skipped,
                runtime_kv_segments_rejected: record
                    .runtime_diagnostics
                    .runtime_kv_segments_rejected,
                recursive_runtime_calls: recursive_runtime_calls_from_notes(
                    &record.process_reward.notes,
                ),
                runtime_errors: runtime_error.map(|stats| stats.errors).unwrap_or(0),
                runtime_timeouts: runtime_error.map(|stats| stats.timeouts).unwrap_or(0),
                runtime_error_message_chars: runtime_error
                    .map(|stats| stats.message_chars)
                    .unwrap_or(0),
                live_online_reward_feedbacks: record.live_evolution.online_reward_feedbacks,
                live_online_reward_reinforcements: record
                    .live_evolution
                    .online_reward_reinforcements,
                live_online_reward_penalties: record.live_evolution.online_reward_penalties,
                live_memory_feedback_updates: live_memory_feedback
                    .map(|stats| stats.updates())
                    .unwrap_or(0),
                live_memory_feedback_reinforced: live_memory_feedback
                    .map(|stats| stats.reinforced)
                    .unwrap_or(0),
                live_memory_feedback_penalized: live_memory_feedback
                    .map(|stats| stats.penalized)
                    .unwrap_or(0),
                live_memory_feedback_applied: live_memory_feedback
                    .map(|stats| stats.applied)
                    .unwrap_or(0),
                live_memory_feedback_removed: live_memory_feedback
                    .map(|stats| stats.removed)
                    .unwrap_or(0),
                live_memory_feedback_missing: live_memory_feedback
                    .map(|stats| stats.missing)
                    .unwrap_or(0),
                live_memory_feedback_strength_delta: live_memory_feedback
                    .map(|stats| stats.strength_delta)
                    .unwrap_or(0.0),
                live_memory_feedback_detail: live_memory_feedback
                    .map(|stats| stats.has_detailed_update_evidence())
                    .unwrap_or(false),
                rust_check_passed: rust_check.map(|stats| stats.passed).unwrap_or(0),
                rust_check_failed: rust_check.map(|stats| stats.failed).unwrap_or(0),
                rust_check_diagnostic_chars: rust_check
                    .map(|stats| stats.diagnostic_chars)
                    .unwrap_or(0),
                business_contract_passed: business_contract.map(|stats| stats.passed).unwrap_or(0),
                business_contract_failed: business_contract.map(|stats| stats.failed).unwrap_or(0),
                business_contract_missing_signals: business_contract
                    .map(|stats| stats.missing_signals)
                    .unwrap_or(0),
                business_contract_protocol_leaks: business_contract
                    .map(|stats| stats.protocol_leaks)
                    .unwrap_or(0),
                business_contract_substitutions: business_contract
                    .map(|stats| stats.substitutions)
                    .unwrap_or(0),
                business_contract_evasive_denials: business_contract
                    .map(|stats| stats.evasive_denials)
                    .unwrap_or(0),
                business_contract_missing_handling_signals: business_contract
                    .map(|stats| stats.missing_handling_signals)
                    .unwrap_or(0),
                business_contract_raw_passed: business_contract
                    .map(|stats| stats.raw_passed)
                    .unwrap_or(0),
                business_contract_raw_failed: business_contract
                    .map(|stats| stats.raw_failed)
                    .unwrap_or(0),
                business_contract_response_normalized: business_contract
                    .map(|stats| stats.response_normalized)
                    .unwrap_or(0),
                business_contract_sanitized: business_contract
                    .map(|stats| stats.sanitized)
                    .unwrap_or(0),
                business_contract_canonical_fallbacks: business_contract
                    .map(|stats| stats.canonical_fallbacks)
                    .unwrap_or(0),
                pool_dispatch_items: pool_dispatch.as_ref().map(|stats| stats.items).unwrap_or(0),
                pool_dispatch_selected_roles: pool_dispatch
                    .as_ref()
                    .map(|stats| stats.selected_roles.clone())
                    .unwrap_or_default(),
                pool_dispatch_forwarded: pool_dispatch
                    .as_ref()
                    .map(|stats| stats.forwarded)
                    .unwrap_or(0),
                pool_dispatch_clamped: pool_dispatch
                    .as_ref()
                    .map(|stats| stats.clamped)
                    .unwrap_or(0),
                pool_dispatch_low_priority: pool_dispatch
                    .as_ref()
                    .map(|stats| stats.low_priority)
                    .unwrap_or(0),
                reflection_issues: record.reflection_issues.len(),
                critical_reflection_issues: record
                    .reflection_issues
                    .iter()
                    .filter(|issue| {
                        issue.severity == crate::reflection::ReflectionSeverity::Critical
                    })
                    .count(),
                revision_actions: record.revision_actions.len(),
                lesson: compact(&record.lesson, 160),
            }
        })
        .collect::<Vec<_>>()
}
