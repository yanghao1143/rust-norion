use super::*;
use crate::disk_kv::DiskKvStore;
use crate::hierarchy::{
    HierarchyState, HierarchyWeights, ProfileHierarchyObservations, ProfileHierarchyWeights,
};
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::router::{ProfileObservations, ProfileThresholds, RouterState};
use crate::tiered_cache::{MemoryPlacement, MemoryTier, TieredCachePlan};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn adaptive_state_roundtrips_through_disk_kv() {
    let path = temp_path("adaptive-state");
    let state = AdaptiveState {
        router: RouterState {
            threshold: 0.61,
            observations: 17,
            profile_thresholds: ProfileThresholds {
                general: 0.61,
                coding: 0.49,
                writing: 0.66,
                long_document: 0.42,
            },
            profile_observations: ProfileObservations {
                general: 8,
                coding: 5,
                writing: 3,
                long_document: 1,
            },
        },
        hierarchy: HierarchyState {
            current: HierarchyWeights::new(0.2, 0.6, 0.2),
            profile_weights: ProfileHierarchyWeights {
                general: HierarchyWeights::new(0.36, 0.42, 0.22),
                coding: HierarchyWeights::new(0.18, 0.68, 0.14),
                writing: HierarchyWeights::new(0.60, 0.26, 0.14),
                long_document: HierarchyWeights::new(0.24, 0.18, 0.58),
            },
            profile_observations: ProfileHierarchyObservations {
                general: 2,
                coding: 7,
                writing: 5,
                long_document: 3,
            },
        },
        tier_plan: TieredCachePlan::new(vec![MemoryPlacement {
            id: 7,
            tier: MemoryTier::WarmRam,
            score: 0.42,
            reason: "warm\tstate".to_owned(),
        }]),
        memory_retention_policy: MemoryRetentionPolicy {
            stale_after: 11,
            decay_rate: 0.12,
            remove_below_strength: 0.08,
            remove_after_failures: 7,
        },
        memory_compaction_policy: MemoryCompactionPolicy {
            similarity_threshold: 0.91,
            max_candidates: 64,
            max_merges: 4,
        },
        evolution_ledger: EvolutionLedger {
            live_inference_runs: 11,
            live_router_threshold_mutations: 8,
            live_hierarchy_weight_mutations: 6,
            live_router_threshold_delta: 0.19,
            live_hierarchy_weight_delta: 0.13,
            live_online_reward_feedbacks: 6,
            live_online_reward_reinforcements: 4,
            live_online_reward_penalties: 2,
            live_online_reward_strength: 3.25,
            live_online_reward_reinforcement_strength: 2.15,
            live_online_reward_penalty_strength: 1.10,
            live_memory_reinforcements: 9,
            live_memory_penalties: 4,
            live_stored_memories: 3,
            live_stored_gist_memories: 5,
            live_stored_runtime_kv_memories: 2,
            live_reflection_issues: 7,
            live_critical_reflection_issues: 1,
            live_revision_actions: 10,
            replay_runs: 3,
            replay_items: 9,
            router_threshold_mutations: 5,
            hierarchy_weight_mutations: 7,
            router_threshold_delta: 0.42,
            hierarchy_weight_delta: 0.21,
            memory_reinforcements: 4,
            memory_penalties: 2,
            replay_live_memory_feedback_items: 3,
            replay_live_memory_feedback_reinforcements: 5,
            replay_live_memory_feedback_penalties: 1,
            replay_live_memory_feedback_detail_items: 2,
            replay_live_memory_feedback_applied: 4,
            replay_live_memory_feedback_removed: 1,
            replay_live_memory_feedback_missing: 1,
            replay_live_memory_feedback_strength_delta: 0.72,
            replay_rust_check_items: 2,
            replay_rust_check_passed: 2,
            replay_rust_check_failed: 0,
            replay_rust_check_diagnostic_chars: 17,
            replay_rust_check_live_memory_feedback_items: 2,
            replay_rust_check_live_memory_feedback_updates: 5,
            replay_rust_check_live_memory_feedback_applied: 4,
            replay_rust_check_live_memory_feedback_strength_delta: 0.68,
            replay_business_contract_items: 3,
            replay_business_contract_passed: 3,
            replay_business_contract_failed: 0,
            replay_business_contract_raw_passed: 1,
            replay_business_contract_raw_failed: 2,
            replay_business_contract_response_normalized: 2,
            replay_business_contract_sanitized: 0,
            replay_business_contract_canonical_fallbacks: 2,
            replay_live_evolution_items: 4,
            replay_live_evolution_router_threshold_mutations: 2,
            replay_live_evolution_hierarchy_weight_mutations: 1,
            replay_live_evolution_router_threshold_delta: 0.08,
            replay_live_evolution_hierarchy_weight_delta: 0.05,
            replay_live_evolution_online_reward_feedbacks: 3,
            replay_live_evolution_online_reward_reinforcements: 2,
            replay_live_evolution_online_reward_penalties: 1,
            replay_live_evolution_online_reward_strength: 1.75,
            replay_live_evolution_online_reward_reinforcement_strength: 1.20,
            replay_live_evolution_online_reward_penalty_strength: 0.55,
            replay_live_evolution_memory_updates: 6,
            replay_live_evolution_stored_memory_updates: 3,
            replay_live_evolution_reflection_issues: 5,
            replay_live_evolution_critical_reflection_issues: 1,
            replay_live_evolution_revision_actions: 4,
            recursive_replay_items: 1,
            recursive_runtime_calls: 8,
            drift_rollbacks: 2,
            rollback_router_threshold_delta: 0.11,
            rollback_hierarchy_weight_delta: 0.09,
            external_feedbacks: 2,
            external_feedback_reinforcements: 3,
            external_feedback_penalties: 1,
            external_feedback_memory_updates: 4,
            external_feedback_removed: 1,
            external_feedback_missing: 2,
            external_feedback_strength_delta: 0.31,
        },
    };

    state.save_to_disk_kv(&path).unwrap();
    let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

    assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
    assert_eq!(loaded.router.observations, 17);
    assert!((loaded.router.profile_thresholds.coding - 0.49).abs() < 0.0001);
    assert_eq!(loaded.router.profile_observations.writing, 3);
    assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
    assert!((loaded.hierarchy.profile_weights.coding.local - 0.68).abs() < 0.0001);
    assert_eq!(loaded.hierarchy.profile_observations.long_document, 3);
    let placement = loaded.tier_plan.placement_for(7).unwrap();
    assert_eq!(placement.tier, MemoryTier::WarmRam);
    assert_eq!(placement.reason, "warm\tstate");
    assert_eq!(loaded.memory_retention_policy.stale_after, 11);
    assert!((loaded.memory_retention_policy.decay_rate - 0.12).abs() < 0.0001);
    assert!((loaded.memory_retention_policy.remove_below_strength - 0.08).abs() < 0.0001);
    assert_eq!(loaded.memory_retention_policy.remove_after_failures, 7);
    assert!((loaded.memory_compaction_policy.similarity_threshold - 0.91).abs() < 0.0001);
    assert_eq!(loaded.memory_compaction_policy.max_candidates, 64);
    assert_eq!(loaded.memory_compaction_policy.max_merges, 4);
    assert_eq!(loaded.evolution_ledger.replay_runs, 3);
    assert_eq!(loaded.evolution_ledger.live_inference_runs, 11);
    assert_eq!(loaded.evolution_ledger.live_router_threshold_mutations, 8);
    assert_eq!(loaded.evolution_ledger.live_hierarchy_weight_mutations, 6);
    assert!((loaded.evolution_ledger.live_router_threshold_delta - 0.19).abs() < 0.0001);
    assert!((loaded.evolution_ledger.live_hierarchy_weight_delta - 0.13).abs() < 0.0001);
    assert_eq!(loaded.evolution_ledger.live_online_reward_feedbacks, 6);
    assert_eq!(loaded.evolution_ledger.live_online_reward_reinforcements, 4);
    assert_eq!(loaded.evolution_ledger.live_online_reward_penalties, 2);
    assert!((loaded.evolution_ledger.live_online_reward_strength - 3.25).abs() < 0.0001);
    assert!(
        (loaded
            .evolution_ledger
            .live_online_reward_reinforcement_strength
            - 2.15)
            .abs()
            < 0.0001
    );
    assert!((loaded.evolution_ledger.live_online_reward_penalty_strength - 1.10).abs() < 0.0001);
    assert_eq!(loaded.evolution_ledger.live_memory_updates(), 13);
    assert_eq!(loaded.evolution_ledger.live_stored_memory_updates(), 10);
    assert_eq!(loaded.evolution_ledger.live_reflection_issues, 7);
    assert_eq!(loaded.evolution_ledger.live_critical_reflection_issues, 1);
    assert_eq!(loaded.evolution_ledger.live_revision_actions, 10);
    assert_eq!(loaded.evolution_ledger.replay_items, 9);
    assert_eq!(loaded.evolution_ledger.router_threshold_mutations, 5);
    assert_eq!(loaded.evolution_ledger.hierarchy_weight_mutations, 7);
    assert!((loaded.evolution_ledger.router_threshold_delta - 0.42).abs() < 0.0001);
    assert!((loaded.evolution_ledger.hierarchy_weight_delta - 0.21).abs() < 0.0001);
    assert_eq!(loaded.evolution_ledger.memory_updates(), 6);
    assert_eq!(loaded.evolution_ledger.replay_live_memory_feedback_items, 3);
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_memory_feedback_updates(),
        6
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_memory_feedback_reinforcements,
        5
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_memory_feedback_penalties,
        1
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_memory_feedback_detail_items,
        2
    );
    assert_eq!(
        loaded.evolution_ledger.replay_live_memory_feedback_applied,
        4
    );
    assert_eq!(
        loaded.evolution_ledger.replay_live_memory_feedback_removed,
        1
    );
    assert_eq!(
        loaded.evolution_ledger.replay_live_memory_feedback_missing,
        1
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_memory_feedback_detail_updates(),
        5
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_memory_feedback_strength_delta
            - 0.72)
            .abs()
            < 0.0001
    );
    assert_eq!(loaded.evolution_ledger.replay_rust_check_items, 2);
    assert_eq!(loaded.evolution_ledger.replay_rust_check_passed, 2);
    assert_eq!(loaded.evolution_ledger.replay_rust_check_failed, 0);
    assert_eq!(loaded.evolution_ledger.replay_rust_check_total(), 2);
    assert_eq!(
        loaded.evolution_ledger.replay_rust_check_diagnostic_chars,
        17
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_items,
        2
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_updates,
        5
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_applied,
        4
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_strength_delta
            - 0.68)
            .abs()
            < 0.0001
    );
    assert_eq!(loaded.evolution_ledger.replay_business_contract_items, 3);
    assert_eq!(loaded.evolution_ledger.replay_business_contract_passed, 3);
    assert_eq!(loaded.evolution_ledger.replay_business_contract_failed, 0);
    assert_eq!(loaded.evolution_ledger.replay_business_contract_total(), 3);
    assert_eq!(
        loaded.evolution_ledger.replay_business_contract_raw_passed,
        1
    );
    assert_eq!(
        loaded.evolution_ledger.replay_business_contract_raw_failed,
        2
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_business_contract_response_normalized,
        2
    );
    assert_eq!(
        loaded.evolution_ledger.replay_business_contract_sanitized,
        0
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_business_contract_canonical_fallbacks,
        2
    );
    assert_eq!(loaded.evolution_ledger.replay_live_evolution_items, 4);
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_router_threshold_mutations,
        2
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_hierarchy_weight_mutations,
        1
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_evolution_router_threshold_delta
            - 0.08)
            .abs()
            < 0.0001
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_evolution_hierarchy_weight_delta
            - 0.05)
            .abs()
            < 0.0001
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_feedbacks,
        3
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_reinforcements,
        2
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_penalties,
        1
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_strength
            - 1.75)
            .abs()
            < 0.0001
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_reinforcement_strength
            - 1.20)
            .abs()
            < 0.0001
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_penalty_strength
            - 0.55)
            .abs()
            < 0.0001
    );
    assert_eq!(
        loaded.evolution_ledger.replay_live_evolution_memory_updates,
        6
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_stored_memory_updates,
        3
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_reflection_issues,
        5
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_critical_reflection_issues,
        1
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_revision_actions,
        4
    );
    assert_eq!(loaded.evolution_ledger.recursive_replay_items, 1);
    assert_eq!(loaded.evolution_ledger.recursive_runtime_calls, 8);
    assert_eq!(loaded.evolution_ledger.drift_rollbacks, 2);
    assert!((loaded.evolution_ledger.rollback_router_threshold_delta - 0.11).abs() < 0.0001);
    assert!((loaded.evolution_ledger.rollback_hierarchy_weight_delta - 0.09).abs() < 0.0001);
    assert_eq!(loaded.evolution_ledger.external_feedbacks, 2);
    assert_eq!(loaded.evolution_ledger.external_feedback_reinforcements, 3);
    assert_eq!(loaded.evolution_ledger.external_feedback_penalties, 1);
    assert_eq!(loaded.evolution_ledger.external_feedback_memory_updates, 4);
    assert_eq!(loaded.evolution_ledger.external_feedback_removed, 1);
    assert_eq!(loaded.evolution_ledger.external_feedback_missing, 2);
    assert!((loaded.evolution_ledger.external_feedback_strength_delta - 0.31).abs() < 0.0001);
    cleanup(path);
}

#[test]
fn evolution_ledger_loads_legacy_without_rollback_fields() {
    let legacy = "3\t9\t5\t7\t0.420000\t0.210000\t4\t2\t1\t8";
    let ledger = parse_evolution_ledger(legacy).unwrap();

    assert_eq!(ledger.replay_runs, 3);
    assert_eq!(ledger.memory_updates(), 6);
    assert_eq!(ledger.replay_live_memory_feedback_items, 0);
    assert_eq!(ledger.replay_live_memory_feedback_updates(), 0);
    assert_eq!(ledger.replay_live_memory_feedback_detail_items, 0);
    assert_eq!(ledger.replay_live_memory_feedback_detail_updates(), 0);
    assert_eq!(ledger.live_online_reward_strength, 0.0);
    assert_eq!(ledger.live_online_reward_reinforcement_strength, 0.0);
    assert_eq!(ledger.live_online_reward_penalty_strength, 0.0);
    assert_eq!(ledger.replay_live_evolution_online_reward_strength, 0.0);
    assert_eq!(
        ledger.replay_live_evolution_online_reward_reinforcement_strength,
        0.0
    );
    assert_eq!(
        ledger.replay_live_evolution_online_reward_penalty_strength,
        0.0
    );
    assert_eq!(ledger.recursive_runtime_calls, 8);
    assert_eq!(ledger.drift_rollbacks, 0);
    assert_eq!(ledger.rollback_router_threshold_delta, 0.0);
    assert_eq!(ledger.rollback_hierarchy_weight_delta, 0.0);
    assert_eq!(ledger.external_feedbacks, 0);
    assert_eq!(ledger.external_feedback_memory_updates, 0);
    assert_eq!(ledger.external_feedback_strength_delta, 0.0);
    assert_eq!(ledger.replay_rust_check_items, 0);
    assert_eq!(ledger.replay_rust_check_total(), 0);
    assert_eq!(ledger.replay_rust_check_diagnostic_chars, 0);
    assert_eq!(ledger.replay_rust_check_live_memory_feedback_updates, 0);
    assert_eq!(
        ledger.replay_rust_check_live_memory_feedback_strength_delta,
        0.0
    );
    assert_eq!(ledger.replay_business_contract_items, 0);
    assert_eq!(ledger.replay_business_contract_total(), 0);
    assert_eq!(ledger.replay_business_contract_raw_failed, 0);
    assert_eq!(ledger.replay_business_contract_canonical_fallbacks, 0);
}

#[test]
fn adaptive_state_loads_legacy_files_without_memory_policies() {
    let path = temp_path("adaptive-state-legacy");
    {
        let mut store = DiskKvStore::open(&path).unwrap();
        store.put("adaptive/router", b"0.610000\t17").unwrap();
        store
            .put("adaptive/hierarchy", b"0.200000\t0.600000\t0.200000")
            .unwrap();
        store.compact().unwrap();
    }

    let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

    assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
    assert_eq!(loaded.router.observations, 17);
    assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
    assert_eq!(
        loaded.memory_retention_policy.stale_after,
        MemoryRetentionPolicy::default().stale_after
    );
    assert!(
        (loaded.memory_compaction_policy.similarity_threshold
            - MemoryCompactionPolicy::default().similarity_threshold)
            .abs()
            < 0.0001
    );
    assert_eq!(loaded.evolution_ledger, EvolutionLedger::default());
    cleanup(path);
}

fn temp_path(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rust-norion-{label}-{}-{nanos}.ndkv",
        std::process::id()
    ))
}

fn cleanup(path: std::path::PathBuf) {
    let _ = fs::remove_file(path);
}
