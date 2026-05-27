use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use crate::disk_kv::DiskKvStore;
use crate::kv_quant::{QuantizationBits, QuantizedVector};

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: u64,
    pub key: String,
    pub vector: Vec<f32>,
    pub strength: f32,
    pub hits: u64,
    pub failures: u64,
    pub last_score: f32,
    pub created_at: u64,
    pub last_access: u64,
}

#[derive(Debug, Clone)]
pub struct MemoryMatch {
    pub id: u64,
    pub key: String,
    pub similarity: f32,
    pub strength: f32,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryRetentionPolicy {
    pub stale_after: u64,
    pub decay_rate: f32,
    pub remove_below_strength: f32,
    pub remove_after_failures: u64,
}

impl Default for MemoryRetentionPolicy {
    fn default() -> Self {
        Self {
            stale_after: 64,
            decay_rate: 0.04,
            remove_below_strength: 0.04,
            remove_after_failures: 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryCompactionPolicy {
    pub similarity_threshold: f32,
    pub max_candidates: usize,
    pub max_merges: usize,
}

impl Default for MemoryCompactionPolicy {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.92,
            max_candidates: 512,
            max_merges: 32,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryCompactionMerge {
    pub primary_id: u64,
    pub removed_id: u64,
    pub similarity: f32,
}

#[derive(Debug, Clone)]
pub struct MemoryCompactionReport {
    pub before: usize,
    pub after: usize,
    pub merged: Vec<MemoryCompactionMerge>,
    pub removed: Vec<u64>,
}

impl MemoryCompactionReport {
    pub fn skipped(current_len: usize) -> Self {
        Self {
            before: current_len,
            after: current_len,
            merged: Vec::new(),
            removed: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetentionReport {
    pub before: usize,
    pub after: usize,
    pub decayed: usize,
    pub removed: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct KvFusionCache {
    entries: Vec<MemoryEntry>,
    similarity_threshold: f32,
    max_entries: usize,
    next_id: u64,
    clock: u64,
}

impl Default for KvFusionCache {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            similarity_threshold: 0.78,
            max_entries: 4096,
            next_id: 1,
            clock: 0,
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

    pub fn clock(&self) -> u64 {
        self.clock
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

    pub fn store_or_fuse(
        &mut self,
        key: impl Into<String>,
        vector: Vec<f32>,
        usefulness: f32,
    ) -> u64 {
        let key = key.into();
        let usefulness = usefulness.clamp(0.05, 1.0);
        let now = self.tick();

        if let Some((index, score)) = self.best_match_index(&vector) {
            if score >= self.similarity_threshold {
                let entry = &mut self.entries[index];
                fuse_vector(&mut entry.vector, &vector, entry.strength, usefulness);
                entry.key = merge_key(&entry.key, &key);
                entry.strength = (entry.strength + usefulness * 0.28).clamp(0.01, 3.0);
                entry.hits += 1;
                entry.last_score = score;
                entry.last_access = now;
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
            created_at: now,
            last_access: now,
        });
        self.prune_if_needed();
        id
    }

    pub fn reinforce(&mut self, id: u64, amount: f32) {
        if let Some(index) = self.entries.iter().position(|entry| entry.id == id) {
            let now = self.tick();
            let entry = &mut self.entries[index];
            entry.strength = (entry.strength + amount.clamp(0.0, 1.0) * 0.18).clamp(0.01, 3.0);
            entry.hits += 1;
            entry.last_access = now;
        }
    }

    pub fn penalize(&mut self, id: u64, amount: f32) {
        if let Some(index) = self.entries.iter().position(|entry| entry.id == id) {
            let now = self.tick();
            let entry = &mut self.entries[index];
            entry.strength = (entry.strength - amount.clamp(0.0, 1.0) * 0.22).clamp(0.0, 3.0);
            entry.failures += 1;
            entry.last_access = now;
        }
        self.entries
            .retain(|entry| entry.strength > 0.03 || entry.hits > entry.failures);
    }

    pub fn apply_retention(&mut self, policy: MemoryRetentionPolicy) -> RetentionReport {
        let before = self.entries.len();
        let now = self.tick();
        let stale_after = policy.stale_after.max(1);
        let decay_rate = policy.decay_rate.clamp(0.0, 0.95);
        let mut decayed = 0;

        for entry in &mut self.entries {
            let idle = now.saturating_sub(entry.last_access);
            if idle <= policy.stale_after {
                continue;
            }

            let periods = (idle - policy.stale_after) as f32 / stale_after as f32;
            let decay = (decay_rate * periods.max(1.0)).clamp(0.0, 0.95);
            let before_strength = entry.strength;
            entry.strength = (entry.strength * (1.0 - decay)).clamp(0.0, 3.0);
            if entry.strength < before_strength {
                decayed += 1;
            }
        }

        let mut removed = Vec::new();
        self.entries.retain(|entry| {
            let idle = now.saturating_sub(entry.last_access);
            let weak_and_stale = entry.strength <= policy.remove_below_strength
                && idle > policy.stale_after
                && entry.failures >= entry.hits;
            let repeatedly_failed =
                entry.failures >= policy.remove_after_failures && entry.hits == 0;
            let remove = weak_and_stale || repeatedly_failed;
            if remove {
                removed.push(entry.id);
            }
            !remove
        });

        RetentionReport {
            before,
            after: self.entries.len(),
            decayed,
            removed,
        }
    }

    pub fn compact_similar(&mut self, policy: MemoryCompactionPolicy) -> MemoryCompactionReport {
        self.compact_similar_with_protected(policy, &[])
    }

    pub fn compact_similar_with_protected(
        &mut self,
        policy: MemoryCompactionPolicy,
        protected_ids: &[u64],
    ) -> MemoryCompactionReport {
        let before = self.entries.len();
        if before < 2 || policy.max_merges == 0 || policy.max_candidates < 2 {
            return MemoryCompactionReport::skipped(before);
        }

        let now = self.tick();
        let threshold = policy.similarity_threshold.clamp(0.10, 0.999);
        let protected = protected_ids.iter().copied().collect::<HashSet<_>>();
        let mut candidates = self
            .entries
            .iter()
            .map(|entry| (entry.id, memory_value_score(entry, now)))
            .collect::<Vec<_>>();

        candidates.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        candidates.truncate(policy.max_candidates.min(candidates.len()));

        let candidate_ids = candidates.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
        let mut removed = HashSet::new();
        let mut merges = Vec::new();

        'outer: for left_pos in 0..candidate_ids.len() {
            for right_pos in (left_pos + 1)..candidate_ids.len() {
                if merges.len() >= policy.max_merges {
                    break 'outer;
                }

                let left_id = candidate_ids[left_pos];
                let right_id = candidate_ids[right_pos];
                if removed.contains(&left_id) || removed.contains(&right_id) {
                    continue;
                }

                let Some(left_index) = self.entry_index(left_id) else {
                    continue;
                };
                let Some(right_index) = self.entry_index(right_id) else {
                    continue;
                };
                let similarity = cosine_similarity(
                    &self.entries[left_index].vector,
                    &self.entries[right_index].vector,
                );
                if similarity < threshold {
                    continue;
                }

                let Some((primary_id, removed_id)) = choose_compaction_pair(
                    &self.entries[left_index],
                    &self.entries[right_index],
                    &protected,
                    now,
                ) else {
                    continue;
                };
                let Some(primary_index) = self.entry_index(primary_id) else {
                    continue;
                };
                let Some(removed_index) = self.entry_index(removed_id) else {
                    continue;
                };

                let duplicate = self.entries[removed_index].clone();
                merge_memory_entry(
                    &mut self.entries[primary_index],
                    &duplicate,
                    similarity,
                    now,
                );
                removed.insert(removed_id);
                merges.push(MemoryCompactionMerge {
                    primary_id,
                    removed_id,
                    similarity,
                });
            }
        }

        let mut removed_ids = removed.into_iter().collect::<Vec<_>>();
        removed_ids.sort_unstable();
        self.entries
            .retain(|entry| removed_ids.binary_search(&entry.id).is_err());

        MemoryCompactionReport {
            before,
            after: self.entries.len(),
            merged: merges,
            removed: removed_ids,
        }
    }

    pub fn save_to_disk(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut content = String::new();
        content.push_str("# noiron-kv-cache-v1\n");

        for entry in &self.entries {
            content.push_str(&serialize_entry(entry));
            content.push('\n');
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
            let Some(entry) = deserialize_entry(line) else {
                continue;
            };
            let id = entry.id;
            cache.clock = cache.clock.max(entry.created_at).max(entry.last_access);
            cache.entries.push(entry);
            cache.next_id = cache.next_id.max(id + 1);
        }
        cache.clock = cache.clock.saturating_add(1);

        Ok(cache)
    }

    pub fn save_persistent(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();

        match self.save_to_disk_kv(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::InvalidData && path.exists() => {
                let backup_path = legacy_backup_path(path);
                fs::rename(path, &backup_path)?;
                self.save_to_disk_kv(path)
            }
            Err(error) => Err(error),
        }
    }

    pub fn load_persistent(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();

        match Self::load_from_disk_kv(path) {
            Ok(cache) => Ok(cache),
            Err(error) if error.kind() == ErrorKind::InvalidData => Self::load_from_disk(path),
            Err(error) => Err(error),
        }
    }

    pub fn save_to_disk_kv(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut store = DiskKvStore::open(path)?;
        let mut live_keys = HashSet::new();

        for entry in &self.entries {
            let key = format!("memory/{}", entry.id);
            live_keys.insert(key.clone());
            store.put(
                &key,
                serialize_entry_quantized(entry, QuantizationBits::Four).as_bytes(),
            )?;
        }

        for stale_key in store.keys_with_prefix("memory/") {
            if !live_keys.contains(&stale_key) {
                store.delete(&stale_key)?;
            }
        }

        store.put("meta/next_id", self.next_id.to_string().as_bytes())?;
        store.compact()
    }

    pub fn load_from_disk_kv(path: impl AsRef<Path>) -> io::Result<Self> {
        let store = DiskKvStore::open(path)?;
        let mut cache = Self::new();

        for key in store.keys_with_prefix("memory/") {
            let Some(value) = store.get(&key)? else {
                continue;
            };
            let Ok(line) = String::from_utf8(value) else {
                continue;
            };
            let Some(entry) = deserialize_entry(&line) else {
                continue;
            };
            cache.next_id = cache.next_id.max(entry.id + 1);
            cache.clock = cache.clock.max(entry.created_at).max(entry.last_access);
            cache.entries.push(entry);
        }

        if let Some(value) = store.get("meta/next_id")? {
            if let Ok(next_id) = String::from_utf8_lossy(&value).parse::<u64>() {
                cache.next_id = cache.next_id.max(next_id);
            }
        }
        cache.clock = cache.clock.saturating_add(1);

        Ok(cache)
    }

    fn tick(&mut self) -> u64 {
        self.clock = self.clock.saturating_add(1);
        self.clock
    }

    fn best_match_index(&self, vector: &[f32]) -> Option<(usize, f32)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (index, cosine_similarity(vector, &entry.vector)))
            .max_by(|(_, left), (_, right)| left.partial_cmp(right).unwrap_or(Ordering::Equal))
    }

    fn entry_index(&self, id: u64) -> Option<usize> {
        self.entries.iter().position(|entry| entry.id == id)
    }

    fn prune_if_needed(&mut self) {
        if self.entries.len() <= self.max_entries {
            return;
        }

        self.entries.sort_by(|a, b| {
            let left = memory_value_score(a, self.clock);
            let right = memory_value_score(b, self.clock);
            right.partial_cmp(&left).unwrap_or(Ordering::Equal)
        });
        self.entries.truncate(self.max_entries);
    }
}

fn choose_compaction_pair(
    left: &MemoryEntry,
    right: &MemoryEntry,
    protected: &HashSet<u64>,
    now: u64,
) -> Option<(u64, u64)> {
    let left_protected = protected.contains(&left.id);
    let right_protected = protected.contains(&right.id);

    match (left_protected, right_protected) {
        (true, true) => None,
        (true, false) => Some((left.id, right.id)),
        (false, true) => Some((right.id, left.id)),
        (false, false) => {
            let left_score = memory_value_score(left, now);
            let right_score = memory_value_score(right, now);
            if left_score > right_score
                || ((left_score - right_score).abs() < 0.0001 && left.id < right.id)
            {
                Some((left.id, right.id))
            } else {
                Some((right.id, left.id))
            }
        }
    }
}

fn merge_memory_entry(
    primary: &mut MemoryEntry,
    duplicate: &MemoryEntry,
    similarity: f32,
    now: u64,
) {
    let duplicate_weight = duplicate.strength.max(0.05);
    fuse_vector(
        &mut primary.vector,
        &duplicate.vector,
        primary.strength.max(0.05),
        duplicate_weight,
    );
    primary.key = merge_key(&primary.key, &duplicate.key);
    primary.strength = (primary.strength + duplicate.strength * 0.35).clamp(0.01, 3.0);
    primary.hits = primary
        .hits
        .saturating_add(duplicate.hits)
        .saturating_add(1);
    primary.failures = primary.failures.saturating_add(duplicate.failures);
    primary.last_score = primary.last_score.max(similarity);
    primary.created_at = primary.created_at.min(duplicate.created_at);
    primary.last_access = now.max(primary.last_access).max(duplicate.last_access);
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

fn serialize_entry(entry: &MemoryEntry) -> String {
    let vector = entry
        .vector
        .iter()
        .map(|value| format!("{value:.6}"))
        .collect::<Vec<_>>()
        .join(",");

    serialize_entry_with_vector(entry, &vector)
}

fn serialize_entry_quantized(entry: &MemoryEntry, bits: QuantizationBits) -> String {
    let vector = QuantizedVector::quantize(&entry.vector, bits).encode();
    serialize_entry_with_vector(entry, &vector)
}

fn serialize_entry_with_vector(entry: &MemoryEntry, vector: &str) -> String {
    format!(
        "{}\t{:.6}\t{}\t{}\t{:.6}\t{}\t{}\t{}\t{}",
        entry.id,
        entry.strength,
        entry.hits,
        entry.failures,
        entry.last_score,
        entry.created_at,
        entry.last_access,
        escape_field(&entry.key),
        vector
    )
}

fn deserialize_entry(line: &str) -> Option<MemoryEntry> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() != 7 && fields.len() != 9 {
        return None;
    }

    let id = fields[0].parse::<u64>().ok()?;
    let strength = fields[1].parse::<f32>().ok()?;
    let hits = fields[2].parse::<u64>().ok()?;
    let failures = fields[3].parse::<u64>().ok()?;
    let last_score = fields[4].parse::<f32>().ok()?;
    let (created_at, last_access, key, vector) = match fields.len() {
        7 => (
            0,
            hits.saturating_add(failures),
            unescape_field(fields[5]),
            deserialize_vector(fields[6])?,
        ),
        9 => (
            fields[5].parse::<u64>().ok()?,
            fields[6].parse::<u64>().ok()?,
            unescape_field(fields[7]),
            deserialize_vector(fields[8])?,
        ),
        _ => return None,
    };

    Some(MemoryEntry {
        id,
        key,
        vector,
        strength,
        hits,
        failures,
        last_score,
        created_at,
        last_access,
    })
}

fn deserialize_vector(encoded: &str) -> Option<Vec<f32>> {
    if encoded.starts_with('q') {
        return QuantizedVector::decode(encoded)
            .ok()
            .map(|vector| vector.dequantize());
    }

    if encoded.is_empty() {
        return Some(Vec::new());
    }

    Some(
        encoded
            .split(',')
            .filter_map(|value| value.parse::<f32>().ok())
            .collect::<Vec<_>>(),
    )
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

fn memory_value_score(entry: &MemoryEntry, now: u64) -> f32 {
    let idle = now.saturating_sub(entry.last_access) as f32;
    let idle_drag = (idle / 256.0).min(0.35);
    entry.strength - entry.failures as f32 * 0.08 + entry.hits as f32 * 0.02 - idle_drag
}

fn legacy_backup_path(path: &Path) -> PathBuf {
    for index in 0..1024 {
        let extension = if index == 0 {
            "legacy.tsv".to_owned()
        } else {
            format!("legacy.{index}.tsv")
        };
        let candidate = path.with_extension(extension);
        if !candidate.exists() {
            return candidate;
        }
    }

    path.with_extension("legacy.tsv")
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
    fn penalize_removes_weak_bad_memory() {
        let mut cache = KvFusionCache::new();
        let id = cache.store_or_fuse("bad memory", vec![0.1, 0.2], 0.05);

        for _ in 0..3 {
            cache.penalize(id, 1.0);
        }

        assert!(cache.entries().iter().all(|entry| entry.id != id));
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
}
