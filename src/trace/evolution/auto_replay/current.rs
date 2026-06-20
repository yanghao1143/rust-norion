use super::super::super::TRACE_FLOAT_EPSILON;
use super::super::shared::{
    OnlineRewardStrength, check_online_reward_strength, require_usize_at_least,
};
use super::context::AutoReplayTrace;

pub(super) fn evaluate_current_trace(failures: &mut Vec<String>, trace: &AutoReplayTrace) {
    let feedback = &trace.live_memory_feedback;
    let business = &trace.business_contract;
    let live = &trace.live_evolution;

    let expected_live_feedback_updates = feedback.reinforcements.saturating_add(feedback.penalties);
    if feedback.updates != expected_live_feedback_updates {
        failures.push(format!(
            "auto_replay live_memory_feedback_updates {} does not match live_memory_feedback_reinforcements+live_memory_feedback_penalties {expected_live_feedback_updates}",
            feedback.updates
        ));
    }

    let expected_memory_updates = trace
        .memory_reinforcements
        .saturating_add(trace.memory_penalties);
    if trace.touched_memories != expected_memory_updates {
        failures.push(format!(
            "auto_replay touched_memories {} does not match memory_reinforcements+memory_penalties {expected_memory_updates}",
            trace.touched_memories
        ));
    }

    if trace.reinforced.saturating_add(trace.penalized) > trace.applied {
        failures.push(format!(
            "auto_replay reinforced+penalized {} exceeds applied {}",
            trace.reinforced.saturating_add(trace.penalized),
            trace.applied
        ));
    }
    if feedback.detail_items > feedback.items {
        failures.push(format!(
            "auto_replay live_memory_feedback_detail_items {} exceeds live_memory_feedback_items {}",
            feedback.detail_items, feedback.items
        ));
    }
    if feedback.removed > feedback.applied {
        failures.push(format!(
            "auto_replay live_memory_feedback_removed {} exceeds live_memory_feedback_applied {}",
            feedback.removed, feedback.applied
        ));
    }
    if feedback.detail_items == 0 {
        let detail_activity = feedback
            .applied
            .saturating_add(feedback.removed)
            .saturating_add(feedback.missing);
        if detail_activity > 0 {
            failures.push(format!(
                "auto_replay live_memory_feedback_detail_items 0 cannot carry detailed feedback activity {detail_activity}"
            ));
        }
        if feedback.strength_delta > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "auto_replay live_memory_feedback_strength_delta {:.6} requires live_memory_feedback_detail_items > 0",
                feedback.strength_delta
            ));
        }
    }
    if feedback.detail_items > 0 {
        let detailed_updates = feedback.applied.saturating_add(feedback.missing);
        if detailed_updates > feedback.updates {
            failures.push(format!(
                "auto_replay live_memory_feedback_applied+missing {detailed_updates} exceeds live_memory_feedback_updates {}",
                feedback.updates
            ));
        }
    }
    if feedback.strength_delta < -TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "auto_replay live_memory_feedback_strength_delta {:.6} is negative",
            feedback.strength_delta
        ));
    }

    let expected_business_contract_items = business.passed.saturating_add(business.failed);
    if business.items != expected_business_contract_items {
        failures.push(format!(
            "auto_replay business_contract_items {} does not match business_contract_passed+business_contract_failed {expected_business_contract_items}",
            business.items
        ));
    }
    let expected_business_contract_raw_items =
        business.raw_passed.saturating_add(business.raw_failed);
    if business.items != expected_business_contract_raw_items {
        failures.push(format!(
            "auto_replay business_contract_items {} does not match business_contract_raw_passed+business_contract_raw_failed {expected_business_contract_raw_items}",
            business.items
        ));
    }
    let expected_business_contract_normalized = business
        .sanitized
        .saturating_add(business.canonical_fallbacks);
    if business.response_normalized != expected_business_contract_normalized {
        failures.push(format!(
            "auto_replay business_contract_response_normalized {} does not match business_contract_sanitized+business_contract_canonical_fallbacks {expected_business_contract_normalized}",
            business.response_normalized
        ));
    }

    if live.router_threshold_mutations > live.items {
        failures.push(format!(
            "auto_replay live_evolution_router_threshold_mutations {} exceeds live_evolution_items {}",
            live.router_threshold_mutations, live.items
        ));
    }
    if live.hierarchy_weight_mutations > live.items {
        failures.push(format!(
            "auto_replay live_evolution_hierarchy_weight_mutations {} exceeds live_evolution_items {}",
            live.hierarchy_weight_mutations, live.items
        ));
    }
    if live.router_threshold_delta > TRACE_FLOAT_EPSILON && live.router_threshold_mutations == 0 {
        failures.push(format!(
            "auto_replay live_evolution_router_threshold_delta {:.6} requires live_evolution_router_threshold_mutations > 0",
            live.router_threshold_delta
        ));
    }
    if live.hierarchy_weight_delta > TRACE_FLOAT_EPSILON && live.hierarchy_weight_mutations == 0 {
        failures.push(format!(
            "auto_replay live_evolution_hierarchy_weight_delta {:.6} requires live_evolution_hierarchy_weight_mutations > 0",
            live.hierarchy_weight_delta
        ));
    }
    let expected_live_evolution_online_reward_feedbacks = live
        .online_reward_reinforcements
        .saturating_add(live.online_reward_penalties);
    if live.online_reward_feedbacks != expected_live_evolution_online_reward_feedbacks {
        failures.push(format!(
            "auto_replay live_evolution_online_reward_feedbacks {} does not match live_evolution_online_reward_reinforcements+live_evolution_online_reward_penalties {expected_live_evolution_online_reward_feedbacks}",
            live.online_reward_feedbacks
        ));
    }
    check_online_reward_strength(
        failures,
        "auto_replay live_evolution_online_reward",
        OnlineRewardStrength {
            feedbacks: live.online_reward_feedbacks,
            reinforcements: live.online_reward_reinforcements,
            penalties: live.online_reward_penalties,
            total_strength: live.online_reward_strength,
            reinforcement_strength: live.online_reward_reinforcement_strength,
            penalty_strength: live.online_reward_penalty_strength,
        },
    );

    let live_evolution_activity = live
        .router_threshold_mutations
        .saturating_add(live.hierarchy_weight_mutations)
        .saturating_add(live.online_reward_feedbacks)
        .saturating_add(live.memory_updates)
        .saturating_add(live.stored_memory_updates)
        .saturating_add(live.reflection_issues)
        .saturating_add(live.critical_reflection_issues)
        .saturating_add(live.revision_actions);
    if live.items == 0 && live_evolution_activity > 0 {
        failures.push(format!(
            "auto_replay live_evolution_items 0 cannot carry structured live evolution activity {live_evolution_activity}"
        ));
    }

    check_applied_consistency(failures, trace);
    check_control_plane_consistency(failures, trace);

    if trace.applied > 0 {
        require_usize_at_least(
            failures,
            "replay_runs",
            trace.replay_runs,
            "auto_replay_run",
            1,
        );
        require_usize_at_least(
            failures,
            "replay_items",
            trace.replay_items,
            "auto_replay applied",
            trace.applied,
        );
    }
}

fn check_applied_consistency(failures: &mut Vec<String>, trace: &AutoReplayTrace) {
    let feedback = &trace.live_memory_feedback;
    let business = &trace.business_contract;
    let live = &trace.live_evolution;
    let recursive = &trace.recursive_runtime;

    if trace.applied == 0 {
        for (name, value) in [
            ("router_updates", trace.router_updates),
            ("hierarchy_updates", trace.hierarchy_updates),
            (
                "router_threshold_mutations",
                trace.router_threshold_mutations,
            ),
            (
                "hierarchy_weight_mutations",
                trace.hierarchy_weight_mutations,
            ),
            ("reinforced", trace.reinforced),
            ("penalized", trace.penalized),
            ("touched_memories", trace.touched_memories),
            ("memory_reinforcements", trace.memory_reinforcements),
            ("memory_penalties", trace.memory_penalties),
            ("live_memory_feedback_items", feedback.items),
            ("live_memory_feedback_updates", feedback.updates),
            (
                "live_memory_feedback_reinforcements",
                feedback.reinforcements,
            ),
            ("live_memory_feedback_penalties", feedback.penalties),
            ("live_memory_feedback_detail_items", feedback.detail_items),
            ("live_memory_feedback_applied", feedback.applied),
            ("live_memory_feedback_removed", feedback.removed),
            ("live_memory_feedback_missing", feedback.missing),
            ("business_contract_items", business.items),
            ("business_contract_passed", business.passed),
            ("business_contract_failed", business.failed),
            ("business_contract_raw_passed", business.raw_passed),
            ("business_contract_raw_failed", business.raw_failed),
            (
                "business_contract_response_normalized",
                business.response_normalized,
            ),
            ("business_contract_sanitized", business.sanitized),
            (
                "business_contract_canonical_fallbacks",
                business.canonical_fallbacks,
            ),
            ("live_evolution_items", live.items),
            (
                "live_evolution_router_threshold_mutations",
                live.router_threshold_mutations,
            ),
            (
                "live_evolution_hierarchy_weight_mutations",
                live.hierarchy_weight_mutations,
            ),
            (
                "live_evolution_online_reward_feedbacks",
                live.online_reward_feedbacks,
            ),
            (
                "live_evolution_online_reward_reinforcements",
                live.online_reward_reinforcements,
            ),
            (
                "live_evolution_online_reward_penalties",
                live.online_reward_penalties,
            ),
            ("live_evolution_memory_updates", live.memory_updates),
            (
                "live_evolution_stored_memory_updates",
                live.stored_memory_updates,
            ),
            ("live_evolution_reflection_issues", live.reflection_issues),
            (
                "live_evolution_critical_reflection_issues",
                live.critical_reflection_issues,
            ),
            ("live_evolution_revision_actions", live.revision_actions),
            ("recursive_runtime_items", recursive.items),
            ("recursive_runtime_calls", recursive.calls),
        ] {
            if value > 0 {
                failures.push(format!("auto_replay {name} {value} requires applied > 0"));
            }
        }
        if trace.router_threshold_delta > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "auto_replay router_threshold_delta {:.6} requires applied > 0",
                trace.router_threshold_delta
            ));
        }
        if trace.hierarchy_weight_delta > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "auto_replay hierarchy_weight_delta {:.6} requires applied > 0",
                trace.hierarchy_weight_delta
            ));
        }
        if recursive.average_call_pressure > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "auto_replay avg_recursive_call_pressure {:.6} requires applied > 0",
                recursive.average_call_pressure
            ));
        }
        if recursive.max_call_pressure > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "auto_replay max_recursive_call_pressure {:.6} requires applied > 0",
                recursive.max_call_pressure
            ));
        }
        if feedback.strength_delta > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "auto_replay live_memory_feedback_strength_delta {:.6} requires applied > 0",
                feedback.strength_delta
            ));
        }
        if live.router_threshold_delta > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "auto_replay live_evolution_router_threshold_delta {:.6} requires applied > 0",
                live.router_threshold_delta
            ));
        }
        if live.hierarchy_weight_delta > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "auto_replay live_evolution_hierarchy_weight_delta {:.6} requires applied > 0",
                live.hierarchy_weight_delta
            ));
        }
        if live.online_reward_strength > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "auto_replay live_evolution_online_reward_strength {:.6} requires applied > 0",
                live.online_reward_strength
            ));
        }
    } else {
        if trace.router_updates != trace.applied {
            failures.push(format!(
                "auto_replay router_updates {} does not match applied {}",
                trace.router_updates, trace.applied
            ));
        }
        if trace.hierarchy_updates != trace.applied {
            failures.push(format!(
                "auto_replay hierarchy_updates {} does not match applied {}",
                trace.hierarchy_updates, trace.applied
            ));
        }
        if feedback.items > trace.applied {
            failures.push(format!(
                "auto_replay live_memory_feedback_items {} exceeds applied {}",
                feedback.items, trace.applied
            ));
        }
        if live.items > trace.applied {
            failures.push(format!(
                "auto_replay live_evolution_items {} exceeds applied {}",
                live.items, trace.applied
            ));
        }
        if business.items > trace.applied {
            failures.push(format!(
                "auto_replay business_contract_items {} exceeds applied {}",
                business.items, trace.applied
            ));
        }
        if live.online_reward_feedbacks > live.items {
            failures.push(format!(
                "auto_replay live_evolution_online_reward_feedbacks {} exceeds live_evolution_items {}",
                live.online_reward_feedbacks, live.items
            ));
        }
        if recursive.items > trace.applied {
            failures.push(format!(
                "auto_replay recursive_runtime_items {} exceeds applied {}",
                recursive.items, trace.applied
            ));
        }
    }
}

fn check_control_plane_consistency(failures: &mut Vec<String>, trace: &AutoReplayTrace) {
    let recursive = &trace.recursive_runtime;

    if trace.router_threshold_mutations > trace.router_updates {
        failures.push(format!(
            "auto_replay router_threshold_mutations {} exceeds router_updates {}",
            trace.router_threshold_mutations, trace.router_updates
        ));
    }
    if trace.hierarchy_weight_mutations > trace.hierarchy_updates {
        failures.push(format!(
            "auto_replay hierarchy_weight_mutations {} exceeds hierarchy_updates {}",
            trace.hierarchy_weight_mutations, trace.hierarchy_updates
        ));
    }
    if trace.router_threshold_delta > TRACE_FLOAT_EPSILON && trace.router_threshold_mutations == 0 {
        failures.push(format!(
            "auto_replay router_threshold_delta {:.6} requires router_threshold_mutations > 0",
            trace.router_threshold_delta
        ));
    }
    if trace.hierarchy_weight_delta > TRACE_FLOAT_EPSILON && trace.hierarchy_weight_mutations == 0 {
        failures.push(format!(
            "auto_replay hierarchy_weight_delta {:.6} requires hierarchy_weight_mutations > 0",
            trace.hierarchy_weight_delta
        ));
    }
    if recursive.calls > 0 && recursive.items == 0 {
        failures.push(format!(
            "auto_replay recursive_runtime_calls {} requires recursive_runtime_items > 0",
            recursive.calls
        ));
    }
    if recursive.average_call_pressure < -TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "auto_replay avg_recursive_call_pressure {:.6} is negative",
            recursive.average_call_pressure
        ));
    }
    if recursive.max_call_pressure < -TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "auto_replay max_recursive_call_pressure {:.6} is negative",
            recursive.max_call_pressure
        ));
    }
    if recursive.average_call_pressure > recursive.max_call_pressure + TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "auto_replay avg_recursive_call_pressure {:.6} exceeds max_recursive_call_pressure {:.6}",
            recursive.average_call_pressure, recursive.max_call_pressure
        ));
    }
}
