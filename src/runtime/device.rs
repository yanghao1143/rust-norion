use crate::experience::ExperienceMatch;
use crate::hardware::{
    ComputeLane, DeviceClass, DeviceMemoryMode, HardwarePlan, RuntimeAdapterHint,
};
use crate::reflection::RuntimeDiagnostics;

pub(super) fn parse_runtime_adapter_hint(value: &str) -> Option<RuntimeAdapterHint> {
    RuntimeAdapterHint::parse(value)
}

pub(super) fn experience_matches_hardware_plan(
    experience: &ExperienceMatch,
    hardware_plan: &HardwarePlan,
) -> bool {
    let has_device_execution_fields = experience
        .runtime_device_profile
        .as_deref()
        .is_some_and(has_runtime_text)
        || experience
            .runtime_primary_lane
            .as_deref()
            .is_some_and(has_runtime_text)
        || experience
            .runtime_fallback_lane
            .as_deref()
            .is_some_and(has_runtime_text)
        || experience
            .runtime_memory_mode
            .as_deref()
            .is_some_and(has_runtime_text);
    if has_device_execution_fields
        && experience.runtime_device_execution_source.as_deref()
            != Some(RuntimeDiagnostics::runtime_reported_device_execution_source())
    {
        return false;
    }

    if let Some(device_profile) = experience.runtime_device_profile.as_deref() {
        let Some(device) = parse_runtime_device_class(device_profile) else {
            return false;
        };
        if device != hardware_plan.device {
            return false;
        }
    }

    if let Some(primary_lane) = experience.runtime_primary_lane.as_deref() {
        let Some(lane) = parse_runtime_compute_lane(primary_lane) else {
            return false;
        };
        if lane != hardware_plan.execution.primary_lane {
            return false;
        }
    }

    if let Some(fallback_lane) = experience.runtime_fallback_lane.as_deref() {
        let Some(lane) = parse_runtime_compute_lane(fallback_lane) else {
            return false;
        };
        if lane != hardware_plan.execution.fallback_lane {
            return false;
        }
    }

    if let Some(memory_mode) = experience.runtime_memory_mode.as_deref() {
        let Some(mode) = parse_runtime_memory_mode(memory_mode) else {
            return false;
        };
        if mode != hardware_plan.execution.memory_mode {
            return false;
        }
    }

    true
}

fn has_runtime_text(value: &str) -> bool {
    !value.trim().is_empty()
}

pub(super) fn parse_runtime_device_class(value: &str) -> Option<DeviceClass> {
    value.parse::<DeviceClass>().ok()
}

pub(super) fn parse_runtime_compute_lane(value: &str) -> Option<ComputeLane> {
    match value {
        "cpu-portable" => Some(ComputeLane::CpuPortable),
        "cpu-vector" => Some(ComputeLane::CpuVector),
        "integrated-gpu" => Some(ComputeLane::IntegratedGpu),
        "discrete-gpu" => Some(ComputeLane::DiscreteGpu),
        "unified-memory-gpu" => Some(ComputeLane::UnifiedMemoryGpu),
        "neural-accelerator" => Some(ComputeLane::NeuralAccelerator),
        "multi-accelerator" => Some(ComputeLane::MultiAccelerator),
        "disk-backed-streaming" => Some(ComputeLane::DiskBackedStreaming),
        _ => None,
    }
}

pub(super) fn parse_runtime_memory_mode(value: &str) -> Option<DeviceMemoryMode> {
    match value {
        "minimal-disk" => Some(DeviceMemoryMode::MinimalDisk),
        "tiered-disk" => Some(DeviceMemoryMode::TieredDisk),
        "unified-memory" => Some(DeviceMemoryMode::UnifiedMemory),
        "gpu-resident" => Some(DeviceMemoryMode::GpuResident),
        "distributed-sharded" => Some(DeviceMemoryMode::DistributedSharded),
        _ => None,
    }
}
