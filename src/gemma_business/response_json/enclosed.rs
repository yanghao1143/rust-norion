mod body;
mod string_array;

use body::{array_body, object_body};
use string_array::parse_json_string_array;

use crate::model_service::json::json_bool_field;

pub(in crate::gemma_business) fn response_object_bool_field(
    body: &str,
    object: &str,
    field: &str,
) -> bool {
    response_optional_object_bool_field(body, object, field).unwrap_or(false)
}

pub(in crate::gemma_business) fn response_optional_object_bool_field(
    body: &str,
    object: &str,
    field: &str,
) -> Option<bool> {
    object_body(body, object).and_then(|object_body| json_bool_field(object_body, field))
}

pub(in crate::gemma_business) fn response_string_array_field(
    body: &str,
    field: &str,
) -> Vec<String> {
    array_body(body, field)
        .and_then(parse_json_string_array)
        .unwrap_or_default()
}

pub(in crate::gemma_business) fn response_empty_array_field(body: &str, field: &str) -> bool {
    array_body(body, field)
        .and_then(|array_body| array_body.strip_prefix('[')?.strip_suffix(']'))
        .map(|array_items| array_items.trim().is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{
        response_empty_array_field, response_object_bool_field, response_string_array_field,
    };

    #[test]
    fn response_object_bool_field_reads_spaced_nested_gate_results() {
        let body = r#"{ "state_gate" : { "passed" : true, "note": "brace } inside" } }"#;

        assert!(response_object_bool_field(body, "state_gate", "passed"));
        assert!(!response_object_bool_field(body, "trace_gate", "passed"));
    }

    #[test]
    fn response_string_array_field_reads_escaped_string_items() {
        let body = r#"{"business_cases":["rust","quote\"case","line\ncase"]}"#;

        assert_eq!(
            response_string_array_field(body, "business_cases"),
            vec![
                "rust".to_owned(),
                "quote\"case".to_owned(),
                "line\ncase".to_owned()
            ]
        );
        assert_eq!(
            response_string_array_field(body, "missing"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn response_empty_array_field_accepts_spacing_and_rejects_values() {
        assert!(response_empty_array_field(
            "{\"failures\" : [ ]}",
            "failures"
        ));
        assert!(!response_empty_array_field(
            "{\"failures\":[\"bad\"]}",
            "failures"
        ));
        assert!(!response_empty_array_field("{}", "failures"));
    }
}
