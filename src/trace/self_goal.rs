use crate::evolution_goal_queue_store::{
    EVOLUTION_GOAL_QUEUE_STORE_SCHEMA_VERSION, EVOLUTION_GOAL_QUEUE_STORE_WRITE_TRACE_SCHEMA,
};
use crate::privacy_redaction::contains_private_or_executable_marker;
use crate::self_goal_proposal::{
    SELF_GOAL_PROPOSAL_SCHEMA_VERSION, SELF_GOAL_QUEUE_APPEND_APPROVAL_SCHEMA_VERSION,
    SELF_GOAL_QUEUE_APPEND_EXECUTION_SCHEMA_VERSION, SELF_GOAL_QUEUE_APPEND_EXECUTION_TRACE_SCHEMA,
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

pub(super) fn evaluate_self_goal_queue_append_execution_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-goal-queue-append-execution-v1\"",
        ),
        ("execution_schema", "\"execution_schema\":"),
        ("approval_schema", "\"approval_schema\":"),
        ("apply_plan_schema", "\"apply_plan_schema\":"),
        ("queue_preview_schema", "\"queue_preview_schema\":"),
        ("proposal_schema", "\"proposal_schema\":"),
        ("decision", "\"decision\":"),
        ("records", "\"records\":"),
        ("applied_records", "\"applied_records\":"),
        ("held_records", "\"held_records\":"),
        ("rejected_records", "\"rejected_records\":"),
        ("reason_code_count", "\"reason_code_count\":"),
        ("current_queue_digest", "\"current_queue_digest\":"),
        ("rollback_anchor_digest", "\"rollback_anchor_digest\":"),
        ("append_record_digest", "\"append_record_digest\":"),
        ("resulting_queue_digest", "\"resulting_queue_digest\":"),
        ("apply_plan_digest", "\"apply_plan_digest\":"),
        (
            "approval_attestation_digest",
            "\"approval_attestation_digest\":",
        ),
        ("durable_write_allowed", "\"durable_write_allowed\":"),
        ("in_memory_write_allowed", "\"in_memory_write_allowed\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("summary", "\"summary\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!(
                "missing self_goal_queue_append_execution field {name}"
            ));
        }
    }

    if line.contains("\"records\":[")
        || line.contains("\"record_lines\":[")
        || line.contains("\"resulting_queue\":")
    {
        failures.push(
            "self_goal_queue_append_execution must expose count/digest execution evidence only"
                .to_owned(),
        );
    }

    require_bool(
        &mut failures,
        line,
        "durable_write_allowed",
        false,
        "self_goal_queue_append_execution",
    );

    let records = extract_json_usize_field(line, "records").unwrap_or(0);
    let applied_records = extract_json_usize_field(line, "applied_records").unwrap_or(0);
    let held_records = extract_json_usize_field(line, "held_records").unwrap_or(0);
    let rejected_records = extract_json_usize_field(line, "rejected_records").unwrap_or(0);
    let reason_code_count = extract_json_usize_field(line, "reason_code_count").unwrap_or(0);
    let decision = extract_json_string_field(line, "decision").unwrap_or_default();
    let read_only = extract_json_bool_field(line, "read_only").unwrap_or(false);
    let write_allowed = extract_json_bool_field(line, "write_allowed").unwrap_or(false);
    let in_memory_write_allowed =
        extract_json_bool_field(line, "in_memory_write_allowed").unwrap_or(false);
    let applied = extract_json_bool_field(line, "applied").unwrap_or(false);

    if records == 0 {
        failures.push("self_goal_queue_append_execution records must be nonzero".to_owned());
    }
    if applied_records
        .saturating_add(held_records)
        .saturating_add(rejected_records)
        != records
    {
        failures.push(
            "self_goal_queue_append_execution decision record counts do not match records"
                .to_owned(),
        );
    }

    match decision.as_str() {
        "applied" => {
            if applied_records == 0 || held_records > 0 || rejected_records > 0 {
                failures.push(
                    "self_goal_queue_append_execution applied counters are inconsistent".to_owned(),
                );
            }
            if read_only || !write_allowed || !in_memory_write_allowed || !applied {
                failures.push(
                    "self_goal_queue_append_execution applied trace requires in-memory write/applied flags"
                        .to_owned(),
                );
            }
            if reason_code_count != 0 {
                failures.push(
                    "self_goal_queue_append_execution applied trace must not carry reason codes"
                        .to_owned(),
                );
            }
        }
        "hold" => {
            if held_records == 0 || applied_records > 0 || rejected_records > 0 {
                failures.push(
                    "self_goal_queue_append_execution hold counters are inconsistent".to_owned(),
                );
            }
            if !read_only || write_allowed || in_memory_write_allowed || applied {
                failures.push(
                    "self_goal_queue_append_execution hold trace must remain read-only".to_owned(),
                );
            }
            if reason_code_count == 0 {
                failures.push(
                    "self_goal_queue_append_execution hold trace requires reason codes".to_owned(),
                );
            }
        }
        "rejected" => {
            if rejected_records == 0 || applied_records > 0 || held_records > 0 {
                failures.push(
                    "self_goal_queue_append_execution rejected counters are inconsistent"
                        .to_owned(),
                );
            }
            if !read_only || write_allowed || in_memory_write_allowed || applied {
                failures.push(
                    "self_goal_queue_append_execution rejected trace must remain read-only"
                        .to_owned(),
                );
            }
            if reason_code_count == 0 {
                failures.push(
                    "self_goal_queue_append_execution rejected trace requires reason codes"
                        .to_owned(),
                );
            }
        }
        _ => failures.push(format!(
            "self_goal_queue_append_execution decision {decision} is not supported"
        )),
    }

    match extract_json_string_field(line, "schema") {
        Some(value) if value == SELF_GOAL_QUEUE_APPEND_EXECUTION_TRACE_SCHEMA => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_append_execution schema {value} is not supported"
        )),
        None => failures.push("self_goal_queue_append_execution schema missing".to_owned()),
    }
    match extract_json_string_field(line, "execution_schema") {
        Some(value) if value == SELF_GOAL_QUEUE_APPEND_EXECUTION_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_append_execution execution_schema {value} is not supported"
        )),
        None => {
            failures.push("self_goal_queue_append_execution execution_schema missing".to_owned())
        }
    }
    match extract_json_string_field(line, "approval_schema") {
        Some(value) if value == SELF_GOAL_QUEUE_APPEND_APPROVAL_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_append_execution approval_schema {value} is not supported"
        )),
        None => {
            failures.push("self_goal_queue_append_execution approval_schema missing".to_owned())
        }
    }
    match extract_json_string_field(line, "apply_plan_schema") {
        Some(value) if value == SELF_GOAL_QUEUE_APPLY_PLAN_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_append_execution apply_plan_schema {value} is not supported"
        )),
        None => {
            failures.push("self_goal_queue_append_execution apply_plan_schema missing".to_owned())
        }
    }
    match extract_json_string_field(line, "queue_preview_schema") {
        Some(value) if value == SELF_GOAL_QUEUE_PREVIEW_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_append_execution queue_preview_schema {value} is not supported"
        )),
        None => failures
            .push("self_goal_queue_append_execution queue_preview_schema missing".to_owned()),
    }
    match extract_json_string_field(line, "proposal_schema") {
        Some(value) if value == SELF_GOAL_PROPOSAL_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_append_execution proposal_schema {value} is not supported"
        )),
        None => {
            failures.push("self_goal_queue_append_execution proposal_schema missing".to_owned())
        }
    }

    for field in [
        "current_queue_digest",
        "rollback_anchor_digest",
        "append_record_digest",
        "resulting_queue_digest",
        "apply_plan_digest",
        "approval_attestation_digest",
    ] {
        let value = extract_json_string_field(line, field).unwrap_or_default();
        if value != "none" && !value.starts_with("redaction-digest:") {
            failures.push(format!(
                "self_goal_queue_append_execution {field} must be redaction digest or none"
            ));
        }
        if contains_private_or_executable_marker(&value) {
            failures.push(format!(
                "self_goal_queue_append_execution {field} leaked private marker"
            ));
        }
    }

    let summary = extract_json_string_field(line, "summary").unwrap_or_default();
    if contains_private_or_executable_marker(&summary) {
        failures.push("self_goal_queue_append_execution summary leaked private marker".to_owned());
    }

    failures
}

pub(super) fn evaluate_evolution_goal_queue_store_write_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-evolution-goal-queue-store-write-v1\"",
        ),
        ("store_schema", "\"store_schema\":"),
        ("decision", "\"decision\":"),
        ("reason_code_count", "\"reason_code_count\":"),
        ("key_digest", "\"key_digest\":"),
        ("queue_digest", "\"queue_digest\":"),
        ("rollback_anchor_digest", "\"rollback_anchor_digest\":"),
        (
            "approval_attestation_digest",
            "\"approval_attestation_digest\":",
        ),
        ("tenant_isolation_allowed", "\"tenant_isolation_allowed\":"),
        ("isolation_decision", "\"isolation_decision\":"),
        ("durable_write_allowed", "\"durable_write_allowed\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("summary", "\"summary\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!(
                "missing evolution_goal_queue_store_write field {name}"
            ));
        }
    }

    if line.contains("\"queue\":")
        || line.contains("\"goals\":[")
        || line.contains("\"record_lines\":[")
        || line.contains("\"reason_codes\":[")
    {
        failures.push(
            "evolution_goal_queue_store_write must expose digest/count evidence only".to_owned(),
        );
    }

    match extract_json_string_field(line, "schema") {
        Some(value) if value == EVOLUTION_GOAL_QUEUE_STORE_WRITE_TRACE_SCHEMA => {}
        Some(value) => failures.push(format!(
            "evolution_goal_queue_store_write schema {value} is not supported"
        )),
        None => failures.push("evolution_goal_queue_store_write schema missing".to_owned()),
    }
    match extract_json_string_field(line, "store_schema") {
        Some(value) if value == EVOLUTION_GOAL_QUEUE_STORE_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "evolution_goal_queue_store_write store_schema {value} is not supported"
        )),
        None => failures.push("evolution_goal_queue_store_write store_schema missing".to_owned()),
    }

    let decision = extract_json_string_field(line, "decision").unwrap_or_default();
    let reason_code_count = extract_json_usize_field(line, "reason_code_count").unwrap_or(0);
    let tenant_isolation_allowed =
        extract_json_bool_field(line, "tenant_isolation_allowed").unwrap_or(false);
    let read_only = extract_json_bool_field(line, "read_only").unwrap_or(false);
    let write_allowed = extract_json_bool_field(line, "write_allowed").unwrap_or(false);
    let durable_write_allowed =
        extract_json_bool_field(line, "durable_write_allowed").unwrap_or(false);
    let applied = extract_json_bool_field(line, "applied").unwrap_or(false);

    match decision.as_str() {
        "applied" => {
            if reason_code_count != 0 {
                failures.push(
                    "evolution_goal_queue_store_write applied trace must not carry reason codes"
                        .to_owned(),
                );
            }
            if !tenant_isolation_allowed
                || read_only
                || !write_allowed
                || !durable_write_allowed
                || !applied
            {
                failures.push(
                    "evolution_goal_queue_store_write applied trace requires isolated durable write flags"
                        .to_owned(),
                );
            }
        }
        "hold" => {
            if reason_code_count == 0 {
                failures.push(
                    "evolution_goal_queue_store_write hold trace requires reason codes".to_owned(),
                );
            }
            if !read_only || write_allowed || durable_write_allowed || applied {
                failures.push(
                    "evolution_goal_queue_store_write hold trace must remain read-only".to_owned(),
                );
            }
        }
        "rejected" => {
            if reason_code_count == 0 {
                failures.push(
                    "evolution_goal_queue_store_write rejected trace requires reason codes"
                        .to_owned(),
                );
            }
            if !read_only || write_allowed || durable_write_allowed || applied {
                failures.push(
                    "evolution_goal_queue_store_write rejected trace must remain read-only"
                        .to_owned(),
                );
            }
        }
        _ => failures.push(format!(
            "evolution_goal_queue_store_write decision {decision} is not supported"
        )),
    }

    let key_digest = extract_json_string_field(line, "key_digest").unwrap_or_default();
    if !key_digest.starts_with("fnv64:") {
        failures.push("evolution_goal_queue_store_write key_digest must be fnv64".to_owned());
    }
    if contains_private_or_executable_marker(&key_digest) {
        failures
            .push("evolution_goal_queue_store_write key_digest leaked private marker".to_owned());
    }

    for field in [
        "queue_digest",
        "rollback_anchor_digest",
        "approval_attestation_digest",
    ] {
        let value = extract_json_string_field(line, field).unwrap_or_default();
        if value != "none" && !value.starts_with("redaction-digest:") {
            failures.push(format!(
                "evolution_goal_queue_store_write {field} must be redaction digest or none"
            ));
        }
        if contains_private_or_executable_marker(&value) {
            failures.push(format!(
                "evolution_goal_queue_store_write {field} leaked private marker"
            ));
        }
    }

    let isolation_decision =
        extract_json_string_field(line, "isolation_decision").unwrap_or_default();
    if !matches!(isolation_decision.as_str(), "allowed" | "rejected") {
        failures.push(
            "evolution_goal_queue_store_write isolation_decision is not supported".to_owned(),
        );
    }

    let summary = extract_json_string_field(line, "summary").unwrap_or_default();
    if contains_private_or_executable_marker(&summary) {
        failures.push("evolution_goal_queue_store_write summary leaked private marker".to_owned());
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
