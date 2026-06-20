use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_evolution_replay_live_memory_feedback_updates) =
        gate.min_evolution_replay_live_memory_feedback_updates
    {
        let observed = summary
            .evolution_ledger
            .replay_live_memory_feedback_updates();
        if observed < min_evolution_replay_live_memory_feedback_updates {
            failures.push(format!(
                "evolution_replay_live_memory_feedback_updates {} below minimum {}",
                observed, min_evolution_replay_live_memory_feedback_updates
            ));
        }
    }

    if let Some(min_evolution_replay_live_memory_feedback_detail_items) =
        gate.min_evolution_replay_live_memory_feedback_detail_items
    {
        let observed = summary
            .evolution_ledger
            .replay_live_memory_feedback_detail_items;
        if observed < min_evolution_replay_live_memory_feedback_detail_items {
            failures.push(format!(
                "evolution_replay_live_memory_feedback_detail_items {} below minimum {}",
                observed, min_evolution_replay_live_memory_feedback_detail_items
            ));
        }
    }

    if let Some(min_evolution_replay_live_memory_feedback_applied) =
        gate.min_evolution_replay_live_memory_feedback_applied
    {
        let observed = summary.evolution_ledger.replay_live_memory_feedback_applied;
        if observed < min_evolution_replay_live_memory_feedback_applied {
            failures.push(format!(
                "evolution_replay_live_memory_feedback_applied {} below minimum {}",
                observed, min_evolution_replay_live_memory_feedback_applied
            ));
        }
    }

    if let Some(min_evolution_replay_live_memory_feedback_strength_delta) =
        gate.min_evolution_replay_live_memory_feedback_strength_delta
    {
        let observed = summary
            .evolution_ledger
            .replay_live_memory_feedback_strength_delta;
        if observed < min_evolution_replay_live_memory_feedback_strength_delta {
            failures.push(format!(
                "evolution_replay_live_memory_feedback_strength_delta {:.6} below minimum {:.6}",
                observed, min_evolution_replay_live_memory_feedback_strength_delta
            ));
        }
    }
}
