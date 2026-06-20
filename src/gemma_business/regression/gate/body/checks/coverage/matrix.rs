use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use crate::gemma_business::regression::gate::body::evidence::ReportBodyEvidence;
use crate::gemma_business::regression::report_checks::require_report_min_u64;
use crate::gemma_business::response_json::response_string_array_field;

pub(super) fn require_matrix_case_coverage(
    body: &str,
    evidence: &ReportBodyEvidence,
    failures: &mut Vec<String>,
) {
    let expected_case_count = GEMMA_MODEL_SERVICE_BUSINESS_CASES.len() as u64;
    require_report_min_u64(
        failures,
        "case_count",
        evidence.case_count,
        expected_case_count,
    );
    require_report_min_u64(
        failures,
        "passed_cases",
        evidence.passed_cases,
        expected_case_count,
    );
    let reported_cases = response_string_array_field(body, "business_cases");
    for business_case in &GEMMA_MODEL_SERVICE_BUSINESS_CASES {
        if !reported_cases
            .iter()
            .any(|reported_case| reported_case == business_case.name)
        {
            push_missing_matrix_case_failure(business_case.name, failures);
        }
    }
}

fn push_missing_matrix_case_failure(case_name: &str, failures: &mut Vec<String>) {
    failures.push(format!(
        "business case {case_name} missing from matrix report"
    ));
}

#[cfg(test)]
mod tests {
    use super::push_missing_matrix_case_failure;

    #[test]
    fn push_missing_matrix_case_failure_formats_case_name() {
        let mut failures = Vec::new();

        push_missing_matrix_case_failure("refund_escalation", &mut failures);

        assert_eq!(
            failures,
            ["business case refund_escalation missing from matrix report"]
        );
    }
}
