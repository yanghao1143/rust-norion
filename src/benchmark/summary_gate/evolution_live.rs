use super::super::BenchmarkGate;
use super::super::summary::BenchmarkSummary;
use super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_evolution_live_inference_runs) = gate.min_evolution_live_inference_runs {
        let observed = summary.evolution_ledger.live_inference_runs;
        if observed < min_evolution_live_inference_runs {
            failures.push(format!(
                "evolution_live_inference_runs {} below minimum {}",
                observed, min_evolution_live_inference_runs
            ));
        }
    }

    if let Some(min_evolution_live_router_threshold_mutations) =
        gate.min_evolution_live_router_threshold_mutations
    {
        let observed = summary.evolution_ledger.live_router_threshold_mutations;
        if observed < min_evolution_live_router_threshold_mutations {
            failures.push(format!(
                "evolution_live_router_threshold_mutations {} below minimum {}",
                observed, min_evolution_live_router_threshold_mutations
            ));
        }
    }

    if let Some(min_evolution_live_hierarchy_weight_mutations) =
        gate.min_evolution_live_hierarchy_weight_mutations
    {
        let observed = summary.evolution_ledger.live_hierarchy_weight_mutations;
        if observed < min_evolution_live_hierarchy_weight_mutations {
            failures.push(format!(
                "evolution_live_hierarchy_weight_mutations {} below minimum {}",
                observed, min_evolution_live_hierarchy_weight_mutations
            ));
        }
    }

    if let Some(min_evolution_live_router_threshold_delta) =
        gate.min_evolution_live_router_threshold_delta
    {
        let observed = summary.evolution_ledger.live_router_threshold_delta;
        if observed < min_evolution_live_router_threshold_delta {
            failures.push(format!(
                "evolution_live_router_threshold_delta {:.6} below minimum {:.6}",
                observed, min_evolution_live_router_threshold_delta
            ));
        }
    }

    if let Some(min_evolution_live_hierarchy_weight_delta) =
        gate.min_evolution_live_hierarchy_weight_delta
    {
        let observed = summary.evolution_ledger.live_hierarchy_weight_delta;
        if observed < min_evolution_live_hierarchy_weight_delta {
            failures.push(format!(
                "evolution_live_hierarchy_weight_delta {:.6} below minimum {:.6}",
                observed, min_evolution_live_hierarchy_weight_delta
            ));
        }
    }

    if let Some(min_evolution_live_online_reward_feedbacks) =
        gate.min_evolution_live_online_reward_feedbacks
    {
        let observed = summary.evolution_ledger.live_online_reward_feedbacks;
        if observed < min_evolution_live_online_reward_feedbacks {
            failures.push(format!(
                "evolution_live_online_reward_feedbacks {} below minimum {}",
                observed, min_evolution_live_online_reward_feedbacks
            ));
        }
    }

    if let Some(min_evolution_live_online_reward_reinforcements) =
        gate.min_evolution_live_online_reward_reinforcements
    {
        let observed = summary.evolution_ledger.live_online_reward_reinforcements;
        if observed < min_evolution_live_online_reward_reinforcements {
            failures.push(format!(
                "evolution_live_online_reward_reinforcements {} below minimum {}",
                observed, min_evolution_live_online_reward_reinforcements
            ));
        }
    }

    if let Some(min_evolution_live_online_reward_penalties) =
        gate.min_evolution_live_online_reward_penalties
    {
        let observed = summary.evolution_ledger.live_online_reward_penalties;
        if observed < min_evolution_live_online_reward_penalties {
            failures.push(format!(
                "evolution_live_online_reward_penalties {} below minimum {}",
                observed, min_evolution_live_online_reward_penalties
            ));
        }
    }

    if let Some(min_evolution_live_online_reward_strength) =
        gate.min_evolution_live_online_reward_strength
    {
        let observed = summary.evolution_ledger.live_online_reward_strength;
        if observed < min_evolution_live_online_reward_strength {
            failures.push(format!(
                "evolution_live_online_reward_strength {:.6} below minimum {:.6}",
                observed, min_evolution_live_online_reward_strength
            ));
        }
    }

    if let Some(min_evolution_live_online_reward_reinforcement_strength) =
        gate.min_evolution_live_online_reward_reinforcement_strength
    {
        let observed = summary
            .evolution_ledger
            .live_online_reward_reinforcement_strength;
        if observed < min_evolution_live_online_reward_reinforcement_strength {
            failures.push(format!(
                "evolution_live_online_reward_reinforcement_strength {:.6} below minimum {:.6}",
                observed, min_evolution_live_online_reward_reinforcement_strength
            ));
        }
    }

    if let Some(min_evolution_live_online_reward_penalty_strength) =
        gate.min_evolution_live_online_reward_penalty_strength
    {
        let observed = summary.evolution_ledger.live_online_reward_penalty_strength;
        if observed < min_evolution_live_online_reward_penalty_strength {
            failures.push(format!(
                "evolution_live_online_reward_penalty_strength {:.6} below minimum {:.6}",
                observed, min_evolution_live_online_reward_penalty_strength
            ));
        }
    }

    if let Some(min_evolution_live_memory_updates) = gate.min_evolution_live_memory_updates {
        let observed = summary.evolution_ledger.live_memory_updates();
        if observed < min_evolution_live_memory_updates {
            failures.push(format!(
                "evolution_live_memory_updates {} below minimum {}",
                observed, min_evolution_live_memory_updates
            ));
        }
    }

    if let Some(min_evolution_live_stored_memory_updates) =
        gate.min_evolution_live_stored_memory_updates
    {
        let observed = summary.evolution_ledger.live_stored_memory_updates();
        if observed < min_evolution_live_stored_memory_updates {
            failures.push(format!(
                "evolution_live_stored_memory_updates {} below minimum {}",
                observed, min_evolution_live_stored_memory_updates
            ));
        }
    }

    if let Some(min_evolution_live_reflection_issues) = gate.min_evolution_live_reflection_issues {
        let observed = summary.evolution_ledger.live_reflection_issues;
        if observed < min_evolution_live_reflection_issues {
            failures.push(format!(
                "evolution_live_reflection_issues {} below minimum {}",
                observed, min_evolution_live_reflection_issues
            ));
        }
    }

    if let Some(min_evolution_live_critical_reflection_issues) =
        gate.min_evolution_live_critical_reflection_issues
    {
        let observed = summary.evolution_ledger.live_critical_reflection_issues;
        if observed < min_evolution_live_critical_reflection_issues {
            failures.push(format!(
                "evolution_live_critical_reflection_issues {} below minimum {}",
                observed, min_evolution_live_critical_reflection_issues
            ));
        }
    }

    if let Some(min_evolution_live_revision_actions) = gate.min_evolution_live_revision_actions {
        let observed = summary.evolution_ledger.live_revision_actions;
        if observed < min_evolution_live_revision_actions {
            failures.push(format!(
                "evolution_live_revision_actions {} below minimum {}",
                observed, min_evolution_live_revision_actions
            ));
        }
    }

    if let Some(min_evolution_live_inference_device_profiles) =
        gate.min_evolution_live_inference_device_profiles
    {
        let observed = summary.live_evolution_evidence.inference_device_profiles();
        if observed < min_evolution_live_inference_device_profiles {
            failures.push(format!(
                "evolution_live_inference_device_profiles {} below minimum {}",
                observed, min_evolution_live_inference_device_profiles
            ));
        }
    }

    if let Some(min_evolution_live_router_threshold_mutation_device_profiles) =
        gate.min_evolution_live_router_threshold_mutation_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .router_threshold_mutation_device_profiles();
        if observed < min_evolution_live_router_threshold_mutation_device_profiles {
            failures.push(format!(
                "evolution_live_router_threshold_mutation_device_profiles {} below minimum {}",
                observed, min_evolution_live_router_threshold_mutation_device_profiles
            ));
        }
    }

    if let Some(min_evolution_live_hierarchy_weight_mutation_device_profiles) =
        gate.min_evolution_live_hierarchy_weight_mutation_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .hierarchy_weight_mutation_device_profiles();
        if observed < min_evolution_live_hierarchy_weight_mutation_device_profiles {
            failures.push(format!(
                "evolution_live_hierarchy_weight_mutation_device_profiles {} below minimum {}",
                observed, min_evolution_live_hierarchy_weight_mutation_device_profiles
            ));
        }
    }

    if let Some(min_evolution_live_online_reward_device_profiles) =
        gate.min_evolution_live_online_reward_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .online_reward_device_profiles();
        if observed < min_evolution_live_online_reward_device_profiles {
            failures.push(format!(
                "evolution_live_online_reward_device_profiles {} below minimum {}",
                observed, min_evolution_live_online_reward_device_profiles
            ));
        }
    }

    if let Some(min_evolution_live_online_reward_strength_device_profiles) =
        gate.min_evolution_live_online_reward_strength_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .online_reward_strength_device_profiles();
        if observed < min_evolution_live_online_reward_strength_device_profiles {
            failures.push(format!(
                "evolution_live_online_reward_strength_device_profiles {} below minimum {}",
                observed, min_evolution_live_online_reward_strength_device_profiles
            ));
        }
    }

    if let Some(min_evolution_live_memory_update_device_profiles) =
        gate.min_evolution_live_memory_update_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .memory_update_device_profiles();
        if observed < min_evolution_live_memory_update_device_profiles {
            failures.push(format!(
                "evolution_live_memory_update_device_profiles {} below minimum {}",
                observed, min_evolution_live_memory_update_device_profiles
            ));
        }
    }

    if let Some(min_evolution_live_stored_memory_update_device_profiles) =
        gate.min_evolution_live_stored_memory_update_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .stored_memory_update_device_profiles();
        if observed < min_evolution_live_stored_memory_update_device_profiles {
            failures.push(format!(
                "evolution_live_stored_memory_update_device_profiles {} below minimum {}",
                observed, min_evolution_live_stored_memory_update_device_profiles
            ));
        }
    }

    if let Some(min_evolution_live_reflection_issue_device_profiles) =
        gate.min_evolution_live_reflection_issue_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .reflection_issue_device_profiles();
        if observed < min_evolution_live_reflection_issue_device_profiles {
            failures.push(format!(
                "evolution_live_reflection_issue_device_profiles {} below minimum {}",
                observed, min_evolution_live_reflection_issue_device_profiles
            ));
        }
    }

    if let Some(min_evolution_live_critical_reflection_issue_device_profiles) =
        gate.min_evolution_live_critical_reflection_issue_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .critical_reflection_issue_device_profiles();
        if observed < min_evolution_live_critical_reflection_issue_device_profiles {
            failures.push(format!(
                "evolution_live_critical_reflection_issue_device_profiles {} below minimum {}",
                observed, min_evolution_live_critical_reflection_issue_device_profiles
            ));
        }
    }

    if let Some(min_evolution_live_revision_action_device_profiles) =
        gate.min_evolution_live_revision_action_device_profiles
    {
        let observed = summary
            .live_evolution_evidence
            .revision_action_device_profiles();
        if observed < min_evolution_live_revision_action_device_profiles {
            failures.push(format!(
                "evolution_live_revision_action_device_profiles {} below minimum {}",
                observed, min_evolution_live_revision_action_device_profiles
            ));
        }
    }
}
