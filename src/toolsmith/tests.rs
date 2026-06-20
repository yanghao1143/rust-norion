use super::*;
use crate::hardware::HardwarePlan;
use crate::hierarchy::TaskProfile;

#[test]
fn plans_rust_only_toolsmith_probe_for_exploration_prompt() {
    let planner = ToolsmithPlanner::new();
    let plan = planner.plan(ToolsmithInput {
        prompt: "我想让他有自己摸索造工具的能力 工具全部由rust写",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &HardwarePlan::default(),
    });

    assert!(plan.rust_only);
    assert!(plan.exploration_required);
    assert!(plan.passed_rust_gate());
    assert!(
        plan.blueprints
            .iter()
            .any(|blueprint| blueprint.id == "rust_toolsmith_probe")
    );
    assert!(
        plan.blueprints
            .iter()
            .all(|blueprint| blueprint.entrypoint.ends_with(".rs"))
    );
}

#[test]
fn blocks_non_rust_tool_requests() {
    let planner = ToolsmithPlanner::new();
    let plan = planner.plan(ToolsmithInput {
        prompt: "build a python tool for trace analysis",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &HardwarePlan::default(),
    });

    assert!(!plan.rust_only);
    assert!(!plan.passed_rust_gate());
    assert_eq!(
        plan.rejected_requests,
        vec!["non_rust_tool_request_blocked"]
    );
    assert!(
        plan.blueprints
            .iter()
            .all(|blueprint| blueprint.status == ToolBuildStatus::Rejected)
    );
}
