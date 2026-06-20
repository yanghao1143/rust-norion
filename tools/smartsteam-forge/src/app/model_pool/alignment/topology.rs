use std::collections::BTreeSet;

use model_pool_advice_core::{ModelPoolFacts, missing_helper_roles as core_missing_helper_roles};

use super::MODEL_POOL_SMOKE_TASK_KINDS;

pub(super) fn roles_from_lines(summary: &str, line_prefix: &str) -> BTreeSet<String> {
    summary
        .lines()
        .filter(|line| line.starts_with(line_prefix))
        .filter_map(|line| pair_value(line, "role"))
        .collect()
}

pub(super) fn role_line_count(summary: &str, line_prefix: &str, role: &str) -> usize {
    summary
        .lines()
        .filter(|line| line.starts_with(line_prefix))
        .filter(|line| pair_value(line, "role").as_deref() == Some(role))
        .count()
}

pub(super) fn helper_line_count(summary: &str, line_prefix: &str) -> usize {
    summary
        .lines()
        .filter(|line| line.starts_with(line_prefix))
        .filter(|line| pair_value(line, "role").is_some_and(|role| is_helper_role(&role)))
        .count()
}

pub(super) fn missing_helper_roles(roles: &BTreeSet<String>) -> Vec<String> {
    core_missing_helper_roles(&helper_facts_from_roles(roles))
        .into_iter()
        .map(str::to_owned)
        .collect()
}

pub(super) fn missing_smoke_tasks(values: &BTreeSet<String>) -> Vec<String> {
    MODEL_POOL_SMOKE_TASK_KINDS
        .iter()
        .filter(|value| !values.contains(**value))
        .map(|value| (*value).to_owned())
        .collect()
}

pub(super) fn unexpected_smoke_tasks(values: &BTreeSet<String>) -> Vec<String> {
    values
        .iter()
        .filter(|value| !is_helper_role(value))
        .cloned()
        .collect()
}

pub(super) fn unexpected_roles(roles: &BTreeSet<String>) -> Vec<String> {
    roles
        .iter()
        .filter(|role| role.as_str() != "quality" && !is_helper_role(role))
        .cloned()
        .collect()
}

fn helper_facts_from_roles(roles: &BTreeSet<String>) -> ModelPoolFacts {
    ModelPoolFacts {
        has_summary: roles.contains("summary"),
        has_router: roles.contains("router"),
        has_review: roles.contains("review"),
        has_index: roles.contains("index"),
        has_test_gate: roles.contains("test-gate"),
        ..ModelPoolFacts::default()
    }
}

fn is_helper_role(role: &str) -> bool {
    MODEL_POOL_SMOKE_TASK_KINDS.contains(&role)
}

fn pair_value(line: &str, key: &str) -> Option<String> {
    line.split_whitespace()
        .find_map(|part| part.strip_prefix(&format!("{key}=")))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn roles_from_lines_extracts_matching_nonempty_roles() {
        let summary = concat!(
            "manifest_worker role=summary port=8687\n",
            "manifest_worker role= port=8688\n",
            "worker role=router status=healthy\n",
            "manifest_worker role=quality port=8686"
        );

        assert_eq!(
            roles_from_lines(summary, "manifest_worker "),
            set(&["quality", "summary"])
        );
    }

    #[test]
    fn role_line_count_counts_matching_prefixed_role_lines() {
        let summary = concat!(
            "worker role=quality status=healthy\n",
            "worker role=summary status=healthy\n",
            "worker role=quality status=healthy\n",
            "manifest_worker role=quality port=8686"
        );

        assert_eq!(role_line_count(summary, "worker ", "quality"), 2);
    }

    #[test]
    fn helper_line_count_counts_known_helpers_only() {
        let summary = concat!(
            "worker role=quality status=healthy\n",
            "worker role=summary status=healthy\n",
            "worker role=router status=healthy\n",
            "worker role=explore status=healthy"
        );

        assert_eq!(helper_line_count(summary, "worker "), 2);
    }

    #[test]
    fn missing_helper_roles_preserves_core_order() {
        assert_eq!(
            missing_helper_roles(&set(&["summary", "index"])),
            vec!["router", "review", "test-gate"]
        );
    }

    #[test]
    fn route_task_sets_report_missing_and_unexpected_values() {
        let tasks = set(&["summary", "router", "explore"]);

        assert_eq!(
            missing_smoke_tasks(&tasks),
            vec!["review", "index", "test-gate"]
        );
        assert_eq!(unexpected_smoke_tasks(&tasks), vec!["explore"]);
    }

    #[test]
    fn unexpected_roles_allows_quality_and_helpers_only() {
        let roles = set(&["quality", "summary", "router", "explore"]);

        assert_eq!(unexpected_roles(&roles), vec!["explore"]);
    }

    #[test]
    fn pair_value_trims_values_and_ignores_empty_values() {
        assert_eq!(
            pair_value("worker role=summary status=healthy", "role"),
            Some("summary".to_owned())
        );
        assert_eq!(pair_value("worker role= status=healthy", "role"), None);
        assert_eq!(pair_value("worker status=healthy", "role"), None);
    }
}
