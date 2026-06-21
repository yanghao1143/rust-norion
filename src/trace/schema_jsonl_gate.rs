use std::fs;
use std::io;
use std::path::Path;

use super::evaluate_trace_schema_line;
use super::fields::{
    extract_json_bool_field, extract_json_string_array_field, extract_json_string_field,
    extract_json_usize_field, extract_last_json_string_array_field, json_object_after_field,
    trace_note_bool,
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
    pub self_evolution_admission_events: usize,
    pub self_evolution_admission_admitted: usize,
    pub self_evolution_admission_blocked: usize,
    pub self_evolution_admission_review_packets: usize,
    pub self_evolution_admission_evidence_ids: usize,
    pub self_evolution_admission_missing_review_packet_refs: usize,
    pub improvement_corpus_events: usize,
    pub improvement_corpus_episodes: usize,
    pub improvement_corpus_active_adaptation: usize,
    pub improvement_corpus_compiler_passed: usize,
    pub improvement_corpus_test_passed: usize,
    pub improvement_corpus_benchmark_passed: usize,
    pub improvement_corpus_privacy_rejected: usize,
    pub improvement_corpus_secret_leaks: usize,
    pub failures: Vec<String>,
}

impl TraceSchemaGateReport {
    pub fn summary_line(&self) -> String {
        format!(
            "trace_schema_gate: passed={} lines={} failures={} rust_check_events={} rust_check_passed={} rust_check_failed={} rust_check_feedback_updates={} rust_check_feedback_applied={} business_contract_events={} business_contract_event_passed={} business_contract_event_failed={} business_contract_event_missing_signals={} business_contract_event_protocol_leaks={} business_contract_event_substitutions={} business_contract_event_evasive_denials={} business_contract_event_raw_passed={} business_contract_event_raw_failed={} business_contract_event_response_normalized={} business_contract_event_sanitized={} business_contract_event_canonical_fallbacks={} runtime_error_events={} runtime_timeout_events={} self_evolution_admission_events={} self_evolution_admission_admitted={} self_evolution_admission_blocked={} self_evolution_admission_review_packets={} self_evolution_admission_evidence_ids={} self_evolution_admission_missing_review_packet_refs={} improvement_corpus_events={} improvement_corpus_episodes={} improvement_corpus_active_adaptation={} improvement_corpus_compiler_passed={} improvement_corpus_test_passed={} improvement_corpus_benchmark_passed={} improvement_corpus_privacy_rejected={} improvement_corpus_secret_leaks={}",
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
            self.runtime_timeout_events,
            self.self_evolution_admission_events,
            self.self_evolution_admission_admitted,
            self.self_evolution_admission_blocked,
            self.self_evolution_admission_review_packets,
            self.self_evolution_admission_evidence_ids,
            self.self_evolution_admission_missing_review_packet_refs,
            self.improvement_corpus_events,
            self.improvement_corpus_episodes,
            self.improvement_corpus_active_adaptation,
            self.improvement_corpus_compiler_passed,
            self.improvement_corpus_test_passed,
            self.improvement_corpus_benchmark_passed,
            self.improvement_corpus_privacy_rejected,
            self.improvement_corpus_secret_leaks
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
    let mut self_evolution_admission_events = 0;
    let mut self_evolution_admission_admitted = 0;
    let mut self_evolution_admission_blocked = 0;
    let mut self_evolution_admission_review_packets = 0;
    let mut self_evolution_admission_evidence_ids = 0;
    let mut self_evolution_admission_missing_review_packet_refs = 0;
    let mut improvement_corpus_events = 0;
    let mut improvement_corpus_episodes = 0;
    let mut improvement_corpus_active_adaptation = 0;
    let mut improvement_corpus_compiler_passed = 0;
    let mut improvement_corpus_test_passed = 0;
    let mut improvement_corpus_benchmark_passed = 0;
    let mut improvement_corpus_privacy_rejected = 0;
    let mut improvement_corpus_secret_leaks = 0;
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
        if let Some(summary) = self_evolution_admission_trace_gate_summary(line) {
            self_evolution_admission_events += summary.events;
            self_evolution_admission_admitted += summary.admitted;
            self_evolution_admission_blocked += summary.blocked;
            self_evolution_admission_review_packets += summary.review_packets;
            self_evolution_admission_evidence_ids += summary.evidence_ids;
            self_evolution_admission_missing_review_packet_refs +=
                summary.missing_review_packet_refs;
        }
        if let Some(summary) = improvement_corpus_trace_gate_summary(line) {
            improvement_corpus_events += summary.events;
            improvement_corpus_episodes += summary.episodes;
            improvement_corpus_active_adaptation += summary.active_adaptation;
            improvement_corpus_compiler_passed += summary.compiler_passed;
            improvement_corpus_test_passed += summary.test_passed;
            improvement_corpus_benchmark_passed += summary.benchmark_passed;
            improvement_corpus_privacy_rejected += summary.privacy_rejected;
            improvement_corpus_secret_leaks += summary.secret_leaks;
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
        self_evolution_admission_events,
        self_evolution_admission_admitted,
        self_evolution_admission_blocked,
        self_evolution_admission_review_packets,
        self_evolution_admission_evidence_ids,
        self_evolution_admission_missing_review_packet_refs,
        improvement_corpus_events,
        improvement_corpus_episodes,
        improvement_corpus_active_adaptation,
        improvement_corpus_compiler_passed,
        improvement_corpus_test_passed,
        improvement_corpus_benchmark_passed,
        improvement_corpus_privacy_rejected,
        improvement_corpus_secret_leaks,
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

#[derive(Debug, Clone, Copy, Default)]
struct SelfEvolutionAdmissionTraceGateSummary {
    events: usize,
    admitted: usize,
    blocked: usize,
    review_packets: usize,
    evidence_ids: usize,
    missing_review_packet_refs: usize,
}

fn self_evolution_admission_trace_gate_summary(
    line: &str,
) -> Option<SelfEvolutionAdmissionTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-self-evolution-admission-v1\"") {
        return None;
    }

    let admitted = extract_json_bool_field(line, "admitted_for_human_review").unwrap_or(false);
    let blocked_reasons = extract_last_json_string_array_field(line, "blocked_reasons")
        .map(|reasons| reasons.len())
        .unwrap_or(0);
    let review_packet = json_object_after_field(line, "review_packet");
    let review_packets = review_packet
        .and_then(|object| extract_json_string_array_field(object, "approval_review_packet_ids"))
        .map(|ids| ids.len())
        .unwrap_or(0);
    let evidence_ids = review_packet
        .and_then(|object| extract_json_string_array_field(object, "evidence_ids"))
        .map(|ids| ids.len())
        .unwrap_or(0);
    let missing_review_packet_refs = usize::from(review_packets == 0 || evidence_ids == 0);

    Some(SelfEvolutionAdmissionTraceGateSummary {
        events: 1,
        admitted: usize::from(admitted),
        blocked: usize::from(!admitted || blocked_reasons > 0),
        review_packets,
        evidence_ids,
        missing_review_packet_refs,
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct ImprovementCorpusTraceGateSummary {
    events: usize,
    episodes: usize,
    active_adaptation: usize,
    compiler_passed: usize,
    test_passed: usize,
    benchmark_passed: usize,
    privacy_rejected: usize,
    secret_leaks: usize,
}

fn improvement_corpus_trace_gate_summary(line: &str) -> Option<ImprovementCorpusTraceGateSummary> {
    if !line.contains("\"schema\":\"rust-norion-improvement-corpus-v1\"") {
        return None;
    }

    let records = json_object_after_field(line, "records");
    let active_adaptation = json_object_after_field(line, "active_adaptation");
    let evidence = json_object_after_field(line, "evidence");
    let privacy = json_object_after_field(line, "privacy");

    Some(ImprovementCorpusTraceGateSummary {
        events: 1,
        episodes: records
            .and_then(|object| extract_json_usize_field(object, "total"))
            .unwrap_or(0),
        active_adaptation: active_adaptation
            .and_then(|object| extract_json_usize_field(object, "eligible"))
            .unwrap_or(0),
        compiler_passed: evidence
            .and_then(|object| extract_json_usize_field(object, "compiler_passed"))
            .unwrap_or(0),
        test_passed: evidence
            .and_then(|object| extract_json_usize_field(object, "test_passed"))
            .unwrap_or(0),
        benchmark_passed: evidence
            .and_then(|object| extract_json_usize_field(object, "benchmark_passed"))
            .unwrap_or(0),
        privacy_rejected: privacy
            .and_then(|object| extract_json_usize_field(object, "rejected"))
            .unwrap_or(0),
        secret_leaks: privacy
            .and_then(|object| extract_json_usize_field(object, "secret_leaks"))
            .unwrap_or(0),
    })
}
