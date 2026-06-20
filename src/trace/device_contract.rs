use super::fields::{
    contract_value, extract_json_bool_field, extract_json_string_array_field,
    extract_json_string_field, extract_json_usize_field, json_object_after_field,
    require_contract_string, require_contract_usize, split_contract_adapters,
};

pub(super) fn evaluate_trace_device_contract(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(contract) = extract_json_string_field(line, "runtime_device_contract") else {
        return failures;
    };
    let hardware = json_object_after_field(line, "hardware").unwrap_or(line);
    let execution = json_object_after_field(hardware, "execution").unwrap_or(hardware);
    let hot_kv_bits = extract_json_usize_field(execution, "hot_kv_bits");
    let cold_kv_bits = extract_json_usize_field(execution, "cold_kv_bits");

    require_contract_string(
        &mut failures,
        &contract,
        "device",
        extract_json_string_field(hardware, "device"),
    );
    require_contract_string(
        &mut failures,
        &contract,
        "tier",
        extract_json_string_field(hardware, "tier"),
    );
    require_contract_string(
        &mut failures,
        &contract,
        "primary",
        extract_json_string_field(execution, "primary_lane"),
    );
    require_contract_string(
        &mut failures,
        &contract,
        "fallback",
        extract_json_string_field(execution, "fallback_lane"),
    );
    require_contract_string(
        &mut failures,
        &contract,
        "memory",
        extract_json_string_field(execution, "memory_mode"),
    );
    require_contract_usize(
        &mut failures,
        &contract,
        "parallel_chunks",
        extract_json_usize_field(execution, "max_parallel_chunks"),
    );
    require_contract_usize(
        &mut failures,
        &contract,
        "kv_prefetch",
        extract_json_usize_field(execution, "kv_prefetch_blocks"),
    );
    require_contract_string(
        &mut failures,
        &contract,
        "kv_bits",
        match (hot_kv_bits, cold_kv_bits) {
            (Some(hot), Some(cold)) => Some(format!("{hot}/{cold}")),
            _ => None,
        },
    );
    match hot_kv_bits {
        Some(4 | 8) => {}
        Some(value) => failures.push(format!(
            "hardware execution hot_kv_bits {value} must be 4 or 8"
        )),
        None => failures.push("hardware execution hot_kv_bits missing".to_owned()),
    }
    match cold_kv_bits {
        Some(4 | 8) => {}
        Some(value) => failures.push(format!(
            "hardware execution cold_kv_bits {value} must be 4 or 8"
        )),
        None => failures.push("hardware execution cold_kv_bits missing".to_owned()),
    }
    if let (Some(hot), Some(cold)) = (hot_kv_bits, cold_kv_bits)
        && cold > hot
    {
        failures.push(format!(
            "hardware execution cold_kv_bits {cold} must not exceed hot_kv_bits {hot}"
        ));
    }
    require_contract_string(
        &mut failures,
        &contract,
        "disk_spill",
        extract_json_bool_field(execution, "disk_spill").map(|value| value.to_string()),
    );
    require_contract_usize(
        &mut failures,
        &contract,
        "local_kv_tokens",
        extract_json_usize_field(hardware, "local_kv_token_budget"),
    );
    require_contract_usize(
        &mut failures,
        &contract,
        "global_kv_tokens",
        extract_json_usize_field(hardware, "global_kv_token_budget"),
    );

    let adapter_hints =
        extract_json_string_array_field(execution, "adapter_hints").unwrap_or_default();
    if adapter_hints.is_empty() {
        failures.push("adapter_hints must not be empty".to_owned());
    }
    let contract_adapters = contract_value(&contract, "adapters")
        .map(split_contract_adapters)
        .unwrap_or_default();
    if contract_adapters.is_empty() {
        failures.push("runtime_device_contract missing adapters list".to_owned());
    }
    for adapter in &adapter_hints {
        if !contract_adapters
            .iter()
            .any(|contract_adapter| contract_adapter == adapter)
        {
            failures.push(format!(
                "runtime_device_contract adapters missing trace adapter_hint {adapter}"
            ));
        }
    }

    if let Some(selected_adapter) = extract_json_string_field(line, "selected_adapter") {
        if !adapter_hints
            .iter()
            .any(|adapter| adapter == &selected_adapter)
        {
            failures.push(format!(
                "selected_adapter {selected_adapter} is outside trace adapter_hints"
            ));
        }
        if !contract_adapters
            .iter()
            .any(|adapter| adapter == &selected_adapter)
        {
            failures.push(format!(
                "selected_adapter {selected_adapter} is outside runtime_device_contract adapters"
            ));
        }
    }

    failures
}
