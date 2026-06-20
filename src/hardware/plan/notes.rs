use crate::hierarchy::TaskProfile;

use super::super::device::DeviceClass;
use super::super::probe::HardwareSnapshot;
use super::model::DeviceExecutionPlan;

pub(super) fn notes(
    snapshot: HardwareSnapshot,
    profile: TaskProfile,
    pressure: f32,
    prompt_tokens: usize,
    execution: &DeviceExecutionPlan,
) -> Vec<String> {
    let mut notes = vec![
        format!("device:{}", snapshot.device.as_str()),
        format!("tier:{}", snapshot.device.tier().as_str()),
        format!("execution:{}", execution.primary_lane.as_str()),
        format!("fallback:{}", execution.fallback_lane.as_str()),
        format!("memory_mode:{}", execution.memory_mode.as_str()),
    ];

    if pressure >= 0.72 {
        notes.push("pressure:high_reduce_attention_and_kv".to_owned());
    } else if pressure >= 0.45 {
        notes.push("pressure:medium_apply_latency_budget".to_owned());
    } else {
        notes.push("pressure:low_full_budget".to_owned());
    }

    if prompt_tokens >= 8_192 {
        notes.push("context:long_reduce_kv_budget".to_owned());
    }
    if profile == TaskProfile::LongDocument {
        notes.push("profile:long_document_boost_convolution".to_owned());
    }
    match snapshot.device {
        DeviceClass::Mobile => notes.push("device_policy:mobile_thermal_and_ram_guard".to_owned()),
        DeviceClass::Embedded => notes.push("device_policy:embedded_minimal_kv".to_owned()),
        DeviceClass::BrowserWasm => notes.push("device_policy:browser_wasm_sandbox_kv".to_owned()),
        DeviceClass::Microcontroller => {
            notes.push("device_policy:microcontroller_tiny_streaming".to_owned());
        }
        DeviceClass::NpuAccelerator => {
            notes.push("device_policy:npu_gpu_load_as_accelerator_pressure".to_owned());
        }
        DeviceClass::MultiGpu => notes.push("device_policy:multi_gpu_expand_global_kv".to_owned()),
        _ => {}
    }

    notes
}
