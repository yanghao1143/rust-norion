use super::fields::{
    extract_json_bool_field, extract_json_f32_field, extract_json_nullable_f32_field,
    extract_json_nullable_string_field, extract_json_nullable_u64_field,
    extract_json_string_array_field, extract_json_string_field, extract_json_usize_field,
    has_non_empty_trace_text, json_object_after_field,
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
    match json_object_after_field(hardware, "runtime_budget") {
        Some(runtime_budget) => failures.extend(evaluate_trace_runtime_budget(
            runtime_budget,
            hardware,
            execution,
        )),
        None => failures.push("hardware runtime_budget missing".to_owned()),
    }

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
    let imported_kv_blocks =
        extract_json_usize_field(runtime_diagnostics, "imported_kv_blocks").unwrap_or(0);
    let exported_kv_blocks =
        extract_json_usize_field(runtime_diagnostics, "exported_kv_blocks").unwrap_or(0);
    let weak_runtime_kv_imports_skipped =
        extract_json_usize_field(runtime_diagnostics, "weak_runtime_kv_imports_skipped")
            .unwrap_or(0);
    let budget_limited_runtime_kv_imports_skipped = extract_json_usize_field(
        runtime_diagnostics,
        "budget_limited_runtime_kv_imports_skipped",
    )
    .unwrap_or(0);
    let runtime_kv_segments_included =
        extract_json_usize_field(runtime_diagnostics, "runtime_kv_segments_included").unwrap_or(0);
    let runtime_kv_segments_skipped =
        extract_json_usize_field(runtime_diagnostics, "runtime_kv_segments_skipped").unwrap_or(0);
    let runtime_kv_segments_rejected =
        extract_json_usize_field(runtime_diagnostics, "runtime_kv_segments_rejected").unwrap_or(0);
    let runtime_kv_segment_count =
        extract_json_usize_field(runtime_diagnostics, "runtime_kv_segment_count").unwrap_or(0);
    let runtime_kv_segment_lifecycle_records =
        extract_json_usize_field(runtime_diagnostics, "runtime_kv_segment_lifecycle_records")
            .unwrap_or(0);
    let runtime_kv_segment_lifecycle_summaries = extract_json_string_array_field(
        runtime_diagnostics,
        "runtime_kv_segment_lifecycle_summaries",
    )
    .unwrap_or_default();
    let declared_runtime_kv_activity_signal =
        extract_json_bool_field(runtime_diagnostics, "has_runtime_kv_activity_signal");
    let declared_runtime_kv_segment_signal =
        extract_json_bool_field(runtime_diagnostics, "has_runtime_kv_segment_signal");
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
    let expected_runtime_kv_segment_count = runtime_kv_segments_included
        .saturating_add(runtime_kv_segments_skipped)
        .saturating_add(runtime_kv_segments_rejected);
    let has_runtime_kv_segment_signal = expected_runtime_kv_segment_count > 0;
    let has_runtime_kv_activity_signal = imported_kv_blocks
        .saturating_add(exported_kv_blocks)
        .saturating_add(weak_runtime_kv_imports_skipped)
        .saturating_add(budget_limited_runtime_kv_imports_skipped)
        .saturating_add(expected_runtime_kv_segment_count)
        > 0;
    let expected_forward_signal = layer_count > 0
        || has_runtime_forward_metric_signal
        || has_device_execution_signal
        || has_runtime_kv_activity_signal;
    let has_control_plane_static_architecture = device_execution_source.as_deref()
        == Some("control-plane-filled")
        && has_runtime_architecture_signal
        && !has_runtime_forward_metric_signal;

    if runtime_kv_segment_count != expected_runtime_kv_segment_count {
        failures.push(format!(
            "runtime_diagnostics runtime_kv_segment_count={runtime_kv_segment_count} does not match included/skipped/rejected total {expected_runtime_kv_segment_count}"
        ));
    }
    if runtime_kv_segment_lifecycle_records != expected_runtime_kv_segment_count {
        failures.push(format!(
            "runtime_diagnostics runtime_kv_segment_lifecycle_records={runtime_kv_segment_lifecycle_records} does not match segment count {expected_runtime_kv_segment_count}"
        ));
    }
    failures.extend(evaluate_runtime_kv_segment_lifecycle(
        expected_runtime_kv_segment_count,
        runtime_kv_segments_included,
        runtime_kv_segments_skipped,
        runtime_kv_segments_rejected,
        &runtime_kv_segment_lifecycle_summaries,
    ));
    if let Some(declared) = declared_runtime_kv_segment_signal
        && declared != has_runtime_kv_segment_signal
    {
        failures.push(format!(
            "runtime_diagnostics has_runtime_kv_segment_signal={declared} does not match segment count {expected_runtime_kv_segment_count}"
        ));
    }
    if let Some(declared) = declared_runtime_kv_activity_signal
        && declared != has_runtime_kv_activity_signal
    {
        failures.push(format!(
            "runtime_diagnostics has_runtime_kv_activity_signal={declared} does not match runtime KV counters"
        ));
    }
    if has_forward_signal != expected_forward_signal {
        failures.push(format!(
            "runtime_diagnostics has_forward_signal={has_forward_signal} does not match runtime forward/activity counters"
        ));
    }

    if has_forward_signal
        && !has_runtime_kv_activity_signal
        && !has_device_execution_signal
        && !has_runtime_architecture_signal
    {
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

fn evaluate_runtime_kv_segment_lifecycle(
    total: usize,
    included: usize,
    skipped: usize,
    rejected: usize,
    summaries: &[String],
) -> Vec<String> {
    let mut failures = Vec::new();
    if total == 0 {
        if !summaries.is_empty() {
            failures.push(
                "runtime_diagnostics runtime_kv_segment_lifecycle_summaries present without segment count"
                    .to_owned(),
            );
        }
        return failures;
    }

    if summaries.is_empty() {
        failures
            .push("runtime_diagnostics runtime_kv_segment_lifecycle_summaries missing".to_owned());
        return failures;
    }

    for summary in summaries {
        for required in [
            "lifecycle=",
            "reason_code=",
            "source_digest=",
            "shadow_state=",
            "drift_state=",
            "source_ids=",
            "expires_after_steps=",
            "score_milli=",
            "drift_gate_domains=",
            "rollback=redaction-digest:",
            "parent_lineage=",
            "rollback_anchor=",
            "affected_scope=runtime_kv_segment_candidate",
            "readmission_gate=",
            "operator_approval_required=",
        ] {
            if !summary.contains(required) {
                failures.push(format!(
                    "runtime_diagnostics runtime_kv_segment_lifecycle_summaries missing {required}"
                ));
            }
        }
        for domain in [
            "golden_fixture:",
            "routing_behavior:",
            "memory_hygiene:",
            "privacy:",
            "trace_schema:",
        ] {
            if !summary.contains(domain) {
                failures.push(format!(
                    "runtime_diagnostics runtime_kv_segment_lifecycle_summaries missing {domain} drift gate domain"
                ));
            }
        }
        if summary.contains("lifecycle=active")
            && (!summary.contains("shadow_state=ready_for_explicit_apply")
                || !summary.contains("drift_state=drift_passed")
                || !summary.contains("golden_fixture:pass")
                || !summary.contains("routing_behavior:pass")
                || !summary.contains("memory_hygiene:pass")
                || !summary.contains("privacy:pass")
                || !summary.contains("trace_schema:pass"))
        {
            failures.push(
                "runtime_diagnostics runtime_kv_segment active lifecycle missing passed shadow evidence"
                    .to_owned(),
            );
        }
        if summary.contains("lifecycle=recycle_candidate")
            && (!summary.contains("shadow_state=benchmark_pending")
                || !summary.contains("drift_state=benchmark_pending")
                || !summary.contains("golden_fixture:pending")
                || !summary.contains("routing_behavior:pending")
                || !summary.contains("memory_hygiene:pending")
                || !summary.contains("privacy:pending")
                || !summary.contains("trace_schema:pending"))
        {
            failures.push(
                "runtime_diagnostics runtime_kv_segment recycle lifecycle missing pending shadow evidence"
                    .to_owned(),
            );
        }
        if summary.contains("lifecycle=rejected_final")
            && (!summary.contains("shadow_state=quarantined")
                || !summary.contains("drift_state=drift_failed")
                || !summary.contains("golden_fixture:reject")
                || !summary.contains("routing_behavior:reject")
                || !summary.contains("memory_hygiene:reject")
                || !summary.contains("privacy:reject")
                || !summary.contains("trace_schema:reject"))
        {
            failures.push(
                "runtime_diagnostics runtime_kv_segment rejected lifecycle missing quarantine shadow evidence"
                    .to_owned(),
            );
        }
    }

    if included > 0
        && !summaries
            .iter()
            .any(|summary| summary.contains("lifecycle=active"))
    {
        failures.push(
            "runtime_diagnostics runtime_kv_segment included candidates missing lifecycle=active"
                .to_owned(),
        );
    }
    if skipped > 0
        && !summaries
            .iter()
            .any(|summary| summary.contains("lifecycle=recycle_candidate"))
    {
        failures.push(
            "runtime_diagnostics runtime_kv_segment skipped candidates missing lifecycle=recycle_candidate"
                .to_owned(),
        );
    }
    if rejected > 0
        && !summaries
            .iter()
            .any(|summary| summary.contains("lifecycle=rejected_final"))
    {
        failures.push(
            "runtime_diagnostics runtime_kv_segment rejected candidates missing lifecycle=rejected_final"
                .to_owned(),
        );
    }

    failures
}

fn evaluate_trace_runtime_budget(budget: &str, hardware: &str, execution: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let requested_device = extract_json_string_field(budget, "requested_device");
    let selected_device = extract_json_string_field(budget, "selected_device");
    let selected_adapter = extract_json_string_field(budget, "selected_adapter");
    let backend_family = extract_json_string_field(budget, "backend_family");
    let quantization_profile = extract_json_string_field(budget, "quantization_profile");
    let fallback_reason = extract_json_string_field(budget, "fallback_reason");
    let fail_closed_cpu_stub =
        extract_json_bool_field(budget, "fail_closed_cpu_stub").unwrap_or(false);
    let read_only = extract_json_bool_field(budget, "read_only").unwrap_or(false);
    let write_allowed = extract_json_bool_field(budget, "write_allowed").unwrap_or(false);
    let applied = extract_json_bool_field(budget, "applied").unwrap_or(false);
    let weight_bits = extract_json_usize_field(budget, "weight_quantization_bits");
    let kv_bits = extract_json_usize_field(budget, "kv_cache_quantization_bits");
    let gene_bits = extract_json_usize_field(budget, "gene_cache_quantization_bits");
    let model_weight_bytes = extract_json_nullable_u64_field(budget, "model_weight_bytes");
    let kv_cache_bytes = extract_json_nullable_u64_field(budget, "kv_cache_bytes");
    let gene_segment_cache_bytes =
        extract_json_nullable_u64_field(budget, "gene_segment_cache_bytes");
    let routing_reflection_overhead_bytes =
        extract_json_nullable_u64_field(budget, "routing_reflection_overhead_bytes");
    let total_required_bytes = extract_json_nullable_u64_field(budget, "total_required_bytes");
    let available_budget_bytes = extract_json_nullable_u64_field(budget, "available_budget_bytes");
    let memory_pressure = extract_json_f32_field(budget, "memory_pressure");

    for (name, value) in [
        ("requested_device", requested_device.as_deref()),
        ("selected_device", selected_device.as_deref()),
        ("selected_adapter", selected_adapter.as_deref()),
        ("backend_family", backend_family.as_deref()),
        ("quantization_profile", quantization_profile.as_deref()),
        ("fallback_reason", fallback_reason.as_deref()),
    ] {
        if value.map(has_non_empty_trace_text).unwrap_or(false) {
            continue;
        }
        failures.push(format!("runtime_budget {name} missing or empty"));
    }

    match quantization_profile.as_deref() {
        Some("q8" | "q4" | "cpu-stub") => {}
        Some(profile) => failures.push(format!(
            "runtime_budget quantization_profile={profile} is unsupported"
        )),
        None => {}
    }
    match fallback_reason.as_deref() {
        Some(
            "none"
            | "auto-device-cpu-stub"
            | "memory-pressure-quantized"
            | "budget-exceeded-cpu-stub",
        ) => {}
        Some(reason) => failures.push(format!(
            "runtime_budget fallback_reason={reason} is unsupported"
        )),
        None => {}
    }
    match weight_bits {
        Some(4 | 8) => {}
        Some(bits) => failures.push(format!(
            "runtime_budget weight_quantization_bits={bits} must be 4 or 8"
        )),
        None => failures.push("runtime_budget weight_quantization_bits missing".to_owned()),
    }
    match kv_bits {
        Some(4 | 8) => {}
        Some(bits) => failures.push(format!(
            "runtime_budget kv_cache_quantization_bits={bits} must be 4 or 8"
        )),
        None => failures.push("runtime_budget kv_cache_quantization_bits missing".to_owned()),
    }
    match gene_bits {
        Some(4 | 8) => {}
        Some(bits) => failures.push(format!(
            "runtime_budget gene_cache_quantization_bits={bits} must be 4 or 8"
        )),
        None => failures.push("runtime_budget gene_cache_quantization_bits missing".to_owned()),
    }

    if let (Some(model), Some(kv), Some(gene), Some(overhead), Some(total)) = (
        model_weight_bytes,
        kv_cache_bytes,
        gene_segment_cache_bytes,
        routing_reflection_overhead_bytes,
        total_required_bytes,
    ) {
        let expected_total = model
            .saturating_add(kv)
            .saturating_add(gene)
            .saturating_add(overhead);
        if expected_total != total {
            failures.push(format!(
                "runtime_budget total_required_bytes={total} does not match component sum {expected_total}"
            ));
        }
        if model == 0 || kv == 0 || gene == 0 || overhead == 0 {
            failures.push("runtime_budget component byte estimates must be nonzero".to_owned());
        }
    } else {
        failures.push("runtime_budget byte estimates are incomplete".to_owned());
    }

    if let Some(available) = available_budget_bytes {
        if available == 0 {
            failures.push("runtime_budget available_budget_bytes must be nonzero".to_owned());
        }
        if let (Some(total), Some(pressure)) = (total_required_bytes, memory_pressure) {
            let expected = if available == 0 {
                9.999
            } else {
                ((total as f64 / available as f64).min(9.999)) as f32
            };
            if (pressure - expected).abs() > 0.01 {
                failures.push(format!(
                    "runtime_budget memory_pressure={pressure:.3} does not match bytes pressure {expected:.3}"
                ));
            }
        }
    } else {
        failures.push("runtime_budget available_budget_bytes missing".to_owned());
    }

    if !read_only {
        failures.push("runtime_budget read_only must be true".to_owned());
    }
    if write_allowed {
        failures.push("runtime_budget write_allowed must be false".to_owned());
    }
    if applied {
        failures.push("runtime_budget applied must be false".to_owned());
    }

    let hardware_device = extract_json_string_field(hardware, "device");
    match (
        fallback_reason.as_deref(),
        selected_device.as_deref(),
        requested_device.as_deref(),
        hardware_device.as_deref(),
    ) {
        (Some("none"), Some(selected), _, Some(hardware_device)) if selected != hardware_device => {
            failures.push(format!(
                "runtime_budget selected_device={selected} must match hardware device {hardware_device} without fallback"
            ));
        }
        (Some("none"), _, _, _) if fail_closed_cpu_stub => failures.push(
            "runtime_budget fail_closed_cpu_stub must be false when fallback_reason=none"
                .to_owned(),
        ),
        (Some("memory-pressure-quantized"), Some(selected), _, Some(hardware_device))
            if selected != hardware_device =>
        {
            failures.push(format!(
                "runtime_budget selected_device={selected} must stay on hardware device {hardware_device} for quantized fallback"
            ));
        }
        (Some("memory-pressure-quantized"), _, _, _) if fail_closed_cpu_stub => failures.push(
            "runtime_budget fail_closed_cpu_stub must be false for quantized fallback".to_owned(),
        ),
        (Some("auto-device-cpu-stub"), Some("cpu"), Some("auto"), _) => {
            if !fail_closed_cpu_stub {
                failures.push(
                    "runtime_budget auto-device-cpu-stub must fail closed to CPU stub".to_owned(),
                );
            }
        }
        (Some("budget-exceeded-cpu-stub"), Some("cpu"), _, _) => {
            if !fail_closed_cpu_stub {
                failures.push(
                    "runtime_budget budget-exceeded-cpu-stub must fail closed to CPU stub"
                        .to_owned(),
                );
            }
        }
        (Some("auto-device-cpu-stub" | "budget-exceeded-cpu-stub"), Some(selected), _, _) => {
            failures.push(format!(
                "runtime_budget selected_device={selected} must be cpu for CPU stub fallback"
            ));
        }
        _ => {}
    }

    if let Some("cpu-stub") = quantization_profile.as_deref() {
        if !fail_closed_cpu_stub {
            failures.push(
                "runtime_budget quantization_profile=cpu-stub requires fail_closed_cpu_stub=true"
                    .to_owned(),
            );
        }
        if weight_bits != Some(4) || kv_bits != Some(4) {
            failures
                .push("runtime_budget cpu-stub must use 4-bit weight and KV estimates".to_owned());
        }
    }

    if let Some("memory-pressure-quantized") = fallback_reason.as_deref()
        && quantization_profile.as_deref() != Some("q4")
    {
        failures.push(
            "runtime_budget memory-pressure-quantized fallback must use q4 profile".to_owned(),
        );
    }

    let adapter_hints =
        extract_json_string_array_field(execution, "adapter_hints").unwrap_or_default();
    if let Some(adapter) = selected_adapter {
        if !adapter_hints.iter().any(|hint| hint == &adapter) {
            failures.push(format!(
                "runtime_budget selected_adapter={adapter} is outside hardware adapter_hints"
            ));
        }
    }

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
