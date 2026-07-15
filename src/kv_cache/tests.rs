use super::*;
use crate::disk_kv::DiskKvStore;
use crate::gist_memory::{GistLevel, GistRecord};
use crate::tenant_scope::{TenantResourceLane, TenantScope, TenantScopedKey};
use std::fs;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn fuses_similar_memories() {
    let mut cache = KvFusionCache::with_limits(0.7, 16);
    let first = cache.store_or_fuse("rust attention routing", vec![1.0, 0.0, 0.0], 0.8);
    let second = cache.store_or_fuse("rust adaptive routing", vec![0.95, 0.05, 0.0], 0.8);

    assert_eq!(first, second);
    assert_eq!(cache.len(), 1);
    assert!(cache.entries()[0].strength > 0.8);
}

#[test]
fn fusing_multibyte_memory_keys_truncates_on_utf8_boundaries() {
    let mut cache = KvFusionCache::with_limits(0.7, 16);
    let first_key = "本地模型联调计划".repeat(24);
    let second_key = "必须用 cargo test 验证中文记忆融合".repeat(24);
    let first = cache.store_or_fuse(first_key, vec![1.0, 0.0, 0.0], 0.8);
    let second = cache.store_or_fuse(second_key, vec![0.95, 0.05, 0.0], 0.8);

    assert_eq!(first, second);
    let key = &cache.entries()[0].key;
    assert!(key.is_char_boundary(key.len()));
    assert!(key.len() <= 260);
}

#[test]
fn dimension_mismatched_vectors_do_not_fuse() {
    let mut cache = KvFusionCache::with_limits(0.7, 16);
    let first = cache.store_or_fuse("old fallback embedding", vec![1.0], 0.9);
    let second = cache.store_or_fuse("runtime embedding", vec![1.0, 0.0, 0.0, 0.0], 0.9);

    assert_ne!(first, second);
    assert_eq!(cache.len(), 2);
}

#[test]
fn runtime_kv_memories_do_not_fuse_with_semantic_memories() {
    let mut cache = KvFusionCache::with_limits(0.7, 16);
    let semantic = cache.store_or_fuse("prompt lesson memory", vec![1.0, 0.0, 0.0], 0.9);
    let runtime_kv = cache.store_or_fuse(
        "runtime_kv:l0h0:0-1 :: prompt lesson memory",
        vec![1.0, 0.0, 0.0],
        0.9,
    );

    assert_ne!(semantic, runtime_kv);
    assert_eq!(cache.len(), 2);
    assert!(
        cache
            .entries()
            .iter()
            .any(|entry| entry.id == runtime_kv && entry.key.starts_with("runtime_kv:"))
    );
}

#[test]
fn runtime_kv_memories_only_fuse_within_the_same_slot() {
    let mut cache = KvFusionCache::with_limits(0.7, 16);
    let first = cache.store_or_fuse(
        "runtime_kv:l0h0:0-1 :: prompt lesson memory",
        vec![1.0, 0.0, 0.0],
        0.9,
    );
    let same_slot = cache.store_or_fuse(
        "runtime_kv:l0h0:0-1 :: prompt lesson memory followup",
        vec![0.99, 0.01, 0.0],
        0.9,
    );
    let other_layer = cache.store_or_fuse(
        "runtime_kv:l1h0:0-1 :: prompt lesson memory",
        vec![1.0, 0.0, 0.0],
        0.9,
    );
    let other_token_range = cache.store_or_fuse(
        "runtime_kv:l0h0:1-2 :: prompt lesson memory",
        vec![1.0, 0.0, 0.0],
        0.9,
    );

    assert_eq!(first, same_slot);
    assert_ne!(first, other_layer);
    assert_ne!(first, other_token_range);
    assert_ne!(other_layer, other_token_range);
    assert_eq!(cache.len(), 3);
}

#[test]
fn scoped_kv_cache_fails_closed_across_tenants() {
    let mut cache = KvFusionCache::with_limits(0.7, 16);
    let tenant_a = TenantScope::new("tenant-a", "workspace", "session-a");
    let tenant_b = TenantScope::new("tenant-b", "workspace", "session-b");
    let legacy = cache.store_or_fuse("legacy unscoped memory", vec![1.0, 0.0, 0.0], 0.9);

    let first_a = cache.store_scoped_or_fuse(
        &tenant_a,
        TenantResourceLane::RuntimeKv,
        "runtime_kv:l0h0:0-1 :: prompt lesson memory",
        vec![1.0, 0.0, 0.0],
        0.9,
    );
    let same_slot_a = cache.store_scoped_or_fuse(
        &tenant_a,
        TenantResourceLane::RuntimeKv,
        "runtime_kv:l0h0:0-1 :: prompt lesson followup",
        vec![0.99, 0.01, 0.0],
        0.9,
    );
    let first_b = cache.store_scoped_or_fuse(
        &tenant_b,
        TenantResourceLane::RuntimeKv,
        "runtime_kv:l0h0:0-1 :: prompt lesson memory",
        vec![1.0, 0.0, 0.0],
        0.9,
    );

    assert_eq!(first_a, same_slot_a);
    assert_ne!(first_a, first_b);
    assert_ne!(legacy, first_a);
    assert_eq!(cache.len(), 3);

    let key_a = &cache
        .entries()
        .iter()
        .find(|entry| entry.id == first_a)
        .unwrap()
        .key;
    let parsed_a = TenantScopedKey::parse(key_a).expect("tenant scoped key");
    assert_eq!(parsed_a.scope, tenant_a);
    assert_eq!(parsed_a.lane, TenantResourceLane::RuntimeKv);

    let matches_a = cache.lookup_scoped(&tenant_a, &[1.0, 0.0, 0.0], 8);
    let matches_b = cache.lookup_scoped(&tenant_b, &[1.0, 0.0, 0.0], 8);

    assert!(matches_a.iter().any(|item| item.id == first_a));
    assert!(matches_a.iter().all(|item| item.id != first_b));
    assert!(matches_a.iter().all(|item| item.id != legacy));
    assert!(matches_b.iter().any(|item| item.id == first_b));
    assert!(matches_b.iter().all(|item| item.id != first_a));
    assert!(matches_b.iter().all(|item| item.id != legacy));

    let compaction = cache.compact_similar(MemoryCompactionPolicy {
        similarity_threshold: 0.70,
        max_candidates: 16,
        max_merges: 16,
    });
    assert!(compaction.merged.is_empty());
    assert_eq!(cache.len(), 3);
}

#[test]
fn gist_memories_do_not_fuse_with_semantic_memories() {
    let mut cache = KvFusionCache::with_limits(0.7, 16);
    let semantic = cache.store_or_fuse("prompt lesson memory", vec![1.0, 0.0, 0.0], 0.9);
    let gist = cache.store_or_fuse(
        "gist:paragraph:prompt lesson memory",
        vec![1.0, 0.0, 0.0],
        0.9,
    );

    assert_ne!(semantic, gist);
    assert_eq!(cache.len(), 2);
    assert!(
        cache
            .entries()
            .iter()
            .any(|entry| entry.id == gist && entry.key.starts_with("gist:"))
    );
}

#[test]
fn hierarchical_gist_records_store_as_gist_kv_memory() {
    let mut cache = KvFusionCache::with_limits(0.7, 16);
    let record = GistRecord {
        level: GistLevel::Section,
        title: "Memory layer stores durable KV summaries".to_owned(),
        summary: "durable KV summaries".to_owned(),
        source_tokens: 32,
        importance: 0.84,
    };

    let id = cache.store_gist_memory(&record, vec![0.2, 0.7, 0.1]);

    assert_eq!(cache.len(), 1);
    assert_eq!(cache.entries()[0].id, id);
    assert_eq!(
        cache.entries()[0].key,
        "gist:section:Memory layer stores durable KV summaries"
    );
    assert!(cache.entries()[0].strength >= 0.84);
}

#[test]
fn lookup_penalizes_mismatched_embedding_dimensions() {
    let mut cache = KvFusionCache::with_limits(0.7, 16);
    cache.store_or_fuse("short embedding", vec![1.0], 0.9);
    let compatible = cache.store_or_fuse("runtime embedding", vec![1.0, 0.0, 0.0, 0.0], 0.6);

    let matches = cache.lookup(&[1.0, 0.0, 0.0, 0.0], 2);

    assert_eq!(matches[0].id, compatible);
    assert!(matches.iter().any(|item| item.similarity < 0.5));
}

#[test]
fn lookup_prefers_reinforced_memory_over_failed_exact_match() {
    let mut cache = KvFusionCache::with_limits(0.99, 16);
    let failed = cache.store_or_fuse("failed exact runtime lesson", vec![1.0, 0.0], 1.0);
    let reinforced =
        cache.store_or_fuse("reinforced useful runtime lesson", vec![0.95, 0.312], 1.0);
    let now = cache.clock();
    let failed_entry = cache
        .entries
        .iter_mut()
        .find(|entry| entry.id == failed)
        .unwrap();
    failed_entry.failures = 8;
    failed_entry.last_access = now;
    let reinforced_entry = cache
        .entries
        .iter_mut()
        .find(|entry| entry.id == reinforced)
        .unwrap();
    reinforced_entry.hits = 10;
    reinforced_entry.last_access = now;

    let matches = cache.lookup(&[1.0, 0.0], 2);

    assert_eq!(matches[0].id, reinforced);
    assert_eq!(matches[1].id, failed);
    assert!(matches[1].similarity > matches[0].similarity);
}

#[test]
fn penalize_removes_weak_bad_memory() {
    let mut cache = KvFusionCache::new();
    let id = cache.store_or_fuse("bad memory", vec![0.1, 0.2], 0.05);

    let first = cache.penalize(id, 1.0);
    let second = cache.penalize(id, 1.0);
    let third = cache.penalize(id, 1.0);

    assert_eq!(first.action, MemoryUpdateAction::Penalize);
    assert_eq!(first.id, id);
    assert!(first.was_applied());
    assert!(first.strength_delta < 0.0);
    assert!(first.removed || second.removed || third.removed);
    if first.removed {
        assert!(!second.was_applied());
        assert!(!third.was_applied());
    }
    assert!(cache.entries().iter().all(|entry| entry.id != id));
}

#[test]
fn penalize_keeps_unrelated_weak_memory() {
    let mut cache = KvFusionCache::new();
    let target = cache.store_or_fuse("target memory", vec![1.0, 0.0], 1.0);
    let unrelated = cache.store_or_fuse("unrelated weak memory", vec![0.0, 1.0], 1.0);
    let unrelated_entry = cache
        .entries
        .iter_mut()
        .find(|entry| entry.id == unrelated)
        .unwrap();
    unrelated_entry.strength = 0.02;
    unrelated_entry.failures = 1;

    cache.penalize(target, 1.0);

    assert!(cache.entries().iter().any(|entry| entry.id == unrelated));
}

#[test]
fn memory_update_reports_record_reinforcement_strength_delta() {
    let mut cache = KvFusionCache::new();
    let id = cache.store_or_fuse("useful memory", vec![0.4, 0.6], 0.6);
    let before = cache.entries()[0].strength;

    let report = cache.reinforce(id, 0.5);

    assert_eq!(report.action, MemoryUpdateAction::Reinforce);
    assert_eq!(report.id, id);
    assert_eq!(report.requested_amount, 0.5);
    assert_eq!(report.strength_before, Some(before));
    assert_eq!(report.strength_after, Some(cache.entries()[0].strength));
    assert!(report.was_applied());
    assert!(!report.removed);
    assert!(report.strength_delta > 0.0);

    let missing = cache.reinforce(id + 1000, 0.8);
    assert!(!missing.was_applied());
    assert_eq!(missing.strength_delta, 0.0);
}

#[test]
fn retention_decays_stale_memory() {
    let mut cache = KvFusionCache::new();
    cache.store_or_fuse("stale but useful", vec![0.3, 0.4], 0.8);
    cache.entries[0].hits = 3;
    cache.entries[0].last_access = 1;
    cache.clock = 16;

    let report = cache.apply_retention(MemoryRetentionPolicy {
        stale_after: 4,
        decay_rate: 0.20,
        remove_below_strength: 0.01,
        remove_after_failures: 8,
    });

    assert_eq!(report.before, 1);
    assert_eq!(report.after, 1);
    assert_eq!(report.decayed, 1);
    assert!(cache.entries()[0].strength < 0.8);
}

#[test]
fn retention_removes_stale_failed_memory() {
    let mut cache = KvFusionCache::new();
    let id = cache.store_or_fuse("stale failed", vec![0.1, 0.2], 0.05);
    cache.entries[0].strength = 0.02;
    cache.entries[0].failures = 4;
    cache.entries[0].last_access = 1;
    cache.clock = 16;

    let report = cache.apply_retention(MemoryRetentionPolicy {
        stale_after: 4,
        decay_rate: 0.10,
        remove_below_strength: 0.04,
        remove_after_failures: 4,
    });

    assert_eq!(report.before, 1);
    assert_eq!(report.after, 0);
    assert_eq!(report.removed, vec![id]);
    assert!(cache.is_empty());
}

#[test]
fn retention_keeps_protected_rollback_anchor_memory() {
    let mut cache = KvFusionCache::new();
    let protected =
        cache.store_or_fuse("runtime_kv:rollback-anchor:protected", vec![0.1, 0.2], 0.05);
    let removable = cache.store_or_fuse("runtime_kv:rollback-anchor:stale", vec![0.2, 0.1], 0.05);
    for entry in &mut cache.entries {
        entry.strength = 0.02;
        entry.failures = 4;
        entry.last_access = 1;
    }
    cache.clock = 16;

    let report = cache.apply_retention_with_protected(
        MemoryRetentionPolicy {
            stale_after: 4,
            decay_rate: 0.10,
            remove_below_strength: 0.04,
            remove_after_failures: 4,
        },
        &[protected],
    );

    assert_eq!(report.before, 2);
    assert_eq!(report.after, 1);
    assert_eq!(report.removed, vec![removable]);
    assert_eq!(cache.entries()[0].id, protected);
    assert!(cache.entries()[0].strength < 0.02);
}

#[test]
fn compaction_merges_existing_near_duplicate_memories() {
    let mut cache = KvFusionCache::with_limits(0.99, 16);
    let weaker = cache.store_or_fuse("old duplicate", vec![1.0, 0.0, 0.0], 0.35);
    let stronger = cache.store_or_fuse("strong duplicate", vec![0.93, 0.37, 0.0], 0.90);
    let unrelated = cache.store_or_fuse("unrelated memory", vec![0.0, 1.0, 0.0], 0.85);

    assert_eq!(cache.len(), 3);

    let report = cache.compact_similar(MemoryCompactionPolicy {
        similarity_threshold: 0.90,
        max_candidates: 16,
        max_merges: 8,
    });

    assert_eq!(report.before, 3);
    assert_eq!(report.after, 2);
    assert_eq!(report.merged.len(), 1);
    assert_eq!(report.merged[0].primary_id, stronger);
    assert_eq!(report.merged[0].removed_id, weaker);
    assert_eq!(report.merged[0].namespace, "semantic");
    assert_eq!(report.merged[0].primary_vector_dimensions, 3);
    assert_eq!(report.merged[0].removed_vector_dimensions, 3);
    assert!(!report.merged[0].primary_protected);
    assert!(!report.merged[0].removed_protected);
    assert_eq!(report.removed, vec![weaker]);
    assert!(cache.entries().iter().any(|entry| entry.id == stronger));
    assert!(cache.entries().iter().any(|entry| entry.id == unrelated));
    assert!(cache.entries().iter().all(|entry| entry.id != weaker));
    assert!(
        cache
            .entries()
            .iter()
            .find(|entry| entry.id == stronger)
            .unwrap()
            .key
            .contains("old duplicate")
    );
}

#[test]
fn compaction_does_not_merge_runtime_kv_with_semantic_memory() {
    let mut cache = KvFusionCache::with_limits(0.99, 16);
    let semantic = cache.store_or_fuse("semantic duplicate", vec![1.0, 0.0, 0.0], 0.95);
    let runtime_kv = cache.store_or_fuse(
        "runtime_kv:l1h0:0-1 :: semantic duplicate",
        vec![1.0, 0.0, 0.0],
        0.95,
    );

    let report = cache.compact_similar(MemoryCompactionPolicy {
        similarity_threshold: 0.90,
        max_candidates: 16,
        max_merges: 8,
    });

    assert_eq!(report.after, 2);
    assert!(report.merged.is_empty());
    assert!(cache.entries().iter().any(|entry| entry.id == semantic));
    assert!(cache.entries().iter().any(|entry| entry.id == runtime_kv));
}

#[test]
fn compaction_does_not_merge_different_runtime_kv_slots() {
    let mut cache = KvFusionCache::with_limits(0.99, 16);
    let first = cache.store_or_fuse(
        "runtime_kv:l0h0:0-1 :: semantic duplicate",
        vec![1.0, 0.0, 0.0],
        0.95,
    );
    let other_layer = cache.store_or_fuse(
        "runtime_kv:l1h0:0-1 :: semantic duplicate",
        vec![1.0, 0.0, 0.0],
        0.95,
    );
    let other_token_range = cache.store_or_fuse(
        "runtime_kv:l0h0:1-2 :: semantic duplicate",
        vec![1.0, 0.0, 0.0],
        0.95,
    );

    let report = cache.compact_similar(MemoryCompactionPolicy {
        similarity_threshold: 0.90,
        max_candidates: 16,
        max_merges: 8,
    });

    assert_eq!(report.after, 3);
    assert!(report.merged.is_empty());
    assert!(cache.entries().iter().any(|entry| entry.id == first));
    assert!(cache.entries().iter().any(|entry| entry.id == other_layer));
    assert!(
        cache
            .entries()
            .iter()
            .any(|entry| entry.id == other_token_range)
    );
}

#[test]
fn compaction_evidence_uses_safe_runtime_slot_namespace() {
    let mut cache = KvFusionCache::with_limits(0.99, 16);
    let first = cache.store_or_fuse(
        "runtime_kv:l0h0:0-1 :: prompt text must not leak",
        vec![1.0, 0.0, 0.0],
        0.35,
    );
    let stronger = cache.store_or_fuse(
        "runtime_kv:l0h0:0-1 :: another private prompt",
        vec![0.93, 0.37, 0.0],
        0.90,
    );

    let report = cache.compact_similar(MemoryCompactionPolicy {
        similarity_threshold: 0.90,
        max_candidates: 16,
        max_merges: 8,
    });

    assert_eq!(report.after, 1);
    assert_eq!(report.merged.len(), 1);
    assert_eq!(report.merged[0].primary_id, stronger);
    assert_eq!(report.merged[0].removed_id, first);
    assert_eq!(report.merged[0].namespace, "runtime_kv:l0h0:0-1");
    assert!(!report.merged[0].namespace.contains("prompt"));
    assert!(!report.merged[0].namespace.contains(" :: "));
    assert_eq!(report.merged[0].primary_vector_dimensions, 3);
    assert_eq!(report.merged[0].removed_vector_dimensions, 3);
}

#[test]
fn compaction_preserves_protected_current_memory_ids() {
    let mut cache = KvFusionCache::with_limits(0.99, 16);
    let protected = cache.store_or_fuse("protected current memory", vec![1.0, 0.0], 0.30);
    let duplicate = cache.store_or_fuse("strong duplicate", vec![0.94, 0.34], 0.95);

    let report = cache.compact_similar_with_protected(
        MemoryCompactionPolicy {
            similarity_threshold: 0.90,
            max_candidates: 16,
            max_merges: 8,
        },
        &[protected],
    );

    assert_eq!(report.after, 1);
    assert_eq!(report.merged[0].primary_id, protected);
    assert_eq!(report.merged[0].removed_id, duplicate);
    assert_eq!(report.merged[0].namespace, "semantic");
    assert_eq!(report.merged[0].primary_vector_dimensions, 2);
    assert_eq!(report.merged[0].removed_vector_dimensions, 2);
    assert!(report.merged[0].primary_protected);
    assert!(!report.merged[0].removed_protected);
    assert!(cache.entries().iter().any(|entry| entry.id == protected));
    assert!(cache.entries().iter().all(|entry| entry.id != duplicate));
}

#[test]
fn residency_plan_promotes_demotes_and_archives_deterministically() {
    let policy = MemoryResidencyPolicy {
        tenant_id: "tenant-a".to_owned(),
        max_hot: 1,
        max_warm: 2,
        ..MemoryResidencyPolicy::default()
    };
    let candidates = vec![
        MemoryResidencyCandidate::new(1, "tenant-a", "semantic")
            .with_scores(0.92, 12, 0, 98)
            .with_high_frequency_gene(true),
        MemoryResidencyCandidate::new(2, "tenant-a", "runtime_kv:l0h0:0-1")
            .with_scores(0.82, 7, 0, 99)
            .with_high_frequency_gene(true),
        MemoryResidencyCandidate::new(3, "tenant-a", "semantic")
            .with_scores(0.64, 2, 0, 98)
            .with_session_local(true),
        MemoryResidencyCandidate::new(4, "tenant-a", "gist").with_scores(0.28, 1, 0, 90),
    ];

    let first = plan_memory_residency(&candidates, &policy, 100);
    let second = plan_memory_residency(&candidates, &policy, 100);

    assert_eq!(first, second);
    assert_eq!(first.hot_count(), 1);
    assert_eq!(first.warm_count(), 2);
    assert_eq!(first.cold_count(), 1);
    assert_eq!(first.quarantined_count(), 0);
    assert_eq!(first.retired_count(), 0);
    assert!(
        first
            .decisions
            .iter()
            .find(|decision| decision.id == 1)
            .unwrap()
            .is_hot()
    );
    assert!(
        first
            .decisions
            .iter()
            .find(|decision| decision.id == 2)
            .unwrap()
            .blocked_reasons
            .contains(&"memory_residency_hot_budget_exhausted".to_owned())
    );
    assert!(first.summary_line().contains("read_only=true"));
    assert!(first.summary_line().contains("write_allowed=false"));
    assert!(first.replay_digest.starts_with("fnv64:"));
}

#[test]
fn residency_plan_respects_zero_hot_and_warm_budgets() {
    let policy = MemoryResidencyPolicy {
        tenant_id: "tenant-a".to_owned(),
        max_hot: 0,
        max_warm: 0,
        ..MemoryResidencyPolicy::default()
    };
    let candidates = vec![
        MemoryResidencyCandidate::new(5, "tenant-a", "semantic")
            .with_scores(0.95, 10, 0, 50)
            .with_high_frequency_gene(true),
        MemoryResidencyCandidate::new(6, "tenant-a", "semantic").with_scores(0.70, 5, 0, 50),
    ];

    let plan = plan_memory_residency(&candidates, &policy, 64);

    assert_eq!(plan.hot_count(), 0);
    assert_eq!(plan.warm_count(), 0);
    assert_eq!(plan.cold_count(), 2);
    assert!(plan.decisions.iter().any(|decision| {
        decision
            .blocked_reasons
            .contains(&"memory_residency_hot_budget_exhausted".to_owned())
    }));
    assert!(plan.decisions.iter().any(|decision| {
        decision
            .blocked_reasons
            .contains(&"memory_residency_warm_budget_exhausted".to_owned())
    }));
}

#[test]
fn residency_plan_blocks_privacy_and_tenant_mismatch_from_shared_hot() {
    let policy = MemoryResidencyPolicy {
        tenant_id: "tenant-a".to_owned(),
        max_hot: 4,
        ..MemoryResidencyPolicy::default()
    };
    let candidates = vec![
        MemoryResidencyCandidate::new(10, "tenant-b", "semantic")
            .with_scores(0.95, 20, 0, 10)
            .with_high_frequency_gene(true),
        MemoryResidencyCandidate::new(11, "tenant-a", "semantic")
            .with_scores(0.95, 20, 0, 10)
            .with_privacy(true, 0.35)
            .with_high_frequency_gene(true),
        MemoryResidencyCandidate::new(12, "tenant-a", "semantic")
            .with_scores(0.95, 20, 0, 10)
            .with_privacy(false, 0.05)
            .with_high_frequency_gene(true),
    ];

    let report = plan_memory_residency(&candidates, &policy, 12);

    let tenant_mismatch = report
        .decisions
        .iter()
        .find(|decision| decision.id == 10)
        .unwrap();
    let privacy_risky = report
        .decisions
        .iter()
        .find(|decision| decision.id == 11)
        .unwrap();
    let privacy_missing = report
        .decisions
        .iter()
        .find(|decision| decision.id == 12)
        .unwrap();

    assert!(tenant_mismatch.is_quarantined());
    assert!(
        tenant_mismatch
            .blocked_reasons
            .contains(&"memory_residency_tenant_mismatch".to_owned())
    );
    assert!(privacy_risky.is_cold());
    assert!(
        privacy_risky
            .blocked_reasons
            .contains(&"memory_residency_shared_privacy_risk".to_owned())
    );
    assert!(privacy_missing.is_quarantined());
    assert!(
        privacy_missing
            .blocked_reasons
            .contains(&"memory_residency_privacy_check_missing".to_owned())
    );
    assert_eq!(report.hot_count(), 0);
    assert!(report.write_allowed == false && report.applied == false);
}

#[test]
fn residency_plan_feeds_compaction_protected_rollback_anchors() {
    let mut cache = KvFusionCache::with_limits(0.99, 16);
    let protected = cache.store_or_fuse("rollback anchored memory", vec![1.0, 0.0], 0.30);
    let duplicate = cache.store_or_fuse("strong duplicate memory", vec![0.94, 0.34], 0.95);
    let policy = MemoryResidencyPolicy {
        tenant_id: "tenant-a".to_owned(),
        ..MemoryResidencyPolicy::default()
    };
    let candidates = vec![
        MemoryResidencyCandidate::new(protected, "tenant-a", "semantic")
            .with_scores(0.40, 0, 0, 8)
            .with_rollback_anchor("rollback:approved-experiment", true),
        MemoryResidencyCandidate::new(duplicate, "tenant-a", "semantic").with_scores(0.91, 9, 0, 9),
    ];
    let residency = plan_memory_residency(&candidates, &policy, 10);

    assert_eq!(residency.protected_ids_for_compaction(), vec![protected]);

    let report = cache.compact_similar_with_protected(
        MemoryCompactionPolicy {
            similarity_threshold: 0.90,
            max_candidates: 16,
            max_merges: 8,
        },
        &residency.protected_ids_for_compaction(),
    );

    assert_eq!(report.after, 1);
    assert_eq!(report.merged.len(), 1);
    assert_eq!(report.merged[0].primary_id, protected);
    assert_eq!(report.merged[0].removed_id, duplicate);
    assert!(report.merged[0].primary_protected);
    assert!(!report.merged[0].removed_protected);
    assert!(cache.entries().iter().any(|entry| entry.id == protected));
    assert!(cache.entries().iter().all(|entry| entry.id != duplicate));
}

#[test]
fn disk_kv_roundtrip_preserves_entries() {
    let path = temp_path("cache-roundtrip");
    let mut cache = KvFusionCache::new();
    let id = cache.store_or_fuse("durable memory", vec![0.4, 0.7, 0.1], 0.9);
    cache.reinforce(id, 0.5);

    cache.save_to_disk_kv(&path).unwrap();
    let loaded = KvFusionCache::load_from_disk_kv(&path).unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.entries()[0].id, id);
    assert_eq!(loaded.entries()[0].key, "durable memory");
    assert!(loaded.entries()[0].strength > 0.9);
    cleanup(path);
}

#[test]
fn disk_kv_uses_quantized_vectors_and_loads_them() {
    let path = temp_path("cache-quantized");
    let mut cache = KvFusionCache::new();
    let id = cache.store_or_fuse("compressed memory", vec![0.4, 0.7, 0.1], 0.9);

    cache.save_to_disk_kv(&path).unwrap();
    let store = DiskKvStore::open(&path).unwrap();
    let stored = String::from_utf8(store.get(&format!("memory/{id}")).unwrap().unwrap())
        .expect("memory record should be utf-8");

    assert!(stored.contains("\tq4:"));

    let loaded = KvFusionCache::load_from_disk_kv(&path).unwrap();
    let restored = &loaded.entries()[0].vector;

    assert_eq!(restored.len(), 3);
    assert!((restored[0] - 0.4).abs() <= 0.05);
    assert!((restored[1] - 0.7).abs() <= 0.05);
    assert!((restored[2] - 0.1).abs() <= 0.05);
    cleanup(path);
}

#[test]
fn disk_kv_loader_accepts_legacy_plain_vectors() {
    let path = temp_path("cache-legacy");
    let mut store = DiskKvStore::open(&path).unwrap();
    store
        .put(
            "memory/42",
            b"42\t0.900000\t1\t0\t1.000000\tlegacy\t0.100000,0.200000",
        )
        .unwrap();
    store.put("meta/next_id", b"43").unwrap();

    let loaded = KvFusionCache::load_from_disk_kv(&path).unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.entries()[0].id, 42);
    assert_eq!(loaded.entries()[0].vector, vec![0.1, 0.2]);
    assert_eq!(loaded.entries()[0].created_at, 0);
    assert_eq!(loaded.entries()[0].last_access, 1);
    cleanup(path);
}

#[test]
fn disk_kv_read_only_loader_does_not_create_or_repair_state() {
    let missing = temp_path("cache-read-only-missing");
    let absent = KvFusionCache::load_from_disk_kv_read_only_existing(&missing).unwrap();
    assert!(absent.is_none());
    assert!(!missing.exists());

    let path = temp_path("cache-read-only-existing");
    let mut cache = KvFusionCache::new();
    cache.store_or_fuse("read only durable memory", vec![0.3, 0.6], 0.9);
    cache.save_to_disk_kv(&path).unwrap();
    let clean_len = fs::metadata(&path).unwrap().len();
    {
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"NDK1\x01").unwrap();
    }
    let dirty_len = fs::metadata(&path).unwrap().len();

    let loaded = KvFusionCache::load_from_disk_kv_read_only_existing(&path)
        .unwrap()
        .unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.entries()[0].key, "read only durable memory");
    assert_eq!(fs::metadata(&path).unwrap().len(), dirty_len);
    assert!(dirty_len > clean_len);
    cleanup(missing);
    cleanup(path);
}

#[test]
fn persistent_save_uses_append_only_disk_kv() {
    let path = temp_path("persistent-disk-kv");
    let mut cache = KvFusionCache::new();
    cache.store_or_fuse("persistent disk kv", vec![0.2, 0.4, 0.6], 0.9);

    cache.save_persistent(&path).unwrap();

    let bytes = fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"NDK1"));

    let loaded = KvFusionCache::load_persistent(&path).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.entries()[0].key, "persistent disk kv");
    cleanup(path);
}

#[test]
fn persistent_load_accepts_legacy_tsv_and_migrates_on_save() {
    let path = temp_path("persistent-legacy").with_extension("tsv");
    let mut legacy = KvFusionCache::new();
    legacy.store_or_fuse("legacy tsv memory", vec![0.1, 0.8], 0.85);
    legacy.save_to_disk(&path).unwrap();
    let backup_path = legacy_backup_path(&path);

    let loaded = KvFusionCache::load_persistent(&path).unwrap();
    loaded.save_persistent(&path).unwrap();

    let bytes = fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"NDK1"));
    assert!(backup_path.exists());

    let migrated = KvFusionCache::load_persistent(&path).unwrap();
    assert_eq!(migrated.len(), 1);
    assert!(migrated.entries()[0].key.contains("legacy tsv memory"));

    cleanup(path);
    cleanup(backup_path);
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
    let _ = std::fs::remove_file(path);
}
