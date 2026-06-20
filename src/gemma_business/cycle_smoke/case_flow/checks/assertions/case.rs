use crate::gemma_business::GemmaModelServiceBusinessCase;

pub(in crate::gemma_business::cycle_smoke::case_flow::checks) fn push_case_failure(
    business_case: &GemmaModelServiceBusinessCase,
    message: &str,
    failures: &mut Vec<String>,
) {
    failures.push(format!("{} {message}", business_case.name));
}

pub(in crate::gemma_business::cycle_smoke::case_flow::checks) fn fail_case(
    business_case: &GemmaModelServiceBusinessCase,
    message: &str,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    push_case_failure(business_case, message, failures);
    *case_passed = false;
}

#[cfg(test)]
mod tests {
    use rust_norion::TaskProfile;

    use crate::gemma_business::GemmaModelServiceBusinessCase;

    use super::{fail_case, push_case_failure};

    const CASE: GemmaModelServiceBusinessCase = GemmaModelServiceBusinessCase {
        name: "cycle-case",
        profile: TaskProfile::General,
        prompt: "",
        contract_line: "",
        required_answer_signals: &[],
    };

    #[test]
    fn push_case_failure_prefixes_cycle_case_name() {
        let mut failures = Vec::new();

        push_case_failure(&CASE, "failed", &mut failures);

        assert_eq!(failures, vec!["cycle-case failed".to_owned()]);
    }

    #[test]
    fn fail_case_records_failure_and_marks_case_failed() {
        let mut failures = Vec::new();
        let mut case_passed = true;

        fail_case(&CASE, "failed", &mut failures, &mut case_passed);

        assert_eq!(failures, vec!["cycle-case failed".to_owned()]);
        assert!(!case_passed);
    }
}
