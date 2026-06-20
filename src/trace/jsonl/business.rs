use std::io;
use std::path::Path;

use super::json::{option_string_json, option_u64_json, string_array_json};
use super::writer::append_line;

#[allow(clippy::too_many_arguments)]
pub fn business_contract_trace_json_line(
    case_name: &str,
    experience_id: Option<u64>,
    required_signals: usize,
    matched_signals: usize,
    missing_signals: &[String],
    has_runtime_model_experiences: bool,
    protocol_leak: bool,
    substituted_runtime_model_experiences: bool,
    evasive_denial: bool,
    handling_signal: bool,
    raw_passed: bool,
    normalization: &str,
    response_normalized: bool,
    canonical_fallback: bool,
) -> String {
    let passed = has_runtime_model_experiences
        && !protocol_leak
        && !substituted_runtime_model_experiences
        && !evasive_denial
        && handling_signal
        && missing_signals.is_empty();
    format!(
        "{{\
         \"schema\":\"rust-norion-business-contract-v1\",\
         \"case\":{},\
         \"experience_id\":{},\
         \"business_contract\":{{\"passed\":{},\"required_signals\":{},\"matched_signals\":{},\"missing_signal_count\":{},\"missing_signals\":{},\"has_runtime_model_experiences\":{},\"protocol_leak\":{},\"substituted_runtime_model_experiences\":{},\"evasive_denial\":{},\"handling_signal\":{},\"raw_passed\":{},\"normalization\":{},\"response_normalized\":{},\"canonical_fallback\":{}}}\
         }}",
        option_string_json(Some(case_name)),
        option_u64_json(experience_id),
        passed,
        required_signals,
        matched_signals,
        missing_signals.len(),
        string_array_json(missing_signals),
        has_runtime_model_experiences,
        protocol_leak,
        substituted_runtime_model_experiences,
        evasive_denial,
        handling_signal,
        raw_passed,
        option_string_json(Some(normalization)),
        response_normalized,
        canonical_fallback
    )
}

#[allow(clippy::too_many_arguments)]
pub fn append_business_contract_trace_jsonl(
    path: impl AsRef<Path>,
    case_name: &str,
    experience_id: Option<u64>,
    required_signals: usize,
    matched_signals: usize,
    missing_signals: &[String],
    has_runtime_model_experiences: bool,
    protocol_leak: bool,
    substituted_runtime_model_experiences: bool,
    evasive_denial: bool,
    handling_signal: bool,
    raw_passed: bool,
    normalization: &str,
    response_normalized: bool,
    canonical_fallback: bool,
) -> io::Result<()> {
    let line = business_contract_trace_json_line(
        case_name,
        experience_id,
        required_signals,
        matched_signals,
        missing_signals,
        has_runtime_model_experiences,
        protocol_leak,
        substituted_runtime_model_experiences,
        evasive_denial,
        handling_signal,
        raw_passed,
        normalization,
        response_normalized,
        canonical_fallback,
    );
    append_line(path, &line)
}
