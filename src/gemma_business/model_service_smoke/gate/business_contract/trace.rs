use crate::gemma_business::model_service_smoke::evidence::InspectEvidence;

use super::super::checks::{
    require_at_least_u64, require_business_contract_normalization_match, require_zero_u64,
};

pub(super) fn push_trace_business_contract_failures(
    inspect: &InspectEvidence,
    contract_case_count: u64,
    failures: &mut Vec<String>,
) {
    let trace = inspect.business_contract_trace;
    require_at_least_u64(
        trace.items,
        contract_case_count,
        "inspect trace gate did not record every business contract event",
        failures,
    );
    require_at_least_u64(
        trace.passed,
        contract_case_count,
        "inspect trace gate did not record every business contract pass",
        failures,
    );
    require_zero_u64(
        trace.failed,
        "inspect trace gate recorded business_contract_event_failed",
        failures,
    );
    require_zero_u64(
        trace.missing_signals,
        "inspect trace gate recorded business_contract_event_missing_signals",
        failures,
    );
    require_at_least_u64(
        trace.raw_total(),
        contract_case_count,
        "inspect trace gate did not record every raw business contract audit",
        failures,
    );
    require_business_contract_normalization_match(
        trace,
        "inspect trace business contract normalization counters disagree",
        failures,
    );
}
