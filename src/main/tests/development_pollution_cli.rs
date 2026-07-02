use rust_norion::{
    DefenseSpacerDecision, DevelopmentEvidenceUseSurface, DevelopmentHygieneState,
    DevelopmentNutrientTarget, DevelopmentPollutionClass, DevelopmentPollutionLifecycleStage,
};

use super::*;

#[test]
fn development_pollution_cli_reports_digest_only_quarantine_gates() {
    let args = Args::parse(vec![
        "--development-pollution".to_owned(),
        "--development-pollution-event-id".to_owned(),
        "polluted-window".to_owned(),
        "--development-pollution-source-kind".to_owned(),
        "thread_summary".to_owned(),
        "--development-pollution-reason".to_owned(),
        "development_evidence_contamination".to_owned(),
        "--development-pollution-scope".to_owned(),
        "pr_body".to_owned(),
        "raw polluted payload must not print".to_owned(),
    ]);

    let report = crate::cli::development_pollution::run_development_pollution_report(&args);

    let finding = &report.report.findings[0];
    assert_eq!(finding.class, DevelopmentPollutionClass::Quarantine);
    assert_eq!(
        finding.lifecycle_stage,
        DevelopmentPollutionLifecycleStage::Quarantine
    );
    assert_eq!(finding.hygiene_state, DevelopmentHygieneState::Polluted);
    assert_eq!(
        finding.nutrient_target,
        DevelopmentNutrientTarget::EvidencePacketTemplate
    );
    assert!(!finding.summary_line().contains("raw polluted payload"));

    let prompt_gate = report
        .surface_gates
        .iter()
        .find(|gate| gate.surface == DevelopmentEvidenceUseSurface::Prompt)
        .unwrap();
    assert!(!prompt_gate.allowed);
    assert_eq!(prompt_gate.reason, "digest_only_quarantine_required");

    let marker_gate = report
        .surface_gates
        .iter()
        .find(|gate| gate.surface == DevelopmentEvidenceUseSurface::DigestMarker)
        .unwrap();
    assert!(marker_gate.allowed);
    assert_eq!(marker_gate.reason, "digest_marker_allowed");

    let spacer = &report.spacers[0];
    assert_eq!(spacer.decision, DefenseSpacerDecision::Quarantine);
    assert!(!spacer.summary_line().contains("raw polluted payload"));

    let activation = &report.activation_gates[0];
    assert!(!activation.allowed);
    assert_eq!(activation.decision, DefenseSpacerDecision::Quarantine);
    assert_eq!(activation.reason, "matched_quarantine_defense_spacer");
}

#[test]
fn development_pollution_cli_promotes_repeated_tool_gap_to_capability_candidate() {
    let args = Args::parse(vec![
        "--development-pollution".to_owned(),
        "--development-pollution-reason".to_owned(),
        "missing_cleanup".to_owned(),
        "--development-pollution-hit-count".to_owned(),
        "2".to_owned(),
        "--development-pollution-ttl".to_owned(),
        "next_release".to_owned(),
        "dirty local output cleanup repeated twice".to_owned(),
    ]);

    let report = crate::cli::development_pollution::run_development_pollution_report(&args);
    let lines = crate::cli::development_pollution::development_pollution_report_lines(&report);

    assert_eq!(report.capability_candidates.len(), 1);
    assert_eq!(
        report.capability_candidates[0].reason_code,
        "missing_cleanup"
    );
    assert_eq!(
        report.capability_candidates[0].target,
        DevelopmentNutrientTarget::SkillPlaybook
    );
    assert_eq!(report.capability_candidates[0].hit_count, 2);
    assert_eq!(
        report.report.findings[0].ttl.as_deref(),
        Some("next_release")
    );
    assert!(
        !report.report.findings[0]
            .summary_line()
            .contains("dirty local output")
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("lifecycle_nutrient=1"))
    );
    assert!(lines.iter().any(|line| line.contains("lifecycle=nutrient")));
}

#[test]
fn development_pollution_cli_reports_repeated_no_nutrient_value_decision() {
    let args = Args::parse(vec![
        "--development-pollution".to_owned(),
        "--development-pollution-reason".to_owned(),
        "reproducible_junk".to_owned(),
        "--development-pollution-hit-count".to_owned(),
        "2".to_owned(),
        "--development-pollution-ttl".to_owned(),
        "next_release".to_owned(),
        "generated output payload must not print".to_owned(),
    ]);

    let report = crate::cli::development_pollution::run_development_pollution_report(&args);
    let lines = crate::cli::development_pollution::development_pollution_report_lines(&report);

    assert!(report.capability_candidates.is_empty());
    assert_eq!(
        report.report.findings[0].nutrient_target,
        DevelopmentNutrientTarget::NoNutrientValue
    );
    assert!(lines.iter().any(|line| {
        line == "no_nutrient_value reason=reproducible_junk hits=2 class=delete_candidate action=delete_after_proof proof=missing ttl=next_release"
    }));
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("generated output payload"))
    );
}

#[test]
fn development_pollution_cli_aggregates_no_nutrient_value_decisions_by_reason() {
    let args = Args::parse(vec![
        "--development-pollution-dirty-worktree".to_owned(),
        "ignored manual payload".to_owned(),
    ]);
    let events =
        crate::cli::development_pollution::development_pollution_events_from_git_status_with_retirement_version(
            "?? output/tmp/report-a.json\n?? output/tmp/report-b.json\n",
            "next_release",
        );

    let report =
        crate::cli::development_pollution::development_pollution_report_for_events(&args, events);
    let lines = crate::cli::development_pollution::development_pollution_report_lines(&report);

    assert!(report.capability_candidates.is_empty());
    assert!(lines.iter().any(|line| {
        line == "no_nutrient_value reason=reproducible_junk hits=2 class=delete_candidate action=delete_after_proof proof=missing ttl=next_release"
    }));
    assert!(!lines.iter().any(|line| line.contains("report-a.json")));
    assert!(!lines.iter().any(|line| line.contains("report-b.json")));
}

#[test]
fn development_pollution_dirty_worktree_report_classifies_git_status_without_raw_paths() {
    let args = Args::parse(vec![
        "--development-pollution-dirty-worktree".to_owned(),
        "ignored manual payload".to_owned(),
    ]);
    let events =
        crate::cli::development_pollution::development_pollution_events_from_git_status_with_retirement_version(
            " M tools/smartsteam-forge/scripts/status-forge.ps1\n M tools/smartsteam-forge/scripts/start-remote-gemma-chain.ps1\n?? output/tmp/report.json\n",
            "next_release",
        );

    let report =
        crate::cli::development_pollution::development_pollution_report_for_events(&args, events);
    let lines = crate::cli::development_pollution::development_pollution_report_lines(&report);

    assert_eq!(report.report.findings.len(), 3);
    assert_eq!(report.report.findings[0].source_kind, "dirty_path");
    assert_eq!(
        report.report.findings[0].nutrient_target,
        DevelopmentNutrientTarget::SkillPlaybook
    );
    assert_eq!(
        report.report.findings[0].lifecycle_stage,
        DevelopmentPollutionLifecycleStage::Nutrient
    );
    assert_eq!(report.report.findings[2].source_kind, "output_artifact");
    assert_eq!(
        report.report.findings[2].class,
        DevelopmentPollutionClass::DeleteCandidate
    );
    assert_eq!(
        report.report.findings[2].lifecycle_stage,
        DevelopmentPollutionLifecycleStage::Cut
    );
    assert_eq!(report.capability_candidates.len(), 1);
    assert_eq!(
        report.capability_candidates[0].reason_code,
        "missing_cleanup"
    );
    assert_eq!(report.capability_candidates[0].hit_count, 2);
    assert!(lines.iter().any(|line| line.contains("lifecycle_cut=1")));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("lifecycle_nutrient=2"))
    );
    assert!(lines.iter().any(|line| {
        line == "development_pollution_deprecation version=next_release deprecation=missing_cleanup hits=2 class=nutrient action=admit_as_nutrient nutrient_target=skill_playbook proof=missing"
    }));
    assert!(lines.iter().any(|line| {
        line == "development_pollution_deprecation version=next_release deprecation=reproducible_junk hits=1 class=delete_candidate action=delete_after_proof nutrient_target=no_nutrient_value proof=missing"
    }));
    assert!(
        lines.iter().any(|line| line
            == "capability_candidate reason=missing_cleanup target=skill_playbook hits=2")
    );
    assert!(!lines.iter().any(|line| line.contains("status-forge.ps1")));
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("start-remote-gemma-chain.ps1"))
    );
    assert!(!lines.iter().any(|line| line.contains("output/tmp/report")));
}

#[test]
fn development_pollution_dirty_worktree_report_uses_requested_retirement_version() {
    let args = Args::parse(vec![
        "--development-pollution-dirty-worktree".to_owned(),
        "--development-pollution-ttl".to_owned(),
        "0.1.0-dirty-retirement".to_owned(),
    ]);
    let events =
        crate::cli::development_pollution::development_pollution_events_from_git_status_with_retirement_version(
            " M tools/smartsteam-forge/scripts/status-forge.ps1\n?? output/tmp/report.json\n",
            args.development_pollution_ttl.as_deref().unwrap(),
        );

    let report =
        crate::cli::development_pollution::development_pollution_report_for_events(&args, events);
    let lines = crate::cli::development_pollution::development_pollution_report_lines(&report);

    assert!(lines.iter().any(|line| {
        line == "development_pollution_deprecation version=0.1.0-dirty-retirement deprecation=missing_cleanup hits=1 class=nutrient action=admit_as_nutrient nutrient_target=skill_playbook proof=missing"
    }));
    assert!(lines.iter().any(|line| {
        line == "development_pollution_deprecation version=0.1.0-dirty-retirement deprecation=reproducible_junk hits=1 class=delete_candidate action=delete_after_proof nutrient_target=no_nutrient_value proof=missing"
    }));
    assert!(
        !lines.iter().any(|line| {
            line.contains("development_pollution_deprecation version=next_release")
        })
    );
}
