use crate::engine::InferenceOutcome;
use crate::hardware::DeviceClass;

use super::{BenchmarkCase, explicit_device_count, push_unique_device};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkEmbeddingEvidence {
    pub cases: usize,
    pub runtime_cases: usize,
    pub fallback_cases: usize,
    pub runtime_calls: usize,
    pub fallback_calls: usize,
    pub failures: Vec<String>,
    pub(super) runtime_devices: Vec<DeviceClass>,
}

impl BenchmarkEmbeddingEvidence {
    pub(super) fn record(&mut self, case: &BenchmarkCase, outcome: &InferenceOutcome) {
        let diagnostics = &outcome.embedding_diagnostics;
        let device = outcome.hardware_plan.device;
        self.cases += 1;

        if diagnostics.query.dimensions == 0 {
            self.failures.push(format!(
                "{}:{} embedding query dimensions are missing",
                device.as_str(),
                case.name
            ));
        }

        let expected_calls = diagnostics.total_calls();
        let observed_calls = diagnostics
            .runtime_calls
            .saturating_add(diagnostics.fallback_calls);
        if observed_calls != expected_calls {
            self.failures.push(format!(
                "{}:{} embedding calls {} do not match expected {}",
                device.as_str(),
                case.name,
                observed_calls,
                expected_calls
            ));
        }

        if diagnostics.runtime_embedding_available() {
            self.runtime_cases += 1;
            push_unique_device(&mut self.runtime_devices, device);
        }
        if diagnostics.fallback_embedding_used() {
            self.fallback_cases += 1;
        }
        self.runtime_calls += diagnostics.runtime_calls;
        self.fallback_calls += diagnostics.fallback_calls;
    }

    pub fn runtime_device_profiles(&self) -> usize {
        explicit_device_count(&self.runtime_devices)
    }

    pub fn runtime_devices_csv(&self) -> String {
        if self.runtime_devices.is_empty() {
            "none".to_owned()
        } else {
            self.runtime_devices
                .iter()
                .map(|device| device.as_str())
                .collect::<Vec<_>>()
                .join("+")
        }
    }
}
