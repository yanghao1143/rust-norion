use crate::hardware::DeviceClass;

use super::report::ProductionKernelConformanceReport;

#[derive(Debug, Clone, PartialEq)]
pub struct ProductionKernelConformanceDeviceReport {
    pub device: DeviceClass,
    pub report: ProductionKernelConformanceReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProductionKernelConformanceMatrixReport {
    pub passed: bool,
    pub device_reports: Vec<ProductionKernelConformanceDeviceReport>,
    pub failures: Vec<String>,
}

impl ProductionKernelConformanceMatrixReport {
    pub fn evaluate(device_reports: Vec<ProductionKernelConformanceDeviceReport>) -> Self {
        let mut failures = Vec::new();

        if device_reports.is_empty() {
            failures
                .push("no production kernel conformance device reports were recorded".to_owned());
        }

        let missing = missing_production_kernel_conformance_devices(&device_reports);
        if !missing.is_empty() {
            failures.push(format!(
                "production_kernel_conformance_devices {} below expected {} missing={}",
                explicit_production_kernel_conformance_devices(&device_reports),
                DeviceClass::explicit_profiles().len(),
                missing
                    .iter()
                    .map(|device| device.as_str())
                    .collect::<Vec<_>>()
                    .join("+")
            ));
        }

        for device_report in &device_reports {
            if !device_report.report.passed {
                failures.push(format!(
                    "device {} production kernel conformance failed with {} failures",
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
        explicit_production_kernel_conformance_devices(&self.device_reports)
    }

    pub fn missing_devices(&self) -> Vec<DeviceClass> {
        missing_production_kernel_conformance_devices(&self.device_reports)
    }

    pub fn failed_devices(&self) -> Vec<DeviceClass> {
        self.device_reports
            .iter()
            .filter(|device_report| !device_report.report.passed)
            .map(|device_report| device_report.device)
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "production_kernel_conformance_matrix: passed={} devices={} expected_devices={} failed_devices={} failures={}",
            self.passed,
            self.covered_devices(),
            DeviceClass::explicit_profiles().len(),
            self.failed_devices().len(),
            self.failures.len()
        )
    }
}

fn explicit_production_kernel_conformance_devices(
    device_reports: &[ProductionKernelConformanceDeviceReport],
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

fn missing_production_kernel_conformance_devices(
    device_reports: &[ProductionKernelConformanceDeviceReport],
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
