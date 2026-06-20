use std::fs;
use std::io;
use std::path::Path;

use super::evaluate_trace_schema_line;
use super::fields::{
    extract_json_bool_field, extract_json_string_array_field, extract_json_string_field,
    extract_json_usize_field, json_object_after_field, trace_note_bool,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceSchemaGateReport {
    pub passed: bool,
    pub checked_lines: usize,
    pub rust_check_events: usize,
    pub rust_check_passed: usize,
    pub rust_check_failed: usize,
    pub rust_check_feedback_updates: usize,
    pub rust_check_feedback_applied: usize,
    pub business_contract_events: usize,
    pub business_contract_event_passed: usize,
    pub business_contract_event_failed: usize,
    pub business_contract_event_missing_signals: usize,
    pub business_contract_event_protocol_leaks: usize,
    pub business_contract_event_substitutions: usize,
    pub business_contract_event_evasive_denials: usize,
    pub business_contract_event_raw_passed: usize,
    pub business_contract_event_raw_failed: usize,
    pub business_contract_event_response_normalized: usize,
    pub business_contract_event_sanitized: usize,
    pub business_contract_event_canonical_fallbacks: usize,
    pub runtime_error_events: usize,
    pub runtime_timeout_events: usize,
    pub failures: Vec<String>,
}

impl TraceSchemaGateReport {
    pub fn summary_line(&self) -> String {
        format!(
            "trace_schema_gate: passed={} lines={} failures={} rust_check_events={} rust_check_passed={} rust_check_failed={} rust_check_feedback_updates={} rust_check_feedback_applied={} business_contract_events={} business_contract_event_passed={} business_contract_event_failed={} business_contract_event_missing_signals={} business_contract_event_protocol_leaks={} business_contract_event_substitutions={} business_contract_event_evasive_denials={} business_contract_event_raw_passed={} business_contract_event_raw_failed={} business_contract_event_response_normalized={} business_contract_event_sanitized={} business_contract_event_canonical_fallbacks={} runtime_error_events={} runtime_timeout_events={}",
            self.passed,
            self.checked_lines,
            self.failures.len(),
            self.rust_check_events,
            self.rust_check_passed,
            self.rust_check_failed,
            self.rust_check_feedback_updates,
            self.rust_check_feedback_applied,
            self.business_contract_events,
            self.business_contract_event_passed,
            self.business_contract_event_failed,
            self.business_contract_event_missing_signals,
            self.business_contract_event_protocol_leaks,
            self.business_contract_event_substitutions,
            self.business_contract_event_evasive_denials,
            self.business_contract_event_raw_passed,
            self.business_contract_event_raw_failed,
            self.business_contract_event_response_normalized,
            self.business_contract_event_sanitized,
            self.business_contract_event_canonical_fallbacks,
            self.runtime_error_events,
            self.runtime_timeout_events
        )
    }
}

pub fn evaluate_trace_schema_jsonl(path: impl AsRef<Path>) -> io::Result<TraceSchemaGateReport> {
    let content = fs::read_to_string(path)?;
    let mut checked_lines = 0;
    let mut rust_check_events = 0;
    let mut rust_check_passed = 0;
    let mut rust_check_failed = 0;
    let mut rust_check_feedback_updates = 0;
    let mut rust_check_feedback_applied = 0;
    let mut business_contract_events = 0;
    let mut business_contract_event_passed = 0;
    let mut business_contract_event_failed = 0;
    let mut business_contract_event_missing_signals = 0;
    let mut business_contract_event_protocol_leaks = 0;
    let mut business_contract_event_substitutions = 0;
    let mut business_contract_event_evasive_denials = 0;
    let mut business_contract_event_raw_passed = 0;
    let mut business_contract_event_raw_failed = 0;
    let mut business_contract_event_response_normalized = 0;
    let mut business_contract_event_sanitized = 0;
    let mut business_contract_event_canonical_fallbacks = 0;
    let mut runtime_error_events = 0;
    let mut runtime_timeout_events = 0;
    let mut failures = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        checked_lines += 1;
        if let Some(summary) = rust_check_trace_gate_summary(line) {
            rust_check_events += summary.events;
            rust_check_passed += summary.passed;
            rust_check_failed += summary.failed;
            rust_check_feedback_updates += summary.feedback_updates;
            rust_check_feedback_applied += summary.feedback_applied;
        }
        if let Some(summary) = business_contract_trace_gate_summary(line) {
            business_contract_events += summary.events;
            business_contract_event_passed += summary.passed;
            business_contract_event_failed += summary.failed;
            business_contract_event_missing_signals += summary.missing_signals;
            business_contract_event_protocol_leaks += summary.protocol_leaks;
            business_contract_event_substitutions += summary.substitutions;
            business_contract_event_evasive_denials += summary.evasive_denials;
            business_contract_event_raw_passed += summary.raw_passed;
            business_contract_event_raw_failed += summary.raw_failed;
            business_contract_event_response_normalized += summary.response_normalized;
            business_contract_event_sanitized += summary.sanitized;
            business_contract_event_canonical_fallbacks += summary.canonical_fallbacks;
        }
        if let Some(summary) = runtime_error_trace_gate_summary(line) {
            runtime_error_events += summary.events;
            runtime_timeout_events += summary.timeouts;
        }
        failures.extend(
            evaluate_trace_schema_line(line)
                .into_iter()
                .map(|failure| format!("line {}: {failure}", index + 1)),
        );
    }

    if checked_lines == 0 {
        failures.push("trace file did not contain any non-empty JSONL records".to_owned());
    }

    Ok(TraceSchemaGateReport {
        passed: failures.is_empty(),
        checked_lines,
        rust_check_events,
        rust_check_passed,
        rust_check_failed,
        rust_check_feedback_updates,
        rust_check_feedback_applied,
        business_contract_events,
        business_contract_event_passed,
        business_contract_event_failed,
        business_contract_event_missing_signals,
        business_contract_event_protocol_leaks,
        business_contract_event_substitutions,
        business_contract_event_evasive_denials,
        business_contract_event_raw_passed,
        business_contract_event_raw_failed,
        business_contract_event_response_normalized,
        business_contract_event_sanitized,
        business_contract_event_canonical_fallbacks,
        runtime_error_events,
        runtime_timeout_events,
        failures,
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct RustCheckTraceGateSummary {
    events: usize,
    passed: usize,
    failed: usize,
    feedback_updates: usize,
    feedback_applied: usize,
}

fn rust_check_trace_gate_summary(line: &str) -> Option<RustCheckTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-rust-check-v1\"") {
        return None;
    }

    let mut summary = RustCheckTraceGateSummary {
        events: 1,
        ..RustCheckTraceGateSummary::default()
    };

    if let Some(rust_check) = json_object_after_field(line, "rust_check") {
        match extract_json_bool_field(rust_check, "passed") {
            Some(true) => summary.passed = 1,
            Some(false) => summary.failed = 1,
            None => {}
        }
    }
    if let Some(feedback) = json_object_after_field(line, "feedback") {
        let applied = extract_json_usize_field(feedback, "applied").unwrap_or(0);
        let missing = extract_json_usize_field(feedback, "missing").unwrap_or(0);
        summary.feedback_updates = applied.saturating_add(missing);
        summary.feedback_applied = applied;
    }

    Some(summary)
}

#[derive(Debug, Clone, Copy, Default)]
struct BusinessContractTraceGateSummary {
    events: usize,
    passed: usize,
    failed: usize,
    missing_signals: usize,
    protocol_leaks: usize,
    substitutions: usize,
    evasive_denials: usize,
    raw_passed: usize,
    raw_failed: usize,
    response_normalized: usize,
    sanitized: usize,
    canonical_fallbacks: usize,
}

fn business_contract_trace_gate_summary(line: &str) -> Option<BusinessContractTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-business-contract-v1\"") {
        return None;
    }

    let mut summary = BusinessContractTraceGateSummary {
        events: 1,
        ..BusinessContractTraceGateSummary::default()
    };
    let business_contract = json_object_after_field(line, "business_contract")?;

    match extract_json_bool_field(business_contract, "passed") {
        Some(true) => summary.passed = 1,
        Some(false) => summary.failed = 1,
        None => {}
    }
    summary.missing_signals =
        extract_json_usize_field(business_contract, "missing_signal_count").unwrap_or(0);
    summary.protocol_leaks =
        usize::from(extract_json_bool_field(business_contract, "protocol_leak").unwrap_or(false));
    summary.substitutions = usize::from(
        extract_json_bool_field(business_contract, "substituted_runtime_model_experiences")
            .unwrap_or(false),
    );
    summary.evasive_denials =
        usize::from(extract_json_bool_field(business_contract, "evasive_denial").unwrap_or(false));
    match extract_json_bool_field(business_contract, "raw_passed") {
        Some(true) => summary.raw_passed = 1,
        Some(false) => summary.raw_failed = 1,
        None => {}
    }
    summary.response_normalized = usize::from(
        extract_json_bool_field(business_contract, "response_normalized").unwrap_or(false),
    );
    let normalization =
        extract_json_string_field(business_contract, "normalization").unwrap_or_default();
    summary.sanitized = usize::from(normalization == "sanitized");
    summary.canonical_fallbacks = usize::from(
        extract_json_bool_field(business_contract, "canonical_fallback").unwrap_or(false),
    );

    Some(summary)
}

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeErrorTraceGateSummary {
    events: usize,
    timeouts: usize,
}

fn runtime_error_trace_gate_summary(line: &str) -> Option<RuntimeErrorTraceGateSummary> {
    if line.contains("\"schema\":\"rust-norion-rust-check-v1\"") {
        return None;
    }
    let process_reward = json_object_after_field(line, "process_reward")?;
    let notes = extract_json_string_array_field(process_reward, "notes").unwrap_or_default();
    let mut summary = RuntimeErrorTraceGateSummary::default();
    for note in notes
        .iter()
        .filter(|note| note.starts_with("runtime_error:"))
    {
        summary.events = summary.events.saturating_add(1);
        if trace_note_bool(note, "timeout=").unwrap_or(false) {
            summary.timeouts = summary.timeouts.saturating_add(1);
        }
    }

    (summary.events > 0).then_some(summary)
}
