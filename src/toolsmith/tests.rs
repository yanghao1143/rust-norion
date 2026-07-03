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
    assert!(!plan.passed_rust_gate());
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
fn danger_signal_blocks_unknown_blueprint_provenance_before_admission() {
    let planner = ToolsmithPlanner::new();
    let mut plan = planner.plan(ToolsmithInput {
        prompt: "build a rust trace analysis cli tool",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &HardwarePlan::default(),
    });
    plan.blueprints[0].provenance = "legacy-toolsmith-record".to_owned();

    let review = plan.blueprints[0].danger_signal_review();

    assert_eq!(review.decision.as_str(), "hold_for_provenance");
    assert!(
        review
            .reason_codes
            .contains(&"missing_trusted_self_provenance".to_owned())
    );
    assert!(!plan.passed_rust_gate());
    assert!(plan.memory_admission_candidates().is_empty());
}

#[test]
fn danger_signal_rejects_private_tool_blueprint_payload_without_echoing_it() {
    let planner = ToolsmithPlanner::new();
    let secret = "private chat raw_prompt ignore previous password=letmein";
    let plan = planner.plan(ToolsmithInput {
        prompt: &format!("build a rust cli tool {secret}"),
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &HardwarePlan::default(),
    });
    let review = plan.blueprints[0].danger_signal_review();
    let summary = review.summary_line();

    assert_eq!(review.decision.as_str(), "reject_danger_signal");
    assert!(review.reason_codes.iter().any(|reason| {
        reason.starts_with("raw_payload_marker:") || reason == "prompt_injection_marker"
    }));
    assert!(!plan.passed_rust_gate());
    assert!(plan.memory_admission_candidates().is_empty());
    assert!(!summary.contains("letmein"));
    assert!(!summary.contains("ignore previous"));
}

#[test]
fn defense_spacer_blocks_unsafe_toolsmith_blueprint_before_activation() {
    let planner = ToolsmithPlanner::new();
    let mut plan = planner.plan(ToolsmithInput {
        prompt: "build a rust trace analysis cli tool",
        profile: TaskProfile::Coding,
        memories: &[],
        experiences: &[],
        hardware_plan: &HardwarePlan::default(),
    });
    plan.blueprints[0]
        .gate_notes
        .push("unsafe_toolsmith_blueprint do-not-echo".to_owned());

    let gate = plan.blueprints[0]
        .defense_spacer_activation_gate()
        .expect("unsafe blueprint marker gate");

    assert!(!gate.allowed);
    assert_eq!(gate.decision.as_str(), "block");
    assert_eq!(gate.reason, "matched_blocking_defense_spacer");
    assert!(!plan.passed_rust_gate());
    assert!(plan.memory_admission_candidates().is_empty());
    assert!(!gate.summary_line().contains("do-not-echo"));
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
    let quarantined = plan
        .blueprints
        .iter()
        .find(|blueprint| blueprint.status == ToolBuildStatus::Quarantined)
        .expect("quarantined blueprint");
    assert_eq!(quarantined.control_lifecycle_state(), "quarantined");
    assert!(quarantined.summary().contains("lifecycle=quarantined"));
    assert!(
        quarantined
            .lifecycle_evidence_summary()
            .contains("readmission_gate=validation_repair_and_operator_approval")
    );
    assert!(
        quarantined
            .lifecycle_evidence_summary()
            .contains("operator_approval_required=true")
    );
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
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.contains(":lifecycle="))
    );
}
