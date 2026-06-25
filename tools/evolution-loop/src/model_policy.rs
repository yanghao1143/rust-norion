pub(crate) fn is_gpt5_series_model(model: &str) -> bool {
    let lower = model.trim().to_ascii_lowercase();
    lower.contains("gpt-5") || lower.contains("gpt5")
}

pub(crate) fn allowed_non_gpt5_models(value: &str) -> Vec<String> {
    value
        .split([',', ';', '\n', '\r'])
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .filter(|model| !is_gpt5_series_model(model))
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn model_priority(task_kind: &str, model: &str) -> u8 {
    let task = task_kind.trim().to_ascii_lowercase();
    let model = model.trim().to_ascii_lowercase();
    if model.contains("kimi-k2.6") {
        return 90;
    }
    if model.contains("qwen3.5-397b") {
        return if matches!(
            task.as_str(),
            "quality" | "review" | "test-gate" | "coding" | "code"
        ) {
            0
        } else {
            20
        };
    }
    if model.contains("qwen3-next-80b-a3b-instruct") {
        return if matches!(task.as_str(), "summary" | "router" | "index") {
            0
        } else {
            10
        };
    }
    30
}

pub(crate) fn sorted_allowed_models(value: &str, task_kind: &str) -> Vec<String> {
    let mut models = allowed_non_gpt5_models(value);
    models.sort_by_key(|model| model_priority(task_kind, model));
    models
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_gpt5_series_models_from_allowed_list() {
        let models =
            allowed_non_gpt5_models("qwen3-coder,gpt-5,gpt5-mini,openai/gpt-5.1,deepseek-chat");

        assert_eq!(models, vec!["qwen3-coder", "deepseek-chat"]);
    }

    #[test]
    fn sorts_known_newapi_models_by_task_shape() {
        let value = concat!(
            "moonshotai/kimi-k2.6,",
            "qwen/qwen3.5-397b-a17b,",
            "qwen/qwen3-next-80b-a3b-instruct"
        );

        assert_eq!(
            sorted_allowed_models(value, "summary"),
            vec![
                "qwen/qwen3-next-80b-a3b-instruct",
                "qwen/qwen3.5-397b-a17b",
                "moonshotai/kimi-k2.6",
            ]
        );
        assert_eq!(
            sorted_allowed_models(value, "review"),
            vec![
                "qwen/qwen3.5-397b-a17b",
                "qwen/qwen3-next-80b-a3b-instruct",
                "moonshotai/kimi-k2.6",
            ]
        );
    }

    #[test]
    fn detects_gpt5_series_spelling_variants() {
        assert!(is_gpt5_series_model("gpt-5"));
        assert!(is_gpt5_series_model("openai/gpt-5-mini"));
        assert!(is_gpt5_series_model("gpt5-proxy"));
        assert!(!is_gpt5_series_model("gpt-4.1"));
        assert!(!is_gpt5_series_model("qwen3-coder"));
    }
}
