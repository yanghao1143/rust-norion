use super::super::device::{
    ComputeLane, DeviceClass, DeviceMemoryMode, DeviceTier, RuntimeAdapterHint,
};
use super::model::DeviceExecutionPlan;

pub(super) fn device_execution_plan(device: DeviceClass, pressure: f32) -> DeviceExecutionPlan {
    let tier = device.tier();
    let base_parallel_chunks = match tier {
        DeviceTier::Tiny => 1,
        DeviceTier::Constrained => 1,
        DeviceTier::Balanced => 2,
        DeviceTier::Accelerated => 4,
        DeviceTier::Distributed => 8,
        DeviceTier::Auto => 2,
    };
    let max_parallel_chunks = if pressure >= 0.72 {
        1
    } else if pressure >= 0.45 {
        (base_parallel_chunks / 2).max(1)
    } else {
        base_parallel_chunks
    };
    let kv_prefetch_blocks = if pressure >= 0.72 {
        1
    } else {
        match tier {
            DeviceTier::Tiny => 1,
            DeviceTier::Constrained => 2,
            DeviceTier::Balanced => 3,
            DeviceTier::Accelerated => 5,
            DeviceTier::Distributed => 8,
            DeviceTier::Auto => 3,
        }
    };
    let hot_kv_precision_bits = if matches!(
        device,
        DeviceClass::Embedded | DeviceClass::BrowserWasm | DeviceClass::Microcontroller
    ) || pressure >= 0.88
    {
        4
    } else {
        8
    };

    let (primary_lane, fallback_lane, memory_mode, adapter_hints, allow_disk_spill) = match device {
        DeviceClass::CpuOnly => (
            ComputeLane::CpuVector,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapterHint::PortableRust,
                RuntimeAdapterHint::CpuSimd,
                RuntimeAdapterHint::OpenVino,
            ],
            true,
        ),
        DeviceClass::IntegratedGpu => (
            ComputeLane::IntegratedGpu,
            ComputeLane::CpuVector,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::Vulkan,
                RuntimeAdapterHint::DirectMl,
                RuntimeAdapterHint::OneApi,
                RuntimeAdapterHint::PortableRust,
            ],
            true,
        ),
        DeviceClass::DiscreteGpu => (
            ComputeLane::DiscreteGpu,
            ComputeLane::CpuVector,
            DeviceMemoryMode::GpuResident,
            vec![
                RuntimeAdapterHint::Cuda,
                RuntimeAdapterHint::Rocm,
                RuntimeAdapterHint::Vulkan,
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::OneApi,
                RuntimeAdapterHint::DirectMl,
                RuntimeAdapterHint::PortableRust,
            ],
            true,
        ),
        DeviceClass::UnifiedMemory => (
            ComputeLane::UnifiedMemoryGpu,
            ComputeLane::CpuVector,
            DeviceMemoryMode::UnifiedMemory,
            vec![
                RuntimeAdapterHint::Metal,
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::Vulkan,
                RuntimeAdapterHint::PortableRust,
            ],
            true,
        ),
        DeviceClass::Mobile => (
            ComputeLane::IntegratedGpu,
            ComputeLane::CpuVector,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapterHint::CoreMl,
                RuntimeAdapterHint::Nnapi,
                RuntimeAdapterHint::Qnn,
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::WebGpu,
                RuntimeAdapterHint::PortableRust,
            ],
            true,
        ),
        DeviceClass::Embedded => (
            ComputeLane::DiskBackedStreaming,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::MinimalDisk,
            vec![
                RuntimeAdapterHint::PortableRust,
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::Nnapi,
                RuntimeAdapterHint::Qnn,
                RuntimeAdapterHint::Rknn,
            ],
            true,
        ),
        DeviceClass::BrowserWasm => (
            ComputeLane::IntegratedGpu,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapterHint::WebGpu,
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::PortableRust,
            ],
            true,
        ),
        DeviceClass::Microcontroller => (
            ComputeLane::DiskBackedStreaming,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::MinimalDisk,
            vec![RuntimeAdapterHint::PortableRust],
            true,
        ),
        DeviceClass::NpuAccelerator => (
            ComputeLane::NeuralAccelerator,
            ComputeLane::IntegratedGpu,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapterHint::CoreMl,
                RuntimeAdapterHint::Nnapi,
                RuntimeAdapterHint::Qnn,
                RuntimeAdapterHint::Cann,
                RuntimeAdapterHint::Mlu,
                RuntimeAdapterHint::Rknn,
                RuntimeAdapterHint::OpenVino,
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::CustomAccelerator,
                RuntimeAdapterHint::PortableRust,
            ],
            true,
        ),
        DeviceClass::MultiGpu => (
            ComputeLane::MultiAccelerator,
            ComputeLane::DiscreteGpu,
            DeviceMemoryMode::DistributedSharded,
            vec![
                RuntimeAdapterHint::MultiDevice,
                RuntimeAdapterHint::Cuda,
                RuntimeAdapterHint::Rocm,
                RuntimeAdapterHint::OneApi,
                RuntimeAdapterHint::Vulkan,
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::CustomAccelerator,
                RuntimeAdapterHint::PortableRust,
            ],
            false,
        ),
        DeviceClass::Edge => (
            ComputeLane::CpuVector,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapterHint::PortableRust,
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::Vulkan,
                RuntimeAdapterHint::Nnapi,
                RuntimeAdapterHint::Qnn,
                RuntimeAdapterHint::Rknn,
                RuntimeAdapterHint::CustomAccelerator,
            ],
            true,
        ),
        DeviceClass::Server => (
            ComputeLane::DiscreteGpu,
            ComputeLane::CpuVector,
            DeviceMemoryMode::GpuResident,
            vec![
                RuntimeAdapterHint::Cuda,
                RuntimeAdapterHint::Rocm,
                RuntimeAdapterHint::OneApi,
                RuntimeAdapterHint::Vulkan,
                RuntimeAdapterHint::Wgpu,
                RuntimeAdapterHint::OpenVino,
                RuntimeAdapterHint::PortableRust,
            ],
            true,
        ),
        DeviceClass::Auto => (
            ComputeLane::CpuVector,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapterHint::PortableRust,
                RuntimeAdapterHint::CpuSimd,
                RuntimeAdapterHint::Wgpu,
            ],
            true,
        ),
    };

    DeviceExecutionPlan {
        primary_lane,
        fallback_lane,
        memory_mode,
        adapter_hints,
        max_parallel_chunks,
        kv_prefetch_blocks,
        hot_kv_precision_bits,
        cold_kv_precision_bits: 4,
        allow_disk_spill,
    }
}
