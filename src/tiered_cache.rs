use std::cmp::Ordering;
use std::collections::HashMap;
use std::str::FromStr;

use crate::kv_cache::{MemoryEntry, MemoryMatch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryTier {
    HotGpu,
    WarmRam,
    ColdDisk,
}

impl MemoryTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HotGpu => "hot_gpu",
            Self::WarmRam => "warm_ram",
            Self::ColdDisk => "cold_disk",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::HotGpu => 0,
            Self::WarmRam => 1,
            Self::ColdDisk => 2,
        }
    }
}

impl FromStr for MemoryTier {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "hot_gpu" => Ok(Self::HotGpu),
            "warm_ram" => Ok(Self::WarmRam),
            "cold_disk" => Ok(Self::ColdDisk),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryPlacement {
    pub id: u64,
    pub tier: MemoryTier,
    pub score: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierMigrationAction {
    New,
    Promote,
    Demote,
    Retain,
    Evict,
}

#[derive(Debug, Clone)]
pub struct TierMigration {
    pub id: u64,
    pub from: Option<MemoryTier>,
    pub to: Option<MemoryTier>,
    pub action: TierMigrationAction,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct TieredCachePlan {
    placements: Vec<MemoryPlacement>,
}

impl TieredCachePlan {
    pub fn new(placements: Vec<MemoryPlacement>) -> Self {
        Self { placements }
    }

    pub fn placements(&self) -> &[MemoryPlacement] {
        &self.placements
    }

    pub fn placement_for(&self, id: u64) -> Option<&MemoryPlacement> {
        self.placements.iter().find(|placement| placement.id == id)
    }

    pub fn migrations_from(&self, previous: &TieredCachePlan) -> Vec<TierMigration> {
        let previous_by_id = previous
            .placements
            .iter()
            .map(|placement| (placement.id, placement))
            .collect::<HashMap<_, _>>();
        let current_by_id = self
            .placements
            .iter()
            .map(|placement| (placement.id, placement))
            .collect::<HashMap<_, _>>();
        let mut migrations = Vec::new();

        for current in &self.placements {
            let Some(previous) = previous_by_id.get(&current.id) else {
                migrations.push(TierMigration {
                    id: current.id,
                    from: None,
                    to: Some(current.tier),
                    action: TierMigrationAction::New,
                    reason: format!("new:{}", current.reason),
                });
                continue;
            };

            let action = if current.tier.rank() < previous.tier.rank() {
                TierMigrationAction::Promote
            } else if current.tier.rank() > previous.tier.rank() {
                TierMigrationAction::Demote
            } else {
                TierMigrationAction::Retain
            };

            migrations.push(TierMigration {
                id: current.id,
                from: Some(previous.tier),
                to: Some(current.tier),
                action,
                reason: format!("{} -> {}", previous.reason, current.reason),
            });
        }

        for previous in &previous.placements {
            if !current_by_id.contains_key(&previous.id) {
                migrations.push(TierMigration {
                    id: previous.id,
                    from: Some(previous.tier),
                    to: None,
                    action: TierMigrationAction::Evict,
                    reason: format!("evict:{}", previous.reason),
                });
            }
        }

        migrations
    }

    pub fn counts(&self) -> TierCounts {
        let mut counts = TierCounts::default();

        for placement in &self.placements {
            match placement.tier {
                MemoryTier::HotGpu => counts.hot_gpu += 1,
                MemoryTier::WarmRam => counts.warm_ram += 1,
                MemoryTier::ColdDisk => counts.cold_disk += 1,
            }
        }

        counts
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TierCounts {
    pub hot_gpu: usize,
    pub warm_ram: usize,
    pub cold_disk: usize,
}

#[derive(Debug, Clone)]
pub struct TieredCacheScheduler {
    hot_capacity: usize,
    warm_capacity: usize,
    hot_threshold: f32,
    warm_threshold: f32,
    active_boost: f32,
    failure_penalty: f32,
}

impl Default for TieredCacheScheduler {
    fn default() -> Self {
        Self {
            hot_capacity: 8,
            warm_capacity: 64,
            hot_threshold: 0.85,
            warm_threshold: 0.32,
            active_boost: 0.55,
            failure_penalty: 0.08,
        }
    }
}

impl TieredCacheScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacities(hot_capacity: usize, warm_capacity: usize) -> Self {
        Self {
            hot_capacity,
            warm_capacity,
            ..Self::default()
        }
    }

    pub fn plan(&self, entries: &[MemoryEntry], active_matches: &[MemoryMatch]) -> TieredCachePlan {
        let active_similarity = active_matches
            .iter()
            .map(|memory| (memory.id, memory.similarity))
            .collect::<HashMap<_, _>>();
        let mut scored = entries
            .iter()
            .map(|entry| {
                let active = active_similarity.get(&entry.id).copied().unwrap_or(0.0);
                let score = self.score_entry(entry, active);
                (entry.id, score, active)
            })
            .collect::<Vec<_>>();

        scored.sort_by(|(_, left, _), (_, right, _)| {
            right.partial_cmp(left).unwrap_or(Ordering::Equal)
        });

        let placements = scored
            .into_iter()
            .enumerate()
            .map(|(rank, (id, score, active))| {
                let tier = self.assign_tier(rank, score);
                MemoryPlacement {
                    id,
                    tier,
                    score,
                    reason: placement_reason(tier, score, active),
                }
            })
            .collect();

        TieredCachePlan::new(placements)
    }

    fn score_entry(&self, entry: &MemoryEntry, active_similarity: f32) -> f32 {
        let attempts = entry.hits + entry.failures;
        let reliability = if attempts == 0 {
            0.5
        } else {
            entry.hits as f32 / attempts as f32
        };
        let failure_drag = entry.failures as f32 * self.failure_penalty;

        (entry.strength * 0.45
            + entry.last_score.max(0.0) * 0.18
            + reliability * 0.22
            + active_similarity * self.active_boost
            - failure_drag)
            .clamp(0.0, 3.0)
    }

    fn assign_tier(&self, rank: usize, score: f32) -> MemoryTier {
        if rank < self.hot_capacity && score >= self.hot_threshold {
            MemoryTier::HotGpu
        } else if rank < self.hot_capacity + self.warm_capacity && score >= self.warm_threshold {
            MemoryTier::WarmRam
        } else {
            MemoryTier::ColdDisk
        }
    }
}

fn placement_reason(tier: MemoryTier, score: f32, active_similarity: f32) -> String {
    format!(
        "{}:score={score:.3}:active_similarity={active_similarity:.3}",
        tier.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_strong_memory_is_promoted_hot() {
        let scheduler = TieredCacheScheduler::with_capacities(1, 2);
        let entries = vec![entry(1, 1.4, 4, 0, 0.95), entry(2, 0.7, 1, 0, 0.65)];
        let matches = vec![MemoryMatch {
            id: 1,
            key: "hot".to_owned(),
            similarity: 0.9,
            strength: 1.4,
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
}
