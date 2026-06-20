use std::cmp::Ordering;

use super::cache::KvFusionCache;
use super::model::MemoryMatch;
use super::ops::cosine_similarity;

impl KvFusionCache {
    pub fn lookup(&self, query: &[f32], limit: usize) -> Vec<MemoryMatch> {
        let mut matches = self
            .entries
            .iter()
            .map(|entry| MemoryMatch {
                id: entry.id,
                key: entry.key.clone(),
                similarity: cosine_similarity(query, &entry.vector),
                strength: entry.strength,
                vector: entry.vector.clone(),
            })
            .filter(|item| item.similarity > 0.05)
            .collect::<Vec<_>>();

        matches.sort_by(|a, b| {
            let a_score = a.similarity * a.strength;
            let b_score = b.similarity * b.strength;
            b_score.partial_cmp(&a_score).unwrap_or(Ordering::Equal)
        });
        matches.truncate(limit);
        matches
    }
}
