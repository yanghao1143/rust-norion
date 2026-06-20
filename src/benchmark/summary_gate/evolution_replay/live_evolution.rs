use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    evaluate_ledger(summary, gate, failures);
    evaluate_device_profiles(summary, gate, failures);
}

fn evaluate_ledger(summary: &BenchmarkSummary, gate: &BenchmarkGate, failures: &mut GateFailures) {
    if let Some(min_evolution_replay_live_evolution_items) =
        gate.min_evolution_replay_live_evolution_items
    {
        let observed = summary.evolution_ledger.replay_live_evolution_items;
        if observed < min_evolution_replay_live_evolution_items {
            failures.push(format!(
                "evolution_replay_live_evolution_items {} below minimum {}",
                observed, min_evolution_replay_live_evolution_items
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_online_reward_feedbacks) =
        gate.min_evolution_replay_live_evolution_online_reward_feedbacks
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_online_reward_feedbacks;
        if observed < min_evolution_replay_live_evolution_online_reward_feedbacks {
            failures.push(format!(
                "evolution_replay_live_evolution_online_reward_feedbacks {} below minimum {}",
                observed, min_evolution_replay_live_evolution_online_reward_feedbacks
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_online_reward_reinforcements) =
        gate.min_evolution_replay_live_evolution_online_reward_reinforcements
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_online_reward_reinforcements;
        if observed < min_evolution_replay_live_evolution_online_reward_reinforcements {
            failures.push(format!(
                "evolution_replay_live_evolution_online_reward_reinforcements {} below minimum {}",
                observed, min_evolution_replay_live_evolution_online_reward_reinforcements
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_online_reward_penalties) =
        gate.min_evolution_replay_live_evolution_online_reward_penalties
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_online_reward_penalties;
        if observed < min_evolution_replay_live_evolution_online_reward_penalties {
            failures.push(format!(
                "evolution_replay_live_evolution_online_reward_penalties {} below minimum {}",
                observed, min_evolution_replay_live_evolution_online_reward_penalties
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_online_reward_strength) =
        gate.min_evolution_replay_live_evolution_online_reward_strength
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_online_reward_strength;
        if observed < min_evolution_replay_live_evolution_online_reward_strength {
            failures.push(format!(
                "evolution_replay_live_evolution_online_reward_strength {:.6} below minimum {:.6}",
                observed, min_evolution_replay_live_evolution_online_reward_strength
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_online_reward_reinforcement_strength) =
        gate.min_evolution_replay_live_evolution_online_reward_reinforcement_strength
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_online_reward_reinforcement_strength;
        if observed < min_evolution_replay_live_evolution_online_reward_reinforcement_strength {
            failures.push(format!(
                "evolution_replay_live_evolution_online_reward_reinforcement_strength {:.6} below minimum {:.6}",
                observed,
                min_evolution_replay_live_evolution_online_reward_reinforcement_strength
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_online_reward_penalty_strength) =
        gate.min_evolution_replay_live_evolution_online_reward_penalty_strength
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_online_reward_penalty_strength;
        if observed < min_evolution_replay_live_evolution_online_reward_penalty_strength {
            failures.push(format!(
                "evolution_replay_live_evolution_online_reward_penalty_strength {:.6} below minimum {:.6}",
                observed, min_evolution_replay_live_evolution_online_reward_penalty_strength
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_memory_updates) =
        gate.min_evolution_replay_live_evolution_memory_updates
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_memory_updates;
        if observed < min_evolution_replay_live_evolution_memory_updates {
            failures.push(format!(
                "evolution_replay_live_evolution_memory_updates {} below minimum {}",
                observed, min_evolution_replay_live_evolution_memory_updates
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_stored_memory_updates) =
        gate.min_evolution_replay_live_evolution_stored_memory_updates
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_stored_memory_updates;
        if observed < min_evolution_replay_live_evolution_stored_memory_updates {
            failures.push(format!(
                "evolution_replay_live_evolution_stored_memory_updates {} below minimum {}",
                observed, min_evolution_replay_live_evolution_stored_memory_updates
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_reflection_issues) =
        gate.min_evolution_replay_live_evolution_reflection_issues
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_reflection_issues;
        if observed < min_evolution_replay_live_evolution_reflection_issues {
            failures.push(format!(
                "evolution_replay_live_evolution_reflection_issues {} below minimum {}",
                observed, min_evolution_replay_live_evolution_reflection_issues
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_critical_reflection_issues) =
        gate.min_evolution_replay_live_evolution_critical_reflection_issues
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_critical_reflection_issues;
        if observed < min_evolution_replay_live_evolution_critical_reflection_issues {
            failures.push(format!(
                "evolution_replay_live_evolution_critical_reflection_issues {} below minimum {}",
                observed, min_evolution_replay_live_evolution_critical_reflection_issues
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_revision_actions) =
        gate.min_evolution_replay_live_evolution_revision_actions
    {
        let observed = summary
            .evolution_ledger
            .replay_live_evolution_revision_actions;
        if observed < min_evolution_replay_live_evolution_revision_actions {
            failures.push(format!(
                "evolution_replay_live_evolution_revision_actions {} below minimum {}",
                observed, min_evolution_replay_live_evolution_revision_actions
            ));
        }
    }
}

fn evaluate_device_profiles(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_evolution_replay_live_evolution_device_profiles) =
        gate.min_evolution_replay_live_evolution_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .replay_live_evolution_device_profiles();
        if observed < min_evolution_replay_live_evolution_device_profiles {
            failures.push(format!(
                "evolution_replay_live_evolution_device_profiles {} below minimum {}",
                observed, min_evolution_replay_live_evolution_device_profiles
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_online_reward_device_profiles) =
        gate.min_evolution_replay_live_evolution_online_reward_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .replay_live_evolution_online_reward_device_profiles();
        if observed < min_evolution_replay_live_evolution_online_reward_device_profiles {
            failures.push(format!(
                "evolution_replay_live_evolution_online_reward_device_profiles {} below minimum {}",
                observed, min_evolution_replay_live_evolution_online_reward_device_profiles
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_online_reward_strength_device_profiles) =
        gate.min_evolution_replay_live_evolution_online_reward_strength_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .replay_live_evolution_online_reward_strength_device_profiles();
        if observed < min_evolution_replay_live_evolution_online_reward_strength_device_profiles {
            failures.push(format!(
                "evolution_replay_live_evolution_online_reward_strength_device_profiles {} below minimum {}",
                observed,
                min_evolution_replay_live_evolution_online_reward_strength_device_profiles
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_memory_update_device_profiles) =
        gate.min_evolution_replay_live_evolution_memory_update_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .replay_live_evolution_memory_update_device_profiles();
        if observed < min_evolution_replay_live_evolution_memory_update_device_profiles {
            failures.push(format!(
                "evolution_replay_live_evolution_memory_update_device_profiles {} below minimum {}",
                observed, min_evolution_replay_live_evolution_memory_update_device_profiles
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles) =
        gate.min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .replay_live_evolution_critical_reflection_issue_device_profiles();
        if observed < min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles
        {
            failures.push(format!(
                "evolution_replay_live_evolution_critical_reflection_issue_device_profiles {} below minimum {}",
                observed,
                min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles
            ));
        }
    }

    if let Some(min_evolution_replay_live_evolution_revision_action_device_profiles) =
        gate.min_evolution_replay_live_evolution_revision_action_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .replay_live_evolution_revision_action_device_profiles();
        if observed < min_evolution_replay_live_evolution_revision_action_device_profiles {
            failures.push(format!(
                "evolution_replay_live_evolution_revision_action_device_profiles {} below minimum {}",
                observed, min_evolution_replay_live_evolution_revision_action_device_profiles
            ));
        }
    }
}
