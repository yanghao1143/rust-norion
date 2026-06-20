use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_runtime_uncertainty_cases) = gate.min_runtime_uncertainty_cases {
        let runtime_uncertainty_cases = summary.runtime_uncertainty_cases();
        if runtime_uncertainty_cases < min_runtime_uncertainty_cases {
            failures.push(format!(
                "runtime_uncertainty_cases {} below minimum {}",
                runtime_uncertainty_cases, min_runtime_uncertainty_cases
            ));
        }
    }

    if let Some(min_runtime_uncertainty_tokens) = gate.min_runtime_uncertainty_tokens {
        let runtime_uncertainty_tokens = summary.total_runtime_uncertainty_tokens();
        if runtime_uncertainty_tokens < min_runtime_uncertainty_tokens {
            failures.push(format!(
                "runtime_uncertainty_tokens {} below minimum {}",
                runtime_uncertainty_tokens, min_runtime_uncertainty_tokens
            ));
        }
    }

    if let Some(min_runtime_uncertainty_device_profiles) =
        gate.min_runtime_uncertainty_device_profiles
    {
        let runtime_uncertainty_device_profiles = summary.runtime_uncertainty_device_profiles();
        if runtime_uncertainty_device_profiles < min_runtime_uncertainty_device_profiles {
            failures.push(format!(
                "runtime_uncertainty_device_profiles {} below minimum {} devices={}",
                runtime_uncertainty_device_profiles,
                min_runtime_uncertainty_device_profiles,
                summary.runtime_uncertainty_devices_csv()
            ));
        }
    }

    if let Some(min_runtime_uncertainty_token_device_profiles) =
        gate.min_runtime_uncertainty_token_device_profiles
    {
        let runtime_uncertainty_token_device_profiles =
            summary.runtime_uncertainty_token_device_profiles();
        if runtime_uncertainty_token_device_profiles < min_runtime_uncertainty_token_device_profiles
        {
            failures.push(format!(
                "runtime_uncertainty_token_device_profiles {} below minimum {} devices={}",
                runtime_uncertainty_token_device_profiles,
                min_runtime_uncertainty_token_device_profiles,
                summary.runtime_uncertainty_token_devices_csv()
            ));
        }
    }
}
