use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_runtime_embedding_cases) = gate.min_runtime_embedding_cases {
        let runtime_embedding_cases = summary.runtime_embedding_cases();
        if runtime_embedding_cases < min_runtime_embedding_cases {
            failures.push(format!(
                "runtime_embedding_cases {} below minimum {}",
                runtime_embedding_cases, min_runtime_embedding_cases
            ));
        }
    }

    if let Some(min_runtime_embedding_device_profiles) = gate.min_runtime_embedding_device_profiles
    {
        let runtime_embedding_device_profiles = summary.runtime_embedding_device_profiles();
        if runtime_embedding_device_profiles < min_runtime_embedding_device_profiles {
            failures.push(format!(
                "runtime_embedding_device_profiles {} below minimum {}",
                runtime_embedding_device_profiles, min_runtime_embedding_device_profiles
            ));
        }
    }

    if let Some(max_embedding_fallback_cases) = gate.max_embedding_fallback_cases {
        let embedding_fallback_cases = summary.embedding_fallback_cases();
        if embedding_fallback_cases > max_embedding_fallback_cases {
            failures.push(format!(
                "embedding_fallback_cases {} above maximum {}",
                embedding_fallback_cases, max_embedding_fallback_cases
            ));
        }
    }

    if let Some(max_embedding_evidence_failures) = gate.max_embedding_evidence_failures {
        let embedding_evidence_failures = summary.total_embedding_evidence_failures();
        if embedding_evidence_failures > max_embedding_evidence_failures {
            failures.push(format!(
                "embedding_evidence_failures {} above maximum {}: {}",
                embedding_evidence_failures,
                max_embedding_evidence_failures,
                summary.embedding_evidence.failures.join("; ")
            ));
        }
    }
}
