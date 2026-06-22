use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};

use super::super::probe::HardwareSnapshot;
use super::budget::{device_budget_scale, latency_budget, scaled_tokens};
use super::execution::device_execution_plan;
use super::hierarchy::adapt_hierarchy;
use super::memory_governance::{MemoryGovernancePlan, memory_governance_plan};
use super::model::{DeviceExecutionPlan, HardwarePlan};
use super::notes::notes;
use super::runtime_budget::{RuntimeBudgetInput, RuntimeBudgetReport, runtime_budget_plan};

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
        self.plan_with_runtime_budget(
            snapshot,
            profile,
            prompt_tokens,
            base_hierarchy,
            RuntimeBudgetInput::fixture(prompt_tokens),
        )
    }

    pub fn plan_with_runtime_budget(
        &self,
        snapshot: HardwareSnapshot,
        profile: TaskProfile,
        prompt_tokens: usize,
        base_hierarchy: HierarchyWeights,
        runtime_budget_input: RuntimeBudgetInput,
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
        let runtime_budget = self.runtime_budget_plan(snapshot, &execution, runtime_budget_input);
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
            runtime_budget,
            notes,
        }
    }

    pub fn runtime_budget_plan(
        &self,
        snapshot: HardwareSnapshot,
        execution: &DeviceExecutionPlan,
        input: RuntimeBudgetInput,
    ) -> RuntimeBudgetReport {
        runtime_budget_plan(snapshot, execution, input)
    }

    pub fn memory_governance_plan(
        &self,
        snapshot: HardwareSnapshot,
        retention_policy: MemoryRetentionPolicy,
        compaction_policy: MemoryCompactionPolicy,
    ) -> MemoryGovernancePlan {
        memory_governance_plan(snapshot, retention_policy, compaction_policy)
    }
}
