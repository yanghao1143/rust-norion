use super::fields::{
    contract_value, extract_json_bool_field, extract_json_nullable_f32_field,
    extract_json_nullable_string_field, extract_json_nullable_u64_field,
    extract_json_string_array_field, extract_json_string_field, extract_json_usize_field,
    has_non_empty_trace_text, json_object_after_field, split_contract_adapters,
};

pub(super) fn evaluate_trace_adapter_observations(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let observations =
        json_object_after_field(line, "runtime_adapter_observations").unwrap_or(line);
    let runtime_diagnostics = json_object_after_field(line, "runtime_diagnostics").unwrap_or(line);
    let observation_count =
        extract_json_usize_field(observations, "observation_count").unwrap_or(0);
    let best_adapter = extract_json_nullable_string_field(observations, "best_adapter");
    let best_score = extract_json_nullable_f32_field(observations, "best_score");
    let best_reward = extract_json_nullable_f32_field(observations, "best_reward");
    let best_quality = extract_json_nullable_f32_field(observations, "best_quality");
    let best_experience_id = extract_json_nullable_u64_field(observations, "best_experience_id");
    let selection_mismatch = extract_json_bool_field(observations, "selection_mismatch");
    let selected_adapter = extract_json_string_field(runtime_diagnostics, "selected_adapter");
    let adapter_stream_trace_id =
        extract_json_nullable_string_field(runtime_diagnostics, "adapter_stream_trace_id");
    let adapter_stream_gate_summary_digest = extract_json_nullable_string_field(
        runtime_diagnostics,
        "adapter_stream_gate_summary_digest",
    );
    let adapter_stream_read_only =
        extract_json_bool_field(runtime_diagnostics, "adapter_stream_read_only");
    let adapter_stream_write_allowed =
        extract_json_bool_field(runtime_diagnostics, "adapter_stream_write_allowed");
    let adapter_stream_applied =
        extract_json_bool_field(runtime_diagnostics, "adapter_stream_applied");

    if selection_mismatch.is_none() {
        failures.push("runtime_adapter_observations selection_mismatch is missing".to_owned());
    }
    let has_adapter_stream_gate_state = adapter_stream_trace_id
        .as_deref()
        .map(has_non_empty_trace_text)
        .unwrap_or(false)
        || adapter_stream_gate_summary_digest
            .as_deref()
            .map(has_non_empty_trace_text)
            .unwrap_or(false)
        || adapter_stream_read_only.is_some()
        || adapter_stream_write_allowed.is_some()
        || adapter_stream_applied.is_some();

    if has_adapter_stream_gate_state {
        if adapter_stream_read_only != Some(true) {
            failures.push("adapter_stream_read_only must be true".to_owned());
        }
        if adapter_stream_write_allowed != Some(false) {
            failures.push("adapter_stream_write_allowed must be false".to_owned());
        }
        if adapter_stream_applied != Some(false) {
            failures.push("adapter_stream_applied must be false".to_owned());
        }
    }

    if observation_count == 0 {
        if best_adapter.is_some()
            || best_score.is_some()
            || best_reward.is_some()
            || best_quality.is_some()
            || best_experience_id.is_some()
        {
            failures.push(
                "runtime_adapter_observations count is zero but best observation fields are populated"
                    .to_owned(),
            );
        }
        if selection_mismatch == Some(true) {
            failures.push(
                "runtime_adapter_observations count is zero but selection_mismatch=true".to_owned(),
            );
        }
        return failures;
    }

    let Some(best_adapter) = best_adapter else {
        failures.push(
            "runtime_adapter_observations count is positive but best_adapter is missing".to_owned(),
        );
        return failures;
    };

    for (name, value) in [
        ("best_score", best_score),
        ("best_reward", best_reward),
        ("best_quality", best_quality),
    ] {
        match value {
            Some(value) if (0.0..=1.0).contains(&value) => {}
            Some(value) => failures.push(format!(
                "runtime_adapter_observations {name} {value:.3} is outside 0..1"
            )),
            None => failures.push(format!(
                "runtime_adapter_observations count is positive but {name} is missing"
            )),
        }
    }

    if best_experience_id.is_none() {
        failures.push(
            "runtime_adapter_observations count is positive but best_experience_id is missing"
                .to_owned(),
        );
    }

    let expected_selection_mismatch = selected_adapter
        .as_deref()
        .map(|selected_adapter| selected_adapter != best_adapter.as_str())
        .unwrap_or(false);
    if let Some(selection_mismatch) = selection_mismatch
        && selection_mismatch != expected_selection_mismatch
    {
        failures.push(format!(
                "runtime_adapter_observations selection_mismatch {selection_mismatch} does not match best_adapter/selected_adapter comparison {expected_selection_mismatch}"
            ));
    }

    let adapter_hints = extract_json_string_array_field(line, "adapter_hints").unwrap_or_default();
    if !adapter_hints.iter().any(|adapter| adapter == &best_adapter) {
        failures.push(format!(
            "best_adapter {best_adapter} is outside trace adapter_hints"
        ));
    }

    if let Some(contract) = extract_json_string_field(line, "runtime_device_contract") {
        let contract_adapters = contract_value(&contract, "adapters")
            .map(split_contract_adapters)
            .unwrap_or_default();
        if !contract_adapters
            .iter()
            .any(|adapter| adapter == &best_adapter)
        {
            failures.push(format!(
                "best_adapter {best_adapter} is outside runtime_device_contract adapters"
            ));
        }
    }

    failures
}
