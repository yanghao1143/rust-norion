use std::str::FromStr;

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
pub struct HardwarePlan {
    pub device: DeviceClass,
    pub tier: DeviceTier,
    pub pressure: f32,
    pub latency_budget_ms: Option<u64>,
    pub local_kv_token_budget: usize,
    pub global_kv_token_budget: usize,
    pub hierarchy: HierarchyWeights,
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
            "device={} tier={} pressure={:.3} latency_budget_ms={} local_kv_tokens={} global_kv_tokens={} hierarchy=({:.2},{:.2},{:.2})",
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
            self.hierarchy.convolution
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
        let notes = notes(snapshot, profile, pressure, prompt_tokens);

        HardwarePlan {
            device: snapshot.device,
            tier: snapshot.device.tier(),
            pressure,
            latency_budget_ms,
            local_kv_token_budget,
            global_kv_token_budget,
            hierarchy,
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
) -> Vec<String> {
    let mut notes = vec![
        format!("device:{}", snapshot.device.as_str()),
        format!("tier:{}", snapshot.device.tier().as_str()),
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
            assert!((hierarchy_total - 1.0).abs() < 0.001);
            assert!(plan.notes.iter().any(|note| note.starts_with("device:")));
            assert!(plan.notes.iter().any(|note| note.starts_with("tier:")));
        }
    }

    #[test]
    fn load_accepts_percent_values() {
        let snapshot = HardwareSnapshot::new(DeviceClass::Auto, 75.0, 25.0, 50.0, 0.10);

        assert!((snapshot.cpu_load - 0.75).abs() < 0.0001);
        assert!((snapshot.gpu_load - 0.25).abs() < 0.0001);
        assert!((snapshot.ram_load - 0.50).abs() < 0.0001);
        assert!((snapshot.disk_load - 0.10).abs() < 0.0001);
    }
}
