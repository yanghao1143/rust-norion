use super::super::super::super::TRACE_FLOAT_EPSILON;
use super::super::context::AutoReplayTrace;

pub(super) fn evaluate_cumulative_consistency(
    failures: &mut Vec<String>,
    trace: &AutoReplayTrace,
) -> usize {
    let cumulative = &trace.cumulative;
    let feedback = &cumulative.replay_live_memory_feedback;
    let business = &cumulative.replay_business_contract;
    let live = &cumulative.replay_live_evolution;

    let expected_memory_updates = trace
        .memory_reinforcements
        .saturating_add(trace.memory_penalties);
    let expected_cumulative_memory_updates = cumulative
        .memory_reinforcements
        .saturating_add(cumulative.memory_penalties);
    if cumulative.memory_updates != expected_cumulative_memory_updates {
        failures.push(format!(
            "cumulative_memory_updates {} does not match cumulative_memory_reinforcements+cumulative_memory_penalties {expected_cumulative_memory_updates}",
            cumulative.memory_updates
        ));
    }

    let expected_cumulative_live_feedback_updates =
        feedback.reinforcements.saturating_add(feedback.penalties);
    if feedback.updates != expected_cumulative_live_feedback_updates {
        failures.push(format!(
            "cumulative_replay_live_memory_feedback_updates {} does not match cumulative replay live feedback components {expected_cumulative_live_feedback_updates}",
            feedback.updates
        ));
    }
    let cumulative_detailed_updates = feedback.applied.saturating_add(feedback.missing);
    if cumulative_detailed_updates > feedback.updates {
        failures.push(format!(
            "cumulative_replay_live_memory_feedback_applied+missing {cumulative_detailed_updates} exceeds cumulative_replay_live_memory_feedback_updates {}",
            feedback.updates
        ));
    }
    if feedback.removed > feedback.applied {
        failures.push(format!(
            "cumulative_replay_live_memory_feedback_removed {} exceeds cumulative_replay_live_memory_feedback_applied {}",
            feedback.removed, feedback.applied
        ));
    }
    if feedback.strength_delta < -TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "cumulative_replay_live_memory_feedback_strength_delta {:.6} is negative",
            feedback.strength_delta
        ));
    }

    let expected_business_items = business.passed.saturating_add(business.failed);
    if business.items != expected_business_items {
        failures.push(format!(
            "cumulative_replay_business_contract_items {} does not match cumulative replay business contract pass/fail components {expected_business_items}",
            business.items
        ));
    }
    let expected_business_raw_items = business.raw_passed.saturating_add(business.raw_failed);
    if business.items != expected_business_raw_items {
        failures.push(format!(
            "cumulative_replay_business_contract_items {} does not match cumulative replay business contract raw components {expected_business_raw_items}",
            business.items
        ));
    }
    let expected_business_normalized = business
        .sanitized
        .saturating_add(business.canonical_fallbacks);
    if business.response_normalized != expected_business_normalized {
        failures.push(format!(
            "cumulative_replay_business_contract_response_normalized {} does not match cumulative replay business contract normalization components {expected_business_normalized}",
            business.response_normalized
        ));
    }

    if live.router_threshold_mutations > live.items {
        failures.push(format!(
            "cumulative_replay_live_evolution_router_threshold_mutations {} exceeds cumulative_replay_live_evolution_items {}",
            live.router_threshold_mutations, live.items
        ));
    }
    if live.hierarchy_weight_mutations > live.items {
        failures.push(format!(
            "cumulative_replay_live_evolution_hierarchy_weight_mutations {} exceeds cumulative_replay_live_evolution_items {}",
            live.hierarchy_weight_mutations, live.items
        ));
    }
    if live.router_threshold_delta < -TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "cumulative_replay_live_evolution_router_threshold_delta {:.6} is negative",
            live.router_threshold_delta
        ));
    }
    if live.hierarchy_weight_delta < -TRACE_FLOAT_EPSILON {
        failures.push(format!(
            "cumulative_replay_live_evolution_hierarchy_weight_delta {:.6} is negative",
            live.hierarchy_weight_delta
        ));
    }

    expected_memory_updates
}
