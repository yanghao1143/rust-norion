use super::super::BenchmarkGate;
use super::super::summary::BenchmarkSummary;
use super::GateFailures;
use crate::hardware::DeviceClass;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_device_profiles) = gate.min_device_profiles {
        let device_profiles = summary.explicit_device_profiles_covered();
        if device_profiles < min_device_profiles {
            let missing = summary
                .missing_explicit_device_profiles()
                .into_iter()
                .map(DeviceClass::as_str)
                .collect::<Vec<_>>()
                .join("+");
            failures.push(format!(
                "device_profiles {} below minimum {} missing={}",
                device_profiles, min_device_profiles, missing
            ));
        }
    }

    if let Some(min_recursive_device_profiles) = gate.min_recursive_device_profiles {
        let recursive_device_profiles = summary.recursive_device_profiles_covered();
        if recursive_device_profiles < min_recursive_device_profiles {
            let missing = summary
                .missing_recursive_device_profiles()
                .into_iter()
                .map(DeviceClass::as_str)
                .collect::<Vec<_>>()
                .join("+");
            failures.push(format!(
                "recursive_device_profiles {} below minimum {} missing={}",
                recursive_device_profiles, min_recursive_device_profiles, missing
            ));
        }
    }

    if let Some(max_drift_blocks) = gate.max_drift_blocks {
        let drift_blocks = summary.drift_blocks();
        if drift_blocks > max_drift_blocks {
            failures.push(format!(
                "drift_blocks {} above maximum {}",
                drift_blocks, max_drift_blocks
            ));
        }
    }

    if let Some(max_drift_rollbacks) = gate.max_drift_rollbacks {
        let drift_rollbacks = summary.drift_rollbacks();
        if drift_rollbacks > max_drift_rollbacks {
            failures.push(format!(
                "drift_rollbacks {} above maximum {}",
                drift_rollbacks, max_drift_rollbacks
            ));
        }
    }
}
