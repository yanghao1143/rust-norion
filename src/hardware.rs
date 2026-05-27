use std::env;
use std::str::FromStr;
use std::thread;

use crate::hierarchy::{HierarchyWeights, TaskProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Auto,
    CpuOnly,
    IntegratedGpu,
    DiscreteGpu,
    UnifiedMemory,
    Mobile,
    Embedded,
    NpuAccelerator,
    MultiGpu,
    Edge,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTier {
    Auto,
    Tiny,
    Constrained,
    Balanced,
    Accelerated,
    Distributed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeLane {
    CpuPortable,
    CpuVector,
    IntegratedGpu,
    DiscreteGpu,
    UnifiedMemoryGpu,
    NeuralAccelerator,
    MultiAccelerator,
    DiskBackedStreaming,
}

impl ComputeLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CpuPortable => "cpu-portable",
            Self::CpuVector => "cpu-vector",
            Self::IntegratedGpu => "integrated-gpu",
            Self::DiscreteGpu => "discrete-gpu",
            Self::UnifiedMemoryGpu => "unified-memory-gpu",
            Self::NeuralAccelerator => "neural-accelerator",
            Self::MultiAccelerator => "multi-accelerator",
            Self::DiskBackedStreaming => "disk-backed-streaming",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceMemoryMode {
    MinimalDisk,
    TieredDisk,
    UnifiedMemory,
    GpuResident,
    DistributedSharded,
}

impl DeviceMemoryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MinimalDisk => "minimal-disk",
            Self::TieredDisk => "tiered-disk",
            Self::UnifiedMemory => "unified-memory",
            Self::GpuResident => "gpu-resident",
            Self::DistributedSharded => "distributed-sharded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAdapterHint {
    PortableRust,
    CpuSimd,
    Wgpu,
    Vulkan,
    Metal,
    Cuda,
    Rocm,
    DirectMl,
    CoreMl,
    Nnapi,
    Qnn,
    OpenVino,
    MultiDevice,
}

impl RuntimeAdapterHint {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PortableRust => "portable-rust",
            Self::CpuSimd => "cpu-simd",
            Self::Wgpu => "wgpu",
            Self::Vulkan => "vulkan",
            Self::Metal => "metal",
            Self::Cuda => "cuda",
            Self::Rocm => "rocm",
            Self::DirectMl => "directml",
            Self::CoreMl => "coreml",
            Self::Nnapi => "nnapi",
            Self::Qnn => "qnn",
            Self::OpenVino => "openvino",
            Self::MultiDevice => "multi-device",
        }
    }
}

impl DeviceTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Tiny => "tiny",
            Self::Constrained => "constrained",
            Self::Balanced => "balanced",
            Self::Accelerated => "accelerated",
            Self::Distributed => "distributed",
        }
    }
}

impl DeviceClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::CpuOnly => "cpu",
            Self::IntegratedGpu => "integrated",
            Self::DiscreteGpu => "discrete",
            Self::UnifiedMemory => "uma",
            Self::Mobile => "mobile",
            Self::Embedded => "embedded",
            Self::NpuAccelerator => "npu",
            Self::MultiGpu => "multi-gpu",
            Self::Edge => "edge",
            Self::Server => "server",
        }
    }

    pub fn supported_profiles() -> &'static [Self] {
        const PROFILES: [DeviceClass; 11] = [
            DeviceClass::Auto,
            DeviceClass::CpuOnly,
            DeviceClass::IntegratedGpu,
            DeviceClass::DiscreteGpu,
            DeviceClass::UnifiedMemory,
            DeviceClass::Mobile,
            DeviceClass::Embedded,
            DeviceClass::NpuAccelerator,
            DeviceClass::MultiGpu,
            DeviceClass::Edge,
            DeviceClass::Server,
        ];

        &PROFILES
    }

    pub fn explicit_profiles() -> &'static [Self] {
        const PROFILES: [DeviceClass; 10] = [
            DeviceClass::CpuOnly,
            DeviceClass::IntegratedGpu,
            DeviceClass::DiscreteGpu,
            DeviceClass::UnifiedMemory,
            DeviceClass::Mobile,
            DeviceClass::Embedded,
            DeviceClass::NpuAccelerator,
            DeviceClass::MultiGpu,
            DeviceClass::Edge,
            DeviceClass::Server,
        ];

        &PROFILES
    }

    pub fn tier(self) -> DeviceTier {
        match self {
            Self::Auto => DeviceTier::Auto,
            Self::Embedded => DeviceTier::Tiny,
            Self::CpuOnly | Self::Mobile | Self::Edge => DeviceTier::Constrained,
            Self::IntegratedGpu | Self::UnifiedMemory | Self::NpuAccelerator => {
                DeviceTier::Balanced
            }
            Self::DiscreteGpu | Self::Server => DeviceTier::Accelerated,
            Self::MultiGpu => DeviceTier::Distributed,
        }
    }
}

impl FromStr for DeviceClass {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "cpu" | "cpu-only" | "cpu_only" | "pc-cpu" | "desktop-cpu" => Ok(Self::CpuOnly),
            "integrated" | "igpu" | "integrated-gpu" | "laptop" | "notebook" | "intel-gpu"
            | "amd-apu" | "apu" => Ok(Self::IntegratedGpu),
            "discrete" | "dgpu" | "discrete-gpu" | "desktop-gpu" | "cuda" | "rtx" | "nvidia"
            | "radeon" => Ok(Self::DiscreteGpu),
            "uma" | "unified" | "unified-memory" | "apple" | "mac" | "macbook" | "m-series"
            | "m1" | "m2" | "m3" | "m4" => Ok(Self::UnifiedMemory),
            "mobile" | "phone" | "tablet" | "android" | "ios" | "handheld" | "iphone" | "ipad"
            | "smartphone" => Ok(Self::Mobile),
            "embedded" | "iot" | "rpi" | "raspberry-pi" | "raspberry_pi" | "micro" => {
                Ok(Self::Embedded)
            }
            "npu"
            | "ane"
            | "tpu"
            | "ai-accelerator"
            | "ai_accelerator"
            | "neural"
            | "snapdragon"
            | "qualcomm"
            | "apple-neural-engine" => Ok(Self::NpuAccelerator),
            "multi-gpu" | "multi_gpu" | "multi" | "multi-accelerator" | "distributed"
            | "cluster" => Ok(Self::MultiGpu),
            "edge" | "gateway" | "edge-gateway" | "jetson" => Ok(Self::Edge),
            "server" | "workstation" | "rack" | "datacenter" | "local-cloud" => Ok(Self::Server),
            other => Err(format!("unknown device class: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HardwareSnapshot {
    pub device: DeviceClass,
    pub cpu_load: f32,
    pub gpu_load: f32,
    pub ram_load: f32,
    pub disk_load: f32,
}

impl Default for HardwareSnapshot {
    fn default() -> Self {
        Self {
            device: DeviceClass::Auto,
            cpu_load: 0.20,
            gpu_load: 0.20,
            ram_load: 0.35,
            disk_load: 0.15,
        }
    }
}

impl HardwareSnapshot {
    pub fn new(
        device: DeviceClass,
        cpu_load: f32,
        gpu_load: f32,
        ram_load: f32,
        disk_load: f32,
    ) -> Self {
        Self {
            device,
            cpu_load: normalize_load(cpu_load),
            gpu_load: normalize_load(gpu_load),
            ram_load: normalize_load(ram_load),
            disk_load: normalize_load(disk_load),
        }
    }

    pub fn auto_detect() -> Self {
        HardwareProbe::current().snapshot()
    }

    pub fn pressure(&self) -> f32 {
        let weights = device_pressure_weights(self.device);
        (self.cpu_load * weights.cpu
            + self.gpu_load * weights.gpu
            + self.ram_load * weights.ram
            + self.disk_load * weights.disk)
            .clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone)]
pub struct HardwareProbe {
    os: String,
    arch: String,
    cpu_count: usize,
    env: Vec<(String, String)>,
}

impl HardwareProbe {
    pub fn current() -> Self {
        Self {
            os: env::consts::OS.to_owned(),
            arch: env::consts::ARCH.to_owned(),
            cpu_count: thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(1),
            env: env::vars().collect(),
        }
    }

    pub fn new(os: impl Into<String>, arch: impl Into<String>, cpu_count: usize) -> Self {
        Self {
            os: os.into(),
            arch: arch.into(),
            cpu_count: cpu_count.max(1),
            env: Vec::new(),
        }
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn snapshot(&self) -> HardwareSnapshot {
        let device = self.detect_device();
        let defaults = default_probe_loads(device);

        HardwareSnapshot::new(
            device,
            self.load_hint("NOIRON_CPU_LOAD", defaults.cpu),
            self.load_hint("NOIRON_GPU_LOAD", defaults.gpu),
            self.load_hint("NOIRON_RAM_LOAD", defaults.ram),
            self.load_hint("NOIRON_DISK_LOAD", defaults.disk),
        )
    }

    pub fn detect_device(&self) -> DeviceClass {
        if let Some(device) = self
            .env_value("NOIRON_DEVICE_PROFILE")
            .and_then(|value| value.parse::<DeviceClass>().ok())
            .filter(|device| *device != DeviceClass::Auto)
        {
            return device;
        }

        let os = self.os.to_ascii_lowercase();
        let arch = self.arch.to_ascii_lowercase();

        if os == "android" || os == "ios" {
            return DeviceClass::Mobile;
        }
        if arch.starts_with("wasm") {
            return DeviceClass::Embedded;
        }
        if self.has_npu_hint() {
            return DeviceClass::NpuAccelerator;
        }

        let accelerator_count = self.accelerator_count();
        if accelerator_count > 1 {
            return DeviceClass::MultiGpu;
        }
        if accelerator_count == 1 {
            if self.has_unified_memory_hint() {
                return DeviceClass::UnifiedMemory;
            }
            if self.has_integrated_gpu_hint() {
                return DeviceClass::IntegratedGpu;
            }
            return DeviceClass::DiscreteGpu;
        }

        if self.has_unified_memory_hint() || (os == "macos" && is_arm_arch(&arch)) {
            return DeviceClass::UnifiedMemory;
        }
        if self.has_integrated_gpu_hint() {
            return DeviceClass::IntegratedGpu;
        }
        if os == "linux" && is_arm_arch(&arch) {
            return if self.cpu_count <= 4 {
                DeviceClass::Embedded
            } else {
                DeviceClass::Edge
            };
        }
        if self.cpu_count >= 32 {
            return DeviceClass::Server;
        }

        DeviceClass::CpuOnly
    }

    fn load_hint(&self, key: &str, fallback: f32) -> f32 {
        self.env_value(key)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(fallback)
    }

    fn env_value(&self, key: &str) -> Option<&str> {
        self.env
            .iter()
            .find(|(env_key, _)| env_key == key)
            .map(|(_, value)| value.as_str())
    }

    fn env_value_any(&self, keys: &[&str]) -> Option<&str> {
        keys.iter().find_map(|key| self.env_value(key))
    }

    fn has_npu_hint(&self) -> bool {
        self.env_flag("NOIRON_NPU")
            || self
                .env_value_any(&[
                    "QNN_SDK_ROOT",
                    "HEXAGON_SDK_ROOT",
                    "COREML_ENABLE_NEURAL_ENGINE",
                    "ANDROID_NNAPI_DEVICE",
                ])
                .is_some()
            || self.adapter_hint_contains(&["npu", "neural", "ane", "hexagon", "qnn"])
    }

    fn has_unified_memory_hint(&self) -> bool {
        self.env_flag("NOIRON_UNIFIED_MEMORY")
            || self.adapter_hint_contains(&["apple", "m1", "m2", "m3", "m4", "unified"])
    }

    fn has_integrated_gpu_hint(&self) -> bool {
        self.env_flag("NOIRON_INTEGRATED_GPU")
            || self.adapter_hint_contains(&["integrated", "iris", "uhd", "intel", "apu"])
    }

    fn adapter_hint_contains(&self, needles: &[&str]) -> bool {
        self.env_value_any(&[
            "NOIRON_GPU_ADAPTER",
            "WGPU_ADAPTER_NAME",
            "GPU_DEVICE_NAME",
            "DXGI_ADAPTER_NAME",
            "METAL_DEVICE_NAME",
        ])
        .map(|value| {
            let lower = value.to_ascii_lowercase();
            needles.iter().any(|needle| lower.contains(needle))
        })
        .unwrap_or(false)
    }

    fn env_flag(&self, key: &str) -> bool {
        self.env_value(key)
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false)
    }

    fn accelerator_count(&self) -> usize {
        self.env_value_any(&[
            "NOIRON_ACCELERATOR_DEVICES",
            "CUDA_VISIBLE_DEVICES",
            "NVIDIA_VISIBLE_DEVICES",
            "HIP_VISIBLE_DEVICES",
            "ROCR_VISIBLE_DEVICES",
            "ONEAPI_DEVICE_SELECTOR",
        ])
        .map(count_visible_devices)
        .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy)]
struct ProbeLoads {
    cpu: f32,
    gpu: f32,
    ram: f32,
    disk: f32,
}

#[derive(Debug, Clone)]
pub struct DeviceExecutionPlan {
    pub primary_lane: ComputeLane,
    pub fallback_lane: ComputeLane,
    pub memory_mode: DeviceMemoryMode,
    pub adapter_hints: Vec<RuntimeAdapterHint>,
    pub max_parallel_chunks: usize,
    pub kv_prefetch_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub allow_disk_spill: bool,
}

impl DeviceExecutionPlan {
    pub fn summary(&self) -> String {
        let adapters = self
            .adapter_hints
            .iter()
            .map(|adapter| adapter.as_str())
            .collect::<Vec<_>>()
            .join("+");
        format!(
            "primary={} fallback={} memory={} adapters={} parallel_chunks={} kv_prefetch={} kv_bits={}/{} disk_spill={}",
            self.primary_lane.as_str(),
            self.fallback_lane.as_str(),
            self.memory_mode.as_str(),
            adapters,
            self.max_parallel_chunks,
            self.kv_prefetch_blocks,
            self.hot_kv_precision_bits,
            self.cold_kv_precision_bits,
            self.allow_disk_spill
        )
    }
}

#[derive(Debug, Clone)]
pub struct HardwarePlan {
    pub device: DeviceClass,
    pub tier: DeviceTier,
    pub pressure: f32,
    pub latency_budget_ms: Option<u64>,
    pub local_kv_token_budget: usize,
    pub global_kv_token_budget: usize,
    pub hierarchy: HierarchyWeights,
    pub execution: DeviceExecutionPlan,
    pub notes: Vec<String>,
}

impl Default for HardwarePlan {
    fn default() -> Self {
        HardwareAllocator::new().plan(
            HardwareSnapshot::default(),
            TaskProfile::General,
            0,
            HierarchyWeights::default(),
        )
    }
}

impl HardwarePlan {
    pub fn summary(&self) -> String {
        format!(
            "device={} tier={} pressure={:.3} latency_budget_ms={} local_kv_tokens={} global_kv_tokens={} hierarchy=({:.2},{:.2},{:.2}) execution=({})",
            self.device.as_str(),
            self.tier.as_str(),
            self.pressure,
            self.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.local_kv_token_budget,
            self.global_kv_token_budget,
            self.hierarchy.global,
            self.hierarchy.local,
            self.hierarchy.convolution,
            self.execution.summary()
        )
    }
}

#[derive(Debug, Clone)]
pub struct HardwareAllocator {
    base_local_tokens: usize,
    base_global_tokens: usize,
}

impl Default for HardwareAllocator {
    fn default() -> Self {
        Self {
            base_local_tokens: 512,
            base_global_tokens: 4096,
        }
    }
}

impl HardwareAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn plan(
        &self,
        snapshot: HardwareSnapshot,
        profile: TaskProfile,
        prompt_tokens: usize,
        base_hierarchy: HierarchyWeights,
    ) -> HardwarePlan {
        let pressure = snapshot.pressure();
        let device_scale = device_budget_scale(snapshot.device);
        let pressure_scale = (1.0 - pressure * 0.62).clamp(0.24, 1.0);
        let long_context_scale = if prompt_tokens >= 32_000 {
            0.70
        } else if prompt_tokens >= 8_192 {
            0.82
        } else {
            1.0
        };
        let local_kv_token_budget = scaled_tokens(
            self.base_local_tokens,
            device_scale.local * pressure_scale * long_context_scale,
        );
        let global_kv_token_budget = scaled_tokens(
            self.base_global_tokens,
            device_scale.global * pressure_scale * long_context_scale,
        );
        let latency_budget_ms = latency_budget(snapshot.device, pressure);
        let hierarchy = adapt_hierarchy(base_hierarchy, snapshot.device, profile, pressure);
        let execution = device_execution_plan(snapshot.device, pressure);
        let notes = notes(snapshot, profile, pressure, prompt_tokens, &execution);

        HardwarePlan {
            device: snapshot.device,
            tier: snapshot.device.tier(),
            pressure,
            latency_budget_ms,
            local_kv_token_budget,
            global_kv_token_budget,
            hierarchy,
            execution,
            notes,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PressureWeights {
    cpu: f32,
    gpu: f32,
    ram: f32,
    disk: f32,
}

#[derive(Debug, Clone, Copy)]
struct BudgetScale {
    local: f32,
    global: f32,
}

fn device_pressure_weights(device: DeviceClass) -> PressureWeights {
    match device {
        DeviceClass::CpuOnly => PressureWeights {
            cpu: 0.46,
            gpu: 0.04,
            ram: 0.32,
            disk: 0.18,
        },
        DeviceClass::IntegratedGpu | DeviceClass::UnifiedMemory => PressureWeights {
            cpu: 0.26,
            gpu: 0.24,
            ram: 0.36,
            disk: 0.14,
        },
        DeviceClass::DiscreteGpu => PressureWeights {
            cpu: 0.18,
            gpu: 0.42,
            ram: 0.26,
            disk: 0.14,
        },
        DeviceClass::Mobile => PressureWeights {
            cpu: 0.28,
            gpu: 0.18,
            ram: 0.42,
            disk: 0.12,
        },
        DeviceClass::Embedded => PressureWeights {
            cpu: 0.42,
            gpu: 0.06,
            ram: 0.40,
            disk: 0.12,
        },
        DeviceClass::NpuAccelerator => PressureWeights {
            cpu: 0.18,
            gpu: 0.34,
            ram: 0.36,
            disk: 0.12,
        },
        DeviceClass::MultiGpu => PressureWeights {
            cpu: 0.16,
            gpu: 0.46,
            ram: 0.22,
            disk: 0.16,
        },
        DeviceClass::Edge => PressureWeights {
            cpu: 0.34,
            gpu: 0.12,
            ram: 0.38,
            disk: 0.16,
        },
        DeviceClass::Server => PressureWeights {
            cpu: 0.24,
            gpu: 0.34,
            ram: 0.24,
            disk: 0.18,
        },
        DeviceClass::Auto => PressureWeights {
            cpu: 0.25,
            gpu: 0.25,
            ram: 0.34,
            disk: 0.16,
        },
    }
}

fn device_budget_scale(device: DeviceClass) -> BudgetScale {
    match device {
        DeviceClass::CpuOnly => BudgetScale {
            local: 0.62,
            global: 0.48,
        },
        DeviceClass::IntegratedGpu => BudgetScale {
            local: 0.82,
            global: 0.70,
        },
        DeviceClass::UnifiedMemory => BudgetScale {
            local: 1.15,
            global: 1.20,
        },
        DeviceClass::DiscreteGpu => BudgetScale {
            local: 1.25,
            global: 1.10,
        },
        DeviceClass::Mobile => BudgetScale {
            local: 0.55,
            global: 0.42,
        },
        DeviceClass::Embedded => BudgetScale {
            local: 0.36,
            global: 0.28,
        },
        DeviceClass::NpuAccelerator => BudgetScale {
            local: 0.95,
            global: 0.78,
        },
        DeviceClass::MultiGpu => BudgetScale {
            local: 2.20,
            global: 2.40,
        },
        DeviceClass::Edge => BudgetScale {
            local: 0.48,
            global: 0.36,
        },
        DeviceClass::Server => BudgetScale {
            local: 1.50,
            global: 1.60,
        },
        DeviceClass::Auto => BudgetScale {
            local: 1.0,
            global: 1.0,
        },
    }
}

fn latency_budget(device: DeviceClass, pressure: f32) -> Option<u64> {
    if pressure < 0.45 {
        return None;
    }

    let base: u64 = match device {
        DeviceClass::Embedded => 90,
        DeviceClass::Mobile => 110,
        DeviceClass::Edge => 120,
        DeviceClass::CpuOnly => 160,
        DeviceClass::IntegratedGpu => 220,
        DeviceClass::NpuAccelerator => 240,
        DeviceClass::UnifiedMemory => 260,
        DeviceClass::DiscreteGpu => 320,
        DeviceClass::Server => 420,
        DeviceClass::MultiGpu => 520,
        DeviceClass::Auto => 240,
    };
    let pressure_discount = ((pressure - 0.45) * 180.0).round() as u64;
    Some(base.saturating_sub(pressure_discount).max(80))
}

fn device_execution_plan(device: DeviceClass, pressure: f32) -> DeviceExecutionPlan {
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
    let hot_kv_precision_bits = if matches!(device, DeviceClass::Embedded) || pressure >= 0.88 {
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
                RuntimeAdapterHint::Nnapi,
                RuntimeAdapterHint::Qnn,
            ],
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
                RuntimeAdapterHint::OpenVino,
                RuntimeAdapterHint::Wgpu,
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
                RuntimeAdapterHint::Vulkan,
                RuntimeAdapterHint::Wgpu,
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

fn adapt_hierarchy(
    mut hierarchy: HierarchyWeights,
    device: DeviceClass,
    profile: TaskProfile,
    pressure: f32,
) -> HierarchyWeights {
    match device {
        DeviceClass::CpuOnly | DeviceClass::Edge | DeviceClass::Mobile => {
            hierarchy.local += 0.08;
            hierarchy.convolution += 0.10 + pressure * 0.12;
            hierarchy.global -= pressure * 0.10;
        }
        DeviceClass::Embedded => {
            hierarchy.local += 0.06;
            hierarchy.convolution += 0.18 + pressure * 0.16;
            hierarchy.global -= pressure * 0.14;
        }
        DeviceClass::IntegratedGpu | DeviceClass::UnifiedMemory => {
            hierarchy.local += 0.04;
            hierarchy.convolution += pressure * 0.08;
        }
        DeviceClass::NpuAccelerator => {
            hierarchy.local += 0.05;
            hierarchy.convolution += pressure * 0.05;
            hierarchy.global += 0.02 * (1.0 - pressure);
        }
        DeviceClass::DiscreteGpu | DeviceClass::Server | DeviceClass::MultiGpu => {
            hierarchy.global += 0.04 * (1.0 - pressure);
            hierarchy.local += 0.03;
        }
        DeviceClass::Auto => {
            hierarchy.convolution += pressure * 0.06;
        }
    }

    if profile == TaskProfile::LongDocument {
        hierarchy.convolution += 0.06;
    }
    if device == DeviceClass::MultiGpu && pressure < 0.45 {
        hierarchy.global += 0.05;
    }

    hierarchy.normalize();
    hierarchy
}

fn notes(
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
        DeviceClass::NpuAccelerator => {
            notes.push("device_policy:npu_gpu_load_as_accelerator_pressure".to_owned());
        }
        DeviceClass::MultiGpu => notes.push("device_policy:multi_gpu_expand_global_kv".to_owned()),
        _ => {}
    }

    notes
}

fn scaled_tokens(base: usize, scale: f32) -> usize {
    ((base as f32 * scale).round() as usize).max(32)
}

fn normalize_load(value: f32) -> f32 {
    if value > 1.0 {
        (value / 100.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn default_probe_loads(device: DeviceClass) -> ProbeLoads {
    match device {
        DeviceClass::Mobile => ProbeLoads {
            cpu: 0.30,
            gpu: 0.20,
            ram: 0.55,
            disk: 0.10,
        },
        DeviceClass::Embedded => ProbeLoads {
            cpu: 0.35,
            gpu: 0.05,
            ram: 0.60,
            disk: 0.15,
        },
        DeviceClass::Edge => ProbeLoads {
            cpu: 0.32,
            gpu: 0.15,
            ram: 0.48,
            disk: 0.18,
        },
        DeviceClass::NpuAccelerator => ProbeLoads {
            cpu: 0.22,
            gpu: 0.28,
            ram: 0.42,
            disk: 0.12,
        },
        DeviceClass::MultiGpu => ProbeLoads {
            cpu: 0.18,
            gpu: 0.24,
            ram: 0.28,
            disk: 0.12,
        },
        DeviceClass::Server => ProbeLoads {
            cpu: 0.18,
            gpu: 0.22,
            ram: 0.30,
            disk: 0.16,
        },
        _ => ProbeLoads {
            cpu: 0.20,
            gpu: 0.20,
            ram: 0.35,
            disk: 0.15,
        },
    }
}

fn count_visible_devices(value: &str) -> usize {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || matches!(
            trimmed.to_ascii_lowercase().as_str(),
            "none" | "void" | "disabled" | "-1"
        )
    {
        return 0;
    }
    if trimmed.eq_ignore_ascii_case("all") {
        return 1;
    }

    trimmed
        .split([',', ';'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .filter(|item| *item != "-1")
        .count()
}

fn is_arm_arch(arch: &str) -> bool {
    arch.contains("arm") || arch.contains("aarch64")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_only_high_pressure_tightens_budget_and_boosts_convolution() {
        let allocator = HardwareAllocator::new();
        let base = HierarchyWeights::new(0.3, 0.4, 0.3);

        let plan = allocator.plan(
            HardwareSnapshot::new(DeviceClass::CpuOnly, 0.92, 0.0, 0.88, 0.40),
            TaskProfile::LongDocument,
            16_384,
            base,
        );

        assert!(plan.latency_budget_ms.is_some());
        assert!(plan.local_kv_token_budget < 512);
        assert!(plan.global_kv_token_budget < 4096);
        assert!(plan.hierarchy.convolution > base.convolution);
    }

    #[test]
    fn server_low_pressure_expands_kv_budget() {
        let allocator = HardwareAllocator::new();

        let plan = allocator.plan(
            HardwareSnapshot::new(DeviceClass::Server, 0.10, 0.20, 0.18, 0.08),
            TaskProfile::Coding,
            1024,
            HierarchyWeights::new(0.2, 0.6, 0.2),
        );

        assert!(plan.latency_budget_ms.is_none());
        assert!(plan.local_kv_token_budget > 512);
        assert!(plan.global_kv_token_budget > 4096);
    }

    #[test]
    fn mobile_and_embedded_profiles_tighten_kv_budgets() {
        let allocator = HardwareAllocator::new();
        let base = HierarchyWeights::new(0.30, 0.40, 0.30);

        let mobile = allocator.plan(
            HardwareSnapshot::new(DeviceClass::Mobile, 0.30, 0.35, 0.82, 0.10),
            TaskProfile::General,
            2048,
            base,
        );
        let embedded = allocator.plan(
            HardwareSnapshot::new(DeviceClass::Embedded, 0.45, 0.0, 0.80, 0.20),
            TaskProfile::LongDocument,
            2048,
            base,
        );

        assert!(mobile.local_kv_token_budget < 512);
        assert!(mobile.global_kv_token_budget < 4096);
        assert!(mobile.hierarchy.convolution > base.convolution);
        assert!(embedded.local_kv_token_budget < mobile.local_kv_token_budget);
        assert!(embedded.global_kv_token_budget < mobile.global_kv_token_budget);
    }

    #[test]
    fn accelerator_profiles_parse_and_expand_when_capacity_exists() {
        let allocator = HardwareAllocator::new();
        let multi_gpu = "cluster".parse::<DeviceClass>().unwrap();
        let npu = "ane".parse::<DeviceClass>().unwrap();

        assert_eq!(multi_gpu, DeviceClass::MultiGpu);
        assert_eq!(npu, DeviceClass::NpuAccelerator);

        let plan = allocator.plan(
            HardwareSnapshot::new(multi_gpu, 0.12, 0.18, 0.20, 0.12),
            TaskProfile::LongDocument,
            4096,
            HierarchyWeights::new(0.30, 0.40, 0.30),
        );

        assert!(plan.local_kv_token_budget > 512);
        assert!(plan.global_kv_token_budget > 4096);
        assert!(plan.hierarchy.global > 0.30);
        assert!(plan.latency_budget_ms.is_none());
    }

    #[test]
    fn common_device_aliases_parse_to_profiles() {
        assert_eq!(
            "laptop".parse::<DeviceClass>().unwrap(),
            DeviceClass::IntegratedGpu
        );
        assert_eq!(
            "rtx".parse::<DeviceClass>().unwrap(),
            DeviceClass::DiscreteGpu
        );
        assert_eq!(
            "macbook".parse::<DeviceClass>().unwrap(),
            DeviceClass::UnifiedMemory
        );
        assert_eq!(
            "iphone".parse::<DeviceClass>().unwrap(),
            DeviceClass::Mobile
        );
        assert_eq!(
            "snapdragon".parse::<DeviceClass>().unwrap(),
            DeviceClass::NpuAccelerator
        );
        assert_eq!("jetson".parse::<DeviceClass>().unwrap(), DeviceClass::Edge);
        assert_eq!(
            "datacenter".parse::<DeviceClass>().unwrap(),
            DeviceClass::Server
        );
    }

    #[test]
    fn every_supported_device_profile_has_a_plan() {
        let allocator = HardwareAllocator::new();
        let base = HierarchyWeights::new(0.30, 0.40, 0.30);

        for device in DeviceClass::supported_profiles() {
            let plan = allocator.plan(
                HardwareSnapshot::new(*device, 0.35, 0.30, 0.45, 0.20),
                TaskProfile::General,
                4096,
                base,
            );
            let hierarchy_total =
                plan.hierarchy.global + plan.hierarchy.local + plan.hierarchy.convolution;

            assert_eq!(plan.device, *device);
            assert_eq!(plan.tier, device.tier());
            assert!(plan.local_kv_token_budget >= 32);
            assert!(plan.global_kv_token_budget >= 32);
            assert!(plan.execution.max_parallel_chunks >= 1);
            assert!(plan.execution.kv_prefetch_blocks >= 1);
            assert!(!plan.execution.adapter_hints.is_empty());
            assert!(matches!(plan.execution.hot_kv_precision_bits, 4 | 8));
            assert_eq!(plan.execution.cold_kv_precision_bits, 4);
            assert!((hierarchy_total - 1.0).abs() < 0.001);
            assert!(plan.notes.iter().any(|note| note.starts_with("device:")));
            assert!(plan.notes.iter().any(|note| note.starts_with("tier:")));
            assert!(plan.notes.iter().any(|note| note.starts_with("execution:")));
            assert!(
                plan.notes
                    .iter()
                    .any(|note| note.starts_with("memory_mode:"))
            );
        }
    }

    #[test]
    fn execution_profiles_map_devices_to_portable_fallbacks() {
        let allocator = HardwareAllocator::new();
        let base = HierarchyWeights::new(0.30, 0.40, 0.30);

        let embedded = allocator.plan(
            HardwareSnapshot::new(DeviceClass::Embedded, 0.40, 0.0, 0.70, 0.30),
            TaskProfile::General,
            2048,
            base,
        );
        let mobile = allocator.plan(
            HardwareSnapshot::new(DeviceClass::Mobile, 0.30, 0.20, 0.50, 0.10),
            TaskProfile::General,
            2048,
            base,
        );
        let multi_gpu = allocator.plan(
            HardwareSnapshot::new(DeviceClass::MultiGpu, 0.12, 0.20, 0.20, 0.10),
            TaskProfile::Coding,
            2048,
            base,
        );
        let uma = allocator.plan(
            HardwareSnapshot::new(DeviceClass::UnifiedMemory, 0.16, 0.20, 0.26, 0.10),
            TaskProfile::LongDocument,
            2048,
            base,
        );

        assert_eq!(
            embedded.execution.primary_lane,
            ComputeLane::DiskBackedStreaming
        );
        assert_eq!(embedded.execution.fallback_lane, ComputeLane::CpuPortable);
        assert_eq!(
            embedded.execution.memory_mode,
            DeviceMemoryMode::MinimalDisk
        );
        assert_eq!(embedded.execution.hot_kv_precision_bits, 4);
        assert!(embedded.execution.allow_disk_spill);

        assert_eq!(mobile.execution.primary_lane, ComputeLane::IntegratedGpu);
        assert!(
            mobile
                .execution
                .adapter_hints
                .contains(&RuntimeAdapterHint::Nnapi)
        );
        assert!(
            mobile
                .execution
                .adapter_hints
                .contains(&RuntimeAdapterHint::Qnn)
        );

        assert_eq!(
            multi_gpu.execution.primary_lane,
            ComputeLane::MultiAccelerator
        );
        assert_eq!(
            multi_gpu.execution.memory_mode,
            DeviceMemoryMode::DistributedSharded
        );
        assert!(
            multi_gpu
                .execution
                .adapter_hints
                .contains(&RuntimeAdapterHint::MultiDevice)
        );
        assert!(!multi_gpu.execution.allow_disk_spill);

        assert_eq!(uma.execution.primary_lane, ComputeLane::UnifiedMemoryGpu);
        assert_eq!(uma.execution.memory_mode, DeviceMemoryMode::UnifiedMemory);
        assert!(
            uma.execution
                .adapter_hints
                .contains(&RuntimeAdapterHint::Metal)
        );
    }

    #[test]
    fn execution_budget_degrades_under_pressure() {
        let allocator = HardwareAllocator::new();
        let base = HierarchyWeights::new(0.30, 0.40, 0.30);

        let calm = allocator.plan(
            HardwareSnapshot::new(DeviceClass::Server, 0.10, 0.15, 0.20, 0.10),
            TaskProfile::Coding,
            1024,
            base,
        );
        let overloaded = allocator.plan(
            HardwareSnapshot::new(DeviceClass::Server, 0.95, 0.95, 0.90, 0.80),
            TaskProfile::Coding,
            1024,
            base,
        );

        assert!(calm.execution.max_parallel_chunks > overloaded.execution.max_parallel_chunks);
        assert!(calm.execution.kv_prefetch_blocks > overloaded.execution.kv_prefetch_blocks);
        assert_eq!(overloaded.execution.max_parallel_chunks, 1);
        assert_eq!(overloaded.execution.kv_prefetch_blocks, 1);
        assert_eq!(overloaded.execution.hot_kv_precision_bits, 4);
    }

    #[test]
    fn load_accepts_percent_values() {
        let snapshot = HardwareSnapshot::new(DeviceClass::Auto, 75.0, 25.0, 50.0, 0.10);

        assert!((snapshot.cpu_load - 0.75).abs() < 0.0001);
        assert!((snapshot.gpu_load - 0.25).abs() < 0.0001);
        assert!((snapshot.ram_load - 0.50).abs() < 0.0001);
        assert!((snapshot.disk_load - 0.10).abs() < 0.0001);
    }

    #[test]
    fn probe_prefers_explicit_environment_profile() {
        let snapshot = HardwareProbe::new("windows", "x86_64", 8)
            .with_env("NOIRON_DEVICE_PROFILE", "rtx")
            .with_env("NOIRON_CPU_LOAD", "80")
            .snapshot();

        assert_eq!(snapshot.device, DeviceClass::DiscreteGpu);
        assert!((snapshot.cpu_load - 0.80).abs() < 0.0001);
    }

    #[test]
    fn probe_detects_mobile_arm_and_multi_gpu_targets() {
        let mobile = HardwareProbe::new("ios", "aarch64", 6).detect_device();
        let multi_gpu = HardwareProbe::new("linux", "x86_64", 32)
            .with_env("CUDA_VISIBLE_DEVICES", "0,1")
            .detect_device();

        assert_eq!(mobile, DeviceClass::Mobile);
        assert_eq!(multi_gpu, DeviceClass::MultiGpu);
    }

    #[test]
    fn probe_detects_unified_integrated_and_edge_targets() {
        let uma = HardwareProbe::new("macos", "aarch64", 10).detect_device();
        let integrated = HardwareProbe::new("windows", "x86_64", 8)
            .with_env("WGPU_ADAPTER_NAME", "Intel Iris Xe Graphics")
            .detect_device();
        let edge = HardwareProbe::new("linux", "aarch64", 8).detect_device();

        assert_eq!(uma, DeviceClass::UnifiedMemory);
        assert_eq!(integrated, DeviceClass::IntegratedGpu);
        assert_eq!(edge, DeviceClass::Edge);
    }
}
