#[cfg(test)]
pub(in crate::gemma_business::smoke_report) fn single_check_json(
    checked: bool,
    passed: bool,
) -> String {
    format!("{{\"checked\":{},\"passed\":{}}}", checked, passed)
}

pub(in crate::gemma_business::smoke_report) fn matrix_check_json(
    checked: bool,
    passed: bool,
    checked_cases: usize,
    passed_cases: usize,
) -> String {
    format!(
        "{{\"checked\":{},\"passed\":{},\"checked_cases\":{},\"passed_cases\":{}}}",
        checked, passed, checked_cases, passed_cases
    )
}

#[cfg(test)]
mod tests {
    use super::{matrix_check_json, single_check_json};

    #[test]
    fn single_check_json_renders_check_status() {
        assert_eq!(
            single_check_json(true, false),
            "{\"checked\":true,\"passed\":false}"
        );
    }

    #[test]
    fn matrix_check_json_renders_case_counts() {
        assert_eq!(
            matrix_check_json(true, true, 3, 2),
            "{\"checked\":true,\"passed\":true,\"checked_cases\":3,\"passed_cases\":2}"
        );
    }
}
