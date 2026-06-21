use super::fields::{
    extract_json_bool_field, extract_json_string_array_field, extract_json_string_field,
    extract_json_usize_field, extract_last_json_string_array_field, json_object_after_field,
};

pub(super) fn evaluate_self_evolution_admission_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-evolution-admission-v1\"",
        ),
        ("candidate_id", "\"candidate_id\":"),
        ("read_only", "\"read_only\":"),
        ("report_only", "\"report_only\":"),
        ("preview_only", "\"preview_only\":"),
        ("policy_valid", "\"policy_valid\":"),
        (
            "admitted_for_human_review",
            "\"admitted_for_human_review\":",
        ),
        ("human_approval_required", "\"human_approval_required\":"),
        ("review_packet", "\"review_packet\":"),
        ("rust_check", "\"rust_check\":"),
        ("benchmark_gate", "\"benchmark_gate\":"),
        ("rollback", "\"rollback\":"),
        ("adaptive_preview", "\"adaptive_preview\":"),
        ("writes", "\"writes\":"),
        ("blocked_reasons", "\"blocked_reasons\":"),
        ("telemetry", "\"telemetry\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing self_evolution_admission field {name}"));
        }
    }

    let candidate_id = extract_json_string_field(line, "candidate_id").unwrap_or_default();
    if candidate_id.trim().is_empty() {
        failures.push("self_evolution_admission candidate_id is empty".to_owned());
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_evolution_admission",
    );
    require_bool(
        &mut failures,
        line,
        "report_only",
        true,
        "self_evolution_admission",
    );
    require_bool(
        &mut failures,
        line,
        "preview_only",
        true,
        "self_evolution_admission",
    );
    require_bool(
        &mut failures,
        line,
        "human_approval_required",
        true,
        "self_evolution_admission",
    );
    require_bool(
        &mut failures,
        line,
        "policy_valid",
        true,
        "self_evolution_admission",
    );

    let admitted_for_human_review = extract_json_bool_field(line, "admitted_for_human_review");
    let blocked_reasons =
        extract_last_json_string_array_field(line, "blocked_reasons").unwrap_or_default();
    match admitted_for_human_review {
        Some(true) if !blocked_reasons.is_empty() => failures.push(
            "self_evolution_admission admitted review packet must not have blocked reasons"
                .to_owned(),
        ),
        Some(false) if blocked_reasons.is_empty() => failures.push(
            "self_evolution_admission blocked review packet requires blocked reasons".to_owned(),
        ),
        Some(_) => {}
        None => failures
            .push("self_evolution_admission admitted_for_human_review must be boolean".to_owned()),
    }

    evaluate_rust_check(&mut failures, line);
    evaluate_benchmark_gate(&mut failures, line);
    evaluate_review_packet(&mut failures, line, admitted_for_human_review);
    evaluate_rollback(&mut failures, line);
    evaluate_adaptive_preview(&mut failures, line, admitted_for_human_review);
    evaluate_writes(&mut failures, line);
    evaluate_telemetry(&mut failures, line);

    failures
}

fn evaluate_review_packet(
    failures: &mut Vec<String>,
    line: &str,
    admitted_for_human_review: Option<bool>,
) {
    let Some(review_packet) = json_object_after_field(line, "review_packet") else {
        failures.push("self_evolution_admission review_packet object is missing".to_owned());
        return;
    };

    require_bool(
        failures,
        review_packet,
        "read_only",
        true,
        "self_evolution_admission review_packet",
    );
    require_bool(
        failures,
        review_packet,
        "approval_tokens_included",
        false,
        "self_evolution_admission review_packet",
    );

    let approval_review_packet_ids =
        require_string_array(failures, review_packet, "approval_review_packet_ids");
    let evidence_ids = require_string_array(failures, review_packet, "evidence_ids");
    let rollback_anchor_ids = require_string_array(failures, review_packet, "rollback_anchor_ids");
    let content_digests = require_string_array(failures, review_packet, "content_digests");
    let source_report_schemas =
        require_string_array(failures, review_packet, "source_report_schemas");

    require_count(
        failures,
        review_packet,
        "approval_review_packet_count",
        approval_review_packet_ids.len(),
    );
    require_count(
        failures,
        review_packet,
        "evidence_count",
        evidence_ids.len(),
    );
    require_count(
        failures,
        review_packet,
        "rollback_anchor_count",
        rollback_anchor_ids.len(),
    );
    require_count(
        failures,
        review_packet,
        "content_digest_count",
        content_digests.len(),
    );
    require_count(
        failures,
        review_packet,
        "source_report_schema_count",
        source_report_schemas.len(),
    );

    if admitted_for_human_review == Some(true) {
        if approval_review_packet_ids.is_empty() {
            failures.push(
                "self_evolution_admission admitted packet requires review packet ids".to_owned(),
            );
        }
        if evidence_ids.is_empty() {
            failures.push(
                "self_evolution_admission admitted packet requires review evidence ids".to_owned(),
            );
        }
        if content_digests.is_empty() {
            failures.push(
                "self_evolution_admission admitted packet requires review content digests"
                    .to_owned(),
            );
        }
        if source_report_schemas.is_empty() {
            failures.push(
                "self_evolution_admission admitted packet requires review source schemas"
                    .to_owned(),
            );
        }
    }
}

fn evaluate_rust_check(failures: &mut Vec<String>, line: &str) {
    let Some(rust_check) = json_object_after_field(line, "rust_check") else {
        failures.push("self_evolution_admission rust_check object is missing".to_owned());
        return;
    };
    let items = extract_json_usize_field(rust_check, "items");
    let passed = extract_json_usize_field(rust_check, "passed");
    let failed = extract_json_usize_field(rust_check, "failed");
    let validation_passed = extract_json_bool_field(rust_check, "validation_passed");

    if items.is_none() || passed.is_none() || failed.is_none() || validation_passed.is_none() {
        failures.push("self_evolution_admission rust_check fields are incomplete".to_owned());
    }
    if let (Some(items), Some(passed), Some(failed)) = (items, passed, failed) {
        if passed.saturating_add(failed) > items {
            failures.push(format!(
                "self_evolution_admission rust_check passed+failed {} exceeds items {items}",
                passed.saturating_add(failed)
            ));
        }
        if validation_passed == Some(true) && (items == 0 || passed == 0 || failed > 0) {
            failures.push(
                "self_evolution_admission rust_validation_passed requires passed checks and no failures"
                    .to_owned(),
            );
        }
    }
}

fn evaluate_benchmark_gate(failures: &mut Vec<String>, line: &str) {
    let Some(benchmark_gate) = json_object_after_field(line, "benchmark_gate") else {
        failures.push("self_evolution_admission benchmark_gate object is missing".to_owned());
        return;
    };
    let passed = extract_json_bool_field(benchmark_gate, "passed");
    let failures_array =
        extract_json_string_array_field(benchmark_gate, "failures").unwrap_or_default();
    match passed {
        Some(true) if !failures_array.is_empty() => failures.push(
            "self_evolution_admission benchmark_gate passed=true must not include failures"
                .to_owned(),
        ),
        Some(false) if failures_array.is_empty() => failures.push(
            "self_evolution_admission benchmark_gate passed=false requires failures".to_owned(),
        ),
        Some(_) => {}
        None => {
            failures.push("self_evolution_admission benchmark_gate passed is missing".to_owned())
        }
    }
}

fn evaluate_rollback(failures: &mut Vec<String>, line: &str) {
    let Some(rollback) = json_object_after_field(line, "rollback") else {
        failures.push("self_evolution_admission rollback object is missing".to_owned());
        return;
    };
    if extract_json_bool_field(rollback, "budget_clean").is_none() {
        failures.push("self_evolution_admission rollback budget_clean is missing".to_owned());
    }
    if extract_json_usize_field(rollback, "drift_rollbacks").is_none() {
        failures.push("self_evolution_admission rollback drift_rollbacks is missing".to_owned());
    }
}

fn evaluate_adaptive_preview(
    failures: &mut Vec<String>,
    line: &str,
    admitted_for_human_review: Option<bool>,
) {
    let Some(adaptive_preview) = json_object_after_field(line, "adaptive_preview") else {
        failures.push("self_evolution_admission adaptive_preview object is missing".to_owned());
        return;
    };
    let read_only = require_bool_value(
        failures,
        adaptive_preview,
        "read_only",
        "self_evolution_admission adaptive_preview",
    );
    let report_only = require_bool_value(
        failures,
        adaptive_preview,
        "report_only",
        "self_evolution_admission adaptive_preview",
    );
    let preview_only = require_bool_value(
        failures,
        adaptive_preview,
        "preview_only",
        "self_evolution_admission adaptive_preview",
    );
    let write_allowed = require_bool_value(
        failures,
        adaptive_preview,
        "write_allowed",
        "self_evolution_admission adaptive_preview",
    );
    let applied = require_bool_value(
        failures,
        adaptive_preview,
        "applied",
        "self_evolution_admission adaptive_preview",
    );

    let evidence_present = extract_json_bool_field(adaptive_preview, "evidence_present");
    let source_count = extract_json_usize_field(adaptive_preview, "source_count").unwrap_or(0);
    if evidence_present == Some(true) && source_count == 0 {
        failures.push(
            "self_evolution_admission adaptive_preview evidence requires source_count".to_owned(),
        );
    }
    if admitted_for_human_review == Some(true) && evidence_present != Some(true) {
        failures.push(
            "self_evolution_admission admitted packet requires adaptive preview evidence"
                .to_owned(),
        );
    }
    if admitted_for_human_review == Some(true) {
        if read_only != Some(true) {
            failures.push(
                "self_evolution_admission admitted packet requires adaptive_preview read_only=true"
                    .to_owned(),
            );
        }
        if report_only != Some(true) {
            failures.push(
                "self_evolution_admission admitted packet requires adaptive_preview report_only=true"
                    .to_owned(),
            );
        }
        if preview_only != Some(true) {
            failures.push(
                "self_evolution_admission admitted packet requires adaptive_preview preview_only=true"
                    .to_owned(),
            );
        }
        if write_allowed != Some(false) {
            failures.push(
                "self_evolution_admission admitted packet requires adaptive_preview write_allowed=false"
                    .to_owned(),
            );
        }
        if applied != Some(false) {
            failures.push(
                "self_evolution_admission admitted packet requires adaptive_preview applied=false"
                    .to_owned(),
            );
        }
    }
}

fn evaluate_writes(failures: &mut Vec<String>, line: &str) {
    let Some(writes) = json_object_after_field(line, "writes") else {
        failures.push("self_evolution_admission writes object is missing".to_owned());
        return;
    };
    for field in [
        "mutation_allowed",
        "memory_store_allowed",
        "ndkv_allowed",
        "model_weight_allowed",
        "git_allowed",
    ] {
        require_bool(
            failures,
            writes,
            field,
            false,
            "self_evolution_admission writes",
        );
    }
}

fn evaluate_telemetry(failures: &mut Vec<String>, line: &str) {
    let telemetry = extract_json_string_array_field(line, "telemetry").unwrap_or_default();
    if !telemetry
        .iter()
        .any(|entry| entry == "self_evolution_admission=true")
    {
        failures.push(
            "self_evolution_admission telemetry must include self_evolution_admission=true"
                .to_owned(),
        );
    }
}

fn require_bool(
    failures: &mut Vec<String>,
    object: &str,
    field: &str,
    expected: bool,
    context: &str,
) {
    match extract_json_bool_field(object, field) {
        Some(actual) if actual == expected => {}
        Some(actual) => failures.push(format!(
            "{context} {field}={actual} does not match required {expected}"
        )),
        None => failures.push(format!("{context} {field} is missing")),
    }
}

fn require_bool_value(
    failures: &mut Vec<String>,
    object: &str,
    field: &str,
    context: &str,
) -> Option<bool> {
    match extract_json_bool_field(object, field) {
        Some(value) => Some(value),
        None => {
            failures.push(format!("{context} {field} is missing"));
            None
        }
    }
}

fn require_string_array(failures: &mut Vec<String>, object: &str, field: &str) -> Vec<String> {
    match extract_json_string_array_field(object, field) {
        Some(items) => {
            if items.iter().any(|item| item.trim().is_empty()) {
                failures.push(format!(
                    "self_evolution_admission review_packet {field} contains empty item"
                ));
            }
            items
        }
        None => {
            failures.push(format!(
                "self_evolution_admission review_packet {field} is missing"
            ));
            Vec::new()
        }
    }
}

fn require_count(failures: &mut Vec<String>, object: &str, field: &str, expected: usize) {
    match extract_json_usize_field(object, field) {
        Some(actual) if actual == expected => {}
        Some(actual) => failures.push(format!(
            "self_evolution_admission review_packet {field}={actual} does not match array length {expected}"
        )),
        None => failures.push(format!(
            "self_evolution_admission review_packet {field} is missing"
        )),
    }
}
