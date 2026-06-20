use std::cmp::Ordering;
use std::collections::HashMap;

use crate::kv_cache::{MemoryEntry, MemoryMatch};

use super::{MemoryPlacement, MemoryTier, TieredCachePlan};

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
