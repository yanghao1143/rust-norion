use std::path::Path;

pub(super) fn require_snapshot_asset(
    condition: bool,
    model_path: &Path,
    message: &str,
    failures: &mut Vec<String>,
) {
    if !condition {
        failures.push(format!(
            "Gemma business smoke local snapshot {message}: {}",
            model_path.display()
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::require_snapshot_asset;

    #[test]
    fn require_snapshot_asset_records_false_only() {
        let mut failures = Vec::new();
        let model_path = Path::new("models/gemma-12b");

        require_snapshot_asset(true, model_path, "present", &mut failures);
        require_snapshot_asset(false, model_path, "missing tokenizer asset", &mut failures);

        assert_eq!(
            failures,
            vec![
                "Gemma business smoke local snapshot missing tokenizer asset: models/gemma-12b"
                    .to_owned()
            ]
        );
    }
}
