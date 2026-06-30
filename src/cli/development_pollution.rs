use rust_norion::{
    CapabilityCandidate, DefenseSpacer, DefenseSpacerActivationGate, DefenseSpacerCandidate,
    DevelopmentEvidenceAdmission, DevelopmentEvidenceSurfaceGate, DevelopmentEvidenceUseSurface,
    DevelopmentNutrientTarget, DevelopmentPollutionEvent, DevelopmentPollutionFinding,
    DevelopmentPollutionReport, admit_development_evidence_for_current_use,
    classify_development_pollution, gate_defense_spacer_activation,
    gate_development_evidence_surface,
};

use crate::Args;

const DEVELOPMENT_POLLUTION_SURFACES: [DevelopmentEvidenceUseSurface; 7] = [
    DevelopmentEvidenceUseSurface::Prompt,
    DevelopmentEvidenceUseSurface::Trace,
    DevelopmentEvidenceUseSurface::Benchmark,
    DevelopmentEvidenceUseSurface::PullRequestBody,
    DevelopmentEvidenceUseSurface::ExperienceRetrieval,
    DevelopmentEvidenceUseSurface::DurableMemory,
    DevelopmentEvidenceUseSurface::DigestMarker,
];

#[derive(Debug, Clone)]
pub(crate) struct DevelopmentPollutionCommandReport {
    pub(crate) report: DevelopmentPollutionReport,
    pub(crate) capability_candidates: Vec<CapabilityCandidate>,
    pub(crate) admissions: Vec<DevelopmentEvidenceAdmission>,
    pub(crate) surface_gates: Vec<DevelopmentEvidenceSurfaceGate>,
    pub(crate) spacers: Vec<DefenseSpacer>,
    pub(crate) activation_gates: Vec<DefenseSpacerActivationGate>,
}

pub(crate) fn run_development_pollution_report(args: &Args) -> DevelopmentPollutionCommandReport {
    let mut event = DevelopmentPollutionEvent::new(
        &args.development_pollution_event_id,
        &args.development_pollution_source_kind,
        &args.prompt,
        &args.development_pollution_reason,
    )
    .with_hit_count(args.development_pollution_hit_count)
    .with_current_proof(args.development_pollution_current_proof);
    if let Some(ttl) = &args.development_pollution_ttl {
        event = event.with_ttl(ttl);
    }

    let report = classify_development_pollution(&[event]);
    let mut admissions = Vec::new();
    let mut surface_gates = Vec::new();
    let mut spacers = Vec::new();
    let mut activation_gates = Vec::new();

    for finding in &report.findings {
        let admission = admit_development_evidence_for_current_use(finding);
        surface_gates.extend(
            DEVELOPMENT_POLLUTION_SURFACES
                .iter()
                .copied()
                .map(|surface| gate_development_evidence_surface(&admission, surface)),
        );
        admissions.push(admission);

        let spacer = DefenseSpacer::from_finding(
            finding,
            &args.development_pollution_scope,
            "preview",
            "live_validation_privacy_license_rollback_or_explicit_approval",
        );
        let candidate =
            DefenseSpacerCandidate::from_finding(finding, &args.development_pollution_scope);
        activation_gates.push(gate_defense_spacer_activation(
            std::slice::from_ref(&spacer),
            &candidate,
        ));
        spacers.push(spacer);
    }

    DevelopmentPollutionCommandReport {
        capability_candidates: report.capability_candidates.clone(),
        report,
        admissions,
        surface_gates,
        spacers,
        activation_gates,
    }
}

pub(crate) fn print_development_pollution_report(report: &DevelopmentPollutionCommandReport) {
    for line in development_pollution_report_lines(report) {
        println!("{line}");
    }
}

pub(crate) fn development_pollution_report_lines(
    report: &DevelopmentPollutionCommandReport,
) -> Vec<String> {
    let mut lines = vec![
        "Noiron development pollution report".to_owned(),
        "writes_state=false durable_write_allowed=false applied=false".to_owned(),
        report.report.summary_line(),
    ];
    for finding in &report.report.findings {
        lines.push(finding.summary_line());
    }
    if report.capability_candidates.is_empty() {
        lines.push("capability_candidates: none".to_owned());
    } else {
        for candidate in &report.capability_candidates {
            lines.push(format!(
                "capability_candidate reason={} target={} hits={}",
                candidate.reason_code,
                candidate.target.as_str(),
                candidate.hit_count
            ));
        }
    }
    for finding in &report.report.findings {
        if let Some(line) = no_nutrient_value_decision_line(finding) {
            lines.push(line);
        }
    }
    for admission in &report.admissions {
        lines.push(admission.summary_line());
    }
    for gate in &report.surface_gates {
        lines.push(gate.summary_line());
    }
    for spacer in &report.spacers {
        lines.push(spacer.summary_line());
    }
    for gate in &report.activation_gates {
        lines.push(gate.summary_line());
    }
    lines
}

fn no_nutrient_value_decision_line(finding: &DevelopmentPollutionFinding) -> Option<String> {
    (finding.hit_count >= 2
        && finding.nutrient_target == DevelopmentNutrientTarget::NoNutrientValue)
        .then(|| {
            format!(
                "no_nutrient_value reason={} hits={} class={} action={} proof={} ttl={}",
                finding.reason_code,
                finding.hit_count,
                finding.class.as_str(),
                finding.action.as_str(),
                finding.proof,
                finding.ttl.as_deref().unwrap_or("missing")
            )
        })
}
