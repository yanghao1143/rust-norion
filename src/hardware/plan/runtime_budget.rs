use super::super::device::{DeviceClass, DeviceMemoryMode, RuntimeAdapterHint};
use super::super::probe::HardwareSnapshot;
use super::model::DeviceExecutionPlan;

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeDeviceCapability {
    pub device: DeviceClass,
    pub backend_family: &'static str,
    pub memory_capacity_bytes: u64,
    pub cpu_fallback: RuntimeAdapterHint,
}

impl RuntimeDeviceCapability {
    pub fn for_device(device: DeviceClass) -> Self {
        match device {
            DeviceClass::Auto | DeviceClass::CpuOnly => Self {
                device: DeviceClass::CpuOnly,
                backend_family: "cpu",
                memory_capacity_bytes: 8 * GIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::IntegratedGpu => Self {
                device,
                backend_family: "wgpu+vulkan+directml",
                memory_capacity_bytes: 12 * GIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::DiscreteGpu => Self {
                device,
                backend_family: "cuda+vulkan+directml",
                memory_capacity_bytes: 16 * GIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::UnifiedMemory => Self {
                device,
                backend_family: "metal+wgpu+vulkan",
                memory_capacity_bytes: 24 * GIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::Mobile => Self {
                device,
                backend_family: "coreml+nnapi+qnn",
                memory_capacity_bytes: 6 * GIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::Embedded => Self {
                device,
                backend_family: "edge-wgpu+portable",
                memory_capacity_bytes: 2 * GIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::BrowserWasm => Self {
                device,
                backend_family: "webgpu+wasm",
                memory_capacity_bytes: 512 * MIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::Microcontroller => Self {
                device,
                backend_family: "no-std-stub",
                memory_capacity_bytes: 64 * MIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::NpuAccelerator => Self {
                device,
                backend_family: "npu+openvino+wgpu",
                memory_capacity_bytes: 8 * GIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::MultiGpu => Self {
                device,
                backend_family: "multi-device+cuda+vulkan",
                memory_capacity_bytes: 48 * GIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::Edge => Self {
                device,
                backend_family: "edge-vulkan+wgpu+portable",
                memory_capacity_bytes: 4 * GIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
            DeviceClass::Server => Self {
                device,
                backend_family: "cuda+rocm+oneapi+vulkan",
                memory_capacity_bytes: 64 * GIB,
                cpu_fallback: RuntimeAdapterHint::PortableRust,
            },
        }
    }
}

pub fn runtime_device_capability_catalog() -> Vec<RuntimeDeviceCapability> {
    DeviceClass::explicit_profiles()
        .iter()
        .copied()
        .map(RuntimeDeviceCapability::for_device)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeQuantizationProfile {
    Q8,
    Q4,
    CpuStub,
}

impl RuntimeQuantizationProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Q8 => "q8",
            Self::Q4 => "q4",
            Self::CpuStub => "cpu-stub",
        }
    }

    pub fn weight_bits(self) -> u8 {
        match self {
            Self::Q8 => 8,
            Self::Q4 | Self::CpuStub => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBudgetFallbackReason {
    None,
    AutoDeviceCpuStub,
    MemoryPressureQuantized,
    BudgetExceededCpuStub,
}

impl RuntimeBudgetFallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::AutoDeviceCpuStub => "auto-device-cpu-stub",
            Self::MemoryPressureQuantized => "memory-pressure-quantized",
            Self::BudgetExceededCpuStub => "budget-exceeded-cpu-stub",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBudgetInput {
    pub model_parameter_count: u64,
    pub requested_weight_bits: u8,
    pub context_tokens: usize,
    pub layer_count: usize,
    pub hidden_size: usize,
    pub gene_segment_count: usize,
    pub gene_segment_tokens: usize,
    pub reflection_passes: usize,
    pub available_memory_bytes: Option<u64>,
}

impl RuntimeBudgetInput {
    pub fn fixture(context_tokens: usize) -> Self {
        let context_tokens = context_tokens.max(1_024);
        let gene_segment_count = context_tokens.div_ceil(512).max(1);
        Self {
            model_parameter_count: 750_000_000,
            requested_weight_bits: 8,
            context_tokens,
            layer_count: 24,
            hidden_size: 1_536,
            gene_segment_count,
            gene_segment_tokens: 256,
            reflection_passes: 1,
            available_memory_bytes: None,
        }
    }

    pub fn with_model_parameter_count(mut self, parameter_count: u64) -> Self {
        self.model_parameter_count = parameter_count.max(1);
        self
    }

    pub fn with_context_tokens(mut self, context_tokens: usize) -> Self {
        self.context_tokens = context_tokens.max(1);
        self
    }

    pub fn with_architecture(mut self, layer_count: usize, hidden_size: usize) -> Self {
        self.layer_count = layer_count.max(1);
        self.hidden_size = hidden_size.max(1);
        self
    }

    pub fn with_gene_segments(mut self, count: usize, tokens_per_segment: usize) -> Self {
        self.gene_segment_count = count.max(1);
        self.gene_segment_tokens = tokens_per_segment.max(1);
        self
    }

    pub fn with_reflection_passes(mut self, reflection_passes: usize) -> Self {
        self.reflection_passes = reflection_passes;
        self
    }

    pub fn with_available_memory_bytes(mut self, available_memory_bytes: u64) -> Self {
        self.available_memory_bytes = Some(available_memory_bytes.max(1));
        self
    }

    fn requested_weight_bits(self) -> u8 {
        match self.requested_weight_bits {
            4 => 4,
            _ => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeBudgetReport {
    pub requested_device: DeviceClass,
    pub selected_device: DeviceClass,
    pub selected_adapter: RuntimeAdapterHint,
    pub backend_family: &'static str,
    pub quantization_profile: RuntimeQuantizationProfile,
    pub weight_quantization_bits: u8,
    pub kv_cache_quantization_bits: u8,
    pub gene_cache_quantization_bits: u8,
    pub model_weight_bytes: u64,
    pub kv_cache_bytes: u64,
    pub gene_segment_cache_bytes: u64,
    pub routing_reflection_overhead_bytes: u64,
    pub total_required_bytes: u64,
    pub available_budget_bytes: u64,
    pub memory_pressure: f32,
    pub fallback_reason: RuntimeBudgetFallbackReason,
    pub fail_closed_cpu_stub: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl RuntimeBudgetReport {
    pub fn summary(&self) -> String {
        format!(
            "requested={} selected={} adapter={} backend={} quant={} weight_bits={} kv_bits={} gene_bits={} required_bytes={} available_bytes={} pressure={:.3} fallback={} cpu_stub={} read_only={} write_allowed={} applied={}",
            self.requested_device.as_str(),
            self.selected_device.as_str(),
            self.selected_adapter.as_str(),
            self.backend_family,
            self.quantization_profile.as_str(),
            self.weight_quantization_bits,
            self.kv_cache_quantization_bits,
            self.gene_cache_quantization_bits,
            self.total_required_bytes,
            self.available_budget_bytes,
            self.memory_pressure,
            self.fallback_reason.as_str(),
            self.fail_closed_cpu_stub,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

pub(super) fn runtime_budget_plan(
    snapshot: HardwareSnapshot,
    execution: &DeviceExecutionPlan,
    input: RuntimeBudgetInput,
) -> RuntimeBudgetReport {
    if snapshot.device == DeviceClass::Auto {
        let cpu_capability = RuntimeDeviceCapability::for_device(DeviceClass::CpuOnly);
        let estimate = estimate_runtime_budget(&input, RuntimeQuantizationProfile::CpuStub, 4, 4);
        return report(
            snapshot.device,
            cpu_capability.device,
            cpu_capability.cpu_fallback,
            cpu_capability.backend_family,
            RuntimeQuantizationProfile::CpuStub,
            estimate,
            available_budget_bytes(snapshot, DeviceClass::CpuOnly, execution, &input),
            RuntimeBudgetFallbackReason::AutoDeviceCpuStub,
            true,
        );
    }

    let requested_capability = RuntimeDeviceCapability::for_device(snapshot.device);
    let requested_available =
        available_budget_bytes(snapshot, requested_capability.device, execution, &input);
    let requested_weight_bits = input.requested_weight_bits();
    let q8_profile = if requested_weight_bits == 4 {
        RuntimeQuantizationProfile::Q4
    } else {
        RuntimeQuantizationProfile::Q8
    };
    let q8_estimate = estimate_runtime_budget(&input, q8_profile, requested_weight_bits, 4);
    if q8_estimate.total_required_bytes <= requested_available {
        return report(
            snapshot.device,
            requested_capability.device,
            preferred_adapter(execution, requested_capability.cpu_fallback),
            requested_capability.backend_family,
            q8_profile,
            q8_estimate,
            requested_available,
            RuntimeBudgetFallbackReason::None,
            false,
        );
    }

    let q4_estimate = estimate_runtime_budget(&input, RuntimeQuantizationProfile::Q4, 4, 4);
    if q4_estimate.total_required_bytes <= requested_available {
        return report(
            snapshot.device,
            requested_capability.device,
            preferred_adapter(execution, requested_capability.cpu_fallback),
            requested_capability.backend_family,
            RuntimeQuantizationProfile::Q4,
            q4_estimate,
            requested_available,
            RuntimeBudgetFallbackReason::MemoryPressureQuantized,
            false,
        );
    }

    let cpu_capability = RuntimeDeviceCapability::for_device(DeviceClass::CpuOnly);
    let cpu_available = available_budget_bytes(snapshot, DeviceClass::CpuOnly, execution, &input);
    let stub_estimate = estimate_runtime_budget(&input, RuntimeQuantizationProfile::CpuStub, 4, 4);
    report(
        snapshot.device,
        cpu_capability.device,
        cpu_capability.cpu_fallback,
        cpu_capability.backend_family,
        RuntimeQuantizationProfile::CpuStub,
        stub_estimate,
        cpu_available,
        RuntimeBudgetFallbackReason::BudgetExceededCpuStub,
        true,
    )
}

#[derive(Debug, Clone, Copy)]
struct RuntimeBudgetEstimate {
    model_weight_bytes: u64,
    kv_cache_bytes: u64,
    gene_segment_cache_bytes: u64,
    routing_reflection_overhead_bytes: u64,
    total_required_bytes: u64,
}

fn estimate_runtime_budget(
    input: &RuntimeBudgetInput,
    quantization_profile: RuntimeQuantizationProfile,
    kv_cache_quantization_bits: u8,
    gene_cache_quantization_bits: u8,
) -> RuntimeBudgetEstimate {
    let model_weight_bytes = bytes_for_bits(
        input.model_parameter_count,
        quantization_profile.weight_bits(),
    );
    let kv_units = input
        .context_tokens
        .max(1)
        .saturating_mul(input.layer_count.max(1))
        .saturating_mul(input.hidden_size.max(1))
        .saturating_mul(2) as u64;
    let kv_cache_bytes = bytes_for_bits(kv_units, kv_cache_quantization_bits);
    let gene_units = input
        .gene_segment_count
        .max(1)
        .saturating_mul(input.gene_segment_tokens.max(1))
        .saturating_mul(input.hidden_size.max(1)) as u64;
    let gene_segment_cache_bytes = bytes_for_bits(gene_units, gene_cache_quantization_bits);
    let routing_reflection_overhead_bytes = input
        .context_tokens
        .max(1)
        .saturating_mul(input.hidden_size.max(1))
        .saturating_mul(input.reflection_passes.saturating_add(1))
        .saturating_mul(2) as u64
        / 4;
    let total_required_bytes = model_weight_bytes
        .saturating_add(kv_cache_bytes)
        .saturating_add(gene_segment_cache_bytes)
        .saturating_add(routing_reflection_overhead_bytes);

    RuntimeBudgetEstimate {
        model_weight_bytes,
        kv_cache_bytes,
        gene_segment_cache_bytes,
        routing_reflection_overhead_bytes,
        total_required_bytes,
    }
}

fn available_budget_bytes(
    snapshot: HardwareSnapshot,
    selected_device: DeviceClass,
    execution: &DeviceExecutionPlan,
    input: &RuntimeBudgetInput,
) -> u64 {
    if let Some(available) = input.available_memory_bytes {
        return available;
    }

    let capability = RuntimeDeviceCapability::for_device(selected_device);
    let free_fraction = (1.0 - snapshot.ram_load).clamp(0.04, 0.92);
    let mode_fraction = match execution.memory_mode {
        DeviceMemoryMode::GpuResident => 0.78,
        DeviceMemoryMode::UnifiedMemory => 0.74,
        DeviceMemoryMode::DistributedSharded => 0.82,
        DeviceMemoryMode::TieredDisk => 0.62,
        DeviceMemoryMode::MinimalDisk => 0.35,
    };
    let spill_bonus = if execution.allow_disk_spill {
        (1.0 - snapshot.disk_load).clamp(0.0, 1.0) * 0.06
    } else {
        0.0
    };
    let budget_fraction = (free_fraction * (mode_fraction + spill_bonus)).clamp(0.02, 0.88);
    (capability.memory_capacity_bytes as f64 * budget_fraction as f64).round() as u64
}

fn report(
    requested_device: DeviceClass,
    selected_device: DeviceClass,
    selected_adapter: RuntimeAdapterHint,
    backend_family: &'static str,
    quantization_profile: RuntimeQuantizationProfile,
    estimate: RuntimeBudgetEstimate,
    available_budget_bytes: u64,
    fallback_reason: RuntimeBudgetFallbackReason,
    fail_closed_cpu_stub: bool,
) -> RuntimeBudgetReport {
    RuntimeBudgetReport {
        requested_device,
        selected_device,
        selected_adapter,
        backend_family,
        quantization_profile,
        weight_quantization_bits: quantization_profile.weight_bits(),
        kv_cache_quantization_bits: match quantization_profile {
            RuntimeQuantizationProfile::Q8 => 8,
            RuntimeQuantizationProfile::Q4 | RuntimeQuantizationProfile::CpuStub => 4,
        },
        gene_cache_quantization_bits: 4,
        model_weight_bytes: estimate.model_weight_bytes,
        kv_cache_bytes: estimate.kv_cache_bytes,
        gene_segment_cache_bytes: estimate.gene_segment_cache_bytes,
        routing_reflection_overhead_bytes: estimate.routing_reflection_overhead_bytes,
        total_required_bytes: estimate.total_required_bytes,
        available_budget_bytes,
        memory_pressure: memory_pressure(estimate.total_required_bytes, available_budget_bytes),
        fallback_reason,
        fail_closed_cpu_stub,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn preferred_adapter(
    execution: &DeviceExecutionPlan,
    fallback: RuntimeAdapterHint,
) -> RuntimeAdapterHint {
    execution.adapter_hints.first().copied().unwrap_or(fallback)
}

fn bytes_for_bits(units: u64, bits: u8) -> u64 {
    units
        .saturating_mul(bits as u64)
        .saturating_add(7)
        .saturating_div(8)
}

fn memory_pressure(required: u64, available: u64) -> f32 {
    if available == 0 {
        return 9.999;
    }
    ((required as f64 / available as f64).min(9.999)) as f32
}
