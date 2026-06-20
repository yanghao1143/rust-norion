use crate::gemma_business::GemmaModelServiceBusinessCase;

pub(super) fn push_case_failure(
    business_case: &GemmaModelServiceBusinessCase,
    message: &str,
    failures: &mut Vec<String>,
) {
    failures.push(format!("{} {message}", business_case.name));
}

pub(super) fn require_case_condition(
    business_case: &GemmaModelServiceBusinessCase,
    condition: bool,
    message: &str,
    failures: &mut Vec<String>,
) {
    if !condition {
        push_case_failure(business_case, message, failures);
    }
}

pub(super) fn require_case_u64_at_least(
    business_case: &GemmaModelServiceBusinessCase,
    actual: u64,
    expected: u64,
    message: &str,
    failures: &mut Vec<String>,
) {
    if actual < expected {
        push_case_failure(business_case, message, failures);
    }
}

#[cfg(test)]
mod tests {
    use rust_norion::TaskProfile;

    use crate::gemma_business::GemmaModelServiceBusinessCase;

    use super::{push_case_failure, require_case_condition, require_case_u64_at_least};

    const CASE: GemmaModelServiceBusinessCase = GemmaModelServiceBusinessCase {
        name: "case-a",
        profile: TaskProfile::General,
        prompt: "",
        contract_line: "",
        required_answer_signals: &[],
    };

    #[test]
    fn push_case_failure_prefixes_case_name() {
        let mut failures = Vec::new();

        push_case_failure(&CASE, "failed", &mut failures);

        assert_eq!(failures, vec!["case-a failed".to_owned()]);
    }

    #[test]
    fn require_case_condition_records_false_only() {
        let mut failures = Vec::new();

        require_case_condition(&CASE, true, "true failed", &mut failures);
        require_case_condition(&CASE, false, "false failed", &mut failures);

        assert_eq!(failures, vec!["case-a false failed".to_owned()]);
    }

    #[test]
    fn require_case_u64_at_least_records_underflow_only() {
        let mut failures = Vec::new();

        require_case_u64_at_least(&CASE, 1, 2, "below", &mut failures);
        require_case_u64_at_least(&CASE, 2, 2, "equal", &mut failures);
        require_case_u64_at_least(&CASE, 3, 2, "above", &mut failures);

        assert_eq!(failures, vec!["case-a below".to_owned()]);
    }
}
