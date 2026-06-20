use super::TRACE_FLOAT_EPSILON;
use super::fields::*;
use super::required_fields::{
    BUSINESS_CONTRACT_TRACE_REQUIRED_FIELDS, RUST_CHECK_TRACE_REQUIRED_FIELDS,
};
pub(super) fn evaluate_business_contract_trace_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for field in BUSINESS_CONTRACT_TRACE_REQUIRED_FIELDS {
        if !line.contains(field.marker) {
            failures.push(format!(
                "missing business_contract trace field {}",
                field.name
            ));
        }
    }

    if extract_json_nullable_u64_field(line, "experience_id").is_none() {
        failures.push("business_contract experience_id is missing or null".to_owned());
    }

    let Some(contract) = json_object_after_field(line, "business_contract") else {
        failures.push("business_contract object is missing or invalid".to_owned());
        return failures;
    };

    let passed = extract_json_bool_field(contract, "passed");
    let required_signals = extract_json_usize_field(contract, "required_signals");
    let matched_signals = extract_json_usize_field(contract, "matched_signals");
    let missing_signal_count = extract_json_usize_field(contract, "missing_signal_count");
    let missing_signals =
        extract_json_string_array_field(contract, "missing_signals").unwrap_or_default();
    let has_runtime_model_experiences =
        extract_json_bool_field(contract, "has_runtime_model_experiences");
    let protocol_leak = extract_json_bool_field(contract, "protocol_leak");
    let substituted = extract_json_bool_field(contract, "substituted_runtime_model_experiences");
    let evasive_denial = extract_json_bool_field(contract, "evasive_denial");
    let handling_signal = extract_json_bool_field(contract, "handling_signal");
    let raw_passed = extract_json_bool_field(contract, "raw_passed");
    let normalization = extract_json_string_field(contract, "normalization");
    let response_normalized = extract_json_bool_field(contract, "response_normalized");
    let canonical_fallback = extract_json_bool_field(contract, "canonical_fallback");

    if missing_signal_count != Some(missing_signals.len()) {
        failures.push(format!(
            "business_contract missing_signal_count {:?} does not match missing_signals {}",
            missing_signal_count,
            missing_signals.len()
        ));
    }
    if let (Some(required), Some(matched), Some(missing)) =
        (required_signals, matched_signals, missing_signal_count)
    {
        if matched > required {
            failures.push(format!(
                "business_contract matched_signals {matched} exceeds required_signals {required}"
            ));
        }
        if missing > required {
            failures.push(format!(
                "business_contract missing_signal_count {missing} exceeds required_signals {required}"
            ));
        }
        if matched.saturating_add(missing) != required {
            failures.push(format!(
                "business_contract matched+missing {} does not equal required_signals {required}",
                matched.saturating_add(missing)
            ));
        }
    }

    match passed {
        Some(true) => {
            if missing_signal_count.unwrap_or(0) > 0 {
                failures
                    .push("business_contract passed=true requires no missing signals".to_owned());
            }
            if has_runtime_model_experiences != Some(true) {
                failures.push(
                    "business_contract passed=true requires runtime_model_experiences".to_owned(),
                );
            }
            if protocol_leak == Some(true) {
                failures.push("business_contract passed=true forbids protocol leak".to_owned());
            }
            if substituted == Some(true) {
                failures.push(
                    "business_contract passed=true forbids runtime_model_experiences substitution"
                        .to_owned(),
                );
            }
            if evasive_denial == Some(true) {
                failures.push("business_contract passed=true forbids evasive denial".to_owned());
            }
            if handling_signal != Some(true) {
                failures.push("business_contract passed=true requires handling signal".to_owned());
            }
        }
        Some(false) => {
            let has_failure_reason = missing_signal_count.unwrap_or(0) > 0
                || has_runtime_model_experiences == Some(false)
                || protocol_leak == Some(true)
                || substituted == Some(true)
                || evasive_denial == Some(true)
                || handling_signal == Some(false);
            if !has_failure_reason {
                failures.push(
                    "business_contract passed=false requires at least one failed audit signal"
                        .to_owned(),
                );
            }
        }
        None => failures.push("business_contract passed flag is missing".to_owned()),
    }
    match normalization.as_deref() {
        Some("raw_direct" | "sanitized" | "canonical_fallback") => {}
        Some(other) => failures.push(format!(
            "business_contract normalization {other:?} is not a known source"
        )),
        None => failures.push("business_contract normalization is missing".to_owned()),
    }
    if raw_passed.is_none() {
        failures.push("business_contract raw_passed flag is missing".to_owned());
    }
    if response_normalized.is_none() {
        failures.push("business_contract response_normalized flag is missing".to_owned());
    }
    if canonical_fallback.is_none() {
        failures.push("business_contract canonical_fallback flag is missing".to_owned());
    }
    if response_normalized == Some(true)
        && !matches!(
            normalization.as_deref(),
            Some("sanitized" | "canonical_fallback")
        )
    {
        failures.push(
            "business_contract response_normalized=true requires sanitized or canonical_fallback"
                .to_owned(),
        );
    }
    if response_normalized == Some(false)
        && matches!(
            normalization.as_deref(),
            Some("sanitized" | "canonical_fallback")
        )
    {
        failures.push(
            "business_contract response_normalized=false conflicts with normalization source"
                .to_owned(),
        );
    }
    if canonical_fallback == Some(true) && normalization.as_deref() != Some("canonical_fallback") {
        failures.push(
            "business_contract canonical_fallback=true requires normalization=canonical_fallback"
                .to_owned(),
        );
    }
    if canonical_fallback == Some(false) && normalization.as_deref() == Some("canonical_fallback") {
        failures.push(
            "business_contract canonical_fallback=false conflicts with normalization=canonical_fallback"
                .to_owned(),
        );
    }

    failures
}

pub(super) fn evaluate_rust_check_trace_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for field in RUST_CHECK_TRACE_REQUIRED_FIELDS {
        if !line.contains(field.marker) {
            failures.push(format!("missing rust_check trace field {}", field.name));
        }
    }

    let Some(rust_check) = json_object_after_field(line, "rust_check") else {
        failures.push("rust_check object is missing or invalid".to_owned());
        return failures;
    };
    let Some(feedback) = json_object_after_field(line, "feedback") else {
        failures.push("rust_check feedback object is missing or invalid".to_owned());
        return failures;
    };

    let passed = extract_json_bool_field(rust_check, "passed");
    let label = extract_json_string_field(rust_check, "label").unwrap_or_default();
    let diagnostic_chars = extract_json_usize_field(rust_check, "diagnostic_chars").unwrap_or(0);
    let status_code_present = value_after_json_field(rust_check, "status_code").is_some();
    let action = extract_json_string_field(feedback, "action").unwrap_or_default();
    let amount = extract_json_f32_field(feedback, "amount").unwrap_or(f32::NAN);
    let memory_ids = extract_json_u64_array_field(feedback, "memory_ids").unwrap_or_default();
    let applied = extract_json_usize_field(feedback, "applied").unwrap_or(0);
    let missing = extract_json_usize_field(feedback, "missing").unwrap_or(0);
    let removed = extract_json_usize_field(feedback, "removed").unwrap_or(0);
    let strength_delta = extract_json_f32_field(feedback, "strength_delta").unwrap_or(f32::NAN);

    match passed {
        Some(true) if label != "rustc_passed" => failures.push(format!(
            "rust_check passed=true requires label rustc_passed, got {label}"
        )),
        Some(false) if label != "rustc_failed" => failures.push(format!(
            "rust_check passed=false requires label rustc_failed, got {label}"
        )),
        None => failures.push("rust_check passed must be boolean".to_owned()),
        _ => {}
    }
    if !status_code_present {
        failures.push("rust_check status_code must be present, even when null".to_owned());
    }
    if diagnostic_chars == 0 && passed == Some(false) {
        failures.push("rust_check failed checks must carry diagnostics".to_owned());
    }
    match action.as_str() {
        "reinforce" if passed == Some(false) => {
            failures.push("rust_check failed checks must not reinforce feedback".to_owned())
        }
        "penalize" if passed == Some(true) => {
            failures.push("rust_check passed checks must not penalize feedback".to_owned())
        }
        "reinforce" | "penalize" => {}
        _ => failures.push(format!("rust_check feedback action {action} is invalid")),
    }
    if !amount.is_finite() || amount <= 0.0 {
        failures.push(format!(
            "rust_check feedback amount {amount:.6} must be positive"
        ));
    }
    if applied.saturating_add(missing) != memory_ids.len() {
        failures.push(format!(
            "rust_check feedback applied+missing {} does not match memory_ids {}",
            applied.saturating_add(missing),
            memory_ids.len()
        ));
    }
    if removed > applied {
        failures.push(format!(
            "rust_check feedback removed {removed} exceeds applied {applied}"
        ));
    }
    if !strength_delta.is_finite() || strength_delta < -TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "rust_check feedback strength_delta {strength_delta:.6} must be finite and non-negative"
        ));
    }

    failures
}
