use crate::json::{
    json_array_field, json_bool_field, json_number_field, json_object_field, json_object_items,
    json_string, json_string_field,
};
use model_pool_advice_core::{
    CAPACITY_POLICY, HELPER_ROLES, MAX_QUALITY_12B_WORKERS, ModelPoolFacts, POLICY,
    RECOMMENDED_LAUNCH_ROLES, missing_helper_roles as core_missing_helper_roles,
    model_pool_advice_text_zh, model_pool_decision,
};

pub(crate) fn model_pool_advice_json(status_body: &str) -> String {
    let facts = facts_from_status(status_body);
    let advice = model_pool_decision(&facts);
    let advice_text = model_pool_advice_text_zh(&facts, &advice);
    let missing_helper_roles = missing_helper_roles_json(&facts);
    format!(
        "{{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"policy\":{},\"capacity_policy\":{},\"avoid_extra_12b\":true,\"max_quality_12b_workers\":{},\"extra_quality_12b_detected\":{},\"safe_to_enable_pool_workers\":{},\"next_step\":{},\"reason\":{},\"kind\":{},\"advice\":{},\"quality_ready\":{},\"quality_context_sufficient\":{},\"quality_context_tokens\":{},\"quality_required_context_tokens\":{},\"quality_runtime_accelerated\":{},\"capacity_recommendation\":{},\"capacity_expansion_allowed\":{},\"healthy_helper_worker_count\":{},\"unknown_runtime_worker_count\":{},\"quality_worker_count\":{},\"helper_worker_count\":{},\"expected_helper_roles\":{},\"missing_helper_roles\":{},\"recommended_launch_order\":{},\"helper_roles\":{{\"summary\":{},\"router\":{},\"review\":{},\"index\":{},\"test_gate\":{}}}}}",
        json_string(POLICY),
        json_string(CAPACITY_POLICY),
        MAX_QUALITY_12B_WORKERS,
        facts.extra_quality_12b_detected(),
        advice.safe_to_enable_pool_workers,
        json_string(advice.next_step),
        json_string(advice.reason),
        json_string(advice.kind.as_str()),
        json_string(&advice_text),
        option_bool_json(facts.quality_ready),
        option_bool_json(facts.quality_context_sufficient),
        option_string_json(facts.quality_context_tokens.as_deref()),
        option_string_json(facts.quality_required_context_tokens.as_deref()),
        option_bool_json(facts.quality_runtime_accelerated),
        option_string_json(facts.capacity_recommendation.as_deref()),
        option_bool_json(facts.expansion_allowed),
        option_usize_string_json(facts.healthy_helper_worker_count),
        option_usize_string_json(facts.unknown_runtime_worker_count),
        facts.quality_worker_count,
        facts.helper_worker_count,
        json_string_array(&HELPER_ROLES),
        missing_helper_roles,
        json_string_array(&RECOMMENDED_LAUNCH_ROLES),
        facts.has_summary,
        facts.has_router,
        facts.has_review,
        facts.has_index,
        facts.has_test_gate
    )
}

fn missing_helper_roles_json(facts: &ModelPoolFacts) -> String {
    let missing = core_missing_helper_roles(facts);
    json_string_array(&missing)
}

fn facts_from_status(status_body: &str) -> ModelPoolFacts {
    let quality = worker_by_role(status_body, "quality");
    let capacity = json_object_field(status_body, "capacity");
    let worker_roles = worker_roles(status_body);
    ModelPoolFacts {
        quality_ready: json_bool_field(status_body, "quality_ready")
            .or_else(|| quality.and_then(worker_ready)),
        quality_context_sufficient: json_bool_field(status_body, "quality_context_sufficient"),
        quality_context_tokens: json_number_field(status_body, "quality_context_tokens")
            .or_else(|| worker_context(quality)),
        quality_required_context_tokens: json_number_field(
            status_body,
            "quality_context_required_tokens",
        ),
        quality_runtime_accelerated: capacity
            .and_then(|capacity| json_bool_field(capacity, "quality_runtime_accelerated")),
        capacity_recommendation: capacity
            .and_then(|capacity| json_string_field(capacity, "recommendation")),
        expansion_allowed: capacity
            .and_then(|capacity| json_bool_field(capacity, "expansion_allowed")),
        healthy_helper_worker_count: capacity
            .and_then(|capacity| json_number_field(capacity, "healthy_helper_worker_count"))
            .and_then(|value| value.parse::<usize>().ok()),
        unknown_runtime_worker_count: capacity
            .and_then(|capacity| json_number_field(capacity, "unknown_runtime_worker_count"))
            .and_then(|value| value.parse::<usize>().ok()),
        has_summary: ready_worker_by_role(status_body, "summary"),
        has_router: ready_worker_by_role(status_body, "router"),
        has_review: ready_worker_by_role(status_body, "review"),
        has_index: ready_worker_by_role(status_body, "index"),
        has_test_gate: ready_worker_by_role(status_body, "test-gate"),
        quality_worker_count: worker_roles
            .iter()
            .filter(|role| role.as_str() == "quality")
            .count(),
        helper_worker_count: worker_roles
            .iter()
            .filter(|role| {
                matches!(
                    role.as_str(),
                    "summary" | "router" | "review" | "index" | "test-gate"
                )
            })
            .count(),
        quality_cpu_fallback: quality.is_some_and(worker_looks_cpu_bound),
        quality_zero_gpu_layers: quality.is_some_and(worker_has_zero_gpu_layers),
        helper_cpu_or_no_gpu_roles: helper_cpu_or_no_gpu_roles(status_body),
    }
}

fn worker_by_role<'a>(status_body: &'a str, role: &str) -> Option<&'a str> {
    json_array_field(status_body, "workers")?
        .pipe(json_object_items)
        .into_iter()
        .find(|worker| json_string_field(worker, "role").as_deref() == Some(role))
}

fn ready_worker_by_role(status_body: &str, role: &str) -> bool {
    worker_by_role(status_body, role)
        .and_then(worker_ready)
        .unwrap_or(false)
}

fn worker_roles(status_body: &str) -> Vec<String> {
    json_array_field(status_body, "workers")
        .map(json_object_items)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|worker| json_string_field(worker, "role"))
        .collect()
}

fn helper_cpu_or_no_gpu_roles(status_body: &str) -> Vec<String> {
    json_array_field(status_body, "workers")
        .map(json_object_items)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|worker| {
            let role = json_string_field(worker, "role")?;
            if HELPER_ROLES.contains(&role.as_str())
                && (worker_looks_cpu_bound(worker) || worker_has_zero_gpu_layers(worker))
            {
                Some(role)
            } else {
                None
            }
        })
        .collect()
}

fn worker_ready(worker: &str) -> Option<bool> {
    json_bool_field(worker, "ready")
        .or_else(|| json_bool_field(worker, "role_ready"))
        .or_else(|| match json_string_field(worker, "status").as_deref() {
            Some("healthy" | "ready") => Some(true),
            Some(_) => Some(false),
            None => None,
        })
}

fn worker_context(worker: Option<&str>) -> Option<String> {
    let worker = worker?;
    json_number_field(worker, "context_window")
        .or_else(|| json_number_field(worker, "default_context_tokens"))
}

fn worker_looks_cpu_bound(worker: &str) -> bool {
    matches!(
        json_string_field(worker, "runtime_device").as_deref(),
        Some("cpu" | "cpu-vector")
    ) || matches!(
        json_string_field(worker, "runtime_accelerator").as_deref(),
        Some("cpu" | "none")
    )
}

fn worker_has_zero_gpu_layers(worker: &str) -> bool {
    json_number_field(worker, "gpu_layers").as_deref() == Some("0")
}

fn option_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_usize_string_json(value: Option<usize>) -> String {
    value
        .map(|value| json_string(&value.to_string()))
        .unwrap_or_else(|| "null".to_owned())
}

fn option_string_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn json_string_array(values: &[&str]) -> String {
    let values = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advice_blocks_when_quality_is_down() {
        let json = model_pool_advice_json(
            "{\"ok\":true,\"quality_ready\":false,\"quality_context_tokens\":262144,\"quality_context_required_tokens\":262144,\"quality_context_sufficient\":false,\"capacity\":{\"recommendation\":\"restore_quality_gate_first\",\"expansion_allowed\":false,\"healthy_helper_worker_count\":0},\"workers\":[{\"role\":\"quality\",\"status\":\"unreachable\",\"ready\":false,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99}]}",
        );

        assert!(json.contains("\"safe_to_enable_pool_workers\":false"));
        assert!(json.contains("\"next_step\":\"start_or_fix_quality_worker_8686\""));
        assert!(json.contains("\"capacity_policy\":\"one_quality_plus_small_helpers\""));
        assert!(json.contains("\"avoid_extra_12b\":true"));
        assert!(json.contains("先恢复 quality 12B"));
    }

    #[test]
    fn advice_blocks_cpu_fallback() {
        let json = model_pool_advice_json(
            "{\"ok\":true,\"quality_ready\":true,\"quality_context_sufficient\":true,\"capacity\":{\"recommendation\":\"add_summary_worker_first\",\"expansion_allowed\":true},\"workers\":[{\"role\":\"quality\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"cpu\",\"runtime_accelerator\":\"cpu\",\"gpu_layers\":0}]}",
        );

        assert!(json.contains("\"safe_to_enable_pool_workers\":false"));
        assert!(json.contains("\"reason\":\"quality_worker_not_gpu_accelerated\""));
        assert!(json.contains("先修 Metal/GPU"));
    }

    #[test]
    fn advice_blocks_helper_cpu_fallback() {
        let json = model_pool_advice_json(
            "{\"ok\":true,\"quality_ready\":true,\"quality_context_sufficient\":true,\"capacity\":{\"expansion_allowed\":true,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99},{\"role\":\"summary\",\"status\":\"healthy\",\"role_ready\":true,\"runtime_device\":\"cpu\",\"runtime_accelerator\":\"cpu\",\"gpu_layers\":0}]}",
        );

        assert!(json.contains("\"safe_to_enable_pool_workers\":false"));
        assert!(json.contains("\"reason\":\"helper_workers_not_gpu_accelerated\""));
        assert!(json.contains("helper 小模型仍在 CPU/无 GPU 路径(summary)"));
    }

    #[test]
    fn advice_recommends_summary_first_after_quality_ready() {
        let json = model_pool_advice_json(
            "{\"ok\":true,\"quality_ready\":true,\"quality_context_tokens\":262144,\"quality_context_required_tokens\":262144,\"quality_context_sufficient\":true,\"capacity\":{\"recommendation\":\"add_summary_worker_first\",\"expansion_allowed\":true,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99}]}",
        );

        assert!(json.contains("\"safe_to_enable_pool_workers\":true"));
        assert!(json.contains("\"next_step\":\"add_summary_worker_first\""));
        assert!(json.contains("\"quality_runtime_accelerated\":true"));
        assert!(json.contains("\"summary\":false"));
        assert!(json.contains(
            "\"expected_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]"
        ));
        assert!(json.contains(
            "\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]"
        ));
        assert!(json.contains(
            "\"missing_helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"]"
        ));
    }

    #[test]
    fn advice_recognizes_ready_helper_pool() {
        let json = model_pool_advice_json(
            "{\"ok\":true,\"quality_ready\":true,\"quality_context_sufficient\":true,\"capacity\":{\"expansion_allowed\":true,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99},{\"role\":\"summary\",\"status\":\"healthy\",\"role_ready\":true},{\"role\":\"router\",\"status\":\"healthy\",\"role_ready\":true},{\"role\":\"review\",\"status\":\"healthy\",\"role_ready\":true},{\"role\":\"index\",\"status\":\"healthy\",\"role_ready\":true},{\"role\":\"test-gate\",\"status\":\"healthy\",\"role_ready\":true}]}",
        );

        assert!(json.contains(
            "\"next_step\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\""
        ));
        assert!(json.contains("\"router\":true"));
        assert!(json.contains("\"test_gate\":true"));
        assert!(json.contains("\"quality_worker_count\":1"));
        assert!(json.contains("\"helper_worker_count\":5"));
        assert!(json.contains("helper 池已成形"));
    }

    #[test]
    fn advice_treats_summary_and_test_gate_as_partial_pool() {
        let json = model_pool_advice_json(
            "{\"ok\":true,\"quality_ready\":true,\"quality_context_sufficient\":true,\"capacity\":{\"expansion_allowed\":true,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99},{\"role\":\"summary\",\"status\":\"healthy\",\"role_ready\":true},{\"role\":\"test-gate\",\"status\":\"healthy\",\"role_ready\":true}]}",
        );

        assert!(json.contains("\"next_step\":\"add_remaining_helper_roles_one_at_a_time\""));
        assert!(json.contains("\"reason\":\"partial_helper_pool_visible\""));
        assert!(json.contains("\"summary\":true"));
        assert!(json.contains("\"test_gate\":true"));
        assert!(json.contains("\"helper_worker_count\":2"));
        assert!(json.contains("\"missing_helper_roles\":[\"router\",\"review\",\"index\"]"));
    }

    #[test]
    fn advice_blocks_extra_quality_12b_workers() {
        let json = model_pool_advice_json(
            "{\"ok\":true,\"quality_ready\":true,\"quality_context_sufficient\":true,\"capacity\":{\"expansion_allowed\":true,\"quality_runtime_accelerated\":true},\"workers\":[{\"role\":\"quality\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99},{\"role\":\"quality\",\"status\":\"healthy\",\"ready\":true,\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99},{\"role\":\"summary\",\"status\":\"healthy\",\"role_ready\":true}]}",
        );

        assert!(json.contains("\"extra_quality_12b_detected\":true"));
        assert!(json.contains("\"quality_worker_count\":2"));
        assert!(json.contains("\"helper_worker_count\":1"));
        assert!(json.contains("\"safe_to_enable_pool_workers\":false"));
        assert!(json.contains(
            "\"next_step\":\"stop_extra_quality_12b_workers_keep_one_quality_plus_helpers\""
        ));
        assert!(json.contains("\"reason\":\"extra_quality_12b_wastes_shared_apple_memory\""));
        assert!(json.contains("检测到多个 quality 12B"));
    }
}
