use super::super::BenchmarkGate;
use super::super::summary::BenchmarkSummary;
use super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(max_adaptive_routing_failures) = gate.max_adaptive_routing_failures {
        let adaptive_routing_failures = summary.total_adaptive_routing_failures();
        if adaptive_routing_failures > max_adaptive_routing_failures {
            failures.push(format!(
                "adaptive_routing_failures {} above maximum {}: {}",
                adaptive_routing_failures,
                max_adaptive_routing_failures,
                summary.routing_evidence.failures.join("; ")
            ));
        }
    }

    if let Some(min_adaptive_routing_cases) = gate.min_adaptive_routing_cases {
        let observed = summary.adaptive_routing_cases();
        if observed < min_adaptive_routing_cases {
            failures.push(format!(
                "adaptive_routing_cases {} below minimum {}",
                observed, min_adaptive_routing_cases
            ));
        }
    }

    if let Some(min_adaptive_routing_device_profiles) = gate.min_adaptive_routing_device_profiles {
        let observed = summary.adaptive_routing_device_profiles();
        if observed < min_adaptive_routing_device_profiles {
            failures.push(format!(
                "adaptive_routing_device_profiles {} below minimum {}",
                observed, min_adaptive_routing_device_profiles
            ));
        }
    }

    if let Some(min_adaptive_routing_saved_tokens) = gate.min_adaptive_routing_saved_tokens {
        let observed = summary.total_adaptive_routing_saved_tokens();
        if observed < min_adaptive_routing_saved_tokens {
            failures.push(format!(
                "adaptive_routing_saved_tokens {} below minimum {}",
                observed, min_adaptive_routing_saved_tokens
            ));
        }
    }

    if let Some(min_adaptive_routing_saved_token_device_profiles) =
        gate.min_adaptive_routing_saved_token_device_profiles
    {
        let observed = summary.adaptive_routing_saved_token_device_profiles();
        if observed < min_adaptive_routing_saved_token_device_profiles {
            failures.push(format!(
                "adaptive_routing_saved_token_device_profiles {} below minimum {}",
                observed, min_adaptive_routing_saved_token_device_profiles
            ));
        }
    }

    if let Some(min_task_hierarchy_cases) = gate.min_task_hierarchy_cases {
        let observed = summary.task_hierarchy_cases();
        if observed < min_task_hierarchy_cases {
            failures.push(format!(
                "task_hierarchy_cases {} below minimum {}",
                observed, min_task_hierarchy_cases
            ));
        }
    }

    if let Some(min_task_hierarchy_modes) = gate.min_task_hierarchy_modes {
        let observed = summary.task_hierarchy_mode_count();
        if observed < min_task_hierarchy_modes {
            failures.push(format!(
                "task_hierarchy_modes {} below minimum {}",
                observed, min_task_hierarchy_modes
            ));
        }
    }

    if let Some(min_task_hierarchy_mutation_records) = gate.min_task_hierarchy_mutation_records {
        let observed = summary.total_task_hierarchy_mutation_records();
        if observed < min_task_hierarchy_mutation_records {
            failures.push(format!(
                "task_hierarchy_mutation_records {} below minimum {}",
                observed, min_task_hierarchy_mutation_records
            ));
        }
    }

    if let Some(min_task_hierarchy_compute_reduction_milli) =
        gate.min_task_hierarchy_compute_reduction_milli
    {
        let observed = summary.total_task_hierarchy_compute_reduction_milli();
        if observed < min_task_hierarchy_compute_reduction_milli {
            failures.push(format!(
                "task_hierarchy_compute_reduction_milli {} below minimum {}",
                observed, min_task_hierarchy_compute_reduction_milli
            ));
        }
    }

    if let Some(min_compute_budget_avoided_tokens) = gate.min_compute_budget_avoided_tokens {
        let observed = summary.total_compute_budget_avoided_tokens();
        if observed < min_compute_budget_avoided_tokens {
            failures.push(format!(
                "compute_budget_avoided_tokens {} below minimum {}",
                observed, min_compute_budget_avoided_tokens
            ));
        }
    }

    if let Some(min_compute_budget_fanout_reduction) = gate.min_compute_budget_fanout_reduction {
        let observed = summary.total_compute_budget_fanout_reduction();
        if observed < min_compute_budget_fanout_reduction {
            failures.push(format!(
                "compute_budget_fanout_reduction {} below minimum {}",
                observed, min_compute_budget_fanout_reduction
            ));
        }
    }
}
