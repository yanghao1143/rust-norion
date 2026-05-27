use std::collections::HashSet;
use std::io;
use std::path::Path;

use crate::disk_kv::DiskKvStore;
use crate::gist_memory::{GistLevel, GistRecord};
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::process_reward::{ProcessRewardComponents, ProcessRewardReport, RewardAction};
use crate::router::RouteBudget;

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
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub used_memory_ids: Vec<u64>,
    pub gist_records: Vec<GistRecord>,
    pub gist_memory_ids: Vec<u64>,
    pub stored_runtime_kv_memory_ids: Vec<u64>,
    pub process_reward: ProcessRewardReport,
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
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub used_memory_ids: Vec<u64>,
    pub gist_records: Vec<GistRecord>,
    pub gist_memory_ids: Vec<u64>,
    pub stored_runtime_kv_memory_ids: Vec<u64>,
    pub process_reward: ProcessRewardReport,
}

#[derive(Debug, Clone)]
pub struct ExperienceMatch {
    pub id: u64,
    pub prompt: String,
    pub lesson: String,
    pub quality: f32,
    pub score: f32,
    pub gist_hints: Vec<String>,
    pub process_reward: f32,
    pub reward_action: RewardAction,
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
            route_budget: input.route_budget,
            hierarchy: input.hierarchy,
            used_memory_ids: input.used_memory_ids,
            gist_records: input.gist_records,
            gist_memory_ids: input.gist_memory_ids,
            stored_runtime_kv_memory_ids: input.stored_runtime_kv_memory_ids,
            process_reward: input.process_reward,
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

    pub fn retrieve_lessons(
        &self,
        prompt: &str,
        profile: TaskProfile,
        limit: usize,
    ) -> Vec<ExperienceMatch> {
        let mut matches = self
            .records
            .iter()
            .filter_map(|record| {
                let gist_text = record
                    .gist_records
                    .iter()
                    .map(|gist| format!("{} {}", gist.title, gist.summary))
                    .collect::<Vec<_>>()
                    .join(" ");
                let overlap = lexical_overlap(
                    prompt,
                    &format!("{} {} {}", record.prompt, record.lesson, gist_text),
                );
                let profile_bonus = if record.profile == profile { 0.16 } else { 0.0 };
                let gist_bonus = record
                    .gist_records
                    .iter()
                    .map(|gist| gist.importance)
                    .fold(0.0, f32::max)
                    * 0.08;
                let reward_bonus = record.process_reward.total * 0.08;
                let contradiction_penalty = (record.contradictions.len() as f32 * 0.08).min(0.32);
                let score = (overlap * 0.52
                    + record.quality * 0.36
                    + profile_bonus
                    + gist_bonus
                    + reward_bonus
                    - contradiction_penalty)
                    .clamp(0.0, 1.0);

                if score < 0.12 {
                    return None;
                }

                Some(ExperienceMatch {
                    id: record.id,
                    prompt: record.prompt.clone(),
                    lesson: record.lesson.clone(),
                    quality: record.quality,
                    score,
                    gist_hints: record
                        .gist_records
                        .iter()
                        .take(3)
                        .map(GistRecord::hint)
                        .collect(),
                    process_reward: record.process_reward.total,
                    reward_action: record.process_reward.action,
                })
            })
            .collect::<Vec<_>>();

        matches.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(limit);
        matches
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
    let gist_records = serialize_gists(&record.gist_records);
    let gist_memory_ids = serialize_ids(&record.gist_memory_ids);
    let used_memory_ids = serialize_ids(&record.used_memory_ids);
    let stored_runtime_kv_memory_ids = serialize_ids(&record.stored_runtime_kv_memory_ids);
    let process_reward = serialize_process_reward(&record.process_reward);
    let route_budget = serialize_route_budget(record.route_budget);

    format!(
        "{}\t{}\t{:.6}\t{}\t{:.6}\t{}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
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
        contradictions,
        escape_field(&gist_records),
        escape_field(&gist_memory_ids),
        escape_field(&process_reward),
        escape_field(&route_budget),
        escape_field(&used_memory_ids),
        escape_field(&stored_runtime_kv_memory_ids)
    )
}

fn deserialize_record(line: &str) -> Option<ExperienceRecord> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() < 12 {
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
    let gist_records = fields
        .get(12)
        .map(|value| deserialize_gists(&unescape_field(value)))
        .unwrap_or_default();
    let gist_memory_ids = fields
        .get(13)
        .map(|value| deserialize_ids(&unescape_field(value)))
        .unwrap_or_default();
    let process_reward = fields
        .get(14)
        .and_then(|value| deserialize_process_reward(&unescape_field(value)))
        .unwrap_or_default();
    let route_budget = fields
        .get(15)
        .and_then(|value| deserialize_route_budget(&unescape_field(value)))
        .unwrap_or(RouteBudget {
            threshold: router_threshold_after,
            attention_tokens: 0,
            fast_tokens: 0,
            attention_fraction: 0.0,
        });
    let used_memory_ids = fields
        .get(16)
        .map(|value| deserialize_ids(&unescape_field(value)))
        .unwrap_or_default();
    let stored_runtime_kv_memory_ids = fields
        .get(17)
        .map(|value| deserialize_ids(&unescape_field(value)))
        .unwrap_or_default();

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
        route_budget,
        hierarchy,
        used_memory_ids,
        gist_records,
        gist_memory_ids,
        stored_runtime_kv_memory_ids,
        process_reward,
    })
}

fn serialize_route_budget(route_budget: RouteBudget) -> String {
    format!(
        "{:.6},{},{},{:.6}",
        route_budget.threshold,
        route_budget.attention_tokens,
        route_budget.fast_tokens,
        route_budget.attention_fraction
    )
}

fn deserialize_route_budget(value: &str) -> Option<RouteBudget> {
    let fields = value.split(',').collect::<Vec<_>>();
    if fields.len() != 4 {
        return None;
    }

    Some(RouteBudget {
        threshold: fields[0].parse::<f32>().ok()?,
        attention_tokens: fields[1].parse::<usize>().ok()?,
        fast_tokens: fields[2].parse::<usize>().ok()?,
        attention_fraction: fields[3].parse::<f32>().ok()?.clamp(0.0, 1.0),
    })
}

fn serialize_gists(records: &[GistRecord]) -> String {
    records
        .iter()
        .map(|record| {
            [
                record.level.as_str().to_owned(),
                format!("{:.6}", record.importance),
                record.source_tokens.to_string(),
                sanitize_gist_part(&record.title),
                sanitize_gist_part(&record.summary),
            ]
            .join("\u{1f}")
        })
        .collect::<Vec<_>>()
        .join("\u{1e}")
}

fn deserialize_gists(value: &str) -> Vec<GistRecord> {
    if value.is_empty() {
        return Vec::new();
    }

    value
        .split('\u{1e}')
        .filter_map(|item| {
            let fields = item.split('\u{1f}').collect::<Vec<_>>();
            if fields.len() != 5 {
                return None;
            }

            Some(GistRecord {
                level: GistLevel::from_str(fields[0])?,
                importance: fields[1].parse::<f32>().ok()?.clamp(0.0, 1.0),
                source_tokens: fields[2].parse::<usize>().ok()?,
                title: fields[3].to_owned(),
                summary: fields[4].to_owned(),
            })
        })
        .collect()
}

fn serialize_ids(ids: &[u64]) -> String {
    ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
}

fn deserialize_ids(value: &str) -> Vec<u64> {
    value
        .split(',')
        .filter_map(|item| item.parse::<u64>().ok())
        .collect()
}

fn serialize_process_reward(report: &ProcessRewardReport) -> String {
    let notes = report
        .notes
        .iter()
        .map(|note| sanitize_control_part(note))
        .collect::<Vec<_>>()
        .join("\u{1e}");
    [
        format!("{:.6}", report.total),
        report.action.as_str().to_owned(),
        format!("{:.6}", report.components.route),
        format!("{:.6}", report.components.memory),
        format!("{:.6}", report.components.hierarchy),
        format!("{:.6}", report.components.reflection),
        format!("{:.6}", report.components.latency),
        format!("{:.6}", report.components.admission),
        notes,
    ]
    .join("\u{1f}")
}

fn deserialize_process_reward(value: &str) -> Option<ProcessRewardReport> {
    if value.is_empty() {
        return Some(ProcessRewardReport::default());
    }

    let fields = value.split('\u{1f}').collect::<Vec<_>>();
    if fields.len() != 9 {
        return None;
    }

    let notes = if fields[8].is_empty() {
        Vec::new()
    } else {
        fields[8].split('\u{1e}').map(ToOwned::to_owned).collect()
    };

    Some(ProcessRewardReport {
        total: fields[0].parse::<f32>().ok()?.clamp(0.0, 1.0),
        action: RewardAction::from_str(fields[1])?,
        components: ProcessRewardComponents {
            route: fields[2].parse::<f32>().ok()?.clamp(0.0, 1.0),
            memory: fields[3].parse::<f32>().ok()?.clamp(0.0, 1.0),
            hierarchy: fields[4].parse::<f32>().ok()?.clamp(0.0, 1.0),
            reflection: fields[5].parse::<f32>().ok()?.clamp(0.0, 1.0),
            latency: fields[6].parse::<f32>().ok()?.clamp(0.0, 1.0),
            admission: fields[7].parse::<f32>().ok()?.clamp(0.0, 1.0),
        },
        notes,
    })
}

fn sanitize_gist_part(value: &str) -> String {
    sanitize_control_part(value)
}

fn sanitize_control_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\u{1e}' | '\u{1f}' | '\t' | '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
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

fn lexical_overlap(left: &str, right: &str) -> f32 {
    let left_chars = left
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .collect::<HashSet<_>>();
    let right_chars = right
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .collect::<HashSet<_>>();

    if left_chars.is_empty() || right_chars.is_empty() {
        return 0.0;
    }

    let shared = left_chars.intersection(&right_chars).count() as f32;
    let denom = left_chars.len().min(right_chars.len()) as f32;
    (shared / denom).clamp(0.0, 1.0)
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
    fn retrieves_relevant_lessons() {
        let mut store = ExperienceStore::new();
        store.record(ExperienceInput {
            prompt: "Rust adaptive router".to_owned(),
            lesson: "prefer token-window feedback for router stability".to_owned(),
            ..input("router", 0.9)
        });
        store.record(ExperienceInput {
            prompt: "long form story writing".to_owned(),
            profile: TaskProfile::Writing,
            lesson: "prefer global continuity".to_owned(),
            ..input("writing", 0.9)
        });

        let matches = store.retrieve_lessons("Rust router feedback", TaskProfile::Coding, 2);

        assert!(!matches.is_empty());
        assert!(matches[0].lesson.contains("router"));
    }

    #[test]
    fn disk_kv_roundtrip_preserves_experience() {
        let path = temp_path("experience");
        let mut store = ExperienceStore::new();
        let id = store.record(ExperienceInput {
            gist_records: vec![gist("document", GistLevel::Document, 0.88)],
            gist_memory_ids: vec![7, 8],
            ..input("stored", 0.87)
        });

        store.save_to_disk_kv(&path).unwrap();
        let loaded = ExperienceStore::load_from_disk_kv(&path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.records()[0].id, id);
        assert_eq!(loaded.records()[0].lesson, "stored");
        assert_eq!(loaded.records()[0].profile, TaskProfile::Coding);
        assert_eq!(loaded.records()[0].gist_records.len(), 1);
        assert_eq!(loaded.records()[0].gist_memory_ids, vec![7, 8]);
        assert_eq!(loaded.records()[0].used_memory_ids, vec![3, 5]);
        assert_eq!(loaded.records()[0].stored_runtime_kv_memory_ids, vec![11]);
        assert!((loaded.records()[0].route_budget.attention_fraction - 0.4).abs() < 0.0001);
        assert!((loaded.records()[0].process_reward.total - 0.5).abs() < 0.0001);
        cleanup(path);
    }

    #[test]
    fn retrieve_lessons_includes_gist_hints() {
        let mut store = ExperienceStore::new();
        store.record(ExperienceInput {
            prompt: "long context scheduler".to_owned(),
            lesson: "reuse recursive chunk summaries".to_owned(),
            gist_records: vec![gist(
                "recursive chunks preserve overlap",
                GistLevel::Section,
                0.91,
            )],
            ..input("gist", 0.9)
        });

        let matches = store.retrieve_lessons("recursive overlap", TaskProfile::Coding, 1);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].gist_hints.len(), 1);
        assert!(matches[0].gist_hints[0].contains("recursive chunks"));
        assert_eq!(matches[0].reward_action, RewardAction::Hold);
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
            route_budget: RouteBudget {
                threshold: 0.55,
                attention_tokens: 2,
                fast_tokens: 3,
                attention_fraction: 0.4,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: vec![3, 5],
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: vec![11],
            process_reward: ProcessRewardReport::default(),
        }
    }

    fn gist(summary: &str, level: GistLevel, importance: f32) -> GistRecord {
        GistRecord {
            level,
            title: "gist title".to_owned(),
            summary: summary.to_owned(),
            source_tokens: 8,
            importance,
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
