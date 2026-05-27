use std::collections::HashSet;
use std::io;
use std::path::Path;

use crate::disk_kv::DiskKvStore;
use crate::hierarchy::{HierarchyWeights, TaskProfile};

#[derive(Debug, Clone)]
pub struct ExperienceInput {
    pub prompt: String,
    pub profile: TaskProfile,
    pub lesson: String,
    pub quality: f32,
    pub contradictions: Vec<String>,
    pub stored_memory_id: Option<u64>,
    pub router_threshold_after: f32,
    pub stream_windows: usize,
    pub hierarchy: HierarchyWeights,
}

#[derive(Debug, Clone)]
pub struct ExperienceRecord {
    pub id: u64,
    pub prompt: String,
    pub profile: TaskProfile,
    pub lesson: String,
    pub quality: f32,
    pub contradictions: Vec<String>,
    pub stored_memory_id: Option<u64>,
    pub router_threshold_after: f32,
    pub stream_windows: usize,
    pub hierarchy: HierarchyWeights,
}

#[derive(Debug, Clone)]
pub struct ExperienceStore {
    records: Vec<ExperienceRecord>,
    next_id: u64,
}

impl Default for ExperienceStore {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
        }
    }
}

impl ExperienceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn records(&self) -> &[ExperienceRecord] {
        &self.records
    }

    pub fn record(&mut self, input: ExperienceInput) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.records.push(ExperienceRecord {
            id,
            prompt: input.prompt,
            profile: input.profile,
            lesson: input.lesson,
            quality: input.quality.clamp(0.0, 1.0),
            contradictions: input.contradictions,
            stored_memory_id: input.stored_memory_id,
            router_threshold_after: input.router_threshold_after,
            stream_windows: input.stream_windows,
            hierarchy: input.hierarchy,
        });
        id
    }

    pub fn recent(&self, limit: usize) -> Vec<&ExperienceRecord> {
        self.records.iter().rev().take(limit).collect()
    }

    pub fn top_lessons(&self, min_quality: f32, limit: usize) -> Vec<&ExperienceRecord> {
        let mut records = self
            .records
            .iter()
            .filter(|record| record.quality >= min_quality)
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            right
                .quality
                .partial_cmp(&left.quality)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        records.truncate(limit);
        records
    }

    pub fn save_to_disk_kv(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut store = DiskKvStore::open(path)?;
        let mut live_keys = HashSet::new();

        for record in &self.records {
            let key = format!("experience/{}", record.id);
            live_keys.insert(key.clone());
            store.put(&key, serialize_record(record).as_bytes())?;
        }

        for stale_key in store.keys_with_prefix("experience/") {
            if !live_keys.contains(&stale_key) {
                store.delete(&stale_key)?;
            }
        }

        store.put(
            "meta/next_experience_id",
            self.next_id.to_string().as_bytes(),
        )?;
        store.compact()
    }

    pub fn load_from_disk_kv(path: impl AsRef<Path>) -> io::Result<Self> {
        let store = DiskKvStore::open(path)?;
        let mut out = Self::new();

        for key in store.keys_with_prefix("experience/") {
            let Some(value) = store.get(&key)? else {
                continue;
            };
            let Ok(line) = String::from_utf8(value) else {
                continue;
            };
            let Some(record) = deserialize_record(&line) else {
                continue;
            };
            out.next_id = out.next_id.max(record.id + 1);
            out.records.push(record);
        }

        out.records.sort_by_key(|record| record.id);
        if let Some(value) = store.get("meta/next_experience_id")? {
            if let Ok(next_id) = String::from_utf8_lossy(&value).parse::<u64>() {
                out.next_id = out.next_id.max(next_id);
            }
        }

        Ok(out)
    }
}

fn serialize_record(record: &ExperienceRecord) -> String {
    let stored_memory_id = record
        .stored_memory_id
        .map(|id| id.to_string())
        .unwrap_or_default();
    let contradictions = record
        .contradictions
        .iter()
        .map(|item| escape_field(item))
        .collect::<Vec<_>>()
        .join("|");

    format!(
        "{}\t{}\t{:.6}\t{}\t{:.6}\t{}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}",
        record.id,
        profile_to_str(record.profile),
        record.quality,
        stored_memory_id,
        record.router_threshold_after,
        record.stream_windows,
        record.hierarchy.global,
        record.hierarchy.local,
        record.hierarchy.convolution,
        escape_field(&record.prompt),
        escape_field(&record.lesson),
        contradictions
    )
}

fn deserialize_record(line: &str) -> Option<ExperienceRecord> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() != 12 {
        return None;
    }

    let id = fields[0].parse::<u64>().ok()?;
    let profile = str_to_profile(fields[1])?;
    let quality = fields[2].parse::<f32>().ok()?;
    let stored_memory_id = if fields[3].is_empty() {
        None
    } else {
        Some(fields[3].parse::<u64>().ok()?)
    };
    let router_threshold_after = fields[4].parse::<f32>().ok()?;
    let stream_windows = fields[5].parse::<usize>().ok()?;
    let hierarchy = HierarchyWeights::new(
        fields[6].parse::<f32>().ok()?,
        fields[7].parse::<f32>().ok()?,
        fields[8].parse::<f32>().ok()?,
    );
    let prompt = unescape_field(fields[9]);
    let lesson = unescape_field(fields[10]);
    let contradictions = if fields[11].is_empty() {
        Vec::new()
    } else {
        fields[11].split('|').map(unescape_field).collect()
    };

    Some(ExperienceRecord {
        id,
        prompt,
        profile,
        lesson,
        quality,
        contradictions,
        stored_memory_id,
        router_threshold_after,
        stream_windows,
        hierarchy,
    })
}

fn profile_to_str(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn str_to_profile(value: &str) -> Option<TaskProfile> {
    match value {
        "general" => Some(TaskProfile::General),
        "coding" => Some(TaskProfile::Coding),
        "writing" => Some(TaskProfile::Writing),
        "long_document" => Some(TaskProfile::LongDocument),
        _ => None,
    }
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('|', "\\p")
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
            Some('p') => out.push('|'),
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
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn records_and_ranks_lessons() {
        let mut store = ExperienceStore::new();
        store.record(input("weak", 0.2));
        store.record(input("strong", 0.9));

        let lessons = store.top_lessons(0.5, 4);

        assert_eq!(lessons.len(), 1);
        assert_eq!(lessons[0].lesson, "strong");
    }

    #[test]
    fn disk_kv_roundtrip_preserves_experience() {
        let path = temp_path("experience");
        let mut store = ExperienceStore::new();
        let id = store.record(input("stored", 0.87));

        store.save_to_disk_kv(&path).unwrap();
        let loaded = ExperienceStore::load_from_disk_kv(&path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.records()[0].id, id);
        assert_eq!(loaded.records()[0].lesson, "stored");
        assert_eq!(loaded.records()[0].profile, TaskProfile::Coding);
        cleanup(path);
    }

    fn input(lesson: &str, quality: f32) -> ExperienceInput {
        ExperienceInput {
            prompt: "build a Noiron loop".to_owned(),
            profile: TaskProfile::Coding,
            lesson: lesson.to_owned(),
            quality,
            contradictions: Vec::new(),
            stored_memory_id: Some(42),
            router_threshold_after: 0.55,
            stream_windows: 3,
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        }
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
}
