use super::super::fields::json_escape;

pub(super) fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn option_i32_json(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn option_u8_json(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn option_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn option_f32_json(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn option_string_json(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn option_owned_string_json(value: Option<&str>) -> String {
    option_string_json(value)
}

pub(super) fn string_array_json(items: &[String]) -> String {
    let values = items
        .iter()
        .map(|item| format!("\"{}\"", json_escape(item)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

pub(super) fn u64_array_json(items: &[u64]) -> String {
    let values = items
        .iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}
