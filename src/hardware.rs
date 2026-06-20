#[cfg(test)]
use crate::hierarchy::{HierarchyWeights, TaskProfile};
#[cfg(test)]
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};

mod device;
mod gate;
mod plan;
mod probe;

#[cfg(test)]
use crate::runtime_manifest::RuntimeManifest;
pub use device::{
    ComputeLane, DeviceClass, DeviceMemoryMode, DeviceProfileDescriptor, DeviceTier,
    RuntimeAdapterHint,
};
pub use gate::{
    DevicePlanGateReport, DevicePlanGateRow, KvPrecisionPolicySummary,
    RuntimeManifestDeviceGateReport,
};
#[cfg(test)]
use gate::{
    validate_device_plan, validate_runtime_device_contract, validate_runtime_manifest_for_device,
};
pub use plan::{DeviceExecutionPlan, HardwareAllocator, HardwarePlan, MemoryGovernancePlan};
pub use probe::{HardwareProbe, HardwareProbeReport, HardwareSnapshot};

#[cfg(test)]
mod tests;
