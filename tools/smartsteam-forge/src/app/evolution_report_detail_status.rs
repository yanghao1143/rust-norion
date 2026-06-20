use super::evolution_worker_window_status::{
    next_round_decision_status_lines, worker_window_replacement_report_lines,
};
use super::status_json::{
    bool_value_text, compact_line, json_bool_field, json_object_field, json_object_keys,
    json_string_array_field, json_string_field, json_top_level_object_field, scalar_value,
};

pub(super) fn report_detail_lines(report_json: &str) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(report_gate) = json_top_level_object_field(report_json, "report_gate") {
        lines.push(format!(
            "report_gate passed={}",
            bool_value(report_gate, "passed")
        ));
    }

    if let Some(strict_gate) = json_top_level_object_field(report_json, "strict_report_gate") {
        lines.push(format!(
            "strict_report_gate passed={} failures={}",
            bool_value(strict_gate, "passed"),
            list_value(json_string_array_field(strict_gate, "failures"))
        ));
    }

    if let Some(ledger_gate) = json_top_level_object_field(report_json, "ledger_gate_report_v1") {
        lines.push(format!(
            "ledger_gate allow_next_round={} gate_blocked={} success_rate={}",
            bool_value(ledger_gate, "allow_next_round"),
            bool_value(ledger_gate, "gate_blocked"),
            scalar_value(ledger_gate, "success_rate")
        ));
    }

    if let Some(continuation_gate) =
        json_top_level_object_field(report_json, "continuation_gate_report_v1")
    {
        lines.push(format!(
            "continuation_gate allow_unattended={} gate_blocked={} strict_report_gate_passed={} latest_round={} latest_success={} latest_runtime_response_failure={} historical_runtime_response_failures={} failures={}",
            bool_value(continuation_gate, "allow_unattended_continuation"),
            bool_value(continuation_gate, "gate_blocked"),
            bool_value(continuation_gate, "strict_report_gate_passed"),
            scalar_value(continuation_gate, "latest_round"),
            bool_value(continuation_gate, "latest_success"),
            bool_value(continuation_gate, "latest_runtime_response_failure"),
            scalar_value(continuation_gate, "historical_runtime_response_failures"),
            list_value(json_string_array_field(continuation_gate, "failure_reasons"))
        ));
    }

    if let Some(helper_feedback) =
        json_top_level_object_field(report_json, "helper_stage_feedback_by_role")
    {
        let roles = ordered_helper_roles(json_object_keys(helper_feedback));
        if !roles.is_empty() {
            lines.push(format!(
                "helper_stage feedback_roles={} required_ready={} count={}",
                roles.join(","),
                bool_value_text(helper_roles_cover_strict_gate(&roles)),
                roles.len()
            ));
        }
    }

    if let Some(test_gate) = json_top_level_object_field(report_json, "test_gate") {
        lines.push(format!(
            "test_gate verdict={} validation_command_safety={} failure_kind={} validation_command={}",
            string_value(test_gate, "latest_verdict"),
            string_value(test_gate, "latest_validation_command_safety"),
            json_string_field(test_gate, "latest_failure_kind").unwrap_or_else(|| "none".to_owned()),
            compact_line(&string_value(test_gate, "latest_validation_command"), 120)
        ));
    }

    if let Some(budget) =
        json_top_level_object_field(report_json, "model_pool_budget_fairness_report_v1")
    {
        let workers = json_object_field(budget, "workers");
        lines.push(format!(
            "model_pool_budget_fairness blocked={} allow_pool_expansion={} workers={}/{} feedback_workers={} runtime_tokens={} max_role_runtime_token_share={}",
            bool_value(budget, "budget_fairness_blocked"),
            bool_value(budget, "allow_pool_expansion"),
            workers
                .map(|value| scalar_value(value, "successful"))
                .unwrap_or_else(|| "unknown".to_owned()),
            workers
                .map(|value| scalar_value(value, "total"))
                .unwrap_or_else(|| "unknown".to_owned()),
            workers
                .map(|value| scalar_value(value, "feedback_bearing"))
                .unwrap_or_else(|| "unknown".to_owned()),
            scalar_value(budget, "total_runtime_tokens"),
            scalar_value(budget, "max_role_runtime_token_share")
        ));
    }

    if let Some(alignment) = json_top_level_object_field(report_json, "model_pool_alignment") {
        lines.extend(model_pool_alignment_lines(alignment));
    }

    if let Some(last) = json_top_level_object_field(report_json, "last") {
        lines.extend(latest_model_output_lines(last));
    }

    let model_pool = json_top_level_object_field(report_json, "model_pool")
        .or_else(|| json_object_field(report_json, "model_pool"));
    if let Some(model_pool) = model_pool {
        let workers = json_object_field(model_pool, "workers");
        let healthy = scalar_value(model_pool, "healthy_worker_count");
        let total = scalar_value(model_pool, "worker_count");
        let healthy = if healthy == "unknown" {
            workers
                .map(|value| scalar_value(value, "healthy"))
                .unwrap_or(healthy)
        } else {
            healthy
        };
        let total = if total == "unknown" {
            workers
                .map(|value| scalar_value(value, "total"))
                .unwrap_or(total)
        } else {
            total
        };
        let classification = json_string_field(model_pool, "chain_classification")
            .or_else(|| json_string_field(model_pool, "reason"))
            .unwrap_or_else(|| "unknown".to_owned());
        let reason = json_string_field(model_pool, "launch_block_reason")
            .or_else(|| json_string_field(model_pool, "reason"))
            .unwrap_or_else(|| "unknown".to_owned());
        lines.push(format!(
            "model_pool launch_allowed={} classification={} workers={healthy}/{total} min_context_tokens={} reason={}",
            bool_value(model_pool, "launch_allowed"),
            classification,
            scalar_value(model_pool, "min_context_tokens"),
            reason
        ));

        if let Some(capacity) = json_object_field(model_pool, "capacity") {
            lines.push(format!(
                "model_pool_capacity helpers={}/{} metal_workers={} cpu_workers={} quality_accelerated={}",
                scalar_value(capacity, "healthy_helper_worker_count"),
                scalar_value(capacity, "helper_worker_count"),
                scalar_value(capacity, "metal_worker_count"),
                scalar_value(capacity, "cpu_worker_count"),
                bool_value(capacity, "quality_runtime_accelerated")
            ));
        }
    }

    lines.extend(next_round_decision_status_lines(Some(report_json)));
    lines.extend(worker_window_replacement_report_lines(report_json));

    lines
}

pub(super) fn latest_model_output_lines(record_json: &str) -> Vec<String> {
    let answer = json_string_field(record_json, "answer").unwrap_or_default();
    if answer.trim().is_empty() {
        return Vec::new();
    }

    vec![
        format!(
            "latest_model_output round={} model={} runtime_tokens={} elapsed_ms={} feedback={} self_improve_passed={}",
            scalar_value(record_json, "round"),
            string_value(record_json, "runtime_model"),
            scalar_value(record_json, "runtime_tokens"),
            scalar_value(record_json, "elapsed_ms"),
            scalar_value(record_json, "feedback_applied"),
            scalar_value(record_json, "self_improve_passed")
        ),
        format!("latest_answer_preview={}", compact_line(&answer, 280)),
    ]
}

fn model_pool_alignment_lines(alignment: &str) -> Vec<String> {
    let quality_workers = json_object_field(alignment, "quality_workers");
    let helper_workers = json_object_field(alignment, "helper_workers");
    let route_blocked_or_failed = list_value(json_string_array_field(
        alignment,
        "route_blocked_or_failed",
    ));
    let route_dependency_failures = list_value(json_string_array_field(
        alignment,
        "route_dependency_failures",
    ));

    let mut lines = vec![format!(
        "model_pool_alignment ok={} quality_workers={}/{}/{} helper_workers={}/{}/{} route_blocked_or_failed={} route_dependency_failures={}",
        bool_value(alignment, "alignment_ok"),
        object_scalar_value(quality_workers, "manifest"),
        object_scalar_value(quality_workers, "status"),
        object_scalar_value(quality_workers, "max"),
        object_scalar_value(helper_workers, "manifest"),
        object_scalar_value(helper_workers, "status"),
        object_scalar_value(helper_workers, "target"),
        route_blocked_or_failed,
        route_dependency_failures
    )];

    if json_bool_field(alignment, "alignment_ok") == Some(false) {
        lines.push(format!(
            "model_pool_alignment_failures missing_manifest_helper_roles={} missing_status_helper_roles={} missing_status_roles={} unplanned_status_roles={} missing_inputs={}",
            list_value(json_string_array_field(alignment, "missing_manifest_helper_roles")),
            list_value(json_string_array_field(alignment, "missing_status_helper_roles")),
            list_value(json_string_array_field(alignment, "missing_status_roles")),
            list_value(json_string_array_field(alignment, "unplanned_status_roles")),
            list_value(json_string_array_field(alignment, "missing_inputs"))
        ));
    }

    lines
}

fn ordered_helper_roles(mut roles: Vec<String>) -> Vec<String> {
    const ORDER: [&str; 5] = ["summary", "router", "review", "index", "test-gate"];
    roles.sort_by(|left, right| {
        let left_rank = helper_role_rank(left, &ORDER);
        let right_rank = helper_role_rank(right, &ORDER);
        left_rank
            .cmp(&right_rank)
            .then_with(|| left.as_str().cmp(right.as_str()))
    });
    roles
}

fn helper_role_rank(role: &str, order: &[&str]) -> usize {
    order
        .iter()
        .position(|expected| role.eq_ignore_ascii_case(expected))
        .unwrap_or(order.len())
}

fn helper_roles_cover_strict_gate(roles: &[String]) -> bool {
    ["summary", "router", "review", "index", "test-gate"]
        .iter()
        .all(|expected| roles.iter().any(|role| role.eq_ignore_ascii_case(expected)))
}

fn bool_value(object: &str, field: &str) -> &'static str {
    match json_bool_field(object, field) {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

fn string_value(object: &str, field: &str) -> String {
    json_string_field(object, field).unwrap_or_else(|| "unknown".to_owned())
}

fn object_scalar_value(object: Option<&str>, field: &str) -> String {
    object
        .map(|object| scalar_value(object, field))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn list_value(values: Option<Vec<String>>) -> String {
    values
        .filter(|values| !values.is_empty())
        .map(|values| values.join(","))
        .unwrap_or_else(|| "none".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_details_use_top_level_pool_and_gate_reports() {
        let report = r#"{
            "remote_chain": {
                "model_pool": {
                    "available": true,
                    "reason": "nested",
                    "worker_count": 1,
                    "healthy_worker_count": 1
                }
            },
            "model_pool": {
                "available": true,
                "launch_allowed": true,
                "reason": "ready",
                "min_context_tokens": 4096,
                "workers": {"total": 6, "healthy": 6},
                "capacity": {
                    "helper_worker_count": 5,
                    "healthy_helper_worker_count": 5,
                    "metal_worker_count": 6,
                    "cpu_worker_count": 0,
                    "quality_runtime_accelerated": true
                }
            },
            "ledger_gate_report_v1": {
                "allow_next_round": true,
                "gate_blocked": false,
                "success_rate": 1.0
            },
            "strict_report_gate": {
                "passed": false,
                "failures": ["runtime response failures 1 above maximum 0"]
            },
            "continuation_gate_report_v1": {
                "allow_unattended_continuation": true,
                "gate_blocked": false,
                "failure_reasons": [],
                "strict_report_gate_passed": false,
                "latest_round": 10,
                "latest_success": true,
                "latest_runtime_response_failure": false,
                "historical_runtime_response_failures": 1
            },
            "helper_stage_feedback_by_role": {
                "test-gate": ["verdict: pass"],
                "index": ["clean_gist: keep"],
                "summary": ["memory_update: keep"],
                "review": ["risk: low"],
                "router": ["route_intent: index"]
            },
            "test_gate": {
                "latest_verdict": "pass",
                "latest_validation_command": "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml",
                "latest_validation_command_safety": "safe",
                "latest_failure_kind": null
            },
            "model_pool_budget_fairness_report_v1": {
                "workers": {"total": 6, "successful": 6, "feedback_bearing": 6},
                "total_runtime_tokens": 423,
                "max_role_runtime_token_share": 0.463,
                "budget_fairness_blocked": false,
                "allow_pool_expansion": true
            },
            "model_pool_alignment": {
                "alignment_ok": true,
                "quality_workers": {"manifest": 1, "status": 1, "max": 1},
                "helper_workers": {"manifest": 5, "status": 5, "target": 5},
                "missing_manifest_helper_roles": [],
                "missing_status_helper_roles": [],
                "missing_status_roles": [],
                "unplanned_status_roles": [],
                "route_blocked_or_failed": [],
                "route_dependency_failures": [],
                "missing_inputs": []
            },
            "last": {
                "round": 10,
                "runtime_model": "google/gemma-4-12B-it",
                "runtime_tokens": 64,
                "elapsed_ms": 50697,
                "feedback_applied": 4,
                "self_improve_passed": true,
                "answer": "**Improvement Candidate:** Introduce a context_compression_ratio check in the router stage."
            },
            "report_gate": {"passed": true, "failures": []}
        }"#;

        let details = report_detail_lines(report).join("\n");

        assert!(details.contains("report_gate passed=true"));
        assert!(details.contains(
            "strict_report_gate passed=false failures=runtime response failures 1 above maximum 0"
        ));
        assert!(details.contains(
            "continuation_gate allow_unattended=true gate_blocked=false strict_report_gate_passed=false latest_round=10 latest_success=true latest_runtime_response_failure=false historical_runtime_response_failures=1 failures=none"
        ));
        assert!(
            details
                .contains("ledger_gate allow_next_round=true gate_blocked=false success_rate=1.0")
        );
        assert!(details.contains(
            "helper_stage feedback_roles=summary,router,review,index,test-gate required_ready=true count=5"
        ));
        assert!(details.contains("test_gate verdict=pass validation_command_safety=safe failure_kind=none validation_command=cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"));
        assert!(details.contains(
            "model_pool_budget_fairness blocked=false allow_pool_expansion=true workers=6/6 feedback_workers=6 runtime_tokens=423 max_role_runtime_token_share=0.463"
        ));
        assert!(details.contains(
            "model_pool_alignment ok=true quality_workers=1/1/1 helper_workers=5/5/5 route_blocked_or_failed=none route_dependency_failures=none"
        ));
        assert!(details.contains("latest_model_output round=10 model=google/gemma-4-12B-it runtime_tokens=64 elapsed_ms=50697 feedback=4 self_improve_passed=true"));
        assert!(details.contains("latest_answer_preview=**Improvement Candidate:** Introduce a context_compression_ratio check in the router stage."));
        assert!(details.contains(
            "model_pool launch_allowed=true classification=ready workers=6/6 min_context_tokens=4096 reason=ready"
        ));
        assert!(details.contains(
            "model_pool_capacity helpers=5/5 metal_workers=6 cpu_workers=0 quality_accelerated=true"
        ));
        assert!(!details.contains("reason=nested"));
    }

    #[test]
    fn report_details_surface_model_pool_alignment_failures() {
        let report = r#"{
            "model_pool_alignment": {
                "alignment_ok": false,
                "quality_workers": {"manifest": 1, "status": 1, "max": 1},
                "helper_workers": {"manifest": 5, "status": 4, "target": 5},
                "missing_manifest_helper_roles": [],
                "missing_status_helper_roles": ["router"],
                "missing_status_roles": ["router"],
                "unplanned_status_roles": [],
                "route_blocked_or_failed": [],
                "route_dependency_failures": ["index:dependency_health_failed:required_roles=summary,router missing_roles=router unhealthy_roles=summary:tcp_only status_roles=quality,summary,index"],
                "missing_inputs": []
            },
            "report_gate": {"passed": false, "failures": ["model_pool_alignment"]}
        }"#;

        let details = report_detail_lines(report).join("\n");

        assert!(details.contains("report_gate passed=false"));
        assert!(details.contains(
            "model_pool_alignment ok=false quality_workers=1/1/1 helper_workers=5/4/5 route_blocked_or_failed=none route_dependency_failures=index:dependency_health_failed:required_roles=summary,router missing_roles=router unhealthy_roles=summary:tcp_only status_roles=quality,summary,index"
        ));
        assert!(details.contains(
            "model_pool_alignment_failures missing_manifest_helper_roles=none missing_status_helper_roles=router missing_status_roles=router unplanned_status_roles=none missing_inputs=none"
        ));
    }
}
