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
    assert!(
        plan.blueprints
            .iter()
            .all(|blueprint| blueprint.provenance.starts_with("toolsmith-planner:v1"))
    );
    assert!(!plan.memory_admission_candidates().is_empty());
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

#[test]
fn holds_runtime_adapter_until_contract_review() {
    let planner = ToolsmithPlanner::new();
    let plan = planner.plan(ToolsmithInput {
        prompt: "build a rust runtime adapter probe tool",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &HardwarePlan::default(),
    });

    assert_eq!(plan.held_count(), 1);
    assert_eq!(
        plan.blueprint_count(),
        plan.ready_count() + plan.held_count()
    );
    assert!(
        plan.blueprints
            .iter()
            .any(|blueprint| blueprint.status == ToolBuildStatus::Held)
    );
}

#[test]
fn tracks_duplicate_blueprints_without_promoting_them() {
    let planner = ToolsmithPlanner::new();
    let plan = planner.plan(ToolsmithInput {
        prompt: "duplicate rust cli tool for trace analysis",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &HardwarePlan::default(),
    });

    assert_eq!(plan.duplicate_count(), 1);
    assert!(!plan.passed_rust_gate());
    assert_eq!(plan.ready_count(), plan.blueprint_count() - 1);
    assert!(
        plan.blueprints
            .iter()
            .any(|blueprint| blueprint.status == ToolBuildStatus::Duplicate)
    );
}

#[test]
fn quarantines_failed_validation_blueprints() {
    let planner = ToolsmithPlanner::new();
    let plan = planner.plan(ToolsmithInput {
        prompt: "rust gate runner tool with failed-validation evidence",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &HardwarePlan::default(),
    });

    assert_eq!(plan.failed_validation_count(), 1);
    assert_eq!(plan.quarantined_count(), 1);
    assert!(!plan.passed_rust_gate());
    assert_eq!(plan.ready_count(), 0);
    assert!(plan.blueprints.iter().any(|blueprint| {
        blueprint
            .gate_notes
            .iter()
            .any(|note| note == "quarantined_no_runtime_default")
    }));
}

#[test]
fn memory_admission_candidates_do_not_leak_raw_prompt() {
    let secret_prompt = "rust trace tool for customer secret prompt do-not-leak";
    let planner = ToolsmithPlanner::new();
    let plan = planner.plan(ToolsmithInput {
        prompt: secret_prompt,
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &HardwarePlan::default(),
    });

    let candidates = plan.memory_admission_candidates();
    assert!(!candidates.is_empty());
    assert!(
        candidates
            .iter()
            .all(|candidate| !candidate.contains("do-not-leak"))
    );
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.contains("tool_reliability:"))
    );
}
