use crate::privacy_redaction::contains_private_or_executable_marker;
use crate::writer_gate::UNIFIED_WRITER_GATE_TRACE_SCHEMA;

use super::fields::{extract_json_bool_field, extract_json_string_field, extract_json_usize_field};

pub(super) fn evaluate_unified_writer_gate_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-unified-writer-gate-v1\"",
        ),
        ("gate_schema", "\"gate_schema\":"),
        ("decision", "\"decision\":"),
        ("records", "\"records\":"),
        ("memory_records", "\"memory_records\":"),
        ("genome_records", "\"genome_records\":"),
        (
            "experiment_ledger_records",
            "\"experiment_ledger_records\":",
        ),
        (
            "evolution_goal_queue_records",
            "\"evolution_goal_queue_records\":",
        ),
        ("ready_records", "\"ready_records\":"),
        ("held_records", "\"held_records\":"),
        ("rejected_records", "\"rejected_records\":"),
        ("preview_only_records", "\"preview_only_records\":"),
        ("reason_code_count", "\"reason_code_count\":"),
        ("durable_write_allowed", "\"durable_write_allowed\":"),
        ("explicit_apply_required", "\"explicit_apply_required\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("evidence_digest", "\"evidence_digest\":"),
        ("summary", "\"summary\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing unified_writer_gate field {name}"));
        }
    }

    if line.contains("\"records\":[") || line.contains("\"record_summaries\":[") {
        failures.push("unified_writer_gate must expose records as count/digest only".to_owned());
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "unified_writer_gate",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "unified_writer_gate",
    );
    require_bool(
        &mut failures,
        line,
        "durable_write_allowed",
        false,
        "unified_writer_gate",
    );
    require_bool(&mut failures, line, "applied", false, "unified_writer_gate");
    require_bool(
        &mut failures,
        line,
        "explicit_apply_required",
        true,
        "unified_writer_gate",
    );

    let records = extract_json_usize_field(line, "records").unwrap_or(0);
    let memory_records = extract_json_usize_field(line, "memory_records").unwrap_or(0);
    let genome_records = extract_json_usize_field(line, "genome_records").unwrap_or(0);
    let experiment_ledger_records =
        extract_json_usize_field(line, "experiment_ledger_records").unwrap_or(0);
    let evolution_goal_queue_records =
        extract_json_usize_field(line, "evolution_goal_queue_records").unwrap_or(0);
    let ready_records = extract_json_usize_field(line, "ready_records").unwrap_or(0);
    let held_records = extract_json_usize_field(line, "held_records").unwrap_or(0);
    let rejected_records = extract_json_usize_field(line, "rejected_records").unwrap_or(0);
    let preview_only_records = extract_json_usize_field(line, "preview_only_records").unwrap_or(0);
    let reason_code_count = extract_json_usize_field(line, "reason_code_count").unwrap_or(0);

    if records == 0 {
        failures.push("unified_writer_gate records must be nonzero".to_owned());
    }
    if memory_records
        .saturating_add(genome_records)
        .saturating_add(experiment_ledger_records)
        .saturating_add(evolution_goal_queue_records)
        != records
    {
        failures.push("unified_writer_gate domain record counts do not match records".to_owned());
    }
    if ready_records
        .saturating_add(held_records)
        .saturating_add(rejected_records)
        .saturating_add(preview_only_records)
        != records
    {
        failures.push("unified_writer_gate decision record counts do not match records".to_owned());
    }
    if ready_records > 0 {
        failures.push(
            "unified_writer_gate ready_records require a separate explicit apply issue".to_owned(),
        );
    }
    if records > 0 && reason_code_count == 0 && ready_records == 0 {
        failures
            .push("unified_writer_gate non-ready preview records require reason codes".to_owned());
    }

    let decision = extract_json_string_field(line, "decision").unwrap_or_default();
    let expected_decision = if rejected_records > 0 {
        "reject"
    } else if held_records > 0 {
        "hold"
    } else {
        "preview_only"
    };
    if decision != expected_decision {
        failures.push(format!(
            "unified_writer_gate decision {decision} does not match counters {expected_decision}"
        ));
    }

    match extract_json_string_field(line, "gate_schema") {
        Some(value) if value == "unified_writer_gate_v1" => {}
        Some(value) => failures.push(format!(
            "unified_writer_gate gate_schema {value} is not supported"
        )),
        None => failures.push("unified_writer_gate gate_schema missing".to_owned()),
    }
    match extract_json_string_field(line, "schema") {
        Some(value) if value == UNIFIED_WRITER_GATE_TRACE_SCHEMA => {}
        Some(value) => failures.push(format!(
            "unified_writer_gate schema {value} is not supported"
        )),
        None => failures.push("unified_writer_gate schema missing".to_owned()),
    }

    let evidence_digest = extract_json_string_field(line, "evidence_digest").unwrap_or_default();
    if evidence_digest.trim().is_empty() {
        failures.push("unified_writer_gate evidence_digest missing".to_owned());
    }
    if contains_private_or_executable_marker(&evidence_digest) {
        failures.push("unified_writer_gate evidence_digest leaked private marker".to_owned());
    }
    let summary = extract_json_string_field(line, "summary").unwrap_or_default();
    if contains_private_or_executable_marker(&summary) {
        failures.push("unified_writer_gate summary leaked private marker".to_owned());
    }

    failures
}

fn require_bool(failures: &mut Vec<String>, line: &str, field: &str, expected: bool, label: &str) {
    match extract_json_bool_field(line, field) {
        Some(actual) if actual == expected => {}
        Some(actual) => failures.push(format!("{label} {field}={actual} expected {expected}")),
        None => failures.push(format!("{label} {field} missing")),
    }
}
