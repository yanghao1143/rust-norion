use model_pool_advice_core::{
    CAPACITY_POLICY, HELPER_ROLES, HELPER_TARGET_WORKERS, MAX_QUALITY_12B_WORKERS,
    ModelPoolDecision, ModelPoolFacts, POLICY, RECOMMENDED_LAUNCH_ROLES, missing_helper_roles,
};

use crate::app::status_json::{
    json_string_literal, require_json_bool_equals, require_json_string_equals, required_json_string,
};

const ADVICE_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_advice.v1";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ModelPoolAdviceJsonSummary {
    pub(super) safe_to_enable_pool_workers: bool,
    pub(super) next_step: String,
    pub(super) reason: String,
    pub(super) kind: String,
    pub(super) missing_helper_roles: Vec<String>,
    pub(super) helper_cpu_or_no_gpu_roles: Vec<String>,
}

pub(super) fn model_pool_advice_json(
    facts: &ModelPoolFacts,
    decision: &ModelPoolDecision,
) -> String {
    let visible_helper_roles = visible_helper_roles(facts);
    let missing_helper_roles = missing_helper_roles(facts);
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":true,",
            "\"launches_process\":false,",
            "\"sends_prompt\":false,",
            "\"policy\":{},",
            "\"capacity_policy\":{},",
            "\"avoid_extra_12b\":true,",
            "\"max_quality_12b_workers\":{},",
            "\"safe_to_enable_pool_workers\":{},",
            "\"next_step\":{},",
            "\"reason\":{},",
            "\"kind\":{},",
            "\"quality_ready\":{},",
            "\"quality_context_sufficient\":{},",
            "\"quality_context_tokens\":{},",
            "\"quality_required_context_tokens\":{},",
            "\"quality_runtime_accelerated\":{},",
            "\"capacity_recommendation\":{},",
            "\"capacity_expansion_allowed\":{},",
            "\"quality_worker_count\":{},",
            "\"helper_worker_count\":{},",
            "\"healthy_helper_worker_count\":{},",
            "\"unknown_runtime_worker_count\":{},",
            "\"helper_target_worker_count\":{},",
            "\"helper_roles\":{},",
            "\"expected_helper_roles\":{},",
            "\"missing_helper_roles\":{},",
            "\"helper_cpu_or_no_gpu_roles\":{},",
            "\"recommended_launch_order\":{},",
            "\"extra_quality_12b_detected\":{},",
            "\"worker_shape\":{{",
            "\"quality\":{},",
            "\"helpers_visible\":{},",
            "\"helpers_healthy\":{},",
            "\"helper_target\":{}",
            "}}",
            "}}"
        ),
        json_string_literal(ADVICE_JSON_SCHEMA),
        json_string_literal(POLICY),
        json_string_literal(CAPACITY_POLICY),
        MAX_QUALITY_12B_WORKERS,
        bool_json(decision.safe_to_enable_pool_workers),
        json_string_literal(decision.next_step),
        json_string_literal(decision.reason),
        json_string_literal(decision.kind.as_str()),
        option_bool_json(facts.quality_ready),
        option_bool_json(facts.quality_context_sufficient),
        option_string_json(facts.quality_context_tokens.as_deref()),
        option_string_json(facts.quality_required_context_tokens.as_deref()),
        option_bool_json(facts.quality_runtime_accelerated),
        option_string_json(facts.capacity_recommendation.as_deref()),
        option_bool_json(facts.expansion_allowed),
        facts.quality_worker_count,
        facts.helper_worker_count,
        option_usize_json(facts.healthy_helper_worker_count),
        option_usize_json(facts.unknown_runtime_worker_count),
        HELPER_TARGET_WORKERS,
        json_str_array(&visible_helper_roles),
        json_str_array(&HELPER_ROLES),
        json_str_array(&missing_helper_roles),
        json_string_vec_array(&facts.helper_cpu_or_no_gpu_roles),
        json_str_array(&RECOMMENDED_LAUNCH_ROLES),
        bool_json(facts.extra_quality_12b_detected()),
        facts.quality_worker_count,
        facts.helper_worker_count,
        option_usize_json(facts.healthy_helper_worker_count),
        HELPER_TARGET_WORKERS,
    )
}

#[cfg(test)]
pub(super) fn validate_model_pool_advice_json(advice_json: &str) -> Result<(), String> {
    model_pool_advice_json_summary(advice_json).map(|_| ())
}

pub(super) fn model_pool_advice_json_summary(
    advice_json: &str,
) -> Result<ModelPoolAdviceJsonSummary, String> {
    require_json_string_equals(
        advice_json,
        "schema",
        ADVICE_JSON_SCHEMA,
        "model pool advice JSON schema",
    )?;
    require_json_bool_equals(
        advice_json,
        "read_only",
        true,
        "model pool advice JSON read_only",
    )?;
    require_json_bool_equals(
        advice_json,
        "launches_process",
        false,
        "model pool advice JSON launches_process",
    )?;
    require_json_bool_equals(
        advice_json,
        "sends_prompt",
        false,
        "model pool advice JSON sends_prompt",
    )?;
    let safe_to_enable_pool_workers = required_bool(
        advice_json,
        "safe_to_enable_pool_workers",
        "model pool advice JSON safe_to_enable_pool_workers",
    )?;
    let next_step =
        required_json_string(advice_json, "next_step", "model pool advice JSON next_step")?;
    let reason = required_json_string(advice_json, "reason", "model pool advice JSON reason")?;
    let kind = required_json_string(advice_json, "kind", "model pool advice JSON kind")?;
    let missing_helper_roles =
        crate::app::status_json::json_string_array_field(advice_json, "missing_helper_roles")
            .ok_or_else(|| "model pool advice JSON missing missing_helper_roles".to_owned())?;
    let helper_cpu_or_no_gpu_roles =
        crate::app::status_json::json_string_array_field(advice_json, "helper_cpu_or_no_gpu_roles")
            .ok_or_else(|| {
                "model pool advice JSON missing helper_cpu_or_no_gpu_roles".to_owned()
            })?;

    Ok(ModelPoolAdviceJsonSummary {
        safe_to_enable_pool_workers,
        next_step,
        reason,
        kind,
        missing_helper_roles,
        helper_cpu_or_no_gpu_roles,
    })
}

fn required_bool(object: &str, field: &str, label: &str) -> Result<bool, String> {
    crate::app::status_json::json_bool_field(object, field)
        .ok_or_else(|| format!("{label} missing {field}"))
}

fn visible_helper_roles(facts: &ModelPoolFacts) -> Vec<&'static str> {
    HELPER_ROLES
        .into_iter()
        .filter(|role| match *role {
            "summary" => facts.has_summary,
            "router" => facts.has_router,
            "review" => facts.has_review,
            "index" => facts.has_index,
            "test-gate" => facts.has_test_gate,
            _ => false,
        })
        .collect()
}

fn json_str_array(values: &[&str]) -> String {
    let values = values
        .iter()
        .map(|value| json_string_literal(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn json_string_vec_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| json_string_literal(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn option_string_json(value: Option<&str>) -> String {
    value
        .map(json_string_literal)
        .unwrap_or_else(|| "null".to_owned())
}

fn option_usize_json(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_bool_json(value: Option<bool>) -> &'static str {
    value.map(bool_json).unwrap_or("null")
}

fn bool_json(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::status_json::{json_bool_field, json_string_array_field, json_string_field};

    #[test]
    fn advice_json_projects_decision_and_helper_roles() {
        let facts = ModelPoolFacts {
            quality_ready: Some(true),
            quality_context_sufficient: Some(true),
            quality_context_tokens: Some("262144".to_owned()),
            quality_required_context_tokens: Some("262144".to_owned()),
            quality_runtime_accelerated: Some(true),
            capacity_recommendation: Some("add_remaining_helper_roles_one_at_a_time".to_owned()),
            expansion_allowed: Some(true),
            healthy_helper_worker_count: Some(2),
            has_summary: true,
            has_test_gate: true,
            quality_worker_count: 1,
            helper_worker_count: 2,
            ..ModelPoolFacts::default()
        };
        let decision = ModelPoolDecision {
            safe_to_enable_pool_workers: true,
            next_step: "add_remaining_helper_roles_one_at_a_time",
            reason: "partial_helper_pool_visible",
            kind: model_pool_advice_core::AdviceKind::Busy,
        };

        let json = model_pool_advice_json(&facts, &decision);

        assert_eq!(
            json_string_field(&json, "schema").as_deref(),
            Some(ADVICE_JSON_SCHEMA)
        );
        assert_eq!(json_bool_field(&json, "read_only"), Some(true));
        assert_eq!(
            json_bool_field(&json, "safe_to_enable_pool_workers"),
            Some(true)
        );
        assert_eq!(
            json_string_field(&json, "next_step").as_deref(),
            Some("add_remaining_helper_roles_one_at_a_time")
        );
        assert_eq!(
            json_string_array_field(&json, "helper_roles"),
            Some(vec!["summary".to_owned(), "test-gate".to_owned()])
        );
        assert_eq!(
            json_string_array_field(&json, "missing_helper_roles"),
            Some(vec![
                "router".to_owned(),
                "review".to_owned(),
                "index".to_owned()
            ])
        );
        assert_eq!(
            json_string_array_field(&json, "helper_cpu_or_no_gpu_roles"),
            Some(vec![])
        );
        assert_eq!(
            json_string_array_field(&json, "recommended_launch_order"),
            Some(
                RECOMMENDED_LAUNCH_ROLES
                    .iter()
                    .map(|role| (*role).to_owned())
                    .collect()
            )
        );
    }

    #[test]
    fn advice_json_summary_projects_machine_readable_decision() {
        let facts = ModelPoolFacts {
            has_summary: true,
            has_review: true,
            quality_worker_count: 1,
            helper_worker_count: 2,
            ..ModelPoolFacts::default()
        };
        let decision = ModelPoolDecision {
            safe_to_enable_pool_workers: false,
            next_step: "add_remaining_helper_roles_one_at_a_time",
            reason: "partial_helper_pool_visible",
            kind: model_pool_advice_core::AdviceKind::Busy,
        };
        let json = model_pool_advice_json(&facts, &decision);

        let summary = model_pool_advice_json_summary(&json).unwrap();

        assert_eq!(
            summary,
            ModelPoolAdviceJsonSummary {
                safe_to_enable_pool_workers: false,
                next_step: "add_remaining_helper_roles_one_at_a_time".to_owned(),
                reason: "partial_helper_pool_visible".to_owned(),
                kind: "busy".to_owned(),
                missing_helper_roles: vec![
                    "router".to_owned(),
                    "index".to_owned(),
                    "test-gate".to_owned()
                ],
                helper_cpu_or_no_gpu_roles: vec![],
            }
        );
    }

    #[test]
    fn advice_json_summary_projects_helper_cpu_or_no_gpu_roles() {
        let facts = ModelPoolFacts {
            has_summary: true,
            has_review: true,
            quality_worker_count: 1,
            helper_worker_count: 2,
            helper_cpu_or_no_gpu_roles: vec!["review".to_owned()],
            ..ModelPoolFacts::default()
        };
        let decision = ModelPoolDecision {
            safe_to_enable_pool_workers: false,
            next_step: "fix_helper_metal_or_gpu_layers_before_more_pool_workers",
            reason: "helper_workers_not_gpu_accelerated",
            kind: model_pool_advice_core::AdviceKind::Error,
        };
        let json = model_pool_advice_json(&facts, &decision);

        let summary = model_pool_advice_json_summary(&json).unwrap();

        assert_eq!(summary.helper_cpu_or_no_gpu_roles, vec!["review"]);
        assert_eq!(
            json_string_array_field(&json, "helper_cpu_or_no_gpu_roles"),
            Some(vec!["review".to_owned()])
        );
    }

    #[test]
    fn advice_json_validation_rejects_schema_and_side_effect_drift() {
        let facts = ModelPoolFacts::default();
        let decision = ModelPoolDecision {
            safe_to_enable_pool_workers: false,
            next_step: "start_or_fix_quality_worker_8686",
            reason: "quality_worker_not_ready",
            kind: model_pool_advice_core::AdviceKind::Error,
        };
        let json = model_pool_advice_json(&facts, &decision);
        let wrong_schema = json.replacen(
            "\"schema\":\"smartsteam.forge.model_pool_advice.v1\"",
            "\"schema\":\"wrong.v1\"",
            1,
        );
        let sends_prompt = json.replacen("\"sends_prompt\":false", "\"sends_prompt\":true", 1);

        assert!(
            validate_model_pool_advice_json(&wrong_schema)
                .unwrap_err()
                .contains("advice JSON schema")
        );
        assert!(
            validate_model_pool_advice_json(&sends_prompt)
                .unwrap_err()
                .contains("advice JSON sends_prompt")
        );
    }

    #[test]
    fn advice_json_validation_rejects_missing_decision_fields() {
        let facts = ModelPoolFacts::default();
        let decision = ModelPoolDecision {
            safe_to_enable_pool_workers: false,
            next_step: "start_or_fix_quality_worker_8686",
            reason: "quality_worker_not_ready",
            kind: model_pool_advice_core::AdviceKind::Error,
        };
        let json = model_pool_advice_json(&facts, &decision).replacen(
            "\"next_step\":\"start_or_fix_quality_worker_8686\",",
            "",
            1,
        );

        assert!(
            validate_model_pool_advice_json(&json)
                .unwrap_err()
                .contains("advice JSON next_step")
        );
    }
}
