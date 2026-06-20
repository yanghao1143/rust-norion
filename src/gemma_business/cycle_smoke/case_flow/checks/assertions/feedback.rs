use super::case::fail_case;
use crate::gemma_business::GemmaModelServiceBusinessCase;

pub(in crate::gemma_business::cycle_smoke::case_flow::checks) fn require_positive_feedback(
    business_case: &GemmaModelServiceBusinessCase,
    amount: u64,
    message: &str,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    if amount == 0 {
        fail_case(business_case, message, failures, case_passed);
    }
}

#[cfg(test)]
mod tests {
    use rust_norion::TaskProfile;

    use crate::gemma_business::GemmaModelServiceBusinessCase;

    use super::require_positive_feedback;

    const CASE: GemmaModelServiceBusinessCase = GemmaModelServiceBusinessCase {
        name: "cycle-case",
        profile: TaskProfile::General,
        prompt: "",
        contract_line: "",
        required_answer_signals: &[],
    };

    #[test]
    fn require_positive_feedback_records_zero_only() {
        let mut failures = Vec::new();
        let mut case_passed = true;

        require_positive_feedback(&CASE, 1, "one failed", &mut failures, &mut case_passed);
        require_positive_feedback(&CASE, 0, "zero failed", &mut failures, &mut case_passed);

        assert_eq!(failures, vec!["cycle-case zero failed".to_owned()]);
        assert!(!case_passed);
    }
}
