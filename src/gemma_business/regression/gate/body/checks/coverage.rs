mod matrix;

use crate::gemma_business::regression::gate::body::evidence::ReportBodyEvidence;
use crate::gemma_business::regression::report_checks::require_report_string;
use crate::gemma_business::response_json::response_optional_string_field;

use matrix::require_matrix_case_coverage;

pub(super) fn require_schema_and_case_coverage(
    body: &str,
    evidence: &ReportBodyEvidence,
    failures: &mut Vec<String>,
) {
    require_report_string(
        failures,
        "schema",
        evidence.schema.as_deref(),
        "rust-norion-gemma-business-cycle-smoke-v1",
    );
    if evidence.case_count > 1 {
        require_report_string(
            failures,
            "business_case",
            response_optional_string_field(body, "business_case").as_deref(),
            "gemma-business-cycle-matrix",
        );
        require_matrix_case_coverage(body, evidence, failures);
    } else {
        require_report_string(
            failures,
            "business_case",
            response_optional_string_field(body, "business_case").as_deref(),
            "gemma-service-rust-feedback",
        );
    }
}
