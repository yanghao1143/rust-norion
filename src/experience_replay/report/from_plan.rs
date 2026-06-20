use super::super::item::{ExperienceReplayItem, ExperienceReplayPlan};
use super::super::stats::{LiveMemoryFeedbackStats, nonnegative_f32};
use super::ExperienceReplayReport;

impl ExperienceReplayReport {
    pub fn from_plan(plan: &ExperienceReplayPlan) -> Self {
        let average_reward = if plan.items.is_empty() {
            0.0
        } else {
            plan.items.iter().map(|item| item.reward).sum::<f32>() / plan.items.len() as f32
        };
        let recursive_runtime_items = plan
            .items
            .iter()
            .filter(|item| item.recursive_runtime_calls.is_some())
            .count();
        let recursive_runtime_calls = plan
            .items
            .iter()
            .filter_map(|item| item.recursive_runtime_calls)
            .sum();
        let recursive_call_pressure_total = plan
            .items
            .iter()
            .map(ExperienceReplayItem::recursive_call_pressure)
            .sum::<f32>();
        let average_recursive_call_pressure = if plan.items.is_empty() {
            0.0
        } else {
            recursive_call_pressure_total / plan.items.len() as f32
        };
        let max_recursive_call_pressure = plan
            .items
            .iter()
            .map(ExperienceReplayItem::recursive_call_pressure)
            .fold(0.0_f32, f32::max);
        let live_memory_feedback_items = plan
            .items
            .iter()
            .filter(|item| item.live_memory_feedback.is_some())
            .count();
        let live_memory_feedback_reinforcements = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.reinforced)
            .sum();
        let live_memory_feedback_penalties = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.penalized)
            .sum();
        let live_memory_feedback_updates =
            live_memory_feedback_reinforcements + live_memory_feedback_penalties;
        let live_memory_feedback_detail_items = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .filter(LiveMemoryFeedbackStats::has_detailed_update_evidence)
            .count();
        let live_memory_feedback_applied = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.applied)
            .sum();
        let live_memory_feedback_removed = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.removed)
            .sum();
        let live_memory_feedback_missing = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.missing)
            .sum();
        let live_memory_feedback_strength_delta = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.strength_delta)
            .sum();
        let rust_check_items = plan
            .items
            .iter()
            .filter(|item| item.rust_check_stats.is_some())
            .count();
        let rust_check_passed = plan
            .items
            .iter()
            .filter_map(|item| item.rust_check_stats)
            .map(|stats| stats.passed)
            .sum();
        let rust_check_failed = plan
            .items
            .iter()
            .filter_map(|item| item.rust_check_stats)
            .map(|stats| stats.failed)
            .sum();
        let rust_check_diagnostic_chars = plan
            .items
            .iter()
            .filter_map(|item| item.rust_check_stats)
            .map(|stats| stats.diagnostic_chars)
            .sum();
        let rust_check_live_memory_feedback_items = plan
            .items
            .iter()
            .filter(|item| item.rust_check_live_memory_feedback.is_some())
            .count();
        let rust_check_live_memory_feedback_updates = plan
            .items
            .iter()
            .filter_map(|item| item.rust_check_live_memory_feedback)
            .map(|feedback| feedback.updates())
            .sum();
        let rust_check_live_memory_feedback_applied = plan
            .items
            .iter()
            .filter_map(|item| item.rust_check_live_memory_feedback)
            .map(|feedback| feedback.applied)
            .sum();
        let rust_check_live_memory_feedback_missing = plan
            .items
            .iter()
            .filter_map(|item| item.rust_check_live_memory_feedback)
            .map(|feedback| feedback.missing)
            .sum();
        let rust_check_live_memory_feedback_strength_delta = plan
            .items
            .iter()
            .filter_map(|item| item.rust_check_live_memory_feedback)
            .map(|feedback| feedback.strength_delta)
            .sum();
        let business_contract_items = plan
            .items
            .iter()
            .filter(|item| item.business_contract_stats.is_some())
            .count();
        let business_contract_passed = plan
            .items
            .iter()
            .filter_map(|item| item.business_contract_stats)
            .map(|stats| stats.passed)
            .sum();
        let business_contract_failed = plan
            .items
            .iter()
            .filter_map(|item| item.business_contract_stats)
            .map(|stats| stats.failed)
            .sum();
        let business_contract_raw_passed = plan
            .items
            .iter()
            .filter_map(|item| item.business_contract_stats)
            .map(|stats| stats.raw_passed)
            .sum();
        let business_contract_raw_failed = plan
            .items
            .iter()
            .filter_map(|item| item.business_contract_stats)
            .map(|stats| stats.raw_failed)
            .sum();
        let business_contract_response_normalized = plan
            .items
            .iter()
            .filter_map(|item| item.business_contract_stats)
            .map(|stats| stats.response_normalized)
            .sum();
        let business_contract_sanitized = plan
            .items
            .iter()
            .filter_map(|item| item.business_contract_stats)
            .map(|stats| stats.sanitized)
            .sum();
        let business_contract_canonical_fallbacks = plan
            .items
            .iter()
            .filter_map(|item| item.business_contract_stats)
            .map(|stats| stats.canonical_fallbacks)
            .sum();
        let pool_dispatch_items = plan
            .items
            .iter()
            .filter_map(|item| item.pool_dispatch_stats.as_ref())
            .map(|stats| stats.items)
            .sum();
        let pool_dispatch_forwarded = plan
            .items
            .iter()
            .filter_map(|item| item.pool_dispatch_stats.as_ref())
            .map(|stats| stats.forwarded)
            .sum();
        let pool_dispatch_clamped = plan
            .items
            .iter()
            .filter_map(|item| item.pool_dispatch_stats.as_ref())
            .map(|stats| stats.clamped)
            .sum();
        let pool_dispatch_low_priority = plan
            .items
            .iter()
            .filter_map(|item| item.pool_dispatch_stats.as_ref())
            .map(|stats| stats.low_priority)
            .sum();
        let live_evolution_items = plan
            .items
            .iter()
            .filter(|item| item.live_evolution.has_evidence())
            .count();
        let live_evolution_router_threshold_mutations = plan
            .items
            .iter()
            .filter(|item| item.live_evolution.router_threshold_delta > 0.000001)
            .count();
        let live_evolution_hierarchy_weight_mutations = plan
            .items
            .iter()
            .filter(|item| item.live_evolution.hierarchy_weight_delta > 0.000001)
            .count();
        let live_evolution_router_threshold_delta = plan
            .items
            .iter()
            .map(|item| item.live_evolution.router_threshold_delta.max(0.0))
            .sum();
        let live_evolution_hierarchy_weight_delta = plan
            .items
            .iter()
            .map(|item| item.live_evolution.hierarchy_weight_delta.max(0.0))
            .sum();
        let live_evolution_online_reward_feedbacks = plan
            .items
            .iter()
            .map(|item| item.live_evolution.online_reward_feedbacks)
            .sum();
        let live_evolution_online_reward_reinforcements = plan
            .items
            .iter()
            .map(|item| item.live_evolution.online_reward_reinforcements)
            .sum();
        let live_evolution_online_reward_penalties = plan
            .items
            .iter()
            .map(|item| item.live_evolution.online_reward_penalties)
            .sum();
        let live_evolution_online_reward_strength = plan
            .items
            .iter()
            .map(|item| nonnegative_f32(item.live_evolution.online_reward_strength))
            .sum();
        let live_evolution_online_reward_reinforcement_strength = plan
            .items
            .iter()
            .map(|item| nonnegative_f32(item.live_evolution.online_reward_reinforcement_strength))
            .sum();
        let live_evolution_online_reward_penalty_strength = plan
            .items
            .iter()
            .map(|item| nonnegative_f32(item.live_evolution.online_reward_penalty_strength))
            .sum();
        let live_evolution_memory_updates = plan
            .items
            .iter()
            .map(|item| item.live_evolution.memory_updates())
            .sum();
        let live_evolution_stored_memory_updates = plan
            .items
            .iter()
            .map(|item| item.live_evolution.stored_memory_updates())
            .sum();
        let live_evolution_reflection_issues = plan
            .items
            .iter()
            .map(|item| item.live_evolution.reflection_issues)
            .sum();
        let live_evolution_critical_reflection_issues = plan
            .items
            .iter()
            .map(|item| item.live_evolution.critical_reflection_issues)
            .sum();
        let live_evolution_revision_actions = plan
            .items
            .iter()
            .map(|item| item.live_evolution.revision_actions)
            .sum();

        Self {
            planned: plan.items.len(),
            average_reward,
            recursive_runtime_items,
            recursive_runtime_calls,
            average_recursive_call_pressure,
            max_recursive_call_pressure,
            live_memory_feedback_items,
            live_memory_feedback_updates,
            live_memory_feedback_reinforcements,
            live_memory_feedback_penalties,
            live_memory_feedback_detail_items,
            live_memory_feedback_applied,
            live_memory_feedback_removed,
            live_memory_feedback_missing,
            live_memory_feedback_strength_delta,
            rust_check_items,
            rust_check_passed,
            rust_check_failed,
            rust_check_diagnostic_chars,
            rust_check_live_memory_feedback_items,
            rust_check_live_memory_feedback_updates,
            rust_check_live_memory_feedback_applied,
            rust_check_live_memory_feedback_missing,
            rust_check_live_memory_feedback_strength_delta,
            business_contract_items,
            business_contract_passed,
            business_contract_failed,
            business_contract_raw_passed,
            business_contract_raw_failed,
            business_contract_response_normalized,
            business_contract_sanitized,
            business_contract_canonical_fallbacks,
            pool_dispatch_items,
            pool_dispatch_forwarded,
            pool_dispatch_clamped,
            pool_dispatch_low_priority,
            live_evolution_items,
            live_evolution_router_threshold_mutations,
            live_evolution_hierarchy_weight_mutations,
            live_evolution_router_threshold_delta,
            live_evolution_hierarchy_weight_delta,
            live_evolution_online_reward_feedbacks,
            live_evolution_online_reward_reinforcements,
            live_evolution_online_reward_penalties,
            live_evolution_online_reward_strength,
            live_evolution_online_reward_reinforcement_strength,
            live_evolution_online_reward_penalty_strength,
            live_evolution_memory_updates,
            live_evolution_stored_memory_updates,
            live_evolution_reflection_issues,
            live_evolution_critical_reflection_issues,
            live_evolution_revision_actions,
            ..Self::default()
        }
    }
}
