use super::*;

#[test]
fn gemma_model_service_runtime_audit_flags_runtime_errors() {
    let body = "{\"state\":{\"runtime_error_experiences\":1,\"runtime_errors\":2,\"runtime_timeout_experiences\":3,\"runtime_timeouts\":4},\"trace_gate\":{\"runtime_error_events\":5,\"runtime_timeout_events\":6}}";
    let audit = GemmaModelServiceRuntimeAudit::from_inspect_body(body);
    let mut failures = Vec::new();

    audit.push_failures(&mut failures);

    assert!(!audit.passed());
    assert_eq!(audit.runtime_error_experiences, 1);
    assert_eq!(audit.runtime_errors, 2);
    assert_eq!(audit.runtime_timeout_experiences, 3);
    assert_eq!(audit.runtime_timeouts, 4);
    assert_eq!(audit.trace_runtime_error_events, 5);
    assert_eq!(audit.trace_runtime_timeout_events, 6);
    assert_eq!(
        failures,
        vec![
            "inspect state recorded runtime_error_experiences=1".to_owned(),
            "inspect state recorded runtime_errors=2".to_owned(),
            "inspect state recorded runtime_timeout_experiences=3".to_owned(),
            "inspect state recorded runtime_timeouts=4".to_owned(),
            "inspect trace recorded runtime_error_events=5".to_owned(),
            "inspect trace recorded runtime_timeout_events=6".to_owned(),
        ]
    );
}

#[test]
fn gemma_model_service_rust_feedback_prompt_requires_business_signals() {
    let prompt = GEMMA_MODEL_SERVICE_BUSINESS_CASES
        .iter()
        .find(|business_case| business_case.name == "gemma-service-rust-feedback")
        .expect("rust feedback business case should exist")
        .prompt;

    for signal in [
        "runtime_model_experiences",
        "apply_user_feedback",
        "feedback",
        "to memory",
    ] {
        assert!(prompt.contains(signal), "{prompt}");
    }
}

#[test]
fn gemma_model_service_prompts_cover_required_answer_signals() {
    for business_case in GEMMA_MODEL_SERVICE_BUSINESS_CASES {
        let lower = business_case.prompt.to_ascii_lowercase();
        for signal in business_case.required_answer_signals {
            assert!(
                business_answer_contains_signal(business_case.prompt, &lower, signal),
                "{} prompt missing required signal {signal}",
                business_case.name
            );
        }
    }
}

#[test]
fn gemma_model_service_prompts_avoid_protocol_trigger_words() {
    for business_case in GEMMA_MODEL_SERVICE_BUSINESS_CASES {
        let lower = business_case.prompt.to_ascii_lowercase();
        for trigger in ["thought", "channel", "hidden"] {
            assert!(
                !lower.contains(trigger),
                "{} prompt contains protocol trigger {trigger}",
                business_case.name
            );
        }
    }
}
