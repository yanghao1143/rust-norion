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
