use super::fields::{extract_json_bool_field, extract_json_usize_field};

pub(super) fn evaluate_trace_runtime_kv(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let runtime_kv_exported = extract_json_usize_field(line, "runtime_kv_exported").unwrap_or(0);
    let runtime_kv_stored = extract_json_usize_field(line, "runtime_kv_stored").unwrap_or(0);
    let runtime_kv_hold = extract_json_bool_field(line, "runtime_kv_hold").unwrap_or(false);
    let runtime_kv_held = extract_json_usize_field(line, "runtime_kv_held").unwrap_or(0);
    let diagnostic_exported = extract_json_usize_field(line, "exported_kv_blocks").unwrap_or(0);
    let memory_write = extract_json_bool_field(line, "memory_write").unwrap_or(false);
    let runtime_kv_write = extract_json_bool_field(line, "runtime_kv_write").unwrap_or(false);
    let revision_passes = extract_json_usize_field(line, "revision_passes").unwrap_or(0);
    let expected_runtime_kv_held = runtime_kv_exported.saturating_sub(runtime_kv_stored);

    if diagnostic_exported != runtime_kv_exported {
        failures.push(format!(
            "runtime_diagnostics exported_kv_blocks {diagnostic_exported} does not match memory runtime_kv_exported {runtime_kv_exported}"
        ));
    }

    if runtime_kv_stored > runtime_kv_exported {
        failures.push(format!(
            "runtime_kv_stored {runtime_kv_stored} exceeds runtime_kv_exported {runtime_kv_exported}"
        ));
    }

    if runtime_kv_held != expected_runtime_kv_held {
        failures.push(format!(
            "runtime_kv_held {runtime_kv_held} does not match runtime_kv_exported-runtime_kv_stored {expected_runtime_kv_held}"
        ));
    }

    if runtime_kv_hold != (runtime_kv_held > 0) {
        failures.push(format!(
            "runtime_kv_hold {runtime_kv_hold} does not match runtime_kv_held {runtime_kv_held}"
        ));
    }

    if runtime_kv_stored > 0 && !runtime_kv_write {
        failures.push(format!(
            "runtime_kv_stored {runtime_kv_stored} requires runtime_kv_write=true"
        ));
    }

    if runtime_kv_stored > 0 && !memory_write {
        failures.push(format!(
            "runtime_kv_stored {runtime_kv_stored} requires memory_write=true"
        ));
    }

    if runtime_kv_stored > 0 && revision_passes > 0 {
        failures.push(format!(
            "runtime_kv_stored {runtime_kv_stored} requires revision_passes=0"
        ));
    }

    failures
}
