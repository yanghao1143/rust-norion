use crate::gemma_business::regression::gate::body::evidence::ReportBodyEvidence;
use crate::gemma_business::regression::report_checks::{
    require_report_min_u64, require_report_nonempty_string,
};
use crate::gemma_business::response_json::response_optional_string_field;

pub(super) fn require_runtime_evidence(
    body: &str,
    evidence: &ReportBodyEvidence,
    failures: &mut Vec<String>,
) {
    require_report_min_u64(
        failures,
        "runtime_token_count",
        evidence.runtime_token_count,
        evidence.case_count,
    );
    require_report_nonempty_string(
        failures,
        "runtime_model",
        response_optional_string_field(body, "runtime_model").as_deref(),
    );
}
