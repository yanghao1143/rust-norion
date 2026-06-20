use crate::gemma_business::response_json::{
    response_bool_field, response_optional_string_field, response_optional_u64_field,
};

pub(in crate::gemma_business::regression::gate::body::evidence) fn schema(
    body: &str,
) -> Option<String> {
    response_optional_string_field(body, "schema")
}

pub(in crate::gemma_business::regression::gate::body::evidence) fn case_count(body: &str) -> u64 {
    response_optional_u64_field(body, "case_count")
        .map(at_least_one_case)
        .unwrap_or(1)
}

pub(in crate::gemma_business::regression::gate::body::evidence) fn passed_cases(body: &str) -> u64 {
    response_optional_u64_field(body, "passed_cases")
        .unwrap_or_else(|| single_case_passed_count(body))
}

fn at_least_one_case(count: u64) -> u64 {
    count.max(1)
}

fn single_case_passed_count(body: &str) -> u64 {
    u64::from(response_bool_field(body, "passed"))
}

#[cfg(test)]
mod tests {
    use super::{case_count, passed_cases};

    #[test]
    fn report_body_evidence_defaults_single_case_reports() {
        let passed_body = r#"{ "passed" : true }"#;
        let failed_body = r#"{ "passed" : false }"#;

        assert_eq!(case_count(passed_body), 1);
        assert_eq!(passed_cases(passed_body), 1);
        assert_eq!(case_count(failed_body), 1);
        assert_eq!(passed_cases(failed_body), 0);
    }

    #[test]
    fn report_body_evidence_uses_matrix_case_counts() {
        let body = r#"{ "case_count" : 3, "passed_cases" : 2, "passed" : false }"#;

        assert_eq!(case_count(body), 3);
        assert_eq!(passed_cases(body), 2);
    }

    #[test]
    fn report_body_evidence_never_reports_zero_cases() {
        assert_eq!(case_count(r#"{ "case_count" : 0 }"#), 1);
    }
}
