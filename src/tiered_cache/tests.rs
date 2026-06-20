use super::*;
use crate::kv_cache::{MemoryEntry, MemoryMatch};

#[test]
fn active_strong_memory_is_promoted_hot() {
    let scheduler = TieredCacheScheduler::with_capacities(1, 2);
    let entries = vec![entry(1, 1.4, 4, 0, 0.95), entry(2, 0.7, 1, 0, 0.65)];
    let matches = vec![MemoryMatch {
        id: 1,
        key: "hot".to_owned(),
        similarity: 0.9,
        strength: 1.4,
        vector: vec![1.0, 1.4],
    }];

    let plan = scheduler.plan(&entries, &matches);

    assert_eq!(plan.placement_for(1).unwrap().tier, MemoryTier::HotGpu);
    assert_eq!(plan.counts().hot_gpu, 1);
}

#[test]
fn weak_failed_memory_goes_cold() {
    let scheduler = TieredCacheScheduler::with_capacities(2, 2);
    let entries = vec![entry(7, 0.08, 0, 5, 0.1)];
    let plan = scheduler.plan(&entries, &[]);

    assert_eq!(plan.placement_for(7).unwrap().tier, MemoryTier::ColdDisk);
}

#[test]
fn migrations_capture_new_promote_demote_retain_and_evict() {
    let previous = TieredCachePlan::new(vec![
        placement(1, MemoryTier::ColdDisk),
        placement(2, MemoryTier::HotGpu),
        placement(3, MemoryTier::WarmRam),
        placement(4, MemoryTier::WarmRam),
    ]);
    let current = TieredCachePlan::new(vec![
        placement(1, MemoryTier::HotGpu),
        placement(2, MemoryTier::WarmRam),
        placement(3, MemoryTier::WarmRam),
        placement(5, MemoryTier::ColdDisk),
    ]);

    let migrations = current.migrations_from(&previous);

    assert_eq!(action_for(&migrations, 1), TierMigrationAction::Promote);
    assert_eq!(action_for(&migrations, 2), TierMigrationAction::Demote);
    assert_eq!(action_for(&migrations, 3), TierMigrationAction::Retain);
    assert_eq!(action_for(&migrations, 4), TierMigrationAction::Evict);
    assert_eq!(action_for(&migrations, 5), TierMigrationAction::New);
}

fn entry(id: u64, strength: f32, hits: u64, failures: u64, last_score: f32) -> MemoryEntry {
    MemoryEntry {
        id,
        key: format!("memory {id}"),
        vector: vec![id as f32, strength],
        strength,
        hits,
        failures,
        last_score,
        created_at: 0,
        last_access: hits + failures,
    }
}

fn placement(id: u64, tier: MemoryTier) -> MemoryPlacement {
    MemoryPlacement {
        id,
        tier,
        score: id as f32,
        reason: format!("placement {id}"),
    }
}

fn action_for(migrations: &[TierMigration], id: u64) -> TierMigrationAction {
    migrations
        .iter()
        .find(|migration| migration.id == id)
        .map(|migration| migration.action)
        .expect("migration should exist")
}
