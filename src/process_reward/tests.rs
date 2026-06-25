use super::*;
use crate::agent_team::AgentTeamPlan;
use crate::hierarchy::{HierarchyController, TaskProfile};
use crate::infini_memory::InfiniMemoryCounts;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::router::{GenerationMetrics, RouteBudget};
use crate::tiered_cache::TierCounts;
use crate::toolsmith::ToolsmithPlan;

#[test]
fn high_quality_fast_path_is_reinforced() {
    let report = ProcessRewarder::new().score(input(0.92, 0, 0.05, false));

    assert!(report.total >= 0.72);
    assert_eq!(report.action, RewardAction::Reinforce);
    assert!(report.components.route > 0.8);
}

#[test]
fn low_quality_stored_memory_is_penalized() {
    let mut input = input(0.25, 2, 0.0, false);
    input.stored_memory = true;

    let report = ProcessRewarder::new().score(input);

    assert_eq!(report.action, RewardAction::Penalize);
    assert!(report.components.admission < 0.35);
}

#[test]
fn recursive_plan_adds_trace_note() {
    let mut input = input(0.82, 0, 0.20, true);
    input.recursive_schedule = crate::recursive_scheduler::RecursiveScheduler::new(8, 6, 2, 2)
        .plan("one two three four five six seven eight nine ten");

    let report = ProcessRewarder::new().score(input);

    assert!(report.notes.iter().any(|note| note.contains("recursive")));
}

#[test]
fn recursive_runtime_calls_reduce_latency_reward() {
    let mut cheap = input(0.82, 0, 0.20, true);
    cheap.recursive_runtime_calls = cheap.recursive_schedule.chunk_count().max(1);
    let mut expensive = cheap.clone();
    expensive.recursive_runtime_calls = 96;

    let cheap_report = ProcessRewarder::new().score(cheap);
    let expensive_report = ProcessRewarder::new().score(expensive);

    assert!(expensive_report.components.latency < cheap_report.components.latency);
    assert!(
        expensive_report
            .notes
            .iter()
            .any(|note| note == "latency:recursive_runtime_calls=96")
    );
}

#[test]
fn runtime_kv_segments_add_audit_note() {
    let mut input = input(0.82, 0, 0.20, false);
    input.weak_runtime_kv_imports_skipped = 2;
    input.runtime_kv_segments_included = 2;
    input.runtime_kv_segments_skipped = 1;
    input.runtime_kv_segments_rejected = 1;

    let report = ProcessRewarder::new().score(input);

    assert!(report.notes.iter().any(|note| {
        note == "runtime_kv_segments:included=2:skipped=1:rejected=1:total=4:yield=0.250"
    }));
    assert!(
        report
            .notes
            .iter()
            .any(|note| note == "runtime_kv_import:weak_skipped=2")
    );
}

#[test]
fn low_runtime_kv_segment_yield_reduces_reward_components() {
    let mut efficient = input(0.82, 0, 0.20, false);
    efficient.stored_runtime_kv_memories = 1;
    efficient.runtime_kv_segments_included = 3;
    efficient.runtime_kv_segments_skipped = 0;
    efficient.runtime_kv_segments_rejected = 0;
    let mut wasteful = efficient.clone();
    wasteful.runtime_kv_segments_included = 0;
    wasteful.runtime_kv_segments_skipped = 3;
    wasteful.runtime_kv_segments_rejected = 2;

    let efficient_report = ProcessRewarder::new().score(efficient);
    let wasteful_report = ProcessRewarder::new().score(wasteful);

    assert!(wasteful_report.components.memory < efficient_report.components.memory);
    assert!(wasteful_report.components.latency < efficient_report.components.latency);
    assert!(wasteful_report.components.admission < efficient_report.components.admission);
    assert!(wasteful_report.total < efficient_report.total);
    assert!(wasteful_report.notes.iter().any(|note| {
        note == "runtime_kv_segments:included=0:skipped=3:rejected=2:total=5:yield=0.000"
    }));
}

#[test]
fn critical_reflection_issues_reduce_reward() {
    let mut input = input(0.70, 1, 0.45, false);
    input.reflection_issue_count = 3;
    input.critical_reflection_issue_count = 2;
    input.revision_action_count = 3;

    let report = ProcessRewarder::new().score(input);

    assert!(report.components.reflection < 0.30);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note == "reflection:critical_issues=2")
    );
}

fn input(
    quality: f32,
    contradiction_count: usize,
    attention_fraction: f32,
    requires_recursion: bool,
) -> ProcessRewardInput {
    let recursive_schedule = if requires_recursion {
        crate::recursive_scheduler::RecursiveScheduler::new(8, 6, 2, 2)
            .plan("one two three four five six seven eight nine ten")
    } else {
        RecursiveSchedule::default()
    };

    ProcessRewardInput {
        profile: TaskProfile::Coding,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: (attention_fraction * 10.0).round() as usize,
            fast_tokens: ((1.0 - attention_fraction) * 10.0).round() as usize,
            attention_fraction,
        },
        hierarchy: HierarchyController::target_for_profile(TaskProfile::Coding),
        metrics: GenerationMetrics {
            perplexity: 6.0,
            semantic_consistency: quality,
            contradiction_count,
            token_count: 64,
        },
        quality,
        contradiction_count,
        reflection_issue_count: contradiction_count,
        critical_reflection_issue_count: 0,
        revision_action_count: contradiction_count,
        used_memories: 2,
        used_experiences: 1,
        tier_counts: TierCounts::default(),
        infini_counts: InfiniMemoryCounts::default(),
        recursive_schedule,
        recursive_runtime_calls: if requires_recursion { 6 } else { 1 },
        stream_windows: 4,
        stored_memory: quality > 0.45,
        stored_gist_memories: if quality > 0.45 { 1 } else { 0 },
        stored_runtime_kv_memories: 0,
        weak_runtime_kv_imports_skipped: 0,
        runtime_kv_segments_included: 0,
        runtime_kv_segments_skipped: 0,
        runtime_kv_segments_rejected: 0,
        gist_records: if quality > 0.45 { 3 } else { 0 },
        toolsmith_plan: ToolsmithPlan::default(),
        agent_team_plan: AgentTeamPlan::default(),
    }
}
