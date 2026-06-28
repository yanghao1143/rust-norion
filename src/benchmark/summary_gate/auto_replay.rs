use super::super::summary::BenchmarkSummary;
use super::super::BenchmarkGate;
use super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_auto_replay_router_updates) = gate.min_auto_replay_router_updates {
        let auto_replay_router_updates = summary.total_auto_replay_router_updates();
        if auto_replay_router_updates < min_auto_replay_router_updates {
            failures.push(format!(
                "auto_replay_router_updates {} below minimum {}",
                auto_replay_router_updates, min_auto_replay_router_updates
            ));
        }
    }

    if let Some(min_auto_replay_hierarchy_updates) = gate.min_auto_replay_hierarchy_updates {
        let auto_replay_hierarchy_updates = summary.total_auto_replay_hierarchy_updates();
        if auto_replay_hierarchy_updates < min_auto_replay_hierarchy_updates {
            failures.push(format!(
                "auto_replay_hierarchy_updates {} below minimum {}",
                auto_replay_hierarchy_updates, min_auto_replay_hierarchy_updates
            ));
        }
    }

    if let Some(min_auto_replay_router_threshold_mutations) =
        gate.min_auto_replay_router_threshold_mutations
    {
        let auto_replay_router_threshold_mutations =
            summary.total_auto_replay_router_threshold_mutations();
        if auto_replay_router_threshold_mutations < min_auto_replay_router_threshold_mutations {
            failures.push(format!(
                "auto_replay_router_threshold_mutations {} below minimum {}",
                auto_replay_router_threshold_mutations, min_auto_replay_router_threshold_mutations
            ));
        }
    }

    if let Some(min_auto_replay_hierarchy_weight_mutations) =
        gate.min_auto_replay_hierarchy_weight_mutations
    {
        let auto_replay_hierarchy_weight_mutations =
            summary.total_auto_replay_hierarchy_weight_mutations();
        if auto_replay_hierarchy_weight_mutations < min_auto_replay_hierarchy_weight_mutations {
            failures.push(format!(
                "auto_replay_hierarchy_weight_mutations {} below minimum {}",
                auto_replay_hierarchy_weight_mutations, min_auto_replay_hierarchy_weight_mutations
            ));
        }
    }

    if let Some(min_auto_replay_router_threshold_delta) =
        gate.min_auto_replay_router_threshold_delta
    {
        let auto_replay_router_threshold_delta = summary.total_auto_replay_router_threshold_delta();
        if auto_replay_router_threshold_delta < min_auto_replay_router_threshold_delta {
            failures.push(format!(
                "auto_replay_router_threshold_delta {:.6} below minimum {:.6}",
                auto_replay_router_threshold_delta, min_auto_replay_router_threshold_delta
            ));
        }
    }

    if let Some(min_auto_replay_hierarchy_weight_delta) =
        gate.min_auto_replay_hierarchy_weight_delta
    {
        let auto_replay_hierarchy_weight_delta = summary.total_auto_replay_hierarchy_weight_delta();
        if auto_replay_hierarchy_weight_delta < min_auto_replay_hierarchy_weight_delta {
            failures.push(format!(
                "auto_replay_hierarchy_weight_delta {:.6} below minimum {:.6}",
                auto_replay_hierarchy_weight_delta, min_auto_replay_hierarchy_weight_delta
            ));
        }
    }

    if let Some(min_auto_replay_memory_updates) = gate.min_auto_replay_memory_updates {
        let auto_replay_memory_updates = summary.total_auto_replay_memory_updates();
        if auto_replay_memory_updates < min_auto_replay_memory_updates {
            failures.push(format!(
                "auto_replay_memory_updates {} below minimum {}",
                auto_replay_memory_updates, min_auto_replay_memory_updates
            ));
        }
    }

    if let Some(min_live_memory_feedback_updates) = gate.min_live_memory_feedback_updates {
        let live_memory_feedback_updates = summary.total_live_memory_feedback_updates();
        if live_memory_feedback_updates < min_live_memory_feedback_updates {
            failures.push(format!(
                "live_memory_feedback_updates {} below minimum {}",
                live_memory_feedback_updates, min_live_memory_feedback_updates
            ));
        }
    }

    if let Some(min_live_memory_feedback_reinforcements) =
        gate.min_live_memory_feedback_reinforcements
    {
        let live_memory_feedback_reinforcements =
            summary.total_live_memory_feedback_reinforcements();
        if live_memory_feedback_reinforcements < min_live_memory_feedback_reinforcements {
            failures.push(format!(
                "live_memory_feedback_reinforcements {} below minimum {}",
                live_memory_feedback_reinforcements, min_live_memory_feedback_reinforcements
            ));
        }
    }

    if let Some(min_live_memory_feedback_penalties) = gate.min_live_memory_feedback_penalties {
        let live_memory_feedback_penalties = summary.total_live_memory_feedback_penalties();
        if live_memory_feedback_penalties < min_live_memory_feedback_penalties {
            failures.push(format!(
                "live_memory_feedback_penalties {} below minimum {}",
                live_memory_feedback_penalties, min_live_memory_feedback_penalties
            ));
        }
    }

    if let Some(min_live_memory_feedback_applied) = gate.min_live_memory_feedback_applied {
        let live_memory_feedback_applied = summary.total_live_memory_feedback_applied();
        if live_memory_feedback_applied < min_live_memory_feedback_applied {
            failures.push(format!(
                "live_memory_feedback_applied {} below minimum {}",
                live_memory_feedback_applied, min_live_memory_feedback_applied
            ));
        }
    }

    if let Some(min_live_memory_feedback_strength_delta) =
        gate.min_live_memory_feedback_strength_delta
    {
        let live_memory_feedback_strength_delta =
            summary.total_live_memory_feedback_strength_delta();
        if live_memory_feedback_strength_delta < min_live_memory_feedback_strength_delta {
            failures.push(format!(
                "live_memory_feedback_strength_delta {:.6} below minimum {:.6}",
                live_memory_feedback_strength_delta, min_live_memory_feedback_strength_delta
            ));
        }
    }

    if let Some(max_live_memory_feedback_missing) = gate.max_live_memory_feedback_missing {
        let live_memory_feedback_missing = summary.total_live_memory_feedback_missing();
        if live_memory_feedback_missing > max_live_memory_feedback_missing {
            failures.push(format!(
                "live_memory_feedback_missing {} above maximum {}",
                live_memory_feedback_missing, max_live_memory_feedback_missing
            ));
        }
    }

    if let Some(min_auto_replay_live_memory_feedback_updates) =
        gate.min_auto_replay_live_memory_feedback_updates
    {
        let auto_replay_live_memory_feedback_updates =
            summary.total_auto_replay_live_memory_feedback_updates();
        if auto_replay_live_memory_feedback_updates < min_auto_replay_live_memory_feedback_updates {
            failures.push(format!(
                "auto_replay_live_memory_feedback_updates {} below minimum {}",
                auto_replay_live_memory_feedback_updates,
                min_auto_replay_live_memory_feedback_updates
            ));
        }
    }

    if let Some(min_auto_replay_live_memory_feedback_reinforcements) =
        gate.min_auto_replay_live_memory_feedback_reinforcements
    {
        let auto_replay_live_memory_feedback_reinforcements =
            summary.total_auto_replay_live_memory_feedback_reinforcements();
        if auto_replay_live_memory_feedback_reinforcements
            < min_auto_replay_live_memory_feedback_reinforcements
        {
            failures.push(format!(
                "auto_replay_live_memory_feedback_reinforcements {} below minimum {}",
                auto_replay_live_memory_feedback_reinforcements,
                min_auto_replay_live_memory_feedback_reinforcements
            ));
        }
    }

    if let Some(min_auto_replay_live_memory_feedback_penalties) =
        gate.min_auto_replay_live_memory_feedback_penalties
    {
        let auto_replay_live_memory_feedback_penalties =
            summary.total_auto_replay_live_memory_feedback_penalties();
        if auto_replay_live_memory_feedback_penalties
            < min_auto_replay_live_memory_feedback_penalties
        {
            failures.push(format!(
                "auto_replay_live_memory_feedback_penalties {} below minimum {}",
                auto_replay_live_memory_feedback_penalties,
                min_auto_replay_live_memory_feedback_penalties
            ));
        }
    }

    if let Some(min_auto_replay_live_memory_feedback_detail_items) =
        gate.min_auto_replay_live_memory_feedback_detail_items
    {
        let auto_replay_live_memory_feedback_detail_items =
            summary.total_auto_replay_live_memory_feedback_detail_items();
        if auto_replay_live_memory_feedback_detail_items
            < min_auto_replay_live_memory_feedback_detail_items
        {
            failures.push(format!(
                "auto_replay_live_memory_feedback_detail_items {} below minimum {}",
                auto_replay_live_memory_feedback_detail_items,
                min_auto_replay_live_memory_feedback_detail_items
            ));
        }
    }

    if let Some(min_auto_replay_live_memory_feedback_applied) =
        gate.min_auto_replay_live_memory_feedback_applied
    {
        let auto_replay_live_memory_feedback_applied =
            summary.total_auto_replay_live_memory_feedback_applied();
        if auto_replay_live_memory_feedback_applied < min_auto_replay_live_memory_feedback_applied {
            failures.push(format!(
                "auto_replay_live_memory_feedback_applied {} below minimum {}",
                auto_replay_live_memory_feedback_applied,
                min_auto_replay_live_memory_feedback_applied
            ));
        }
    }

    if let Some(min_auto_replay_live_memory_feedback_strength_delta) =
        gate.min_auto_replay_live_memory_feedback_strength_delta
    {
        let auto_replay_live_memory_feedback_strength_delta =
            summary.total_auto_replay_live_memory_feedback_strength_delta();
        if auto_replay_live_memory_feedback_strength_delta
            < min_auto_replay_live_memory_feedback_strength_delta
        {
            failures.push(format!(
                "auto_replay_live_memory_feedback_strength_delta {:.6} below minimum {:.6}",
                auto_replay_live_memory_feedback_strength_delta,
                min_auto_replay_live_memory_feedback_strength_delta
            ));
        }
    }

    if let Some(max_auto_replay_live_memory_feedback_missing) =
        gate.max_auto_replay_live_memory_feedback_missing
    {
        let auto_replay_live_memory_feedback_missing =
            summary.total_auto_replay_live_memory_feedback_missing();
        if auto_replay_live_memory_feedback_missing > max_auto_replay_live_memory_feedback_missing {
            failures.push(format!(
                "auto_replay_live_memory_feedback_missing {} above maximum {}",
                auto_replay_live_memory_feedback_missing,
                max_auto_replay_live_memory_feedback_missing
            ));
        }
    }

    if let Some(min_auto_replay_recursive_items) = gate.min_auto_replay_recursive_items {
        let auto_replay_recursive_items = summary.total_auto_replay_recursive_items();
        if auto_replay_recursive_items < min_auto_replay_recursive_items {
            failures.push(format!(
                "auto_replay_recursive_items {} below minimum {}",
                auto_replay_recursive_items, min_auto_replay_recursive_items
            ));
        }
    }

    if let Some(min_auto_replay_recursive_call_pressure) =
        gate.min_auto_replay_recursive_call_pressure
    {
        let auto_replay_recursive_call_pressure = summary.max_auto_replay_recursive_call_pressure();
        if auto_replay_recursive_call_pressure < min_auto_replay_recursive_call_pressure {
            failures.push(format!(
                "auto_replay_recursive_call_pressure {:.3} below minimum {:.3}",
                auto_replay_recursive_call_pressure, min_auto_replay_recursive_call_pressure
            ));
        }
    }

    if let Some(max_auto_replay_recursive_call_pressure) =
        gate.max_auto_replay_recursive_call_pressure
    {
        let auto_replay_recursive_call_pressure = summary.max_auto_replay_recursive_call_pressure();
        if auto_replay_recursive_call_pressure > max_auto_replay_recursive_call_pressure {
            failures.push(format!(
                "auto_replay_recursive_call_pressure {:.3} above maximum {:.3}",
                auto_replay_recursive_call_pressure, max_auto_replay_recursive_call_pressure
            ));
        }
    }

    if let Some(min_auto_replay_runtime_kv_budget_pressure_items) =
        gate.min_auto_replay_runtime_kv_budget_pressure_items
    {
        let auto_replay_runtime_kv_budget_pressure_items =
            summary.total_auto_replay_runtime_kv_budget_pressure_items();
        if auto_replay_runtime_kv_budget_pressure_items
            < min_auto_replay_runtime_kv_budget_pressure_items
        {
            failures.push(format!(
                "auto_replay_runtime_kv_budget_pressure_items {} below minimum {}",
                auto_replay_runtime_kv_budget_pressure_items,
                min_auto_replay_runtime_kv_budget_pressure_items
            ));
        }
    }

    if let Some(min_auto_replay_runtime_kv_budget_pressure) =
        gate.min_auto_replay_runtime_kv_budget_pressure
    {
        let auto_replay_runtime_kv_budget_pressure =
            summary.average_auto_replay_runtime_kv_budget_pressure();
        if auto_replay_runtime_kv_budget_pressure < min_auto_replay_runtime_kv_budget_pressure {
            failures.push(format!(
                "auto_replay_runtime_kv_budget_pressure {:.3} below minimum {:.3}",
                auto_replay_runtime_kv_budget_pressure, min_auto_replay_runtime_kv_budget_pressure
            ));
        }
    }

    if let Some(max_auto_replay_runtime_kv_budget_pressure) =
        gate.max_auto_replay_runtime_kv_budget_pressure
    {
        let auto_replay_runtime_kv_budget_pressure =
            summary.max_auto_replay_runtime_kv_budget_pressure();
        if auto_replay_runtime_kv_budget_pressure > max_auto_replay_runtime_kv_budget_pressure {
            failures.push(format!(
                "auto_replay_runtime_kv_budget_pressure {:.3} above maximum {:.3}",
                auto_replay_runtime_kv_budget_pressure, max_auto_replay_runtime_kv_budget_pressure
            ));
        }
    }

    if let Some(min_auto_replay_runtime_kv_weak_import_pressure_items) =
        gate.min_auto_replay_runtime_kv_weak_import_pressure_items
    {
        let auto_replay_runtime_kv_weak_import_pressure_items =
            summary.total_auto_replay_runtime_kv_weak_import_pressure_items();
        if auto_replay_runtime_kv_weak_import_pressure_items
            < min_auto_replay_runtime_kv_weak_import_pressure_items
        {
            failures.push(format!(
                "auto_replay_runtime_kv_weak_import_pressure_items {} below minimum {}",
                auto_replay_runtime_kv_weak_import_pressure_items,
                min_auto_replay_runtime_kv_weak_import_pressure_items
            ));
        }
    }

    if let Some(min_auto_replay_runtime_kv_weak_import_pressure) =
        gate.min_auto_replay_runtime_kv_weak_import_pressure
    {
        let auto_replay_runtime_kv_weak_import_pressure =
            summary.average_auto_replay_runtime_kv_weak_import_pressure();
        if auto_replay_runtime_kv_weak_import_pressure
            < min_auto_replay_runtime_kv_weak_import_pressure
        {
            failures.push(format!(
                "auto_replay_runtime_kv_weak_import_pressure {:.3} below minimum {:.3}",
                auto_replay_runtime_kv_weak_import_pressure,
                min_auto_replay_runtime_kv_weak_import_pressure
            ));
        }
    }

    if let Some(max_auto_replay_runtime_kv_weak_import_pressure) =
        gate.max_auto_replay_runtime_kv_weak_import_pressure
    {
        let auto_replay_runtime_kv_weak_import_pressure =
            summary.max_auto_replay_runtime_kv_weak_import_pressure();
        if auto_replay_runtime_kv_weak_import_pressure
            > max_auto_replay_runtime_kv_weak_import_pressure
        {
            failures.push(format!(
                "auto_replay_runtime_kv_weak_import_pressure {:.3} above maximum {:.3}",
                auto_replay_runtime_kv_weak_import_pressure,
                max_auto_replay_runtime_kv_weak_import_pressure
            ));
        }
    }
}
