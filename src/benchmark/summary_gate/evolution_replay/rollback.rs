use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(max_evolution_drift_rollbacks) = gate.max_evolution_drift_rollbacks {
        let observed = summary.evolution_ledger.drift_rollbacks;
        if observed > max_evolution_drift_rollbacks {
            failures.push(format!(
                "evolution_drift_rollbacks {} above maximum {}",
                observed, max_evolution_drift_rollbacks
            ));
        }
    }

    if let Some(max_evolution_rollback_router_threshold_delta) =
        gate.max_evolution_rollback_router_threshold_delta
    {
        let observed = summary.evolution_ledger.rollback_router_threshold_delta;
        if observed > max_evolution_rollback_router_threshold_delta {
            failures.push(format!(
                "evolution_rollback_router_threshold_delta {:.6} above maximum {:.6}",
                observed, max_evolution_rollback_router_threshold_delta
            ));
        }
    }

    if let Some(max_evolution_rollback_hierarchy_weight_delta) =
        gate.max_evolution_rollback_hierarchy_weight_delta
    {
        let observed = summary.evolution_ledger.rollback_hierarchy_weight_delta;
        if observed > max_evolution_rollback_hierarchy_weight_delta {
            failures.push(format!(
                "evolution_rollback_hierarchy_weight_delta {:.6} above maximum {:.6}",
                observed, max_evolution_rollback_hierarchy_weight_delta
            ));
        }
    }
}
