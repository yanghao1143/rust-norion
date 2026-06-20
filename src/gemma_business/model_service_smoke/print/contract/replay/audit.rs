use crate::gemma_business::model_service_smoke::print::ModelServiceSmokeReport;

pub(super) fn print_contract_audit_evidence(report: &ModelServiceSmokeReport<'_>) {
    let state = report.inspect.business_contract_state;
    let trace = report.inspect.business_contract_trace;
    println!(
        "gemma_model_service_smoke_contract_audit: state_experiences={} state_passed={} state_failed={} state_missing_signals={} state_protocol_leaks={} state_substitutions={} state_evasive_denials={} state_raw_passed={} state_raw_failed={} state_response_normalized={} state_sanitized={} state_canonical_fallbacks={} trace_events={} trace_passed={} trace_failed={} trace_missing_signals={} trace_raw_passed={} trace_raw_failed={} trace_response_normalized={} trace_sanitized={} trace_canonical_fallbacks={}",
        state.items,
        state.passed,
        state.failed,
        state.missing_signals,
        state.protocol_leaks,
        state.substitutions,
        state.evasive_denials,
        state.raw_passed,
        state.raw_failed,
        state.response_normalized,
        state.sanitized,
        state.canonical_fallbacks,
        trace.items,
        trace.passed,
        trace.failed,
        trace.missing_signals,
        trace.raw_passed,
        trace.raw_failed,
        trace.response_normalized,
        trace.sanitized,
        trace.canonical_fallbacks
    );
}
