use std::collections::BTreeSet;

use crate::app::status_json::json_string_literal;

pub(super) fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub(super) fn role_set_text(roles: &BTreeSet<String>) -> String {
    if roles.is_empty() {
        return "none".to_owned();
    }
    roles.iter().cloned().collect::<Vec<_>>().join(",")
}

pub(super) fn json_role_set(roles: &BTreeSet<String>) -> String {
    let values = roles.iter().cloned().collect::<Vec<_>>();
    json_string_array(&values)
}

pub(super) fn json_string_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| json_string_literal(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

pub(super) fn list_text(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn bool_text_is_stable_for_machine_fields() {
        assert_eq!(bool_text(true), "true");
        assert_eq!(bool_text(false), "false");
    }

    #[test]
    fn role_set_text_keeps_sorted_roles_or_none() {
        assert_eq!(role_set_text(&BTreeSet::new()), "none");
        assert_eq!(
            role_set_text(&set(&["summary", "quality", "review"])),
            "quality,review,summary"
        );
    }

    #[test]
    fn list_text_keeps_input_order_or_none() {
        assert_eq!(list_text(&[]), "none");
        assert_eq!(
            list_text(&["summary".to_owned(), "router".to_owned()]),
            "summary,router"
        );
    }

    #[test]
    fn json_string_array_escapes_values() {
        assert_eq!(
            json_string_array(&["summary".to_owned(), "router\"x".to_owned()]),
            "[\"summary\",\"router\\\"x\"]"
        );
    }

    #[test]
    fn json_role_set_sorts_and_renders_json_array() {
        assert_eq!(
            json_role_set(&set(&["summary", "quality"])),
            "[\"quality\",\"summary\"]"
        );
    }
}
