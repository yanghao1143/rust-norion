use super::super::EvolutionLedger;

const LEGACY_REPLAY_LENGTHS: &[usize] = &[10, 13, 16];
const LEGACY_LIVE_LENGTHS: &[usize] = &[29, 34, 44];
const CURRENT_LENGTHS: &[usize] = &[50, 56, 63, 71, 79];

pub(super) fn serialize_evolution_ledger(ledger: EvolutionLedger) -> String {
    [
        ledger.live_inference_runs.to_string(),
        ledger.live_router_threshold_mutations.to_string(),
        ledger.live_hierarchy_weight_mutations.to_string(),
        format!("{:.6}", ledger.live_router_threshold_delta),
        format!("{:.6}", ledger.live_hierarchy_weight_delta),
        ledger.live_online_reward_feedbacks.to_string(),
        ledger.live_online_reward_reinforcements.to_string(),
        ledger.live_online_reward_penalties.to_string(),
        ledger.live_memory_reinforcements.to_string(),
        ledger.live_memory_penalties.to_string(),
        ledger.live_stored_memories.to_string(),
        ledger.live_stored_gist_memories.to_string(),
        ledger.live_stored_runtime_kv_memories.to_string(),
        ledger.live_reflection_issues.to_string(),
        ledger.live_critical_reflection_issues.to_string(),
        ledger.live_revision_actions.to_string(),
        ledger.replay_runs.to_string(),
        ledger.replay_items.to_string(),
        ledger.router_threshold_mutations.to_string(),
        ledger.hierarchy_weight_mutations.to_string(),
        format!("{:.6}", ledger.router_threshold_delta),
        format!("{:.6}", ledger.hierarchy_weight_delta),
        ledger.memory_reinforcements.to_string(),
        ledger.memory_penalties.to_string(),
        ledger.replay_live_memory_feedback_items.to_string(),
        ledger
            .replay_live_memory_feedback_reinforcements
            .to_string(),
        ledger.replay_live_memory_feedback_penalties.to_string(),
        ledger.replay_live_memory_feedback_detail_items.to_string(),
        ledger.replay_live_memory_feedback_applied.to_string(),
        ledger.replay_live_memory_feedback_removed.to_string(),
        ledger.replay_live_memory_feedback_missing.to_string(),
        format!("{:.6}", ledger.replay_live_memory_feedback_strength_delta),
        ledger.replay_live_evolution_items.to_string(),
        ledger
            .replay_live_evolution_router_threshold_mutations
            .to_string(),
        ledger
            .replay_live_evolution_hierarchy_weight_mutations
            .to_string(),
        format!("{:.6}", ledger.replay_live_evolution_router_threshold_delta),
        format!("{:.6}", ledger.replay_live_evolution_hierarchy_weight_delta),
        ledger
            .replay_live_evolution_online_reward_feedbacks
            .to_string(),
        ledger
            .replay_live_evolution_online_reward_reinforcements
            .to_string(),
        ledger
            .replay_live_evolution_online_reward_penalties
            .to_string(),
        ledger.replay_live_evolution_memory_updates.to_string(),
        ledger
            .replay_live_evolution_stored_memory_updates
            .to_string(),
        ledger.replay_live_evolution_reflection_issues.to_string(),
        ledger
            .replay_live_evolution_critical_reflection_issues
            .to_string(),
        ledger.replay_live_evolution_revision_actions.to_string(),
        ledger.recursive_replay_items.to_string(),
        ledger.recursive_runtime_calls.to_string(),
        ledger.drift_rollbacks.to_string(),
        format!("{:.6}", ledger.rollback_router_threshold_delta),
        format!("{:.6}", ledger.rollback_hierarchy_weight_delta),
        format!("{:.6}", ledger.live_online_reward_strength),
        format!("{:.6}", ledger.live_online_reward_reinforcement_strength),
        format!("{:.6}", ledger.live_online_reward_penalty_strength),
        format!("{:.6}", ledger.replay_live_evolution_online_reward_strength),
        format!(
            "{:.6}",
            ledger.replay_live_evolution_online_reward_reinforcement_strength
        ),
        format!(
            "{:.6}",
            ledger.replay_live_evolution_online_reward_penalty_strength
        ),
        ledger.external_feedbacks.to_string(),
        ledger.external_feedback_reinforcements.to_string(),
        ledger.external_feedback_penalties.to_string(),
        ledger.external_feedback_memory_updates.to_string(),
        ledger.external_feedback_removed.to_string(),
        ledger.external_feedback_missing.to_string(),
        format!("{:.6}", ledger.external_feedback_strength_delta),
        ledger.replay_rust_check_items.to_string(),
        ledger.replay_rust_check_passed.to_string(),
        ledger.replay_rust_check_failed.to_string(),
        ledger.replay_rust_check_diagnostic_chars.to_string(),
        ledger
            .replay_rust_check_live_memory_feedback_items
            .to_string(),
        ledger
            .replay_rust_check_live_memory_feedback_updates
            .to_string(),
        ledger
            .replay_rust_check_live_memory_feedback_applied
            .to_string(),
        format!(
            "{:.6}",
            ledger.replay_rust_check_live_memory_feedback_strength_delta
        ),
        ledger.replay_business_contract_items.to_string(),
        ledger.replay_business_contract_passed.to_string(),
        ledger.replay_business_contract_failed.to_string(),
        ledger.replay_business_contract_raw_passed.to_string(),
        ledger.replay_business_contract_raw_failed.to_string(),
        ledger
            .replay_business_contract_response_normalized
            .to_string(),
        ledger.replay_business_contract_sanitized.to_string(),
        ledger
            .replay_business_contract_canonical_fallbacks
            .to_string(),
    ]
    .join("\t")
}

pub(in crate::adaptive_state) fn parse_evolution_ledger(value: &str) -> Option<EvolutionLedger> {
    let fields = value.split('\t').collect::<Vec<_>>();
    match fields.len() {
        len if CURRENT_LENGTHS.contains(&len) => parse_current_ledger(&fields),
        len if LEGACY_LIVE_LENGTHS.contains(&len) => parse_legacy_live_ledger(&fields),
        len if LEGACY_REPLAY_LENGTHS.contains(&len) => parse_legacy_replay_ledger(&fields),
        _ => None,
    }
}

fn parse_current_ledger(fields: &[&str]) -> Option<EvolutionLedger> {
    let mut ledger = EvolutionLedger {
        live_inference_runs: u64_at(fields, 0)?,
        live_router_threshold_mutations: u64_at(fields, 1)?,
        live_hierarchy_weight_mutations: u64_at(fields, 2)?,
        live_router_threshold_delta: f32_at(fields, 3)?,
        live_hierarchy_weight_delta: f32_at(fields, 4)?,
        live_online_reward_feedbacks: u64_at(fields, 5)?,
        live_online_reward_reinforcements: u64_at(fields, 6)?,
        live_online_reward_penalties: u64_at(fields, 7)?,
        live_memory_reinforcements: u64_at(fields, 8)?,
        live_memory_penalties: u64_at(fields, 9)?,
        live_stored_memories: u64_at(fields, 10)?,
        live_stored_gist_memories: u64_at(fields, 11)?,
        live_stored_runtime_kv_memories: u64_at(fields, 12)?,
        live_reflection_issues: u64_at(fields, 13)?,
        live_critical_reflection_issues: u64_at(fields, 14)?,
        live_revision_actions: u64_at(fields, 15)?,
        replay_runs: u64_at(fields, 16)?,
        replay_items: u64_at(fields, 17)?,
        router_threshold_mutations: u64_at(fields, 18)?,
        hierarchy_weight_mutations: u64_at(fields, 19)?,
        router_threshold_delta: f32_at(fields, 20)?,
        hierarchy_weight_delta: f32_at(fields, 21)?,
        memory_reinforcements: u64_at(fields, 22)?,
        memory_penalties: u64_at(fields, 23)?,
        replay_live_memory_feedback_items: u64_at(fields, 24)?,
        replay_live_memory_feedback_reinforcements: u64_at(fields, 25)?,
        replay_live_memory_feedback_penalties: u64_at(fields, 26)?,
        replay_live_memory_feedback_detail_items: u64_at(fields, 27)?,
        replay_live_memory_feedback_applied: u64_at(fields, 28)?,
        replay_live_memory_feedback_removed: u64_at(fields, 29)?,
        replay_live_memory_feedback_missing: u64_at(fields, 30)?,
        replay_live_memory_feedback_strength_delta: f32_at(fields, 31)?,
        replay_live_evolution_items: u64_at(fields, 32)?,
        replay_live_evolution_router_threshold_mutations: u64_at(fields, 33)?,
        replay_live_evolution_hierarchy_weight_mutations: u64_at(fields, 34)?,
        replay_live_evolution_router_threshold_delta: f32_at(fields, 35)?,
        replay_live_evolution_hierarchy_weight_delta: f32_at(fields, 36)?,
        replay_live_evolution_online_reward_feedbacks: u64_at(fields, 37)?,
        replay_live_evolution_online_reward_reinforcements: u64_at(fields, 38)?,
        replay_live_evolution_online_reward_penalties: u64_at(fields, 39)?,
        replay_live_evolution_memory_updates: u64_at(fields, 40)?,
        replay_live_evolution_stored_memory_updates: u64_at(fields, 41)?,
        replay_live_evolution_reflection_issues: u64_at(fields, 42)?,
        replay_live_evolution_critical_reflection_issues: u64_at(fields, 43)?,
        replay_live_evolution_revision_actions: u64_at(fields, 44)?,
        recursive_replay_items: u64_at(fields, 45)?,
        recursive_runtime_calls: u64_at(fields, 46)?,
        drift_rollbacks: u64_at(fields, 47)?,
        rollback_router_threshold_delta: f32_at(fields, 48)?,
        rollback_hierarchy_weight_delta: f32_at(fields, 49)?,
        ..EvolutionLedger::default()
    };

    if fields.len() >= 56 {
        ledger.live_online_reward_strength = f32_at(fields, 50)?;
        ledger.live_online_reward_reinforcement_strength = f32_at(fields, 51)?;
        ledger.live_online_reward_penalty_strength = f32_at(fields, 52)?;
        ledger.replay_live_evolution_online_reward_strength = f32_at(fields, 53)?;
        ledger.replay_live_evolution_online_reward_reinforcement_strength = f32_at(fields, 54)?;
        ledger.replay_live_evolution_online_reward_penalty_strength = f32_at(fields, 55)?;
    }
    if fields.len() >= 63 {
        ledger.external_feedbacks = u64_at(fields, 56)?;
        ledger.external_feedback_reinforcements = u64_at(fields, 57)?;
        ledger.external_feedback_penalties = u64_at(fields, 58)?;
        ledger.external_feedback_memory_updates = u64_at(fields, 59)?;
        ledger.external_feedback_removed = u64_at(fields, 60)?;
        ledger.external_feedback_missing = u64_at(fields, 61)?;
        ledger.external_feedback_strength_delta = f32_at(fields, 62)?;
    }
    if fields.len() >= 71 {
        ledger.replay_rust_check_items = u64_at(fields, 63)?;
        ledger.replay_rust_check_passed = u64_at(fields, 64)?;
        ledger.replay_rust_check_failed = u64_at(fields, 65)?;
        ledger.replay_rust_check_diagnostic_chars = u64_at(fields, 66)?;
        ledger.replay_rust_check_live_memory_feedback_items = u64_at(fields, 67)?;
        ledger.replay_rust_check_live_memory_feedback_updates = u64_at(fields, 68)?;
        ledger.replay_rust_check_live_memory_feedback_applied = u64_at(fields, 69)?;
        ledger.replay_rust_check_live_memory_feedback_strength_delta = f32_at(fields, 70)?;
    }
    if fields.len() == 79 {
        ledger.replay_business_contract_items = u64_at(fields, 71)?;
        ledger.replay_business_contract_passed = u64_at(fields, 72)?;
        ledger.replay_business_contract_failed = u64_at(fields, 73)?;
        ledger.replay_business_contract_raw_passed = u64_at(fields, 74)?;
        ledger.replay_business_contract_raw_failed = u64_at(fields, 75)?;
        ledger.replay_business_contract_response_normalized = u64_at(fields, 76)?;
        ledger.replay_business_contract_sanitized = u64_at(fields, 77)?;
        ledger.replay_business_contract_canonical_fallbacks = u64_at(fields, 78)?;
    }

    Some(ledger)
}

fn parse_legacy_live_ledger(fields: &[&str]) -> Option<EvolutionLedger> {
    let has_memory_feedback_detail = fields.len() >= 34;
    let has_live_evolution = fields.len() == 44;
    let live_evolution_index = has_live_evolution.then_some(29);
    let recursive_replay_index = if has_live_evolution {
        39
    } else if has_memory_feedback_detail {
        29
    } else {
        24
    };

    Some(EvolutionLedger {
        live_inference_runs: u64_at(fields, 0)?,
        live_router_threshold_mutations: u64_at(fields, 1)?,
        live_hierarchy_weight_mutations: u64_at(fields, 2)?,
        live_router_threshold_delta: f32_at(fields, 3)?,
        live_hierarchy_weight_delta: f32_at(fields, 4)?,
        live_memory_reinforcements: u64_at(fields, 5)?,
        live_memory_penalties: u64_at(fields, 6)?,
        live_stored_memories: u64_at(fields, 7)?,
        live_stored_gist_memories: u64_at(fields, 8)?,
        live_stored_runtime_kv_memories: u64_at(fields, 9)?,
        live_reflection_issues: u64_at(fields, 10)?,
        live_critical_reflection_issues: u64_at(fields, 11)?,
        live_revision_actions: u64_at(fields, 12)?,
        replay_runs: u64_at(fields, 13)?,
        replay_items: u64_at(fields, 14)?,
        router_threshold_mutations: u64_at(fields, 15)?,
        hierarchy_weight_mutations: u64_at(fields, 16)?,
        router_threshold_delta: f32_at(fields, 17)?,
        hierarchy_weight_delta: f32_at(fields, 18)?,
        memory_reinforcements: u64_at(fields, 19)?,
        memory_penalties: u64_at(fields, 20)?,
        replay_live_memory_feedback_items: u64_at(fields, 21)?,
        replay_live_memory_feedback_reinforcements: u64_at(fields, 22)?,
        replay_live_memory_feedback_penalties: u64_at(fields, 23)?,
        replay_live_memory_feedback_detail_items: optional_u64(
            fields,
            24,
            has_memory_feedback_detail,
        )?,
        replay_live_memory_feedback_applied: optional_u64(fields, 25, has_memory_feedback_detail)?,
        replay_live_memory_feedback_removed: optional_u64(fields, 26, has_memory_feedback_detail)?,
        replay_live_memory_feedback_missing: optional_u64(fields, 27, has_memory_feedback_detail)?,
        replay_live_memory_feedback_strength_delta: optional_f32(
            fields,
            28,
            has_memory_feedback_detail,
        )?,
        replay_live_evolution_items: optional_indexed_u64(fields, live_evolution_index, 0),
        replay_live_evolution_router_threshold_mutations: optional_indexed_u64(
            fields,
            live_evolution_index,
            1,
        ),
        replay_live_evolution_hierarchy_weight_mutations: optional_indexed_u64(
            fields,
            live_evolution_index,
            2,
        ),
        replay_live_evolution_router_threshold_delta: optional_indexed_f32(
            fields,
            live_evolution_index,
            3,
        ),
        replay_live_evolution_hierarchy_weight_delta: optional_indexed_f32(
            fields,
            live_evolution_index,
            4,
        ),
        replay_live_evolution_memory_updates: optional_indexed_u64(fields, live_evolution_index, 5),
        replay_live_evolution_stored_memory_updates: optional_indexed_u64(
            fields,
            live_evolution_index,
            6,
        ),
        replay_live_evolution_reflection_issues: optional_indexed_u64(
            fields,
            live_evolution_index,
            7,
        ),
        replay_live_evolution_critical_reflection_issues: optional_indexed_u64(
            fields,
            live_evolution_index,
            8,
        ),
        replay_live_evolution_revision_actions: optional_indexed_u64(
            fields,
            live_evolution_index,
            9,
        ),
        recursive_replay_items: u64_at(fields, recursive_replay_index)?,
        recursive_runtime_calls: u64_at(fields, recursive_replay_index + 1)?,
        drift_rollbacks: u64_at(fields, recursive_replay_index + 2)?,
        rollback_router_threshold_delta: f32_at(fields, recursive_replay_index + 3)?,
        rollback_hierarchy_weight_delta: f32_at(fields, recursive_replay_index + 4)?,
        ..EvolutionLedger::default()
    })
}

fn parse_legacy_replay_ledger(fields: &[&str]) -> Option<EvolutionLedger> {
    let has_feedback_counts = fields.len() == 16;
    let recursive_replay_index = if has_feedback_counts { 11 } else { 8 };
    let rollback_index = recursive_replay_index + 2;

    Some(EvolutionLedger {
        replay_runs: u64_at(fields, 0)?,
        replay_items: u64_at(fields, 1)?,
        router_threshold_mutations: u64_at(fields, 2)?,
        hierarchy_weight_mutations: u64_at(fields, 3)?,
        router_threshold_delta: f32_at(fields, 4)?,
        hierarchy_weight_delta: f32_at(fields, 5)?,
        memory_reinforcements: u64_at(fields, 6)?,
        memory_penalties: u64_at(fields, 7)?,
        replay_live_memory_feedback_items: optional_u64(fields, 8, has_feedback_counts)?,
        replay_live_memory_feedback_reinforcements: optional_u64(fields, 9, has_feedback_counts)?,
        replay_live_memory_feedback_penalties: optional_u64(fields, 10, has_feedback_counts)?,
        recursive_replay_items: u64_at(fields, recursive_replay_index)?,
        recursive_runtime_calls: u64_at(fields, recursive_replay_index + 1)?,
        drift_rollbacks: optional_trailing_u64(fields, rollback_index),
        rollback_router_threshold_delta: optional_trailing_f32(fields, rollback_index + 1),
        rollback_hierarchy_weight_delta: optional_trailing_f32(fields, rollback_index + 2),
        ..EvolutionLedger::default()
    })
}

fn u64_at(fields: &[&str], index: usize) -> Option<u64> {
    fields.get(index)?.parse::<u64>().ok()
}

fn f32_at(fields: &[&str], index: usize) -> Option<f32> {
    parse_nonnegative_f32(fields.get(index)?)
}

fn optional_u64(fields: &[&str], index: usize, enabled: bool) -> Option<u64> {
    if enabled {
        u64_at(fields, index)
    } else {
        Some(0)
    }
}

fn optional_f32(fields: &[&str], index: usize, enabled: bool) -> Option<f32> {
    if enabled {
        f32_at(fields, index)
    } else {
        Some(0.0)
    }
}

fn optional_indexed_u64(fields: &[&str], base: Option<usize>, offset: usize) -> u64 {
    base.and_then(|index| u64_at(fields, index + offset))
        .unwrap_or(0)
}

fn optional_indexed_f32(fields: &[&str], base: Option<usize>, offset: usize) -> f32 {
    base.and_then(|index| f32_at(fields, index + offset))
        .unwrap_or(0.0)
}

fn optional_trailing_u64(fields: &[&str], index: usize) -> u64 {
    fields
        .get(index)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

fn optional_trailing_f32(fields: &[&str], index: usize) -> f32 {
    fields
        .get(index)
        .and_then(|value| parse_nonnegative_f32(value))
        .unwrap_or(0.0)
}

fn parse_nonnegative_f32(value: &str) -> Option<f32> {
    value
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
        .map(|value| value.max(0.0))
}
