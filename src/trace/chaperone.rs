use super::fields::{extract_json_bool_field, extract_json_string_field, extract_json_usize_field};

const REASONING_CHAPERONE_SCHEMA: &str = "rust-norion-reasoning-chaperone-fold-guard-v1";

pub(super) fn evaluate_reasoning_chaperone_fold_guard_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    if !line.contains(&format!("\"schema\":\"{REASONING_CHAPERONE_SCHEMA}\"")) {
        return failures;
    }

    let fold_status = require_string(&mut failures, line, "fold_status");
    let undefined_capability_count =
        require_usize(&mut failures, line, "undefined_capability_count");
    let contradiction_count = require_usize(&mut failures, line, "contradiction_count");
    let ungated_side_effect_count = require_usize(&mut failures, line, "ungated_side_effect_count");
    let missing_evidence_count = require_usize(&mut failures, line, "missing_evidence_count");
    let repair_task_count = require_usize(&mut failures, line, "repair_task_count");
    let raw_cot_captured = require_bool(&mut failures, line, "raw_cot_captured");
    let raw_prompt_captured = require_bool(&mut failures, line, "raw_prompt_captured");
    let service_execution_allowed = require_bool(&mut failures, line, "service_execution_allowed");
    let admission_allowed = require_bool(&mut failures, line, "admission_allowed");

    let Some(fold_status) = fold_status else {
        return failures;
    };
    if !matches!(fold_status.as_str(), "stable" | "watch" | "repair") {
        failures.push("reasoning_chaperone_fold_guard invalid fold_status".to_owned());
    }

    if raw_cot_captured.unwrap_or(false) {
        failures.push("reasoning_chaperone_fold_guard raw_cot_captured must be false".to_owned());
    }
    if raw_prompt_captured.unwrap_or(false) {
        failures
            .push("reasoning_chaperone_fold_guard raw_prompt_captured must be false".to_owned());
    }

    let blocking_count = undefined_capability_count.unwrap_or(0)
        + contradiction_count.unwrap_or(0)
        + ungated_side_effect_count.unwrap_or(0)
        + missing_evidence_count.unwrap_or(0)
        + usize::from(raw_cot_captured.unwrap_or(false))
        + usize::from(raw_prompt_captured.unwrap_or(false));
    let repair_task_count = repair_task_count.unwrap_or(0);

    if fold_status == "stable" && blocking_count > 0 {
        failures.push(
            "reasoning_chaperone_fold_guard stable status conflicts with blocking counts"
                .to_owned(),
        );
    }
    if fold_status == "repair" && blocking_count == 0 {
        failures.push(
            "reasoning_chaperone_fold_guard repair status requires a blocking count".to_owned(),
        );
    }
    if fold_status == "repair" && repair_task_count != 1 {
        failures.push(
            "reasoning_chaperone_fold_guard repair status requires exactly one repair task"
                .to_owned(),
        );
    }
    if fold_status != "repair" && repair_task_count != 0 {
        failures.push(
            "reasoning_chaperone_fold_guard non-repair status must not enqueue repair tasks"
                .to_owned(),
        );
    }
    if repair_task_count > 1 {
        failures.push("reasoning_chaperone_fold_guard repair_task_count exceeds one".to_owned());
    }
    if (undefined_capability_count.unwrap_or(0) > 0 || ungated_side_effect_count.unwrap_or(0) > 0)
        && service_execution_allowed.unwrap_or(false)
    {
        failures.push(
            "reasoning_chaperone_fold_guard service execution allowed despite service blockers"
                .to_owned(),
        );
    }
    if missing_evidence_count.unwrap_or(0) > 0 && admission_allowed.unwrap_or(false) {
        failures.push(
            "reasoning_chaperone_fold_guard admission allowed despite missing evidence".to_owned(),
        );
    }

    failures
}

fn require_string(failures: &mut Vec<String>, line: &str, field: &str) -> Option<String> {
    let value = extract_json_string_field(line, field);
    if value.is_none() {
        failures.push(format!("reasoning_chaperone_fold_guard missing {field}"));
    }
    value
}

fn require_usize(failures: &mut Vec<String>, line: &str, field: &str) -> Option<usize> {
    let value = extract_json_usize_field(line, field);
    if value.is_none() {
        failures.push(format!("reasoning_chaperone_fold_guard missing {field}"));
    }
    value
}

fn require_bool(failures: &mut Vec<String>, line: &str, field: &str) -> Option<bool> {
    let value = extract_json_bool_field(line, field);
    if value.is_none() {
        failures.push(format!("reasoning_chaperone_fold_guard missing {field}"));
    }
    value
}
