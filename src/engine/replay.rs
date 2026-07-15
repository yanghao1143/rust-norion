use std::collections::BTreeSet;

use crate::experience_replay::ExperienceReplayReport;
use crate::process_reward::RewardAction;
use crate::tenant_scope::TenantScope;

use super::NoironEngine;
use super::metrics::hierarchy_weight_delta;
use super::replay_feedback::{
    replay_memory_update_amount, replay_metrics, replay_penalty_amount,
    replay_reinforcement_amount, replay_runtime_kv_budget_pressure,
    replay_runtime_kv_weak_import_pressure,
};
use super::text::compact;

impl NoironEngine {
    pub fn replay_experience(&mut self, limit: usize) -> ExperienceReplayReport {
        let plan = self
            .experience_replay_planner
            .plan(self.experience.records(), limit);
        self.apply_replay_plan(plan)
    }

    pub fn replay_experience_scoped(
        &mut self,
        limit: usize,
        scope: &TenantScope,
    ) -> ExperienceReplayReport {
        let visible_memory_ids = self
            .cache
            .entries_scoped(scope)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<BTreeSet<_>>();
        let mut plan = self
            .experience_replay_planner
            .plan(self.experience.records(), limit);
        for item in &mut plan.items {
            item.memory_ids
                .retain(|memory_id| visible_memory_ids.contains(memory_id));
        }
        plan.items.retain(|item| !item.memory_ids.is_empty());
        self.apply_replay_plan(plan)
    }

    fn apply_replay_plan(
        &mut self,
        plan: crate::experience_replay::ExperienceReplayPlan,
    ) -> ExperienceReplayReport {
        let mut report = ExperienceReplayReport::from_plan(&plan);

        for item in plan.items {
            let metrics = replay_metrics(&item);
            let router_before = self.router.threshold_for(item.profile);
            self.router.observe_with_profile(item.profile, metrics);
            report.router_updates += 1;
            let router_after = self.router.threshold_for(item.profile);
            let router_delta = (router_after - router_before).abs();
            if router_delta > 0.000001 {
                report.router_threshold_mutations += 1;
                report.router_threshold_delta += router_delta;
            }

            let hierarchy_before = self.hierarchy.state().profile_weights.get(item.profile);
            let hierarchy_after = self.hierarchy.observe(item.profile, metrics);
            report.hierarchy_updates += 1;
            let hierarchy_delta = hierarchy_weight_delta(hierarchy_before, hierarchy_after);
            if hierarchy_delta > 0.000001 {
                report.hierarchy_weight_mutations += 1;
                report.hierarchy_weight_delta += hierarchy_delta;
            }

            match item.action {
                RewardAction::Reinforce => {
                    let reinforcement = replay_reinforcement_amount(&item);
                    for memory_id in &item.memory_ids {
                        let update = self.cache.reinforce(*memory_id, reinforcement);
                        report.record_memory_update(update);
                    }
                    report.reinforced += 1;
                }
                RewardAction::Penalize => {
                    let penalty = replay_penalty_amount(&item);
                    for memory_id in &item.memory_ids {
                        let update = self.cache.penalize(*memory_id, penalty);
                        report.record_memory_update(update);
                    }
                    report.penalized += 1;
                }
                RewardAction::Hold => {}
            }

            report.applied += 1;
            report.notes.push(replay_note(&item));
        }

        self.evolution_ledger.record_replay(&report);
        report
    }

    pub(super) fn maybe_auto_replay(
        &mut self,
        scope: &TenantScope,
    ) -> Option<ExperienceReplayReport> {
        if self.auto_replay_limit == 0 || self.experience.is_empty() {
            return None;
        }
        if self.hardware_snapshot.pressure() >= 0.72 {
            return None;
        }

        let report = self.replay_experience_scoped(self.auto_replay_limit, scope);
        if report.applied == 0 {
            None
        } else {
            Some(report)
        }
    }
}

fn replay_note(item: &crate::experience_replay::ExperienceReplayItem) -> String {
    let memory_update = replay_memory_update_amount(item);
    let live_feedback_updates = item
        .live_memory_feedback
        .map(|feedback| feedback.updates())
        .unwrap_or(0);
    let live_feedback_reinforced = item
        .live_memory_feedback
        .map(|feedback| feedback.reinforced)
        .unwrap_or(0);
    let live_feedback_penalized = item
        .live_memory_feedback
        .map(|feedback| feedback.penalized)
        .unwrap_or(0);
    let business_contract_raw_failed = item
        .business_contract_stats
        .map(|stats| stats.raw_failed)
        .unwrap_or(0);
    let business_contract_failed = item
        .business_contract_stats
        .map(|stats| stats.failed)
        .unwrap_or(0);
    let business_contract_sanitized = item
        .business_contract_stats
        .map(|stats| stats.sanitized)
        .unwrap_or(0);
    let business_contract_canonical_fallbacks = item
        .business_contract_stats
        .map(|stats| stats.canonical_fallbacks)
        .unwrap_or(0);
    let pool_dispatch_forwarded = item
        .pool_dispatch_stats
        .as_ref()
        .map(|stats| stats.forwarded)
        .unwrap_or(0);
    let pool_dispatch_clamped = item
        .pool_dispatch_stats
        .as_ref()
        .map(|stats| stats.clamped)
        .unwrap_or(0);
    let pool_dispatch_low_priority = item
        .pool_dispatch_stats
        .as_ref()
        .map(|stats| stats.low_priority)
        .unwrap_or(0);
    let runtime_kv_budget_pressure = replay_runtime_kv_budget_pressure(item);
    let runtime_kv_weak_import_pressure = replay_runtime_kv_weak_import_pressure(item);
    let rust_check_passed = item.rust_check_stats.map(|stats| stats.passed).unwrap_or(0);
    let rust_check_failed = item.rust_check_stats.map(|stats| stats.failed).unwrap_or(0);
    let rust_check_diagnostic_chars = item
        .rust_check_stats
        .map(|stats| stats.diagnostic_chars)
        .unwrap_or(0);

    format!(
        "experience:{}:{} reward={:.3} memory_update={:.3} reflection_issues={} critical={} actions={} recursive_runtime_calls={} live_feedback_updates={} live_feedback_reinforced={} live_feedback_penalized={} business_contract_failed={} business_contract_raw_failed={} business_contract_sanitized={} business_contract_canonical_fallbacks={} pool_dispatch_forwarded={} pool_dispatch_clamped={} pool_dispatch_low_priority={} rust_check_passed={} rust_check_failed={} rust_check_diagnostic_chars={} runtime_kv_budget_pressure={:.3} runtime_kv_weak_import_pressure={:.3} lesson={}",
        item.experience_id,
        item.action.as_str(),
        item.reward,
        memory_update,
        item.reflection_issue_count,
        item.critical_reflection_issue_count,
        item.revision_action_count,
        item.recursive_runtime_calls
            .map(|calls| calls.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        live_feedback_updates,
        live_feedback_reinforced,
        live_feedback_penalized,
        business_contract_failed,
        business_contract_raw_failed,
        business_contract_sanitized,
        business_contract_canonical_fallbacks,
        pool_dispatch_forwarded,
        pool_dispatch_clamped,
        pool_dispatch_low_priority,
        rust_check_passed,
        rust_check_failed,
        rust_check_diagnostic_chars,
        runtime_kv_budget_pressure,
        runtime_kv_weak_import_pressure,
        compact(&item.lesson, 64)
    )
}
