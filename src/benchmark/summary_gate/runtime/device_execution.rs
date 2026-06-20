use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_runtime_device_execution_cases) = gate.min_runtime_device_execution_cases {
        let runtime_device_execution_matched_cases =
            summary.runtime_device_execution_matched_cases();
        if runtime_device_execution_matched_cases < min_runtime_device_execution_cases {
            failures.push(format!(
                "runtime_device_execution_matched_cases {} below minimum {}",
                runtime_device_execution_matched_cases, min_runtime_device_execution_cases
            ));
        }
    }

    if let Some(min_runtime_device_execution_device_profiles) =
        gate.min_runtime_device_execution_device_profiles
    {
        let runtime_device_execution_device_profiles =
            summary.runtime_device_execution_device_profiles();
        if runtime_device_execution_device_profiles < min_runtime_device_execution_device_profiles {
            failures.push(format!(
                "runtime_device_execution_device_profiles {} below minimum {}",
                runtime_device_execution_device_profiles,
                min_runtime_device_execution_device_profiles
            ));
        }
    }

    if let Some(min_runtime_kv_precision_device_profiles) =
        gate.min_runtime_kv_precision_device_profiles
    {
        let runtime_kv_precision_device_profiles = summary.runtime_kv_precision_device_profiles();
        if runtime_kv_precision_device_profiles < min_runtime_kv_precision_device_profiles {
            failures.push(format!(
                "runtime_kv_precision_device_profiles {} below minimum {}",
                runtime_kv_precision_device_profiles, min_runtime_kv_precision_device_profiles
            ));
        }
    }

    if let Some(max_runtime_device_execution_violations) =
        gate.max_runtime_device_execution_violations
    {
        let runtime_device_execution_violations =
            summary.total_runtime_device_execution_violations();
        if runtime_device_execution_violations > max_runtime_device_execution_violations {
            failures.push(format!(
                "runtime_device_execution_violations {} above maximum {}: {}",
                runtime_device_execution_violations,
                max_runtime_device_execution_violations,
                summary
                    .runtime_device_execution_evidence
                    .failures
                    .join("; ")
            ));
        }
    }
}
