use super::fields::{
    extract_json_bool_field, extract_json_f32_field, extract_json_string_array_field,
    extract_json_string_field, extract_json_usize_field,
};
use crate::coding_service_eval::{
    CODING_SERVICE_EVAL_RUNNER_SCHEMA_VERSION, CODING_SERVICE_EVAL_SCHEMA_VERSION,
    CODING_SERVICE_EVAL_TRACE_SCHEMA,
};
use crate::privacy_redaction::contains_private_or_executable_marker;

pub(super) fn evaluate_coding_service_eval_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-coding-service-eval-readiness-v1\"",
        ),
        ("report_schema", "\"report_schema\":"),
        ("report_kind", "\"report_kind\":"),
        ("passed", "\"passed\":"),
        ("request_plan_count", "\"request_plan_count\":"),
        ("fixture_count", "\"fixture_count\":"),
        ("completed_count", "\"completed_count\":"),
        ("profile_count", "\"profile_count\":"),
        ("language_count", "\"language_count\":"),
        ("profiles", "\"profiles\":"),
        ("languages", "\"languages\":"),
        ("evidence_packet_count", "\"evidence_packet_count\":"),
        (
            "rust_validation_checked_count",
            "\"rust_validation_checked_count\":",
        ),
        ("compile_checked_count", "\"compile_checked_count\":"),
        ("unit_test_checked_count", "\"unit_test_checked_count\":"),
        ("benchmark_checked_count", "\"benchmark_checked_count\":"),
        ("benchmark_passed_count", "\"benchmark_passed_count\":"),
        (
            "layer_b_route_proof_ready_count",
            "\"layer_b_route_proof_ready_count\":",
        ),
        (
            "rust_validation_layer_b_route_ready_count",
            "\"rust_validation_layer_b_route_ready_count\":",
        ),
        ("suite_pass_rate", "\"suite_pass_rate\":"),
        ("evidence_digest", "\"evidence_digest\":"),
        ("report_digest", "\"report_digest\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("summary", "\"summary\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing coding_service_eval field {name}"));
        }
    }

    if line.contains("\"prompt\"")
        || line.contains("\"messages\"")
        || line.contains("\"request\"")
        || line.contains("\"request_wire\"")
        || line.contains("\"request_evidence_packets\"")
        || line.contains("\"evidence_packets\"")
        || line.contains("\"run_records\"")
        || line.contains("\"observation\"")
        || line.contains("\"output\"")
        || line.contains("fn parse_port")
        || line.contains("借用 和 所有权")
        || contains_private_or_executable_marker(line)
    {
        failures.push("coding_service_eval trace must expose counts and digests only".to_owned());
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "coding_service_eval",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "coding_service_eval",
    );
    require_bool(&mut failures, line, "applied", false, "coding_service_eval");

    match extract_json_string_field(line, "schema") {
        Some(value) if value == CODING_SERVICE_EVAL_TRACE_SCHEMA => {}
        Some(value) => failures.push(format!(
            "coding_service_eval schema {value} is not supported"
        )),
        None => failures.push("coding_service_eval schema missing".to_owned()),
    }

    let report_schema = extract_json_string_field(line, "report_schema").unwrap_or_default();
    if !matches!(
        report_schema.as_str(),
        CODING_SERVICE_EVAL_SCHEMA_VERSION | CODING_SERVICE_EVAL_RUNNER_SCHEMA_VERSION
    ) {
        failures.push(format!(
            "coding_service_eval report_schema {report_schema} is not supported"
        ));
    }

    let report_kind = extract_json_string_field(line, "report_kind").unwrap_or_default();
    if !matches!(report_kind.as_str(), "readiness" | "runner") {
        failures.push(format!(
            "coding_service_eval report_kind {report_kind} is not supported"
        ));
    }
    if report_kind == "readiness" && report_schema != CODING_SERVICE_EVAL_SCHEMA_VERSION {
        failures.push("coding_service_eval readiness report_schema mismatch".to_owned());
    }
    if report_kind == "runner" && report_schema != CODING_SERVICE_EVAL_RUNNER_SCHEMA_VERSION {
        failures.push("coding_service_eval runner report_schema mismatch".to_owned());
    }

    let request_plan_count = extract_json_usize_field(line, "request_plan_count").unwrap_or(0);
    let fixture_count = extract_json_usize_field(line, "fixture_count").unwrap_or(0);
    let completed_count = extract_json_usize_field(line, "completed_count").unwrap_or(0);
    let profile_count = extract_json_usize_field(line, "profile_count").unwrap_or(0);
    let language_count = extract_json_usize_field(line, "language_count").unwrap_or(0);
    let evidence_packet_count =
        extract_json_usize_field(line, "evidence_packet_count").unwrap_or(0);
    let rust_validation_checked_count =
        extract_json_usize_field(line, "rust_validation_checked_count").unwrap_or(0);
    let compile_checked_count =
        extract_json_usize_field(line, "compile_checked_count").unwrap_or(0);
    let unit_test_checked_count =
        extract_json_usize_field(line, "unit_test_checked_count").unwrap_or(0);
    let benchmark_checked_count =
        extract_json_usize_field(line, "benchmark_checked_count").unwrap_or(0);
    let benchmark_passed_count =
        extract_json_usize_field(line, "benchmark_passed_count").unwrap_or(0);
    let layer_b_route_proof_ready_count =
        extract_json_usize_field(line, "layer_b_route_proof_ready_count").unwrap_or(0);
    let rust_validation_layer_b_route_ready_count =
        extract_json_usize_field(line, "rust_validation_layer_b_route_ready_count").unwrap_or(0);
    let suite_pass_rate = extract_json_f32_field(line, "suite_pass_rate").unwrap_or(-1.0);
    let profiles = extract_json_string_array_field(line, "profiles").unwrap_or_default();
    let languages = extract_json_string_array_field(line, "languages").unwrap_or_default();

    if request_plan_count == 0 {
        failures.push("coding_service_eval request_plan_count must be positive".to_owned());
    }
    if fixture_count != request_plan_count {
        failures.push("coding_service_eval fixture_count must match request_plan_count".to_owned());
    }
    if evidence_packet_count != request_plan_count {
        failures.push(
            "coding_service_eval evidence_packet_count must match request_plan_count".to_owned(),
        );
    }
    if profile_count != profiles.len() {
        failures.push("coding_service_eval profile_count mismatch".to_owned());
    }
    if language_count != languages.len() {
        failures.push("coding_service_eval language_count mismatch".to_owned());
    }
    if !(0.0..=1.0).contains(&suite_pass_rate) {
        failures.push("coding_service_eval suite_pass_rate out of range".to_owned());
    }

    if report_kind == "readiness" && completed_count != 0 {
        failures.push("coding_service_eval readiness must not execute plans".to_owned());
    }
    if report_kind == "runner" {
        if completed_count != request_plan_count {
            failures.push(
                "coding_service_eval runner completed_count must match request_plan_count"
                    .to_owned(),
            );
        }
        if rust_validation_checked_count == 0
            || compile_checked_count == 0
            || unit_test_checked_count == 0
        {
            failures.push(
                "coding_service_eval runner must include Rust validation/compiler/test checks"
                    .to_owned(),
            );
        }
        if benchmark_checked_count != request_plan_count
            || benchmark_passed_count != benchmark_checked_count
        {
            failures
                .push("coding_service_eval runner must include passed benchmark checks".to_owned());
        }
        if layer_b_route_proof_ready_count != request_plan_count {
            failures.push(
                "coding_service_eval runner must route every plan through Layer B proof".to_owned(),
            );
        }
        if rust_validation_layer_b_route_ready_count != rust_validation_checked_count {
            failures.push(
                "coding_service_eval runner Rust validation must carry Layer B route proof"
                    .to_owned(),
            );
        }
    } else if benchmark_checked_count != 0
        || benchmark_passed_count != 0
        || layer_b_route_proof_ready_count != 0
        || rust_validation_layer_b_route_ready_count != 0
    {
        failures.push("coding_service_eval readiness must not claim runner checks".to_owned());
    }

    for field in ["evidence_digest", "report_digest"] {
        let value = extract_json_string_field(line, field).unwrap_or_default();
        if !value.starts_with("redaction-digest:") {
            failures.push(format!(
                "coding_service_eval {field} must be a redaction digest"
            ));
        }
    }

    let summary = extract_json_string_field(line, "summary").unwrap_or_default();
    if contains_private_or_executable_marker(&summary) || summary.contains("prompt") {
        failures.push("coding_service_eval summary leaked private marker".to_owned());
    }

    failures
}

fn require_bool(
    failures: &mut Vec<String>,
    line: &str,
    field: &str,
    expected: bool,
    context: &str,
) {
    match extract_json_bool_field(line, field) {
        Some(value) if value == expected => {}
        Some(value) => failures.push(format!("{context} {field}={value} expected {expected}")),
        None => failures.push(format!("{context} {field} missing")),
    }
}
