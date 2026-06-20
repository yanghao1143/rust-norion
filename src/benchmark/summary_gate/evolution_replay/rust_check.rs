use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_evolution_replay_rust_check_items) = gate.min_evolution_replay_rust_check_items
    {
        let observed = summary.evolution_ledger.replay_rust_check_items;
        if observed < min_evolution_replay_rust_check_items {
            failures.push(format!(
                "evolution_replay_rust_check_items {} below minimum {}",
                observed, min_evolution_replay_rust_check_items
            ));
        }
    }

    if let Some(min_evolution_replay_rust_check_passed) =
        gate.min_evolution_replay_rust_check_passed
    {
        let observed = summary.evolution_ledger.replay_rust_check_passed;
        if observed < min_evolution_replay_rust_check_passed {
            failures.push(format!(
                "evolution_replay_rust_check_passed {} below minimum {}",
                observed, min_evolution_replay_rust_check_passed
            ));
        }
    }

    if let Some(max_evolution_replay_rust_check_failed) =
        gate.max_evolution_replay_rust_check_failed
    {
        let observed = summary.evolution_ledger.replay_rust_check_failed;
        if observed > max_evolution_replay_rust_check_failed {
            failures.push(format!(
                "evolution_replay_rust_check_failed {} above maximum {}",
                observed, max_evolution_replay_rust_check_failed
            ));
        }
    }

    if let Some(min_evolution_replay_rust_check_live_memory_feedback_updates) =
        gate.min_evolution_replay_rust_check_live_memory_feedback_updates
    {
        let observed = summary
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_updates;
        if observed < min_evolution_replay_rust_check_live_memory_feedback_updates {
            failures.push(format!(
                "evolution_replay_rust_check_live_memory_feedback_updates {} below minimum {}",
                observed, min_evolution_replay_rust_check_live_memory_feedback_updates
            ));
        }
    }

    if let Some(min_evolution_replay_rust_check_live_memory_feedback_applied) =
        gate.min_evolution_replay_rust_check_live_memory_feedback_applied
    {
        let observed = summary
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_applied;
        if observed < min_evolution_replay_rust_check_live_memory_feedback_applied {
            failures.push(format!(
                "evolution_replay_rust_check_live_memory_feedback_applied {} below minimum {}",
                observed, min_evolution_replay_rust_check_live_memory_feedback_applied
            ));
        }
    }

    if let Some(min_evolution_replay_rust_check_live_memory_feedback_strength_delta) =
        gate.min_evolution_replay_rust_check_live_memory_feedback_strength_delta
    {
        let observed = summary
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_strength_delta;
        if observed < min_evolution_replay_rust_check_live_memory_feedback_strength_delta {
            failures.push(format!(
                "evolution_replay_rust_check_live_memory_feedback_strength_delta {:.6} below minimum {:.6}",
                observed, min_evolution_replay_rust_check_live_memory_feedback_strength_delta
            ));
        }
    }
}
