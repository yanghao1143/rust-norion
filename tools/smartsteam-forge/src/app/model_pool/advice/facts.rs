use std::collections::BTreeSet;

use model_pool_advice_core::{HELPER_ROLES, ModelPoolFacts};

pub(super) fn facts_from_summary(summary: &str) -> ModelPoolFacts {
    let quality_line = find_worker_line(summary, "quality");
    let worker_roles = worker_roles(summary);
    let quality_worker_count = worker_roles
        .iter()
        .filter(|role| role.as_str() == "quality")
        .count();
    let helper_worker_count = worker_roles
        .iter()
        .filter(|role| {
            matches!(
                role.as_str(),
                "summary" | "router" | "review" | "test-gate" | "index"
            )
        })
        .count();
    ModelPoolFacts {
        quality_ready: bool_value(summary, "quality_ready"),
        quality_context_sufficient: bool_value(summary, "quality_context_sufficient"),
        quality_context_tokens: value(summary, "quality_context_tokens"),
        quality_required_context_tokens: value(summary, "quality_context_required_tokens"),
        quality_runtime_accelerated: capacity_value(summary, "quality_runtime_accelerated")
            .as_deref()
            .and_then(parse_bool),
        capacity_recommendation: capacity_value(summary, "recommendation"),
        expansion_allowed: capacity_value(summary, "expansion_allowed")
            .as_deref()
            .and_then(parse_bool),
        healthy_helper_worker_count: capacity_value(summary, "healthy_helper_worker_count")
            .and_then(|value| value.parse::<usize>().ok()),
        has_summary: find_worker_line(summary, "summary").is_some(),
        has_router: find_worker_line(summary, "router").is_some(),
        has_review: find_worker_line(summary, "review").is_some(),
        has_test_gate: find_worker_line(summary, "test-gate").is_some(),
        has_index: find_worker_line(summary, "index").is_some(),
        quality_worker_count,
        helper_worker_count,
        quality_cpu_fallback: quality_line.is_some_and(worker_looks_cpu_bound),
        quality_zero_gpu_layers: quality_line.is_some_and(worker_has_zero_gpu_layers),
        helper_cpu_or_no_gpu_roles: helper_cpu_or_no_gpu_roles(summary),
        unknown_runtime_worker_count: capacity_value(summary, "unknown_runtime_worker_count")
            .and_then(|value| value.parse::<usize>().ok()),
        ..ModelPoolFacts::default()
    }
}

fn value(summary: &str, key: &str) -> Option<String> {
    summary
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn bool_value(summary: &str, key: &str) -> Option<bool> {
    value(summary, key).and_then(|value| parse_bool(&value))
}

fn capacity_value(summary: &str, key: &str) -> Option<String> {
    summary
        .lines()
        .find(|line| line.starts_with("capacity "))
        .and_then(|line| pair_value(line, key))
}

fn find_worker_line<'a>(summary: &'a str, role: &str) -> Option<&'a str> {
    summary.lines().find(|line| {
        line.starts_with("worker ") && pair_value(line, "role").as_deref() == Some(role)
    })
}

fn worker_roles(summary: &str) -> Vec<String> {
    summary
        .lines()
        .filter(|line| line.starts_with("worker "))
        .filter_map(|line| pair_value(line, "role"))
        .collect()
}

fn helper_cpu_or_no_gpu_roles(summary: &str) -> Vec<String> {
    let mut roles = BTreeSet::new();
    for line in summary.lines().filter(|line| line.starts_with("worker ")) {
        let Some(role) = pair_value(line, "role") else {
            continue;
        };
        if !is_helper_role(&role) {
            continue;
        }
        if worker_looks_cpu_bound(line) || worker_has_zero_gpu_layers(line) {
            roles.insert(role);
        }
    }

    if roles.is_empty()
        && let Some(runtime_shape_roles) = summary
            .lines()
            .find(|line| line.starts_with("runtime_shape "))
            .and_then(|line| pair_value(line, "cpu_or_no_gpu_roles"))
    {
        for role in runtime_shape_roles
            .split(',')
            .map(str::trim)
            .filter(|role| is_helper_role(role))
        {
            roles.insert(role.to_owned());
        }
    }

    roles.into_iter().collect()
}

fn is_helper_role(role: &str) -> bool {
    HELPER_ROLES.contains(&role)
}

fn pair_value(line: &str, key: &str) -> Option<String> {
    line.split_whitespace()
        .find_map(|part| part.strip_prefix(&format!("{key}=")))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn worker_looks_cpu_bound(line: &str) -> bool {
    matches!(
        pair_value(line, "runtime_device").as_deref(),
        Some("cpu" | "cpu-vector")
    ) || matches!(
        pair_value(line, "runtime_accelerator").as_deref(),
        Some("cpu" | "none")
    )
}

fn worker_has_zero_gpu_layers(line: &str) -> bool {
    pair_value(line, "gpu_layers").as_deref() == Some("0")
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facts_parse_quality_capacity_and_helper_workers() {
        let facts = facts_from_summary(
            "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\nquality_context_tokens=262144\nquality_context_required_tokens=262144\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_summary_worker_first healthy_helper_worker_count=2 unknown_runtime_worker_count=1 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99\nworker role=summary status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=80\nworker role=test-gate status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=80",
        );

        assert_eq!(facts.quality_ready, Some(true));
        assert_eq!(facts.quality_context_sufficient, Some(true));
        assert_eq!(facts.quality_context_tokens.as_deref(), Some("262144"));
        assert_eq!(
            facts.quality_required_context_tokens.as_deref(),
            Some("262144")
        );
        assert_eq!(facts.quality_runtime_accelerated, Some(true));
        assert_eq!(
            facts.capacity_recommendation.as_deref(),
            Some("add_summary_worker_first")
        );
        assert_eq!(facts.expansion_allowed, Some(true));
        assert_eq!(facts.healthy_helper_worker_count, Some(2));
        assert_eq!(facts.unknown_runtime_worker_count, Some(1));
        assert!(facts.has_summary);
        assert!(!facts.has_router);
        assert!(!facts.has_review);
        assert!(facts.has_test_gate);
        assert!(!facts.has_index);
        assert_eq!(facts.quality_worker_count, 1);
        assert_eq!(facts.helper_worker_count, 2);
        assert!(!facts.quality_cpu_fallback);
        assert!(!facts.quality_zero_gpu_layers);
    }

    #[test]
    fn facts_detect_cpu_bound_quality_worker() {
        let facts = facts_from_summary(
            "SmartSteam model pool status\nquality_ready=true\nworker role=quality status=healthy ready=true runtime_device=cpu runtime_accelerator=cpu gpu_layers=0",
        );

        assert!(facts.quality_cpu_fallback);
        assert!(facts.quality_zero_gpu_layers);
    }

    #[test]
    fn facts_detect_cpu_or_no_gpu_helper_roles_from_worker_lines() {
        let facts = facts_from_summary(
            "SmartSteam model pool status\nquality_ready=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99\nworker role=summary status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=80\nworker role=review status=healthy ready=true runtime_device=cpu runtime_accelerator=none gpu_layers=0\nworker role=index status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=0",
        );

        assert_eq!(
            facts.helper_cpu_or_no_gpu_roles,
            vec!["index".to_owned(), "review".to_owned()]
        );
    }

    #[test]
    fn facts_detect_cpu_or_no_gpu_helper_roles_from_runtime_shape_fallback() {
        let facts = facts_from_summary(
            "SmartSteam model pool status\nquality_ready=true\nruntime_shape workers=3 metal_workers=1 cpu_or_no_gpu_workers=2 zero_gpu_layer_workers=1 unknown_runtime_workers=0 cpu_or_no_gpu_roles=review,index,quality",
        );

        assert_eq!(
            facts.helper_cpu_or_no_gpu_roles,
            vec!["index".to_owned(), "review".to_owned()]
        );
    }

    #[test]
    fn facts_ignore_empty_values_and_invalid_booleans() {
        let facts = facts_from_summary(
            "SmartSteam model pool status\nquality_ready=yes\nquality_context_tokens=\ncapacity expansion_allowed=maybe healthy_helper_worker_count=nope\nworker role= status=healthy",
        );

        assert_eq!(facts.quality_ready, None);
        assert_eq!(facts.quality_context_tokens, None);
        assert_eq!(facts.expansion_allowed, None);
        assert_eq!(facts.healthy_helper_worker_count, None);
        assert_eq!(facts.quality_worker_count, 0);
        assert_eq!(facts.helper_worker_count, 0);
    }
}
