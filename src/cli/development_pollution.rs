use rust_norion::{
    CapabilityCandidate, DefenseSpacer, DefenseSpacerActivationGate, DefenseSpacerCandidate,
    DevelopmentEvidenceAdmission, DevelopmentEvidenceSurfaceGate, DevelopmentEvidenceUseSurface,
    DevelopmentNutrientTarget, DevelopmentPollutionEvent, DevelopmentPollutionFinding,
    DevelopmentPollutionReport, admit_development_evidence_for_current_use,
    classify_development_pollution, gate_defense_spacer_activation,
    gate_development_evidence_surface,
};
use std::{collections::BTreeMap, process::Command};

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
const DEFAULT_DIRTY_WORKTREE_RETIREMENT_VERSION: &str = "next_release";

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
    let events = if args.development_pollution_dirty_worktree {
        development_pollution_events_from_current_git_status(
            args.development_pollution_ttl
                .as_deref()
                .unwrap_or(DEFAULT_DIRTY_WORKTREE_RETIREMENT_VERSION),
        )
    } else {
        vec![development_pollution_event_from_args(args)]
    };

    development_pollution_report_for_events(args, events)
}

pub(crate) fn development_pollution_report_for_events(
    args: &Args,
    events: Vec<DevelopmentPollutionEvent>,
) -> DevelopmentPollutionCommandReport {
    let report = classify_development_pollution(&events);
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

fn development_pollution_event_from_args(args: &Args) -> DevelopmentPollutionEvent {
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
    event
}

fn development_pollution_events_from_current_git_status(
    retirement_version: &str,
) -> Vec<DevelopmentPollutionEvent> {
    match Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let status = String::from_utf8_lossy(&output.stdout);
            development_pollution_events_from_git_status_with_retirement_version(
                &status,
                retirement_version,
            )
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            vec![DevelopmentPollutionEvent::new(
                "git-status-error",
                "git_status",
                stderr.as_ref(),
                "missing_evidence",
            )]
        }
        Err(error) => vec![DevelopmentPollutionEvent::new(
            "git-status-error",
            "git_status",
            error.to_string(),
            "missing_evidence",
        )],
    }
}

pub(crate) fn development_pollution_events_from_git_status_with_retirement_version(
    status: &str,
    retirement_version: &str,
) -> Vec<DevelopmentPollutionEvent> {
    status
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line = line.trim_end();
            if line.is_empty() {
                return None;
            }
            let path = porcelain_path(line);
            let (source_kind, reason) = if is_output_artifact_path(path) {
                ("output_artifact", "reproducible_junk")
            } else {
                ("dirty_path", "missing_cleanup")
            };
            Some(
                DevelopmentPollutionEvent::new(
                    format!("git-status-{}", index + 1),
                    source_kind,
                    line,
                    reason,
                )
                .with_ttl(retirement_version),
            )
        })
        .collect()
}

fn porcelain_path(line: &str) -> &str {
    line.get(3..).unwrap_or(line).trim()
}

fn is_output_artifact_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized == "output" || normalized.starts_with("output/")
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
    lines.extend(deprecation_ledger_lines(&report.report.findings));
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
    lines.extend(no_nutrient_value_decision_lines(&report.report.findings));
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

fn deprecation_ledger_lines(findings: &[DevelopmentPollutionFinding]) -> Vec<String> {
    let mut totals = BTreeMap::<(String, String, String, String, String, String), usize>::new();
    for finding in findings.iter().filter(|finding| finding.ttl.is_some()) {
        let key = (
            report_token(finding.ttl.as_deref().unwrap_or("missing")),
            report_token(&finding.reason_code),
            finding.class.as_str().to_owned(),
            finding.action.as_str().to_owned(),
            finding.nutrient_target.as_str().to_owned(),
            report_token(&finding.proof),
        );
        *totals.entry(key).or_default() += finding.hit_count;
    }

    totals
        .into_iter()
        .map(
            |((version, deprecation, class, action, nutrient_target, proof), hits)| {
                format!(
                    "development_pollution_deprecation version={} deprecation={} hits={} class={} action={} nutrient_target={} proof={}",
                    version, deprecation, hits, class, action, nutrient_target, proof
                )
            },
        )
        .collect()
}

fn no_nutrient_value_decision_lines(findings: &[DevelopmentPollutionFinding]) -> Vec<String> {
    let mut totals = BTreeMap::<(String, String, String, String, String), usize>::new();
    for finding in findings
        .iter()
        .filter(|finding| finding.nutrient_target == DevelopmentNutrientTarget::NoNutrientValue)
    {
        let key = (
            report_token(&finding.reason_code),
            finding.class.as_str().to_owned(),
            finding.action.as_str().to_owned(),
            report_token(&finding.proof),
            report_token(finding.ttl.as_deref().unwrap_or("missing")),
        );
        *totals.entry(key).or_default() += finding.hit_count;
    }

    totals
        .into_iter()
        .filter_map(|((reason, class, action, proof, ttl), hits)| {
            (hits >= 2).then(|| {
                format!(
                    "no_nutrient_value reason={} hits={} class={} action={} proof={} ttl={}",
                    reason, hits, class, action, proof, ttl
                )
            })
        })
        .collect()
}

fn report_token(value: &str) -> String {
    let mut out = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
                ch
            } else {
                '_'
            }
        })
        .take(80)
        .collect::<String>();
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let out = out.trim_matches('_');
    if out.is_empty() {
        "missing".to_owned()
    } else {
        out.to_owned()
    }
}
