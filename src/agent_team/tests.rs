#![allow(clippy::field_reassign_with_default)]

use super::*;
use crate::hardware::HardwarePlan;
use crate::hierarchy::TaskProfile;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::router::RouteBudget;
use crate::toolsmith::ToolsmithPlan;

#[test]
fn plans_collision_free_team_for_subagent_prompt() {
    let planner = AgentTeamPlanner::new();
    let toolsmith_plan = ToolsmithPlan::default();
    let hardware_plan = HardwarePlan::default();
    let recursive_schedule = RecursiveSchedule::default();

    let plan = planner.plan(AgentTeamInput {
        prompt: "让他拥有一个子agent团队，把消息汇总给主线程快速进化",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &hardware_plan,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        recursive_schedule: &recursive_schedule,
        toolsmith_plan: &toolsmith_plan,
    });

    assert!(plan.enabled);
    assert!(plan.has_role(AgentRole::Aggregator));
    assert!(plan.collision_free());
    assert!(plan.agents.iter().all(|agent| !agent.writes_allowed));
    assert!(plan.summary().contains("collision_free=true"));
    assert_eq!(plan.aggregation.main_thread_writer, "main_thread");
    assert_eq!(plan.aggregation.lane_count, 7);
    assert_eq!(
        plan.aggregation.message_summaries.len(),
        plan.message_count()
    );
}

#[test]
fn aggregation_keeps_source_ids_and_confidence_for_final_messages() {
    let planner = AgentTeamPlanner::new().with_limits(7, 3);
    let toolsmith_plan = ToolsmithPlan::default();
    let hardware_plan = HardwarePlan::default();
    let recursive_schedule = RecursiveSchedule::default();

    let plan = planner.plan(AgentTeamInput {
        prompt: "agent team should aggregate sub-agent reports with source ids",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &hardware_plan,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        recursive_schedule: &recursive_schedule,
        toolsmith_plan: &toolsmith_plan,
    });

    assert_eq!(plan.message_count(), 3);
    assert_eq!(plan.aggregation.source_summaries.len(), 3);
    for (message, source) in plan
        .messages
        .iter()
        .zip(plan.aggregation.source_summaries.iter())
    {
        assert_eq!(source.source_id, message.id);
        assert_eq!(source.confidence, message.confidence);
        assert_eq!(source.role, message.role.as_str());
        assert_eq!(source.lane, message.lane);
    }
    assert!(plan.aggregation.source_summary_lines(1)[0].contains("source=agent-team-"));
}

#[test]
fn records_resolved_conflict_for_blocked_tool_surface() {
    let planner = AgentTeamPlanner::new();
    let mut toolsmith_plan = ToolsmithPlan::default();
    toolsmith_plan.rust_only = false;
    toolsmith_plan
        .rejected_requests
        .push("non_rust_tool_request_blocked".to_owned());
    let hardware_plan = HardwarePlan::default();
    let recursive_schedule = RecursiveSchedule::default();

    let plan = planner.plan(AgentTeamInput {
        prompt: "agent team should coordinate tool creation",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &hardware_plan,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        recursive_schedule: &recursive_schedule,
        toolsmith_plan: &toolsmith_plan,
    });

    assert_eq!(plan.unresolved_conflict_count(), 0);
    assert!(plan.conflict_summaries(1)[0].contains("tool_surface"));
    assert_eq!(plan.aggregation.conflict_topics, vec!["tool_surface"]);
    assert!(plan.aggregation.unresolved_conflict_topics.is_empty());
}

#[test]
fn aggregation_serializes_lanes_under_tight_parallel_budget() {
    let planner = AgentTeamPlanner::new();
    let toolsmith_plan = ToolsmithPlan::default();
    let mut hardware_plan = HardwarePlan::default();
    hardware_plan.execution.max_parallel_chunks = 1;
    let recursive_schedule = RecursiveSchedule::default();

    let plan = planner.plan(AgentTeamInput {
        prompt: "agent team coordinate read-only lanes and aggregate conflicts",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &hardware_plan,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.25,
        },
        recursive_schedule: &recursive_schedule,
        toolsmith_plan: &toolsmith_plan,
    });

    assert!(plan.enabled);
    assert_eq!(
        plan.aggregation.budget_scope,
        "serialized_read_only_lanes_under_main_thread"
    );
    assert_eq!(plan.aggregation.max_parallel_lanes, 1);
    assert_eq!(plan.aggregation.attention_fraction, 0.25);
    assert!(
        plan.reward_notes()
            .iter()
            .any(|note| note.contains("agent_team:aggregation:lanes="))
    );
}

#[test]
fn limited_team_drops_inactive_role_messages_conflicts_and_dependencies() {
    let planner = AgentTeamPlanner::new().with_limits(3, 12);
    let mut toolsmith_plan = ToolsmithPlan::default();
    toolsmith_plan.rust_only = false;
    toolsmith_plan
        .rejected_requests
        .push("non_rust_tool_request_blocked".to_owned());
    let hardware_plan = HardwarePlan::default();
    let recursive_schedule = RecursiveSchedule::default();

    let plan = planner.plan(AgentTeamInput {
        prompt: "agent team should coordinate tool creation",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &hardware_plan,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        recursive_schedule: &recursive_schedule,
        toolsmith_plan: &toolsmith_plan,
    });
    let active_roles = plan
        .agents
        .iter()
        .map(|agent| agent.role)
        .collect::<Vec<_>>();
    let active_role_names = plan
        .agents
        .iter()
        .map(|agent| agent.role.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        active_roles,
        vec![AgentRole::Planner, AgentRole::Researcher, AgentRole::Coder]
    );
    assert!(
        plan.messages
            .iter()
            .all(|message| active_roles.contains(&message.role))
    );
    assert!(plan.conflicts.iter().all(|conflict| {
        conflict
            .roles
            .iter()
            .all(|role| active_roles.contains(role))
    }));
    assert!(plan.agents.iter().all(|agent| {
        agent
            .dependencies
            .iter()
            .all(|dependency| active_role_names.iter().any(|role| role == dependency))
    }));
    assert!(plan.collision_free());
}

#[test]
fn disabled_plan_has_no_reward_notes() {
    let planner = AgentTeamPlanner::new();
    let toolsmith_plan = ToolsmithPlan::default();
    let hardware_plan = HardwarePlan::default();
    let recursive_schedule = RecursiveSchedule::default();

    let plan = planner.plan(AgentTeamInput {
        prompt: "ordinary answer without delegation",
        profile: TaskProfile::General,
        memories: &[],
        experiences: &[],
        hardware_plan: &hardware_plan,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        recursive_schedule: &recursive_schedule,
        toolsmith_plan: &toolsmith_plan,
    });

    assert!(!plan.enabled);
    assert!(plan.reward_notes().is_empty());
}

#[test]
fn handoff_sanitizer_trusts_validated_clean_handoff() {
    let sanitizer = AgentHandoffSanitizer::new();
    let context = handoff_context();
    let report = sanitizer.sanitize(
        &context,
        &[AgentHandoffInput::new(
            "019ee8af-a695-7a23-99f2-fc9d8c7a7ba4",
            AgentRole::Coder,
            "implemented context hygiene sanitizer with focused tests",
        )
        .with_touched_file("src/agent_team/handoff.rs")
        .with_validation("cargo test -q --package rust-norion agent_team")
        .with_issue("#32")
        .with_pr("#1")
        .with_claimed_branch("codex/r83-memory-admission-review-packets")
        .with_claimed_head("30c25654d")],
    );

    assert_eq!(report.trusted_handoffs, 1);
    assert_eq!(report.needs_review_handoffs, 0);
    assert_eq!(report.quarantined_handoffs, 0);
    assert!(report.can_influence_main_thread());
    assert_eq!(report.trusted_lessons().len(), 2);
    assert!(
        report
            .summary()
            .contains("preview_only=true can_influence=true")
    );
}

#[test]
fn handoff_sanitizer_holds_stale_or_unvalidated_handoff() {
    let sanitizer = AgentHandoffSanitizer::new();
    let context = handoff_context();
    let report = sanitizer.sanitize(
        &context,
        &[AgentHandoffInput::new(
            "019ee8af-da22-7d61-867d-f0d030d5921c",
            AgentRole::Researcher,
            "claims old roadmap facts are still current",
        )
        .with_stale_assumption("issue list was read before #32 existed")
        .with_unresolved_risk("CI status still unknown")
        .with_issue("#32")],
    );

    assert_eq!(report.trusted_handoffs, 0);
    assert_eq!(report.needs_review_handoffs, 1);
    assert!(!report.can_influence_main_thread());
    assert!(
        report
            .rejected_claims
            .iter()
            .any(|claim| claim == "handoff_validation_missing")
    );
    assert!(
        report
            .conflicts
            .iter()
            .any(|conflict| conflict.contains("stale_assumption:"))
    );
    assert!(
        report
            .conflicts
            .iter()
            .any(|conflict| conflict.contains("unresolved_risk:"))
    );
}

#[test]
fn handoff_sanitizer_flags_current_state_conflicts() {
    let sanitizer = AgentHandoffSanitizer::new();
    let mut context = handoff_context();
    context
        .dirty_files
        .push("src/agent_team/types.rs".to_owned());
    let report = sanitizer.sanitize(
        &context,
        &[AgentHandoffInput::new(
            "019ee8b0-05ee-7380-970e-8a684bf2b025",
            AgentRole::Reviewer,
            "reviewed agent team changes",
        )
        .with_touched_file("src\\agent_team\\types.rs")
        .with_validation("cargo check passed")
        .with_claimed_branch("old/context-branch")
        .with_claimed_head("deadbeef")],
    );

    assert_eq!(report.needs_review_handoffs, 1);
    assert!(
        report
            .conflicts
            .iter()
            .any(|conflict| conflict.contains("branch_mismatch:"))
    );
    assert!(
        report
            .conflicts
            .iter()
            .any(|conflict| conflict.contains("head_mismatch:"))
    );
    assert!(
        report
            .conflicts
            .iter()
            .any(|conflict| conflict.contains("touched_file_dirty_in_main:"))
    );
}

#[test]
fn handoff_sanitizer_quarantines_polluted_payload_without_leaking_raw_text() {
    let sanitizer = AgentHandoffSanitizer::new();
    let context = handoff_context();
    let report = sanitizer.sanitize(
        &context,
        &[AgentHandoffInput::new(
            "polluted window!",
            AgentRole::Aggregator,
            "raw prompt password=letmein sk-test-secret should not be retained",
        )
        .with_validation("cargo test passed")
        .with_raw_payload_present(true)
        .with_private_payload_present(true)],
    );

    assert_eq!(report.quarantined_handoffs, 1);
    assert_eq!(report.raw_payloads_blocked, 1);
    assert_eq!(report.private_payloads_blocked, 1);
    assert!(report.redactions >= 2);
    assert!(!report.can_influence_main_thread());
    assert!(report.accepted_facts.is_empty());
    let rendered = format!("{report:?}");
    assert!(!rendered.contains("letmein"));
    assert!(!rendered.contains("sk-test-secret"));
    assert!(
        report
            .rejected_claims
            .iter()
            .any(|claim| claim == "handoff_raw_or_private_payload_blocked")
    );
}

#[test]
fn handoff_sanitizer_detects_duplicate_agent_claims() {
    let sanitizer = AgentHandoffSanitizer::new();
    let context = handoff_context();
    let first = AgentHandoffInput::new(
        "source-a",
        AgentRole::Tester,
        "validated the agent handoff sanitizer",
    )
    .with_touched_file("src/agent_team/handoff.rs")
    .with_validation("cargo test passed");
    let second = AgentHandoffInput::new(
        "source-b",
        AgentRole::Tester,
        "validated the agent handoff sanitizer",
    )
    .with_touched_file("src/agent_team/handoff.rs")
    .with_validation("cargo test passed");
    let report = sanitizer.sanitize(&context, &[first, second]);

    assert_eq!(report.trusted_handoffs, 1);
    assert_eq!(report.needs_review_handoffs, 1);
    assert_eq!(report.duplicate_claims, 1);
    assert!(
        report
            .conflicts
            .iter()
            .any(|conflict| conflict.contains("duplicate_claim_fingerprint:"))
    );
}

fn handoff_context() -> AgentHandoffContext {
    AgentHandoffContext {
        current_branch: "codex/r83-memory-admission-review-packets".to_owned(),
        current_head: "30c25654d".to_owned(),
        dirty_files: Vec::new(),
        known_issue_refs: vec!["#32".to_owned()],
        known_pr_refs: vec!["#1".to_owned()],
    }
}
