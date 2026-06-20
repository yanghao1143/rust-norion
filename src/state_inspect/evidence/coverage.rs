use crate::hardware::DeviceClass;

use super::super::StateInspectionDeviceGateReport;

pub(in crate::state_inspect) fn explicit_state_inspection_evidence_devices<F>(
    device_reports: &[StateInspectionDeviceGateReport],
    has_evidence: F,
) -> usize
where
    F: Fn(&StateInspectionDeviceGateReport) -> bool,
{
    DeviceClass::explicit_profiles()
        .iter()
        .filter(|device| {
            device_reports.iter().any(|device_report| {
                device_report.device == **device && has_evidence(device_report)
            })
        })
        .count()
}

pub(in crate::state_inspect) fn require_min_device_profiles(
    failures: &mut Vec<String>,
    name: &str,
    actual: usize,
    required: Option<usize>,
) {
    if let Some(required) = required
        && actual < required
    {
        failures.push(format!("{name} {actual} below required {required}"));
    }
}

pub(in crate::state_inspect) fn explicit_state_inspection_devices(
    device_reports: &[StateInspectionDeviceGateReport],
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

pub(in crate::state_inspect) fn missing_state_inspection_devices(
    device_reports: &[StateInspectionDeviceGateReport],
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
