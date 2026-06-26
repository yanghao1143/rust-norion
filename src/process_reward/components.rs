use crate::agent_team::AgentTeamPlan;
use crate::hierarchy::{HierarchyController, HierarchyWeights, TaskProfile};
use crate::infini_memory::InfiniMemoryCounts;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::router::RouteBudget;
use crate::tiered_cache::TierCounts;
use crate::toolsmith::ToolsmithPlan;

use super::types::{ProcessRewardComponents, ProcessRewardInput, RewardAction};

pub(super) fn score_components(
    input: &ProcessRewardInput,
    quality: f32,
    quality_score: f32,
) -> ProcessRewardComponents {
    let runtime_kv_segment_yield = input.runtime_kv_segment_yield();
    let runtime_kv_weak_import_pressure = input.runtime_kv_weak_import_pressure();
    ProcessRewardComponents {
        route: route_reward(input.route_budget, quality_score),
        memory: memory_reward(
            input.used_memories,
            input.used_experiences,
            input.gist_records,
            input.contradiction_count,
            quality,
            runtime_kv_segment_yield,
            runtime_kv_weak_import_pressure,
        ),
        hierarchy: hierarchy_reward(input.profile, input.hierarchy),
        reflection: reflection_reward(
            quality,
            input.contradiction_count,
            input.reflection_issue_count,
            input.critical_reflection_issue_count,
            input.revision_action_count,
        ),
        latency: latency_reward(
            input.route_budget,
            input.stream_windows,
            input.tier_counts,
            input.infini_counts,
            &input.recursive_schedule,
            input.recursive_runtime_calls,
            runtime_kv_segment_yield,
            runtime_kv_weak_import_pressure,
        ),
        admission: admission_reward(
            quality,
            input.contradiction_count,
            input.stored_memory,
            input.stored_gist_memories,
            input.stored_runtime_kv_memories,
            runtime_kv_segment_yield,
        ),
    }
}

fn route_reward(route_budget: RouteBudget, quality_score: f32) -> f32 {
    let attention = route_budget.attention_fraction.clamp(0.0, 1.0);
    let fast_fraction = 1.0 - attention;

    if quality_score >= 0.78 {
        (0.58 + fast_fraction * 0.34 + attention * 0.08).clamp(0.0, 1.0)
    } else if quality_score < 0.50 {
        (0.18 + attention * 0.42).clamp(0.0, 1.0)
    } else {
        (0.42 + (1.0 - (attention - 0.45).abs()) * 0.28).clamp(0.0, 1.0)
    }
}

fn memory_reward(
    used_memories: usize,
    used_experiences: usize,
    gist_records: usize,
    contradiction_count: usize,
    quality: f32,
    runtime_kv_segment_yield: Option<f32>,
    runtime_kv_weak_import_pressure: Option<f32>,
) -> f32 {
    let reuse = ((used_memories + used_experiences) as f32 / 6.0).min(1.0);
    let gist_bonus = (gist_records as f32 / 6.0).min(1.0) * 0.14;
    let contradiction_penalty = (contradiction_count as f32 * 0.16).min(0.48);
    let runtime_kv_waste_penalty = runtime_kv_segment_yield
        .map(|segment_yield| (1.0 - segment_yield) * 0.14)
        .unwrap_or(0.0);
    let weak_import_penalty = runtime_kv_weak_import_pressure
        .map(|pressure| pressure * 0.10)
        .unwrap_or(0.0);

    (0.34 + quality * 0.34 + reuse * 0.20 + gist_bonus
        - contradiction_penalty
        - runtime_kv_waste_penalty
        - weak_import_penalty)
        .clamp(0.0, 1.0)
}

fn hierarchy_reward(profile: TaskProfile, hierarchy: HierarchyWeights) -> f32 {
    let target = HierarchyController::target_for_profile(profile);
    let distance = ((hierarchy.global - target.global).abs()
        + (hierarchy.local - target.local).abs()
        + (hierarchy.convolution - target.convolution).abs())
        / 2.0;

    (1.0 - distance).clamp(0.0, 1.0)
}

fn reflection_reward(
    quality: f32,
    contradiction_count: usize,
    issue_count: usize,
    critical_issue_count: usize,
    revision_action_count: usize,
) -> f32 {
    let contradiction_penalty = (contradiction_count as f32 * 0.13).min(0.45);
    let issue_penalty = (issue_count as f32 * 0.04).min(0.20);
    let critical_penalty = (critical_issue_count as f32 * 0.18).min(0.50);
    let action_credit = (revision_action_count as f32 * 0.02).min(0.08);
    (quality - contradiction_penalty - issue_penalty - critical_penalty + action_credit)
        .clamp(0.0, 1.0)
}

fn latency_reward(
    route_budget: RouteBudget,
    stream_windows: usize,
    tier_counts: TierCounts,
    infini_counts: InfiniMemoryCounts,
    recursive_schedule: &RecursiveSchedule,
    recursive_runtime_calls: usize,
    runtime_kv_segment_yield: Option<f32>,
    runtime_kv_weak_import_pressure: Option<f32>,
) -> f32 {
    let fast_fraction = 1.0 - route_budget.attention_fraction.clamp(0.0, 1.0);
    let stream_pressure = (stream_windows as f32 / 48.0).min(0.30);
    let cold_pressure = (tier_counts.cold_disk as f32 / 12.0).min(0.18);
    let sparse_bonus = if infini_counts.skipped > 0 { 0.08 } else { 0.0 };
    let recursion_pressure = if recursive_schedule.requires_recursion {
        let wave_pressure = (recursive_schedule.execution_wave_count() as f32 / 32.0).min(0.14);
        let chunk_overhead = (recursive_schedule.chunk_count() as f32 / 128.0).min(0.04);
        let call_pressure = (recursive_runtime_calls.saturating_sub(1) as f32 / 160.0).min(0.18);
        wave_pressure + chunk_overhead + call_pressure
    } else {
        0.0
    };
    let runtime_kv_waste_pressure = runtime_kv_segment_yield
        .map(|segment_yield| (1.0 - segment_yield) * 0.12)
        .unwrap_or(0.0);
    let weak_import_pressure = runtime_kv_weak_import_pressure
        .map(|pressure| pressure * 0.08)
        .unwrap_or(0.0);

    (0.42 + fast_fraction * 0.34 + sparse_bonus
        - stream_pressure
        - cold_pressure
        - recursion_pressure
        - runtime_kv_waste_pressure
        - weak_import_pressure)
        .clamp(0.0, 1.0)
}

fn admission_reward(
    quality: f32,
    contradiction_count: usize,
    stored_memory: bool,
    stored_gist_memories: usize,
    stored_runtime_kv_memories: usize,
    runtime_kv_segment_yield: Option<f32>,
) -> f32 {
    let stored_any = stored_memory || stored_gist_memories > 0 || stored_runtime_kv_memories > 0;
    let runtime_kv_store_weight =
        stored_runtime_kv_memories as f32 * runtime_kv_segment_yield.unwrap_or(1.0);
    let store_bonus =
        ((stored_gist_memories as f32 + runtime_kv_store_weight) / 4.0).min(1.0) * 0.18;
    let runtime_kv_waste_penalty = if stored_runtime_kv_memories > 0 {
        runtime_kv_segment_yield
            .map(|segment_yield| (1.0 - segment_yield) * 0.12)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    match (quality >= 0.60 && contradiction_count == 0, stored_any) {
        (true, true) => (0.72 + store_bonus - runtime_kv_waste_penalty).clamp(0.0, 1.0),
        (true, false) => 0.44,
        (false, false) => 0.72,
        (false, true) => (0.24 - runtime_kv_waste_penalty * 0.5).clamp(0.0, 1.0),
    }
}

pub(super) fn weighted_total(components: ProcessRewardComponents) -> f32 {
    (components.route * 0.18
        + components.memory * 0.16
        + components.hierarchy * 0.15
        + components.reflection * 0.24
        + components.latency * 0.12
        + components.admission * 0.15)
        .clamp(0.0, 1.0)
}

pub(super) fn toolsmith_adjusted_total(total: f32, plan: &ToolsmithPlan) -> f32 {
    if !plan.passed_rust_gate() {
        return (total - 0.18).clamp(0.0, 1.0);
    }
    if plan.ready_count() > 0 {
        return (total + 0.04).clamp(0.0, 1.0);
    }
    total
}

pub(super) fn coordination_adjusted_total(total: f32, plan: &AgentTeamPlan) -> f32 {
    if !plan.enabled {
        return total;
    }
    if !plan.collision_free() {
        return (total - 0.12).clamp(0.0, 1.0);
    }
    if plan.evolution_signal_count() > 0 {
        return (total + 0.03).clamp(0.0, 1.0);
    }
    total
}

pub(super) fn action_for_total(total: f32) -> RewardAction {
    if total >= 0.72 {
        RewardAction::Reinforce
    } else if total <= 0.42 {
        RewardAction::Penalize
    } else {
        RewardAction::Hold
    }
}
