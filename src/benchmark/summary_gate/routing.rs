use super::super::BenchmarkGate;
use super::super::summary::BenchmarkSummary;
use super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(max_adaptive_routing_failures) = gate.max_adaptive_routing_failures {
        let adaptive_routing_failures = summary.total_adaptive_routing_failures();
        if adaptive_routing_failures > max_adaptive_routing_failures {
            failures.push(format!(
                "adaptive_routing_failures {} above maximum {}: {}",
                adaptive_routing_failures,
                max_adaptive_routing_failures,
                summary.routing_evidence.failures.join("; ")
            ));
        }
    }

    if let Some(min_adaptive_routing_cases) = gate.min_adaptive_routing_cases {
        let observed = summary.adaptive_routing_cases();
        if observed < min_adaptive_routing_cases {
            failures.push(format!(
                "adaptive_routing_cases {} below minimum {}",
                observed, min_adaptive_routing_cases
            ));
        }
    }

    if let Some(min_adaptive_routing_device_profiles) = gate.min_adaptive_routing_device_profiles {
        let observed = summary.adaptive_routing_device_profiles();
        if observed < min_adaptive_routing_device_profiles {
            failures.push(format!(
                "adaptive_routing_device_profiles {} below minimum {}",
                observed, min_adaptive_routing_device_profiles
            ));
        }
    }

    if let Some(min_adaptive_routing_saved_tokens) = gate.min_adaptive_routing_saved_tokens {
        let observed = summary.total_adaptive_routing_saved_tokens();
        if observed < min_adaptive_routing_saved_tokens {
            failures.push(format!(
                "adaptive_routing_saved_tokens {} below minimum {}",
                observed, min_adaptive_routing_saved_tokens
            ));
        }
    }

    if let Some(min_adaptive_routing_saved_token_device_profiles) =
        gate.min_adaptive_routing_saved_token_device_profiles
    {
        let observed = summary.adaptive_routing_saved_token_device_profiles();
        if observed < min_adaptive_routing_saved_token_device_profiles {
            failures.push(format!(
                "adaptive_routing_saved_token_device_profiles {} below minimum {}",
                observed, min_adaptive_routing_saved_token_device_profiles
            ));
        }
    }
}
