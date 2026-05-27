use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: u64,
    pub key: String,
    pub vector: Vec<f32>,
    pub strength: f32,
    pub hits: u64,
    pub failures: u64,
    pub last_score: f32,
}

#[derive(Debug, Clone)]
pub struct MemoryMatch {
    pub id: u64,
    pub key: String,
    pub similarity: f32,
    pub strength: f32,
}

#[derive(Debug, Clone)]
pub struct KvFusionCache {
    entries: Vec<MemoryEntry>,
    similarity_threshold: f32,
    max_entries: usize,
    next_id: u64,
}

impl Default for KvFusionCache {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            similarity_threshold: 0.78,
            max_entries: 4096,
            next_id: 1,
        }
    }
}

impl KvFusionCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limits(similarity_threshold: f32, max_entries: usize) -> Self {
        Self {
            similarity_threshold: similarity_threshold.clamp(0.1, 0.99),
            max_entries: max_entries.max(1),
            ..Self::default()
        }
    }

    pub fn entries(&self) -> &[MemoryEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn lookup(&self, query: &[f32], limit: usize) -> Vec<MemoryMatch> {
        let mut matches = self
            .entries
            .iter()
            .map(|entry| MemoryMatch {
                id: entry.id,
                key: entry.key.clone(),
                similarity: cosine_similarity(query, &entry.vector),
                strength: entry.strength,
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

    pub fn store_or_fuse(
        &mut self,
        key: impl Into<String>,
        vector: Vec<f32>,
        usefulness: f32,
    ) -> u64 {
        let key = key.into();
        let usefulness = usefulness.clamp(0.05, 1.0);

        if let Some((index, score)) = self.best_match_index(&vector) {
            if score >= self.similarity_threshold {
                let entry = &mut self.entries[index];
                fuse_vector(&mut entry.vector, &vector, entry.strength, usefulness);
                entry.key = merge_key(&entry.key, &key);
                entry.strength = (entry.strength + usefulness * 0.28).clamp(0.01, 3.0);
                entry.hits += 1;
                entry.last_score = score;
                return entry.id;
            }
        }

        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(MemoryEntry {
            id,
            key,
            vector,
            strength: usefulness.max(0.2),
            hits: 0,
            failures: 0,
            last_score: 1.0,
        });
        self.prune_if_needed();
        id
    }

    pub fn reinforce(&mut self, id: u64, amount: f32) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            entry.strength = (entry.strength + amount.clamp(0.0, 1.0) * 0.18).clamp(0.01, 3.0);
            entry.hits += 1;
        }
    }

    pub fn penalize(&mut self, id: u64, amount: f32) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            entry.strength = (entry.strength - amount.clamp(0.0, 1.0) * 0.22).clamp(0.0, 3.0);
            entry.failures += 1;
        }
        self.entries
            .retain(|entry| entry.strength > 0.03 || entry.hits > entry.failures);
    }

    pub fn save_to_disk(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut content = String::new();
        content.push_str("# noiron-kv-cache-v1\n");

        for entry in &self.entries {
            let vector = entry
                .vector
                .iter()
                .map(|value| format!("{value:.6}"))
                .collect::<Vec<_>>()
                .join(",");
            content.push_str(&format!(
                "{}\t{:.6}\t{}\t{}\t{:.6}\t{}\t{}\n",
                entry.id,
                entry.strength,
                entry.hits,
                entry.failures,
                entry.last_score,
                escape_field(&entry.key),
                vector
            ));
        }

        fs::write(path, content)
    }

    pub fn load_from_disk(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)?;
        let mut cache = Self::new();

        for line in content.lines().filter(|line| !line.starts_with('#')) {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 7 {
                continue;
            }

            let Ok(id) = fields[0].parse::<u64>() else {
                continue;
            };
            let Ok(strength) = fields[1].parse::<f32>() else {
                continue;
            };
            let Ok(hits) = fields[2].parse::<u64>() else {
                continue;
            };
            let Ok(failures) = fields[3].parse::<u64>() else {
                continue;
            };
            let Ok(last_score) = fields[4].parse::<f32>() else {
                continue;
            };
            let vector = fields[6]
                .split(',')
                .filter_map(|value| value.parse::<f32>().ok())
                .collect::<Vec<_>>();

            cache.entries.push(MemoryEntry {
                id,
                key: unescape_field(fields[5]),
                vector,
                strength,
                hits,
                failures,
                last_score,
            });
            cache.next_id = cache.next_id.max(id + 1);
        }

        Ok(cache)
    }

    fn best_match_index(&self, vector: &[f32]) -> Option<(usize, f32)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (index, cosine_similarity(vector, &entry.vector)))
            .max_by(|(_, left), (_, right)| left.partial_cmp(right).unwrap_or(Ordering::Equal))
    }

    fn prune_if_needed(&mut self) {
        if self.entries.len() <= self.max_entries {
            return;
        }

        self.entries.sort_by(|a, b| {
            let left = a.strength - a.failures as f32 * 0.08 + a.hits as f32 * 0.02;
            let right = b.strength - b.failures as f32 * 0.08 + b.hits as f32 * 0.02;
            right.partial_cmp(&left).unwrap_or(Ordering::Equal)
        });
        self.entries.truncate(self.max_entries);
    }
}

fn fuse_vector(
    existing: &mut Vec<f32>,
    incoming: &[f32],
    existing_weight: f32,
    incoming_weight: f32,
) {
    let len = existing.len().max(incoming.len());
    existing.resize(len, 0.0);
    let total = (existing_weight + incoming_weight).max(0.001);

    for index in 0..len {
        let current = existing[index] * existing_weight;
        let next = incoming.get(index).copied().unwrap_or(0.0) * incoming_weight;
        existing[index] = (current + next) / total;
    }
}

fn merge_key(existing: &str, incoming: &str) -> String {
    if existing.contains(incoming) {
        return existing.to_owned();
    }
    if incoming.contains(existing) {
        return incoming.to_owned();
    }

    let mut merged = existing.to_owned();
    if merged.len() > 160 {
        merged.truncate(160);
    }
    merged.push_str(" | ");
    merged.push_str(incoming);
    if merged.len() > 260 {
        merged.truncate(260);
    }
    merged
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let len = left.len().max(right.len());
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;

    for index in 0..len {
        let l = left.get(index).copied().unwrap_or(0.0);
        let r = right.get(index).copied().unwrap_or(0.0);
        dot += l * r;
        left_norm += l * l;
        right_norm += r * r;
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        (dot / (left_norm.sqrt() * right_norm.sqrt())).clamp(-1.0, 1.0)
    }
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn unescape_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn penalize_removes_weak_bad_memory() {
        let mut cache = KvFusionCache::new();
        let id = cache.store_or_fuse("bad memory", vec![0.1, 0.2], 0.05);

        for _ in 0..3 {
            cache.penalize(id, 1.0);
        }

        assert!(cache.entries().iter().all(|entry| entry.id != id));
    }
}
