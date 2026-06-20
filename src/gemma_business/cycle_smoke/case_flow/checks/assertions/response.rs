use super::case::fail_case;
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::response_json::{
    response_bool_field, response_object_bool_field, response_ok,
};

pub(in crate::gemma_business::cycle_smoke::case_flow::checks) fn require_response_bool_field(
    business_case: &GemmaModelServiceBusinessCase,
    body: &str,
    field: &str,
    message: &str,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    if !response_bool_field(body, field) {
        fail_case(business_case, message, failures, case_passed);
    }
}

pub(in crate::gemma_business::cycle_smoke::case_flow::checks) fn require_response_ok(
    business_case: &GemmaModelServiceBusinessCase,
    body: &str,
    message: &str,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    if !response_ok(body) {
        fail_case(business_case, message, failures, case_passed);
    }
}

pub(in crate::gemma_business::cycle_smoke::case_flow::checks) fn require_response_object_bool_field(
    business_case: &GemmaModelServiceBusinessCase,
    body: &str,
    object: &str,
    field: &str,
    message: &str,
    failures: &mut Vec<String>,
    case_passed: &mut bool,
) {
    if !response_object_bool_field(body, object, field) {
        fail_case(business_case, message, failures, case_passed);
    }
}

#[cfg(test)]
mod tests {
    use rust_norion::TaskProfile;

    use crate::gemma_business::GemmaModelServiceBusinessCase;

    use super::{
        require_response_bool_field, require_response_object_bool_field, require_response_ok,
    };

    const CASE: GemmaModelServiceBusinessCase = GemmaModelServiceBusinessCase {
        name: "cycle-case",
        profile: TaskProfile::General,
        prompt: "",
        contract_line: "",
        required_answer_signals: &[],
    };

    #[test]
    fn response_assertions_record_false_or_missing_only() {
        let mut failures = Vec::new();
        let mut case_passed = true;

        require_response_ok(
            &CASE,
            r#"{"ok":true}"#,
            "ok failed",
            &mut failures,
            &mut case_passed,
        );
        require_response_bool_field(
            &CASE,
            r#"{"feedback":false}"#,
            "feedback",
            "feedback failed",
            &mut failures,
            &mut case_passed,
        );
        require_response_object_bool_field(
            &CASE,
            r#"{"state_gate":{"passed":false}}"#,
            "state_gate",
            "passed",
            "state failed",
            &mut failures,
            &mut case_passed,
        );

        assert_eq!(
            failures,
            vec![
                "cycle-case feedback failed".to_owned(),
                "cycle-case state failed".to_owned()
            ]
        );
        assert!(!case_passed);
    }
}
