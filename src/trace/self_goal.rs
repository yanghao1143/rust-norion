use crate::privacy_redaction::contains_private_or_executable_marker;
use crate::self_goal_proposal::{
    SELF_GOAL_QUEUE_APPLY_PLAN_SCHEMA_VERSION, SELF_GOAL_QUEUE_APPLY_PLAN_TRACE_SCHEMA,
    SELF_GOAL_QUEUE_PREVIEW_SCHEMA_VERSION,
};
use crate::writer_gate::UNIFIED_WRITER_GATE_SCHEMA_VERSION;

use super::fields::{extract_json_bool_field, extract_json_string_field, extract_json_usize_field};

pub(super) fn evaluate_self_goal_queue_apply_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-goal-queue-apply-plan-v1\"",
        ),
        ("plan_schema", "\"plan_schema\":"),
        ("queue_preview_schema", "\"queue_preview_schema\":"),
        ("writer_gate_schema", "\"writer_gate_schema\":"),
        ("decision", "\"decision\":"),
        ("writer_gate_decision", "\"writer_gate_decision\":"),
        ("records", "\"records\":"),
        ("ready_records", "\"ready_records\":"),
        ("held_records", "\"held_records\":"),
        ("rejected_records", "\"rejected_records\":"),
        ("reason_code_count", "\"reason_code_count\":"),
        ("explicit_apply_required", "\"explicit_apply_required\":"),
        ("current_queue_digest", "\"current_queue_digest\":"),
        ("apply_plan_digest", "\"apply_plan_digest\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("summary", "\"summary\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing self_goal_queue_apply field {name}"));
        }
    }

    if line.contains("\"records\":[") || line.contains("\"record_lines\":[") {
        failures.push("self_goal_queue_apply must expose records as count/digest only".to_owned());
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_goal_queue_apply",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "self_goal_queue_apply",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "self_goal_queue_apply",
    );

    let records = extract_json_usize_field(line, "records").unwrap_or(0);
    let ready_records = extract_json_usize_field(line, "ready_records").unwrap_or(0);
    let held_records = extract_json_usize_field(line, "held_records").unwrap_or(0);
    let rejected_records = extract_json_usize_field(line, "rejected_records").unwrap_or(0);
    let reason_code_count = extract_json_usize_field(line, "reason_code_count").unwrap_or(0);
    let explicit_apply_required =
        extract_json_bool_field(line, "explicit_apply_required").unwrap_or(false);

    if records == 0 {
        failures.push("self_goal_queue_apply records must be nonzero".to_owned());
    }
    if ready_records
        .saturating_add(held_records)
        .saturating_add(rejected_records)
        != records
    {
        failures
            .push("self_goal_queue_apply decision record counts do not match records".to_owned());
    }
    if ready_records > 0 {
        failures.push(
            "self_goal_queue_apply ready_records require a separate append executor issue"
                .to_owned(),
        );
    }
    if ready_records > 0 && !explicit_apply_required {
        failures.push("self_goal_queue_apply ready records must require explicit apply".to_owned());
    }
    if records > 0 && reason_code_count == 0 {
        failures.push("self_goal_queue_apply records require reason codes".to_owned());
    }

    let decision = extract_json_string_field(line, "decision").unwrap_or_default();
    if rejected_records > 0 {
        if decision != "rejected" {
            failures.push(format!(
                "self_goal_queue_apply decision {decision} does not match rejected counters"
            ));
        }
    } else if ready_records > 0 && held_records == 0 {
        if decision != "ready_for_explicit_apply" {
            failures.push(format!(
                "self_goal_queue_apply decision {decision} does not match ready counters"
            ));
        }
    } else if !matches!(
        decision.as_str(),
        "held_for_writer_gate" | "held_for_append_packet" | "held_for_duplicate_goal"
    ) {
        failures.push(format!(
            "self_goal_queue_apply decision {decision} is not a supported hold decision"
        ));
    }

    match extract_json_string_field(line, "schema") {
        Some(value) if value == SELF_GOAL_QUEUE_APPLY_PLAN_TRACE_SCHEMA => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_apply schema {value} is not supported"
        )),
        None => failures.push("self_goal_queue_apply schema missing".to_owned()),
    }
    match extract_json_string_field(line, "plan_schema") {
        Some(value) if value == SELF_GOAL_QUEUE_APPLY_PLAN_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_apply plan_schema {value} is not supported"
        )),
        None => failures.push("self_goal_queue_apply plan_schema missing".to_owned()),
    }
    match extract_json_string_field(line, "queue_preview_schema") {
        Some(value) if value == SELF_GOAL_QUEUE_PREVIEW_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_apply queue_preview_schema {value} is not supported"
        )),
        None => failures.push("self_goal_queue_apply queue_preview_schema missing".to_owned()),
    }
    match extract_json_string_field(line, "writer_gate_schema") {
        Some(value) if value == UNIFIED_WRITER_GATE_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_apply writer_gate_schema {value} is not supported"
        )),
        None => failures.push("self_goal_queue_apply writer_gate_schema missing".to_owned()),
    }

    for field in ["current_queue_digest", "apply_plan_digest"] {
        let value = extract_json_string_field(line, field).unwrap_or_default();
        if !value.starts_with("redaction-digest:") {
            failures.push(format!(
                "self_goal_queue_apply {field} must be redaction digest"
            ));
        }
        if contains_private_or_executable_marker(&value) {
            failures.push(format!(
                "self_goal_queue_apply {field} leaked private marker"
            ));
        }
    }

    let summary = extract_json_string_field(line, "summary").unwrap_or_default();
    if contains_private_or_executable_marker(&summary) {
        failures.push("self_goal_queue_apply summary leaked private marker".to_owned());
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
