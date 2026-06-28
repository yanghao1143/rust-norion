use crate::engine::InferenceOutcome;
use crate::hardware::DeviceClass;

use super::{BenchmarkCase, explicit_device_count, push_unique_device};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkRoutingEvidence {
    pub cases: usize,
    pub candidates: usize,
    pub included: usize,
    pub compressed: usize,
    pub deferred: usize,
    pub skipped: usize,
    pub input_tokens: usize,
    pub retained_tokens: usize,
    pub saved_tokens: usize,
    pub task_hierarchy_cases: usize,
    pub task_hierarchy_mutation_records: usize,
    pub task_hierarchy_route_pressure_milli: usize,
    pub task_hierarchy_compute_reduction_milli: usize,
    pub task_hierarchy_modes: Vec<String>,
    pub compute_budget_cases: usize,
    pub compute_budget_low_value_skipped: usize,
    pub compute_budget_kv_lookups_skipped: usize,
    pub compute_budget_validation_cost_tokens: usize,
    pub compute_budget_saved_tokens: usize,
    pub compute_budget_self_evolving_memory_fusion_saved_tokens: usize,
    pub compute_budget_avoided_tokens: usize,
    pub compute_budget_fanout_before: usize,
    pub compute_budget_fanout_after: usize,
    pub compute_budget_fanout_reduction: usize,
    pub failures: Vec<String>,
    pub(super) devices: Vec<DeviceClass>,
    pub(super) saved_token_devices: Vec<DeviceClass>,
}

impl BenchmarkRoutingEvidence {
    pub(super) fn record(&mut self, case: &BenchmarkCase, outcome: &InferenceOutcome) {
        self.cases = self.cases.saturating_add(1);
        let device = outcome.hardware_plan.device;
        push_unique_device(&mut self.devices, device);

        let plan = &outcome.adaptive_route_plan;
        self.candidates = self.candidates.saturating_add(plan.candidates);
        self.included = self.included.saturating_add(plan.include);
        self.compressed = self.compressed.saturating_add(plan.compress);
        self.deferred = self.deferred.saturating_add(plan.defer);
        self.skipped = self.skipped.saturating_add(plan.skip);
        self.input_tokens = self.input_tokens.saturating_add(plan.input_tokens);
        self.retained_tokens = self.retained_tokens.saturating_add(plan.retained_tokens);
        self.saved_tokens = self.saved_tokens.saturating_add(plan.saved_tokens);
        if plan.saved_tokens > 0 {
            push_unique_device(&mut self.saved_token_devices, device);
        }

        if plan.candidates == 0 {
            self.failures.push(format!(
                "{}:{} adaptive_routing must include at least one candidate",
                device.as_str(),
                case.name
            ));
        }
        if !plan.decision_count_matches() {
            self.failures.push(format!(
                "{}:{} adaptive_routing decisions do not match candidate count",
                device.as_str(),
                case.name
            ));
        }
        if !plan.token_accounting_matches() {
            self.failures.push(format!(
                "{}:{} adaptive_routing retained+saved token accounting is inconsistent",
                device.as_str(),
                case.name
            ));
        }
        if !plan.anchors_retained() {
            self.failures.push(format!(
                "{}:{} adaptive_routing skipped a required task anchor",
                device.as_str(),
                case.name
            ));
        }
        if !plan.read_only || plan.write_allowed || plan.applied {
            self.failures.push(format!(
                "{}:{} adaptive_routing must remain read-only/report-only",
                device.as_str(),
                case.name
            ));
        }
        if plan.candidates > 0 && plan.score_summaries(1).is_empty() {
            self.failures.push(format!(
                "{}:{} adaptive_routing must expose score summary evidence",
                device.as_str(),
                case.name
            ));
        }

        let task = &outcome.task_hierarchy_plan;
        self.task_hierarchy_cases = self.task_hierarchy_cases.saturating_add(1);
        self.task_hierarchy_mutation_records = self
            .task_hierarchy_mutation_records
            .saturating_add(task.mutation_count());
        self.task_hierarchy_route_pressure_milli = self
            .task_hierarchy_route_pressure_milli
            .saturating_add(milli(task.route_pressure));
        self.task_hierarchy_compute_reduction_milli = self
            .task_hierarchy_compute_reduction_milli
            .saturating_add(milli(task.compute_reduction));
        push_unique_string(&mut self.task_hierarchy_modes, task.mode.as_str());
        if task.hierarchy_depth == 0 || task.route_fanout == 0 {
            self.failures.push(format!(
                "{}:{} task_hierarchy must choose positive hierarchy_depth and route_fanout",
                device.as_str(),
                case.name
            ));
        }
        if task.selected_lanes.is_empty() || task.memory_lanes.is_empty() {
            self.failures.push(format!(
                "{}:{} task_hierarchy must select hierarchy and memory lanes",
                device.as_str(),
                case.name
            ));
        }
        if task.mutation_count() == 0 || !task.mutation_history_replayable() {
            self.failures.push(format!(
                "{}:{} task_hierarchy mutation history must be replayable and revertible",
                device.as_str(),
                case.name
            ));
        }
        if task.state_write_allowed || task.adaptive_state_write_allowed || task.ndkv_write_allowed
        {
            self.failures.push(format!(
                "{}:{} task_hierarchy mutation history must not write durable state",
                device.as_str(),
                case.name
            ));
        }

        let budget = &outcome.compute_budget_schedule;
        self.compute_budget_cases = self.compute_budget_cases.saturating_add(1);
        self.compute_budget_low_value_skipped = self
            .compute_budget_low_value_skipped
            .saturating_add(budget.low_value_skipped);
        self.compute_budget_kv_lookups_skipped = self
            .compute_budget_kv_lookups_skipped
            .saturating_add(budget.kv_lookups_skipped);
        self.compute_budget_validation_cost_tokens = self
            .compute_budget_validation_cost_tokens
            .saturating_add(budget.validation_cost_tokens);
        self.compute_budget_saved_tokens = self
            .compute_budget_saved_tokens
            .saturating_add(budget.saved_tokens);
        self.compute_budget_self_evolving_memory_fusion_saved_tokens = self
            .compute_budget_self_evolving_memory_fusion_saved_tokens
            .saturating_add(budget.self_evolving_memory_fusion_saved_tokens);
        self.compute_budget_avoided_tokens = self
            .compute_budget_avoided_tokens
            .saturating_add(budget.wasted_compute_avoided_tokens);
        self.compute_budget_fanout_before = self
            .compute_budget_fanout_before
            .saturating_add(budget.route_fanout_before);
        self.compute_budget_fanout_after = self
            .compute_budget_fanout_after
            .saturating_add(budget.route_fanout_after);
        self.compute_budget_fanout_reduction = self.compute_budget_fanout_reduction.saturating_add(
            budget
                .route_fanout_before
                .saturating_sub(budget.route_fanout_after),
        );
        if !budget.anchors_preserved() {
            self.failures.push(format!(
                "{}:{} compute_budget must preserve correctness anchors",
                device.as_str(),
                case.name
            ));
        }
        if !budget.budget_accounting_matches() {
            self.failures.push(format!(
                "{}:{} compute_budget token accounting is inconsistent",
                device.as_str(),
                case.name
            ));
        }
        if !budget.read_only || budget.write_allowed || budget.applied {
            self.failures.push(format!(
                "{}:{} compute_budget must remain read-only/report-only",
                device.as_str(),
                case.name
            ));
        }
    }

    pub fn device_profiles(&self) -> usize {
        explicit_device_count(&self.devices)
    }

    pub fn saved_token_device_profiles(&self) -> usize {
        explicit_device_count(&self.saved_token_devices)
    }

    pub fn task_hierarchy_mode_count(&self) -> usize {
        self.task_hierarchy_modes.len()
    }
}

fn milli(value: f32) -> usize {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as usize
    } else {
        0
    }
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}
