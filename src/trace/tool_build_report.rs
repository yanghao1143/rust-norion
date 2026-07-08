use super::fields::{extract_json_bool_field, extract_json_string_field, extract_json_usize_field};
use crate::privacy_redaction::contains_private_or_executable_marker;

pub const AGENT_TOOL_BUILD_REPORT_TRACE_SCHEMA: &str = "rust-norion-agent-tool-build-report-v1";

pub(super) fn evaluate_agent_tool_build_report_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-agent-tool-build-report-v1\"",
        ),
        ("report_kind", "\"report_kind\":"),
        ("records", "\"records\":"),
        ("requested", "\"requested\":"),
        ("received", "\"received\":"),
        ("built", "\"built\":"),
        ("held", "\"held\":"),
        ("rejected", "\"rejected\":"),
        ("missing_requests", "\"missing_requests\":"),
        ("unexpected_receipts", "\"unexpected_receipts\":"),
        ("duplicate_receipts", "\"duplicate_receipts\":"),
        ("diagnostics", "\"diagnostics\":"),
        ("clean", "\"clean\":"),
        ("reliable", "\"reliable\":"),
        ("open_tool_build_boundary", "\"open_tool_build_boundary\":"),
        ("finalize_eval", "\"finalize_eval\":"),
        ("requires_repair_first", "\"requires_repair_first\":"),
        ("repair_tasks", "\"repair_tasks\":"),
        ("reason_count", "\"reason_count\":"),
        ("health", "\"health\":"),
        ("report_digest", "\"report_digest\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing agent_tool_build_report field {name}"));
        }
    }

    if line.contains("\"artifact\"")
        || line.contains("\"diagnostic\"")
        || line.contains("\"source\"")
        || line.contains("\"prompt\"")
        || contains_private_or_executable_marker(line)
    {
        failures
            .push("agent_tool_build_report trace must expose counts and digests only".to_owned());
    }

    require_bool(&mut failures, line, "read_only", true);
    require_bool(&mut failures, line, "write_allowed", false);
    require_bool(&mut failures, line, "applied", false);

    match extract_json_string_field(line, "schema") {
        Some(value) if value == AGENT_TOOL_BUILD_REPORT_TRACE_SCHEMA => {}
        Some(value) => failures.push(format!(
            "agent_tool_build_report schema {value} is not supported"
        )),
        None => failures.push("agent_tool_build_report schema missing".to_owned()),
    }

    if extract_json_string_field(line, "report_kind").as_deref() != Some("history_gate") {
        failures.push("agent_tool_build_report report_kind must be history_gate".to_owned());
    }

    let records = extract_json_usize_field(line, "records").unwrap_or(0);
    let requested = extract_json_usize_field(line, "requested").unwrap_or(0);
    let received = extract_json_usize_field(line, "received").unwrap_or(0);
    let built = extract_json_usize_field(line, "built").unwrap_or(0);
    let held = extract_json_usize_field(line, "held").unwrap_or(0);
    let rejected = extract_json_usize_field(line, "rejected").unwrap_or(0);
    let missing_requests = extract_json_usize_field(line, "missing_requests").unwrap_or(0);
    let unexpected_receipts = extract_json_usize_field(line, "unexpected_receipts").unwrap_or(0);
    let duplicate_receipts = extract_json_usize_field(line, "duplicate_receipts").unwrap_or(0);
    let diagnostics = extract_json_usize_field(line, "diagnostics").unwrap_or(0);
    let clean = extract_json_bool_field(line, "clean").unwrap_or(false);
    let reliable = extract_json_bool_field(line, "reliable").unwrap_or(false);
    let open_tool_build_boundary =
        extract_json_bool_field(line, "open_tool_build_boundary").unwrap_or(false);
    let finalize_eval = extract_json_bool_field(line, "finalize_eval").unwrap_or(false);
    let requires_repair_first =
        extract_json_bool_field(line, "requires_repair_first").unwrap_or(true);
    let repair_tasks = extract_json_usize_field(line, "repair_tasks").unwrap_or(usize::MAX);
    let reason_count = extract_json_usize_field(line, "reason_count").unwrap_or(usize::MAX);

    if records == 0 {
        failures.push("agent_tool_build_report records must be positive".to_owned());
    }
    if requested == 0 {
        failures.push("agent_tool_build_report requested must be positive".to_owned());
    }
    if received != requested || built != requested {
        failures.push(
            "agent_tool_build_report received and built counts must match requested".to_owned(),
        );
    }
    if held + rejected + missing_requests + unexpected_receipts + duplicate_receipts + diagnostics
        > 0
    {
        failures.push("agent_tool_build_report must be clean receipt-only evidence".to_owned());
    }
    if !clean || !reliable || !open_tool_build_boundary || !finalize_eval || requires_repair_first {
        failures.push("agent_tool_build_report gate is not ready".to_owned());
    }
    if repair_tasks != 0 || reason_count != 0 {
        failures.push("agent_tool_build_report repair pressure must be zero".to_owned());
    }
    if extract_json_string_field(line, "health").as_deref() != Some("Stable") {
        failures.push("agent_tool_build_report health must be Stable".to_owned());
    }
    let report_digest = extract_json_string_field(line, "report_digest").unwrap_or_default();
    if !report_digest.starts_with("redaction-digest:") {
        failures
            .push("agent_tool_build_report report_digest must be a redaction digest".to_owned());
    }

    failures
}

fn require_bool(failures: &mut Vec<String>, line: &str, field: &str, expected: bool) {
    match extract_json_bool_field(line, field) {
        Some(value) if value == expected => {}
        Some(value) => failures.push(format!(
            "agent_tool_build_report {field}={value} expected {expected}"
        )),
        None => failures.push(format!("agent_tool_build_report {field} missing")),
    }
}
