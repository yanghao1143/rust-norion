use std::path::PathBuf;

use rust_norion::{
    DevicePlanGateReport, RuntimeManifest, RuntimeManifestDeviceGateReport,
    RuntimeManifestValidation,
};

pub(crate) fn print_runtime_manifest_gate_report(
    manifest: &RuntimeManifest,
    validation: &RuntimeManifestValidation,
    device_gate: &RuntimeManifestDeviceGateReport,
    all_devices_gate: Option<&DevicePlanGateReport>,
) {
    let all_device_failures = all_devices_gate
        .map(DevicePlanGateReport::failure_count)
        .unwrap_or(0);
    println!("Noiron runtime manifest gate");
    println!(
        "runtime_manifest_gate: passed={} errors={} warnings={} device_failures={} all_device_failures={}",
        validation.passed()
            && device_gate.passed()
            && all_devices_gate
                .map(DevicePlanGateReport::passed)
                .unwrap_or(true),
        validation.errors.len() + device_gate.failures.len() + all_device_failures,
        validation.warnings.len(),
        device_gate.failures.len(),
        all_device_failures
    );
    println!("{}", device_gate.summary_line());
    println!(
        "runtime_metadata: {}",
        manifest.runtime_metadata().summary()
    );
    println!(
        "runtime_assets: weights={} tokenizer={} config={}",
        option_path_display(manifest.assets.weights.as_ref()),
        option_path_display(manifest.assets.tokenizer.as_ref()),
        option_path_display(manifest.assets.config.as_ref())
    );
    println!(
        "runtime_architecture: layers={} hidden={} attention_heads={} kv_heads={} local_window={}",
        manifest.architecture.layer_count,
        manifest.architecture.hidden_size,
        manifest.architecture.attention_heads,
        manifest.architecture.kv_heads,
        manifest.architecture.local_window_tokens
    );
    println!(
        "runtime_kv_policy: import={} export={} max_import={} max_export={} kv_bits={}/{}",
        manifest.kv_policy.import_enabled,
        manifest.kv_policy.export_enabled,
        manifest.kv_policy.max_import_blocks,
        manifest.kv_policy.max_export_blocks,
        manifest.quantization.hot_kv.width(),
        manifest.quantization.cold_kv.width()
    );
    println!(
        "runtime_device: device={} tier={} primary={} fallback={} memory={} adapters={} runtime_adapter={} parallel_chunks={} kv_prefetch={} kv_bits={}/{} disk_spill={} local_kv_tokens={} global_kv_tokens={} latency_budget_ms={}",
        device_gate.device.as_str(),
        device_gate.tier.as_str(),
        device_gate.primary_lane.as_str(),
        device_gate.fallback_lane.as_str(),
        device_gate.memory_mode.as_str(),
        device_gate.adapters_csv(),
        device_gate.runtime_adapter_name(),
        device_gate.max_parallel_chunks,
        device_gate.kv_prefetch_blocks,
        device_gate.hot_kv_precision_bits,
        device_gate.cold_kv_precision_bits,
        device_gate.allow_disk_spill,
        device_gate.local_kv_token_budget,
        device_gate.global_kv_token_budget,
        device_gate
            .latency_budget_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned())
    );
    println!(
        "runtime_device_contract: {}",
        device_gate.runtime_device_contract
    );

    for warning in &validation.warnings {
        println!("runtime_manifest_warning: {warning}");
    }
    for error in &validation.errors {
        println!("runtime_manifest_error: {error}");
    }
    for failure in &device_gate.failures {
        println!("runtime_manifest_device_failure: {failure}");
    }
    if let Some(report) = all_devices_gate {
        print_runtime_manifest_all_devices_gate_report(report);
    }
}

fn print_runtime_manifest_all_devices_gate_report(report: &DevicePlanGateReport) {
    println!("{}", report.summary_line());
    println!(
        "runtime_manifest_all_devices_kv_precision_policy: {}",
        report.kv_precision_policy_summary().summary_line()
    );
    println!(
        "runtime_manifest_all_devices,profile,tier,scope,aliases,primary_lane,fallback_lane,memory_mode,adapters,runtime_adapter,parallel_chunks,kv_prefetch,kv_bits,hot_quant_covered,cold_quant_covered,runtime_quant_covered,kv_order_valid,disk_spill,runtime_kv_import,runtime_kv_export,runtime_max_import,runtime_max_export,runtime_kv_bits,local_kv_tokens,global_kv_tokens,latency_budget_ms,runtime_device_contract,passed"
    );

    for row in &report.rows {
        let fields = vec![
            "runtime_manifest_all_devices".to_owned(),
            row.device.as_str().to_owned(),
            row.tier.as_str().to_owned(),
            row.scope.to_owned(),
            row.aliases_csv(),
            row.primary_lane.as_str().to_owned(),
            row.fallback_lane.as_str().to_owned(),
            row.memory_mode.as_str().to_owned(),
            row.adapters_csv(),
            row.runtime_adapter_name().to_owned(),
            row.max_parallel_chunks.to_string(),
            row.kv_prefetch_blocks.to_string(),
            format!(
                "{}/{}",
                row.hot_kv_precision_bits, row.cold_kv_precision_bits
            ),
            row.hot_quant_policy_covered.to_string(),
            row.cold_quant_policy_covered.to_string(),
            row.runtime_quant_policy_covered.to_string(),
            row.kv_precision_order_valid.to_string(),
            row.allow_disk_spill.to_string(),
            row.runtime_kv_import_enabled.to_string(),
            row.runtime_kv_export_enabled.to_string(),
            row.runtime_max_import_blocks.to_string(),
            row.runtime_max_export_blocks.to_string(),
            format!(
                "{}/{}",
                row.runtime_hot_kv_precision_bits, row.runtime_cold_kv_precision_bits
            ),
            row.local_kv_token_budget.to_string(),
            row.global_kv_token_budget.to_string(),
            row.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            row.runtime_device_contract.clone(),
            row.passed().to_string(),
        ];
        println!("{}", fields.join(","));

        for failure in &row.failures {
            println!(
                "runtime_manifest_all_devices_failure: {}: {}",
                row.device.as_str(),
                failure
            );
        }
    }
}

fn option_path_display(path: Option<&PathBuf>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_owned())
}
