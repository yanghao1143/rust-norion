use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_evolution_recursive_replay_items) = gate.min_evolution_recursive_replay_items {
        let observed = summary.evolution_ledger.recursive_replay_items;
        if observed < min_evolution_recursive_replay_items {
            failures.push(format!(
                "evolution_recursive_replay_items {} below minimum {}",
                observed, min_evolution_recursive_replay_items
            ));
        }
    }

    if let Some(min_evolution_recursive_runtime_calls) = gate.min_evolution_recursive_runtime_calls
    {
        let observed = summary.evolution_ledger.recursive_runtime_calls;
        if observed < min_evolution_recursive_runtime_calls {
            failures.push(format!(
                "evolution_recursive_runtime_calls {} below minimum {}",
                observed, min_evolution_recursive_runtime_calls
            ));
        }
    }
}
