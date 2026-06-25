use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_runtime_adapter_contract_cases) = gate.min_runtime_adapter_contract_cases {
        let runtime_adapter_contract_cases = summary.runtime_adapter_contract_cases();
        if runtime_adapter_contract_cases < min_runtime_adapter_contract_cases {
            failures.push(format!(
                "runtime_adapter_contract_cases {} below minimum {}",
                runtime_adapter_contract_cases, min_runtime_adapter_contract_cases
            ));
        }
    }

    if let Some(min_runtime_adapter_kinds) = gate.min_runtime_adapter_kinds {
        let runtime_adapter_kinds = summary.runtime_adapter_kinds();
        if runtime_adapter_kinds < min_runtime_adapter_kinds {
            failures.push(format!(
                "runtime_adapter_kinds {} below minimum {}",
                runtime_adapter_kinds, min_runtime_adapter_kinds
            ));
        }
    }

    if let Some(min_runtime_adapter_cache_modes) = gate.min_runtime_adapter_cache_modes {
        let runtime_adapter_cache_modes = summary.runtime_adapter_cache_modes();
        if runtime_adapter_cache_modes < min_runtime_adapter_cache_modes {
            failures.push(format!(
                "runtime_adapter_cache_modes {} below minimum {} modes={}",
                runtime_adapter_cache_modes,
                min_runtime_adapter_cache_modes,
                summary.runtime_adapter_cache_modes_csv()
            ));
        }
    }

    if let Some(min_runtime_adapter_stream_trace_cases) =
        gate.min_runtime_adapter_stream_trace_cases
    {
        let runtime_adapter_stream_trace_cases = summary.runtime_adapter_stream_trace_cases();
        if runtime_adapter_stream_trace_cases < min_runtime_adapter_stream_trace_cases {
            failures.push(format!(
                "runtime_adapter_stream_trace_cases {} below minimum {}",
                runtime_adapter_stream_trace_cases, min_runtime_adapter_stream_trace_cases
            ));
        }
    }

    if let Some(min_runtime_adapter_stream_gate_summary_cases) =
        gate.min_runtime_adapter_stream_gate_summary_cases
    {
        let runtime_adapter_stream_gate_summary_cases =
            summary.runtime_adapter_stream_gate_summary_cases();
        if runtime_adapter_stream_gate_summary_cases < min_runtime_adapter_stream_gate_summary_cases
        {
            failures.push(format!(
                "runtime_adapter_stream_gate_summary_cases {} below minimum {}",
                runtime_adapter_stream_gate_summary_cases,
                min_runtime_adapter_stream_gate_summary_cases
            ));
        }
    }

    if let Some(min_runtime_adapter_observations) = gate.min_runtime_adapter_observations {
        let runtime_adapter_observations = summary.total_runtime_adapter_observations();
        if runtime_adapter_observations < min_runtime_adapter_observations {
            failures.push(format!(
                "runtime_adapter_observations {} below minimum {}",
                runtime_adapter_observations, min_runtime_adapter_observations
            ));
        }
    }

    if let Some(min_runtime_adapter_best_score) = gate.min_runtime_adapter_best_score {
        let runtime_adapter_best_score = summary.max_runtime_adapter_score().unwrap_or(0.0);
        if runtime_adapter_best_score < min_runtime_adapter_best_score {
            failures.push(format!(
                "runtime_adapter_best_score {:.3} below minimum {:.3}",
                runtime_adapter_best_score, min_runtime_adapter_best_score
            ));
        }
    }

    if let Some(max_runtime_adapter_contract_violations) =
        gate.max_runtime_adapter_contract_violations
    {
        let runtime_adapter_contract_violations =
            summary.total_runtime_adapter_contract_violations();
        if runtime_adapter_contract_violations > max_runtime_adapter_contract_violations {
            failures.push(format!(
                "runtime_adapter_contract_violations {} above maximum {}",
                runtime_adapter_contract_violations, max_runtime_adapter_contract_violations
            ));
        }
    }

    if let Some(max_runtime_adapter_selection_mismatches) =
        gate.max_runtime_adapter_selection_mismatches
    {
        let runtime_adapter_selection_mismatches =
            summary.total_runtime_adapter_selection_mismatches();
        if runtime_adapter_selection_mismatches > max_runtime_adapter_selection_mismatches {
            failures.push(format!(
                "runtime_adapter_selection_mismatches {} above maximum {}",
                runtime_adapter_selection_mismatches, max_runtime_adapter_selection_mismatches
            ));
        }
    }
}
