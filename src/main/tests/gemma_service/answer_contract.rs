use super::*;

#[test]
fn gemma_business_smoke_rejects_runtime_error_output() {
    assert!(
        gemma_business_smoke_runtime_failure_text(
            "Runtime backend error: runtime command exited with status Some(1)"
        )
        .is_some()
    );
    assert!(gemma_business_smoke_runtime_failure_text("Error 404: Invalid Model Name.").is_some());
    assert!(gemma_business_smoke_runtime_failure_text("本地 Gemma 已处理业务请求。").is_none());
    assert!(gemma_business_smoke_runtime_failure_parts("本地 Gemma 已处理业务请求。", 0).is_some());
    assert!(gemma_business_smoke_runtime_failure_parts("本地 Gemma 已处理业务请求。", 1).is_none());
    assert!(gemma_business_smoke_answer_failure("本地 Gemma 已处理业务请求。").is_some());
    assert!(
        gemma_business_smoke_answer_failure(
            "本地 Gemma 已处理业务请求，可用 runtime_model_experiences 验证。"
        )
        .is_none()
    );
    assert!(
        gemma_business_smoke_answer_failure(
            "由于 runtime_model_experiences 并不属于标准 Rust API，我无法确认它与请求的关联。"
        )
        .is_some()
    );
    assert!(
            gemma_business_smoke_answer_failure(
                "The local Gemma runtime handled the Noiron routing business request; runtime_model_experiences records the audit evidence."
            )
            .is_none()
        );
    assert!(gemma_business_smoke_answer_failure("runtime_model_experiences is present.").is_some());
    assert!(
            gemma_business_smoke_answer_failure(
                ".thought <channel> runtime_model_experiences says local Gemma handled the business request."
            )
            .is_some()
        );
    assert!(
            gemma_business_smoke_answer_failure(
                "The local Gemma runtime handled the business request; runtime_model_experiences is present but memory_experiences is the field used for feedback."
            )
            .is_some()
        );
}

#[test]
fn gemma_model_service_answer_failure_requires_case_signals() {
    let rust_feedback_case = GEMMA_MODEL_SERVICE_BUSINESS_CASES
        .iter()
        .find(|business_case| business_case.name == "gemma-service-rust-feedback")
        .unwrap();

    assert!(
            gemma_model_service_answer_failure(
                rust_feedback_case,
                "Use apply_user_feedback(experience_id, memory_ids, amount) to apply feedback; runtime_model_experiences is audit telemetry, not an API."
            )
            .is_some()
        );
    assert!(
            gemma_model_service_answer_failure(
                rust_feedback_case,
                "Use apply_user_feedback(experience_id, memory_ids, amount) to apply feedback to memory; runtime_model_experiences is audit telemetry, not an API."
            )
            .is_none()
        );
}

#[test]
fn gemma_model_service_business_answer_normalizes_field_alias() {
    let rust_feedback_case = GEMMA_MODEL_SERVICE_BUSINESS_CASES
        .iter()
        .find(|business_case| business_case.name == "gemma-service-rust-feedback")
        .unwrap();
    let routing_case = GEMMA_MODEL_SERVICE_BUSINESS_CASES
        .iter()
        .find(|business_case| business_case.name == "gemma-service-en-routing")
        .unwrap();
    let normalized = normalize_gemma_model_service_business_answer(
            rust_feedback_case,
            "language_model_experiences=audit telemetry; apply_user_feedback=interface; feedback=applied to memory.",
        )
        .expect("field alias should be normalized");

    assert!(normalized.contains("runtime_model_experiences"));
    assert!(!normalized.contains("language_model_experiences"));
    let no_model = normalize_gemma_model_service_business_answer(
        routing_case,
        "no model_experiences=audit telemetry; Noiron=routing; business=handled.",
    )
    .expect("split field alias should be normalized");
    assert!(no_model.contains("runtime_model_experiences"));
    assert!(!no_model.contains("no model_experiences"));
    let prefixed = normalize_gemma_model_service_business_answer(
        routing_case,
        "Noiron=routing; business=handled.",
    )
    .expect("missing audit field should be prefixed");
    assert!(prefixed.starts_with("runtime_model_experiences=audit telemetry;"));
    let canonical = normalize_gemma_model_service_business_answer(
            rust_feedback_case,
            "runtime_model_experiences=audit telemetry; apply_user_feedback=interface; feedback=applied.",
        )
        .expect("missing case signal should use canonical response");
    assert_eq!(canonical, rust_feedback_case.contract_line);
    assert!(
        normalize_gemma_model_service_business_answer(
            rust_feedback_case,
            rust_feedback_case.contract_line
        )
        .is_none()
    );
    assert!(
            normalize_gemma_model_service_business_answer(
                rust_feedback_case,
                ".thought runtime_model_experiences=audit telemetry; apply_user_feedback=interface; feedback=applied to memory."
            )
            .is_some()
        );
    let sanitized = normalize_gemma_model_service_business_answer(
            routing_case,
            "thought <channel|>runtime_model_experiences=audit telemetry; Noiron=routing; business=handled.",
        )
        .expect("channel artifact should be sanitized to canonical response");
    assert_eq!(sanitized, routing_case.contract_line);
    let evasive = gemma_model_service_business_normalization(
        routing_case,
        "I can't comply with that exact business receipt.",
    );
    assert_eq!(
        evasive.kind,
        GemmaModelServiceBusinessNormalizationKind::CanonicalFallback
    );
    assert!(!evasive.raw_audit.passed());
    assert_eq!(evasive.answer, routing_case.contract_line);
}

#[test]
fn gemma_model_service_answer_audit_summarizes_contract_failures() {
    let rust_feedback_case = GEMMA_MODEL_SERVICE_BUSINESS_CASES
        .iter()
        .find(|business_case| business_case.name == "gemma-service-rust-feedback")
        .unwrap();

    let audit = GemmaModelServiceAnswerAudit::from_case(
        rust_feedback_case,
        ".thought <channel> apply_user_feedback records memory_experiences for feedback.",
    );

    assert!(!audit.passed());
    assert_eq!(audit.required_signals, 4);
    assert_eq!(audit.matched_signals, 2);
    assert_eq!(
        audit.missing_signals,
        vec![
            "runtime_model_experiences".to_owned(),
            "to memory".to_owned()
        ]
    );
    assert!(!audit.has_runtime_model_experiences);
    assert!(audit.protocol_leak);
    assert!(audit.substituted_runtime_model_experiences);
    assert!(audit.handling_signal);
    assert_eq!(
        audit.failure().as_deref(),
        Some("answer did not include runtime_model_experiences evidence field")
    );
}
