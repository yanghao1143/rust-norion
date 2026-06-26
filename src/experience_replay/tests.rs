use super::*;
use crate::adaptive_state::LiveInferenceEvolution;
use crate::experience::ExperienceInput;
use crate::experience::ExperienceRecord;
use crate::hierarchy::HierarchyWeights;
use crate::hierarchy::TaskProfile;
use crate::process_reward::{ProcessRewardComponents, ProcessRewardReport, RewardAction};
use crate::reflection::{ReflectionIssue, ReflectionSeverity, RuntimeDiagnostics};
use crate::router::RouteBudget;

#[test]
fn planner_selects_reinforce_and_penalize_records() {
    let planner = ExperienceReplayPlanner::new();
    let records = vec![
        record(1, 0.90, RewardAction::Reinforce),
        record(2, 0.50, RewardAction::Hold),
        record(3, 0.20, RewardAction::Penalize),
    ];

    let plan = planner.plan(&records, 8);

    assert_eq!(plan.items.len(), 2);
    assert!(
        plan.items
            .iter()
            .any(|item| item.action == RewardAction::Reinforce)
    );
    let reinforced = plan
        .items
        .iter()
        .find(|item| item.action == RewardAction::Reinforce)
        .unwrap();
    assert!(reinforced.memory_ids.contains(&1));
    assert!(reinforced.memory_ids.contains(&11));
    assert!(reinforced.memory_ids.contains(&21));
    assert!(reinforced.memory_ids.contains(&31));
    assert_eq!(
        reinforced.runtime_diagnostics.model_id.as_deref(),
        Some("replay-runtime")
    );
    assert_eq!(reinforced.runtime_diagnostics.forward_energy, Some(0.31));
    assert!(
        plan.items
            .iter()
            .any(|item| item.action == RewardAction::Penalize)
    );
    assert!(!plan.items.iter().any(|item| item.experience_id == 2));
    let penalized = plan
        .items
        .iter()
        .find(|item| item.action == RewardAction::Penalize)
        .unwrap();
    assert_eq!(penalized.critical_reflection_issue_count, 1);
    assert_eq!(penalized.revision_action_count, 1);
    assert_eq!(
        reinforced.live_memory_feedback,
        Some(LiveMemoryFeedbackStats {
            reinforced: 2,
            penalized: 0,
            reinforcement_amount: 1.2,
            penalty_amount: 0.0,
            applied: 2,
            removed: 0,
            missing: 0,
            strength_delta: 0.42,
            detailed_evidence: true,
        })
    );
    assert_eq!(
        reinforced.recursive_stats,
        Some(RecursiveReplayStats {
            chunks: Some(4),
            merge_rounds: Some(2),
            waves: Some(2),
            parallel: Some(2),
            runtime_calls: Some(7),
        })
    );
}

#[test]
fn planner_honors_limit_and_priority() {
    let planner = ExperienceReplayPlanner::new();
    let records = vec![
        record(1, 0.73, RewardAction::Reinforce),
        record(2, 0.95, RewardAction::Reinforce),
        record(3, 0.01, RewardAction::Penalize),
    ];

    let plan = planner.plan(&records, 1);

    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].experience_id, 3);
}

#[test]
fn planner_downweights_reinforcement_priority_from_runtime_kv_budget_pressure() {
    let planner = ExperienceReplayPlanner::new();
    let mut pressured = record(1, 0.90, RewardAction::Reinforce);
    pressured
        .runtime_diagnostics
        .budget_limited_runtime_kv_imports_skipped = 8;
    let clean = record(2, 0.85, RewardAction::Reinforce);

    let plan = planner.plan(&[pressured, clean], 2);

    assert_eq!(plan.items.len(), 2);
    assert_eq!(plan.items[0].experience_id, 2);
    assert_eq!(plan.items[1].experience_id, 1);
    assert_eq!(plan.items[1].runtime_kv_budget_pressure(), 0.8);
    assert!(plan.items[1].priority < plan.items[0].priority);
}

#[test]
fn planner_prioritizes_penalty_replay_from_runtime_kv_budget_pressure() {
    let planner = ExperienceReplayPlanner::new();
    let clean = record(1, 0.30, RewardAction::Penalize);
    let mut pressured = record(2, 0.35, RewardAction::Penalize);
    pressured
        .runtime_diagnostics
        .budget_limited_runtime_kv_imports_skipped = 8;

    let plan = planner.plan(&[clean, pressured], 2);

    assert_eq!(plan.items.len(), 2);
    assert_eq!(plan.items[0].experience_id, 2);
    assert_eq!(plan.items[1].experience_id, 1);
    assert_eq!(plan.items[0].runtime_kv_budget_pressure(), 0.8);
    assert!(plan.items[0].priority > plan.items[1].priority);
}

#[test]
fn planner_downweights_reinforcement_priority_from_weak_runtime_kv_imports() {
    let planner = ExperienceReplayPlanner::new();
    let mut weak = record(1, 0.90, RewardAction::Reinforce);
    weak.runtime_diagnostics.imported_kv_blocks = 1;
    weak.runtime_diagnostics.weak_runtime_kv_imports_skipped = 3;
    let clean = record(2, 0.85, RewardAction::Reinforce);

    let plan = planner.plan(&[weak, clean], 2);

    assert_eq!(plan.items.len(), 2);
    assert_eq!(plan.items[0].experience_id, 2);
    assert_eq!(plan.items[1].experience_id, 1);
    assert_eq!(plan.items[1].runtime_kv_weak_import_pressure(), 0.75);
    assert!(plan.items[1].priority < plan.items[0].priority);
}

#[test]
fn planner_prioritizes_penalty_replay_from_weak_runtime_kv_imports() {
    let planner = ExperienceReplayPlanner::new();
    let clean = record(1, 0.30, RewardAction::Penalize);
    let mut weak = record(2, 0.35, RewardAction::Penalize);
    weak.runtime_diagnostics.imported_kv_blocks = 1;
    weak.runtime_diagnostics.weak_runtime_kv_imports_skipped = 3;

    let plan = planner.plan(&[clean, weak], 2);

    assert_eq!(plan.items.len(), 2);
    assert_eq!(plan.items[0].experience_id, 2);
    assert_eq!(plan.items[1].experience_id, 1);
    assert_eq!(plan.items[0].runtime_kv_weak_import_pressure(), 0.75);
    assert!(plan.items[0].priority > plan.items[1].priority);
}

#[test]
fn planner_excludes_hygiene_quarantine_candidates_before_replay() {
    let planner = ExperienceReplayPlanner::new();
    let mut polluted = record(1, 0.99, RewardAction::Reinforce);
    polluted.prompt = "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant: clean loop\nuser: Bash command\nssh -o ConnectTimeout=8 host 'echo ok'\nassistant: gitlab.local merge_requests"
        .to_owned();
    polluted.lesson =
        "polluted replay answer mixed with shell and merge_requests context".to_owned();
    let clean = record(2, 0.80, RewardAction::Reinforce);

    let plan = planner.plan(&[polluted, clean], 8);

    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].experience_id, 2);
    assert!(!plan.items.iter().any(|item| item.experience_id == 1));
}

#[test]
fn planner_keeps_recursive_runtime_sample_when_limit_allows() {
    let planner = ExperienceReplayPlanner::new();
    let mut recursive = record(5, 0.80, RewardAction::Reinforce);
    recursive.profile = TaskProfile::LongDocument;
    recursive.live_evolution = LiveInferenceEvolution::default();
    recursive.process_reward.notes =
        vec!["recursive:chunks=32:merge_rounds=2:waves=8:parallel=2:runtime_calls=96".to_owned()];
    let mut high_priority = record(1, 0.96, RewardAction::Reinforce);
    high_priority.process_reward.notes.clear();
    high_priority.live_evolution = LiveInferenceEvolution::default();
    let mut second_priority = record(2, 0.95, RewardAction::Reinforce);
    second_priority.process_reward.notes.clear();
    second_priority.live_evolution = LiveInferenceEvolution::default();
    let records = vec![high_priority, second_priority, recursive];

    let plan = planner.plan(&records, 2);

    assert_eq!(plan.items.len(), 2);
    assert!(
        plan.items
            .iter()
            .any(|item| item.recursive_runtime_calls == Some(96))
    );
    assert!(
        plan.items
            .iter()
            .find(|item| item.recursive_runtime_calls == Some(96))
            .unwrap()
            .recursive_call_pressure()
            > 0.0
    );
    assert!(
        plan.items
            .iter()
            .any(|item| item.experience_id == 1 || item.experience_id == 2)
    );
}

#[test]
fn planner_keeps_live_evolution_sample_when_limit_allows() {
    let planner = ExperienceReplayPlanner::new();
    let mut high_priority = record(1, 0.98, RewardAction::Reinforce);
    high_priority.live_evolution = LiveInferenceEvolution::default();
    high_priority.process_reward.notes.clear();
    let mut second_priority = record(2, 0.97, RewardAction::Reinforce);
    second_priority.live_evolution = LiveInferenceEvolution::default();
    second_priority.process_reward.notes.clear();
    let mut live_evolution = record(3, 0.80, RewardAction::Reinforce);
    live_evolution.live_evolution.revision_actions = 1;
    live_evolution.process_reward.notes.clear();
    let records = vec![high_priority, second_priority, live_evolution];

    let plan = planner.plan(&records, 2);

    assert_eq!(plan.items.len(), 2);
    assert!(
        plan.items
            .iter()
            .any(|item| item.live_evolution.has_evidence())
    );
    assert!(plan.items.iter().any(|item| item.experience_id == 1));
}

#[test]
fn planner_does_not_displace_only_recursive_sample_for_live_evolution_at_tiny_limit() {
    let planner = ExperienceReplayPlanner::new();
    let mut recursive = record(5, 0.80, RewardAction::Reinforce);
    recursive.profile = TaskProfile::LongDocument;
    recursive.live_evolution = LiveInferenceEvolution::default();
    recursive.process_reward.notes =
        vec!["recursive:chunks=32:merge_rounds=2:waves=8:parallel=2:runtime_calls=96".to_owned()];
    let mut high_priority = record(1, 0.98, RewardAction::Reinforce);
    high_priority.live_evolution = LiveInferenceEvolution::default();
    high_priority.process_reward.notes.clear();
    let mut live_evolution = record(3, 0.80, RewardAction::Reinforce);
    live_evolution.live_evolution.revision_actions = 1;
    live_evolution.process_reward.notes.clear();
    let records = vec![high_priority, recursive, live_evolution];

    let plan = planner.plan(&records, 1);

    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].recursive_runtime_calls, Some(96));
    assert!(!plan.items[0].live_evolution.has_evidence());
}

#[test]
fn recursive_pressure_uses_schedule_stats_not_route_token_count() {
    let planner = ExperienceReplayPlanner::new();
    let mut recursive = record(7, 0.88, RewardAction::Reinforce);
    recursive.route_budget.fast_tokens = 2_222;
    recursive.route_budget.attention_tokens = 0;
    recursive.process_reward.notes =
        vec!["recursive:chunks=89:merge_rounds=4:waves=23:parallel=4:runtime_calls=121".to_owned()];

    let plan = planner.plan(&[recursive], 1);
    let item = &plan.items[0];

    assert_eq!(item.recursive_runtime_calls, Some(121));
    assert_eq!(item.recursive_stats.unwrap().chunks, Some(89));
    assert!(item.route_token_count() > 2_000);
    assert!(item.recursive_call_pressure() > 0.0);
}

#[test]
fn recursive_stats_aggregate_multiple_notes_by_maximum_evidence() {
    let stats = RecursiveReplayStats::from_notes(&[
        "recursive:chunks=4:waves=2:runtime_calls=7".to_owned(),
        "recursive:chunks=8:merge_rounds=3:parallel=4:runtime_calls=11".to_owned(),
        "recursive:chunks=0:runtime_calls=0".to_owned(),
    ])
    .unwrap();

    assert_eq!(
        stats,
        RecursiveReplayStats {
            chunks: Some(8),
            merge_rounds: Some(3),
            waves: Some(2),
            parallel: Some(4),
            runtime_calls: Some(11),
        }
    );
}

#[test]
fn planner_uses_aggregated_recursive_stats_for_runtime_calls() {
    let planner = ExperienceReplayPlanner::new();
    let mut recursive = record(17, 0.88, RewardAction::Reinforce);
    recursive.process_reward.notes = vec![
        "recursive:chunks=4:waves=2:runtime_calls=7".to_owned(),
        "recursive:chunks=8:merge_rounds=3:parallel=4:runtime_calls=11".to_owned(),
    ];

    let plan = planner.plan(&[recursive], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(item.recursive_runtime_calls, Some(11));
    assert_eq!(
        item.recursive_stats,
        Some(RecursiveReplayStats {
            chunks: Some(8),
            merge_rounds: Some(3),
            waves: Some(2),
            parallel: Some(4),
            runtime_calls: Some(11),
        })
    );
    assert_eq!(report.recursive_runtime_calls, 11);
    assert!(report.summary().contains("recursive_runtime_calls=11"));
}

fn record(id: u64, reward: f32, action: RewardAction) -> ExperienceRecord {
    let input = ExperienceInput {
        prompt: "replay prompt".to_owned(),
            profile: TaskProfile::Coding,
            lesson: "replay lesson".to_owned(),
            quality: reward,
            contradictions: if action == RewardAction::Penalize {
                vec!["bad".to_owned()]
            } else {
                Vec::new()
            },
            reflection_issues: if action == RewardAction::Penalize {
                vec![ReflectionIssue::new(
                    "bad",
                    ReflectionSeverity::Critical,
                    "bad replay issue",
                )]
            } else {
                Vec::new()
            },
            revision_actions: if action == RewardAction::Penalize {
                vec!["review_bad_replay".to_owned()]
            } else {
                Vec::new()
            },
            stored_memory_id: Some(id),
            router_threshold_after: 0.5,
            stream_windows: 2,
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 2,
                fast_tokens: 2,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: vec![id + 20],
            gist_records: Vec::new(),
            gist_memory_ids: vec![id + 10],
            stored_runtime_kv_memory_ids: vec![id + 30],
            runtime_diagnostics: RuntimeDiagnostics {
                model_id: Some("replay-runtime".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                device_profile: Some("cpu".to_owned()),
                primary_lane: Some("cpu-vector".to_owned()),
                fallback_lane: Some("cpu-portable".to_owned()),
                memory_mode: Some("tiered-disk".to_owned()),
                device_execution_source: Some(
                    RuntimeDiagnostics::runtime_reported_device_execution_source().to_owned(),
                ),
                layer_count: 12,
                global_layers: 3,
                local_window_layers: 6,
                convolutional_fusion_layers: 3,
                hidden_size: 128,
                local_window_tokens: 4096,
                forward_energy: Some(0.31),
                kv_influence: Some(0.27),
                imported_kv_blocks: 1,
                exported_kv_blocks: 2,
                hot_kv_precision_bits: Some(8),
                cold_kv_precision_bits: Some(4),
                ..RuntimeDiagnostics::default()
            },
            runtime_token_metrics: Default::default(),
            process_reward: ProcessRewardReport {
                total: reward,
                action,
                components: ProcessRewardComponents::default(),
                notes: vec![
                    "recursive:chunks=4:merge_rounds=2:waves=2:parallel=2:runtime_calls=7"
                        .to_owned(),
                    "memory_feedback:reinforced=2:penalized=0:reinforcement_amount=1.200000:penalty_amount=0.000000:applied=2:removed=0:missing=0:strength_delta=0.420000"
                        .to_owned(),
                ],
            },
            live_evolution: LiveInferenceEvolution {
                router_threshold_delta: 0.02,
                hierarchy_weight_delta: 0.03,
                online_reward_feedbacks: 1,
                online_reward_reinforcements: usize::from(action == RewardAction::Reinforce),
                online_reward_penalties: usize::from(action == RewardAction::Penalize),
                online_reward_strength: match action {
                    RewardAction::Reinforce => 0.80,
                    RewardAction::Penalize => 0.60,
                    RewardAction::Hold => 0.0,
                },
                online_reward_reinforcement_strength: if action == RewardAction::Reinforce {
                    0.80
                } else {
                    0.0
                },
                online_reward_penalty_strength: if action == RewardAction::Penalize {
                    0.60
                } else {
                    0.0
                },
                memory_reinforcements: 2,
                memory_penalties: 0,
                stored_memory: true,
                stored_gist_memories: 1,
                stored_runtime_kv_memories: 1,
                reflection_issues: if action == RewardAction::Penalize { 1 } else { 0 },
                critical_reflection_issues: if action == RewardAction::Penalize { 1 } else { 0 },
                revision_actions: if action == RewardAction::Penalize { 1 } else { 0 },
            },
        };

    ExperienceRecord {
        id,
        prompt: input.prompt,
        profile: input.profile,
        lesson: input.lesson,
        quality: input.quality,
        contradictions: input.contradictions,
        reflection_issues: input.reflection_issues,
        revision_actions: input.revision_actions,
        stored_memory_id: input.stored_memory_id,
        router_threshold_after: input.router_threshold_after,
        stream_windows: input.stream_windows,
        route_budget: input.route_budget,
        hierarchy: input.hierarchy,
        used_memory_ids: input.used_memory_ids,
        gist_records: input.gist_records,
        gist_memory_ids: input.gist_memory_ids,
        stored_runtime_kv_memory_ids: input.stored_runtime_kv_memory_ids,
        runtime_diagnostics: input.runtime_diagnostics,
        runtime_token_metrics: input.runtime_token_metrics,
        process_reward: input.process_reward,
        live_evolution: input.live_evolution,
    }
}

#[test]
fn planner_carries_recursive_runtime_calls() {
    let planner = ExperienceReplayPlanner::new();
    let records = vec![record(9, 0.88, RewardAction::Reinforce)];

    let plan = planner.plan(&records, 1);

    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].recursive_runtime_calls, Some(7));
    assert_eq!(plan.items[0].recursive_stats.unwrap().chunks, Some(4));
    assert_eq!(plan.items[0].live_evolution.memory_reinforcements, 2);
    assert_eq!(plan.items[0].live_evolution.stored_runtime_kv_memories, 1);
}

#[test]
fn report_summarizes_structured_live_evolution_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let reinforced = record(9, 0.88, RewardAction::Reinforce);
    let penalized = record(10, 0.12, RewardAction::Penalize);
    let plan = planner.plan(&[reinforced, penalized], 4);

    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(report.live_evolution_items, 2);
    assert_eq!(report.live_evolution_router_threshold_mutations, 2);
    assert_eq!(report.live_evolution_hierarchy_weight_mutations, 2);
    assert!((report.live_evolution_router_threshold_delta - 0.04).abs() < 0.0001);
    assert!((report.live_evolution_hierarchy_weight_delta - 0.06).abs() < 0.0001);
    assert_eq!(report.live_evolution_online_reward_feedbacks, 2);
    assert_eq!(report.live_evolution_online_reward_reinforcements, 1);
    assert_eq!(report.live_evolution_online_reward_penalties, 1);
    assert!((report.live_evolution_online_reward_strength - 1.40).abs() < 0.0001);
    assert!((report.live_evolution_online_reward_reinforcement_strength - 0.80).abs() < 0.0001);
    assert!((report.live_evolution_online_reward_penalty_strength - 0.60).abs() < 0.0001);
    assert_eq!(report.live_evolution_memory_updates, 4);
    assert_eq!(report.live_evolution_stored_memory_updates, 6);
    assert_eq!(report.live_evolution_reflection_issues, 1);
    assert_eq!(report.live_evolution_critical_reflection_issues, 1);
    assert_eq!(report.live_evolution_revision_actions, 1);
    assert!(report.summary().contains("live_evolution_items=2"));
    assert!(
        report
            .summary()
            .contains("live_evolution_online_reward_feedbacks=2")
    );
    assert!(
        report
            .summary()
            .contains("live_evolution_online_reward_strength=1.400000")
    );
    assert!(report.summary().contains("live_evolution_memory_updates=4"));
}

#[test]
fn report_summarizes_recursive_call_pressure() {
    let planner = ExperienceReplayPlanner::new();
    let mut high_cost = record(9, 0.88, RewardAction::Reinforce);
    high_cost.process_reward.notes =
        vec!["recursive:chunks=32:merge_rounds=2:waves=8:parallel=2:runtime_calls=96".to_owned()];
    let plan = planner.plan(&[high_cost], 1);

    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(report.recursive_runtime_items, 1);
    assert_eq!(report.recursive_runtime_calls, 96);
    assert!(report.average_recursive_call_pressure > 0.0);
    assert_eq!(
        report.average_recursive_call_pressure,
        report.max_recursive_call_pressure
    );
    assert!(report.summary().contains("router_updates=0"));
    assert!(report.summary().contains("hierarchy_updates=0"));
    assert!(report.summary().contains("memory_reinforcements=0"));
    assert!(report.summary().contains("memory_penalties=0"));
    assert!(report.summary().contains("recursive_runtime_calls=96"));
    assert!(report.summary().contains("max_recursive_call_pressure="));
}

#[test]
fn report_summarizes_runtime_kv_budget_pressure_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let clean = record(9, 0.88, RewardAction::Reinforce);
    let mut pressured = record(10, 0.86, RewardAction::Reinforce);
    pressured
        .runtime_diagnostics
        .budget_limited_runtime_kv_imports_skipped = 8;

    let plan = planner.plan(&[clean, pressured], 2);
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(report.runtime_kv_budget_pressure_items, 1);
    assert!((report.average_runtime_kv_budget_pressure - 0.4).abs() < 0.0001);
    assert!((report.max_runtime_kv_budget_pressure - 0.8).abs() < 0.0001);
    assert!(
        report
            .summary()
            .contains("runtime_kv_budget_pressure_items=1")
    );
    assert!(
        report
            .summary()
            .contains("avg_runtime_kv_budget_pressure=0.400")
    );
    assert!(
        report
            .summary()
            .contains("max_runtime_kv_budget_pressure=0.800")
    );
}

#[test]
fn report_summarizes_runtime_kv_weak_import_pressure_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let clean = record(9, 0.88, RewardAction::Reinforce);
    let mut weak = record(10, 0.86, RewardAction::Reinforce);
    weak.runtime_diagnostics.imported_kv_blocks = 1;
    weak.runtime_diagnostics.weak_runtime_kv_imports_skipped = 3;

    let plan = planner.plan(&[clean, weak], 2);
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(report.runtime_kv_weak_import_pressure_items, 1);
    assert!((report.average_runtime_kv_weak_import_pressure - 0.375).abs() < 0.0001);
    assert!((report.max_runtime_kv_weak_import_pressure - 0.75).abs() < 0.0001);
    assert!(
        report
            .summary()
            .contains("runtime_kv_weak_import_pressure_items=1")
    );
    assert!(
        report
            .summary()
            .contains("avg_runtime_kv_weak_import_pressure=0.375")
    );
    assert!(
        report
            .summary()
            .contains("max_runtime_kv_weak_import_pressure=0.750")
    );
}

#[test]
fn report_summarizes_live_memory_feedback_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let reinforced = record(9, 0.88, RewardAction::Reinforce);
    let mut penalized = record(10, 0.12, RewardAction::Penalize);
    penalized.process_reward.notes = vec![
            "memory_feedback:reinforced=0:penalized=3:reinforcement_amount=0.000000:penalty_amount=1.500000:applied=2:removed=1:missing=1:strength_delta=0.660000"
                .to_owned(),
        ];

    let plan = planner.plan(&[reinforced, penalized], 4);
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(report.live_memory_feedback_items, 2);
    assert_eq!(report.live_memory_feedback_updates, 5);
    assert_eq!(report.live_memory_feedback_reinforcements, 2);
    assert_eq!(report.live_memory_feedback_penalties, 3);
    assert_eq!(report.live_memory_feedback_detail_items, 2);
    assert_eq!(report.live_memory_feedback_applied, 4);
    assert_eq!(report.live_memory_feedback_removed, 1);
    assert_eq!(report.live_memory_feedback_missing, 1);
    assert!((report.live_memory_feedback_strength_delta - 1.08).abs() < 0.0001);
    assert!(report.summary().contains("live_memory_feedback_updates=5"));
    assert!(
        report
            .summary()
            .contains("live_memory_feedback_detail_items=2")
    );
    assert!(report.summary().contains("live_memory_feedback_applied=4"));
}

#[test]
fn report_summarizes_aggregated_live_memory_feedback_notes() {
    let planner = ExperienceReplayPlanner::new();
    let mut reinforced = record(9, 0.88, RewardAction::Reinforce);
    reinforced.process_reward.notes = vec![
        "memory_feedback:reinforced=2:penalized=0:reinforcement_amount=1.200000:penalty_amount=0.000000:applied=2:removed=0:missing=0:strength_delta=0.420000"
            .to_owned(),
        "memory_feedback:reinforced=0:penalized=1:reinforcement_amount=0.000000:penalty_amount=0.500000:applied=0:removed=0:missing=1:strength_delta=0.060000"
            .to_owned(),
    ];

    let plan = planner.plan(&[reinforced], 4);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(item.live_memory_feedback.unwrap().updates(), 3);
    assert_eq!(report.live_memory_feedback_items, 1);
    assert_eq!(report.live_memory_feedback_updates, 3);
    assert_eq!(report.live_memory_feedback_reinforcements, 2);
    assert_eq!(report.live_memory_feedback_penalties, 1);
    assert_eq!(report.live_memory_feedback_detail_items, 1);
    assert_eq!(report.live_memory_feedback_applied, 2);
    assert_eq!(report.live_memory_feedback_missing, 1);
    assert!((report.live_memory_feedback_strength_delta - 0.48).abs() < 0.0001);
    assert!(report.summary().contains("live_memory_feedback_updates=3"));
}

#[test]
fn report_summarizes_full_width_live_memory_feedback_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let mut reinforced = record(9, 0.88, RewardAction::Reinforce);
    reinforced.process_reward.notes = vec![
            "memory_feedback：reinforced＝2：penalized＝0：reinforcement_amount＝1.200000：penalty_amount＝0.000000：applied＝2：removed＝0：missing＝0：strength_delta＝0.420000"
                .to_owned(),
        ];

    let plan = planner.plan(&[reinforced], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(
        item.live_memory_feedback,
        Some(LiveMemoryFeedbackStats {
            reinforced: 2,
            penalized: 0,
            reinforcement_amount: 1.2,
            penalty_amount: 0.0,
            applied: 2,
            removed: 0,
            missing: 0,
            strength_delta: 0.42,
            detailed_evidence: true,
        })
    );
    assert_eq!(report.live_memory_feedback_items, 1);
    assert_eq!(report.live_memory_feedback_updates, 2);
    assert_eq!(report.live_memory_feedback_reinforcements, 2);
    assert_eq!(report.live_memory_feedback_penalties, 0);
    assert_eq!(report.live_memory_feedback_detail_items, 1);
    assert_eq!(report.live_memory_feedback_applied, 2);
    assert_eq!(report.live_memory_feedback_missing, 0);
    assert!((report.live_memory_feedback_strength_delta - 0.42).abs() < 0.0001);
    assert!(report.summary().contains("live_memory_feedback_updates=2"));
}

#[test]
fn report_summarizes_rust_check_feedback_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(11, 0.91, RewardAction::Reinforce);
    checked.process_reward.notes = vec![
            "rust_check:passed=true:label=rustc_passed:edition=2021:status_code=0:diagnostic_chars=0"
                .to_owned(),
            "Memory_Feedback: Rust_Check :reinforced=2:penalized=0:reinforcement_amount=0.900000:penalty_amount=0.000000:applied=2:removed=0:missing=0:strength_delta=0.180000"
                .to_owned(),
        ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(
        item.rust_check_stats,
        Some(RustCheckReplayStats {
            passed: 1,
            failed: 0,
            diagnostic_chars: 0,
        })
    );
    assert_eq!(item.rust_check_live_memory_feedback.unwrap().updates(), 2);
    assert_eq!(report.rust_check_items, 1);
    assert_eq!(report.rust_check_passed, 1);
    assert_eq!(report.rust_check_failed, 0);
    assert_eq!(report.rust_check_live_memory_feedback_items, 1);
    assert_eq!(report.rust_check_live_memory_feedback_updates, 2);
    assert_eq!(report.rust_check_live_memory_feedback_applied, 2);
    assert_eq!(report.rust_check_live_memory_feedback_missing, 0);
    assert!((report.rust_check_live_memory_feedback_strength_delta - 0.18).abs() < 0.0001);
    assert!(report.summary().contains("rust_check_items=1"));
    assert!(
        report
            .summary()
            .contains("rust_check_live_memory_feedback_updates=2")
    );
}

#[test]
fn report_summarizes_full_width_rust_check_feedback_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(11, 0.91, RewardAction::Reinforce);
    checked.process_reward.notes = vec![
            "ｒｕｓｔ＿ｃｈｅｃｋ：ｐａｓｓｅｄ＝ｔｒｕｅ：ｌａｂｅｌ＝rustc_passed：ｅｄｉｔｉｏｎ＝２０２１：ｓｔａｔｕｓ＿ｃｏｄｅ＝０：ｄｉａｇｎｏｓｔｉｃ＿ｃｈａｒｓ＝０"
                .to_owned(),
            "Ｍｅｍｏｒｙ＿Ｆｅｅｄｂａｃｋ： Ｒｕｓｔ＿Ｃｈｅｃｋ ：ｒｅｉｎｆｏｒｃｅｄ＝２：ｐｅｎａｌｉｚｅｄ＝０：ｒｅｉｎｆｏｒｃｅｍｅｎｔ＿ａｍｏｕｎｔ＝０．９０００００：ｐｅｎａｌｔｙ＿ａｍｏｕｎｔ＝０．００００００：ａｐｐｌｉｅｄ＝２：ｒｅｍｏｖｅｄ＝０：ｍｉｓｓｉｎｇ＝０：ｓｔｒｅｎｇｔｈ＿ｄｅｌｔａ＝０．１８００００"
                .to_owned(),
        ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(
        item.rust_check_stats,
        Some(RustCheckReplayStats {
            passed: 1,
            failed: 0,
            diagnostic_chars: 0,
        })
    );
    assert_eq!(item.rust_check_live_memory_feedback.unwrap().updates(), 2);
    assert_eq!(report.rust_check_items, 1);
    assert_eq!(report.rust_check_passed, 1);
    assert_eq!(report.rust_check_failed, 0);
    assert_eq!(report.rust_check_live_memory_feedback_items, 1);
    assert_eq!(report.rust_check_live_memory_feedback_updates, 2);
    assert_eq!(report.rust_check_live_memory_feedback_applied, 2);
    assert_eq!(report.rust_check_live_memory_feedback_missing, 0);
    assert!((report.rust_check_live_memory_feedback_strength_delta - 0.18).abs() < 0.0001);
    assert!(report.summary().contains("rust_check_items=1"));
    assert!(
        report
            .summary()
            .contains("rust_check_live_memory_feedback_updates=2")
    );
}

#[test]
fn rust_check_stats_ignore_missing_or_malformed_bool_outcomes() {
    let stats = RustCheckReplayStats::from_notes(&[
        "rust_check:label=legacy_rustc:diagnostic_chars=11".to_owned(),
        "rust_check:passed=maybe:label=broken_rustc:diagnostic_chars=7".to_owned(),
        "rust_check:passed=false:label=rustc_failed:diagnostic_chars=13".to_owned(),
    ])
    .unwrap();

    assert_eq!(
        stats,
        RustCheckReplayStats {
            passed: 0,
            failed: 1,
            diagnostic_chars: 31,
        }
    );
}

#[test]
fn rust_check_stats_keep_diagnostics_without_outcome() {
    let stats = RustCheckReplayStats::from_notes(&[
        "rust_check:label=legacy_rustc:diagnostic_chars=11".to_owned(),
        "rust_check:passed=maybe:label=broken_rustc:diagnostic_chars=7".to_owned(),
    ])
    .unwrap();

    assert_eq!(
        stats,
        RustCheckReplayStats {
            passed: 0,
            failed: 0,
            diagnostic_chars: 18,
        }
    );
}

#[test]
fn report_summarizes_rust_check_diagnostics_without_outcome() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(11, 0.91, RewardAction::Reinforce);
    checked.process_reward.notes = vec![
        "rust_check:label=legacy_rustc:diagnostic_chars=11".to_owned(),
        "rust_check:passed=maybe:label=broken_rustc:diagnostic_chars=7".to_owned(),
    ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(
        item.rust_check_stats,
        Some(RustCheckReplayStats {
            passed: 0,
            failed: 0,
            diagnostic_chars: 18,
        })
    );
    assert_eq!(report.rust_check_items, 1);
    assert_eq!(report.rust_check_passed, 0);
    assert_eq!(report.rust_check_failed, 0);
    assert_eq!(report.rust_check_diagnostic_chars, 18);
    assert!(report.summary().contains("rust_check_items=1"));
    assert!(report.summary().contains("rust_check_diagnostic_chars=18"));
}

#[test]
fn planner_replays_neutral_rust_check_diagnostics_without_outcome_as_hold() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(11, 0.65, RewardAction::Hold);
    checked.process_reward.notes = vec![
        "rust_check:label=legacy_rustc:diagnostic_chars=11".to_owned(),
        "rust_check:passed=maybe:label=broken_rustc:diagnostic_chars=7".to_owned(),
    ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(item.action, RewardAction::Hold);
    assert_eq!(
        item.rust_check_stats,
        Some(RustCheckReplayStats {
            passed: 0,
            failed: 0,
            diagnostic_chars: 18,
        })
    );
    assert_eq!(report.planned, 1);
    assert_eq!(report.rust_check_items, 1);
    assert_eq!(report.rust_check_passed, 0);
    assert_eq!(report.rust_check_failed, 0);
    assert_eq!(report.rust_check_diagnostic_chars, 18);
    assert!(report.summary().contains("rust_check_items=1"));
    assert!(report.summary().contains("rust_check_diagnostic_chars=18"));
}

#[test]
fn report_summarizes_business_contract_feedback_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(12, 0.93, RewardAction::Reinforce);
    checked.process_reward.notes = vec![
            "business_contract:case=gemma-service-runtime-model:passed= TRUE :required=4:matched=4:missing=0:has_runtime_model_experiences=true:protocol_leak=false:substituted_runtime_model_experiences=false:evasive_denial=false:handling_signal=true:raw_passed= FALSE :normalization= Sanitized :response_normalized= TRUE :canonical_fallback= FALSE "
                .to_owned(),
        ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(
        item.business_contract_stats,
        Some(BusinessContractReplayStats {
            passed: 1,
            failed: 0,
            raw_passed: 0,
            raw_failed: 1,
            response_normalized: 1,
            sanitized: 1,
            canonical_fallbacks: 0,
        })
    );
    assert_eq!(report.business_contract_items, 1);
    assert_eq!(report.business_contract_passed, 1);
    assert_eq!(report.business_contract_failed, 0);
    assert_eq!(report.business_contract_raw_passed, 0);
    assert_eq!(report.business_contract_raw_failed, 1);
    assert_eq!(report.business_contract_response_normalized, 1);
    assert_eq!(report.business_contract_sanitized, 1);
    assert_eq!(report.business_contract_canonical_fallbacks, 0);
    assert!(report.summary().contains("business_contract_items=1"));
    assert!(report.summary().contains("business_contract_raw_failed=1"));
    assert!(report.summary().contains("business_contract_sanitized=1"));
}

#[test]
fn report_summarizes_full_width_business_contract_feedback_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(12, 0.93, RewardAction::Reinforce);
    checked.process_reward.notes = vec![
            "business_contract：case＝gemma-service-runtime-model：passed＝ TRUE ：required＝4：matched＝4：missing＝0：has_runtime_model_experiences＝true：protocol_leak＝false：substituted_runtime_model_experiences＝false：evasive_denial＝false：handling_signal＝true：raw_passed＝ FALSE ：normalization＝ Sanitized ：response_normalized＝ TRUE ：canonical_fallback＝ FALSE "
                .to_owned(),
        ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(
        item.business_contract_stats,
        Some(BusinessContractReplayStats {
            passed: 1,
            failed: 0,
            raw_passed: 0,
            raw_failed: 1,
            response_normalized: 1,
            sanitized: 1,
            canonical_fallbacks: 0,
        })
    );
    assert_eq!(report.business_contract_items, 1);
    assert_eq!(report.business_contract_passed, 1);
    assert_eq!(report.business_contract_failed, 0);
    assert_eq!(report.business_contract_raw_passed, 0);
    assert_eq!(report.business_contract_raw_failed, 1);
    assert_eq!(report.business_contract_response_normalized, 1);
    assert_eq!(report.business_contract_sanitized, 1);
    assert_eq!(report.business_contract_canonical_fallbacks, 0);
    assert!(report.summary().contains("business_contract_items=1"));
    assert!(report.summary().contains("business_contract_raw_failed=1"));
    assert!(report.summary().contains("business_contract_sanitized=1"));
}

#[test]
fn business_contract_stats_ignore_missing_or_malformed_bool_outcomes() {
    let stats = BusinessContractReplayStats::from_notes(&[
        "business_contract:case=legacy-audit:required=4:matched=3:normalization=sanitized"
            .to_owned(),
        "business_contract:case=broken-audit:passed=maybe:raw_passed=:response_normalized=true"
            .to_owned(),
        "business_contract:case=explicit-audit:passed=false:raw_passed=true".to_owned(),
    ])
    .unwrap();

    assert_eq!(
        stats,
        BusinessContractReplayStats {
            passed: 0,
            failed: 1,
            raw_passed: 1,
            raw_failed: 0,
            response_normalized: 1,
            sanitized: 1,
            canonical_fallbacks: 0,
        }
    );
}

#[test]
fn business_contract_stats_keep_normalization_evidence_without_outcome() {
    let stats = BusinessContractReplayStats::from_notes(&[
        "business_contract:case=legacy-audit:normalization=sanitized".to_owned(),
        "business_contract:case=broken-audit:passed=maybe:raw_passed=:response_normalized=true:canonical_fallback=true"
            .to_owned(),
    ])
    .unwrap();

    assert_eq!(
        stats,
        BusinessContractReplayStats {
            passed: 0,
            failed: 0,
            raw_passed: 0,
            raw_failed: 0,
            response_normalized: 1,
            sanitized: 1,
            canonical_fallbacks: 1,
        }
    );
}

#[test]
fn planner_replays_neutral_business_contract_audit_without_outcome_as_hold() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(12, 0.65, RewardAction::Hold);
    checked.process_reward.notes = vec![
        "business_contract:case=legacy-audit:normalization=sanitized".to_owned(),
        "business_contract:case=broken-audit:passed=maybe:raw_passed=:response_normalized=true:canonical_fallback=true"
            .to_owned(),
    ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(item.action, RewardAction::Hold);
    assert_eq!(
        item.business_contract_stats,
        Some(BusinessContractReplayStats {
            passed: 0,
            failed: 0,
            raw_passed: 0,
            raw_failed: 0,
            response_normalized: 1,
            sanitized: 1,
            canonical_fallbacks: 1,
        })
    );
    assert_eq!(report.planned, 1);
    assert_eq!(report.business_contract_items, 1);
    assert_eq!(report.business_contract_passed, 0);
    assert_eq!(report.business_contract_failed, 0);
    assert_eq!(report.business_contract_response_normalized, 1);
    assert_eq!(report.business_contract_sanitized, 1);
    assert_eq!(report.business_contract_canonical_fallbacks, 1);
    assert!(report.summary().contains("business_contract_items=1"));
    assert!(
        report
            .summary()
            .contains("business_contract_response_normalized=1")
    );
}

#[test]
fn report_summarizes_pool_dispatch_feedback_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(13, 0.94, RewardAction::Reinforce);
    checked.process_reward.notes = vec![
        "pool_dispatch:selected_role= Review :selected_port=8688:selected_endpoint=http://127.0.0.1:8688:context_window=8192:default_max_tokens=1024:configured_max_tokens=4096:effective_max_tokens=1024:max_tokens_clamped=true:low_priority=true:forwarded=true:dispatch_mode=runtime_endpoint_override:dispatch_reason=selected_worker_ready"
            .to_owned(),
        "pool_dispatch:selected_role=review:selected_port=8689:selected_endpoint=http://127.0.0.1:8689:context_window=8192:default_max_tokens=1024:configured_max_tokens=1024:effective_max_tokens=1024:max_tokens_clamped=false:low_priority=false:forwarded=false:dispatch_mode=runtime_endpoint_override:dispatch_reason=worker_busy"
            .to_owned(),
    ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(
        item.pool_dispatch_stats,
        Some(PoolDispatchReplayStats {
            items: 2,
            forwarded: 1,
            clamped: 1,
            low_priority: 1,
            selected_roles: vec!["review".to_owned()],
        })
    );
    assert_eq!(report.pool_dispatch_items, 2);
    assert_eq!(report.pool_dispatch_forwarded, 1);
    assert_eq!(report.pool_dispatch_clamped, 1);
    assert_eq!(report.pool_dispatch_low_priority, 1);
    assert!(report.summary().contains("pool_dispatch_items=2"));
    assert!(report.summary().contains("pool_dispatch_forwarded=1"));
}

#[test]
fn report_summarizes_full_width_pool_dispatch_feedback_consumed_by_replay() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(13, 0.94, RewardAction::Reinforce);
    checked.process_reward.notes = vec![
        "ｐｏｏｌ＿ｄｉｓｐａｔｃｈ：ｓｅｌｅｃｔｅｄ＿ｒｏｌｅ＝ Ｒｅｖｉｅｗ ：ｓｅｌｅｃｔｅｄ＿ｐｏｒｔ＝８６８８：ｓｅｌｅｃｔｅｄ＿ｅｎｄｐｏｉｎｔ＝http://127.0.0.1:8688：ｃｏｎｔｅｘｔ＿ｗｉｎｄｏｗ＝８１９２：ｄｅｆａｕｌｔ＿ｍａｘ＿ｔｏｋｅｎｓ＝１０２４：ｃｏｎｆｉｇｕｒｅｄ＿ｍａｘ＿ｔｏｋｅｎｓ＝４０９６：ｅｆｆｅｃｔｉｖｅ＿ｍａｘ＿ｔｏｋｅｎｓ＝１０２４：ｍａｘ＿ｔｏｋｅｎｓ＿ｃｌａｍｐｅｄ＝ｔｒｕｅ：ｌｏｗ＿ｐｒｉｏｒｉｔｙ＝ｔｒｕｅ：ｆｏｒｗａｒｄｅｄ＝ｔｒｕｅ：ｄｉｓｐａｔｃｈ＿ｍｏｄｅ＝runtime_endpoint_override：ｄｉｓｐａｔｃｈ＿ｒｅａｓｏｎ＝selected_worker_ready"
            .to_owned(),
        "ｐｏｏｌ＿ｄｉｓｐａｔｃｈ：ｓｅｌｅｃｔｅｄ＿ｒｏｌｅ＝ｒｅｖｉｅｗ：ｓｅｌｅｃｔｅｄ＿ｐｏｒｔ＝８６８９：ｓｅｌｅｃｔｅｄ＿ｅｎｄｐｏｉｎｔ＝http://127.0.0.1:8689：ｃｏｎｔｅｘｔ＿ｗｉｎｄｏｗ＝８１９２：ｄｅｆａｕｌｔ＿ｍａｘ＿ｔｏｋｅｎｓ＝１０２４：ｃｏｎｆｉｇｕｒｅｄ＿ｍａｘ＿ｔｏｋｅｎｓ＝１０２４：ｅｆｆｅｃｔｉｖｅ＿ｍａｘ＿ｔｏｋｅｎｓ＝１０２４：ｍａｘ＿ｔｏｋｅｎｓ＿ｃｌａｍｐｅｄ＝ｆａｌｓｅ：ｌｏｗ＿ｐｒｉｏｒｉｔｙ＝ｆａｌｓｅ：ｆｏｒｗａｒｄｅｄ＝ｆａｌｓｅ：ｄｉｓｐａｔｃｈ＿ｍｏｄｅ＝runtime_endpoint_override：ｄｉｓｐａｔｃｈ＿ｒｅａｓｏｎ＝worker_busy"
            .to_owned(),
    ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(
        item.pool_dispatch_stats,
        Some(PoolDispatchReplayStats {
            items: 2,
            forwarded: 1,
            clamped: 1,
            low_priority: 1,
            selected_roles: vec!["review".to_owned()],
        })
    );
    assert_eq!(report.pool_dispatch_items, 2);
    assert_eq!(report.pool_dispatch_forwarded, 1);
    assert_eq!(report.pool_dispatch_clamped, 1);
    assert_eq!(report.pool_dispatch_low_priority, 1);
    assert!(report.summary().contains("pool_dispatch_items=2"));
    assert!(report.summary().contains("pool_dispatch_forwarded=1"));
}

#[test]
fn report_keeps_pool_dispatch_items_without_counting_malformed_bool_flags() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(13, 0.94, RewardAction::Reinforce);
    checked.process_reward.notes = vec![
        "pool_dispatch:selected_role= Review :max_tokens_clamped=maybe:low_priority=:forwarded=yes:dispatch_reason=malformed_flags"
            .to_owned(),
        "pool_dispatch:selected_role=summary:max_tokens_clamped=false:low_priority=false:forwarded=false:dispatch_reason=explicit_false"
            .to_owned(),
        "ｐｏｏｌ＿ｄｉｓｐａｔｃｈ：ｓｅｌｅｃｔｅｄ＿ｒｏｌｅ＝ Ｒｅｖｉｅｗ ：ｍａｘ＿ｔｏｋｅｎｓ＿ｃｌａｍｐｅｄ＝ TRUE ：ｌｏｗ＿ｐｒｉｏｒｉｔｙ＝ TRUE ：ｆｏｒｗａｒｄｅｄ＝ TRUE ：ｄｉｓｐａｔｃｈ＿ｒｅａｓｏｎ＝full_width_true"
            .to_owned(),
    ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(
        item.pool_dispatch_stats,
        Some(PoolDispatchReplayStats {
            items: 3,
            forwarded: 1,
            clamped: 1,
            low_priority: 1,
            selected_roles: vec!["review".to_owned(), "summary".to_owned()],
        })
    );
    assert_eq!(report.pool_dispatch_items, 3);
    assert_eq!(report.pool_dispatch_forwarded, 1);
    assert_eq!(report.pool_dispatch_clamped, 1);
    assert_eq!(report.pool_dispatch_low_priority, 1);
    assert!(report.summary().contains("pool_dispatch_items=3"));
    assert!(report.summary().contains("pool_dispatch_forwarded=1"));
}

#[test]
fn planner_replays_neutral_business_contract_audit_as_hold() {
    let planner = ExperienceReplayPlanner::new();
    let mut checked = record(12, 0.65, RewardAction::Hold);
    checked.process_reward.notes = vec![
            "business_contract:case=gemma-business-runtime:passed=true:required=4:matched=4:missing=0:has_runtime_model_experiences=true:protocol_leak=false:substituted_runtime_model_experiences=false:evasive_denial=false:handling_signal=true:raw_passed=false:normalization=canonical_fallback:response_normalized=true:canonical_fallback=true"
                .to_owned(),
        ];

    let plan = planner.plan(&[checked], 1);
    let item = &plan.items[0];
    let report = ExperienceReplayReport::from_plan(&plan);

    assert_eq!(item.action, RewardAction::Hold);
    assert_eq!(
        item.business_contract_stats,
        Some(BusinessContractReplayStats {
            passed: 1,
            failed: 0,
            raw_passed: 0,
            raw_failed: 1,
            response_normalized: 1,
            sanitized: 0,
            canonical_fallbacks: 1,
        })
    );
    assert_eq!(report.planned, 1);
    assert_eq!(report.business_contract_items, 1);
    assert_eq!(report.business_contract_passed, 1);
    assert_eq!(report.business_contract_raw_failed, 1);
    assert_eq!(report.business_contract_canonical_fallbacks, 1);
}

#[test]
fn live_memory_feedback_stats_keep_legacy_notes_without_detail_evidence() {
    let stats = LiveMemoryFeedbackStats::from_notes(&[
            "memory_feedback:reinforced=1:penalized=0:reinforcement_amount=0.800000:penalty_amount=0.000000"
                .to_owned(),
        ])
        .unwrap();

    assert_eq!(stats.updates(), 1);
    assert_eq!(stats.applied, 0);
    assert!(!stats.has_detailed_update_evidence());
    assert_eq!(stats.applied_ratio(), None);
}

#[test]
fn live_memory_feedback_stats_aggregate_multiple_notes_for_one_experience() {
    let stats = LiveMemoryFeedbackStats::from_notes(&[
        "memory_feedback:reinforced=2:penalized=0:reinforcement_amount=1.200000:penalty_amount=0.000000:applied=2:removed=0:missing=0:strength_delta=0.420000"
            .to_owned(),
        "memory_feedback:reinforced=0:penalized=1:reinforcement_amount=0.000000:penalty_amount=0.500000:applied=0:removed=0:missing=1:strength_delta=0.060000"
            .to_owned(),
        "memory_feedback:reinforced=0:penalized=0:reinforcement_amount=0.000000:penalty_amount=0.000000:applied=0:removed=0:missing=0:strength_delta=0.000000"
            .to_owned(),
    ])
    .unwrap();

    assert_eq!(stats.reinforced, 2);
    assert_eq!(stats.penalized, 1);
    assert_eq!(stats.updates(), 3);
    assert!((stats.reinforcement_amount - 1.2).abs() < 0.0001);
    assert!((stats.penalty_amount - 0.5).abs() < 0.0001);
    assert_eq!(stats.applied, 2);
    assert_eq!(stats.removed, 0);
    assert_eq!(stats.missing, 1);
    assert!((stats.strength_delta - 0.48).abs() < 0.0001);
    assert!(stats.has_detailed_update_evidence());
}

#[test]
fn live_memory_feedback_stats_aggregate_only_matching_source_notes() {
    let stats = LiveMemoryFeedbackStats::from_notes_for_source(
        &[
            "memory_feedback:rust_check:reinforced=1:penalized=0:reinforcement_amount=0.300000:penalty_amount=0.000000:applied=1:removed=0:missing=0:strength_delta=0.050000"
                .to_owned(),
            "memory_feedback:planner:reinforced=0:penalized=4:reinforcement_amount=0.000000:penalty_amount=2.000000:applied=4:removed=0:missing=0:strength_delta=0.400000"
                .to_owned(),
            "memory_feedback:rust_check:reinforced=0:penalized=1:reinforcement_amount=0.000000:penalty_amount=0.600000:applied=0:removed=0:missing=1:strength_delta=0.070000"
                .to_owned(),
        ],
        "rust_check",
    )
    .unwrap();

    assert_eq!(stats.reinforced, 1);
    assert_eq!(stats.penalized, 1);
    assert_eq!(stats.updates(), 2);
    assert!((stats.reinforcement_amount - 0.3).abs() < 0.0001);
    assert!((stats.penalty_amount - 0.6).abs() < 0.0001);
    assert_eq!(stats.applied, 1);
    assert_eq!(stats.missing, 1);
    assert!((stats.strength_delta - 0.12).abs() < 0.0001);
}
