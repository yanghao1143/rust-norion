use crate::model_service::json::option_str_service_json;

pub(in crate::gemma_business::smoke_report) fn generate_json(
    runtime_model: Option<&str>,
    runtime_token_count: u64,
    runtime_uncertainty_signal: bool,
    answer_preview_json: String,
) -> String {
    format!(
        "{{\"runtime_model\":{},\"runtime_token_count\":{},\"runtime_uncertainty_signal\":{},\"answer_preview\":{}}}",
        option_str_service_json(runtime_model),
        runtime_token_count,
        runtime_uncertainty_signal,
        answer_preview_json
    )
}

#[cfg(test)]
mod tests {
    use super::generate_json;

    #[test]
    fn generate_json_renders_runtime_model_and_preview() {
        assert_eq!(
            generate_json(Some("gemma-12b"), 12, false, "\"ok\"".to_owned()),
            "{\"runtime_model\":\"gemma-12b\",\"runtime_token_count\":12,\"runtime_uncertainty_signal\":false,\"answer_preview\":\"ok\"}"
        );
    }

    #[test]
    fn generate_json_renders_missing_runtime_model_as_null() {
        assert_eq!(
            generate_json(None, 0, true, "\"\"".to_owned()),
            "{\"runtime_model\":null,\"runtime_token_count\":0,\"runtime_uncertainty_signal\":true,\"answer_preview\":\"\"}"
        );
    }
}
