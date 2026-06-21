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
    }

    pub fn device_profiles(&self) -> usize {
        explicit_device_count(&self.devices)
    }

    pub fn saved_token_device_profiles(&self) -> usize {
        explicit_device_count(&self.saved_token_devices)
    }
}
