use std::path::PathBuf;

use crate::Args;

use super::snapshot::gemma_local_snapshot_asset_failures;

pub(super) fn gemma_business_smoke_preflight_failures_impl(args: &Args) -> Vec<String> {
    let mut failures = Vec::new();

    require_runtime_settings(args, &mut failures);
    require_local_snapshot_assets(args, &mut failures);
    require_trace_gate_paths(args, &mut failures);

    failures
}

fn require_runtime_settings(args: &Args, failures: &mut Vec<String>) {
    require_preflight_condition(
        args.gemma_12b_runtime,
        "Gemma business smoke requires Gemma 4 12B runtime",
        failures,
    );
    require_preflight_condition(
        args.gemma_runtime_token_source.as_deref() == Some("none"),
        "Gemma business smoke requires --token-source none",
        failures,
    );
}

fn require_local_snapshot_assets(args: &Args, failures: &mut Vec<String>) {
    let model_path = PathBuf::from(&args.runtime_metadata.model_id);
    if model_path.exists() {
        failures.extend(gemma_local_snapshot_asset_failures(&model_path));
    } else {
        failures.push(format!(
            "Gemma business smoke requires an existing local snapshot path: {}",
            args.runtime_metadata.model_id
        ));
    }
}

fn require_trace_gate_paths(args: &Args, failures: &mut Vec<String>) {
    require_preflight_condition(
        args.trace_path.is_some(),
        "Gemma business smoke requires a trace file",
        failures,
    );
    require_preflight_condition(
        args.trace_schema_gate_path.is_some(),
        "Gemma business smoke requires a trace schema gate path",
        failures,
    );
    require_preflight_condition(
        args.trace_path.as_ref() == args.trace_schema_gate_path.as_ref(),
        "Gemma business smoke trace schema gate must target the generated trace file",
        failures,
    );
}

fn require_preflight_condition(condition: bool, message: &str, failures: &mut Vec<String>) {
    if !condition {
        failures.push(message.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::require_preflight_condition;

    #[test]
    fn require_preflight_condition_records_false_only() {
        let mut failures = Vec::new();

        require_preflight_condition(true, "true failed", &mut failures);
        require_preflight_condition(false, "false failed", &mut failures);

        assert_eq!(failures, vec!["false failed".to_owned()]);
    }
}
