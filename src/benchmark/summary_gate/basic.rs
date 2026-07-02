use super::super::BenchmarkGate;
use super::super::summary::BenchmarkSummary;
use super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if summary.is_empty() {
        failures.push("no benchmark cases were recorded".to_owned());
    }

    let development_evidence_surface_blocks = summary.development_evidence_surface_blocks();
    if development_evidence_surface_blocks > 0 {
        failures.push(format!(
            "development_evidence_surface_blocks {} above maximum 0",
            development_evidence_surface_blocks
        ));
    }

    let average_quality = summary.average_quality();
    if average_quality < gate.min_average_quality {
        failures.push(format!(
            "average_quality {:.3} below minimum {:.3}",
            average_quality, gate.min_average_quality
        ));
    }

    let average_reward = summary.average_reward();
    if average_reward < gate.min_average_reward {
        failures.push(format!(
            "average_reward {:.3} below minimum {:.3}",
            average_reward, gate.min_average_reward
        ));
    }

    if let Some(max_total_elapsed_ms) = gate.max_total_elapsed_ms {
        let total_elapsed_ms = summary.total_elapsed_ms();
        if total_elapsed_ms > max_total_elapsed_ms {
            failures.push(format!(
                "total_elapsed_ms {} above maximum {}",
                total_elapsed_ms, max_total_elapsed_ms
            ));
        }
    }

    if let Some(max_case_recursive_chunks) = gate.max_case_recursive_chunks {
        let max_recursive_chunks = summary.max_recursive_chunks();
        if max_recursive_chunks > max_case_recursive_chunks {
            failures.push(format!(
                "max_recursive_chunks {} above maximum {}",
                max_recursive_chunks, max_case_recursive_chunks
            ));
        }
    }

    if let Some(min_recursive_cases) = gate.min_recursive_cases {
        let recursive_cases = summary.recursive_cases();
        if recursive_cases < min_recursive_cases {
            failures.push(format!(
                "recursive_cases {} below minimum {}",
                recursive_cases, min_recursive_cases
            ));
        }
    }

    if let Some(min_recursive_runtime_calls) = gate.min_recursive_runtime_calls {
        let recursive_runtime_calls = summary.total_recursive_runtime_calls();
        if recursive_runtime_calls < min_recursive_runtime_calls {
            failures.push(format!(
                "recursive_runtime_calls {} below minimum {}",
                recursive_runtime_calls, min_recursive_runtime_calls
            ));
        }
    }
}
