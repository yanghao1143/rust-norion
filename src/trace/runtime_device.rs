use super::fields::{
    extract_json_bool_field, extract_json_nullable_f32_field, extract_json_nullable_string_field,
    extract_json_string_field, extract_json_usize_field, has_non_empty_trace_text,
    json_object_after_field,
};

pub(super) fn evaluate_trace_runtime_device_execution(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(runtime_diagnostics) = json_object_after_field(line, "runtime_diagnostics") else {
        return failures;
    };
    let Some(hardware) = json_object_after_field(line, "hardware") else {
        return failures;
    };
    let Some(execution) = json_object_after_field(hardware, "execution") else {
        return failures;
    };

    let has_forward_signal =
        extract_json_bool_field(runtime_diagnostics, "has_forward_signal").unwrap_or(false);
    let declared_runtime_architecture_signal =
        extract_json_bool_field(runtime_diagnostics, "has_runtime_architecture_signal");
    let model_id = extract_json_nullable_string_field(runtime_diagnostics, "model_id");
    let device_profile = extract_json_nullable_string_field(runtime_diagnostics, "device_profile");
    let primary_lane = extract_json_nullable_string_field(runtime_diagnostics, "primary_lane");
    let fallback_lane = extract_json_nullable_string_field(runtime_diagnostics, "fallback_lane");
    let memory_mode = extract_json_nullable_string_field(runtime_diagnostics, "memory_mode");
    let device_execution_source =
        extract_json_nullable_string_field(runtime_diagnostics, "device_execution_source");
    let hot_kv_precision_bits =
        extract_json_usize_field(runtime_diagnostics, "hot_kv_precision_bits");
    let cold_kv_precision_bits =
        extract_json_usize_field(runtime_diagnostics, "cold_kv_precision_bits");
    let layer_count = extract_json_usize_field(runtime_diagnostics, "layer_count").unwrap_or(0);
    let global_layers = extract_json_usize_field(runtime_diagnostics, "global_layers").unwrap_or(0);
    let local_window_layers =
        extract_json_usize_field(runtime_diagnostics, "local_window_layers").unwrap_or(0);
    let convolutional_fusion_layers =
        extract_json_usize_field(runtime_diagnostics, "convolutional_fusion_layers").unwrap_or(0);
    let hidden_size = extract_json_usize_field(runtime_diagnostics, "hidden_size").unwrap_or(0);
    let local_window_tokens =
        extract_json_usize_field(runtime_diagnostics, "local_window_tokens").unwrap_or(0);
    let forward_energy = extract_json_nullable_f32_field(runtime_diagnostics, "forward_energy");
    let kv_influence = extract_json_nullable_f32_field(runtime_diagnostics, "kv_influence");
    let has_kv_precision_signal =
        extract_json_bool_field(runtime_diagnostics, "has_kv_precision_signal").unwrap_or(false);
    let has_runtime_architecture_signal = model_id
        .as_deref()
        .map(has_non_empty_trace_text)
        .unwrap_or(false)
        && layer_count > 0
        && hidden_size > 0
        && local_window_tokens > 0;
    if let Some(declared) = declared_runtime_architecture_signal
        && declared != has_runtime_architecture_signal
    {
        failures.push(format!(
                "runtime_diagnostics has_runtime_architecture_signal={declared} does not match model/layer/hidden/local-window diagnostics"
            ));
    }
    let has_device_execution_signal = device_profile
        .as_deref()
        .map(has_non_empty_trace_text)
        .unwrap_or(false)
        && primary_lane
            .as_deref()
            .map(has_non_empty_trace_text)
            .unwrap_or(false)
        && fallback_lane
            .as_deref()
            .map(has_non_empty_trace_text)
            .unwrap_or(false)
        && memory_mode
            .as_deref()
            .map(has_non_empty_trace_text)
            .unwrap_or(false);
    let has_runtime_reported_device_execution =
        device_execution_source.as_deref() == Some("runtime-reported");
    let has_runtime_forward_metric_signal = global_layers
        .saturating_add(local_window_layers)
        .saturating_add(convolutional_fusion_layers)
        > 0
        || forward_energy.is_some()
        || kv_influence.is_some();
    let has_control_plane_static_architecture = device_execution_source.as_deref()
        == Some("control-plane-filled")
        && has_runtime_architecture_signal
        && !has_runtime_forward_metric_signal;

    if has_forward_signal && !has_device_execution_signal && !has_runtime_architecture_signal {
        failures.push(
            "runtime_diagnostics has_forward_signal=true but device execution diagnostics are incomplete"
                .to_owned(),
        );
    }
    if has_device_execution_signal {
        match device_execution_source.as_deref() {
            Some("runtime-reported" | "control-plane-filled") => {}
            Some(source) => failures.push(format!(
                "runtime_diagnostics device_execution_source={source} is invalid"
            )),
            None => failures.push(
                "runtime_diagnostics device execution is missing device_execution_source"
                    .to_owned(),
            ),
        }
    }

    let has_valid_kv_precision_signal = matches!(hot_kv_precision_bits, Some(4 | 8))
        && matches!(cold_kv_precision_bits, Some(4 | 8))
        && cold_kv_precision_bits <= hot_kv_precision_bits;
    if has_kv_precision_signal != has_valid_kv_precision_signal {
        failures.push(format!(
            "runtime_diagnostics has_kv_precision_signal={has_kv_precision_signal} does not match valid hot/cold KV precision diagnostics"
        ));
    }

    if !has_device_execution_signal {
        return failures;
    }

    if !has_runtime_reported_device_execution {
        if has_control_plane_static_architecture {
            return failures;
        }
        failures.push(format!(
            "runtime_diagnostics device execution source={} is not runtime-reported",
            device_execution_source.as_deref().unwrap_or("unknown")
        ));
        return failures;
    }

    if !has_valid_kv_precision_signal {
        failures.push(
            "runtime_diagnostics device execution is missing valid KV precision diagnostics"
                .to_owned(),
        );
    }

    require_trace_runtime_device_execution_string(
        &mut failures,
        "device_profile",
        device_profile.as_deref(),
        extract_json_string_field(hardware, "device").as_deref(),
    );
    require_trace_runtime_device_execution_string(
        &mut failures,
        "primary_lane",
        primary_lane.as_deref(),
        extract_json_string_field(execution, "primary_lane").as_deref(),
    );
    require_trace_runtime_device_execution_string(
        &mut failures,
        "fallback_lane",
        fallback_lane.as_deref(),
        extract_json_string_field(execution, "fallback_lane").as_deref(),
    );
    require_trace_runtime_device_execution_string(
        &mut failures,
        "memory_mode",
        memory_mode.as_deref(),
        extract_json_string_field(execution, "memory_mode").as_deref(),
    );
    require_trace_runtime_device_execution_usize(
        &mut failures,
        "hot_kv_precision_bits",
        hot_kv_precision_bits,
        extract_json_usize_field(execution, "hot_kv_bits"),
    );
    require_trace_runtime_device_execution_usize(
        &mut failures,
        "cold_kv_precision_bits",
        cold_kv_precision_bits,
        extract_json_usize_field(execution, "cold_kv_bits"),
    );

    failures
}

fn require_trace_runtime_device_execution_string(
    failures: &mut Vec<String>,
    field: &str,
    actual: Option<&str>,
    expected: Option<&str>,
) {
    match (actual, expected) {
        (Some(actual), Some(expected)) if actual == expected => {}
        (Some(actual), Some(expected)) => failures.push(format!(
            "runtime_diagnostics {field}={actual} does not match hardware execution {expected}"
        )),
        (None, Some(expected)) => failures.push(format!(
            "runtime_diagnostics {field} missing for hardware execution {expected}"
        )),
        (Some(actual), None) => failures.push(format!(
            "runtime_diagnostics {field}={actual} has no hardware execution value"
        )),
        (None, None) => {}
    }
}

fn require_trace_runtime_device_execution_usize(
    failures: &mut Vec<String>,
    field: &str,
    actual: Option<usize>,
    expected: Option<usize>,
) {
    match (actual, expected) {
        (Some(actual), Some(expected)) if actual == expected => {}
        (Some(actual), Some(expected)) => failures.push(format!(
            "runtime_diagnostics {field}={actual} does not match hardware execution {expected}"
        )),
        (None, Some(expected)) => failures.push(format!(
            "runtime_diagnostics {field} missing for hardware execution {expected}"
        )),
        (Some(actual), None) => failures.push(format!(
            "runtime_diagnostics {field}={actual} has no hardware execution value"
        )),
        (None, None) => {}
    }
}
