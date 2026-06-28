use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_evolution_replay_runs) = gate.min_evolution_replay_runs {
        let observed = summary.evolution_ledger.replay_runs;
        if observed < min_evolution_replay_runs {
            failures.push(format!(
                "evolution_replay_runs {} below minimum {}",
                observed, min_evolution_replay_runs
            ));
        }
    }

    if let Some(min_evolution_replay_items) = gate.min_evolution_replay_items {
        let observed = summary.evolution_ledger.replay_items;
        if observed < min_evolution_replay_items {
            failures.push(format!(
                "evolution_replay_items {} below minimum {}",
                observed, min_evolution_replay_items
            ));
        }
    }

    if let Some(min_evolution_router_threshold_mutations) =
        gate.min_evolution_router_threshold_mutations
    {
        let observed = summary.evolution_ledger.router_threshold_mutations;
        if observed < min_evolution_router_threshold_mutations {
            failures.push(format!(
                "evolution_router_threshold_mutations {} below minimum {}",
                observed, min_evolution_router_threshold_mutations
            ));
        }
    }

    if let Some(min_evolution_hierarchy_weight_mutations) =
        gate.min_evolution_hierarchy_weight_mutations
    {
        let observed = summary.evolution_ledger.hierarchy_weight_mutations;
        if observed < min_evolution_hierarchy_weight_mutations {
            failures.push(format!(
                "evolution_hierarchy_weight_mutations {} below minimum {}",
                observed, min_evolution_hierarchy_weight_mutations
            ));
        }
    }

    if let Some(min_evolution_router_threshold_delta) = gate.min_evolution_router_threshold_delta {
        let observed = summary.evolution_ledger.router_threshold_delta;
        if observed < min_evolution_router_threshold_delta {
            failures.push(format!(
                "evolution_router_threshold_delta {:.6} below minimum {:.6}",
                observed, min_evolution_router_threshold_delta
            ));
        }
    }

    if let Some(min_evolution_hierarchy_weight_delta) = gate.min_evolution_hierarchy_weight_delta {
        let observed = summary.evolution_ledger.hierarchy_weight_delta;
        if observed < min_evolution_hierarchy_weight_delta {
            failures.push(format!(
                "evolution_hierarchy_weight_delta {:.6} below minimum {:.6}",
                observed, min_evolution_hierarchy_weight_delta
            ));
        }
    }

    if let Some(min_evolution_memory_updates) = gate.min_evolution_memory_updates {
        let observed = summary.evolution_ledger.memory_updates();
        if observed < min_evolution_memory_updates {
            failures.push(format!(
                "evolution_memory_updates {} below minimum {}",
                observed, min_evolution_memory_updates
            ));
        }
    }

    if let Some(min_evolution_external_feedbacks) = gate.min_evolution_external_feedbacks {
        let observed = summary.evolution_ledger.external_feedbacks;
        if observed < min_evolution_external_feedbacks {
            failures.push(format!(
                "evolution_external_feedbacks {} below minimum {}",
                observed, min_evolution_external_feedbacks
            ));
        }
    }

    if let Some(min_evolution_external_feedback_reinforcements) =
        gate.min_evolution_external_feedback_reinforcements
    {
        let observed = summary.evolution_ledger.external_feedback_reinforcements;
        if observed < min_evolution_external_feedback_reinforcements {
            failures.push(format!(
                "evolution_external_feedback_reinforcements {} below minimum {}",
                observed, min_evolution_external_feedback_reinforcements
            ));
        }
    }

    if let Some(min_evolution_external_feedback_penalties) =
        gate.min_evolution_external_feedback_penalties
    {
        let observed = summary.evolution_ledger.external_feedback_penalties;
        if observed < min_evolution_external_feedback_penalties {
            failures.push(format!(
                "evolution_external_feedback_penalties {} below minimum {}",
                observed, min_evolution_external_feedback_penalties
            ));
        }
    }

    if let Some(min_evolution_external_feedback_memory_updates) =
        gate.min_evolution_external_feedback_memory_updates
    {
        let observed = summary.evolution_ledger.external_feedback_memory_updates;
        if observed < min_evolution_external_feedback_memory_updates {
            failures.push(format!(
                "evolution_external_feedback_memory_updates {} below minimum {}",
                observed, min_evolution_external_feedback_memory_updates
            ));
        }
    }

    if let Some(min_evolution_external_feedback_strength_delta) =
        gate.min_evolution_external_feedback_strength_delta
    {
        let observed = summary.evolution_ledger.external_feedback_strength_delta;
        if observed < min_evolution_external_feedback_strength_delta {
            failures.push(format!(
                "evolution_external_feedback_strength_delta {:.6} below minimum {:.6}",
                observed, min_evolution_external_feedback_strength_delta
            ));
        }
    }

    if let Some(max_evolution_external_feedback_missing) =
        gate.max_evolution_external_feedback_missing
    {
        let observed = summary.evolution_ledger.external_feedback_missing;
        if observed > max_evolution_external_feedback_missing {
            failures.push(format!(
                "evolution_external_feedback_missing {} above maximum {}",
                observed, max_evolution_external_feedback_missing
            ));
        }
    }
}
