use std::collections::HashMap;

use super::{MemoryPlacement, MemoryTier, TierCounts, TierMigration, TierMigrationAction};

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
