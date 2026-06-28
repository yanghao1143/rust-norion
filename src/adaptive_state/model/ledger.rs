use crate::experience_replay::ExperienceReplayReport;
use crate::kv_cache::{MemoryUpdateAction, MemoryUpdateReport};

use super::{nonnegative_f32, LiveInferenceEvolution};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EvolutionLedger {
    pub live_inference_runs: u64,
    pub live_router_threshold_mutations: u64,
    pub live_hierarchy_weight_mutations: u64,
    pub live_router_threshold_delta: f32,
    pub live_hierarchy_weight_delta: f32,
    pub live_online_reward_feedbacks: u64,
    pub live_online_reward_reinforcements: u64,
    pub live_online_reward_penalties: u64,
    pub live_online_reward_strength: f32,
    pub live_online_reward_reinforcement_strength: f32,
    pub live_online_reward_penalty_strength: f32,
    pub live_memory_reinforcements: u64,
    pub live_memory_penalties: u64,
    pub live_stored_memories: u64,
    pub live_stored_gist_memories: u64,
    pub live_stored_runtime_kv_memories: u64,
    pub live_reflection_issues: u64,
    pub live_critical_reflection_issues: u64,
    pub live_revision_actions: u64,
    pub replay_runs: u64,
    pub replay_items: u64,
    pub router_threshold_mutations: u64,
    pub hierarchy_weight_mutations: u64,
    pub router_threshold_delta: f32,
    pub hierarchy_weight_delta: f32,
    pub memory_reinforcements: u64,
    pub memory_penalties: u64,
    pub replay_live_memory_feedback_items: u64,
    pub replay_live_memory_feedback_reinforcements: u64,
    pub replay_live_memory_feedback_penalties: u64,
    pub replay_live_memory_feedback_detail_items: u64,
    pub replay_live_memory_feedback_applied: u64,
    pub replay_live_memory_feedback_removed: u64,
    pub replay_live_memory_feedback_missing: u64,
    pub replay_live_memory_feedback_strength_delta: f32,
    pub replay_rust_check_items: u64,
    pub replay_rust_check_passed: u64,
    pub replay_rust_check_failed: u64,
    pub replay_rust_check_diagnostic_chars: u64,
    pub replay_rust_check_live_memory_feedback_items: u64,
    pub replay_rust_check_live_memory_feedback_updates: u64,
    pub replay_rust_check_live_memory_feedback_applied: u64,
    pub replay_rust_check_live_memory_feedback_strength_delta: f32,
    pub replay_business_contract_items: u64,
    pub replay_business_contract_passed: u64,
    pub replay_business_contract_failed: u64,
    pub replay_business_contract_raw_passed: u64,
    pub replay_business_contract_raw_failed: u64,
    pub replay_business_contract_response_normalized: u64,
    pub replay_business_contract_sanitized: u64,
    pub replay_business_contract_canonical_fallbacks: u64,
    pub replay_live_evolution_items: u64,
    pub replay_live_evolution_router_threshold_mutations: u64,
    pub replay_live_evolution_hierarchy_weight_mutations: u64,
    pub replay_live_evolution_router_threshold_delta: f32,
    pub replay_live_evolution_hierarchy_weight_delta: f32,
    pub replay_live_evolution_online_reward_feedbacks: u64,
    pub replay_live_evolution_online_reward_reinforcements: u64,
    pub replay_live_evolution_online_reward_penalties: u64,
    pub replay_live_evolution_online_reward_strength: f32,
    pub replay_live_evolution_online_reward_reinforcement_strength: f32,
    pub replay_live_evolution_online_reward_penalty_strength: f32,
    pub replay_live_evolution_memory_updates: u64,
    pub replay_live_evolution_stored_memory_updates: u64,
    pub replay_live_evolution_reflection_issues: u64,
    pub replay_live_evolution_critical_reflection_issues: u64,
    pub replay_live_evolution_revision_actions: u64,
    pub recursive_replay_items: u64,
    pub recursive_runtime_calls: u64,
    pub drift_rollbacks: u64,
    pub rollback_router_threshold_delta: f32,
    pub rollback_hierarchy_weight_delta: f32,
    pub external_feedbacks: u64,
    pub external_feedback_reinforcements: u64,
    pub external_feedback_penalties: u64,
    pub external_feedback_memory_updates: u64,
    pub external_feedback_removed: u64,
    pub external_feedback_missing: u64,
    pub external_feedback_strength_delta: f32,
}
impl EvolutionLedger {
    pub fn record_live_inference(&mut self, report: LiveInferenceEvolution) {
        self.live_inference_runs = self.live_inference_runs.saturating_add(1);
        if report.router_threshold_delta > 0.000001 {
            self.live_router_threshold_mutations =
                self.live_router_threshold_mutations.saturating_add(1);
            self.live_router_threshold_delta += report.router_threshold_delta;
        }
        if report.hierarchy_weight_delta > 0.000001 {
            self.live_hierarchy_weight_mutations =
                self.live_hierarchy_weight_mutations.saturating_add(1);
            self.live_hierarchy_weight_delta += report.hierarchy_weight_delta;
        }
        self.live_online_reward_feedbacks = self
            .live_online_reward_feedbacks
            .saturating_add(report.online_reward_feedbacks as u64);
        self.live_online_reward_reinforcements = self
            .live_online_reward_reinforcements
            .saturating_add(report.online_reward_reinforcements as u64);
        self.live_online_reward_penalties = self
            .live_online_reward_penalties
            .saturating_add(report.online_reward_penalties as u64);
        self.live_online_reward_strength += nonnegative_f32(report.online_reward_strength);
        self.live_online_reward_reinforcement_strength +=
            nonnegative_f32(report.online_reward_reinforcement_strength);
        self.live_online_reward_penalty_strength +=
            nonnegative_f32(report.online_reward_penalty_strength);
        self.live_memory_reinforcements = self
            .live_memory_reinforcements
            .saturating_add(report.memory_reinforcements as u64);
        self.live_memory_penalties = self
            .live_memory_penalties
            .saturating_add(report.memory_penalties as u64);
        self.live_stored_memories = self
            .live_stored_memories
            .saturating_add(u64::from(report.stored_memory));
        self.live_stored_gist_memories = self
            .live_stored_gist_memories
            .saturating_add(report.stored_gist_memories as u64);
        self.live_stored_runtime_kv_memories = self
            .live_stored_runtime_kv_memories
            .saturating_add(report.stored_runtime_kv_memories as u64);
        self.live_reflection_issues = self
            .live_reflection_issues
            .saturating_add(report.reflection_issues as u64);
        self.live_critical_reflection_issues = self
            .live_critical_reflection_issues
            .saturating_add(report.critical_reflection_issues as u64);
        self.live_revision_actions = self
            .live_revision_actions
            .saturating_add(report.revision_actions as u64);
    }

    pub fn record_replay(&mut self, report: &ExperienceReplayReport) {
        if report.applied == 0 {
            return;
        }

        self.replay_runs = self.replay_runs.saturating_add(1);
        self.replay_items = self.replay_items.saturating_add(report.applied as u64);
        self.router_threshold_mutations = self
            .router_threshold_mutations
            .saturating_add(report.router_threshold_mutations as u64);
        self.hierarchy_weight_mutations = self
            .hierarchy_weight_mutations
            .saturating_add(report.hierarchy_weight_mutations as u64);
        self.router_threshold_delta += report.router_threshold_delta;
        self.hierarchy_weight_delta += report.hierarchy_weight_delta;
        self.memory_reinforcements = self
            .memory_reinforcements
            .saturating_add(report.memory_reinforcements as u64);
        self.memory_penalties = self
            .memory_penalties
            .saturating_add(report.memory_penalties as u64);
        self.replay_live_memory_feedback_items = self
            .replay_live_memory_feedback_items
            .saturating_add(report.live_memory_feedback_items as u64);
        self.replay_live_memory_feedback_reinforcements = self
            .replay_live_memory_feedback_reinforcements
            .saturating_add(report.live_memory_feedback_reinforcements as u64);
        self.replay_live_memory_feedback_penalties = self
            .replay_live_memory_feedback_penalties
            .saturating_add(report.live_memory_feedback_penalties as u64);
        self.replay_live_memory_feedback_detail_items = self
            .replay_live_memory_feedback_detail_items
            .saturating_add(report.live_memory_feedback_detail_items as u64);
        self.replay_live_memory_feedback_applied = self
            .replay_live_memory_feedback_applied
            .saturating_add(report.live_memory_feedback_applied as u64);
        self.replay_live_memory_feedback_removed = self
            .replay_live_memory_feedback_removed
            .saturating_add(report.live_memory_feedback_removed as u64);
        self.replay_live_memory_feedback_missing = self
            .replay_live_memory_feedback_missing
            .saturating_add(report.live_memory_feedback_missing as u64);
        self.replay_live_memory_feedback_strength_delta +=
            report.live_memory_feedback_strength_delta.max(0.0);
        self.replay_rust_check_items = self
            .replay_rust_check_items
            .saturating_add(report.rust_check_items as u64);
        self.replay_rust_check_passed = self
            .replay_rust_check_passed
            .saturating_add(report.rust_check_passed as u64);
        self.replay_rust_check_failed = self
            .replay_rust_check_failed
            .saturating_add(report.rust_check_failed as u64);
        self.replay_rust_check_diagnostic_chars = self
            .replay_rust_check_diagnostic_chars
            .saturating_add(report.rust_check_diagnostic_chars as u64);
        self.replay_rust_check_live_memory_feedback_items = self
            .replay_rust_check_live_memory_feedback_items
            .saturating_add(report.rust_check_live_memory_feedback_items as u64);
        self.replay_rust_check_live_memory_feedback_updates = self
            .replay_rust_check_live_memory_feedback_updates
            .saturating_add(report.rust_check_live_memory_feedback_updates as u64);
        self.replay_rust_check_live_memory_feedback_applied = self
            .replay_rust_check_live_memory_feedback_applied
            .saturating_add(report.rust_check_live_memory_feedback_applied as u64);
        self.replay_rust_check_live_memory_feedback_strength_delta += report
            .rust_check_live_memory_feedback_strength_delta
            .max(0.0);
        self.replay_business_contract_items = self
            .replay_business_contract_items
            .saturating_add(report.business_contract_items as u64);
        self.replay_business_contract_passed = self
            .replay_business_contract_passed
            .saturating_add(report.business_contract_passed as u64);
        self.replay_business_contract_failed = self
            .replay_business_contract_failed
            .saturating_add(report.business_contract_failed as u64);
        self.replay_business_contract_raw_passed = self
            .replay_business_contract_raw_passed
            .saturating_add(report.business_contract_raw_passed as u64);
        self.replay_business_contract_raw_failed = self
            .replay_business_contract_raw_failed
            .saturating_add(report.business_contract_raw_failed as u64);
        self.replay_business_contract_response_normalized = self
            .replay_business_contract_response_normalized
            .saturating_add(report.business_contract_response_normalized as u64);
        self.replay_business_contract_sanitized = self
            .replay_business_contract_sanitized
            .saturating_add(report.business_contract_sanitized as u64);
        self.replay_business_contract_canonical_fallbacks = self
            .replay_business_contract_canonical_fallbacks
            .saturating_add(report.business_contract_canonical_fallbacks as u64);
        self.replay_live_evolution_items = self
            .replay_live_evolution_items
            .saturating_add(report.live_evolution_items as u64);
        self.replay_live_evolution_router_threshold_mutations = self
            .replay_live_evolution_router_threshold_mutations
            .saturating_add(report.live_evolution_router_threshold_mutations as u64);
        self.replay_live_evolution_hierarchy_weight_mutations = self
            .replay_live_evolution_hierarchy_weight_mutations
            .saturating_add(report.live_evolution_hierarchy_weight_mutations as u64);
        self.replay_live_evolution_router_threshold_delta +=
            report.live_evolution_router_threshold_delta.max(0.0);
        self.replay_live_evolution_hierarchy_weight_delta +=
            report.live_evolution_hierarchy_weight_delta.max(0.0);
        self.replay_live_evolution_online_reward_feedbacks = self
            .replay_live_evolution_online_reward_feedbacks
            .saturating_add(report.live_evolution_online_reward_feedbacks as u64);
        self.replay_live_evolution_online_reward_reinforcements = self
            .replay_live_evolution_online_reward_reinforcements
            .saturating_add(report.live_evolution_online_reward_reinforcements as u64);
        self.replay_live_evolution_online_reward_penalties = self
            .replay_live_evolution_online_reward_penalties
            .saturating_add(report.live_evolution_online_reward_penalties as u64);
        self.replay_live_evolution_online_reward_strength +=
            nonnegative_f32(report.live_evolution_online_reward_strength);
        self.replay_live_evolution_online_reward_reinforcement_strength +=
            nonnegative_f32(report.live_evolution_online_reward_reinforcement_strength);
        self.replay_live_evolution_online_reward_penalty_strength +=
            nonnegative_f32(report.live_evolution_online_reward_penalty_strength);
        self.replay_live_evolution_memory_updates = self
            .replay_live_evolution_memory_updates
            .saturating_add(report.live_evolution_memory_updates as u64);
        self.replay_live_evolution_stored_memory_updates = self
            .replay_live_evolution_stored_memory_updates
            .saturating_add(report.live_evolution_stored_memory_updates as u64);
        self.replay_live_evolution_reflection_issues = self
            .replay_live_evolution_reflection_issues
            .saturating_add(report.live_evolution_reflection_issues as u64);
        self.replay_live_evolution_critical_reflection_issues = self
            .replay_live_evolution_critical_reflection_issues
            .saturating_add(report.live_evolution_critical_reflection_issues as u64);
        self.replay_live_evolution_revision_actions = self
            .replay_live_evolution_revision_actions
            .saturating_add(report.live_evolution_revision_actions as u64);
        self.recursive_replay_items = self
            .recursive_replay_items
            .saturating_add(report.recursive_runtime_items as u64);
        self.recursive_runtime_calls = self
            .recursive_runtime_calls
            .saturating_add(report.recursive_runtime_calls as u64);
    }

    pub fn memory_updates(self) -> u64 {
        self.memory_reinforcements
            .saturating_add(self.memory_penalties)
    }

    pub fn live_memory_updates(self) -> u64 {
        self.live_memory_reinforcements
            .saturating_add(self.live_memory_penalties)
    }

    pub fn live_stored_memory_updates(self) -> u64 {
        self.live_stored_memories
            .saturating_add(self.live_stored_gist_memories)
            .saturating_add(self.live_stored_runtime_kv_memories)
    }

    pub fn replay_live_memory_feedback_updates(self) -> u64 {
        self.replay_live_memory_feedback_reinforcements
            .saturating_add(self.replay_live_memory_feedback_penalties)
    }

    pub fn replay_live_memory_feedback_detail_updates(self) -> u64 {
        self.replay_live_memory_feedback_applied
            .saturating_add(self.replay_live_memory_feedback_missing)
    }

    pub fn replay_rust_check_total(self) -> u64 {
        self.replay_rust_check_passed
            .saturating_add(self.replay_rust_check_failed)
    }

    pub fn replay_business_contract_total(self) -> u64 {
        self.replay_business_contract_passed
            .saturating_add(self.replay_business_contract_failed)
    }

    pub fn replay_business_contract_raw_audits(self) -> u64 {
        self.replay_business_contract_raw_passed
            .saturating_add(self.replay_business_contract_raw_failed)
    }

    pub fn record_drift_rollback(
        &mut self,
        router_threshold_delta: f32,
        hierarchy_weight_delta: f32,
    ) {
        self.drift_rollbacks = self.drift_rollbacks.saturating_add(1);
        self.rollback_router_threshold_delta += router_threshold_delta.max(0.0);
        self.rollback_hierarchy_weight_delta += hierarchy_weight_delta.max(0.0);
    }

    pub fn record_external_feedback(&mut self, updates: &[MemoryUpdateReport]) {
        if updates.is_empty() {
            return;
        }

        self.external_feedbacks = self.external_feedbacks.saturating_add(1);
        for update in updates {
            if update.was_applied() {
                self.external_feedback_memory_updates =
                    self.external_feedback_memory_updates.saturating_add(1);
                match update.action {
                    MemoryUpdateAction::Reinforce => {
                        self.external_feedback_reinforcements =
                            self.external_feedback_reinforcements.saturating_add(1);
                    }
                    MemoryUpdateAction::Penalize => {
                        self.external_feedback_penalties =
                            self.external_feedback_penalties.saturating_add(1);
                    }
                }
            } else {
                self.external_feedback_missing = self.external_feedback_missing.saturating_add(1);
            }
            self.external_feedback_removed = self
                .external_feedback_removed
                .saturating_add(u64::from(update.removed));
            self.external_feedback_strength_delta += nonnegative_f32(update.strength_delta.abs());
        }
    }

    pub fn summary_line(self) -> String {
        format!(
            "evolution: live_inference_runs={} live_router_threshold_mutations={} live_hierarchy_weight_mutations={} live_router_threshold_delta={:.6} live_hierarchy_weight_delta={:.6} live_online_reward_feedbacks={} live_online_reward_reinforcements={} live_online_reward_penalties={} live_online_reward_strength={:.6} live_online_reward_reinforcement_strength={:.6} live_online_reward_penalty_strength={:.6} live_memory_updates={} live_stored_memory_updates={} live_reflection_issues={} live_critical_reflection_issues={} live_revision_actions={} replay_runs={} replay_items={} router_threshold_mutations={} hierarchy_weight_mutations={} router_threshold_delta={:.6} hierarchy_weight_delta={:.6} memory_updates={} replay_live_memory_feedback_items={} replay_live_memory_feedback_updates={} replay_live_memory_feedback_reinforcements={} replay_live_memory_feedback_penalties={} replay_live_memory_feedback_detail_items={} replay_live_memory_feedback_applied={} replay_live_memory_feedback_removed={} replay_live_memory_feedback_missing={} replay_live_memory_feedback_strength_delta={:.6} replay_rust_check_items={} replay_rust_check_passed={} replay_rust_check_failed={} replay_rust_check_diagnostic_chars={} replay_rust_check_live_memory_feedback_items={} replay_rust_check_live_memory_feedback_updates={} replay_rust_check_live_memory_feedback_applied={} replay_rust_check_live_memory_feedback_strength_delta={:.6} replay_business_contract_items={} replay_business_contract_passed={} replay_business_contract_failed={} replay_business_contract_raw_passed={} replay_business_contract_raw_failed={} replay_business_contract_response_normalized={} replay_business_contract_sanitized={} replay_business_contract_canonical_fallbacks={} replay_live_evolution_items={} replay_live_evolution_router_threshold_mutations={} replay_live_evolution_hierarchy_weight_mutations={} replay_live_evolution_router_threshold_delta={:.6} replay_live_evolution_hierarchy_weight_delta={:.6} replay_live_evolution_online_reward_feedbacks={} replay_live_evolution_online_reward_reinforcements={} replay_live_evolution_online_reward_penalties={} replay_live_evolution_online_reward_strength={:.6} replay_live_evolution_online_reward_reinforcement_strength={:.6} replay_live_evolution_online_reward_penalty_strength={:.6} replay_live_evolution_memory_updates={} replay_live_evolution_stored_memory_updates={} replay_live_evolution_reflection_issues={} replay_live_evolution_critical_reflection_issues={} replay_live_evolution_revision_actions={} recursive_replay_items={} recursive_runtime_calls={} drift_rollbacks={} rollback_router_threshold_delta={:.6} rollback_hierarchy_weight_delta={:.6} external_feedbacks={} external_feedback_reinforcements={} external_feedback_penalties={} external_feedback_memory_updates={} external_feedback_removed={} external_feedback_missing={} external_feedback_strength_delta={:.6}",
            self.live_inference_runs,
            self.live_router_threshold_mutations,
            self.live_hierarchy_weight_mutations,
            self.live_router_threshold_delta,
            self.live_hierarchy_weight_delta,
            self.live_online_reward_feedbacks,
            self.live_online_reward_reinforcements,
            self.live_online_reward_penalties,
            self.live_online_reward_strength,
            self.live_online_reward_reinforcement_strength,
            self.live_online_reward_penalty_strength,
            self.live_memory_updates(),
            self.live_stored_memory_updates(),
            self.live_reflection_issues,
            self.live_critical_reflection_issues,
            self.live_revision_actions,
            self.replay_runs,
            self.replay_items,
            self.router_threshold_mutations,
            self.hierarchy_weight_mutations,
            self.router_threshold_delta,
            self.hierarchy_weight_delta,
            self.memory_updates(),
            self.replay_live_memory_feedback_items,
            self.replay_live_memory_feedback_updates(),
            self.replay_live_memory_feedback_reinforcements,
            self.replay_live_memory_feedback_penalties,
            self.replay_live_memory_feedback_detail_items,
            self.replay_live_memory_feedback_applied,
            self.replay_live_memory_feedback_removed,
            self.replay_live_memory_feedback_missing,
            self.replay_live_memory_feedback_strength_delta,
            self.replay_rust_check_items,
            self.replay_rust_check_passed,
            self.replay_rust_check_failed,
            self.replay_rust_check_diagnostic_chars,
            self.replay_rust_check_live_memory_feedback_items,
            self.replay_rust_check_live_memory_feedback_updates,
            self.replay_rust_check_live_memory_feedback_applied,
            self.replay_rust_check_live_memory_feedback_strength_delta,
            self.replay_business_contract_items,
            self.replay_business_contract_passed,
            self.replay_business_contract_failed,
            self.replay_business_contract_raw_passed,
            self.replay_business_contract_raw_failed,
            self.replay_business_contract_response_normalized,
            self.replay_business_contract_sanitized,
            self.replay_business_contract_canonical_fallbacks,
            self.replay_live_evolution_items,
            self.replay_live_evolution_router_threshold_mutations,
            self.replay_live_evolution_hierarchy_weight_mutations,
            self.replay_live_evolution_router_threshold_delta,
            self.replay_live_evolution_hierarchy_weight_delta,
            self.replay_live_evolution_online_reward_feedbacks,
            self.replay_live_evolution_online_reward_reinforcements,
            self.replay_live_evolution_online_reward_penalties,
            self.replay_live_evolution_online_reward_strength,
            self.replay_live_evolution_online_reward_reinforcement_strength,
            self.replay_live_evolution_online_reward_penalty_strength,
            self.replay_live_evolution_memory_updates,
            self.replay_live_evolution_stored_memory_updates,
            self.replay_live_evolution_reflection_issues,
            self.replay_live_evolution_critical_reflection_issues,
            self.replay_live_evolution_revision_actions,
            self.recursive_replay_items,
            self.recursive_runtime_calls,
            self.drift_rollbacks,
            self.rollback_router_threshold_delta,
            self.rollback_hierarchy_weight_delta,
            self.external_feedbacks,
            self.external_feedback_reinforcements,
            self.external_feedback_penalties,
            self.external_feedback_memory_updates,
            self.external_feedback_removed,
            self.external_feedback_missing,
            self.external_feedback_strength_delta,
        )
    }
}
