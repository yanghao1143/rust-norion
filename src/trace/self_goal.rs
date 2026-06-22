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

use super::fields::{
    extract_json_bool_field, extract_json_string_array_field, extract_json_string_field,
    extract_json_usize_field,
};

const SELF_GOAL_QUEUE_CONTINUATION_PLAN_SCHEMA_VERSION: &str =
    "self_goal_queue_continuation_plan_v1";
const SELF_GOAL_QUEUE_CONTINUATION_PLAN_TRACE_SCHEMA: &str =
    "rust-norion-self-goal-queue-continuation-plan-v1";
const SELF_GOAL_QUEUE_EVIDENCE_PLAN_SCHEMA_VERSION: &str = "self_goal_queue_evidence_plan_v1";
const SELF_GOAL_QUEUE_EVIDENCE_PLAN_TRACE_SCHEMA: &str =
    "rust-norion-self-goal-queue-evidence-plan-v1";
const SELF_GOAL_QUEUE_EVIDENCE_COLLECTION_SCHEMA_VERSION: &str =
    "self_goal_queue_evidence_collection_v1";
const SELF_GOAL_QUEUE_EVIDENCE_COLLECTION_TRACE_SCHEMA: &str =
    "rust-norion-self-goal-queue-evidence-collection-v1";

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

pub(super) fn evaluate_self_goal_queue_continuation_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-goal-queue-continuation-plan-v1\"",
        ),
        ("plan_schema", "\"plan_schema\":"),
        ("source", "\"source\":"),
        ("ready", "\"ready\":"),
        ("queue_digest", "\"queue_digest\":"),
        ("goals", "\"goals\":"),
        ("active", "\"active\":"),
        ("active_goal_id", "\"active_goal_id\":"),
        ("required_evidence_count", "\"required_evidence_count\":"),
        ("required_evidence", "\"required_evidence\":"),
        ("evidence_template_digest", "\"evidence_template_digest\":"),
        ("continuation_digest", "\"continuation_digest\":"),
        ("budget_attempts", "\"budget_attempts\":"),
        ("budget_steps", "\"budget_steps\":"),
        ("budget_tokens", "\"budget_tokens\":"),
        ("budget_runtime_ms", "\"budget_runtime_ms\":"),
        ("reason_code_count", "\"reason_code_count\":"),
        ("reason_codes", "\"reason_codes\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("summary", "\"summary\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing self_goal_queue_continuation field {name}"));
        }
    }

    if line.contains("\"records\":[")
        || line.contains("\"record_lines\":[")
        || line.contains("\"goals\":[")
        || line.contains("\"objective\":")
        || line.contains("\"resulting_queue\":")
    {
        failures
            .push("self_goal_queue_continuation must expose plan counts/digests only".to_owned());
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_goal_queue_continuation",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "self_goal_queue_continuation",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "self_goal_queue_continuation",
    );

    match extract_json_string_field(line, "schema") {
        Some(value) if value == SELF_GOAL_QUEUE_CONTINUATION_PLAN_TRACE_SCHEMA => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_continuation schema {value} is not supported"
        )),
        None => failures.push("self_goal_queue_continuation schema missing".to_owned()),
    }
    match extract_json_string_field(line, "plan_schema") {
        Some(value) if value == SELF_GOAL_QUEUE_CONTINUATION_PLAN_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_continuation plan_schema {value} is not supported"
        )),
        None => failures.push("self_goal_queue_continuation plan_schema missing".to_owned()),
    }

    let source = extract_json_string_field(line, "source").unwrap_or_default();
    if !matches!(
        source.as_str(),
        "current_queue" | "completion_resulting_queue"
    ) {
        failures.push("self_goal_queue_continuation source is not supported".to_owned());
    }

    let ready = extract_json_bool_field(line, "ready").unwrap_or(false);
    let active = extract_json_bool_field(line, "active").unwrap_or(false);
    if ready != active {
        failures.push("self_goal_queue_continuation ready must match active".to_owned());
    }

    let goals = extract_json_usize_field(line, "goals").unwrap_or(0);
    let required_evidence_count =
        extract_json_usize_field(line, "required_evidence_count").unwrap_or(0);
    let required_evidence =
        extract_json_string_array_field(line, "required_evidence").unwrap_or_default();
    let reason_code_count = extract_json_usize_field(line, "reason_code_count").unwrap_or(0);
    let reason_codes = extract_json_string_array_field(line, "reason_codes").unwrap_or_default();

    if ready && goals == 0 {
        failures.push("self_goal_queue_continuation ready plan requires retained goals".to_owned());
    }
    if active && required_evidence.is_empty() {
        failures
            .push("self_goal_queue_continuation active plan requires evidence kinds".to_owned());
    }
    if required_evidence_count != required_evidence.len() {
        failures.push("self_goal_queue_continuation required evidence count mismatch".to_owned());
    }
    if reason_code_count != reason_codes.len() {
        failures.push("self_goal_queue_continuation reason code count mismatch".to_owned());
    }
    if reason_codes.is_empty() {
        failures.push("self_goal_queue_continuation requires reason codes".to_owned());
    }
    for evidence in &required_evidence {
        if !matches!(
            evidence.as_str(),
            "cargo_check"
                | "focused_tests"
                | "benchmark_gate"
                | "trace_schema_gate"
                | "experiment_ledger"
                | "operator_approval"
        ) {
            failures.push(format!(
                "self_goal_queue_continuation evidence kind {evidence} is not supported"
            ));
        }
        if contains_private_or_executable_marker(evidence) {
            failures.push("self_goal_queue_continuation evidence kind leaked marker".to_owned());
        }
    }
    for reason in &reason_codes {
        if contains_private_or_executable_marker(reason) {
            failures.push("self_goal_queue_continuation reason leaked marker".to_owned());
        }
    }

    let active_goal_id = extract_json_string_field(line, "active_goal_id").unwrap_or_default();
    if active_goal_id != "none" && !active_goal_id.starts_with("redaction-digest:") {
        failures.push(
            "self_goal_queue_continuation active_goal_id must be redaction digest or none"
                .to_owned(),
        );
    }
    if !active && active_goal_id != "none" {
        failures.push("self_goal_queue_continuation inactive plan must not name a goal".to_owned());
    }
    if active && active_goal_id == "none" {
        failures.push("self_goal_queue_continuation active plan must name a goal".to_owned());
    }

    for field in [
        "queue_digest",
        "evidence_template_digest",
        "continuation_digest",
    ] {
        let value = extract_json_string_field(line, field).unwrap_or_default();
        if !value.starts_with("redaction-digest:") {
            failures.push(format!(
                "self_goal_queue_continuation {field} must be redaction digest"
            ));
        }
        if contains_private_or_executable_marker(&value) {
            failures.push(format!(
                "self_goal_queue_continuation {field} leaked private marker"
            ));
        }
    }

    for field in [
        "budget_attempts",
        "budget_steps",
        "budget_tokens",
        "budget_runtime_ms",
    ] {
        if active && extract_json_usize_field(line, field).unwrap_or(0) == 0 {
            failures.push(format!(
                "self_goal_queue_continuation active {field} must be nonzero"
            ));
        }
    }

    let summary = extract_json_string_field(line, "summary").unwrap_or_default();
    if contains_private_or_executable_marker(&summary) {
        failures.push("self_goal_queue_continuation summary leaked private marker".to_owned());
    }

    failures
}

pub(super) fn evaluate_self_goal_queue_evidence_plan_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-goal-queue-evidence-plan-v1\"",
        ),
        ("plan_schema", "\"plan_schema\":"),
        ("source", "\"source\":"),
        ("ready", "\"ready\":"),
        ("active_goal_id", "\"active_goal_id\":"),
        ("required_evidence_count", "\"required_evidence_count\":"),
        ("required_evidence", "\"required_evidence\":"),
        ("planned_step_count", "\"planned_step_count\":"),
        ("step_kinds", "\"step_kinds\":"),
        ("auto_collectible_steps", "\"auto_collectible_steps\":"),
        ("manual_steps", "\"manual_steps\":"),
        ("evidence_template_digest", "\"evidence_template_digest\":"),
        ("evidence_plan_digest", "\"evidence_plan_digest\":"),
        ("packet_template_digests", "\"packet_template_digests\":"),
        ("command_digests", "\"command_digests\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("summary", "\"summary\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!(
                "missing self_goal_queue_evidence_plan field {name}"
            ));
        }
    }

    if line.contains("\"records\":[")
        || line.contains("\"record_lines\":[")
        || line.contains("\"objective\":")
        || line.contains("\"command\":")
        || line.contains("\"commands\":[")
        || line.contains("\"resulting_queue\":")
    {
        failures
            .push("self_goal_queue_evidence_plan must expose plan counts/digests only".to_owned());
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_goal_queue_evidence_plan",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "self_goal_queue_evidence_plan",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "self_goal_queue_evidence_plan",
    );

    match extract_json_string_field(line, "schema") {
        Some(value) if value == SELF_GOAL_QUEUE_EVIDENCE_PLAN_TRACE_SCHEMA => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_evidence_plan schema {value} is not supported"
        )),
        None => failures.push("self_goal_queue_evidence_plan schema missing".to_owned()),
    }
    match extract_json_string_field(line, "plan_schema") {
        Some(value) if value == SELF_GOAL_QUEUE_EVIDENCE_PLAN_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_evidence_plan plan_schema {value} is not supported"
        )),
        None => failures.push("self_goal_queue_evidence_plan plan_schema missing".to_owned()),
    }

    let source = extract_json_string_field(line, "source").unwrap_or_default();
    if !matches!(
        source.as_str(),
        "current_queue" | "completion_resulting_queue"
    ) {
        failures.push("self_goal_queue_evidence_plan source is not supported".to_owned());
    }

    let ready = extract_json_bool_field(line, "ready").unwrap_or(false);
    let required_evidence_count =
        extract_json_usize_field(line, "required_evidence_count").unwrap_or(0);
    let required_evidence =
        extract_json_string_array_field(line, "required_evidence").unwrap_or_default();
    let planned_step_count = extract_json_usize_field(line, "planned_step_count").unwrap_or(0);
    let step_kinds = extract_json_string_array_field(line, "step_kinds").unwrap_or_default();
    let packet_digests =
        extract_json_string_array_field(line, "packet_template_digests").unwrap_or_default();
    let command_digests =
        extract_json_string_array_field(line, "command_digests").unwrap_or_default();
    let auto_collectible_steps =
        extract_json_usize_field(line, "auto_collectible_steps").unwrap_or(0);
    let manual_steps = extract_json_usize_field(line, "manual_steps").unwrap_or(0);

    if required_evidence_count != required_evidence.len() {
        failures.push("self_goal_queue_evidence_plan required evidence count mismatch".to_owned());
    }
    if planned_step_count != step_kinds.len()
        || planned_step_count != packet_digests.len()
        || planned_step_count != command_digests.len()
    {
        failures.push("self_goal_queue_evidence_plan step count mismatch".to_owned());
    }
    if auto_collectible_steps.saturating_add(manual_steps) != planned_step_count {
        failures.push("self_goal_queue_evidence_plan auto/manual count mismatch".to_owned());
    }
    if ready && planned_step_count == 0 {
        failures.push("self_goal_queue_evidence_plan ready plan requires steps".to_owned());
    }
    if !ready && planned_step_count > 0 {
        failures.push("self_goal_queue_evidence_plan held plan must not include steps".to_owned());
    }

    for evidence in required_evidence.iter().chain(step_kinds.iter()) {
        if !matches!(
            evidence.as_str(),
            "cargo_check"
                | "focused_tests"
                | "benchmark_gate"
                | "trace_schema_gate"
                | "experiment_ledger"
                | "operator_approval"
        ) {
            failures.push(format!(
                "self_goal_queue_evidence_plan evidence kind {evidence} is not supported"
            ));
        }
        if contains_private_or_executable_marker(evidence) {
            failures.push("self_goal_queue_evidence_plan evidence kind leaked marker".to_owned());
        }
    }

    let active_goal_id = extract_json_string_field(line, "active_goal_id").unwrap_or_default();
    if active_goal_id != "none" && !active_goal_id.starts_with("redaction-digest:") {
        failures.push(
            "self_goal_queue_evidence_plan active_goal_id must be redaction digest or none"
                .to_owned(),
        );
    }
    if ready && active_goal_id == "none" {
        failures.push("self_goal_queue_evidence_plan ready plan must name a goal".to_owned());
    }
    if !ready && active_goal_id != "none" {
        failures.push("self_goal_queue_evidence_plan held plan must not name a goal".to_owned());
    }

    for field in ["evidence_template_digest", "evidence_plan_digest"] {
        let value = extract_json_string_field(line, field).unwrap_or_default();
        if !value.starts_with("redaction-digest:") {
            failures.push(format!(
                "self_goal_queue_evidence_plan {field} must be redaction digest"
            ));
        }
        if contains_private_or_executable_marker(&value) {
            failures.push(format!(
                "self_goal_queue_evidence_plan {field} leaked private marker"
            ));
        }
    }
    for value in packet_digests.iter().chain(command_digests.iter()) {
        if !value.starts_with("redaction-digest:") {
            failures.push(
                "self_goal_queue_evidence_plan step digest must be redaction digest".to_owned(),
            );
        }
        if contains_private_or_executable_marker(value) {
            failures.push("self_goal_queue_evidence_plan step digest leaked marker".to_owned());
        }
    }

    let summary = extract_json_string_field(line, "summary").unwrap_or_default();
    if contains_private_or_executable_marker(&summary) {
        failures.push("self_goal_queue_evidence_plan summary leaked private marker".to_owned());
    }

    failures
}

pub(super) fn evaluate_self_goal_queue_evidence_collection_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-goal-queue-evidence-collection-v1\"",
        ),
        ("collection_schema", "\"collection_schema\":"),
        ("source", "\"source\":"),
        ("ready", "\"ready\":"),
        ("collection_complete", "\"collection_complete\":"),
        ("active_goal_id", "\"active_goal_id\":"),
        ("planned_step_count", "\"planned_step_count\":"),
        ("step_kinds", "\"step_kinds\":"),
        ("step_statuses", "\"step_statuses\":"),
        ("passed_steps", "\"passed_steps\":"),
        ("failed_steps", "\"failed_steps\":"),
        ("missing_steps", "\"missing_steps\":"),
        ("manual_missing_steps", "\"manual_missing_steps\":"),
        ("auto_collectible_steps", "\"auto_collectible_steps\":"),
        ("manual_required_steps", "\"manual_required_steps\":"),
        ("collected_evidence_count", "\"collected_evidence_count\":"),
        (
            "collected_evidence_digests",
            "\"collected_evidence_digests\":",
        ),
        (
            "collection_packet_digests",
            "\"collection_packet_digests\":",
        ),
        (
            "evidence_collection_digest",
            "\"evidence_collection_digest\":",
        ),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("summary", "\"summary\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!(
                "missing self_goal_queue_evidence_collection field {name}"
            ));
        }
    }

    if line.contains("\"records\":[")
        || line.contains("\"record_lines\":[")
        || line.contains("\"objective\":")
        || line.contains("\"command\":")
        || line.contains("\"commands\":[")
        || line.contains("\"resulting_queue\":")
        || line.contains("\"evidence_packets\":[")
    {
        failures.push(
            "self_goal_queue_evidence_collection must expose collection counts/digests only"
                .to_owned(),
        );
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_goal_queue_evidence_collection",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "self_goal_queue_evidence_collection",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "self_goal_queue_evidence_collection",
    );

    match extract_json_string_field(line, "schema") {
        Some(value) if value == SELF_GOAL_QUEUE_EVIDENCE_COLLECTION_TRACE_SCHEMA => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_evidence_collection schema {value} is not supported"
        )),
        None => failures.push("self_goal_queue_evidence_collection schema missing".to_owned()),
    }
    match extract_json_string_field(line, "collection_schema") {
        Some(value) if value == SELF_GOAL_QUEUE_EVIDENCE_COLLECTION_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "self_goal_queue_evidence_collection collection_schema {value} is not supported"
        )),
        None => failures
            .push("self_goal_queue_evidence_collection collection_schema missing".to_owned()),
    }

    let source = extract_json_string_field(line, "source").unwrap_or_default();
    if !matches!(
        source.as_str(),
        "current_queue" | "completion_resulting_queue"
    ) {
        failures.push("self_goal_queue_evidence_collection source is not supported".to_owned());
    }

    let ready = extract_json_bool_field(line, "ready").unwrap_or(false);
    let collection_complete = extract_json_bool_field(line, "collection_complete").unwrap_or(false);
    let planned_step_count = extract_json_usize_field(line, "planned_step_count").unwrap_or(0);
    let step_kinds = extract_json_string_array_field(line, "step_kinds").unwrap_or_default();
    let step_statuses = extract_json_string_array_field(line, "step_statuses").unwrap_or_default();
    let collected_evidence =
        extract_json_string_array_field(line, "collected_evidence_digests").unwrap_or_default();
    let collection_packets =
        extract_json_string_array_field(line, "collection_packet_digests").unwrap_or_default();
    let passed_steps = extract_json_usize_field(line, "passed_steps").unwrap_or(0);
    let failed_steps = extract_json_usize_field(line, "failed_steps").unwrap_or(0);
    let missing_steps = extract_json_usize_field(line, "missing_steps").unwrap_or(0);
    let manual_missing_steps = extract_json_usize_field(line, "manual_missing_steps").unwrap_or(0);
    let auto_collectible_steps =
        extract_json_usize_field(line, "auto_collectible_steps").unwrap_or(0);
    let manual_required_steps =
        extract_json_usize_field(line, "manual_required_steps").unwrap_or(0);
    let collected_evidence_count =
        extract_json_usize_field(line, "collected_evidence_count").unwrap_or(0);

    if planned_step_count != step_kinds.len()
        || planned_step_count != step_statuses.len()
        || planned_step_count != collection_packets.len()
    {
        failures.push("self_goal_queue_evidence_collection step count mismatch".to_owned());
    }
    if collected_evidence_count != collected_evidence.len() {
        failures.push("self_goal_queue_evidence_collection collected evidence mismatch".to_owned());
    }
    if passed_steps
        .saturating_add(failed_steps)
        .saturating_add(missing_steps)
        .saturating_add(manual_missing_steps)
        != planned_step_count
    {
        failures.push("self_goal_queue_evidence_collection status count mismatch".to_owned());
    }
    if auto_collectible_steps.saturating_add(manual_required_steps) != planned_step_count {
        failures.push("self_goal_queue_evidence_collection auto/manual count mismatch".to_owned());
    }
    if collected_evidence_count > passed_steps.saturating_add(failed_steps) {
        failures.push(
            "self_goal_queue_evidence_collection collected evidence exceeds terminal steps"
                .to_owned(),
        );
    }
    if ready && planned_step_count == 0 {
        failures.push("self_goal_queue_evidence_collection ready plan requires steps".to_owned());
    }
    if !ready && planned_step_count > 0 {
        failures.push(
            "self_goal_queue_evidence_collection held plan must not include steps".to_owned(),
        );
    }
    if collection_complete
        && (!ready
            || planned_step_count == 0
            || failed_steps > 0
            || missing_steps > 0
            || manual_missing_steps > 0
            || passed_steps != planned_step_count)
    {
        failures.push(
            "self_goal_queue_evidence_collection complete flag conflicts with step status"
                .to_owned(),
        );
    }
    if !collection_complete
        && ready
        && planned_step_count > 0
        && failed_steps == 0
        && missing_steps == 0
        && manual_missing_steps == 0
        && passed_steps == planned_step_count
    {
        failures.push(
            "self_goal_queue_evidence_collection complete plan must set collection_complete"
                .to_owned(),
        );
    }

    for evidence in &step_kinds {
        if !matches!(
            evidence.as_str(),
            "cargo_check"
                | "focused_tests"
                | "benchmark_gate"
                | "trace_schema_gate"
                | "experiment_ledger"
                | "operator_approval"
        ) {
            failures.push(format!(
                "self_goal_queue_evidence_collection evidence kind {evidence} is not supported"
            ));
        }
        if contains_private_or_executable_marker(evidence) {
            failures
                .push("self_goal_queue_evidence_collection evidence kind leaked marker".to_owned());
        }
    }
    for status in &step_statuses {
        if !matches!(
            status.as_str(),
            "passed" | "failed" | "missing" | "manual_missing"
        ) {
            failures.push(format!(
                "self_goal_queue_evidence_collection status {status} is not supported"
            ));
        }
    }

    let active_goal_id = extract_json_string_field(line, "active_goal_id").unwrap_or_default();
    if active_goal_id != "none" && !active_goal_id.starts_with("redaction-digest:") {
        failures.push(
            "self_goal_queue_evidence_collection active_goal_id must be redaction digest or none"
                .to_owned(),
        );
    }
    if ready && active_goal_id == "none" {
        failures.push("self_goal_queue_evidence_collection ready plan must name a goal".to_owned());
    }
    if !ready && active_goal_id != "none" {
        failures
            .push("self_goal_queue_evidence_collection held plan must not name a goal".to_owned());
    }

    let digest = extract_json_string_field(line, "evidence_collection_digest").unwrap_or_default();
    if !digest.starts_with("redaction-digest:") {
        failures.push(
            "self_goal_queue_evidence_collection evidence_collection_digest must be redaction digest"
                .to_owned(),
        );
    }
    if contains_private_or_executable_marker(&digest) {
        failures.push(
            "self_goal_queue_evidence_collection evidence_collection_digest leaked private marker"
                .to_owned(),
        );
    }
    for value in collected_evidence.iter().chain(collection_packets.iter()) {
        if !value.starts_with("redaction-digest:") {
            failures.push(
                "self_goal_queue_evidence_collection collection digest must be redaction digest"
                    .to_owned(),
            );
        }
        if contains_private_or_executable_marker(value) {
            failures.push("self_goal_queue_evidence_collection digest leaked marker".to_owned());
        }
    }

    let summary = extract_json_string_field(line, "summary").unwrap_or_default();
    if contains_private_or_executable_marker(&summary) {
        failures
            .push("self_goal_queue_evidence_collection summary leaked private marker".to_owned());
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
