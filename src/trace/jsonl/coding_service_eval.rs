use std::io;
use std::path::Path;

use crate::coding_service_eval::{
    CODING_SERVICE_EVAL_RUNNER_SCHEMA_VERSION, CODING_SERVICE_EVAL_TRACE_SCHEMA,
    CodingServiceEvalReadinessReport, CodingServiceEvalRunnerReport,
};
use crate::privacy_redaction::stable_redaction_digest;

use super::json::{option_string_json, string_array_json};
use super::writer::append_line;

pub fn coding_service_eval_readiness_trace_json_line(
    report: &CodingServiceEvalReadinessReport,
) -> String {
    let profile_labels = report.profile_counts.keys().cloned().collect::<Vec<_>>();
    let language_labels = report.language_counts.keys().cloned().collect::<Vec<_>>();
    let capability_labels = report.capability_counts.keys().cloned().collect::<Vec<_>>();
    let evidence_joined = report.request_evidence_packets.join("\n");
    let summary = report.summary_line();
    let evidence_digest = stable_redaction_digest([
        "coding-service-eval-readiness-evidence",
        &summary,
        &evidence_joined,
    ]);
    let report_digest = stable_redaction_digest(["coding-service-eval-readiness-report", &summary]);

    format!(
        "{{\
         \"schema\":\"{}\",\
         \"report_schema\":\"{}\",\
         \"report_kind\":\"readiness\",\
         \"passed\":{},\
         \"request_plan_count\":{},\
         \"fixture_count\":{},\
         \"completed_count\":0,\
         \"profile_count\":{},\
         \"language_count\":{},\
         \"capability_count\":{},\
         \"profiles\":{},\
         \"languages\":{},\
         \"capabilities\":{},\
         \"missing_capability_count\":{},\
         \"evidence_packet_count\":{},\
         \"rust_validation_checked_count\":0,\
         \"compile_checked_count\":0,\
         \"unit_test_checked_count\":0,\
         \"suite_pass_rate\":{:.6},\
         \"evidence_digest\":{},\
         \"report_digest\":{},\
         \"read_only\":{},\
         \"write_allowed\":{},\
         \"applied\":{},\
         \"summary\":{}\
         }}",
        CODING_SERVICE_EVAL_TRACE_SCHEMA,
        report.schema_version,
        report.passed(),
        report.request_plan_count,
        report.corpus_fixture_count,
        report.profile_counts.len(),
        report.language_counts.len(),
        report.capability_counts.len(),
        string_array_json(&profile_labels),
        string_array_json(&language_labels),
        string_array_json(&capability_labels),
        report.missing_capabilities.len(),
        report.request_evidence_packets.len(),
        report.suite_report.pass_rate(),
        option_string_json(Some(&evidence_digest)),
        option_string_json(Some(&report_digest)),
        report.read_only,
        report.write_allowed,
        report.applied,
        option_string_json(Some(&summary))
    )
}

pub fn append_coding_service_eval_readiness_trace_jsonl(
    path: impl AsRef<Path>,
    report: &CodingServiceEvalReadinessReport,
) -> io::Result<()> {
    let line = coding_service_eval_readiness_trace_json_line(report);
    append_line(path, &line)
}

pub fn coding_service_eval_runner_trace_json_line(
    report: &CodingServiceEvalRunnerReport,
) -> String {
    let profile_labels = report
        .run_records
        .iter()
        .map(|record| record.profile.as_str().to_owned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let language_labels = report
        .run_records
        .iter()
        .map(|record| record.language.as_str().to_owned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let result_class_labels = report
        .result_class_counts
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let failure_class_labels = report
        .failure_class_counts
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let evidence_joined = report.evidence_packets.join("\n");
    let summary = report.summary_line();
    let evidence_digest = stable_redaction_digest([
        "coding-service-eval-runner-evidence",
        &summary,
        &evidence_joined,
    ]);
    let report_digest = stable_redaction_digest(["coding-service-eval-runner-report", &summary]);
    let compile_checked_count = report
        .run_records
        .iter()
        .filter(|record| record.compile_checked)
        .count();
    let unit_test_checked_count = report
        .run_records
        .iter()
        .filter(|record| record.unit_test_checked)
        .count();

    format!(
        "{{\
         \"schema\":\"{}\",\
         \"report_schema\":\"{}\",\
         \"report_kind\":\"runner\",\
         \"passed\":{},\
         \"request_plan_count\":{},\
         \"fixture_count\":{},\
         \"completed_count\":{},\
         \"profile_count\":{},\
         \"language_count\":{},\
         \"capability_count\":0,\
         \"profiles\":{},\
         \"languages\":{},\
         \"capabilities\":[],\
         \"missing_capability_count\":0,\
         \"evidence_packet_count\":{},\
         \"failed_runner_contract_count\":{},\
         \"cancellation_probe_count\":{},\
         \"cancellation_passed_count\":{},\
         \"diagnostics_seen_count\":{},\
         \"health_seen_count\":{},\
         \"model_capabilities_seen_count\":{},\
         \"max_tokens_respected_count\":{},\
         \"rust_validation_checked_count\":{},\
         \"compile_checked_count\":{},\
         \"unit_test_checked_count\":{},\
         \"result_class_count\":{},\
         \"result_classes\":{},\
         \"failure_class_count\":{},\
         \"failure_classes\":{},\
         \"suite_pass_rate\":{:.6},\
         \"evidence_digest\":{},\
         \"report_digest\":{},\
         \"read_only\":{},\
         \"write_allowed\":{},\
         \"applied\":{},\
         \"summary\":{}\
         }}",
        CODING_SERVICE_EVAL_TRACE_SCHEMA,
        CODING_SERVICE_EVAL_RUNNER_SCHEMA_VERSION,
        report.passed(),
        report.plan_count,
        report.run_records.len(),
        report.completed_count,
        profile_labels.len(),
        language_labels.len(),
        string_array_json(&profile_labels),
        string_array_json(&language_labels),
        report.evidence_packets.len(),
        report.failed_runner_contract_count,
        report.cancellation_probe_count,
        report.cancellation_passed_count,
        report.diagnostics_seen_count,
        report.health_seen_count,
        report.model_capabilities_seen_count,
        report.max_tokens_respected_count,
        report.rust_validation_checked_count,
        compile_checked_count,
        unit_test_checked_count,
        result_class_labels.len(),
        string_array_json(&result_class_labels),
        failure_class_labels.len(),
        string_array_json(&failure_class_labels),
        report.suite_report.pass_rate(),
        option_string_json(Some(&evidence_digest)),
        option_string_json(Some(&report_digest)),
        report.read_only,
        report.write_allowed,
        report.applied,
        option_string_json(Some(&summary))
    )
}

pub fn append_coding_service_eval_runner_trace_jsonl(
    path: impl AsRef<Path>,
    report: &CodingServiceEvalRunnerReport,
) -> io::Result<()> {
    let line = coding_service_eval_runner_trace_json_line(report);
    append_line(path, &line)
}
