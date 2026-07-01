use crate::drift::DriftSeverity;
use crate::hardware::{DeviceClass, RuntimeAdapterHint};

use super::display::{option_f32_display, option_str_display};

#[derive(Debug, Clone)]
pub struct PersistentRoundtripInput {
    pub first_stored_memory: bool,
    pub first_runtime_kv_stored: usize,
    pub first_runtime_kv_namespace_preserved: bool,
    pub second_used_memories: usize,
    pub second_used_runtime_kv_memory: bool,
    pub second_used_experiences: usize,
    pub second_imported_runtime_kv_blocks: usize,
    pub second_imported_runtime_kv_from_namespace: bool,
    pub second_runtime_adapter_observations: usize,
    pub second_runtime_adapter_best_score: Option<f32>,
    pub second_runtime_adapter_best_adapter: Option<String>,
    pub second_runtime_selected_adapter: Option<String>,
    pub second_compute_budget_saved_tokens: usize,
    pub second_compute_budget_avoided_tokens: usize,
    pub second_compute_budget_kv_lookups_skipped: usize,
    pub second_quality: f32,
    pub first_drift_severity: DriftSeverity,
    pub second_drift_severity: DriftSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentRoundtripReport {
    pub passed: bool,
    pub first_stored_memory: bool,
    pub first_runtime_kv_stored: usize,
    pub first_runtime_kv_namespace_preserved: bool,
    pub second_used_memories: usize,
    pub second_used_runtime_kv_memory: bool,
    pub second_used_experiences: usize,
    pub second_imported_runtime_kv_blocks: usize,
    pub second_imported_runtime_kv_from_namespace: bool,
    pub second_runtime_adapter_observations: usize,
    pub second_runtime_adapter_best_score: Option<f32>,
    pub second_runtime_adapter_best_adapter: Option<String>,
    pub second_runtime_selected_adapter: Option<String>,
    pub second_compute_budget_saved_tokens: usize,
    pub second_compute_budget_avoided_tokens: usize,
    pub second_compute_budget_kv_lookups_skipped: usize,
    pub second_quality: f32,
    pub first_drift_severity: DriftSeverity,
    pub second_drift_severity: DriftSeverity,
    pub failures: Vec<String>,
}

impl PersistentRoundtripReport {
    pub fn evaluate(input: PersistentRoundtripInput) -> Self {
        let mut failures = Vec::new();
        let second_runtime_adapter_best_adapter = input
            .second_runtime_adapter_best_adapter
            .as_deref()
            .and_then(RuntimeAdapterHint::canonical_name)
            .map(str::to_owned);
        let second_runtime_selected_adapter = input
            .second_runtime_selected_adapter
            .as_deref()
            .and_then(RuntimeAdapterHint::canonical_name)
            .map(str::to_owned);

        if !input.first_stored_memory {
            failures.push("first run did not store durable memory".to_owned());
        }
        if input.first_runtime_kv_stored == 0 {
            failures.push("first run did not store runtime KV memory".to_owned());
        }
        if !input.first_runtime_kv_namespace_preserved {
            failures.push("first run stored runtime KV without runtime_kv namespace".to_owned());
        }
        if input.second_used_memories == 0 {
            failures.push("second run did not retrieve persisted memory".to_owned());
        }
        if !input.second_used_runtime_kv_memory {
            failures.push("second run did not retrieve persisted runtime KV memory".to_owned());
        }
        if input.second_used_experiences == 0 {
            failures.push("second run did not retrieve persisted experience".to_owned());
        }
        if input.second_imported_runtime_kv_blocks == 0 {
            failures.push("second run did not import persisted runtime KV".to_owned());
        }
        if !input.second_imported_runtime_kv_from_namespace {
            failures.push(
                "second run did not import KV reconstructed from persisted runtime_kv namespace"
                    .to_owned(),
            );
        }
        if input.second_runtime_adapter_observations == 0 {
            failures.push(
                "second run did not derive runtime adapter observations from persisted experience"
                    .to_owned(),
            );
        }
        if input
            .second_runtime_adapter_best_score
            .filter(|score| score.is_finite() && *score > 0.0)
            .is_none()
        {
            failures.push(
                "second run did not expose a positive runtime adapter observation score".to_owned(),
            );
        }
        match (
            second_runtime_adapter_best_adapter.as_deref(),
            second_runtime_selected_adapter.as_deref(),
        ) {
            (Some(best_adapter), Some(selected_adapter)) if best_adapter == selected_adapter => {}
            (None, _) => failures.push(
                "second run did not expose a trusted best runtime adapter observation".to_owned(),
            ),
            (_, None) => {
                failures.push("second run did not select a trusted runtime adapter".to_owned())
            }
            (Some(best_adapter), Some(selected_adapter)) => failures.push(format!(
                "second run selected adapter {selected_adapter} but best persisted observation was {best_adapter}"
            )),
        }
        if input.second_compute_budget_avoided_tokens == 0 {
            failures.push("second run did not report compute budget avoided tokens".to_owned());
        }
        if input.second_quality < 0.50 {
            failures.push(format!(
                "second_quality {:.3} below minimum 0.500",
                input.second_quality
            ));
        }
        if input.first_drift_severity == DriftSeverity::Rollback {
            failures.push("first run triggered drift rollback".to_owned());
        }
        if matches!(
            input.second_drift_severity,
            DriftSeverity::Block | DriftSeverity::Rollback
        ) {
            failures.push(format!(
                "second run drift severity was {}",
                input.second_drift_severity.as_str()
            ));
        }

        Self {
            passed: failures.is_empty(),
            first_stored_memory: input.first_stored_memory,
            first_runtime_kv_stored: input.first_runtime_kv_stored,
            first_runtime_kv_namespace_preserved: input.first_runtime_kv_namespace_preserved,
            second_used_memories: input.second_used_memories,
            second_used_runtime_kv_memory: input.second_used_runtime_kv_memory,
            second_used_experiences: input.second_used_experiences,
            second_imported_runtime_kv_blocks: input.second_imported_runtime_kv_blocks,
            second_imported_runtime_kv_from_namespace: input
                .second_imported_runtime_kv_from_namespace,
            second_runtime_adapter_observations: input.second_runtime_adapter_observations,
            second_runtime_adapter_best_score: input.second_runtime_adapter_best_score,
            second_runtime_adapter_best_adapter,
            second_runtime_selected_adapter,
            second_compute_budget_saved_tokens: input.second_compute_budget_saved_tokens,
            second_compute_budget_avoided_tokens: input.second_compute_budget_avoided_tokens,
            second_compute_budget_kv_lookups_skipped: input
                .second_compute_budget_kv_lookups_skipped,
            second_quality: input.second_quality,
            first_drift_severity: input.first_drift_severity,
            second_drift_severity: input.second_drift_severity,
            failures,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "persistent_roundtrip: passed={} first_stored_memory={} first_runtime_kv_stored={} first_runtime_kv_namespace_preserved={} second_used_memories={} second_used_runtime_kv_memory={} second_used_experiences={} second_imported_runtime_kv_blocks={} second_imported_runtime_kv_from_namespace={} second_runtime_adapter_observations={} second_runtime_adapter_best_score={} second_runtime_adapter_best_adapter={} second_runtime_selected_adapter={} second_compute_budget_saved_tokens={} second_compute_budget_avoided_tokens={} second_compute_budget_kv_lookups_skipped={} second_quality={:.3} first_drift={} second_drift={} failures={}",
            self.passed,
            self.first_stored_memory,
            self.first_runtime_kv_stored,
            self.first_runtime_kv_namespace_preserved,
            self.second_used_memories,
            self.second_used_runtime_kv_memory,
            self.second_used_experiences,
            self.second_imported_runtime_kv_blocks,
            self.second_imported_runtime_kv_from_namespace,
            self.second_runtime_adapter_observations,
            option_f32_display(self.second_runtime_adapter_best_score),
            option_str_display(self.second_runtime_adapter_best_adapter.as_deref()),
            option_str_display(self.second_runtime_selected_adapter.as_deref()),
            self.second_compute_budget_saved_tokens,
            self.second_compute_budget_avoided_tokens,
            self.second_compute_budget_kv_lookups_skipped,
            self.second_quality,
            self.first_drift_severity.as_str(),
            self.second_drift_severity.as_str(),
            self.failures.len()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentRoundtripDeviceReport {
    pub device: DeviceClass,
    pub report: PersistentRoundtripReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentRoundtripMatrixReport {
    pub passed: bool,
    pub device_reports: Vec<PersistentRoundtripDeviceReport>,
    pub failures: Vec<String>,
}

impl PersistentRoundtripMatrixReport {
    pub fn evaluate(device_reports: Vec<PersistentRoundtripDeviceReport>) -> Self {
        let mut failures = Vec::new();

        if device_reports.is_empty() {
            failures.push("no persistent roundtrip device reports were recorded".to_owned());
        }

        let missing = missing_persistent_roundtrip_devices(&device_reports);
        if !missing.is_empty() {
            let missing_devices = missing
                .iter()
                .map(|device| device.as_str())
                .collect::<Vec<_>>()
                .join("+");
            failures.push(format!(
                "persistent_roundtrip_devices {} below expected {} missing={}",
                explicit_persistent_roundtrip_devices(&device_reports),
                DeviceClass::explicit_profiles().len(),
                missing_devices
            ));
        }

        for device_report in &device_reports {
            if !device_report.report.passed {
                failures.push(format!(
                    "device {} persistent roundtrip failed with {} failures",
                    device_report.device.as_str(),
                    device_report.report.failures.len()
                ));
            }
        }

        Self {
            passed: failures.is_empty(),
            device_reports,
            failures,
        }
    }

    pub fn covered_devices(&self) -> usize {
        explicit_persistent_roundtrip_devices(&self.device_reports)
    }

    pub fn missing_devices(&self) -> Vec<DeviceClass> {
        missing_persistent_roundtrip_devices(&self.device_reports)
    }

    pub fn failed_devices(&self) -> Vec<DeviceClass> {
        self.device_reports
            .iter()
            .filter(|device_report| !device_report.report.passed)
            .map(|device_report| device_report.device)
            .collect()
    }

    pub fn second_compute_budget_saved_tokens(&self) -> usize {
        self.device_reports
            .iter()
            .map(|device_report| device_report.report.second_compute_budget_saved_tokens)
            .sum()
    }

    pub fn second_compute_budget_avoided_tokens(&self) -> usize {
        self.device_reports
            .iter()
            .map(|device_report| device_report.report.second_compute_budget_avoided_tokens)
            .sum()
    }

    pub fn second_compute_budget_kv_lookups_skipped(&self) -> usize {
        self.device_reports
            .iter()
            .map(|device_report| {
                device_report
                    .report
                    .second_compute_budget_kv_lookups_skipped
            })
            .sum()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "persistent_roundtrip_matrix: passed={} devices={} expected_devices={} failed_devices={} second_compute_budget_saved_tokens={} second_compute_budget_avoided_tokens={} second_compute_budget_kv_lookups_skipped={} failures={}",
            self.passed,
            self.covered_devices(),
            DeviceClass::explicit_profiles().len(),
            self.failed_devices().len(),
            self.second_compute_budget_saved_tokens(),
            self.second_compute_budget_avoided_tokens(),
            self.second_compute_budget_kv_lookups_skipped(),
            self.failures.len()
        )
    }
}

fn explicit_persistent_roundtrip_devices(
    device_reports: &[PersistentRoundtripDeviceReport],
) -> usize {
    DeviceClass::explicit_profiles()
        .iter()
        .filter(|device| {
            device_reports
                .iter()
                .any(|device_report| device_report.device == **device)
        })
        .count()
}

fn missing_persistent_roundtrip_devices(
    device_reports: &[PersistentRoundtripDeviceReport],
) -> Vec<DeviceClass> {
    DeviceClass::explicit_profiles()
        .iter()
        .copied()
        .filter(|device| {
            !device_reports
                .iter()
                .any(|device_report| device_report.device == *device)
        })
        .collect()
}
