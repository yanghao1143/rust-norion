use std::str::FromStr;

use crate::adapter::{AdapterExecutionContext, AdapterExecutionContextSummary, RuntimeAdapter};
use crate::engine::{
    InferenceError, RuntimeFailureBatchSummary, RuntimeFailureReport, RuntimeFailureSummary,
};
use crate::profile::{HierarchyWeights, TaskProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceClass {
    Auto,
    CpuOnly,
    IntegratedGpu,
    DiscreteGpu,
    UnifiedMemory,
    Mobile,
    Embedded,
    BrowserWasm,
    Microcontroller,
    NpuAccelerator,
    MultiGpu,
    Edge,
    Server,
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
            Self::BrowserWasm => "browser-wasm",
            Self::Microcontroller => "microcontroller",
            Self::NpuAccelerator => "npu",
            Self::MultiGpu => "multi-gpu",
            Self::Edge => "edge",
            Self::Server => "server",
        }
    }

    pub fn supported_profiles() -> &'static [Self] {
        const PROFILES: [DeviceClass; 13] = [
            DeviceClass::Auto,
            DeviceClass::CpuOnly,
            DeviceClass::IntegratedGpu,
            DeviceClass::DiscreteGpu,
            DeviceClass::UnifiedMemory,
            DeviceClass::Mobile,
            DeviceClass::Embedded,
            DeviceClass::BrowserWasm,
            DeviceClass::Microcontroller,
            DeviceClass::NpuAccelerator,
            DeviceClass::MultiGpu,
            DeviceClass::Edge,
            DeviceClass::Server,
        ];

        &PROFILES
    }

    pub fn explicit_profiles() -> &'static [Self] {
        const PROFILES: [DeviceClass; 12] = [
            DeviceClass::CpuOnly,
            DeviceClass::IntegratedGpu,
            DeviceClass::DiscreteGpu,
            DeviceClass::UnifiedMemory,
            DeviceClass::Mobile,
            DeviceClass::Embedded,
            DeviceClass::BrowserWasm,
            DeviceClass::Microcontroller,
            DeviceClass::NpuAccelerator,
            DeviceClass::MultiGpu,
            DeviceClass::Edge,
            DeviceClass::Server,
        ];

        &PROFILES
    }

    pub fn descriptor(self) -> DeviceProfileDescriptor {
        device_descriptor(self)
    }

    pub fn tier(self) -> DeviceTier {
        match self {
            Self::Auto => DeviceTier::Auto,
            Self::Microcontroller => DeviceTier::Tiny,
            Self::CpuOnly | Self::Mobile | Self::Embedded | Self::BrowserWasm | Self::Edge => {
                DeviceTier::Constrained
            }
            Self::IntegratedGpu | Self::UnifiedMemory | Self::NpuAccelerator => {
                DeviceTier::Balanced
            }
            Self::DiscreteGpu | Self::Server => DeviceTier::Accelerated,
            Self::MultiGpu => DeviceTier::Distributed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceProfileDescriptor {
    pub device: DeviceClass,
    pub tier: DeviceTier,
    pub scope: &'static str,
    pub aliases: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceProfileDescriptorSummary {
    pub device: DeviceClass,
    pub tier: DeviceTier,
    pub scope_len: usize,
    pub alias_count: usize,
    pub is_auto_profile: bool,
}

impl DeviceProfileDescriptor {
    pub fn aliases_csv(self) -> String {
        self.aliases.join("+")
    }

    pub fn descriptor_summary(self) -> DeviceProfileDescriptorSummary {
        DeviceProfileDescriptorSummary {
            device: self.device,
            tier: self.tier,
            scope_len: self.scope.len(),
            alias_count: self.aliases.len(),
            is_auto_profile: self.device == DeviceClass::Auto,
        }
    }
}

impl DeviceProfileDescriptorSummary {
    pub fn has_aliases(self) -> bool {
        self.alias_count > 0
    }

    pub fn has_scope(self) -> bool {
        self.scope_len > 0
    }

    pub fn tier_matches_device(self) -> bool {
        self.tier == self.device.tier()
    }
}

impl FromStr for DeviceClass {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "cpu" | "cpu-only" | "cpu_only" => Ok(Self::CpuOnly),
            "integrated" | "integrated-gpu" | "igpu" => Ok(Self::IntegratedGpu),
            "discrete" | "discrete-gpu" | "dgpu" | "gpu" => Ok(Self::DiscreteGpu),
            "uma" | "unified-memory" | "unified_memory" => Ok(Self::UnifiedMemory),
            "mobile" => Ok(Self::Mobile),
            "embedded" => Ok(Self::Embedded),
            "browser-wasm" | "wasm" | "web" => Ok(Self::BrowserWasm),
            "microcontroller" | "mcu" => Ok(Self::Microcontroller),
            "npu" | "npu-accelerator" => Ok(Self::NpuAccelerator),
            "multi-gpu" | "multi_gpu" | "distributed" => Ok(Self::MultiGpu),
            "edge" => Ok(Self::Edge),
            "server" => Ok(Self::Server),
            other => Err(format!("unknown device class: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    pub fn compute_headroom(self) -> f32 {
        match self {
            Self::Auto => 0.45,
            Self::Tiny => 0.08,
            Self::Constrained => 0.22,
            Self::Balanced => 0.50,
            Self::Accelerated => 0.78,
            Self::Distributed => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl FromStr for ComputeLane {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "cpu-portable" | "cpu_portable" | "portable" => Ok(Self::CpuPortable),
            "cpu-vector" | "cpu_vector" | "cpu" | "simd" => Ok(Self::CpuVector),
            "integrated-gpu" | "integrated_gpu" | "igpu" => Ok(Self::IntegratedGpu),
            "discrete-gpu" | "discrete_gpu" | "dgpu" | "gpu" => Ok(Self::DiscreteGpu),
            "unified-memory-gpu" | "unified_memory_gpu" | "uma-gpu" => Ok(Self::UnifiedMemoryGpu),
            "neural-accelerator" | "neural_accelerator" | "npu" => Ok(Self::NeuralAccelerator),
            "multi-accelerator" | "multi_accelerator" | "multi-gpu" | "multi_gpu" => {
                Ok(Self::MultiAccelerator)
            }
            "disk-backed-streaming" | "disk_backed_streaming" | "streaming" => {
                Ok(Self::DiskBackedStreaming)
            }
            other => Err(format!("unknown compute lane: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl FromStr for DeviceMemoryMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "minimal-disk" | "minimal_disk" | "minimal" => Ok(Self::MinimalDisk),
            "tiered-disk" | "tiered_disk" | "tiered" | "disk" => Ok(Self::TieredDisk),
            "unified-memory" | "unified_memory" | "uma" => Ok(Self::UnifiedMemory),
            "gpu-resident" | "gpu_resident" | "gpu" => Ok(Self::GpuResident),
            "distributed-sharded" | "distributed_sharded" | "distributed" | "sharded" => {
                Ok(Self::DistributedSharded)
            }
            other => Err(format!("unknown memory mode: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HardwareLoadSnapshot {
    pub device: DeviceClass,
    pub cpu_load: f32,
    pub gpu_load: f32,
    pub ram_load: f32,
    pub disk_load: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HardwareLoadSnapshotSummary {
    pub device: DeviceClass,
    pub tier: DeviceTier,
    pub cpu_load: f32,
    pub gpu_load: f32,
    pub ram_load: f32,
    pub disk_load: f32,
    pub pressure: f32,
    pub pressure_band: HardwarePressureBand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareLoadKind {
    Cpu,
    Gpu,
    Ram,
    Disk,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardwareLoadSnapshotCommitSummary {
    pub snapshot: HardwareLoadSnapshotSummary,
    pub action: HardwareLoadSnapshotCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub failure_reports: Vec<RuntimeFailureReport>,
    pub primary_failure_report: Option<RuntimeFailureReport>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareLoadSnapshotCommitAction {
    CommitHardwareLoadSnapshot,
    ReturnRuntimeFailure,
}

impl HardwareLoadSnapshotCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitHardwareLoadSnapshot)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl HardwareLoadSnapshotSummary {
    pub fn is_pressure_constrained(self) -> bool {
        self.pressure_band.is_constrained()
    }

    pub fn tier_matches_device(self) -> bool {
        self.tier == self.device.tier()
    }

    pub fn cpu_load_is_bounded(self) -> bool {
        bounded_unit_float(self.cpu_load)
    }

    pub fn gpu_load_is_bounded(self) -> bool {
        bounded_unit_float(self.gpu_load)
    }

    pub fn ram_load_is_bounded(self) -> bool {
        bounded_unit_float(self.ram_load)
    }

    pub fn disk_load_is_bounded(self) -> bool {
        bounded_unit_float(self.disk_load)
    }

    pub fn load_values_are_bounded(self) -> bool {
        self.cpu_load_is_bounded()
            && self.gpu_load_is_bounded()
            && self.ram_load_is_bounded()
            && self.disk_load_is_bounded()
    }

    pub fn pressure_is_bounded(self) -> bool {
        bounded_unit_float(self.pressure)
    }

    pub fn pressure_band_matches_pressure(self) -> bool {
        self.pressure_is_bounded()
            && self.pressure_band == HardwarePressureBand::from_pressure(self.pressure)
    }

    pub fn has_gpu_pressure(self) -> bool {
        self.gpu_load >= self.cpu_load && self.gpu_load >= self.ram_load
    }

    pub fn has_memory_pressure(self) -> bool {
        self.ram_load >= 0.72 || self.disk_load >= 0.72
    }

    pub fn dominant_load(self) -> HardwareLoadKind {
        let mut dominant = HardwareLoadKind::Cpu;
        let mut value = self.cpu_load;

        if self.gpu_load > value {
            dominant = HardwareLoadKind::Gpu;
            value = self.gpu_load;
        }
        if self.ram_load > value {
            dominant = HardwareLoadKind::Ram;
            value = self.ram_load;
        }
        if self.disk_load > value {
            dominant = HardwareLoadKind::Disk;
        }

        dominant
    }

    pub fn load_range(self) -> (f32, f32) {
        let min = self
            .cpu_load
            .min(self.gpu_load)
            .min(self.ram_load)
            .min(self.disk_load);
        let max = self
            .cpu_load
            .max(self.gpu_load)
            .max(self.ram_load)
            .max(self.disk_load);

        (min, max)
    }

    pub fn load_value_signal_component_count(self) -> usize {
        usize::from(self.cpu_load_is_bounded())
            + usize::from(self.gpu_load_is_bounded())
            + usize::from(self.ram_load_is_bounded())
            + usize::from(self.disk_load_is_bounded())
    }

    pub fn pressure_shape_signal_component_count(self) -> usize {
        usize::from(self.pressure_is_bounded())
            + usize::from(self.pressure_band_matches_pressure())
            + usize::from(self.tier_matches_device())
    }

    pub fn snapshot_signal_component_count(self) -> usize {
        self.load_value_signal_component_count()
            .saturating_add(self.pressure_shape_signal_component_count())
    }

    pub fn has_snapshot_signals(self) -> bool {
        self.snapshot_signal_component_count() > 0
    }

    pub fn load_value_problem_component_count(self) -> usize {
        usize::from(!self.cpu_load_is_bounded())
            + usize::from(!self.gpu_load_is_bounded())
            + usize::from(!self.ram_load_is_bounded())
            + usize::from(!self.disk_load_is_bounded())
    }

    pub fn pressure_shape_problem_component_count(self) -> usize {
        usize::from(!self.pressure_is_bounded())
            + usize::from(!self.pressure_band_matches_pressure())
    }

    pub fn tier_shape_problem_component_count(self) -> usize {
        usize::from(!self.tier_matches_device())
    }

    pub fn snapshot_shape_problem_component_count(self) -> usize {
        self.load_value_problem_component_count()
            .saturating_add(self.pressure_shape_problem_component_count())
            .saturating_add(self.tier_shape_problem_component_count())
    }

    pub fn has_snapshot_shape_problem_components(self) -> bool {
        self.snapshot_shape_problem_component_count() > 0
    }

    pub fn snapshot_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .load_value_signal_component_count()
            .saturating_add(self.pressure_shape_signal_component_count());
        let expected_problem_count = self
            .load_value_problem_component_count()
            .saturating_add(self.pressure_shape_problem_component_count())
            .saturating_add(self.tier_shape_problem_component_count());

        self.snapshot_signal_component_count() == expected_signal_count
            && self.has_snapshot_signals() == (expected_signal_count > 0)
            && self.snapshot_shape_problem_component_count() == expected_problem_count
            && self.has_snapshot_shape_problem_components() == (expected_problem_count > 0)
    }

    pub fn snapshot_shape_is_clean(self) -> bool {
        !self.has_snapshot_shape_problem_components() && self.snapshot_accounting_is_consistent()
    }

    pub fn hardware_snapshot_commit_signal_component_count(self) -> usize {
        self.snapshot_signal_component_count()
    }

    pub fn has_hardware_snapshot_commit_signals(self) -> bool {
        self.hardware_snapshot_commit_signal_component_count() > 0
    }

    pub fn hardware_snapshot_commit_blocker_component_count(self) -> usize {
        self.snapshot_shape_problem_component_count()
    }

    pub fn has_hardware_snapshot_commit_blockers(self) -> bool {
        self.hardware_snapshot_commit_blocker_component_count() > 0
    }

    pub fn hardware_snapshot_commit_accounting_is_consistent(self) -> bool {
        self.snapshot_accounting_is_consistent()
            && self.hardware_snapshot_commit_signal_component_count()
                == self.snapshot_signal_component_count()
            && self.has_hardware_snapshot_commit_signals()
                == (self.hardware_snapshot_commit_signal_component_count() > 0)
            && self.hardware_snapshot_commit_blocker_component_count()
                == self.snapshot_shape_problem_component_count()
            && self.has_hardware_snapshot_commit_blockers()
                == (self.hardware_snapshot_commit_blocker_component_count() > 0)
    }

    pub fn hardware_snapshot_commit_is_clean(self) -> bool {
        !self.has_hardware_snapshot_commit_blockers()
            && self.hardware_snapshot_commit_accounting_is_consistent()
    }

    pub fn can_commit_hardware_snapshot(self) -> bool {
        self.hardware_snapshot_commit_is_clean()
    }

    pub fn hardware_load_snapshot_commit_action(self) -> HardwareLoadSnapshotCommitAction {
        if self.can_commit_hardware_snapshot() {
            HardwareLoadSnapshotCommitAction::CommitHardwareLoadSnapshot
        } else {
            HardwareLoadSnapshotCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_hardware_snapshot(self) -> bool {
        self.snapshot_shape_is_clean()
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.hardware_snapshot_commit_accounting_is_consistent())
    }

    pub fn hardware_snapshot_problem_component_count(self) -> usize {
        self.hardware_snapshot_commit_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_hardware_snapshot_problem_components(self) -> bool {
        self.hardware_snapshot_problem_component_count() > 0
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.hardware_snapshot_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "hardware load snapshot failed: components={component_count}"
            )))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.failure_report().into_iter().collect()
    }

    pub fn failure_report_count(self) -> usize {
        usize::from(self.failure_report().is_some())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.failure_report()
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(self) -> HardwareLoadSnapshotCommitSummary {
        HardwareLoadSnapshotCommitSummary::new(self)
    }
}

impl HardwareLoadSnapshotCommitSummary {
    pub fn new(snapshot: HardwareLoadSnapshotSummary) -> Self {
        let failure_reports = snapshot.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = snapshot.can_commit_hardware_snapshot();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = snapshot.hardware_load_snapshot_commit_action();

        Self {
            snapshot,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: snapshot
                .hardware_snapshot_commit_signal_component_count(),
            total_blocker_component_count: snapshot
                .hardware_snapshot_commit_blocker_component_count(),
            component_accounting_consistent: snapshot
                .hardware_snapshot_commit_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(&self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(&self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(&self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_batch_shape_is_clean(&self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(&self) -> HardwareFailureReturnSummary {
        HardwareFailureReturnSummary::new(
            HardwareFailureReturnSource::LoadSnapshot,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<HardwareFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                HardwareFailureReturnReport::new(
                    HardwareFailureReturnSource::LoadSnapshot,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(&self) -> bool {
        self.can_commit == self.snapshot.can_commit_hardware_snapshot()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.snapshot.hardware_load_snapshot_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.snapshot.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self
                    .snapshot
                    .hardware_snapshot_commit_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .snapshot
                    .hardware_snapshot_commit_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .snapshot
                    .hardware_snapshot_commit_accounting_is_consistent()
    }

    pub fn can_commit_hardware_snapshot(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl HardwareLoadSnapshot {
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

    pub fn pressure(self) -> f32 {
        let weights = pressure_weights(self.device);
        (self.cpu_load * weights.cpu
            + self.gpu_load * weights.gpu
            + self.ram_load * weights.ram
            + self.disk_load * weights.disk)
            .clamp(0.0, 1.0)
    }

    pub fn snapshot_summary(self) -> HardwareLoadSnapshotSummary {
        let pressure = self.pressure();

        HardwareLoadSnapshotSummary {
            device: self.device,
            tier: self.device.tier(),
            cpu_load: self.cpu_load,
            gpu_load: self.gpu_load,
            ram_load: self.ram_load,
            disk_load: self.disk_load,
            pressure,
            pressure_band: HardwarePressureBand::from_pressure(pressure),
        }
    }
}

impl Default for HardwareLoadSnapshot {
    fn default() -> Self {
        Self::new(DeviceClass::Auto, 0.20, 0.20, 0.35, 0.15)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceExecutionPlan {
    pub primary_lane: ComputeLane,
    pub fallback_lane: ComputeLane,
    pub memory_mode: DeviceMemoryMode,
    pub adapter_hints: Vec<RuntimeAdapter>,
    pub max_parallel_chunks: usize,
    pub kv_prefetch_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub allow_disk_spill: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceExecutionPlanSummary {
    pub primary_lane: ComputeLane,
    pub fallback_lane: ComputeLane,
    pub memory_mode: DeviceMemoryMode,
    pub adapter_count: usize,
    pub max_parallel_chunks: usize,
    pub kv_prefetch_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub allow_disk_spill: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceExecutionAdapterSummary {
    pub adapter_count: usize,
    pub portable_count: usize,
    pub cpu_count: usize,
    pub gpu_count: usize,
    pub neural_count: usize,
    pub multi_device_count: usize,
    pub custom_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceExecutionPlanCommitSummary {
    pub execution: DeviceExecutionPlanSummary,
    pub action: DeviceExecutionPlanCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub failure_reports: Vec<RuntimeFailureReport>,
    pub primary_failure_report: Option<RuntimeFailureReport>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceExecutionPlanCommitAction {
    CommitDeviceExecutionPlan,
    ReturnRuntimeFailure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceExecutionAdapterCommitSummary {
    pub adapter: DeviceExecutionAdapterSummary,
    pub action: DeviceExecutionAdapterCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub failure_reports: Vec<RuntimeFailureReport>,
    pub primary_failure_report: Option<RuntimeFailureReport>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceExecutionAdapterCommitAction {
    CommitDeviceExecutionAdapters,
    ReturnRuntimeFailure,
}

impl DeviceExecutionPlanCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitDeviceExecutionPlan)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl DeviceExecutionAdapterCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitDeviceExecutionAdapters)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl DeviceExecutionPlanSummary {
    pub fn has_adapter_hints(self) -> bool {
        self.adapter_count > 0
    }

    pub fn missing_adapter_hints(self) -> bool {
        !self.has_adapter_hints()
    }

    pub fn has_parallel_capacity(self) -> bool {
        self.max_parallel_chunks > 1
    }

    pub fn lacks_parallel_capacity(self) -> bool {
        !self.has_parallel_capacity()
    }

    pub fn has_kv_prefetch_capacity(self) -> bool {
        self.kv_prefetch_blocks > 0
    }

    pub fn lacks_kv_prefetch_capacity(self) -> bool {
        !self.has_kv_prefetch_capacity()
    }

    pub fn uses_same_fallback_lane(self) -> bool {
        self.primary_lane == self.fallback_lane
    }

    pub fn has_distinct_fallback_lane(self) -> bool {
        !self.uses_same_fallback_lane()
    }

    pub fn uses_gpu_or_accelerator(self) -> bool {
        matches!(
            self.primary_lane,
            ComputeLane::IntegratedGpu
                | ComputeLane::DiscreteGpu
                | ComputeLane::UnifiedMemoryGpu
                | ComputeLane::NeuralAccelerator
                | ComputeLane::MultiAccelerator
        )
    }

    pub fn uses_cpu_primary_lane(self) -> bool {
        matches!(
            self.primary_lane,
            ComputeLane::CpuPortable | ComputeLane::CpuVector
        )
    }

    pub fn uses_disk_streaming_lane(self) -> bool {
        matches!(self.primary_lane, ComputeLane::DiskBackedStreaming)
    }

    pub fn uses_disk_backed_memory(self) -> bool {
        matches!(
            self.memory_mode,
            DeviceMemoryMode::MinimalDisk | DeviceMemoryMode::TieredDisk
        )
    }

    pub fn uses_compressed_hot_kv(self) -> bool {
        self.hot_kv_precision_bits <= 4
    }

    pub fn kv_precision_is_compressed(self) -> bool {
        self.hot_kv_precision_bits < 8 || self.cold_kv_precision_bits < 8
    }

    pub fn has_valid_hot_kv_precision(self) -> bool {
        valid_kv_precision_bits(self.hot_kv_precision_bits)
    }

    pub fn has_valid_cold_kv_precision(self) -> bool {
        valid_kv_precision_bits(self.cold_kv_precision_bits)
    }

    pub fn cold_kv_not_wider_than_hot(self) -> bool {
        self.cold_kv_precision_bits <= self.hot_kv_precision_bits
    }

    pub fn has_precision_inversion(self) -> bool {
        !self.cold_kv_not_wider_than_hot()
    }

    pub fn hot_and_cold_precision_match(self) -> bool {
        self.hot_kv_precision_bits == self.cold_kv_precision_bits
    }

    pub fn adapter_hint_signal_component_count(self) -> usize {
        usize::from(self.has_adapter_hints())
    }

    pub fn execution_capacity_signal_component_count(self) -> usize {
        usize::from(self.has_parallel_capacity()) + usize::from(self.has_kv_prefetch_capacity())
    }

    pub fn primary_lane_signal_component_count(self) -> usize {
        usize::from(self.uses_cpu_primary_lane())
            + usize::from(self.uses_gpu_or_accelerator())
            + usize::from(self.uses_disk_streaming_lane())
    }

    pub fn fallback_lane_signal_component_count(self) -> usize {
        usize::from(self.has_distinct_fallback_lane()) + usize::from(self.uses_same_fallback_lane())
    }

    pub fn memory_mode_signal_component_count(self) -> usize {
        usize::from(self.uses_disk_backed_memory()) + usize::from(!self.uses_disk_backed_memory())
    }

    pub fn kv_precision_signal_component_count(self) -> usize {
        usize::from(self.has_valid_hot_kv_precision())
            + usize::from(self.has_valid_cold_kv_precision())
            + usize::from(
                self.has_valid_hot_kv_precision()
                    && self.has_valid_cold_kv_precision()
                    && self.kv_precision_is_compressed(),
            )
    }

    pub fn execution_constraint_signal_component_count(self) -> usize {
        usize::from(self.uses_same_fallback_lane())
            + usize::from(self.uses_disk_streaming_lane())
            + usize::from(self.uses_disk_backed_memory())
            + usize::from(self.uses_compressed_hot_kv())
    }

    pub fn execution_shape_signal_component_count(self) -> usize {
        self.adapter_hint_signal_component_count()
            .saturating_add(self.execution_capacity_signal_component_count())
            .saturating_add(self.primary_lane_signal_component_count())
            .saturating_add(self.fallback_lane_signal_component_count())
            .saturating_add(self.memory_mode_signal_component_count())
            .saturating_add(self.kv_precision_signal_component_count())
            .saturating_add(self.execution_constraint_signal_component_count())
    }

    pub fn has_execution_shape_signals(self) -> bool {
        self.execution_shape_signal_component_count() > 0
    }

    pub fn adapter_hint_problem_component_count(self) -> usize {
        usize::from(self.missing_adapter_hints())
    }

    pub fn execution_capacity_problem_component_count(self) -> usize {
        usize::from(self.lacks_parallel_capacity()) + usize::from(self.lacks_kv_prefetch_capacity())
    }

    pub fn precision_problem_component_count(self) -> usize {
        usize::from(!self.has_valid_hot_kv_precision())
            + usize::from(!self.has_valid_cold_kv_precision())
            + usize::from(self.has_precision_inversion())
    }

    pub fn execution_shape_problem_component_count(self) -> usize {
        self.adapter_hint_problem_component_count()
            .saturating_add(self.execution_capacity_problem_component_count())
            .saturating_add(self.precision_problem_component_count())
    }

    pub fn has_execution_shape_problem_components(self) -> bool {
        self.execution_shape_problem_component_count() > 0
    }

    pub fn hardware_execution_signal_component_count(self) -> usize {
        self.execution_shape_signal_component_count()
    }

    pub fn has_hardware_execution_signals(self) -> bool {
        self.hardware_execution_signal_component_count() > 0
    }

    pub fn hardware_execution_blocker_component_count(self) -> usize {
        self.execution_shape_problem_component_count()
    }

    pub fn has_hardware_execution_blockers(self) -> bool {
        self.hardware_execution_blocker_component_count() > 0
    }

    pub fn execution_shape_risk_component_count(self) -> usize {
        self.execution_shape_problem_component_count()
            .saturating_add(self.execution_constraint_signal_component_count())
    }

    pub fn has_execution_shape_risk(self) -> bool {
        self.execution_shape_risk_component_count() > 0
    }

    pub fn execution_shape_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .adapter_hint_signal_component_count()
            .saturating_add(self.execution_capacity_signal_component_count())
            .saturating_add(self.primary_lane_signal_component_count())
            .saturating_add(self.fallback_lane_signal_component_count())
            .saturating_add(self.memory_mode_signal_component_count())
            .saturating_add(self.kv_precision_signal_component_count())
            .saturating_add(self.execution_constraint_signal_component_count());
        let expected_problem_count = self
            .adapter_hint_problem_component_count()
            .saturating_add(self.execution_capacity_problem_component_count())
            .saturating_add(self.precision_problem_component_count());
        let expected_risk_count = expected_problem_count
            .saturating_add(self.execution_constraint_signal_component_count());

        self.execution_shape_signal_component_count() == expected_signal_count
            && self.has_execution_shape_signals() == (expected_signal_count > 0)
            && self.execution_shape_problem_component_count() == expected_problem_count
            && self.has_execution_shape_problem_components() == (expected_problem_count > 0)
            && self.execution_shape_risk_component_count() == expected_risk_count
            && self.has_execution_shape_risk() == (expected_risk_count > 0)
    }

    pub fn execution_shape_is_clean(self) -> bool {
        !self.has_execution_shape_problem_components()
            && self.execution_shape_accounting_is_consistent()
    }

    pub fn hardware_execution_accounting_is_consistent(self) -> bool {
        self.execution_shape_accounting_is_consistent()
            && self.hardware_execution_signal_component_count()
                == self.execution_shape_signal_component_count()
            && self.has_hardware_execution_signals()
                == (self.hardware_execution_signal_component_count() > 0)
            && self.hardware_execution_blocker_component_count()
                == self.execution_shape_problem_component_count()
            && self.has_hardware_execution_blockers()
                == (self.hardware_execution_blocker_component_count() > 0)
    }

    pub fn hardware_execution_commit_is_clean(self) -> bool {
        !self.has_hardware_execution_blockers()
            && self.hardware_execution_accounting_is_consistent()
    }

    pub fn can_commit_device_execution_plan(self) -> bool {
        self.hardware_execution_commit_is_clean()
    }

    pub fn device_execution_plan_commit_action(self) -> DeviceExecutionPlanCommitAction {
        if self.can_commit_device_execution_plan() {
            DeviceExecutionPlanCommitAction::CommitDeviceExecutionPlan
        } else {
            DeviceExecutionPlanCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_device_execution_plan(self) -> bool {
        self.execution_shape_is_clean()
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.hardware_execution_accounting_is_consistent())
    }

    pub fn hardware_execution_problem_component_count(self) -> usize {
        self.hardware_execution_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_hardware_execution_problem_components(self) -> bool {
        self.hardware_execution_problem_component_count() > 0
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.hardware_execution_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "device execution plan failed: components={component_count}"
            )))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.failure_report().into_iter().collect()
    }

    pub fn failure_report_count(self) -> usize {
        usize::from(self.failure_report().is_some())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.failure_report()
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(self) -> DeviceExecutionPlanCommitSummary {
        DeviceExecutionPlanCommitSummary::new(self)
    }
}

impl DeviceExecutionPlanCommitSummary {
    pub fn new(execution: DeviceExecutionPlanSummary) -> Self {
        let failure_reports = execution.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = execution.can_commit_device_execution_plan();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = execution.device_execution_plan_commit_action();

        Self {
            execution,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: execution.hardware_execution_signal_component_count(),
            total_blocker_component_count: execution.hardware_execution_blocker_component_count(),
            component_accounting_consistent: execution
                .hardware_execution_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(&self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(&self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(&self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_batch_shape_is_clean(&self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(&self) -> HardwareFailureReturnSummary {
        HardwareFailureReturnSummary::new(
            HardwareFailureReturnSource::DeviceExecutionPlan,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<HardwareFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                HardwareFailureReturnReport::new(
                    HardwareFailureReturnSource::DeviceExecutionPlan,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(&self) -> bool {
        self.can_commit == self.execution.can_commit_device_execution_plan()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.execution.device_execution_plan_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.execution.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self.execution.hardware_execution_signal_component_count()
            && self.total_blocker_component_count
                == self.execution.hardware_execution_blocker_component_count()
            && self.component_accounting_consistent
                == self.execution.hardware_execution_accounting_is_consistent()
    }

    pub fn can_commit_device_execution_plan(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl DeviceExecutionAdapterSummary {
    pub fn from_adapters(adapters: &[RuntimeAdapter]) -> Self {
        let mut summary = Self {
            adapter_count: adapters.len(),
            ..Self::default()
        };

        for adapter in adapters {
            match adapter {
                RuntimeAdapter::PortableRust => summary.portable_count += 1,
                RuntimeAdapter::CpuSimd | RuntimeAdapter::OpenVino => summary.cpu_count += 1,
                RuntimeAdapter::Wgpu
                | RuntimeAdapter::WebGpu
                | RuntimeAdapter::Vulkan
                | RuntimeAdapter::Metal
                | RuntimeAdapter::Cuda
                | RuntimeAdapter::Rocm
                | RuntimeAdapter::OneApi
                | RuntimeAdapter::DirectMl => summary.gpu_count += 1,
                RuntimeAdapter::CoreMl
                | RuntimeAdapter::Nnapi
                | RuntimeAdapter::Qnn
                | RuntimeAdapter::Cann
                | RuntimeAdapter::Mlu
                | RuntimeAdapter::Rknn => summary.neural_count += 1,
                RuntimeAdapter::MultiDevice => summary.multi_device_count += 1,
                RuntimeAdapter::CustomAccelerator => summary.custom_count += 1,
            }
        }

        summary
    }

    pub fn has_portable_fallback(self) -> bool {
        self.portable_count > 0 || self.cpu_count > 0
    }

    pub fn has_accelerator(self) -> bool {
        self.gpu_count > 0
            || self.neural_count > 0
            || self.multi_device_count > 0
            || self.custom_count > 0
    }

    pub fn is_empty(self) -> bool {
        self.adapter_count == 0
    }

    pub fn fallback_adapter_count(self) -> usize {
        self.portable_count + self.cpu_count
    }

    pub fn accelerator_adapter_count(self) -> usize {
        self.gpu_count + self.neural_count + self.multi_device_count + self.custom_count
    }

    pub fn family_member_count(self) -> usize {
        self.fallback_adapter_count()
            .saturating_add(self.accelerator_adapter_count())
    }

    pub fn adapter_count_matches_families(self) -> bool {
        self.adapter_count == self.family_member_count()
    }

    pub fn adapter_family_count(self) -> usize {
        usize::from(self.portable_count > 0)
            + usize::from(self.cpu_count > 0)
            + usize::from(self.gpu_count > 0)
            + usize::from(self.neural_count > 0)
            + usize::from(self.multi_device_count > 0)
            + usize::from(self.custom_count > 0)
    }

    pub fn has_mixed_fallback_and_accelerator(self) -> bool {
        self.has_portable_fallback() && self.has_accelerator()
    }

    pub fn is_accelerator_only(self) -> bool {
        self.has_accelerator() && !self.has_portable_fallback()
    }

    pub fn is_fallback_only(self) -> bool {
        self.has_portable_fallback() && !self.has_accelerator()
    }

    pub fn adapter_family_signal_component_count(self) -> usize {
        usize::from(!self.is_empty())
            + usize::from(self.has_portable_fallback())
            + usize::from(self.has_accelerator())
            + usize::from(self.has_mixed_fallback_and_accelerator())
            + usize::from(self.is_accelerator_only())
            + usize::from(self.is_fallback_only())
    }

    pub fn has_adapter_family_signals(self) -> bool {
        self.adapter_family_signal_component_count() > 0
    }

    pub fn adapter_family_problem_component_count(self) -> usize {
        usize::from(!self.adapter_count_matches_families())
    }

    pub fn has_adapter_family_problem_components(self) -> bool {
        self.adapter_family_problem_component_count() > 0
    }

    pub fn adapter_family_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(!self.is_empty())
            .saturating_add(usize::from(self.has_portable_fallback()))
            .saturating_add(usize::from(self.has_accelerator()))
            .saturating_add(usize::from(self.has_mixed_fallback_and_accelerator()))
            .saturating_add(usize::from(self.is_accelerator_only()))
            .saturating_add(usize::from(self.is_fallback_only()));
        let expected_problem_count = usize::from(!self.adapter_count_matches_families());

        self.family_member_count()
            == self
                .fallback_adapter_count()
                .saturating_add(self.accelerator_adapter_count())
            && self.adapter_family_signal_component_count() == expected_signal_count
            && self.has_adapter_family_signals() == (expected_signal_count > 0)
            && self.adapter_family_problem_component_count() == expected_problem_count
            && self.has_adapter_family_problem_components() == (expected_problem_count > 0)
    }

    pub fn adapter_family_shape_is_clean(self) -> bool {
        !self.has_adapter_family_problem_components()
            && self.adapter_family_accounting_is_consistent()
    }

    pub fn can_use_adapter_family(self) -> bool {
        !self.is_empty() && self.adapter_family_shape_is_clean()
    }

    pub fn adapter_family_commit_signal_component_count(self) -> usize {
        self.adapter_family_signal_component_count()
    }

    pub fn has_adapter_family_commit_signals(self) -> bool {
        self.adapter_family_commit_signal_component_count() > 0
    }

    pub fn adapter_family_commit_blocker_component_count(self) -> usize {
        usize::from(self.is_empty()).saturating_add(self.adapter_family_problem_component_count())
    }

    pub fn has_adapter_family_commit_blockers(self) -> bool {
        self.adapter_family_commit_blocker_component_count() > 0
    }

    pub fn adapter_family_commit_accounting_is_consistent(self) -> bool {
        self.adapter_family_accounting_is_consistent()
            && self.adapter_family_commit_signal_component_count()
                == self.adapter_family_signal_component_count()
            && self.has_adapter_family_commit_signals()
                == (self.adapter_family_commit_signal_component_count() > 0)
            && self.adapter_family_commit_blocker_component_count()
                == usize::from(self.is_empty())
                    .saturating_add(self.adapter_family_problem_component_count())
            && self.has_adapter_family_commit_blockers()
                == (self.adapter_family_commit_blocker_component_count() > 0)
    }

    pub fn adapter_family_commit_is_clean(self) -> bool {
        !self.has_adapter_family_commit_blockers()
            && self.adapter_family_commit_accounting_is_consistent()
    }

    pub fn can_commit_device_execution_adapters(self) -> bool {
        self.adapter_family_commit_is_clean()
    }

    pub fn device_execution_adapter_commit_action(self) -> DeviceExecutionAdapterCommitAction {
        if self.can_commit_device_execution_adapters() {
            DeviceExecutionAdapterCommitAction::CommitDeviceExecutionAdapters
        } else {
            DeviceExecutionAdapterCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.adapter_family_commit_accounting_is_consistent())
    }

    pub fn adapter_family_commit_problem_component_count(self) -> usize {
        self.adapter_family_commit_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_adapter_family_commit_problem_components(self) -> bool {
        self.adapter_family_commit_problem_component_count() > 0
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.adapter_family_commit_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "device execution adapter family failed: components={component_count}"
            )))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.failure_report().into_iter().collect()
    }

    pub fn failure_report_count(self) -> usize {
        usize::from(self.failure_report().is_some())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.failure_report()
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(self) -> DeviceExecutionAdapterCommitSummary {
        DeviceExecutionAdapterCommitSummary::new(self)
    }
}

impl DeviceExecutionAdapterCommitSummary {
    pub fn new(adapter: DeviceExecutionAdapterSummary) -> Self {
        let failure_reports = adapter.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = adapter.can_commit_device_execution_adapters();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = adapter.device_execution_adapter_commit_action();

        Self {
            adapter,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: adapter.adapter_family_commit_signal_component_count(),
            total_blocker_component_count: adapter.adapter_family_commit_blocker_component_count(),
            component_accounting_consistent: adapter
                .adapter_family_commit_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(&self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(&self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(&self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_batch_shape_is_clean(&self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(&self) -> HardwareFailureReturnSummary {
        HardwareFailureReturnSummary::new(
            HardwareFailureReturnSource::DeviceExecutionAdapters,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<HardwareFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                HardwareFailureReturnReport::new(
                    HardwareFailureReturnSource::DeviceExecutionAdapters,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(&self) -> bool {
        self.can_commit == self.adapter.can_commit_device_execution_adapters()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.adapter.device_execution_adapter_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.adapter.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self.adapter.adapter_family_commit_signal_component_count()
            && self.total_blocker_component_count
                == self.adapter.adapter_family_commit_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .adapter
                    .adapter_family_commit_accounting_is_consistent()
    }

    pub fn can_commit_device_execution_adapters(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl DeviceExecutionPlan {
    pub fn execution_summary(&self) -> DeviceExecutionPlanSummary {
        DeviceExecutionPlanSummary {
            primary_lane: self.primary_lane,
            fallback_lane: self.fallback_lane,
            memory_mode: self.memory_mode,
            adapter_count: self.adapter_hints.len(),
            max_parallel_chunks: self.max_parallel_chunks,
            kv_prefetch_blocks: self.kv_prefetch_blocks,
            hot_kv_precision_bits: self.hot_kv_precision_bits,
            cold_kv_precision_bits: self.cold_kv_precision_bits,
            allow_disk_spill: self.allow_disk_spill,
        }
    }

    pub fn adapter_hint_summary(&self) -> DeviceExecutionAdapterSummary {
        DeviceExecutionAdapterSummary::from_adapters(&self.adapter_hints)
    }

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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HardwarePressureBand {
    Low,
    Medium,
    High,
    Critical,
}

impl HardwarePressureBand {
    pub fn from_pressure(pressure: f32) -> Self {
        let pressure = pressure.clamp(0.0, 1.0);
        if pressure >= 0.88 {
            Self::Critical
        } else if pressure >= 0.72 {
            Self::High
        } else if pressure >= 0.45 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn is_constrained(self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HardwarePlanSummary {
    pub device: DeviceClass,
    pub tier: DeviceTier,
    pub pressure: f32,
    pub pressure_band: HardwarePressureBand,
    pub compute_headroom: f32,
    pub latency_budget_ms: Option<u64>,
    pub local_kv_token_budget: usize,
    pub global_kv_token_budget: usize,
    pub max_parallel_chunks: usize,
    pub tier_parallel_chunks: usize,
    pub kv_prefetch_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
    pub adapter_count: usize,
    pub allow_disk_spill: bool,
    pub note_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardwarePlanCommitSummary {
    pub plan: HardwarePlanSummary,
    pub action: HardwarePlanCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub failure_reports: Vec<RuntimeFailureReport>,
    pub primary_failure_report: Option<RuntimeFailureReport>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwarePlanCommitAction {
    CommitHardwarePlan,
    ReturnRuntimeFailure,
}

impl HardwarePlanCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitHardwarePlan)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HardwareAdapterBridgeSummary {
    pub plan: HardwarePlanSummary,
    pub execution: DeviceExecutionPlanSummary,
    pub context: AdapterExecutionContextSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardwareAdapterBridgeCommitSummary {
    pub bridge: HardwareAdapterBridgeSummary,
    pub action: HardwareAdapterBridgeCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub failure_reports: Vec<RuntimeFailureReport>,
    pub primary_failure_report: Option<RuntimeFailureReport>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareAdapterBridgeCommitAction {
    CommitHardwareAdapterBridge,
    ReturnRuntimeFailure,
}

impl HardwareAdapterBridgeCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitHardwareAdapterBridge)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareRuntimeReadinessStage {
    LoadSnapshot,
    HardwarePlan,
    DeviceExecution,
    AdapterBridge,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HardwareRuntimeReadinessSummary {
    pub snapshot: HardwareLoadSnapshotSummary,
    pub plan: HardwarePlanSummary,
    pub execution: DeviceExecutionPlanSummary,
    pub bridge: HardwareAdapterBridgeSummary,
    pub snapshot_signal_component_count: usize,
    pub hardware_plan_signal_component_count: usize,
    pub device_execution_signal_component_count: usize,
    pub adapter_bridge_signal_component_count: usize,
    pub snapshot_blocker_component_count: usize,
    pub hardware_plan_blocker_component_count: usize,
    pub device_execution_blocker_component_count: usize,
    pub adapter_bridge_blocker_component_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardwareRuntimeCommitSummary {
    pub readiness: HardwareRuntimeReadinessSummary,
    pub action: HardwareRuntimeCommitAction,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub first_unready_stage: Option<HardwareRuntimeReadinessStage>,
    pub first_blocking_stage: Option<HardwareRuntimeReadinessStage>,
    pub failure_reports: Vec<RuntimeFailureReport>,
    pub primary_failure_report: Option<RuntimeFailureReport>,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_signal_component_count: usize,
    pub total_blocker_component_count: usize,
    pub component_accounting_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareRuntimeCommitAction {
    CommitHardwareRuntime,
    ReturnRuntimeFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareFailureReturnSource {
    LoadSnapshot,
    DeviceExecutionPlan,
    DeviceExecutionAdapters,
    HardwarePlan,
    AdapterBridge,
    HardwareRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HardwareFailureReturnSummary {
    pub source: HardwareFailureReturnSource,
    pub can_commit: bool,
    pub should_return_failure: bool,
    pub has_primary_failure_summary: bool,
    pub primary_failure_summary: Option<RuntimeFailureSummary>,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
    pub commit_decision_accounting_consistent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardwareFailureReturnReport {
    pub source: HardwareFailureReturnSource,
    pub primary_failure: RuntimeFailureReport,
    pub primary_failure_summary: RuntimeFailureSummary,
    pub failure_batch: RuntimeFailureBatchSummary,
    pub failure_report_count: usize,
    pub can_format_runtime_failures: bool,
    pub total_blocker_component_count: usize,
}

impl HardwareRuntimeReadinessStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::LoadSnapshot => "load_snapshot",
            Self::HardwarePlan => "hardware_plan",
            Self::DeviceExecution => "device_execution",
            Self::AdapterBridge => "adapter_bridge",
        }
    }
}

impl HardwareRuntimeCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitHardwareRuntime)
    }

    pub fn should_return_failure(self) -> bool {
        matches!(self, Self::ReturnRuntimeFailure)
    }
}

impl HardwareFailureReturnSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::LoadSnapshot => "load_snapshot",
            Self::DeviceExecutionPlan => "device_execution_plan",
            Self::DeviceExecutionAdapters => "device_execution_adapters",
            Self::HardwarePlan => "hardware_plan",
            Self::AdapterBridge => "adapter_bridge",
            Self::HardwareRuntime => "hardware_runtime",
        }
    }
}

impl HardwareFailureReturnSummary {
    pub fn new(
        source: HardwareFailureReturnSource,
        can_commit: bool,
        should_return_failure: bool,
        primary_failure_summary: Option<RuntimeFailureSummary>,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_blocker_component_count: usize,
        commit_decision_accounting_consistent: bool,
    ) -> Self {
        Self {
            source,
            can_commit,
            should_return_failure,
            has_primary_failure_summary: primary_failure_summary.is_some(),
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_blocker_component_count,
            commit_decision_accounting_consistent,
        }
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count > 0
    }

    pub fn has_blocker_components(self) -> bool {
        self.total_blocker_component_count > 0
    }

    pub fn failure_return_accounting_is_consistent(self) -> bool {
        self.commit_decision_accounting_consistent
            && self.should_return_failure == (!self.can_commit && self.has_failure_reports())
            && self.has_primary_failure_summary == self.primary_failure_summary.is_some()
            && self.has_primary_failure_summary == self.has_failure_reports()
            && self.failure_batch.total_count == self.failure_report_count
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && (!self.has_failure_reports() || self.has_blocker_components())
    }

    pub fn can_return_runtime_failure(self) -> bool {
        self.should_return_failure
            && self.has_primary_failure_summary
            && self.can_format_runtime_failures
            && self.failure_return_accounting_is_consistent()
    }
}

impl HardwareFailureReturnReport {
    pub fn new(
        source: HardwareFailureReturnSource,
        primary_failure: RuntimeFailureReport,
        failure_batch: RuntimeFailureBatchSummary,
        failure_report_count: usize,
        can_format_runtime_failures: bool,
        total_blocker_component_count: usize,
    ) -> Self {
        let primary_failure_summary = primary_failure.failure_summary();
        Self {
            source,
            primary_failure,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_blocker_component_count,
        }
    }

    pub fn backend_message(&self) -> String {
        self.primary_failure.backend_message()
    }

    pub fn diagnostics_note(&self) -> String {
        self.primary_failure.diagnostics_note()
    }

    pub fn inference_error(&self) -> InferenceError {
        InferenceError::from_failure(self.primary_failure.clone())
    }

    pub fn failure_return_report_shape_is_clean(&self) -> bool {
        self.primary_failure_summary == self.primary_failure.failure_summary()
            && self.failure_report_count > 0
            && self.failure_batch.total_count == self.failure_report_count
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.can_format_runtime_failures
            && self.total_blocker_component_count > 0
    }

    pub fn can_use_hardware_failure_return_report(&self) -> bool {
        self.failure_return_report_shape_is_clean()
    }
}

impl HardwarePlanSummary {
    pub fn tier_matches_device(self) -> bool {
        self.tier == self.device.tier()
    }

    pub fn pressure_is_bounded(self) -> bool {
        bounded_unit_float(self.pressure)
    }

    pub fn pressure_band_matches_pressure(self) -> bool {
        self.pressure_is_bounded()
            && self.pressure_band == HardwarePressureBand::from_pressure(self.pressure)
    }

    pub fn compute_headroom_is_bounded(self) -> bool {
        bounded_unit_float(self.compute_headroom)
    }

    pub fn has_adapter_hints(self) -> bool {
        self.adapter_count > 0
    }

    pub fn has_parallel_capacity(self) -> bool {
        self.max_parallel_chunks > 0
    }

    pub fn has_tier_parallel_capacity(self) -> bool {
        self.tier_parallel_chunks > 0
    }

    pub fn has_valid_hot_kv_precision(self) -> bool {
        valid_kv_precision_bits(self.hot_kv_precision_bits)
    }

    pub fn has_valid_cold_kv_precision(self) -> bool {
        valid_kv_precision_bits(self.cold_kv_precision_bits)
    }

    pub fn cold_kv_not_wider_than_hot(self) -> bool {
        self.cold_kv_precision_bits <= self.hot_kv_precision_bits
    }

    pub fn pressure_is_constrained(self) -> bool {
        self.pressure_band.is_constrained()
    }

    pub fn parallelism_was_reduced(self) -> bool {
        self.max_parallel_chunks < self.tier_parallel_chunks
    }

    pub fn kv_prefetch_is_minimal(self) -> bool {
        self.kv_prefetch_blocks <= 1
    }

    pub fn uses_compressed_hot_kv(self) -> bool {
        self.hot_kv_precision_bits <= 4
    }

    pub fn has_latency_budget(self) -> bool {
        self.latency_budget_ms.is_some()
    }

    pub fn can_spill_to_disk(self) -> bool {
        self.allow_disk_spill
    }

    pub fn has_notes(self) -> bool {
        self.note_count > 0
    }

    pub fn cannot_spill_to_disk(self) -> bool {
        !self.can_spill_to_disk()
    }

    pub fn pressure_constraint_signal_component_count(self) -> usize {
        usize::from(self.pressure_is_constrained())
    }

    pub fn parallelism_constraint_signal_component_count(self) -> usize {
        usize::from(self.parallelism_was_reduced())
    }

    pub fn kv_prefetch_constraint_signal_component_count(self) -> usize {
        usize::from(self.kv_prefetch_is_minimal())
    }

    pub fn precision_constraint_signal_component_count(self) -> usize {
        usize::from(self.uses_compressed_hot_kv())
    }

    pub fn latency_constraint_signal_component_count(self) -> usize {
        usize::from(self.has_latency_budget())
    }

    pub fn disk_spill_constraint_signal_component_count(self) -> usize {
        usize::from(self.cannot_spill_to_disk())
    }

    pub fn note_signal_component_count(self) -> usize {
        usize::from(self.has_notes())
    }

    pub fn plan_constraint_signal_component_count(self) -> usize {
        usize::from(self.pressure_is_constrained())
            .saturating_add(self.parallelism_constraint_signal_component_count())
            .saturating_add(self.kv_prefetch_constraint_signal_component_count())
            .saturating_add(self.precision_constraint_signal_component_count())
            .saturating_add(self.latency_constraint_signal_component_count())
            .saturating_add(self.disk_spill_constraint_signal_component_count())
            .saturating_add(self.note_signal_component_count())
    }

    pub fn plan_constraint_component_count(self) -> usize {
        self.plan_constraint_signal_component_count()
    }

    pub fn has_plan_constraint_signals(self) -> bool {
        self.plan_constraint_signal_component_count() > 0
    }

    pub fn has_plan_constraints(self) -> bool {
        self.has_plan_constraint_signals()
    }

    pub fn plan_tier_problem_component_count(self) -> usize {
        usize::from(!self.tier_matches_device())
    }

    pub fn plan_pressure_problem_component_count(self) -> usize {
        usize::from(!self.pressure_is_bounded())
            + usize::from(!self.pressure_band_matches_pressure())
            + usize::from(!self.compute_headroom_is_bounded())
    }

    pub fn plan_capacity_problem_component_count(self) -> usize {
        usize::from(!self.has_adapter_hints())
            + usize::from(!self.has_parallel_capacity())
            + usize::from(!self.has_tier_parallel_capacity())
    }

    pub fn plan_precision_problem_component_count(self) -> usize {
        usize::from(!self.has_valid_hot_kv_precision())
            + usize::from(!self.has_valid_cold_kv_precision())
            + usize::from(!self.cold_kv_not_wider_than_hot())
    }

    pub fn plan_shape_problem_component_count(self) -> usize {
        self.plan_tier_problem_component_count()
            .saturating_add(self.plan_pressure_problem_component_count())
            .saturating_add(self.plan_capacity_problem_component_count())
            .saturating_add(self.plan_precision_problem_component_count())
    }

    pub fn has_plan_shape_problem_components(self) -> bool {
        self.plan_shape_problem_component_count() > 0
    }

    pub fn plan_constraint_signal_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .pressure_constraint_signal_component_count()
            .saturating_add(self.parallelism_constraint_signal_component_count())
            .saturating_add(self.kv_prefetch_constraint_signal_component_count())
            .saturating_add(self.precision_constraint_signal_component_count())
            .saturating_add(self.latency_constraint_signal_component_count())
            .saturating_add(self.disk_spill_constraint_signal_component_count())
            .saturating_add(self.note_signal_component_count());

        self.plan_constraint_signal_component_count() == expected_signal_count
            && self.plan_constraint_component_count() == expected_signal_count
            && self.has_plan_constraint_signals() == (expected_signal_count > 0)
            && self.has_plan_constraints() == self.has_plan_constraint_signals()
    }

    pub fn plan_shape_accounting_is_consistent(self) -> bool {
        let expected_problem_count = self
            .plan_tier_problem_component_count()
            .saturating_add(self.plan_pressure_problem_component_count())
            .saturating_add(self.plan_capacity_problem_component_count())
            .saturating_add(self.plan_precision_problem_component_count());

        self.plan_constraint_signal_accounting_is_consistent()
            && self.plan_shape_problem_component_count() == expected_problem_count
            && self.has_plan_shape_problem_components() == (expected_problem_count > 0)
    }

    pub fn hardware_plan_shape_is_clean(self) -> bool {
        !self.has_plan_shape_problem_components() && self.plan_shape_accounting_is_consistent()
    }

    pub fn hardware_plan_commit_signal_component_count(self) -> usize {
        self.plan_constraint_signal_component_count()
    }

    pub fn has_hardware_plan_commit_signals(self) -> bool {
        self.hardware_plan_commit_signal_component_count() > 0
    }

    pub fn hardware_plan_commit_blocker_component_count(self) -> usize {
        self.plan_shape_problem_component_count()
    }

    pub fn has_hardware_plan_commit_blockers(self) -> bool {
        self.hardware_plan_commit_blocker_component_count() > 0
    }

    pub fn hardware_plan_commit_accounting_is_consistent(self) -> bool {
        self.plan_shape_accounting_is_consistent()
            && self.hardware_plan_commit_signal_component_count()
                == self.plan_constraint_signal_component_count()
            && self.has_hardware_plan_commit_signals()
                == (self.hardware_plan_commit_signal_component_count() > 0)
            && self.hardware_plan_commit_blocker_component_count()
                == self.plan_shape_problem_component_count()
            && self.has_hardware_plan_commit_blockers()
                == (self.hardware_plan_commit_blocker_component_count() > 0)
    }

    pub fn hardware_plan_commit_is_clean(self) -> bool {
        !self.has_hardware_plan_commit_blockers()
            && self.hardware_plan_commit_accounting_is_consistent()
    }

    pub fn can_commit_hardware_plan(self) -> bool {
        self.hardware_plan_commit_is_clean()
    }

    pub fn hardware_plan_commit_action(self) -> HardwarePlanCommitAction {
        if self.can_commit_hardware_plan() {
            HardwarePlanCommitAction::CommitHardwarePlan
        } else {
            HardwarePlanCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_hardware_plan(self) -> bool {
        self.hardware_plan_shape_is_clean()
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.hardware_plan_commit_accounting_is_consistent())
    }

    pub fn hardware_plan_problem_component_count(self) -> usize {
        self.hardware_plan_commit_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_hardware_plan_problem_components(self) -> bool {
        self.hardware_plan_problem_component_count() > 0
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.hardware_plan_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "hardware plan failed: components={component_count}"
            )))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.failure_report().into_iter().collect()
    }

    pub fn failure_report_count(self) -> usize {
        usize::from(self.failure_report().is_some())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.failure_report()
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(self) -> HardwarePlanCommitSummary {
        HardwarePlanCommitSummary::new(self)
    }
}

impl HardwarePlanCommitSummary {
    pub fn new(plan: HardwarePlanSummary) -> Self {
        let failure_reports = plan.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = plan.can_commit_hardware_plan();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = plan.hardware_plan_commit_action();

        Self {
            plan,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: plan.hardware_plan_commit_signal_component_count(),
            total_blocker_component_count: plan.hardware_plan_commit_blocker_component_count(),
            component_accounting_consistent: plan.hardware_plan_commit_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(&self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(&self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(&self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_batch_shape_is_clean(&self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(&self) -> HardwareFailureReturnSummary {
        HardwareFailureReturnSummary::new(
            HardwareFailureReturnSource::HardwarePlan,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<HardwareFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                HardwareFailureReturnReport::new(
                    HardwareFailureReturnSource::HardwarePlan,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(&self) -> bool {
        self.can_commit == self.plan.can_commit_hardware_plan()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.plan.hardware_plan_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.plan.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self.plan.hardware_plan_commit_signal_component_count()
            && self.total_blocker_component_count
                == self.plan.hardware_plan_commit_blocker_component_count()
            && self.component_accounting_consistent
                == self.plan.hardware_plan_commit_accounting_is_consistent()
    }

    pub fn can_commit_hardware_plan(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl HardwareAdapterBridgeSummary {
    pub fn adapter_counts_match(self) -> bool {
        self.plan.adapter_count == self.context.adapter_count
            && self.execution.adapter_count == self.context.adapter_count
    }

    pub fn pressure_matches(self) -> bool {
        float_close(self.plan.pressure, self.context.hardware_pressure)
            && float_close(self.plan.compute_headroom, self.context.compute_headroom)
    }

    pub fn latency_matches(self) -> bool {
        self.plan.latency_budget_ms == self.context.latency_budget_ms
    }

    pub fn parallelism_matches(self) -> bool {
        self.plan.max_parallel_chunks == self.context.max_parallel_chunks
            && self.execution.max_parallel_chunks == self.context.max_parallel_chunks
    }

    pub fn kv_prefetch_matches(self) -> bool {
        self.plan.kv_prefetch_blocks == self.context.kv_prefetch_blocks
            && self.execution.kv_prefetch_blocks == self.context.kv_prefetch_blocks
    }

    pub fn precision_matches(self) -> bool {
        self.plan.hot_kv_precision_bits == self.context.hot_kv_precision_bits
            && self.plan.cold_kv_precision_bits == self.context.cold_kv_precision_bits
            && self.execution.hot_kv_precision_bits == self.context.hot_kv_precision_bits
            && self.execution.cold_kv_precision_bits == self.context.cold_kv_precision_bits
    }

    pub fn token_budgets_match(self) -> bool {
        self.plan.local_kv_token_budget == self.context.local_kv_token_budget
            && self.plan.global_kv_token_budget == self.context.global_kv_token_budget
    }

    pub fn disk_spill_matches(self) -> bool {
        self.plan.allow_disk_spill == self.context.allow_disk_spill
            && self.execution.allow_disk_spill == self.context.allow_disk_spill
    }

    pub fn adapter_count_drifted(self) -> bool {
        !self.adapter_counts_match()
    }

    pub fn pressure_drifted(self) -> bool {
        !self.pressure_matches()
    }

    pub fn latency_drifted(self) -> bool {
        !self.latency_matches()
    }

    pub fn parallelism_drifted(self) -> bool {
        !self.parallelism_matches()
    }

    pub fn kv_prefetch_drifted(self) -> bool {
        !self.kv_prefetch_matches()
    }

    pub fn precision_drifted(self) -> bool {
        !self.precision_matches()
    }

    pub fn token_budget_drifted(self) -> bool {
        !self.token_budgets_match()
    }

    pub fn disk_spill_drifted(self) -> bool {
        !self.disk_spill_matches()
    }

    pub fn adapter_count_drift_component_count(self) -> usize {
        usize::from(self.adapter_count_drifted())
    }

    pub fn pressure_drift_component_count(self) -> usize {
        usize::from(self.pressure_drifted())
    }

    pub fn latency_drift_component_count(self) -> usize {
        usize::from(self.latency_drifted())
    }

    pub fn parallelism_drift_component_count(self) -> usize {
        usize::from(self.parallelism_drifted())
    }

    pub fn kv_prefetch_drift_component_count(self) -> usize {
        usize::from(self.kv_prefetch_drifted())
    }

    pub fn precision_drift_component_count(self) -> usize {
        usize::from(self.precision_drifted())
    }

    pub fn token_budget_drift_component_count(self) -> usize {
        usize::from(self.token_budget_drifted())
    }

    pub fn disk_spill_drift_component_count(self) -> usize {
        usize::from(self.disk_spill_drifted())
    }

    pub fn adapter_bridge_drift_component_count(self) -> usize {
        self.adapter_count_drift_component_count()
            + self.pressure_drift_component_count()
            + self.latency_drift_component_count()
            + self.parallelism_drift_component_count()
            + self.kv_prefetch_drift_component_count()
            + self.precision_drift_component_count()
            + self.token_budget_drift_component_count()
            + self.disk_spill_drift_component_count()
    }

    pub fn has_adapter_bridge_drift_components(self) -> bool {
        self.adapter_bridge_drift_component_count() > 0
    }

    pub fn adapter_bridge_preservation_signal_component_count(self) -> usize {
        usize::from(self.adapter_counts_match())
            .saturating_add(usize::from(self.pressure_matches()))
            .saturating_add(usize::from(self.latency_matches()))
            .saturating_add(usize::from(self.parallelism_matches()))
            .saturating_add(usize::from(self.kv_prefetch_matches()))
            .saturating_add(usize::from(self.precision_matches()))
            .saturating_add(usize::from(self.token_budgets_match()))
            .saturating_add(usize::from(self.disk_spill_matches()))
    }

    pub fn has_adapter_bridge_preservation_signals(self) -> bool {
        self.adapter_bridge_preservation_signal_component_count() > 0
    }

    pub fn hardware_adapter_bridge_signal_component_count(self) -> usize {
        self.adapter_bridge_preservation_signal_component_count()
    }

    pub fn has_hardware_adapter_bridge_signals(self) -> bool {
        self.hardware_adapter_bridge_signal_component_count() > 0
    }

    pub fn hardware_adapter_bridge_blocker_component_count(self) -> usize {
        self.adapter_bridge_drift_component_count()
    }

    pub fn has_hardware_adapter_bridge_blockers(self) -> bool {
        self.hardware_adapter_bridge_blocker_component_count() > 0
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.hardware_adapter_bridge_accounting_is_consistent())
    }

    pub fn hardware_adapter_bridge_problem_component_count(self) -> usize {
        self.hardware_adapter_bridge_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_hardware_adapter_bridge_problem_components(self) -> bool {
        self.hardware_adapter_bridge_problem_component_count() > 0
    }

    pub fn adapter_bridge_accounting_is_consistent(self) -> bool {
        let expected_drift_count = self
            .adapter_count_drift_component_count()
            .saturating_add(self.pressure_drift_component_count())
            .saturating_add(self.latency_drift_component_count())
            .saturating_add(self.parallelism_drift_component_count())
            .saturating_add(self.kv_prefetch_drift_component_count())
            .saturating_add(self.precision_drift_component_count())
            .saturating_add(self.token_budget_drift_component_count())
            .saturating_add(self.disk_spill_drift_component_count());
        let expected_preservation_signal_count = usize::from(self.adapter_counts_match())
            .saturating_add(usize::from(self.pressure_matches()))
            .saturating_add(usize::from(self.latency_matches()))
            .saturating_add(usize::from(self.parallelism_matches()))
            .saturating_add(usize::from(self.kv_prefetch_matches()))
            .saturating_add(usize::from(self.precision_matches()))
            .saturating_add(usize::from(self.token_budgets_match()))
            .saturating_add(usize::from(self.disk_spill_matches()));

        self.adapter_bridge_drift_component_count() == expected_drift_count
            && self.has_adapter_bridge_drift_components() == (expected_drift_count > 0)
            && self.adapter_bridge_preservation_signal_component_count()
                == expected_preservation_signal_count
            && self.has_adapter_bridge_preservation_signals()
                == (expected_preservation_signal_count > 0)
    }

    pub fn is_lossless_bridge(self) -> bool {
        self.adapter_counts_match()
            && self.pressure_matches()
            && self.latency_matches()
            && self.parallelism_matches()
            && self.kv_prefetch_matches()
            && self.precision_matches()
            && self.token_budgets_match()
            && self.disk_spill_matches()
    }

    pub fn adapter_bridge_shape_is_clean(self) -> bool {
        !self.has_adapter_bridge_drift_components()
            && self.adapter_bridge_accounting_is_consistent()
            && self.is_lossless_bridge()
    }

    pub fn hardware_adapter_bridge_accounting_is_consistent(self) -> bool {
        self.adapter_bridge_accounting_is_consistent()
            && self.hardware_adapter_bridge_signal_component_count()
                == self.adapter_bridge_preservation_signal_component_count()
            && self.has_hardware_adapter_bridge_signals()
                == (self.hardware_adapter_bridge_signal_component_count() > 0)
            && self.hardware_adapter_bridge_blocker_component_count()
                == self.adapter_bridge_drift_component_count()
            && self.has_hardware_adapter_bridge_blockers()
                == (self.hardware_adapter_bridge_blocker_component_count() > 0)
    }

    pub fn hardware_adapter_bridge_commit_is_clean(self) -> bool {
        !self.has_hardware_adapter_bridge_blockers()
            && self.hardware_adapter_bridge_accounting_is_consistent()
            && self.is_lossless_bridge()
    }

    pub fn can_commit_hardware_adapter_bridge(self) -> bool {
        self.hardware_adapter_bridge_commit_is_clean()
    }

    pub fn hardware_adapter_bridge_commit_action(self) -> HardwareAdapterBridgeCommitAction {
        if self.can_commit_hardware_adapter_bridge() {
            HardwareAdapterBridgeCommitAction::CommitHardwareAdapterBridge
        } else {
            HardwareAdapterBridgeCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn can_use_hardware_adapter_bridge(self) -> bool {
        self.adapter_bridge_shape_is_clean()
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.hardware_adapter_bridge_problem_component_count();
        if component_count == 0 {
            None
        } else {
            Some(RuntimeFailureReport::contract_violation(format!(
                "hardware adapter bridge failed: components={component_count}"
            )))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.failure_report().into_iter().collect()
    }

    pub fn failure_report_count(self) -> usize {
        usize::from(self.failure_report().is_some())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.failure_report()
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn commit_summary(self) -> HardwareAdapterBridgeCommitSummary {
        HardwareAdapterBridgeCommitSummary::new(self)
    }
}

impl HardwareAdapterBridgeCommitSummary {
    pub fn new(bridge: HardwareAdapterBridgeSummary) -> Self {
        let failure_reports = bridge.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = bridge.can_commit_hardware_adapter_bridge();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = bridge.hardware_adapter_bridge_commit_action();

        Self {
            bridge,
            action,
            can_commit,
            should_return_failure,
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: bridge.hardware_adapter_bridge_signal_component_count(),
            total_blocker_component_count: bridge.hardware_adapter_bridge_blocker_component_count(),
            component_accounting_consistent: bridge
                .hardware_adapter_bridge_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(&self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(&self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(&self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_batch_shape_is_clean(&self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(&self) -> HardwareFailureReturnSummary {
        HardwareFailureReturnSummary::new(
            HardwareFailureReturnSource::AdapterBridge,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<HardwareFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                HardwareFailureReturnReport::new(
                    HardwareFailureReturnSource::AdapterBridge,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(&self) -> bool {
        self.can_commit == self.bridge.can_commit_hardware_adapter_bridge()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.bridge.hardware_adapter_bridge_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.bridge.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self.bridge.hardware_adapter_bridge_signal_component_count()
            && self.total_blocker_component_count
                == self
                    .bridge
                    .hardware_adapter_bridge_blocker_component_count()
            && self.component_accounting_consistent
                == self
                    .bridge
                    .hardware_adapter_bridge_accounting_is_consistent()
    }

    pub fn can_commit_hardware_adapter_bridge(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl HardwareRuntimeReadinessSummary {
    pub fn new(
        snapshot: HardwareLoadSnapshotSummary,
        plan: HardwarePlanSummary,
        execution: DeviceExecutionPlanSummary,
        bridge: HardwareAdapterBridgeSummary,
    ) -> Self {
        Self {
            snapshot,
            plan,
            execution,
            bridge,
            snapshot_signal_component_count: snapshot
                .hardware_snapshot_commit_signal_component_count(),
            hardware_plan_signal_component_count: plan
                .hardware_plan_commit_signal_component_count(),
            device_execution_signal_component_count: execution
                .hardware_execution_signal_component_count(),
            adapter_bridge_signal_component_count: bridge
                .hardware_adapter_bridge_signal_component_count(),
            snapshot_blocker_component_count: snapshot
                .hardware_snapshot_commit_blocker_component_count(),
            hardware_plan_blocker_component_count: plan
                .hardware_plan_commit_blocker_component_count(),
            device_execution_blocker_component_count: execution
                .hardware_execution_blocker_component_count(),
            adapter_bridge_blocker_component_count: bridge
                .hardware_adapter_bridge_blocker_component_count(),
        }
    }

    pub fn from_plan(snapshot: HardwareLoadSnapshotSummary, plan: &HardwarePlan) -> Self {
        Self::new(
            snapshot,
            plan.plan_summary(),
            plan.execution.execution_summary(),
            plan.adapter_bridge_summary(),
        )
    }

    pub fn stage_order() -> [HardwareRuntimeReadinessStage; 4] {
        [
            HardwareRuntimeReadinessStage::LoadSnapshot,
            HardwareRuntimeReadinessStage::HardwarePlan,
            HardwareRuntimeReadinessStage::DeviceExecution,
            HardwareRuntimeReadinessStage::AdapterBridge,
        ]
    }

    pub fn snapshot_ready(self) -> bool {
        self.snapshot.can_commit_hardware_snapshot()
    }

    pub fn hardware_plan_ready(self) -> bool {
        self.plan.can_commit_hardware_plan()
    }

    pub fn device_execution_ready(self) -> bool {
        self.execution.can_commit_device_execution_plan()
    }

    pub fn adapter_bridge_ready(self) -> bool {
        self.bridge.can_commit_hardware_adapter_bridge()
    }

    pub fn stage_ready(self, stage: HardwareRuntimeReadinessStage) -> bool {
        match stage {
            HardwareRuntimeReadinessStage::LoadSnapshot => self.snapshot_ready(),
            HardwareRuntimeReadinessStage::HardwarePlan => self.hardware_plan_ready(),
            HardwareRuntimeReadinessStage::DeviceExecution => self.device_execution_ready(),
            HardwareRuntimeReadinessStage::AdapterBridge => self.adapter_bridge_ready(),
        }
    }

    pub fn stage_signal_component_count(self, stage: HardwareRuntimeReadinessStage) -> usize {
        match stage {
            HardwareRuntimeReadinessStage::LoadSnapshot => self.snapshot_signal_component_count,
            HardwareRuntimeReadinessStage::HardwarePlan => {
                self.hardware_plan_signal_component_count
            }
            HardwareRuntimeReadinessStage::DeviceExecution => {
                self.device_execution_signal_component_count
            }
            HardwareRuntimeReadinessStage::AdapterBridge => {
                self.adapter_bridge_signal_component_count
            }
        }
    }

    pub fn stage_blocker_component_count(self, stage: HardwareRuntimeReadinessStage) -> usize {
        match stage {
            HardwareRuntimeReadinessStage::LoadSnapshot => self.snapshot_blocker_component_count,
            HardwareRuntimeReadinessStage::HardwarePlan => {
                self.hardware_plan_blocker_component_count
            }
            HardwareRuntimeReadinessStage::DeviceExecution => {
                self.device_execution_blocker_component_count
            }
            HardwareRuntimeReadinessStage::AdapterBridge => {
                self.adapter_bridge_blocker_component_count
            }
        }
    }

    pub fn first_unready_stage(self) -> Option<HardwareRuntimeReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| !self.stage_ready(*stage))
    }

    pub fn first_blocking_stage(self) -> Option<HardwareRuntimeReadinessStage> {
        Self::stage_order()
            .into_iter()
            .find(|stage| self.stage_blocker_component_count(*stage) > 0)
    }

    pub fn component_accounting_drift_count(self) -> usize {
        usize::from(!self.hardware_runtime_accounting_is_consistent())
    }

    pub fn hardware_runtime_signal_component_count(self) -> usize {
        self.snapshot_signal_component_count
            .saturating_add(self.hardware_plan_signal_component_count)
            .saturating_add(self.device_execution_signal_component_count)
            .saturating_add(self.adapter_bridge_signal_component_count)
    }

    pub fn has_hardware_runtime_signals(self) -> bool {
        self.hardware_runtime_signal_component_count() > 0
    }

    pub fn hardware_runtime_blocker_component_count(self) -> usize {
        self.snapshot_blocker_component_count
            .saturating_add(self.hardware_plan_blocker_component_count)
            .saturating_add(self.device_execution_blocker_component_count)
            .saturating_add(self.adapter_bridge_blocker_component_count)
    }

    pub fn has_hardware_runtime_blockers(self) -> bool {
        self.hardware_runtime_blocker_component_count() > 0
    }

    pub fn hardware_runtime_problem_component_count(self) -> usize {
        self.hardware_runtime_blocker_component_count()
            .saturating_add(self.component_accounting_drift_count())
    }

    pub fn has_hardware_runtime_problem_components(self) -> bool {
        self.hardware_runtime_problem_component_count() > 0
    }

    pub fn failure_report(self) -> Option<RuntimeFailureReport> {
        let component_count = self.hardware_runtime_problem_component_count();
        if component_count == 0 {
            None
        } else {
            let stage = self
                .first_blocking_stage()
                .or_else(|| self.first_unready_stage())
                .map(HardwareRuntimeReadinessStage::label)
                .unwrap_or("accounting");
            Some(RuntimeFailureReport::contract_violation(format!(
                "hardware runtime readiness failed: stage={stage} components={component_count}"
            )))
        }
    }

    pub fn failure_reports(self) -> Vec<RuntimeFailureReport> {
        self.failure_report().into_iter().collect()
    }

    pub fn failure_report_count(self) -> usize {
        usize::from(self.failure_report().is_some())
    }

    pub fn has_failure_reports(self) -> bool {
        self.failure_report_count() > 0
    }

    pub fn failure_batch_summary(self) -> RuntimeFailureBatchSummary {
        RuntimeFailureReport::batch_summary(&self.failure_reports())
    }

    pub fn can_format_runtime_failures(self) -> bool {
        self.failure_batch_summary().can_format_runtime_failures()
    }

    pub fn primary_failure_report(self) -> Option<RuntimeFailureReport> {
        self.failure_report()
    }

    pub fn primary_failure_summary(self) -> Option<RuntimeFailureSummary> {
        self.primary_failure_report()
            .map(|failure| failure.failure_summary())
    }

    pub fn hardware_runtime_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .snapshot_signal_component_count
            .saturating_add(self.hardware_plan_signal_component_count)
            .saturating_add(self.device_execution_signal_component_count)
            .saturating_add(self.adapter_bridge_signal_component_count);
        let expected_blocker_count = self
            .snapshot_blocker_component_count
            .saturating_add(self.hardware_plan_blocker_component_count)
            .saturating_add(self.device_execution_blocker_component_count)
            .saturating_add(self.adapter_bridge_blocker_component_count);

        self.snapshot
            .hardware_snapshot_commit_accounting_is_consistent()
            && self.plan.hardware_plan_commit_accounting_is_consistent()
            && self.execution.hardware_execution_accounting_is_consistent()
            && self
                .bridge
                .hardware_adapter_bridge_accounting_is_consistent()
            && self.snapshot_signal_component_count
                == self
                    .snapshot
                    .hardware_snapshot_commit_signal_component_count()
            && self.hardware_plan_signal_component_count
                == self.plan.hardware_plan_commit_signal_component_count()
            && self.device_execution_signal_component_count
                == self.execution.hardware_execution_signal_component_count()
            && self.adapter_bridge_signal_component_count
                == self.bridge.hardware_adapter_bridge_signal_component_count()
            && self.snapshot_blocker_component_count
                == self
                    .snapshot
                    .hardware_snapshot_commit_blocker_component_count()
            && self.hardware_plan_blocker_component_count
                == self.plan.hardware_plan_commit_blocker_component_count()
            && self.device_execution_blocker_component_count
                == self.execution.hardware_execution_blocker_component_count()
            && self.adapter_bridge_blocker_component_count
                == self
                    .bridge
                    .hardware_adapter_bridge_blocker_component_count()
            && self.hardware_runtime_signal_component_count() == expected_signal_count
            && self.has_hardware_runtime_signals() == (expected_signal_count > 0)
            && self.hardware_runtime_blocker_component_count() == expected_blocker_count
            && self.has_hardware_runtime_blockers() == (expected_blocker_count > 0)
    }

    pub fn hardware_runtime_commit_is_clean(self) -> bool {
        !self.has_hardware_runtime_blockers() && self.hardware_runtime_accounting_is_consistent()
    }

    pub fn can_commit_hardware_runtime(self) -> bool {
        self.hardware_runtime_commit_is_clean()
            && self.snapshot_ready()
            && self.hardware_plan_ready()
            && self.device_execution_ready()
            && self.adapter_bridge_ready()
    }

    pub fn hardware_runtime_commit_action(self) -> HardwareRuntimeCommitAction {
        if self.can_commit_hardware_runtime() {
            HardwareRuntimeCommitAction::CommitHardwareRuntime
        } else {
            HardwareRuntimeCommitAction::ReturnRuntimeFailure
        }
    }

    pub fn commit_summary(self) -> HardwareRuntimeCommitSummary {
        HardwareRuntimeCommitSummary::new(self)
    }
}

impl HardwareRuntimeCommitSummary {
    pub fn new(readiness: HardwareRuntimeReadinessSummary) -> Self {
        let failure_reports = readiness.failure_reports();
        let primary_failure_report = failure_reports.first().cloned();
        let primary_failure_summary = primary_failure_report
            .as_ref()
            .map(|failure| failure.failure_summary());
        let failure_batch = RuntimeFailureReport::batch_summary(&failure_reports);
        let can_commit = readiness.can_commit_hardware_runtime();
        let failure_report_count = failure_reports.len();
        let can_format_runtime_failures = failure_batch.can_format_runtime_failures();
        let should_return_failure = !can_commit && failure_report_count > 0;
        let action = readiness.hardware_runtime_commit_action();

        Self {
            readiness,
            action,
            can_commit,
            should_return_failure,
            first_unready_stage: readiness.first_unready_stage(),
            first_blocking_stage: readiness.first_blocking_stage(),
            failure_reports,
            primary_failure_report,
            primary_failure_summary,
            failure_batch,
            failure_report_count,
            can_format_runtime_failures,
            total_signal_component_count: readiness.hardware_runtime_signal_component_count(),
            total_blocker_component_count: readiness.hardware_runtime_blocker_component_count(),
            component_accounting_consistent: readiness.hardware_runtime_accounting_is_consistent(),
        }
    }

    pub fn action_can_commit(&self) -> bool {
        self.action.can_commit()
    }

    pub fn action_should_return_failure(&self) -> bool {
        self.action.should_return_failure()
    }

    pub fn has_primary_failure_summary(&self) -> bool {
        self.primary_failure_summary.is_some()
    }

    pub fn failure_batch_shape_is_clean(&self) -> bool {
        self.failure_batch.failure_batch_shape_is_clean()
    }

    pub fn failure_return_summary(&self) -> HardwareFailureReturnSummary {
        HardwareFailureReturnSummary::new(
            HardwareFailureReturnSource::HardwareRuntime,
            self.can_commit,
            self.should_return_failure,
            self.primary_failure_summary,
            self.failure_batch,
            self.failure_report_count,
            self.can_format_runtime_failures,
            self.total_blocker_component_count,
            self.commit_decision_accounting_is_consistent(),
        )
    }

    pub fn runtime_failure_return_report(&self) -> Option<HardwareFailureReturnReport> {
        if self.failure_return_summary().can_return_runtime_failure() {
            self.primary_failure_report.clone().map(|failure| {
                HardwareFailureReturnReport::new(
                    HardwareFailureReturnSource::HardwareRuntime,
                    failure,
                    self.failure_batch,
                    self.failure_report_count,
                    self.can_format_runtime_failures,
                    self.total_blocker_component_count,
                )
            })
        } else {
            None
        }
    }

    pub fn commit_decision_accounting_is_consistent(&self) -> bool {
        self.can_commit == self.readiness.can_commit_hardware_runtime()
            && self.should_return_failure == (!self.can_commit && self.failure_report_count > 0)
            && self.action == self.readiness.hardware_runtime_commit_action()
            && self.action_can_commit() == self.can_commit
            && self.action_should_return_failure() == self.should_return_failure
            && self.first_unready_stage == self.readiness.first_unready_stage()
            && self.first_blocking_stage == self.readiness.first_blocking_stage()
            && self.failure_report_count == self.failure_reports.len()
            && self.failure_report_count == self.readiness.failure_report_count()
            && self.primary_failure_report.as_ref() == self.failure_reports.first()
            && self.primary_failure_summary
                == self
                    .primary_failure_report
                    .as_ref()
                    .map(|failure| failure.failure_summary())
            && self.failure_batch == RuntimeFailureReport::batch_summary(&self.failure_reports)
            && self.can_format_runtime_failures == self.failure_batch.can_format_runtime_failures()
            && self.total_signal_component_count
                == self.readiness.hardware_runtime_signal_component_count()
            && self.total_blocker_component_count
                == self.readiness.hardware_runtime_blocker_component_count()
            && self.component_accounting_consistent
                == self.readiness.hardware_runtime_accounting_is_consistent()
    }

    pub fn can_commit_hardware_runtime(&self) -> bool {
        self.can_commit && self.commit_decision_accounting_is_consistent()
    }

    pub fn hardware_runtime_admission_signal_component_count(&self) -> usize {
        self.total_signal_component_count
    }

    pub fn has_hardware_runtime_admission_signals(&self) -> bool {
        self.hardware_runtime_admission_signal_component_count() > 0
    }

    pub fn runtime_not_committable_component_count(&self) -> usize {
        usize::from(!self.can_commit)
    }

    pub fn hardware_runtime_admission_blocker_component_count(&self) -> usize {
        self.total_blocker_component_count
            .saturating_add(self.failure_report_count)
            .saturating_add(self.runtime_not_committable_component_count())
    }

    pub fn has_hardware_runtime_admission_blockers(&self) -> bool {
        self.hardware_runtime_admission_blocker_component_count() > 0
    }

    pub fn hardware_runtime_admission_accounting_is_consistent(&self) -> bool {
        let expected_signal_count = self.total_signal_component_count;
        let expected_blocker_count = self
            .total_blocker_component_count
            .saturating_add(self.failure_report_count)
            .saturating_add(usize::from(!self.can_commit));

        self.commit_decision_accounting_is_consistent()
            && self.hardware_runtime_admission_signal_component_count() == expected_signal_count
            && self.has_hardware_runtime_admission_signals() == (expected_signal_count > 0)
            && self.runtime_not_committable_component_count() == usize::from(!self.can_commit)
            && self.hardware_runtime_admission_blocker_component_count() == expected_blocker_count
            && self.has_hardware_runtime_admission_blockers() == (expected_blocker_count > 0)
    }

    pub fn hardware_runtime_admission_is_clean(&self) -> bool {
        !self.has_hardware_runtime_admission_blockers()
            && self.hardware_runtime_admission_accounting_is_consistent()
    }

    pub fn can_admit_hardware_runtime(&self) -> bool {
        self.can_commit_hardware_runtime() && self.hardware_runtime_admission_is_clean()
    }

    pub fn should_return_runtime_failure(&self) -> bool {
        self.should_return_failure && self.commit_decision_accounting_is_consistent()
    }
}

impl HardwarePlan {
    pub fn compute_headroom(&self) -> f32 {
        self.tier.compute_headroom()
    }

    pub fn pressure_band(&self) -> HardwarePressureBand {
        HardwarePressureBand::from_pressure(self.pressure)
    }

    pub fn is_pressure_constrained(&self) -> bool {
        self.pressure_band().is_constrained()
    }

    pub fn plan_summary(&self) -> HardwarePlanSummary {
        HardwarePlanSummary {
            device: self.device,
            tier: self.tier,
            pressure: self.pressure,
            pressure_band: self.pressure_band(),
            compute_headroom: self.compute_headroom(),
            latency_budget_ms: self.latency_budget_ms,
            local_kv_token_budget: self.local_kv_token_budget,
            global_kv_token_budget: self.global_kv_token_budget,
            max_parallel_chunks: self.execution.max_parallel_chunks,
            tier_parallel_chunks: tier_parallel_chunks(self.tier),
            kv_prefetch_blocks: self.execution.kv_prefetch_blocks,
            hot_kv_precision_bits: self.execution.hot_kv_precision_bits,
            cold_kv_precision_bits: self.execution.cold_kv_precision_bits,
            adapter_count: self.execution.adapter_hints.len(),
            allow_disk_spill: self.execution.allow_disk_spill,
            note_count: self.notes.len(),
        }
    }

    pub fn adapter_execution_context(&self) -> AdapterExecutionContext {
        AdapterExecutionContext::new(self.execution.adapter_hints.clone())
            .with_pressure(self.pressure, self.compute_headroom())
            .with_latency_budget_ms(self.latency_budget_ms)
            .with_parallel_chunks(self.execution.max_parallel_chunks)
            .with_kv_prefetch_blocks(self.execution.kv_prefetch_blocks)
            .with_kv_precision(
                self.execution.hot_kv_precision_bits,
                self.execution.cold_kv_precision_bits,
            )
            .with_kv_token_budgets(self.local_kv_token_budget, self.global_kv_token_budget)
            .with_disk_spill(self.execution.allow_disk_spill)
    }

    pub fn adapter_bridge_summary(&self) -> HardwareAdapterBridgeSummary {
        HardwareAdapterBridgeSummary {
            plan: self.plan_summary(),
            execution: self.execution.execution_summary(),
            context: self.adapter_execution_context().context_summary(),
        }
    }

    pub fn runtime_readiness_summary(
        &self,
        snapshot: HardwareLoadSnapshotSummary,
    ) -> HardwareRuntimeReadinessSummary {
        HardwareRuntimeReadinessSummary::from_plan(snapshot, self)
    }

    pub fn summary(&self) -> String {
        format!(
            "device={} tier={} pressure={:.3} compute_headroom={:.2} latency_budget_ms={} local_kv_tokens={} global_kv_tokens={} hierarchy=({:.2},{:.2},{:.2}) execution=({})",
            self.device.as_str(),
            self.tier.as_str(),
            self.pressure,
            self.compute_headroom(),
            self.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.local_kv_token_budget,
            self.global_kv_token_budget,
            self.hierarchy.global,
            self.hierarchy.local,
            self.hierarchy.fusion,
            self.execution.summary()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareAllocator {
    pub base_local_tokens: usize,
    pub base_global_tokens: usize,
}

impl HardwareAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn plan(
        &self,
        snapshot: HardwareLoadSnapshot,
        profile: TaskProfile,
        prompt_tokens: usize,
        base_hierarchy: HierarchyWeights,
    ) -> HardwarePlan {
        let pressure = snapshot.pressure();
        let scale = budget_scale(snapshot.device);
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
            scale.local * pressure_scale * long_context_scale,
        );
        let global_kv_token_budget = scaled_tokens(
            self.base_global_tokens,
            scale.global * pressure_scale * long_context_scale,
        );
        let hierarchy = adapt_hierarchy(base_hierarchy, snapshot.device, profile, pressure);
        let execution = device_execution_plan(snapshot.device, pressure);

        HardwarePlan {
            device: snapshot.device,
            tier: snapshot.device.tier(),
            pressure,
            latency_budget_ms: latency_budget(snapshot.device, pressure),
            local_kv_token_budget,
            global_kv_token_budget,
            hierarchy,
            execution,
            notes: hardware_notes(snapshot.device, pressure, prompt_tokens),
        }
    }
}

impl Default for HardwareAllocator {
    fn default() -> Self {
        Self {
            base_local_tokens: 512,
            base_global_tokens: 4096,
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

fn normalize_load(value: f32) -> f32 {
    if value > 1.0 {
        (value / 100.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn bounded_unit_float(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn device_descriptor(device: DeviceClass) -> DeviceProfileDescriptor {
    match device {
        DeviceClass::Auto => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "best-effort local probe with manual override",
            aliases: &["auto"],
        },
        DeviceClass::CpuOnly => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "portable CPU-only local targets",
            aliases: &["cpu", "cpu-only", "cpu_only"],
        },
        DeviceClass::IntegratedGpu => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "integrated GPU and APU local targets",
            aliases: &["integrated", "integrated-gpu", "igpu"],
        },
        DeviceClass::DiscreteGpu => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "single discrete GPU local targets",
            aliases: &["discrete", "discrete-gpu", "dgpu", "gpu"],
        },
        DeviceClass::UnifiedMemory => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "unified-memory local targets",
            aliases: &["uma", "unified-memory", "unified_memory"],
        },
        DeviceClass::Mobile => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "mobile and handheld local targets",
            aliases: &["mobile"],
        },
        DeviceClass::Embedded => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "embedded local targets",
            aliases: &["embedded"],
        },
        DeviceClass::BrowserWasm => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "browser, WASM, and web sandbox targets",
            aliases: &["browser-wasm", "wasm", "web"],
        },
        DeviceClass::Microcontroller => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "microcontroller and tiny no-std targets",
            aliases: &["microcontroller", "mcu"],
        },
        DeviceClass::NpuAccelerator => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "NPU and neural accelerator targets",
            aliases: &["npu", "npu-accelerator"],
        },
        DeviceClass::MultiGpu => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "multi-accelerator local targets",
            aliases: &["multi-gpu", "multi_gpu", "distributed"],
        },
        DeviceClass::Edge => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "edge gateway and industrial local targets",
            aliases: &["edge"],
        },
        DeviceClass::Server => DeviceProfileDescriptor {
            device,
            tier: device.tier(),
            scope: "server and workstation local targets",
            aliases: &["server"],
        },
    }
}

fn pressure_weights(device: DeviceClass) -> PressureWeights {
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
        DeviceClass::BrowserWasm => PressureWeights {
            cpu: 0.30,
            gpu: 0.18,
            ram: 0.44,
            disk: 0.08,
        },
        DeviceClass::Microcontroller => PressureWeights {
            cpu: 0.50,
            gpu: 0.00,
            ram: 0.42,
            disk: 0.08,
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

fn budget_scale(device: DeviceClass) -> BudgetScale {
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
            local: 0.42,
            global: 0.32,
        },
        DeviceClass::BrowserWasm => BudgetScale {
            local: 0.40,
            global: 0.30,
        },
        DeviceClass::Microcontroller => BudgetScale {
            local: 0.18,
            global: 0.12,
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
        DeviceClass::Microcontroller => 80,
        DeviceClass::BrowserWasm => 90,
        DeviceClass::Embedded => 105,
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

fn scaled_tokens(base: usize, scale: f32) -> usize {
    ((base as f32 * scale).round() as usize).max(32)
}

fn float_close(left: f32, right: f32) -> bool {
    (left - right).abs() <= 0.0001
}

fn valid_kv_precision_bits(bits: u8) -> bool {
    matches!(bits, 4 | 8)
}

fn device_execution_plan(device: DeviceClass, pressure: f32) -> DeviceExecutionPlan {
    let tier = device.tier();
    let base_parallel_chunks = tier_parallel_chunks(tier);
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
            DeviceTier::Balanced | DeviceTier::Auto => 3,
            DeviceTier::Accelerated => 5,
            DeviceTier::Distributed => 8,
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
    let (primary_lane, fallback_lane, memory_mode, adapter_hints, allow_disk_spill) =
        execution_shape(device);

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

fn tier_parallel_chunks(tier: DeviceTier) -> usize {
    match tier {
        DeviceTier::Tiny | DeviceTier::Constrained => 1,
        DeviceTier::Balanced | DeviceTier::Auto => 2,
        DeviceTier::Accelerated => 4,
        DeviceTier::Distributed => 8,
    }
}

fn execution_shape(
    device: DeviceClass,
) -> (
    ComputeLane,
    ComputeLane,
    DeviceMemoryMode,
    Vec<RuntimeAdapter>,
    bool,
) {
    match device {
        DeviceClass::CpuOnly => (
            ComputeLane::CpuVector,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapter::PortableRust,
                RuntimeAdapter::CpuSimd,
                RuntimeAdapter::OpenVino,
            ],
            true,
        ),
        DeviceClass::IntegratedGpu => (
            ComputeLane::IntegratedGpu,
            ComputeLane::CpuVector,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapter::Wgpu,
                RuntimeAdapter::Vulkan,
                RuntimeAdapter::DirectMl,
                RuntimeAdapter::OneApi,
                RuntimeAdapter::PortableRust,
            ],
            true,
        ),
        DeviceClass::DiscreteGpu => (
            ComputeLane::DiscreteGpu,
            ComputeLane::CpuVector,
            DeviceMemoryMode::GpuResident,
            vec![
                RuntimeAdapter::Cuda,
                RuntimeAdapter::Rocm,
                RuntimeAdapter::Vulkan,
                RuntimeAdapter::Wgpu,
                RuntimeAdapter::OneApi,
                RuntimeAdapter::DirectMl,
                RuntimeAdapter::PortableRust,
            ],
            true,
        ),
        DeviceClass::UnifiedMemory => (
            ComputeLane::UnifiedMemoryGpu,
            ComputeLane::CpuVector,
            DeviceMemoryMode::UnifiedMemory,
            vec![
                RuntimeAdapter::Metal,
                RuntimeAdapter::Wgpu,
                RuntimeAdapter::Vulkan,
                RuntimeAdapter::PortableRust,
            ],
            true,
        ),
        DeviceClass::Mobile => (
            ComputeLane::IntegratedGpu,
            ComputeLane::CpuVector,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapter::CoreMl,
                RuntimeAdapter::Nnapi,
                RuntimeAdapter::Qnn,
                RuntimeAdapter::Wgpu,
                RuntimeAdapter::WebGpu,
                RuntimeAdapter::PortableRust,
            ],
            true,
        ),
        DeviceClass::Embedded => (
            ComputeLane::DiskBackedStreaming,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::MinimalDisk,
            vec![
                RuntimeAdapter::PortableRust,
                RuntimeAdapter::Wgpu,
                RuntimeAdapter::Nnapi,
                RuntimeAdapter::Qnn,
                RuntimeAdapter::Rknn,
            ],
            true,
        ),
        DeviceClass::BrowserWasm => (
            ComputeLane::IntegratedGpu,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapter::WebGpu,
                RuntimeAdapter::Wgpu,
                RuntimeAdapter::PortableRust,
            ],
            true,
        ),
        DeviceClass::Microcontroller => (
            ComputeLane::DiskBackedStreaming,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::MinimalDisk,
            vec![RuntimeAdapter::PortableRust],
            true,
        ),
        DeviceClass::NpuAccelerator => (
            ComputeLane::NeuralAccelerator,
            ComputeLane::IntegratedGpu,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapter::CoreMl,
                RuntimeAdapter::Nnapi,
                RuntimeAdapter::Qnn,
                RuntimeAdapter::Cann,
                RuntimeAdapter::Mlu,
                RuntimeAdapter::Rknn,
                RuntimeAdapter::OpenVino,
                RuntimeAdapter::Wgpu,
                RuntimeAdapter::CustomAccelerator,
                RuntimeAdapter::PortableRust,
            ],
            true,
        ),
        DeviceClass::MultiGpu => (
            ComputeLane::MultiAccelerator,
            ComputeLane::DiscreteGpu,
            DeviceMemoryMode::DistributedSharded,
            vec![
                RuntimeAdapter::MultiDevice,
                RuntimeAdapter::Cuda,
                RuntimeAdapter::Rocm,
                RuntimeAdapter::OneApi,
                RuntimeAdapter::Vulkan,
                RuntimeAdapter::Wgpu,
                RuntimeAdapter::CustomAccelerator,
                RuntimeAdapter::PortableRust,
            ],
            false,
        ),
        DeviceClass::Edge => (
            ComputeLane::CpuVector,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapter::PortableRust,
                RuntimeAdapter::Wgpu,
                RuntimeAdapter::Vulkan,
                RuntimeAdapter::Nnapi,
                RuntimeAdapter::Qnn,
                RuntimeAdapter::Rknn,
                RuntimeAdapter::CustomAccelerator,
            ],
            true,
        ),
        DeviceClass::Server => (
            ComputeLane::DiscreteGpu,
            ComputeLane::CpuVector,
            DeviceMemoryMode::GpuResident,
            vec![
                RuntimeAdapter::Cuda,
                RuntimeAdapter::Rocm,
                RuntimeAdapter::OneApi,
                RuntimeAdapter::Vulkan,
                RuntimeAdapter::Wgpu,
                RuntimeAdapter::OpenVino,
                RuntimeAdapter::PortableRust,
            ],
            true,
        ),
        DeviceClass::Auto => (
            ComputeLane::CpuVector,
            ComputeLane::CpuPortable,
            DeviceMemoryMode::TieredDisk,
            vec![
                RuntimeAdapter::PortableRust,
                RuntimeAdapter::CpuSimd,
                RuntimeAdapter::Wgpu,
            ],
            true,
        ),
    }
}

fn adapt_hierarchy(
    base: HierarchyWeights,
    device: DeviceClass,
    profile: TaskProfile,
    pressure: f32,
) -> HierarchyWeights {
    let mut global = base.global;
    let mut local = base.local;
    let mut fusion = base.fusion;

    match device.tier() {
        DeviceTier::Tiny | DeviceTier::Constrained => {
            fusion += 0.16 + pressure * 0.08;
            global -= pressure * 0.12;
        }
        DeviceTier::Balanced => {
            local += 0.06;
            fusion += pressure * 0.06;
        }
        DeviceTier::Accelerated => {
            global += 0.04 * (1.0 - pressure);
            local += 0.03;
        }
        DeviceTier::Distributed => {
            global += 0.08 * (1.0 - pressure);
            fusion += 0.05;
        }
        DeviceTier::Auto => {}
    }

    if profile == TaskProfile::LongDocument {
        fusion += 0.08;
        local += 0.03;
    }

    HierarchyWeights::new(global, local, fusion)
}

fn hardware_notes(device: DeviceClass, pressure: f32, prompt_tokens: usize) -> Vec<String> {
    let mut notes = vec![
        format!("device:{}", device.as_str()),
        format!("tier:{}", device.tier().as_str()),
    ];

    if pressure >= 0.72 {
        notes.push("pressure:high_reduce_attention_and_prefetch".to_owned());
    } else if pressure >= 0.45 {
        notes.push("pressure:medium_apply_latency_budget".to_owned());
    } else {
        notes.push("pressure:low_full_budget".to_owned());
    }
    if prompt_tokens >= 32_000 {
        notes.push("context:very_long_reduce_kv_budget".to_owned());
    } else if prompt_tokens >= 8_192 {
        notes.push("context:long_reduce_kv_budget".to_owned());
    }

    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::RuntimeFailureKind;

    fn assert_clean_hardware_failure_return(
        failure_return: HardwareFailureReturnSummary,
        source: HardwareFailureReturnSource,
    ) {
        assert_eq!(failure_return.source, source);
        assert_eq!(failure_return.source.label(), source.label());
        assert!(failure_return.can_commit);
        assert!(!failure_return.should_return_failure);
        assert!(!failure_return.has_primary_failure_summary);
        assert_eq!(failure_return.primary_failure_summary, None);
        assert_eq!(failure_return.failure_report_count, 0);
        assert!(!failure_return.has_failure_reports());
        assert!(!failure_return.can_format_runtime_failures);
        assert_eq!(failure_return.total_blocker_component_count, 0);
        assert!(!failure_return.has_blocker_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(!failure_return.can_return_runtime_failure());
    }

    fn assert_blocked_hardware_failure_return(
        failure_return: HardwareFailureReturnSummary,
        report: HardwareFailureReturnReport,
        source: HardwareFailureReturnSource,
        message_fragment: &str,
    ) {
        assert_eq!(failure_return.source, source);
        assert_eq!(failure_return.source.label(), source.label());
        assert!(!failure_return.can_commit);
        assert!(failure_return.should_return_failure);
        assert!(failure_return.has_primary_failure_summary);
        assert_eq!(failure_return.failure_report_count, 1);
        assert!(failure_return.has_failure_reports());
        assert!(failure_return.can_format_runtime_failures);
        assert!(failure_return.total_blocker_component_count > 0);
        assert!(failure_return.has_blocker_components());
        assert!(failure_return.failure_return_accounting_is_consistent());
        assert!(failure_return.can_return_runtime_failure());

        assert_eq!(report.source, source);
        assert_eq!(
            report.primary_failure_summary,
            failure_return
                .primary_failure_summary
                .expect("primary failure summary is projected")
        );
        assert_eq!(report.failure_batch, failure_return.failure_batch);
        assert_eq!(
            report.failure_report_count,
            failure_return.failure_report_count
        );
        assert_eq!(
            report.can_format_runtime_failures,
            failure_return.can_format_runtime_failures
        );
        assert_eq!(
            report.total_blocker_component_count,
            failure_return.total_blocker_component_count
        );
        assert!(report.backend_message().contains(message_fragment));
        assert!(report.diagnostics_note().contains(message_fragment));
        assert_eq!(report.inference_error().message, report.backend_message());
        assert!(report.can_use_hardware_failure_return_report());
    }

    #[test]
    fn hardware_failure_return_projection_covers_clean_and_blocked_commits() {
        let clean_snapshot = HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20);
        let clean_plan = HardwareAllocator::new().plan(
            clean_snapshot,
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );

        let clean_snapshot_commit = clean_snapshot.snapshot_summary().commit_summary();
        assert_clean_hardware_failure_return(
            clean_snapshot_commit.failure_return_summary(),
            HardwareFailureReturnSource::LoadSnapshot,
        );
        assert_eq!(clean_snapshot_commit.runtime_failure_return_report(), None);

        let clean_execution_commit = clean_plan.execution.execution_summary().commit_summary();
        assert_clean_hardware_failure_return(
            clean_execution_commit.failure_return_summary(),
            HardwareFailureReturnSource::DeviceExecutionPlan,
        );
        assert_eq!(clean_execution_commit.runtime_failure_return_report(), None);

        let clean_adapter_commit = clean_plan.execution.adapter_hint_summary().commit_summary();
        assert_clean_hardware_failure_return(
            clean_adapter_commit.failure_return_summary(),
            HardwareFailureReturnSource::DeviceExecutionAdapters,
        );
        assert_eq!(clean_adapter_commit.runtime_failure_return_report(), None);

        let clean_plan_commit = clean_plan.plan_summary().commit_summary();
        assert_clean_hardware_failure_return(
            clean_plan_commit.failure_return_summary(),
            HardwareFailureReturnSource::HardwarePlan,
        );
        assert_eq!(clean_plan_commit.runtime_failure_return_report(), None);

        let clean_bridge_commit = clean_plan.adapter_bridge_summary().commit_summary();
        assert_clean_hardware_failure_return(
            clean_bridge_commit.failure_return_summary(),
            HardwareFailureReturnSource::AdapterBridge,
        );
        assert_eq!(clean_bridge_commit.runtime_failure_return_report(), None);

        let clean_runtime_commit = HardwareRuntimeReadinessSummary::from_plan(
            clean_snapshot.snapshot_summary(),
            &clean_plan,
        )
        .commit_summary();
        assert_clean_hardware_failure_return(
            clean_runtime_commit.failure_return_summary(),
            HardwareFailureReturnSource::HardwareRuntime,
        );
        assert_eq!(clean_runtime_commit.runtime_failure_return_report(), None);

        let blocked_snapshot_commit = HardwareLoadSnapshotSummary {
            device: DeviceClass::CpuOnly,
            tier: DeviceTier::Accelerated,
            cpu_load: -0.10,
            gpu_load: 1.20,
            ram_load: f32::NAN,
            disk_load: 0.20,
            pressure: 0.80,
            pressure_band: HardwarePressureBand::Low,
        }
        .commit_summary();
        assert_blocked_hardware_failure_return(
            blocked_snapshot_commit.failure_return_summary(),
            blocked_snapshot_commit
                .runtime_failure_return_report()
                .expect("snapshot failure return report"),
            HardwareFailureReturnSource::LoadSnapshot,
            "hardware load snapshot failed",
        );

        let blocked_execution_commit = DeviceExecutionPlanSummary {
            primary_lane: ComputeLane::DiskBackedStreaming,
            fallback_lane: ComputeLane::DiskBackedStreaming,
            memory_mode: DeviceMemoryMode::MinimalDisk,
            adapter_count: 0,
            max_parallel_chunks: 1,
            kv_prefetch_blocks: 0,
            hot_kv_precision_bits: 4,
            cold_kv_precision_bits: 8,
            allow_disk_spill: false,
        }
        .commit_summary();
        assert_blocked_hardware_failure_return(
            blocked_execution_commit.failure_return_summary(),
            blocked_execution_commit
                .runtime_failure_return_report()
                .expect("execution failure return report"),
            HardwareFailureReturnSource::DeviceExecutionPlan,
            "device execution plan failed",
        );

        let blocked_adapter_commit =
            DeviceExecutionAdapterSummary::from_adapters(&[]).commit_summary();
        assert_blocked_hardware_failure_return(
            blocked_adapter_commit.failure_return_summary(),
            blocked_adapter_commit
                .runtime_failure_return_report()
                .expect("adapter family failure return report"),
            HardwareFailureReturnSource::DeviceExecutionAdapters,
            "device execution adapter family failed",
        );

        let blocked_plan_commit = HardwarePlanSummary {
            device: DeviceClass::CpuOnly,
            tier: DeviceTier::Accelerated,
            pressure: f32::NAN,
            pressure_band: HardwarePressureBand::Low,
            compute_headroom: 1.20,
            latency_budget_ms: None,
            local_kv_token_budget: 0,
            global_kv_token_budget: 0,
            max_parallel_chunks: 0,
            tier_parallel_chunks: 0,
            kv_prefetch_blocks: 0,
            hot_kv_precision_bits: 6,
            cold_kv_precision_bits: 8,
            adapter_count: 0,
            allow_disk_spill: false,
            note_count: 0,
        }
        .commit_summary();
        assert_blocked_hardware_failure_return(
            blocked_plan_commit.failure_return_summary(),
            blocked_plan_commit
                .runtime_failure_return_report()
                .expect("hardware plan failure return report"),
            HardwareFailureReturnSource::HardwarePlan,
            "hardware plan failed",
        );

        let mut drifted_bridge = clean_plan.adapter_bridge_summary();
        drifted_bridge.context.max_parallel_chunks =
            drifted_bridge.context.max_parallel_chunks.saturating_add(1);
        let blocked_bridge_commit = drifted_bridge.commit_summary();
        assert_blocked_hardware_failure_return(
            blocked_bridge_commit.failure_return_summary(),
            blocked_bridge_commit
                .runtime_failure_return_report()
                .expect("adapter bridge failure return report"),
            HardwareFailureReturnSource::AdapterBridge,
            "hardware adapter bridge failed",
        );

        let blocked_runtime_commit = HardwareRuntimeReadinessSummary::new(
            clean_snapshot.snapshot_summary(),
            clean_plan.plan_summary(),
            clean_plan.execution.execution_summary(),
            drifted_bridge,
        )
        .commit_summary();
        assert_blocked_hardware_failure_return(
            blocked_runtime_commit.failure_return_summary(),
            blocked_runtime_commit
                .runtime_failure_return_report()
                .expect("hardware runtime failure return report"),
            HardwareFailureReturnSource::HardwareRuntime,
            "hardware runtime readiness failed",
        );
    }

    #[test]
    fn snapshot_normalizes_percent_loads_and_computes_pressure() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 50.0, 80.0, 30.0, 10.0);

        assert_eq!(snapshot.cpu_load, 0.50);
        assert_eq!(snapshot.gpu_load, 0.80);
        assert!(snapshot.pressure() > 0.50);
    }

    #[test]
    fn hardware_load_snapshot_summary_reports_pressure_shape() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.50, 0.90, 0.30, 0.10);

        let summary = snapshot.snapshot_summary();
        let (min_load, max_load) = summary.load_range();

        assert_eq!(summary.device, DeviceClass::DiscreteGpu);
        assert_eq!(summary.tier, DeviceTier::Accelerated);
        assert_eq!(summary.cpu_load, 0.50);
        assert_eq!(summary.gpu_load, 0.90);
        assert_eq!(summary.ram_load, 0.30);
        assert_eq!(summary.disk_load, 0.10);
        assert_eq!(summary.pressure, snapshot.pressure());
        assert_eq!(summary.pressure_band, HardwarePressureBand::Medium);
        assert!(!summary.is_pressure_constrained());
        assert!(summary.has_gpu_pressure());
        assert!(!summary.has_memory_pressure());
        assert_eq!(summary.dominant_load(), HardwareLoadKind::Gpu);
        assert_eq!(min_load, 0.10);
        assert_eq!(max_load, 0.90);
        assert!(summary.tier_matches_device());
        assert!(summary.load_values_are_bounded());
        assert!(summary.pressure_is_bounded());
        assert!(summary.pressure_band_matches_pressure());
        assert_eq!(summary.load_value_signal_component_count(), 4);
        assert_eq!(summary.pressure_shape_signal_component_count(), 3);
        assert_eq!(summary.snapshot_signal_component_count(), 7);
        assert!(summary.has_snapshot_signals());
        assert_eq!(summary.load_value_problem_component_count(), 0);
        assert_eq!(summary.pressure_shape_problem_component_count(), 0);
        assert_eq!(summary.tier_shape_problem_component_count(), 0);
        assert_eq!(summary.snapshot_shape_problem_component_count(), 0);
        assert!(!summary.has_snapshot_shape_problem_components());
        assert!(summary.snapshot_accounting_is_consistent());
        assert!(summary.snapshot_shape_is_clean());
        assert_eq!(summary.hardware_snapshot_commit_signal_component_count(), 7);
        assert!(summary.has_hardware_snapshot_commit_signals());
        assert_eq!(
            summary.hardware_snapshot_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_hardware_snapshot_commit_blockers());
        assert!(summary.hardware_snapshot_commit_accounting_is_consistent());
        assert!(summary.hardware_snapshot_commit_is_clean());
        assert!(summary.can_commit_hardware_snapshot());
        assert!(summary.can_use_hardware_snapshot());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(summary.hardware_snapshot_problem_component_count(), 0);
        assert!(!summary.has_hardware_snapshot_problem_components());
        assert_eq!(
            summary.hardware_load_snapshot_commit_action(),
            HardwareLoadSnapshotCommitAction::CommitHardwareLoadSnapshot
        );
        assert_eq!(summary.failure_report(), None);
        assert_eq!(summary.failure_reports(), Vec::new());
        assert_eq!(summary.failure_report_count(), 0);
        assert!(!summary.has_failure_reports());
        assert_eq!(summary.failure_batch_summary().total_count, 0);
        assert!(!summary.can_format_runtime_failures());
        assert_eq!(summary.primary_failure_report(), None);
        assert_eq!(summary.primary_failure_summary(), None);
        let commit = summary.commit_summary();
        assert_eq!(
            commit.action,
            HardwareLoadSnapshotCommitAction::CommitHardwareLoadSnapshot
        );
        assert_eq!(
            commit.action,
            summary.hardware_load_snapshot_commit_action()
        );
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_return_failure());
        assert!(commit.can_commit_hardware_snapshot());
        assert!(!commit.should_return_runtime_failure());
        assert!(commit.failure_reports.is_empty());
        assert_eq!(commit.primary_failure_report, None);
        assert_eq!(commit.primary_failure_summary, None);
        assert_eq!(commit.failure_report_count, 0);
        assert!(!commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 7);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(!commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn hardware_load_snapshot_summary_reports_memory_and_constrained_pressure() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::CpuOnly, 95.0, 5.0, 92.0, 80.0);

        let summary = snapshot.snapshot_summary();

        assert_eq!(summary.cpu_load, 0.95);
        assert_eq!(summary.gpu_load, 0.05);
        assert_eq!(summary.ram_load, 0.92);
        assert_eq!(summary.disk_load, 0.80);
        assert_eq!(summary.pressure_band, HardwarePressureBand::High);
        assert!(summary.is_pressure_constrained());
        assert!(!summary.has_gpu_pressure());
        assert!(summary.has_memory_pressure());
        assert_eq!(summary.dominant_load(), HardwareLoadKind::Cpu);
    }

    #[test]
    fn hardware_load_snapshot_summary_counts_public_shape_drift() {
        let summary = HardwareLoadSnapshotSummary {
            device: DeviceClass::CpuOnly,
            tier: DeviceTier::Accelerated,
            cpu_load: -0.10,
            gpu_load: 1.20,
            ram_load: f32::NAN,
            disk_load: 0.20,
            pressure: 0.80,
            pressure_band: HardwarePressureBand::Low,
        };

        assert!(!summary.tier_matches_device());
        assert!(!summary.cpu_load_is_bounded());
        assert!(!summary.gpu_load_is_bounded());
        assert!(!summary.ram_load_is_bounded());
        assert!(summary.disk_load_is_bounded());
        assert!(!summary.load_values_are_bounded());
        assert!(summary.pressure_is_bounded());
        assert!(!summary.pressure_band_matches_pressure());
        assert_eq!(summary.load_value_signal_component_count(), 1);
        assert_eq!(summary.pressure_shape_signal_component_count(), 1);
        assert_eq!(summary.snapshot_signal_component_count(), 2);
        assert!(summary.has_snapshot_signals());
        assert_eq!(summary.load_value_problem_component_count(), 3);
        assert_eq!(summary.pressure_shape_problem_component_count(), 1);
        assert_eq!(summary.tier_shape_problem_component_count(), 1);
        assert_eq!(summary.snapshot_shape_problem_component_count(), 5);
        assert!(summary.has_snapshot_shape_problem_components());
        assert!(summary.snapshot_accounting_is_consistent());
        assert!(!summary.snapshot_shape_is_clean());
        assert_eq!(summary.hardware_snapshot_commit_signal_component_count(), 2);
        assert!(summary.has_hardware_snapshot_commit_signals());
        assert_eq!(
            summary.hardware_snapshot_commit_blocker_component_count(),
            5
        );
        assert!(summary.has_hardware_snapshot_commit_blockers());
        assert!(summary.hardware_snapshot_commit_accounting_is_consistent());
        assert!(!summary.hardware_snapshot_commit_is_clean());
        assert!(!summary.can_commit_hardware_snapshot());
        assert!(!summary.can_use_hardware_snapshot());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(summary.hardware_snapshot_problem_component_count(), 5);
        assert!(summary.has_hardware_snapshot_problem_components());
        assert_eq!(
            summary.hardware_load_snapshot_commit_action(),
            HardwareLoadSnapshotCommitAction::ReturnRuntimeFailure
        );
        let report = summary.failure_report().expect("snapshot failure report");
        assert_eq!(report.kind, RuntimeFailureKind::ContractViolation);
        assert!(report.message.contains("components=5"));
        assert_eq!(summary.failure_reports(), vec![report.clone()]);
        assert_eq!(summary.failure_report_count(), 1);
        assert!(summary.has_failure_reports());
        assert_eq!(summary.failure_batch_summary().total_count, 1);
        assert_eq!(summary.failure_batch_summary().contract_violation_count, 1);
        assert!(summary.can_format_runtime_failures());
        assert_eq!(summary.primary_failure_report(), Some(report.clone()));
        assert_eq!(
            summary
                .primary_failure_summary()
                .map(|failure| failure.kind),
            Some(RuntimeFailureKind::ContractViolation)
        );
        let commit = summary.commit_summary();
        assert_eq!(
            commit.action,
            HardwareLoadSnapshotCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.action,
            summary.hardware_load_snapshot_commit_action()
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_hardware_snapshot());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.failure_reports, vec![report]);
        assert_eq!(commit.failure_report_count, 1);
        assert_eq!(commit.failure_batch.total_count, 1);
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 2);
        assert_eq!(commit.total_blocker_component_count, 5);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn allocator_reduces_kv_budget_under_pressure_and_long_context() {
        let allocator = HardwareAllocator::new();
        let low = allocator.plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.10, 0.10, 0.20, 0.10),
            TaskProfile::General,
            1024,
            HierarchyWeights::default(),
        );
        let high = allocator.plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.95, 0.95, 0.95, 0.95),
            TaskProfile::LongDocument,
            40_000,
            HierarchyWeights::for_profile(TaskProfile::LongDocument),
        );

        assert!(low.local_kv_token_budget > high.local_kv_token_budget);
        assert!(low.global_kv_token_budget > high.global_kv_token_budget);
        assert_eq!(high.execution.max_parallel_chunks, 1);
        assert_eq!(high.execution.kv_prefetch_blocks, 1);
        assert_eq!(high.execution.hot_kv_precision_bits, 4);
        assert!(high.latency_budget_ms.is_some());
    }

    #[test]
    fn hardware_plan_builds_adapter_execution_context() {
        let plan = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20),
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );

        let context = plan.adapter_execution_context();
        let execution_summary = plan.execution.execution_summary();
        let bridge = plan.adapter_bridge_summary();

        assert!(context.adapters.contains(&RuntimeAdapter::Cuda));
        assert_eq!(
            context.max_parallel_chunks,
            plan.execution.max_parallel_chunks
        );
        assert_eq!(
            context.kv_prefetch_blocks,
            plan.execution.kv_prefetch_blocks
        );
        assert_eq!(context.local_kv_token_budget, plan.local_kv_token_budget);
        assert_eq!(context.global_kv_token_budget, plan.global_kv_token_budget);
        assert_eq!(execution_summary.primary_lane, ComputeLane::DiscreteGpu);
        assert_eq!(execution_summary.fallback_lane, ComputeLane::CpuVector);
        assert_eq!(execution_summary.memory_mode, DeviceMemoryMode::GpuResident);
        assert_eq!(
            execution_summary.adapter_count,
            plan.execution.adapter_hints.len()
        );
        assert!(execution_summary.has_adapter_hints());
        assert!(!execution_summary.missing_adapter_hints());
        assert!(execution_summary.has_parallel_capacity());
        assert!(!execution_summary.lacks_parallel_capacity());
        assert!(execution_summary.has_kv_prefetch_capacity());
        assert!(!execution_summary.lacks_kv_prefetch_capacity());
        assert!(!execution_summary.uses_same_fallback_lane());
        assert!(execution_summary.has_distinct_fallback_lane());
        assert!(execution_summary.uses_gpu_or_accelerator());
        assert!(!execution_summary.uses_cpu_primary_lane());
        assert!(!execution_summary.uses_disk_streaming_lane());
        assert!(!execution_summary.uses_disk_backed_memory());
        assert!(!execution_summary.uses_compressed_hot_kv());
        assert!(execution_summary.kv_precision_is_compressed());
        assert!(execution_summary.cold_kv_not_wider_than_hot());
        assert!(!execution_summary.has_precision_inversion());
        assert!(!execution_summary.hot_and_cold_precision_match());
        assert!(execution_summary.has_valid_hot_kv_precision());
        assert!(execution_summary.has_valid_cold_kv_precision());
        assert_eq!(execution_summary.adapter_hint_signal_component_count(), 1);
        assert_eq!(
            execution_summary.execution_capacity_signal_component_count(),
            2
        );
        assert_eq!(execution_summary.primary_lane_signal_component_count(), 1);
        assert_eq!(execution_summary.fallback_lane_signal_component_count(), 1);
        assert_eq!(execution_summary.memory_mode_signal_component_count(), 1);
        assert_eq!(execution_summary.kv_precision_signal_component_count(), 3);
        assert_eq!(
            execution_summary.execution_constraint_signal_component_count(),
            0
        );
        assert_eq!(
            execution_summary.execution_shape_signal_component_count(),
            9
        );
        assert!(execution_summary.has_execution_shape_signals());
        assert_eq!(execution_summary.adapter_hint_problem_component_count(), 0);
        assert_eq!(
            execution_summary.execution_capacity_problem_component_count(),
            0
        );
        assert_eq!(execution_summary.precision_problem_component_count(), 0);
        assert_eq!(
            execution_summary.execution_shape_problem_component_count(),
            0
        );
        assert!(!execution_summary.has_execution_shape_problem_components());
        assert_eq!(execution_summary.execution_shape_risk_component_count(), 0);
        assert!(!execution_summary.has_execution_shape_risk());
        assert!(execution_summary.execution_shape_accounting_is_consistent());
        assert!(execution_summary.execution_shape_is_clean());
        assert_eq!(
            execution_summary.hardware_execution_signal_component_count(),
            9
        );
        assert!(execution_summary.has_hardware_execution_signals());
        assert_eq!(
            execution_summary.hardware_execution_blocker_component_count(),
            0
        );
        assert!(!execution_summary.has_hardware_execution_blockers());
        assert!(execution_summary.hardware_execution_accounting_is_consistent());
        assert!(execution_summary.hardware_execution_commit_is_clean());
        assert!(execution_summary.can_commit_device_execution_plan());
        assert!(execution_summary.can_use_device_execution_plan());
        assert_eq!(execution_summary.component_accounting_drift_count(), 0);
        assert_eq!(
            execution_summary.hardware_execution_problem_component_count(),
            0
        );
        assert!(!execution_summary.has_hardware_execution_problem_components());
        assert_eq!(execution_summary.failure_report(), None);
        assert_eq!(execution_summary.failure_reports(), Vec::new());
        assert_eq!(execution_summary.failure_report_count(), 0);
        assert!(!execution_summary.has_failure_reports());
        assert_eq!(execution_summary.failure_batch_summary().total_count, 0);
        assert!(!execution_summary.can_format_runtime_failures());
        assert_eq!(execution_summary.primary_failure_report(), None);
        assert_eq!(execution_summary.primary_failure_summary(), None);
        assert_eq!(
            execution_summary.device_execution_plan_commit_action(),
            DeviceExecutionPlanCommitAction::CommitDeviceExecutionPlan
        );
        let execution_commit = execution_summary.commit_summary();
        assert_eq!(
            execution_commit.action,
            DeviceExecutionPlanCommitAction::CommitDeviceExecutionPlan
        );
        assert_eq!(
            execution_commit.action,
            execution_summary.device_execution_plan_commit_action()
        );
        assert!(execution_commit.action_can_commit());
        assert!(!execution_commit.action_should_return_failure());
        assert!(execution_commit.can_commit_device_execution_plan());
        assert!(!execution_commit.should_return_runtime_failure());
        assert!(execution_commit.failure_reports.is_empty());
        assert_eq!(execution_commit.primary_failure_report, None);
        assert_eq!(execution_commit.primary_failure_summary, None);
        assert_eq!(execution_commit.failure_report_count, 0);
        assert!(!execution_commit.can_format_runtime_failures);
        assert_eq!(execution_commit.total_signal_component_count, 9);
        assert_eq!(execution_commit.total_blocker_component_count, 0);
        assert!(execution_commit.component_accounting_consistent);
        assert!(!execution_commit.has_primary_failure_summary());
        assert!(execution_commit.failure_batch_shape_is_clean());
        assert!(execution_commit.commit_decision_accounting_is_consistent());
        assert_eq!(bridge.plan, plan.plan_summary());
        assert_eq!(bridge.execution, execution_summary);
        assert_eq!(bridge.context, context.context_summary());
        assert!(bridge.adapter_counts_match());
        assert!(bridge.pressure_matches());
        assert!(bridge.latency_matches());
        assert!(bridge.parallelism_matches());
        assert!(bridge.kv_prefetch_matches());
        assert!(bridge.precision_matches());
        assert!(bridge.token_budgets_match());
        assert!(bridge.disk_spill_matches());
        assert!(!bridge.adapter_count_drifted());
        assert!(!bridge.pressure_drifted());
        assert!(!bridge.latency_drifted());
        assert!(!bridge.parallelism_drifted());
        assert!(!bridge.kv_prefetch_drifted());
        assert!(!bridge.precision_drifted());
        assert!(!bridge.token_budget_drifted());
        assert!(!bridge.disk_spill_drifted());
        assert_eq!(bridge.adapter_count_drift_component_count(), 0);
        assert_eq!(bridge.pressure_drift_component_count(), 0);
        assert_eq!(bridge.latency_drift_component_count(), 0);
        assert_eq!(bridge.parallelism_drift_component_count(), 0);
        assert_eq!(bridge.kv_prefetch_drift_component_count(), 0);
        assert_eq!(bridge.precision_drift_component_count(), 0);
        assert_eq!(bridge.token_budget_drift_component_count(), 0);
        assert_eq!(bridge.disk_spill_drift_component_count(), 0);
        assert_eq!(bridge.adapter_bridge_drift_component_count(), 0);
        assert!(!bridge.has_adapter_bridge_drift_components());
        assert_eq!(
            bridge.adapter_bridge_preservation_signal_component_count(),
            8
        );
        assert!(bridge.has_adapter_bridge_preservation_signals());
        assert_eq!(bridge.hardware_adapter_bridge_signal_component_count(), 8);
        assert!(bridge.has_hardware_adapter_bridge_signals());
        assert_eq!(bridge.hardware_adapter_bridge_blocker_component_count(), 0);
        assert!(!bridge.has_hardware_adapter_bridge_blockers());
        assert_eq!(bridge.component_accounting_drift_count(), 0);
        assert_eq!(bridge.hardware_adapter_bridge_problem_component_count(), 0);
        assert!(!bridge.has_hardware_adapter_bridge_problem_components());
        assert!(bridge.adapter_bridge_accounting_is_consistent());
        assert!(bridge.is_lossless_bridge());
        assert!(bridge.adapter_bridge_shape_is_clean());
        assert!(bridge.hardware_adapter_bridge_accounting_is_consistent());
        assert!(bridge.hardware_adapter_bridge_commit_is_clean());
        assert!(bridge.can_commit_hardware_adapter_bridge());
        assert!(bridge.can_use_hardware_adapter_bridge());
        assert_eq!(bridge.failure_report(), None);
        assert_eq!(bridge.failure_reports(), Vec::new());
        assert_eq!(bridge.failure_report_count(), 0);
        assert!(!bridge.has_failure_reports());
        assert_eq!(bridge.failure_batch_summary().total_count, 0);
        assert!(!bridge.can_format_runtime_failures());
        assert_eq!(bridge.primary_failure_report(), None);
        assert_eq!(bridge.primary_failure_summary(), None);
        assert_eq!(
            bridge.hardware_adapter_bridge_commit_action(),
            HardwareAdapterBridgeCommitAction::CommitHardwareAdapterBridge
        );
        let bridge_commit = bridge.commit_summary();
        assert_eq!(
            bridge_commit.action,
            HardwareAdapterBridgeCommitAction::CommitHardwareAdapterBridge
        );
        assert_eq!(
            bridge_commit.action,
            bridge.hardware_adapter_bridge_commit_action()
        );
        assert!(bridge_commit.action_can_commit());
        assert!(!bridge_commit.action_should_return_failure());
        assert!(bridge_commit.can_commit_hardware_adapter_bridge());
        assert!(!bridge_commit.should_return_runtime_failure());
        assert!(bridge_commit.failure_reports.is_empty());
        assert_eq!(bridge_commit.primary_failure_report, None);
        assert_eq!(bridge_commit.primary_failure_summary, None);
        assert_eq!(bridge_commit.failure_report_count, 0);
        assert!(!bridge_commit.can_format_runtime_failures);
        assert_eq!(bridge_commit.total_signal_component_count, 8);
        assert_eq!(bridge_commit.total_blocker_component_count, 0);
        assert!(bridge_commit.component_accounting_consistent);
        assert!(!bridge_commit.has_primary_failure_summary());
        assert!(bridge_commit.failure_batch_shape_is_clean());
        assert!(bridge_commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn hardware_runtime_readiness_summary_confirms_stage_order_and_counts() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20);
        let plan = HardwareAllocator::new().plan(
            snapshot,
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let readiness = plan.runtime_readiness_summary(snapshot.snapshot_summary());

        assert_eq!(
            HardwareRuntimeReadinessSummary::stage_order(),
            [
                HardwareRuntimeReadinessStage::LoadSnapshot,
                HardwareRuntimeReadinessStage::HardwarePlan,
                HardwareRuntimeReadinessStage::DeviceExecution,
                HardwareRuntimeReadinessStage::AdapterBridge,
            ]
        );
        assert!(readiness.snapshot_ready());
        assert!(readiness.hardware_plan_ready());
        assert!(readiness.device_execution_ready());
        assert!(readiness.adapter_bridge_ready());
        assert_eq!(readiness.first_unready_stage(), None);
        assert_eq!(readiness.first_blocking_stage(), None);
        assert_eq!(
            readiness.stage_signal_component_count(HardwareRuntimeReadinessStage::LoadSnapshot),
            readiness.snapshot_signal_component_count
        );
        assert_eq!(
            readiness.stage_blocker_component_count(HardwareRuntimeReadinessStage::AdapterBridge),
            readiness.adapter_bridge_blocker_component_count
        );
        assert_eq!(readiness.snapshot_signal_component_count, 7);
        assert_eq!(readiness.hardware_plan_signal_component_count, 1);
        assert_eq!(readiness.device_execution_signal_component_count, 9);
        assert_eq!(readiness.adapter_bridge_signal_component_count, 8);
        assert_eq!(readiness.hardware_runtime_signal_component_count(), 25);
        assert!(readiness.has_hardware_runtime_signals());
        assert_eq!(readiness.hardware_runtime_blocker_component_count(), 0);
        assert!(!readiness.has_hardware_runtime_blockers());
        assert_eq!(readiness.component_accounting_drift_count(), 0);
        assert_eq!(readiness.hardware_runtime_problem_component_count(), 0);
        assert!(!readiness.has_hardware_runtime_problem_components());
        assert_eq!(readiness.failure_report(), None);
        assert_eq!(readiness.failure_reports(), Vec::new());
        assert_eq!(readiness.failure_report_count(), 0);
        assert!(!readiness.has_failure_reports());
        assert_eq!(readiness.failure_batch_summary().total_count, 0);
        assert!(!readiness.can_format_runtime_failures());
        assert_eq!(readiness.primary_failure_report(), None);
        assert_eq!(readiness.primary_failure_summary(), None);
        assert!(readiness.hardware_runtime_accounting_is_consistent());
        assert!(readiness.hardware_runtime_commit_is_clean());
        assert!(readiness.can_commit_hardware_runtime());
        assert_eq!(
            readiness.hardware_runtime_commit_action(),
            HardwareRuntimeCommitAction::CommitHardwareRuntime
        );
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            HardwareRuntimeCommitAction::CommitHardwareRuntime
        );
        assert_eq!(commit.action, readiness.hardware_runtime_commit_action());
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_return_failure());
        assert!(commit.can_commit_hardware_runtime());
        assert!(!commit.should_return_runtime_failure());
        assert_eq!(commit.first_unready_stage, None);
        assert_eq!(commit.first_blocking_stage, None);
        assert!(commit.failure_reports.is_empty());
        assert_eq!(commit.primary_failure_report, None);
        assert_eq!(commit.primary_failure_summary, None);
        assert_eq!(commit.failure_report_count, 0);
        assert!(!commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 25);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(!commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn hardware_runtime_readiness_summary_routes_device_execution_blockers() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.95, 0.95, 0.95, 0.95);
        let plan = HardwareAllocator::new().plan(
            snapshot,
            TaskProfile::LongDocument,
            40_000,
            HierarchyWeights::for_profile(TaskProfile::LongDocument),
        );
        let readiness = plan.runtime_readiness_summary(snapshot.snapshot_summary());

        assert!(readiness.snapshot_ready());
        assert!(readiness.hardware_plan_ready());
        assert!(!readiness.device_execution_ready());
        assert!(readiness.adapter_bridge_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(HardwareRuntimeReadinessStage::DeviceExecution)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(HardwareRuntimeReadinessStage::DeviceExecution)
        );
        assert_eq!(readiness.snapshot_blocker_component_count, 0);
        assert_eq!(readiness.hardware_plan_blocker_component_count, 0);
        assert_eq!(readiness.device_execution_blocker_component_count, 1);
        assert_eq!(readiness.adapter_bridge_blocker_component_count, 0);
        assert_eq!(readiness.hardware_runtime_blocker_component_count(), 1);
        assert!(readiness.has_hardware_runtime_blockers());
        assert_eq!(readiness.hardware_runtime_problem_component_count(), 1);
        assert!(readiness.has_hardware_runtime_problem_components());
        assert!(readiness.hardware_runtime_accounting_is_consistent());
        assert!(!readiness.hardware_runtime_commit_is_clean());
        assert!(!readiness.can_commit_hardware_runtime());
        let failures = readiness.failure_reports();
        let primary_summary = readiness
            .primary_failure_summary()
            .expect("device execution failure summary is reported");
        assert_eq!(failures.len(), 1);
        assert_eq!(readiness.failure_report_count(), 1);
        assert!(readiness.has_failure_reports());
        assert!(failures[0].message.contains("device_execution"));
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(
            readiness.primary_failure_report(),
            Some(failures[0].clone())
        );
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert!(readiness.can_format_runtime_failures());
        assert_eq!(
            readiness.hardware_runtime_commit_action(),
            HardwareRuntimeCommitAction::ReturnRuntimeFailure
        );
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            HardwareRuntimeCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(commit.action, readiness.hardware_runtime_commit_action());
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_hardware_runtime());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(
            commit.first_unready_stage,
            Some(HardwareRuntimeReadinessStage::DeviceExecution)
        );
        assert_eq!(
            commit.first_blocking_stage,
            Some(HardwareRuntimeReadinessStage::DeviceExecution)
        );
        assert_eq!(commit.failure_reports, failures.clone());
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_blocker_component_count, 1);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn hardware_runtime_failure_return_preserves_device_execution_stage() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.95, 0.95, 0.95, 0.95);
        let plan = HardwareAllocator::new().plan(
            snapshot,
            TaskProfile::LongDocument,
            40_000,
            HierarchyWeights::for_profile(TaskProfile::LongDocument),
        );
        let readiness = plan.runtime_readiness_summary(snapshot.snapshot_summary());
        let commit = readiness.commit_summary();
        let failure_return = commit.failure_return_summary();
        let report = commit
            .runtime_failure_return_report()
            .expect("device execution hardware runtime failure return report");

        assert_eq!(
            readiness.first_blocking_stage(),
            Some(HardwareRuntimeReadinessStage::DeviceExecution)
        );
        assert_eq!(
            failure_return.source,
            HardwareFailureReturnSource::HardwareRuntime
        );
        assert!(failure_return.should_return_failure);
        assert!(failure_return.has_failure_reports());
        assert!(failure_return.has_blocker_components());
        assert!(failure_return.can_return_runtime_failure());
        assert_eq!(report.source, HardwareFailureReturnSource::HardwareRuntime);
        assert_eq!(
            report.primary_failure.kind,
            RuntimeFailureKind::ContractViolation
        );
        assert!(report.primary_failure.message.contains("device_execution"));
        assert_eq!(report.failure_batch.contract_violation_count, 1);
        assert_eq!(report.failure_report_count, 1);
        assert_eq!(report.total_blocker_component_count, 1);
        assert!(report.failure_return_report_shape_is_clean());
        assert!(report.can_use_hardware_failure_return_report());
        assert!(commit.should_return_runtime_failure());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn hardware_runtime_commit_summary_exposes_runtime_admission_boundary() {
        let clean_snapshot = HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20);
        let clean_plan = HardwareAllocator::new().plan(
            clean_snapshot,
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let clean = clean_plan
            .runtime_readiness_summary(clean_snapshot.snapshot_summary())
            .commit_summary();

        let blocked_snapshot =
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.95, 0.95, 0.95, 0.95);
        let blocked_plan = HardwareAllocator::new().plan(
            blocked_snapshot,
            TaskProfile::LongDocument,
            40_000,
            HierarchyWeights::for_profile(TaskProfile::LongDocument),
        );
        let blocked = blocked_plan
            .runtime_readiness_summary(blocked_snapshot.snapshot_summary())
            .commit_summary();

        assert_eq!(
            clean.action,
            HardwareRuntimeCommitAction::CommitHardwareRuntime
        );
        assert_eq!(
            clean.hardware_runtime_admission_signal_component_count(),
            25
        );
        assert!(clean.has_hardware_runtime_admission_signals());
        assert_eq!(clean.runtime_not_committable_component_count(), 0);
        assert_eq!(
            clean.hardware_runtime_admission_blocker_component_count(),
            0
        );
        assert!(!clean.has_hardware_runtime_admission_blockers());
        assert!(clean.hardware_runtime_admission_accounting_is_consistent());
        assert!(clean.hardware_runtime_admission_is_clean());
        assert!(clean.can_admit_hardware_runtime());
        assert!(clean.can_commit_hardware_runtime());
        assert!(!clean.should_return_runtime_failure());

        assert_eq!(
            blocked.action,
            HardwareRuntimeCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            blocked.hardware_runtime_admission_signal_component_count(),
            blocked.total_signal_component_count
        );
        assert!(blocked.has_hardware_runtime_admission_signals());
        assert_eq!(blocked.runtime_not_committable_component_count(), 1);
        assert_eq!(blocked.total_blocker_component_count, 1);
        assert_eq!(blocked.failure_report_count, 1);
        assert_eq!(
            blocked.hardware_runtime_admission_blocker_component_count(),
            3
        );
        assert!(blocked.has_hardware_runtime_admission_blockers());
        assert!(blocked.hardware_runtime_admission_accounting_is_consistent());
        assert!(!blocked.hardware_runtime_admission_is_clean());
        assert!(!blocked.can_admit_hardware_runtime());
        assert!(!blocked.can_commit_hardware_runtime());
        assert!(blocked.should_return_runtime_failure());
        assert_eq!(
            blocked.first_blocking_stage,
            Some(HardwareRuntimeReadinessStage::DeviceExecution)
        );
    }

    #[test]
    fn hardware_runtime_readiness_summary_routes_adapter_bridge_drift() {
        let snapshot = HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20);
        let plan = HardwareAllocator::new().plan(
            snapshot,
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let mut bridge = plan.adapter_bridge_summary();
        bridge.context.adapter_count = bridge.context.adapter_count.saturating_add(1);
        let readiness = HardwareRuntimeReadinessSummary::new(
            snapshot.snapshot_summary(),
            plan.plan_summary(),
            plan.execution.execution_summary(),
            bridge,
        );

        assert!(readiness.snapshot_ready());
        assert!(readiness.hardware_plan_ready());
        assert!(readiness.device_execution_ready());
        assert!(!readiness.adapter_bridge_ready());
        assert_eq!(
            readiness.first_unready_stage(),
            Some(HardwareRuntimeReadinessStage::AdapterBridge)
        );
        assert_eq!(
            readiness.first_blocking_stage(),
            Some(HardwareRuntimeReadinessStage::AdapterBridge)
        );
        assert_eq!(readiness.adapter_bridge_blocker_component_count, 1);
        assert_eq!(readiness.hardware_runtime_blocker_component_count(), 1);
        assert!(readiness.has_hardware_runtime_blockers());
        assert_eq!(readiness.hardware_runtime_problem_component_count(), 1);
        assert!(readiness.has_hardware_runtime_problem_components());
        assert!(readiness.hardware_runtime_accounting_is_consistent());
        assert!(!readiness.hardware_runtime_commit_is_clean());
        assert!(!readiness.can_commit_hardware_runtime());
        let failures = readiness.failure_reports();
        let primary_summary = readiness
            .primary_failure_summary()
            .expect("adapter bridge failure summary is reported");
        assert_eq!(failures.len(), 1);
        assert_eq!(readiness.failure_report_count(), 1);
        assert!(readiness.has_failure_reports());
        assert!(failures[0].message.contains("adapter_bridge"));
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert!(readiness.can_format_runtime_failures());
        assert_eq!(
            readiness.hardware_runtime_commit_action(),
            HardwareRuntimeCommitAction::ReturnRuntimeFailure
        );
        let commit = readiness.commit_summary();
        assert_eq!(
            commit.action,
            HardwareRuntimeCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(commit.action, readiness.hardware_runtime_commit_action());
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_hardware_runtime());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(
            commit.first_unready_stage,
            Some(HardwareRuntimeReadinessStage::AdapterBridge)
        );
        assert_eq!(
            commit.first_blocking_stage,
            Some(HardwareRuntimeReadinessStage::AdapterBridge)
        );
        assert_eq!(commit.failure_reports, failures.clone());
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_blocker_component_count, 1);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn hardware_adapter_bridge_summary_counts_bridge_drift_components() {
        let summary = HardwareAdapterBridgeSummary {
            plan: HardwarePlanSummary {
                device: DeviceClass::DiscreteGpu,
                tier: DeviceTier::Accelerated,
                pressure: 0.25,
                pressure_band: HardwarePressureBand::Low,
                compute_headroom: 0.85,
                latency_budget_ms: Some(80),
                local_kv_token_budget: 128,
                global_kv_token_budget: 256,
                max_parallel_chunks: 4,
                tier_parallel_chunks: 4,
                kv_prefetch_blocks: 3,
                hot_kv_precision_bits: 8,
                cold_kv_precision_bits: 4,
                adapter_count: 2,
                allow_disk_spill: true,
                note_count: 0,
            },
            execution: DeviceExecutionPlanSummary {
                primary_lane: ComputeLane::DiscreteGpu,
                fallback_lane: ComputeLane::CpuVector,
                memory_mode: DeviceMemoryMode::GpuResident,
                adapter_count: 1,
                max_parallel_chunks: 2,
                kv_prefetch_blocks: 1,
                hot_kv_precision_bits: 4,
                cold_kv_precision_bits: 4,
                allow_disk_spill: false,
            },
            context: AdapterExecutionContextSummary {
                adapter_count: 1,
                hardware_pressure: 0.40,
                compute_headroom: 0.70,
                latency_budget_ms: Some(40),
                max_parallel_chunks: 2,
                kv_prefetch_blocks: 1,
                hot_kv_precision_bits: 4,
                cold_kv_precision_bits: 4,
                local_kv_token_budget: 64,
                global_kv_token_budget: 128,
                allow_disk_spill: false,
            },
        };

        assert!(summary.adapter_count_drifted());
        assert!(summary.pressure_drifted());
        assert!(summary.latency_drifted());
        assert!(summary.parallelism_drifted());
        assert!(summary.kv_prefetch_drifted());
        assert!(summary.precision_drifted());
        assert!(summary.token_budget_drifted());
        assert!(summary.disk_spill_drifted());
        assert_eq!(summary.adapter_count_drift_component_count(), 1);
        assert_eq!(summary.pressure_drift_component_count(), 1);
        assert_eq!(summary.latency_drift_component_count(), 1);
        assert_eq!(summary.parallelism_drift_component_count(), 1);
        assert_eq!(summary.kv_prefetch_drift_component_count(), 1);
        assert_eq!(summary.precision_drift_component_count(), 1);
        assert_eq!(summary.token_budget_drift_component_count(), 1);
        assert_eq!(summary.disk_spill_drift_component_count(), 1);
        assert_eq!(summary.adapter_bridge_drift_component_count(), 8);
        assert!(summary.has_adapter_bridge_drift_components());
        assert_eq!(
            summary.adapter_bridge_preservation_signal_component_count(),
            0
        );
        assert!(!summary.has_adapter_bridge_preservation_signals());
        assert_eq!(summary.hardware_adapter_bridge_signal_component_count(), 0);
        assert!(!summary.has_hardware_adapter_bridge_signals());
        assert_eq!(summary.hardware_adapter_bridge_blocker_component_count(), 8);
        assert!(summary.has_hardware_adapter_bridge_blockers());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(summary.hardware_adapter_bridge_problem_component_count(), 8);
        assert!(summary.has_hardware_adapter_bridge_problem_components());
        assert!(summary.adapter_bridge_accounting_is_consistent());
        assert!(!summary.is_lossless_bridge());
        assert!(!summary.adapter_bridge_shape_is_clean());
        assert!(summary.hardware_adapter_bridge_accounting_is_consistent());
        assert!(!summary.hardware_adapter_bridge_commit_is_clean());
        assert!(!summary.can_commit_hardware_adapter_bridge());
        assert!(!summary.can_use_hardware_adapter_bridge());
        let failures = summary.failure_reports();
        let primary_summary = summary
            .primary_failure_summary()
            .expect("hardware adapter bridge failure summary is reported");
        assert_eq!(failures.len(), 1);
        assert_eq!(summary.failure_report_count(), 1);
        assert!(summary.has_failure_reports());
        assert_eq!(failures[0].kind, RuntimeFailureKind::ContractViolation);
        assert!(
            failures[0]
                .message
                .contains("hardware adapter bridge failed")
        );
        assert_eq!(summary.primary_failure_report(), Some(failures[0].clone()));
        assert_eq!(primary_summary.kind, RuntimeFailureKind::ContractViolation);
        assert!(summary.can_format_runtime_failures());
        assert_eq!(
            summary.hardware_adapter_bridge_commit_action(),
            HardwareAdapterBridgeCommitAction::ReturnRuntimeFailure
        );
        let commit = summary.commit_summary();
        assert_eq!(
            commit.action,
            HardwareAdapterBridgeCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.action,
            summary.hardware_adapter_bridge_commit_action()
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_hardware_adapter_bridge());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.failure_reports, failures.clone());
        assert_eq!(commit.primary_failure_report, Some(failures[0].clone()));
        assert_eq!(commit.primary_failure_summary, Some(primary_summary));
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert_eq!(commit.failure_report_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 0);
        assert_eq!(commit.total_blocker_component_count, 8);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn hardware_plan_summary_reports_pressure_and_budget_shape() {
        let low = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::Server, 0.20, 0.20, 0.20, 0.20),
            TaskProfile::Coding,
            4096,
            HierarchyWeights::for_profile(TaskProfile::Coding),
        );
        let high = HardwareAllocator::new().plan(
            HardwareLoadSnapshot::new(DeviceClass::DiscreteGpu, 0.95, 0.95, 0.95, 0.95),
            TaskProfile::LongDocument,
            40_000,
            HierarchyWeights::for_profile(TaskProfile::LongDocument),
        );

        let low_summary = low.plan_summary();
        let high_summary = high.plan_summary();

        assert_eq!(low_summary.pressure_band, HardwarePressureBand::Low);
        assert_eq!(low.pressure_band().as_str(), "low");
        assert!(!low.is_pressure_constrained());
        assert!(!low_summary.pressure_is_constrained());
        assert!(!low_summary.parallelism_was_reduced());
        assert!(!low_summary.kv_prefetch_is_minimal());
        assert!(!low_summary.uses_compressed_hot_kv());
        assert!(!low_summary.has_latency_budget());
        assert!(low_summary.can_spill_to_disk());
        assert!(!low_summary.cannot_spill_to_disk());
        assert!(low_summary.has_notes());
        assert_eq!(low_summary.plan_constraint_component_count(), 1);
        assert_eq!(low_summary.pressure_constraint_signal_component_count(), 0);
        assert_eq!(
            low_summary.parallelism_constraint_signal_component_count(),
            0
        );
        assert_eq!(
            low_summary.kv_prefetch_constraint_signal_component_count(),
            0
        );
        assert_eq!(low_summary.precision_constraint_signal_component_count(), 0);
        assert_eq!(low_summary.latency_constraint_signal_component_count(), 0);
        assert_eq!(
            low_summary.disk_spill_constraint_signal_component_count(),
            0
        );
        assert_eq!(low_summary.note_signal_component_count(), 1);
        assert_eq!(low_summary.plan_constraint_signal_component_count(), 1);
        assert!(low_summary.has_plan_constraint_signals());
        assert!(low_summary.has_plan_constraints());
        assert!(low_summary.plan_constraint_signal_accounting_is_consistent());
        assert!(low_summary.tier_matches_device());
        assert!(low_summary.pressure_is_bounded());
        assert!(low_summary.pressure_band_matches_pressure());
        assert!(low_summary.compute_headroom_is_bounded());
        assert!(low_summary.has_adapter_hints());
        assert!(low_summary.has_parallel_capacity());
        assert!(low_summary.has_tier_parallel_capacity());
        assert!(low_summary.has_valid_hot_kv_precision());
        assert!(low_summary.has_valid_cold_kv_precision());
        assert!(low_summary.cold_kv_not_wider_than_hot());
        assert_eq!(low_summary.plan_tier_problem_component_count(), 0);
        assert_eq!(low_summary.plan_pressure_problem_component_count(), 0);
        assert_eq!(low_summary.plan_capacity_problem_component_count(), 0);
        assert_eq!(low_summary.plan_precision_problem_component_count(), 0);
        assert_eq!(low_summary.plan_shape_problem_component_count(), 0);
        assert!(!low_summary.has_plan_shape_problem_components());
        assert!(low_summary.plan_shape_accounting_is_consistent());
        assert!(low_summary.hardware_plan_shape_is_clean());
        assert_eq!(low_summary.hardware_plan_commit_signal_component_count(), 1);
        assert!(low_summary.has_hardware_plan_commit_signals());
        assert_eq!(
            low_summary.hardware_plan_commit_blocker_component_count(),
            0
        );
        assert!(!low_summary.has_hardware_plan_commit_blockers());
        assert!(low_summary.hardware_plan_commit_accounting_is_consistent());
        assert!(low_summary.hardware_plan_commit_is_clean());
        assert!(low_summary.can_commit_hardware_plan());
        assert!(low_summary.can_use_hardware_plan());
        assert_eq!(low_summary.component_accounting_drift_count(), 0);
        assert_eq!(low_summary.hardware_plan_problem_component_count(), 0);
        assert!(!low_summary.has_hardware_plan_problem_components());
        assert_eq!(low_summary.failure_report(), None);
        assert_eq!(low_summary.failure_reports(), Vec::new());
        assert_eq!(low_summary.failure_report_count(), 0);
        assert!(!low_summary.has_failure_reports());
        assert_eq!(low_summary.failure_batch_summary().total_count, 0);
        assert!(!low_summary.can_format_runtime_failures());
        assert_eq!(low_summary.primary_failure_report(), None);
        assert_eq!(low_summary.primary_failure_summary(), None);
        assert_eq!(
            low_summary.hardware_plan_commit_action(),
            HardwarePlanCommitAction::CommitHardwarePlan
        );
        let low_commit = low_summary.commit_summary();
        assert_eq!(
            low_commit.action,
            HardwarePlanCommitAction::CommitHardwarePlan
        );
        assert_eq!(low_commit.action, low_summary.hardware_plan_commit_action());
        assert!(low_commit.action_can_commit());
        assert!(!low_commit.action_should_return_failure());
        assert!(low_commit.can_commit_hardware_plan());
        assert!(!low_commit.should_return_runtime_failure());
        assert!(low_commit.failure_reports.is_empty());
        assert_eq!(low_commit.primary_failure_report, None);
        assert_eq!(low_commit.primary_failure_summary, None);
        assert_eq!(low_commit.failure_report_count, 0);
        assert!(!low_commit.can_format_runtime_failures);
        assert_eq!(low_commit.total_signal_component_count, 1);
        assert_eq!(low_commit.total_blocker_component_count, 0);
        assert!(low_commit.component_accounting_consistent);
        assert!(!low_commit.has_primary_failure_summary());
        assert!(low_commit.failure_batch_shape_is_clean());
        assert!(low_commit.commit_decision_accounting_is_consistent());
        assert_eq!(low_summary.tier_parallel_chunks, 4);
        assert_eq!(low_summary.adapter_count, low.execution.adapter_hints.len());

        assert_eq!(high_summary.pressure_band, HardwarePressureBand::Critical);
        assert_eq!(high.pressure_band().as_str(), "critical");
        assert!(high.is_pressure_constrained());
        assert!(high_summary.pressure_is_constrained());
        assert!(high_summary.parallelism_was_reduced());
        assert!(high_summary.kv_prefetch_is_minimal());
        assert!(high_summary.uses_compressed_hot_kv());
        assert!(high_summary.has_latency_budget());
        assert!(!high_summary.cannot_spill_to_disk());
        assert!(high_summary.has_notes());
        assert_eq!(high_summary.plan_constraint_component_count(), 6);
        assert_eq!(high_summary.pressure_constraint_signal_component_count(), 1);
        assert_eq!(
            high_summary.parallelism_constraint_signal_component_count(),
            1
        );
        assert_eq!(
            high_summary.kv_prefetch_constraint_signal_component_count(),
            1
        );
        assert_eq!(
            high_summary.precision_constraint_signal_component_count(),
            1
        );
        assert_eq!(high_summary.latency_constraint_signal_component_count(), 1);
        assert_eq!(
            high_summary.disk_spill_constraint_signal_component_count(),
            0
        );
        assert_eq!(high_summary.note_signal_component_count(), 1);
        assert_eq!(high_summary.plan_constraint_signal_component_count(), 6);
        assert!(high_summary.has_plan_constraint_signals());
        assert!(high_summary.has_plan_constraints());
        assert!(high_summary.plan_constraint_signal_accounting_is_consistent());
        assert!(high_summary.tier_matches_device());
        assert!(high_summary.pressure_is_bounded());
        assert!(high_summary.pressure_band_matches_pressure());
        assert!(high_summary.compute_headroom_is_bounded());
        assert!(high_summary.has_adapter_hints());
        assert!(high_summary.has_parallel_capacity());
        assert!(high_summary.has_tier_parallel_capacity());
        assert!(high_summary.has_valid_hot_kv_precision());
        assert!(high_summary.has_valid_cold_kv_precision());
        assert!(high_summary.cold_kv_not_wider_than_hot());
        assert_eq!(high_summary.plan_tier_problem_component_count(), 0);
        assert_eq!(high_summary.plan_pressure_problem_component_count(), 0);
        assert_eq!(high_summary.plan_capacity_problem_component_count(), 0);
        assert_eq!(high_summary.plan_precision_problem_component_count(), 0);
        assert_eq!(high_summary.plan_shape_problem_component_count(), 0);
        assert!(!high_summary.has_plan_shape_problem_components());
        assert!(high_summary.plan_shape_accounting_is_consistent());
        assert!(high_summary.hardware_plan_shape_is_clean());
        assert_eq!(
            high_summary.hardware_plan_commit_signal_component_count(),
            6
        );
        assert!(high_summary.has_hardware_plan_commit_signals());
        assert_eq!(
            high_summary.hardware_plan_commit_blocker_component_count(),
            0
        );
        assert!(!high_summary.has_hardware_plan_commit_blockers());
        assert!(high_summary.hardware_plan_commit_accounting_is_consistent());
        assert!(high_summary.hardware_plan_commit_is_clean());
        assert!(high_summary.can_commit_hardware_plan());
        assert!(high_summary.can_use_hardware_plan());
        assert_eq!(high_summary.max_parallel_chunks, 1);
        assert_eq!(high_summary.tier_parallel_chunks, 4);
        assert_eq!(high_summary.note_count, high.notes.len());

        let high_execution = high.execution.execution_summary();

        assert_eq!(high_execution.max_parallel_chunks, 1);
        assert_eq!(high_execution.kv_prefetch_blocks, 1);
        assert_eq!(high_execution.hot_kv_precision_bits, 4);
        assert_eq!(high_execution.cold_kv_precision_bits, 4);
        assert!(high_execution.uses_compressed_hot_kv());
        assert!(high_execution.kv_precision_is_compressed());
        assert!(high_execution.hot_and_cold_precision_match());
        assert!(high_execution.allow_disk_spill);
        assert!(high_execution.has_adapter_hints());
        assert!(!high_execution.has_parallel_capacity());
        assert!(high_execution.lacks_parallel_capacity());
        assert!(high_execution.has_kv_prefetch_capacity());
        assert!(!high_execution.lacks_kv_prefetch_capacity());
        assert_eq!(high_execution.adapter_hint_signal_component_count(), 1);
        assert_eq!(
            high_execution.execution_capacity_signal_component_count(),
            1
        );
        assert_eq!(high_execution.primary_lane_signal_component_count(), 1);
        assert_eq!(high_execution.fallback_lane_signal_component_count(), 1);
        assert_eq!(high_execution.memory_mode_signal_component_count(), 1);
        assert_eq!(high_execution.kv_precision_signal_component_count(), 3);
        assert_eq!(
            high_execution.execution_constraint_signal_component_count(),
            1
        );
        assert_eq!(high_execution.execution_shape_signal_component_count(), 9);
        assert!(high_execution.has_execution_shape_signals());
        assert_eq!(high_execution.adapter_hint_problem_component_count(), 0);
        assert_eq!(
            high_execution.execution_capacity_problem_component_count(),
            1
        );
        assert_eq!(high_execution.precision_problem_component_count(), 0);
        assert_eq!(high_execution.execution_shape_problem_component_count(), 1);
        assert!(high_execution.has_execution_shape_problem_components());
        assert_eq!(high_execution.execution_shape_risk_component_count(), 2);
        assert!(high_execution.has_execution_shape_risk());
        assert!(high_execution.execution_shape_accounting_is_consistent());
        assert!(!high_execution.execution_shape_is_clean());
        assert_eq!(
            high_execution.hardware_execution_signal_component_count(),
            9
        );
        assert!(high_execution.has_hardware_execution_signals());
        assert_eq!(
            high_execution.hardware_execution_blocker_component_count(),
            1
        );
        assert!(high_execution.has_hardware_execution_blockers());
        assert!(high_execution.hardware_execution_accounting_is_consistent());
        assert!(!high_execution.hardware_execution_commit_is_clean());
        assert!(!high_execution.can_commit_device_execution_plan());
        assert!(!high_execution.can_use_device_execution_plan());

        let low_adapters = low.execution.adapter_hint_summary();
        let high_adapters = high.execution.adapter_hint_summary();

        assert_eq!(
            low_adapters.adapter_count,
            low.execution.adapter_hints.len()
        );
        assert!(low_adapters.has_portable_fallback());
        assert!(low_adapters.has_accelerator());
        assert!(low_adapters.has_mixed_fallback_and_accelerator());
        assert!(!low_adapters.is_accelerator_only());
        assert!(!low_adapters.is_fallback_only());
        assert!(low_adapters.gpu_count > 0);
        assert!(!low_adapters.is_empty());
        assert_eq!(
            high_adapters.adapter_count,
            high.execution.adapter_hints.len()
        );
        assert!(high_adapters.has_portable_fallback());
        assert!(high_adapters.has_accelerator());
        assert!(high_adapters.has_mixed_fallback_and_accelerator());
        assert!(high_adapters.gpu_count > 0 || high_adapters.neural_count > 0);
        assert!(!high_adapters.is_empty());
    }

    #[test]
    fn hardware_plan_summary_counts_commit_shape_blockers() {
        let summary = HardwarePlanSummary {
            device: DeviceClass::CpuOnly,
            tier: DeviceTier::Accelerated,
            pressure: f32::NAN,
            pressure_band: HardwarePressureBand::Low,
            compute_headroom: 1.20,
            latency_budget_ms: None,
            local_kv_token_budget: 0,
            global_kv_token_budget: 0,
            max_parallel_chunks: 0,
            tier_parallel_chunks: 0,
            kv_prefetch_blocks: 0,
            hot_kv_precision_bits: 6,
            cold_kv_precision_bits: 8,
            adapter_count: 0,
            allow_disk_spill: false,
            note_count: 0,
        };

        assert!(!summary.tier_matches_device());
        assert!(!summary.pressure_is_bounded());
        assert!(!summary.pressure_band_matches_pressure());
        assert!(!summary.compute_headroom_is_bounded());
        assert!(!summary.has_adapter_hints());
        assert!(!summary.has_parallel_capacity());
        assert!(!summary.has_tier_parallel_capacity());
        assert!(!summary.has_valid_hot_kv_precision());
        assert!(summary.has_valid_cold_kv_precision());
        assert!(!summary.cold_kv_not_wider_than_hot());
        assert!(!summary.pressure_is_constrained());
        assert!(!summary.parallelism_was_reduced());
        assert!(summary.kv_prefetch_is_minimal());
        assert!(!summary.uses_compressed_hot_kv());
        assert!(!summary.has_latency_budget());
        assert!(summary.cannot_spill_to_disk());
        assert!(!summary.has_notes());
        assert_eq!(summary.plan_constraint_signal_component_count(), 2);
        assert!(summary.has_plan_constraint_signals());
        assert!(summary.has_plan_constraints());
        assert!(summary.plan_constraint_signal_accounting_is_consistent());
        assert_eq!(summary.plan_tier_problem_component_count(), 1);
        assert_eq!(summary.plan_pressure_problem_component_count(), 3);
        assert_eq!(summary.plan_capacity_problem_component_count(), 3);
        assert_eq!(summary.plan_precision_problem_component_count(), 2);
        assert_eq!(summary.plan_shape_problem_component_count(), 9);
        assert!(summary.has_plan_shape_problem_components());
        assert!(summary.plan_shape_accounting_is_consistent());
        assert!(!summary.hardware_plan_shape_is_clean());
        assert_eq!(summary.hardware_plan_commit_signal_component_count(), 2);
        assert!(summary.has_hardware_plan_commit_signals());
        assert_eq!(summary.hardware_plan_commit_blocker_component_count(), 9);
        assert!(summary.has_hardware_plan_commit_blockers());
        assert!(summary.hardware_plan_commit_accounting_is_consistent());
        assert!(!summary.hardware_plan_commit_is_clean());
        assert!(!summary.can_commit_hardware_plan());
        assert!(!summary.can_use_hardware_plan());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(summary.hardware_plan_problem_component_count(), 9);
        assert!(summary.has_hardware_plan_problem_components());
        let report = summary
            .failure_report()
            .expect("hardware plan failure report");
        assert_eq!(report.kind, RuntimeFailureKind::ContractViolation);
        assert!(report.message.contains("components=9"));
        assert_eq!(summary.failure_reports(), vec![report.clone()]);
        assert_eq!(summary.failure_report_count(), 1);
        assert!(summary.has_failure_reports());
        assert_eq!(summary.failure_batch_summary().total_count, 1);
        assert_eq!(summary.failure_batch_summary().contract_violation_count, 1);
        assert!(summary.can_format_runtime_failures());
        assert_eq!(summary.primary_failure_report(), Some(report.clone()));
        assert_eq!(
            summary
                .primary_failure_summary()
                .map(|failure| failure.kind),
            Some(RuntimeFailureKind::ContractViolation)
        );
        assert_eq!(
            summary.hardware_plan_commit_action(),
            HardwarePlanCommitAction::ReturnRuntimeFailure
        );
        let commit = summary.commit_summary();
        assert_eq!(
            commit.action,
            HardwarePlanCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(commit.action, summary.hardware_plan_commit_action());
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_hardware_plan());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.failure_reports, vec![report]);
        assert_eq!(commit.failure_report_count, 1);
        assert_eq!(commit.failure_batch.total_count, 1);
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 2);
        assert_eq!(commit.total_blocker_component_count, 9);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn device_execution_plan_summary_counts_execution_shape_risk_components() {
        let summary = DeviceExecutionPlanSummary {
            primary_lane: ComputeLane::DiskBackedStreaming,
            fallback_lane: ComputeLane::DiskBackedStreaming,
            memory_mode: DeviceMemoryMode::MinimalDisk,
            adapter_count: 0,
            max_parallel_chunks: 1,
            kv_prefetch_blocks: 0,
            hot_kv_precision_bits: 4,
            cold_kv_precision_bits: 8,
            allow_disk_spill: false,
        };

        assert!(summary.missing_adapter_hints());
        assert!(summary.lacks_parallel_capacity());
        assert!(summary.lacks_kv_prefetch_capacity());
        assert!(summary.uses_same_fallback_lane());
        assert!(summary.uses_disk_streaming_lane());
        assert!(summary.uses_disk_backed_memory());
        assert!(summary.uses_compressed_hot_kv());
        assert!(summary.has_precision_inversion());
        assert!(!summary.cold_kv_not_wider_than_hot());
        assert!(summary.has_valid_hot_kv_precision());
        assert!(summary.has_valid_cold_kv_precision());
        assert_eq!(summary.adapter_hint_signal_component_count(), 0);
        assert_eq!(summary.execution_capacity_signal_component_count(), 0);
        assert_eq!(summary.primary_lane_signal_component_count(), 1);
        assert_eq!(summary.fallback_lane_signal_component_count(), 1);
        assert_eq!(summary.memory_mode_signal_component_count(), 1);
        assert_eq!(summary.kv_precision_signal_component_count(), 3);
        assert_eq!(summary.execution_constraint_signal_component_count(), 4);
        assert_eq!(summary.execution_shape_signal_component_count(), 10);
        assert!(summary.has_execution_shape_signals());
        assert_eq!(summary.adapter_hint_problem_component_count(), 1);
        assert_eq!(summary.execution_capacity_problem_component_count(), 2);
        assert_eq!(summary.precision_problem_component_count(), 1);
        assert_eq!(summary.execution_shape_problem_component_count(), 4);
        assert!(summary.has_execution_shape_problem_components());
        assert_eq!(summary.execution_shape_risk_component_count(), 8);
        assert!(summary.has_execution_shape_risk());
        assert!(summary.execution_shape_accounting_is_consistent());
        assert!(!summary.execution_shape_is_clean());
        assert_eq!(summary.hardware_execution_signal_component_count(), 10);
        assert!(summary.has_hardware_execution_signals());
        assert_eq!(summary.hardware_execution_blocker_component_count(), 4);
        assert!(summary.has_hardware_execution_blockers());
        assert!(summary.hardware_execution_accounting_is_consistent());
        assert!(!summary.hardware_execution_commit_is_clean());
        assert!(!summary.can_commit_device_execution_plan());
        assert!(!summary.can_use_device_execution_plan());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(summary.hardware_execution_problem_component_count(), 4);
        assert!(summary.has_hardware_execution_problem_components());
        let report = summary.failure_report().expect("execution failure report");
        assert_eq!(report.kind, RuntimeFailureKind::ContractViolation);
        assert!(report.message.contains("components=4"));
        assert_eq!(summary.failure_reports(), vec![report.clone()]);
        assert_eq!(summary.failure_report_count(), 1);
        assert!(summary.has_failure_reports());
        assert_eq!(summary.failure_batch_summary().total_count, 1);
        assert_eq!(summary.failure_batch_summary().contract_violation_count, 1);
        assert!(summary.can_format_runtime_failures());
        assert_eq!(summary.primary_failure_report(), Some(report.clone()));
        assert_eq!(
            summary
                .primary_failure_summary()
                .map(|failure| failure.kind),
            Some(RuntimeFailureKind::ContractViolation)
        );
        assert_eq!(
            summary.device_execution_plan_commit_action(),
            DeviceExecutionPlanCommitAction::ReturnRuntimeFailure
        );
        let commit = summary.commit_summary();
        assert_eq!(
            commit.action,
            DeviceExecutionPlanCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(commit.action, summary.device_execution_plan_commit_action());
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_device_execution_plan());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.failure_reports, vec![report]);
        assert_eq!(commit.failure_report_count, 1);
        assert_eq!(commit.failure_batch.total_count, 1);
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 10);
        assert_eq!(commit.total_blocker_component_count, 4);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn device_execution_adapter_summary_groups_adapter_families() {
        let summary = DeviceExecutionAdapterSummary::from_adapters(&[
            RuntimeAdapter::PortableRust,
            RuntimeAdapter::CpuSimd,
            RuntimeAdapter::Cuda,
            RuntimeAdapter::CoreMl,
            RuntimeAdapter::MultiDevice,
            RuntimeAdapter::CustomAccelerator,
        ]);

        assert_eq!(summary.adapter_count, 6);
        assert_eq!(summary.portable_count, 1);
        assert_eq!(summary.cpu_count, 1);
        assert_eq!(summary.gpu_count, 1);
        assert_eq!(summary.neural_count, 1);
        assert_eq!(summary.multi_device_count, 1);
        assert_eq!(summary.custom_count, 1);
        assert_eq!(summary.fallback_adapter_count(), 2);
        assert_eq!(summary.accelerator_adapter_count(), 4);
        assert_eq!(summary.family_member_count(), 6);
        assert!(summary.adapter_count_matches_families());
        assert_eq!(summary.adapter_family_count(), 6);
        assert!(summary.has_portable_fallback());
        assert!(summary.has_accelerator());
        assert!(summary.has_mixed_fallback_and_accelerator());
        assert!(!summary.is_accelerator_only());
        assert!(!summary.is_fallback_only());
        assert!(!summary.is_empty());
        assert_eq!(summary.adapter_family_signal_component_count(), 4);
        assert!(summary.has_adapter_family_signals());
        assert_eq!(summary.adapter_family_problem_component_count(), 0);
        assert!(!summary.has_adapter_family_problem_components());
        assert!(summary.adapter_family_accounting_is_consistent());
        assert!(summary.adapter_family_shape_is_clean());
        assert!(summary.can_use_adapter_family());
        assert_eq!(summary.adapter_family_commit_signal_component_count(), 4);
        assert!(summary.has_adapter_family_commit_signals());
        assert_eq!(summary.adapter_family_commit_blocker_component_count(), 0);
        assert!(!summary.has_adapter_family_commit_blockers());
        assert!(summary.adapter_family_commit_accounting_is_consistent());
        assert!(summary.adapter_family_commit_is_clean());
        assert!(summary.can_commit_device_execution_adapters());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(summary.adapter_family_commit_problem_component_count(), 0);
        assert!(!summary.has_adapter_family_commit_problem_components());
        assert_eq!(summary.failure_report(), None);
        assert_eq!(summary.failure_reports(), Vec::new());
        assert_eq!(summary.failure_report_count(), 0);
        assert!(!summary.has_failure_reports());
        assert_eq!(summary.failure_batch_summary().total_count, 0);
        assert!(!summary.can_format_runtime_failures());
        assert_eq!(summary.primary_failure_report(), None);
        assert_eq!(summary.primary_failure_summary(), None);
        assert_eq!(
            summary.device_execution_adapter_commit_action(),
            DeviceExecutionAdapterCommitAction::CommitDeviceExecutionAdapters
        );
        let commit = summary.commit_summary();
        assert_eq!(
            commit.action,
            DeviceExecutionAdapterCommitAction::CommitDeviceExecutionAdapters
        );
        assert_eq!(
            commit.action,
            summary.device_execution_adapter_commit_action()
        );
        assert!(commit.action_can_commit());
        assert!(!commit.action_should_return_failure());
        assert!(commit.can_commit_device_execution_adapters());
        assert!(!commit.should_return_runtime_failure());
        assert!(commit.failure_reports.is_empty());
        assert_eq!(commit.failure_report_count, 0);
        assert_eq!(commit.total_signal_component_count, 4);
        assert_eq!(commit.total_blocker_component_count, 0);
        assert!(commit.component_accounting_consistent);
        assert!(!commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());

        let empty = DeviceExecutionAdapterSummary::from_adapters(&[]);
        assert!(empty.is_empty());
        assert_eq!(empty.fallback_adapter_count(), 0);
        assert_eq!(empty.accelerator_adapter_count(), 0);
        assert_eq!(empty.family_member_count(), 0);
        assert!(empty.adapter_count_matches_families());
        assert_eq!(empty.adapter_family_count(), 0);
        assert!(!empty.has_portable_fallback());
        assert!(!empty.has_accelerator());
        assert_eq!(empty.adapter_family_signal_component_count(), 0);
        assert!(!empty.has_adapter_family_signals());
        assert_eq!(empty.adapter_family_problem_component_count(), 0);
        assert!(empty.adapter_family_accounting_is_consistent());
        assert!(empty.adapter_family_shape_is_clean());
        assert!(!empty.can_use_adapter_family());
        assert_eq!(empty.adapter_family_commit_signal_component_count(), 0);
        assert!(!empty.has_adapter_family_commit_signals());
        assert_eq!(empty.adapter_family_commit_blocker_component_count(), 1);
        assert!(empty.has_adapter_family_commit_blockers());
        assert!(empty.adapter_family_commit_accounting_is_consistent());
        assert!(!empty.adapter_family_commit_is_clean());
        assert!(!empty.can_commit_device_execution_adapters());
        assert_eq!(empty.component_accounting_drift_count(), 0);
        assert_eq!(empty.adapter_family_commit_problem_component_count(), 1);
        assert!(empty.has_adapter_family_commit_problem_components());
        let empty_report = empty.failure_report().expect("empty adapter failure");
        assert_eq!(empty_report.kind, RuntimeFailureKind::ContractViolation);
        assert!(empty_report.message.contains("components=1"));
        assert_eq!(empty.failure_reports(), vec![empty_report.clone()]);
        assert!(empty.can_format_runtime_failures());
        assert_eq!(
            empty.device_execution_adapter_commit_action(),
            DeviceExecutionAdapterCommitAction::ReturnRuntimeFailure
        );
        let empty_commit = empty.commit_summary();
        assert_eq!(
            empty_commit.action,
            DeviceExecutionAdapterCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            empty_commit.action,
            empty.device_execution_adapter_commit_action()
        );
        assert!(!empty_commit.action_can_commit());
        assert!(empty_commit.action_should_return_failure());
        assert!(!empty_commit.can_commit_device_execution_adapters());
        assert!(empty_commit.should_return_runtime_failure());
        assert_eq!(empty_commit.failure_reports, vec![empty_report]);
        assert_eq!(empty_commit.failure_report_count, 1);
        assert_eq!(empty_commit.failure_batch.contract_violation_count, 1);
        assert_eq!(empty_commit.total_signal_component_count, 0);
        assert_eq!(empty_commit.total_blocker_component_count, 1);
        assert!(empty_commit.component_accounting_consistent);
        assert!(empty_commit.has_primary_failure_summary());
        assert!(empty_commit.failure_batch_shape_is_clean());
        assert!(empty_commit.commit_decision_accounting_is_consistent());

        let accelerator_only = DeviceExecutionAdapterSummary::from_adapters(&[
            RuntimeAdapter::Cuda,
            RuntimeAdapter::CoreMl,
        ]);
        assert!(accelerator_only.is_accelerator_only());
        assert!(!accelerator_only.is_fallback_only());
        assert!(!accelerator_only.has_mixed_fallback_and_accelerator());
        assert_eq!(accelerator_only.adapter_family_signal_component_count(), 3);
        assert!(accelerator_only.adapter_family_shape_is_clean());
        assert!(accelerator_only.can_use_adapter_family());

        let fallback_only = DeviceExecutionAdapterSummary::from_adapters(&[
            RuntimeAdapter::PortableRust,
            RuntimeAdapter::CpuSimd,
        ]);
        assert!(fallback_only.is_fallback_only());
        assert!(!fallback_only.is_accelerator_only());
        assert!(!fallback_only.has_mixed_fallback_and_accelerator());
        assert_eq!(fallback_only.adapter_family_signal_component_count(), 3);
        assert!(fallback_only.adapter_family_shape_is_clean());
        assert!(fallback_only.can_use_adapter_family());
    }

    #[test]
    fn device_execution_adapter_summary_counts_public_family_drift() {
        let summary = DeviceExecutionAdapterSummary {
            adapter_count: 2,
            portable_count: 1,
            cpu_count: 1,
            gpu_count: 1,
            neural_count: 0,
            multi_device_count: 0,
            custom_count: 0,
        };

        assert_eq!(summary.fallback_adapter_count(), 2);
        assert_eq!(summary.accelerator_adapter_count(), 1);
        assert_eq!(summary.family_member_count(), 3);
        assert!(!summary.adapter_count_matches_families());
        assert_eq!(summary.adapter_family_count(), 3);
        assert!(summary.has_portable_fallback());
        assert!(summary.has_accelerator());
        assert!(summary.has_mixed_fallback_and_accelerator());
        assert_eq!(summary.adapter_family_signal_component_count(), 4);
        assert!(summary.has_adapter_family_signals());
        assert_eq!(summary.adapter_family_problem_component_count(), 1);
        assert!(summary.has_adapter_family_problem_components());
        assert!(summary.adapter_family_accounting_is_consistent());
        assert!(!summary.adapter_family_shape_is_clean());
        assert!(!summary.can_use_adapter_family());
        assert_eq!(summary.adapter_family_commit_signal_component_count(), 4);
        assert!(summary.has_adapter_family_commit_signals());
        assert_eq!(summary.adapter_family_commit_blocker_component_count(), 1);
        assert!(summary.has_adapter_family_commit_blockers());
        assert!(summary.adapter_family_commit_accounting_is_consistent());
        assert!(!summary.adapter_family_commit_is_clean());
        assert!(!summary.can_commit_device_execution_adapters());
        assert_eq!(summary.component_accounting_drift_count(), 0);
        assert_eq!(summary.adapter_family_commit_problem_component_count(), 1);
        assert!(summary.has_adapter_family_commit_problem_components());
        let report = summary
            .failure_report()
            .expect("adapter family failure report");
        assert_eq!(report.kind, RuntimeFailureKind::ContractViolation);
        assert!(report.message.contains("components=1"));
        assert_eq!(summary.failure_reports(), vec![report.clone()]);
        assert_eq!(summary.failure_report_count(), 1);
        assert!(summary.has_failure_reports());
        assert_eq!(summary.failure_batch_summary().contract_violation_count, 1);
        assert!(summary.can_format_runtime_failures());
        assert_eq!(summary.primary_failure_report(), Some(report.clone()));
        assert_eq!(
            summary
                .primary_failure_summary()
                .map(|failure| failure.kind),
            Some(RuntimeFailureKind::ContractViolation)
        );
        assert_eq!(
            summary.device_execution_adapter_commit_action(),
            DeviceExecutionAdapterCommitAction::ReturnRuntimeFailure
        );
        let commit = summary.commit_summary();
        assert_eq!(
            commit.action,
            DeviceExecutionAdapterCommitAction::ReturnRuntimeFailure
        );
        assert_eq!(
            commit.action,
            summary.device_execution_adapter_commit_action()
        );
        assert!(!commit.action_can_commit());
        assert!(commit.action_should_return_failure());
        assert!(!commit.can_commit_device_execution_adapters());
        assert!(commit.should_return_runtime_failure());
        assert_eq!(commit.failure_reports, vec![report]);
        assert_eq!(commit.failure_report_count, 1);
        assert_eq!(commit.failure_batch.contract_violation_count, 1);
        assert!(commit.can_format_runtime_failures);
        assert_eq!(commit.total_signal_component_count, 4);
        assert_eq!(commit.total_blocker_component_count, 1);
        assert!(commit.component_accounting_consistent);
        assert!(commit.has_primary_failure_summary());
        assert!(commit.failure_batch_shape_is_clean());
        assert!(commit.commit_decision_accounting_is_consistent());
    }

    #[test]
    fn device_profile_descriptors_summarize_supported_profiles() {
        assert_eq!(DeviceClass::supported_profiles().len(), 13);
        assert_eq!(DeviceClass::explicit_profiles().len(), 12);
        assert!(DeviceClass::supported_profiles().contains(&DeviceClass::Auto));
        assert!(!DeviceClass::explicit_profiles().contains(&DeviceClass::Auto));

        for device in DeviceClass::supported_profiles() {
            let descriptor = device.descriptor();
            let summary = descriptor.descriptor_summary();

            assert_eq!(descriptor.device, *device);
            assert_eq!(descriptor.tier, device.tier());
            assert!(!descriptor.scope.is_empty());
            assert!(!descriptor.aliases.is_empty());
            assert!(!descriptor.aliases_csv().is_empty());
            assert_eq!(summary.device, *device);
            assert_eq!(summary.tier, device.tier());
            assert_eq!(summary.scope_len, descriptor.scope.len());
            assert_eq!(summary.alias_count, descriptor.aliases.len());
            assert_eq!(summary.is_auto_profile, *device == DeviceClass::Auto);
            assert!(summary.has_scope());
            assert!(summary.has_aliases());
            assert!(summary.tier_matches_device());

            for alias in descriptor.aliases {
                assert_eq!(
                    alias.parse::<DeviceClass>(),
                    Ok(*device),
                    "alias {alias} should parse as {}",
                    device.as_str()
                );
            }
        }
    }

    #[test]
    fn device_aliases_parse_to_core_classes() {
        assert_eq!("cpu-only".parse::<DeviceClass>(), Ok(DeviceClass::CpuOnly));
        assert_eq!("dgpu".parse::<DeviceClass>(), Ok(DeviceClass::DiscreteGpu));
        assert_eq!("wasm".parse::<DeviceClass>(), Ok(DeviceClass::BrowserWasm));
        assert_eq!(
            "multi_gpu".parse::<DeviceClass>(),
            Ok(DeviceClass::MultiGpu)
        );
        assert!("quantum".parse::<DeviceClass>().is_err());
    }

    #[test]
    fn execution_aliases_parse_to_core_lanes_and_memory_modes() {
        assert_eq!("gpu".parse::<ComputeLane>(), Ok(ComputeLane::DiscreteGpu));
        assert_eq!("cpu".parse::<ComputeLane>(), Ok(ComputeLane::CpuVector));
        assert_eq!(
            "disk_backed_streaming".parse::<ComputeLane>(),
            Ok(ComputeLane::DiskBackedStreaming)
        );
        assert!("warp".parse::<ComputeLane>().is_err());

        assert_eq!(
            "gpu-resident".parse::<DeviceMemoryMode>(),
            Ok(DeviceMemoryMode::GpuResident)
        );
        assert_eq!(
            "tiered".parse::<DeviceMemoryMode>(),
            Ok(DeviceMemoryMode::TieredDisk)
        );
        assert!("hologram".parse::<DeviceMemoryMode>().is_err());
    }
}
