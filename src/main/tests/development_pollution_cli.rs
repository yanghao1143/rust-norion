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
