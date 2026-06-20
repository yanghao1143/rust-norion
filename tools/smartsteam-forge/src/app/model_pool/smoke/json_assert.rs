use crate::app::status_json::{
    bool_value_text, require_json_bool_equals, require_json_string_equals,
};

pub(super) fn require_json_string(
    object: &str,
    field: &str,
    expected: &str,
    label: &str,
) -> Result<(), String> {
    require_json_string_equals(object, field, expected, label)
}

pub(super) fn require_json_bool(
    object: &str,
    field: &str,
    expected: bool,
    label: &str,
) -> Result<(), String> {
    require_json_bool_equals(object, field, expected, label)
}

pub(super) fn bool_text(value: bool) -> &'static str {
    bool_value_text(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_assert_accepts_expected_fields() {
        let value = "{\"schema\":\"demo.v1\",\"ok\":true,\"blocked\":false}";

        require_json_string(value, "schema", "demo.v1", "demo schema").unwrap();
        require_json_bool(value, "ok", true, "demo ok").unwrap();
        require_json_bool(value, "blocked", false, "demo blocked").unwrap();
    }

    #[test]
    fn json_assert_reports_mismatch_and_missing_fields() {
        let value = "{\"schema\":\"demo.v2\",\"ok\":false}";

        assert!(
            require_json_string(value, "schema", "demo.v1", "demo schema")
                .unwrap_err()
                .contains("expected \"demo.v1\"")
        );
        assert!(
            require_json_bool(value, "ok", true, "demo ok")
                .unwrap_err()
                .contains("expected true, got false")
        );
        assert!(
            require_json_bool(value, "missing", false, "demo missing")
                .unwrap_err()
                .contains("missing missing")
        );
    }
}
