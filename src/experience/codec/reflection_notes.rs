use crate::reflection::{ReflectionIssue, ReflectionSeverity};

use super::fields::sanitize_control_part;

pub(super) fn serialize_reflection_issues(issues: &[ReflectionIssue]) -> String {
    issues
        .iter()
        .map(|issue| {
            [
                sanitize_control_part(&issue.code),
                issue.severity.as_str().to_owned(),
                sanitize_control_part(&issue.detail),
            ]
            .join("\u{1f}")
        })
        .collect::<Vec<_>>()
        .join("\u{1e}")
}

pub(super) fn deserialize_reflection_issues(value: &str) -> Vec<ReflectionIssue> {
    if value.is_empty() {
        return Vec::new();
    }

    value
        .split('\u{1e}')
        .filter_map(|item| {
            let fields = item.split('\u{1f}').collect::<Vec<_>>();
            if fields.len() != 3 {
                return None;
            }

            Some(ReflectionIssue::new(
                fields[0],
                fields[1].parse::<ReflectionSeverity>().ok()?,
                fields[2],
            ))
        })
        .collect()
}

pub(super) fn serialize_revision_actions(actions: &[String]) -> String {
    actions
        .iter()
        .map(|action| sanitize_control_part(action))
        .collect::<Vec<_>>()
        .join("\u{1e}")
}

pub(super) fn deserialize_revision_actions(value: &str) -> Vec<String> {
    if value.is_empty() {
        Vec::new()
    } else {
        value.split('\u{1e}').map(ToOwned::to_owned).collect()
    }
}
