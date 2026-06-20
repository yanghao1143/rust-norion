use std::path::PathBuf;

use rust_norion::{
    ExperienceHygieneFinding, ExperienceHygieneQuarantinePlan, ExperienceHygieneReport,
    ExperienceIndexFinding, ExperienceIndexReport,
};

use super::super::json::{
    option_path_service_json, service_json_string, service_json_string_array, service_u64_array,
};

pub(crate) struct ModelServiceExperienceHygieneView<'a> {
    pub(crate) request_id: usize,
    pub(crate) experience_path: &'a PathBuf,
    pub(crate) report: Option<&'a ExperienceHygieneReport>,
    pub(crate) index_report: Option<&'a ExperienceIndexReport>,
    pub(crate) quarantine_plan: Option<&'a ExperienceHygieneQuarantinePlan>,
    pub(crate) error: Option<&'a str>,
}

pub(crate) struct ModelServiceExperienceHygieneQuarantineView<'a> {
    pub(crate) request_id: usize,
    pub(crate) experience_path: &'a PathBuf,
    pub(crate) applied: bool,
    pub(crate) backup_path: Option<&'a PathBuf>,
    pub(crate) quarantine_path: Option<&'a PathBuf>,
    pub(crate) plan: &'a ExperienceHygieneQuarantinePlan,
}

pub(crate) fn model_service_experience_hygiene_response_json(
    view: ModelServiceExperienceHygieneView<'_>,
) -> String {
    format!(
        "{{\"ok\":true,\"request_id\":{},\"experience_file\":{},\"checked\":{},\"error\":{},\"report\":{},\"index_report\":{},\"quarantine_plan\":{}}}",
        view.request_id,
        service_json_string(&view.experience_path.display().to_string()),
        view.report.is_some(),
        view.error
            .map(service_json_string)
            .unwrap_or_else(|| "null".to_owned()),
        option_report_json(view.report),
        option_index_report_json(view.index_report),
        option_quarantine_plan_json(view.quarantine_plan, false)
    )
}

pub(crate) fn model_service_experience_hygiene_quarantine_response_json(
    view: ModelServiceExperienceHygieneQuarantineView<'_>,
) -> String {
    format!(
        "{{\"ok\":true,\"request_id\":{},\"experience_file\":{},\"applied\":{},\"backup_file\":{},\"quarantine_file\":{},\"plan\":{}}}",
        view.request_id,
        service_json_string(&view.experience_path.display().to_string()),
        view.applied,
        option_path_service_json(view.backup_path),
        option_path_service_json(view.quarantine_path),
        quarantine_plan_json(view.plan, view.applied)
    )
}

fn option_report_json(report: Option<&ExperienceHygieneReport>) -> String {
    report.map(report_json).unwrap_or_else(|| "null".to_owned())
}

pub(super) fn report_json(report: &ExperienceHygieneReport) -> String {
    format!(
        "{{\"total_records\":{},\"findings\":{},\"watch\":{},\"quarantine_candidates\":{},\"legacy_metadata_lessons\":{},\"legacy_metadata_without_clean_gist\":{},\"clean\":{},\"listed_findings\":{}}}",
        report.total_records,
        report.finding_count,
        report.watch_count,
        report.quarantine_candidate_count,
        report.legacy_metadata_lesson_count,
        report.legacy_metadata_without_clean_gist_count,
        report.quarantine_candidate_count == 0,
        findings_json(&report.findings)
    )
}

fn option_index_report_json(report: Option<&ExperienceIndexReport>) -> String {
    report
        .map(index_report_json)
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn index_report_json(report: &ExperienceIndexReport) -> String {
    format!(
        "{{\"total_records\":{},\"compacted_records\":{},\"overlong_records\":{},\"overlong_without_clean_gist\":{},\"max_record_chars\":{},\"noisy_records\":{},\"duplicate_outputs\":{},\"max_noise_penalty\":{:.6},\"quality_score\":{:.6},\"retrieval_ready\":{},\"risk_level\":{},\"recommended_action\":{},\"listed_findings\":{}}}",
        report.total_records,
        report.compacted_record_count,
        report.overlong_record_count,
        report.overlong_without_clean_gist_count,
        report.max_record_chars,
        report.noisy_record_count,
        report.duplicate_output_count,
        report.max_noise_penalty,
        report.quality_score,
        report.retrieval_ready,
        service_json_string(&report.risk_level),
        service_json_string(&report.recommended_action),
        index_findings_json(&report.findings)
    )
}

fn option_quarantine_plan_json(
    plan: Option<&ExperienceHygieneQuarantinePlan>,
    applied: bool,
) -> String {
    plan.map(|plan| quarantine_plan_json(plan, applied))
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn quarantine_plan_json(
    plan: &ExperienceHygieneQuarantinePlan,
    applied: bool,
) -> String {
    format!(
        "{{\"applied\":{},\"total_records\":{},\"retained_records\":{},\"quarantine_candidates\":{},\"candidate_ids\":{},\"listed_findings\":{}}}",
        applied,
        plan.total_records,
        plan.retained_records,
        plan.quarantine_candidate_count,
        service_u64_array(&plan.candidate_ids),
        findings_json(&plan.listed_findings)
    )
}

fn findings_json(findings: &[ExperienceHygieneFinding]) -> String {
    let items = findings
        .iter()
        .map(finding_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn index_findings_json(findings: &[ExperienceIndexFinding]) -> String {
    let items = findings
        .iter()
        .map(index_finding_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn finding_json(finding: &ExperienceHygieneFinding) -> String {
    format!(
        "{{\"experience_id\":{},\"severity\":\"{}\",\"reason\":{},\"markers\":{},\"prompt_preview\":{},\"lesson_preview\":{}}}",
        finding.experience_id,
        finding.severity.as_str(),
        service_json_string(&finding.reason),
        service_json_string_array(&finding.markers),
        service_json_string(&finding.prompt_preview),
        service_json_string(&finding.lesson_preview)
    )
}

fn index_finding_json(finding: &ExperienceIndexFinding) -> String {
    format!(
        "{{\"experience_id\":{},\"reason\":{},\"compacted\":{},\"noise_penalty\":{:.6},\"duplicate_of\":{},\"prompt_chars\":{},\"lesson_chars\":{},\"prompt_preview\":{},\"lesson_preview\":{}}}",
        finding.experience_id,
        service_json_string(&finding.reason),
        finding.compacted,
        finding.noise_penalty,
        option_u64_json(finding.duplicate_of),
        finding.prompt_chars,
        finding.lesson_chars,
        service_json_string(&finding.prompt_preview),
        service_json_string(&finding.lesson_preview)
    )
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::ExperienceHygieneSeverity;

    #[test]
    fn hygiene_response_reports_missing_store_without_plan() {
        let path = PathBuf::from("missing.ndkv");
        let body =
            model_service_experience_hygiene_response_json(ModelServiceExperienceHygieneView {
                request_id: 3,
                experience_path: &path,
                report: None,
                index_report: None,
                quarantine_plan: None,
                error: Some("experience_file_missing"),
            });

        assert!(body.contains("\"request_id\":3"));
        assert!(body.contains("\"checked\":false"));
        assert!(body.contains("\"error\":\"experience_file_missing\""));
        assert!(body.contains("\"report\":null"));
        assert!(body.contains("\"index_report\":null"));
        assert!(body.contains("\"quarantine_plan\":null"));
    }

    #[test]
    fn hygiene_response_reports_index_health() {
        let path = PathBuf::from("experience.ndkv");
        let report = ExperienceHygieneReport {
            total_records: 2,
            ..ExperienceHygieneReport::default()
        };
        let index_report = ExperienceIndexReport {
            total_records: 2,
            compacted_record_count: 1,
            overlong_record_count: 1,
            overlong_without_clean_gist_count: 1,
            max_record_chars: 4128,
            noisy_record_count: 1,
            max_noise_penalty: 0.18,
            duplicate_output_count: 1,
            quality_score: 0.495,
            retrieval_ready: false,
            risk_level: "blocked".to_owned(),
            recommended_action: "pause_chat_and_add_clean_gists".to_owned(),
            findings: vec![ExperienceIndexFinding {
                experience_id: 7,
                reason: "unstructured_long_transcript".to_owned(),
                compacted: true,
                noise_penalty: 0.18,
                duplicate_of: Some(3),
                prompt_chars: 4096,
                lesson_chars: 32,
                prompt_preview: "Conversation transcript".to_owned(),
                lesson_preview: "lesson".to_owned(),
            }],
        };
        let body =
            model_service_experience_hygiene_response_json(ModelServiceExperienceHygieneView {
                request_id: 5,
                experience_path: &path,
                report: Some(&report),
                index_report: Some(&index_report),
                quarantine_plan: None,
                error: None,
            });

        assert!(body.contains("\"index_report\":{"));
        assert!(body.contains("\"compacted_records\":1"));
        assert!(body.contains("\"overlong_records\":1"));
        assert!(body.contains("\"overlong_without_clean_gist\":1"));
        assert!(body.contains("\"max_record_chars\":4128"));
        assert!(body.contains("\"noisy_records\":1"));
        assert!(body.contains("\"duplicate_outputs\":1"));
        assert!(body.contains("\"max_noise_penalty\":0.180000"));
        assert!(body.contains("\"quality_score\":0.495000"));
        assert!(body.contains("\"retrieval_ready\":false"));
        assert!(body.contains("\"risk_level\":\"blocked\""));
        assert!(body.contains("\"recommended_action\":\"pause_chat_and_add_clean_gists\""));
        assert!(body.contains("\"listed_findings\":["));
        assert!(body.contains("\"experience_id\":7"));
        assert!(body.contains("\"reason\":\"unstructured_long_transcript\""));
        assert!(body.contains("\"duplicate_of\":3"));
    }

    #[test]
    fn quarantine_response_lists_candidate_ids() {
        let path = PathBuf::from("experience.ndkv");
        let plan = ExperienceHygieneQuarantinePlan {
            total_records: 2,
            retained_records: 1,
            quarantine_candidate_count: 1,
            candidate_ids: vec![42],
            listed_findings: vec![ExperienceHygieneFinding {
                experience_id: 42,
                severity: ExperienceHygieneSeverity::QuarantineCandidate,
                reason: "cross_task_shell_transcript".to_owned(),
                markers: vec!["gitlab_local".to_owned()],
                prompt_preview: "prompt".to_owned(),
                lesson_preview: "lesson".to_owned(),
            }],
        };
        let body = model_service_experience_hygiene_quarantine_response_json(
            ModelServiceExperienceHygieneQuarantineView {
                request_id: 8,
                experience_path: &path,
                applied: false,
                backup_path: None,
                quarantine_path: None,
                plan: &plan,
            },
        );

        assert!(body.contains("\"applied\":false"));
        assert!(body.contains("\"candidate_ids\":[42]"));
        assert!(body.contains("\"quarantine_candidates\":1"));
        assert!(body.contains("\"severity\":\"quarantine_candidate\""));
    }
}
