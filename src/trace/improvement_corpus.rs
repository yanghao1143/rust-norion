use super::fields::{
    extract_json_bool_field, extract_json_string_array_field, extract_json_usize_field,
    json_object_after_field,
};

pub(super) fn evaluate_improvement_corpus_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        ("schema", "\"schema\":\"rust-norion-improvement-corpus-v1\""),
        ("corpus_id", "\"corpus_id\":"),
        ("read_only", "\"read_only\":"),
        ("report_only", "\"report_only\":"),
        ("preview_only", "\"preview_only\":"),
        ("dataset_export_enabled", "\"dataset_export_enabled\":"),
        ("records", "\"records\":"),
        ("active_adaptation", "\"active_adaptation\":"),
        ("approval", "\"approval\":"),
        ("validation", "\"validation\":"),
        ("evidence", "\"evidence\":"),
        ("rollback", "\"rollback\":"),
        ("privacy", "\"privacy\":"),
        ("record_summaries", "\"record_summaries\":"),
        ("blocked_reasons", "\"blocked_reasons\":"),
        ("telemetry", "\"telemetry\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing improvement_corpus field {name}"));
        }
    }

    require_bool(&mut failures, line, "read_only", true, "improvement_corpus");
    require_bool(
        &mut failures,
        line,
        "report_only",
        true,
        "improvement_corpus",
    );
    require_bool(
        &mut failures,
        line,
        "preview_only",
        true,
        "improvement_corpus",
    );
    require_bool(
        &mut failures,
        line,
        "dataset_export_enabled",
        false,
        "improvement_corpus",
    );

    let total = evaluate_records(&mut failures, line);
    let active = evaluate_active_adaptation(&mut failures, line);
    let approved = evaluate_approval(&mut failures, line);
    let validation_passed = evaluate_validation(&mut failures, line);
    evaluate_evidence(&mut failures, line);
    let rollback_replayed = evaluate_rollback(&mut failures, line);
    evaluate_privacy(&mut failures, line);
    evaluate_record_summaries(&mut failures, line, total);
    evaluate_telemetry(&mut failures, line);

    if let (Some(active), Some(approved)) = (active, approved) {
        if active > approved {
            failures.push(format!(
                "improvement_corpus active_adaptation eligible {active} exceeds approved {approved}"
            ));
        }
    }
    if let (Some(active), Some(validation_passed)) = (active, validation_passed) {
        if active > validation_passed {
            failures.push(format!(
                "improvement_corpus active_adaptation eligible {active} exceeds validation_passed {validation_passed}"
            ));
        }
    }
    if let (Some(active), Some(rollback_replayed)) = (active, rollback_replayed) {
        if active > rollback_replayed {
            failures.push(format!(
                "improvement_corpus active_adaptation eligible {active} exceeds rollback_replayed {rollback_replayed}"
            ));
        }
    }

    failures
}

fn evaluate_records(failures: &mut Vec<String>, line: &str) -> Option<usize> {
    let Some(records) = json_object_after_field(line, "records") else {
        failures.push("improvement_corpus records object is missing".to_owned());
        return None;
    };
    let total = extract_json_usize_field(records, "total");
    let accepted = extract_json_usize_field(records, "accepted").unwrap_or(0);
    let failed = extract_json_usize_field(records, "failed").unwrap_or(0);
    let flaky = extract_json_usize_field(records, "flaky").unwrap_or(0);
    let privacy_blocked = extract_json_usize_field(records, "privacy_blocked").unwrap_or(0);
    let research_only = extract_json_usize_field(records, "research_only").unwrap_or(0);

    let Some(total) = total else {
        failures.push("improvement_corpus records total is missing".to_owned());
        return None;
    };
    let classified = accepted
        .saturating_add(failed)
        .saturating_add(flaky)
        .saturating_add(privacy_blocked)
        .saturating_add(research_only);
    if classified != total {
        failures.push(format!(
            "improvement_corpus classified records {classified} does not match total {total}"
        ));
    }
    Some(total)
}

fn evaluate_active_adaptation(failures: &mut Vec<String>, line: &str) -> Option<usize> {
    let Some(active) = json_object_after_field(line, "active_adaptation") else {
        failures.push("improvement_corpus active_adaptation object is missing".to_owned());
        return None;
    };
    let eligible = extract_json_usize_field(active, "eligible");
    let blocked = extract_json_usize_field(active, "blocked");
    if eligible.is_none() || blocked.is_none() {
        failures.push("improvement_corpus active_adaptation fields are incomplete".to_owned());
    }
    eligible
}

fn evaluate_approval(failures: &mut Vec<String>, line: &str) -> Option<usize> {
    let Some(approval) = json_object_after_field(line, "approval") else {
        failures.push("improvement_corpus approval object is missing".to_owned());
        return None;
    };
    let approved = extract_json_usize_field(approval, "approved");
    if approved.is_none()
        || extract_json_usize_field(approval, "pending").is_none()
        || extract_json_usize_field(approval, "rejected").is_none()
    {
        failures.push("improvement_corpus approval fields are incomplete".to_owned());
    }
    approved
}

fn evaluate_validation(failures: &mut Vec<String>, line: &str) -> Option<usize> {
    let Some(validation) = json_object_after_field(line, "validation") else {
        failures.push("improvement_corpus validation object is missing".to_owned());
        return None;
    };
    let passed = extract_json_usize_field(validation, "passed");
    if passed.is_none()
        || extract_json_usize_field(validation, "pending").is_none()
        || extract_json_usize_field(validation, "failed").is_none()
        || extract_json_usize_field(validation, "flaky").is_none()
    {
        failures.push("improvement_corpus validation fields are incomplete".to_owned());
    }
    passed
}

fn evaluate_evidence(failures: &mut Vec<String>, line: &str) {
    let Some(evidence) = json_object_after_field(line, "evidence") else {
        failures.push("improvement_corpus evidence object is missing".to_owned());
        return;
    };

    for field in [
        "compiler_items",
        "compiler_passed",
        "compiler_failed",
        "compiler_flaky",
        "test_items",
        "test_passed",
        "test_failed",
        "test_flaky",
        "benchmark_items",
        "benchmark_passed",
        "benchmark_failed",
        "benchmark_flaky",
        "source_trace_ids",
        "evidence_ids",
    ] {
        if extract_json_usize_field(evidence, field).is_none() {
            failures.push(format!("improvement_corpus evidence {field} is missing"));
        }
    }

    check_lane_counts(failures, evidence, "compiler");
    check_lane_counts(failures, evidence, "test");
    check_lane_counts(failures, evidence, "benchmark");
}

fn check_lane_counts(failures: &mut Vec<String>, evidence: &str, lane: &str) {
    let items = extract_json_usize_field(evidence, &format!("{lane}_items"));
    let passed = extract_json_usize_field(evidence, &format!("{lane}_passed")).unwrap_or(0);
    let failed = extract_json_usize_field(evidence, &format!("{lane}_failed")).unwrap_or(0);
    let flaky = extract_json_usize_field(evidence, &format!("{lane}_flaky")).unwrap_or(0);

    if let Some(items) = items {
        let observed = passed.saturating_add(failed).saturating_add(flaky);
        if observed > items {
            failures.push(format!(
                "improvement_corpus evidence {lane} passed+failed+flaky {observed} exceeds items {items}"
            ));
        }
    }
}

fn evaluate_rollback(failures: &mut Vec<String>, line: &str) -> Option<usize> {
    let Some(rollback) = json_object_after_field(line, "rollback") else {
        failures.push("improvement_corpus rollback object is missing".to_owned());
        return None;
    };
    if extract_json_usize_field(rollback, "anchors").is_none() {
        failures.push("improvement_corpus rollback anchors is missing".to_owned());
    }
    let replayed = extract_json_usize_field(rollback, "replayed");
    if replayed.is_none() {
        failures.push("improvement_corpus rollback replayed is missing".to_owned());
    }
    replayed
}

fn evaluate_privacy(failures: &mut Vec<String>, line: &str) {
    let Some(privacy) = json_object_after_field(line, "privacy") else {
        failures.push("improvement_corpus privacy object is missing".to_owned());
        return;
    };
    for field in [
        "rejected",
        "redactions",
        "raw_prompt_payloads_stored",
        "raw_response_payloads_stored",
        "secret_leaks",
    ] {
        if extract_json_usize_field(privacy, field).is_none() {
            failures.push(format!("improvement_corpus privacy {field} is missing"));
        }
    }
    for field in [
        "raw_prompt_payloads_stored",
        "raw_response_payloads_stored",
        "secret_leaks",
    ] {
        if extract_json_usize_field(privacy, field).unwrap_or(0) > 0 {
            failures.push(format!("improvement_corpus privacy {field} must be 0"));
        }
    }
}

fn evaluate_record_summaries(failures: &mut Vec<String>, line: &str, total: Option<usize>) {
    let summaries = extract_json_string_array_field(line, "record_summaries").unwrap_or_default();
    if let Some(total) = total {
        if summaries.len() != total {
            failures.push(format!(
                "improvement_corpus record_summaries {} does not match total {total}",
                summaries.len()
            ));
        }
    }
    if summaries
        .iter()
        .any(|summary| contains_sensitive_payload(summary))
    {
        failures.push("improvement_corpus record_summaries contain sensitive payload".to_owned());
    }
}

fn evaluate_telemetry(failures: &mut Vec<String>, line: &str) {
    let telemetry = extract_json_string_array_field(line, "telemetry").unwrap_or_default();
    if !telemetry
        .iter()
        .any(|entry| entry == "improvement_corpus=true")
    {
        failures
            .push("improvement_corpus telemetry must include improvement_corpus=true".to_owned());
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

fn contains_sensitive_payload(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "secret",
        "password",
        "passwd",
        "token=",
        "private:",
        "private_key",
        "begin private key",
        "sk-",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}
