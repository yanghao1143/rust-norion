use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::gist_memory::{GistLevel, GistRecord};
use crate::process_reward::RewardAction;

use super::evidence::ExperienceEvidenceNote;
use super::model::ExperienceRecord;
use super::noise::{
    ExperienceRetrievalNoise, retrieval_noise, strip_reflection_lesson_suffix,
    strip_response_lesson_prefix, strip_reusable_text_prefixes, text_has_metadata_lesson_shape,
    text_has_transcript_shape,
};
use super::relevance::{is_cjk_punctuation, is_signal_char};
use super::text_normalize::{normalize_full_width_ascii, normalize_full_width_ascii_char};

const PROMPT_INDEX_CHARS: usize = 960;
const LESSON_INDEX_CHARS: usize = 960;
const INDEX_SKETCH_CHARS: usize = 256;
const ADMISSION_NOTE_TRIGGER_CHARS: usize = 2_400;
const DUPLICATE_REFERENCE_PREVIEW_CHARS: usize = 220;
const DUPLICATE_REFERENCE_QUALITY_CAP: f32 = 0.72;
const RUNTIME_BACKEND_ERROR_QUALITY_CAP: f32 = 0.62;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ExperienceIndexDocument {
    pub text: String,
    pub compacted: bool,
    pub noise_penalty: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperienceIndexFinding {
    pub experience_id: u64,
    pub reason: String,
    pub compacted: bool,
    pub noise_penalty: f32,
    pub duplicate_of: Option<u64>,
    pub prompt_chars: usize,
    pub lesson_chars: usize,
    pub prompt_preview: String,
    pub lesson_preview: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperienceIndexReport {
    pub total_records: usize,
    pub compacted_record_count: usize,
    pub overlong_record_count: usize,
    pub overlong_without_clean_gist_count: usize,
    pub max_record_chars: usize,
    pub noisy_record_count: usize,
    pub duplicate_output_count: usize,
    pub max_noise_penalty: f32,
    pub quality_score: f32,
    pub retrieval_ready: bool,
    pub risk_level: String,
    pub recommended_action: String,
    pub findings: Vec<ExperienceIndexFinding>,
}

impl Default for ExperienceIndexReport {
    fn default() -> Self {
        Self {
            total_records: 0,
            compacted_record_count: 0,
            overlong_record_count: 0,
            overlong_without_clean_gist_count: 0,
            max_record_chars: 0,
            noisy_record_count: 0,
            duplicate_output_count: 0,
            max_noise_penalty: 0.0,
            quality_score: 1.0,
            retrieval_ready: true,
            risk_level: "clean".to_owned(),
            recommended_action: "seed_experience".to_owned(),
            findings: Vec::new(),
        }
    }
}

pub(super) fn inspect_records(records: &[ExperienceRecord], limit: usize) -> ExperienceIndexReport {
    let duplicate_outputs = duplicate_output_map(records);
    let mut report = ExperienceIndexReport {
        total_records: records.len(),
        ..ExperienceIndexReport::default()
    };
    for record in records {
        let duplicate_of = duplicate_outputs.get(&record.id).copied();
        let assessment = assess_record_index(record, duplicate_of);
        if assessment.compacted {
            report.compacted_record_count += 1;
        }
        if assessment.overlong {
            report.overlong_record_count += 1;
        }
        if assessment.overlong_without_clean_gist {
            report.overlong_without_clean_gist_count += 1;
        }
        report.max_record_chars = report.max_record_chars.max(assessment.record_chars);
        if duplicate_of.is_some() {
            report.duplicate_output_count += 1;
        }
        if assessment.noise_penalty > 0.0 {
            report.noisy_record_count += 1;
            report.max_noise_penalty = report.max_noise_penalty.max(assessment.noise_penalty);
            report.findings.push(index_finding(record, &assessment));
        }
    }
    report.findings.sort_by(|left, right| {
        right
            .noise_penalty
            .partial_cmp(&left.noise_penalty)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.experience_id.cmp(&right.experience_id))
    });
    report.findings.truncate(limit);
    report.refresh_quality_signal();
    report
}

impl ExperienceIndexReport {
    fn refresh_quality_signal(&mut self) {
        let total = self.total_records.max(1) as f32;
        let noisy_ratio = self.noisy_record_count as f32 / total;
        let duplicate_ratio = self.duplicate_output_count as f32 / total;
        let compacted_ratio = self.compacted_record_count as f32 / total;
        let overlong_without_gist_ratio = self.overlong_without_clean_gist_count as f32 / total;
        self.quality_score = (1.0
            - self.max_noise_penalty
            - noisy_ratio * 0.35
            - duplicate_ratio * 0.25
            - overlong_without_gist_ratio * 0.12
            - compacted_ratio * 0.05)
            .clamp(0.0, 1.0);
        self.risk_level = index_risk_level(self).to_owned();
        self.retrieval_ready = self.risk_level != "blocked"
            && (self.total_records == 0 || self.noisy_record_count < self.total_records)
            && self.max_noise_penalty < 0.45;
        self.recommended_action = index_recommended_action(self).to_owned();
    }
}

fn index_risk_level(report: &ExperienceIndexReport) -> &'static str {
    if report.quality_score >= 0.92
        && report.noisy_record_count == 0
        && report.duplicate_output_count == 0
        && report.overlong_without_clean_gist_count == 0
    {
        "clean"
    } else if report.quality_score >= 0.75 && report.max_noise_penalty < 0.25 {
        "watch"
    } else if report.quality_score >= 0.50 && report.max_noise_penalty < 0.45 {
        "degraded"
    } else {
        "blocked"
    }
}

fn index_recommended_action(report: &ExperienceIndexReport) -> &'static str {
    if report.total_records == 0 {
        return "seed_experience";
    }
    if !report.retrieval_ready || report.risk_level == "blocked" {
        if report.overlong_without_clean_gist_count > 0 {
            return "pause_chat_and_add_clean_gists";
        }
        if report.duplicate_output_count > 0 {
            return "pause_chat_and_deduplicate_outputs";
        }
        return "pause_chat_and_run_cleanup_audit";
    }
    if report.duplicate_output_count > 0 {
        return "deduplicate_repeated_lessons";
    }
    if report.overlong_without_clean_gist_count > 0 {
        return "add_clean_gists_for_long_records";
    }
    if report.noisy_record_count > 0 || report.max_noise_penalty > 0.0 {
        return "review_index_findings";
    }
    "ready_for_retrieval"
}

pub(super) fn record_index_document(record: &ExperienceRecord) -> ExperienceIndexDocument {
    let assessment = assess_record_index(record, None);
    let prompt = compact_index_segment("prompt", &record.prompt, PROMPT_INDEX_CHARS);
    let lesson = compact_index_segment("lesson", &record.lesson, LESSON_INDEX_CHARS);
    let sketch = index_sketch(&[record.prompt.as_str(), record.lesson.as_str()]);

    ExperienceIndexDocument {
        text: format!("{}\n{}\nindex_sketch:{}", prompt.text, lesson.text, sketch),
        compacted: assessment.compacted,
        noise_penalty: assessment.noise_penalty,
    }
}

pub(super) fn apply_admission_index_note(record: &mut ExperienceRecord) {
    let prompt_chars = record.prompt.chars().count();
    let lesson_chars = record.lesson.chars().count();
    let record_chars = prompt_chars + lesson_chars;
    let overlong =
        prompt_chars > ADMISSION_NOTE_TRIGGER_CHARS || lesson_chars > ADMISSION_NOTE_TRIGGER_CHARS;
    if !overlong {
        return;
    }
    if record
        .process_reward
        .notes
        .iter()
        .any(|note| is_experience_index_note(note))
    {
        return;
    }

    let document = record_index_document(record);
    let retrieval_noise = retrieval_noise(record);
    record.process_reward.notes.push(format!(
        "experience_index:compacted={}:overlong={}:overlong_without_clean_gist={}:record_chars={}:prompt_chars={}:lesson_chars={}:prompt_index_chars={}:lesson_index_chars={}:noise_penalty={:.3}",
        document.compacted,
        overlong,
        !retrieval_noise.has_clean_gist,
        record_chars,
        prompt_chars,
        lesson_chars,
        PROMPT_INDEX_CHARS,
        LESSON_INDEX_CHARS,
        document.noise_penalty
    ));
}

pub(super) fn apply_runtime_backend_error_clean_gist(record: &mut ExperienceRecord) {
    let Some(clean_gist) = runtime_backend_error_clean_gist(record) else {
        return;
    };
    ensure_clean_gist(
        record,
        "Runtime backend error guard",
        &clean_gist,
        RUNTIME_BACKEND_ERROR_QUALITY_CAP,
    );
    record.quality = record.quality.min(RUNTIME_BACKEND_ERROR_QUALITY_CAP);
    record.process_reward.total = record
        .process_reward
        .total
        .min(RUNTIME_BACKEND_ERROR_QUALITY_CAP);
    record.process_reward.action = RewardAction::Hold;
    if !record
        .process_reward
        .notes
        .iter()
        .any(|note| experience_index_note_has_tag(note, "runtime_backend_error_clean_gist"))
    {
        record
            .process_reward
            .notes
            .push("experience_index:runtime_backend_error_clean_gist".to_owned());
    }
}

pub(super) fn apply_generated_response_clean_gist(record: &mut ExperienceRecord) {
    if retrieval_noise(record).has_clean_gist {
        return;
    }
    let overlong = record.prompt.chars().count() > ADMISSION_NOTE_TRIGGER_CHARS
        || record.lesson.chars().count() > ADMISSION_NOTE_TRIGGER_CHARS;
    if !overlong {
        return;
    }
    let Some(clean_gist) = generated_response_clean_gist(&record.lesson) else {
        return;
    };
    ensure_clean_gist(
        record,
        "Generated response clean gist",
        &clean_gist,
        record.quality.max(0.72),
    );
    if !record
        .process_reward
        .notes
        .iter()
        .any(|note| experience_index_note_has_tag(note, "generated_response_clean_gist"))
    {
        record
            .process_reward
            .notes
            .push("experience_index:generated_response_clean_gist".to_owned());
    }
}

fn generated_response_clean_gist(lesson: &str) -> Option<String> {
    let trimmed = lesson.trim();
    let body = strip_response_lesson_prefix(trimmed)?;
    let body = strip_reflection_lesson_suffix(body);
    let body = strip_reusable_text_prefixes(body);
    if body.is_empty() || text_has_transcript_shape(body) || text_has_metadata_lesson_shape(body) {
        return None;
    }
    let clean = compact_preview(body, 300);
    let signal_chars = clean
        .chars()
        .filter(|ch| is_signal_char(*ch))
        .take(12)
        .count();
    (signal_chars >= 12).then_some(clean)
}

pub(super) fn runtime_backend_error_clean_gist(record: &ExperienceRecord) -> Option<String> {
    if retrieval_noise(record).has_clean_gist {
        return None;
    }
    let lesson = record.lesson.to_ascii_lowercase();
    if !lesson.contains("runtime backend error") {
        return None;
    }
    if lesson.contains("exceeds the available context size") || lesson.contains("context size") {
        return Some(
            "Runtime backend rejected an oversized prompt; compact report and pool context before retrying the selected small worker."
                .to_owned(),
        );
    }
    if lesson.contains("timed out") || lesson.contains("timeout") {
        return Some(
            "Runtime backend timed out; retry with smaller prompt context or route the task to the quality worker."
                .to_owned(),
        );
    }
    Some(
        "Runtime backend failed; keep the failure as diagnostic evidence and avoid retrieving the raw error as normal guidance."
            .to_owned(),
    )
}

pub(super) fn runtime_backend_error_index_repair_clean_gist(
    record: &ExperienceRecord,
) -> Option<String> {
    let clean_gist = runtime_backend_error_clean_gist(record)?;
    let assessment = assess_record_index(record, None);
    assessment.overlong_without_clean_gist.then_some(clean_gist)
}

pub(super) fn generated_response_index_repair_clean_gist(
    record: &ExperienceRecord,
) -> Option<String> {
    if retrieval_noise(record).has_clean_gist {
        return None;
    }
    let clean_gist = generated_response_clean_gist(&record.lesson)?;
    let assessment = assess_record_index(record, None);
    assessment.overlong_without_clean_gist.then_some(clean_gist)
}

pub(super) fn ensure_clean_gist(
    record: &mut ExperienceRecord,
    title: &str,
    summary: &str,
    importance: f32,
) -> bool {
    if record
        .gist_records
        .iter()
        .any(|gist| gist.summary == summary)
    {
        return false;
    }
    record.gist_records.push(GistRecord {
        level: GistLevel::Document,
        title: title.to_owned(),
        summary: summary.to_owned(),
        source_tokens: approximate_record_tokens(record).max(1),
        importance: importance.clamp(0.0, 1.0),
    });
    true
}

pub(super) fn apply_admission_duplicate_guard(
    record: &mut ExperienceRecord,
    existing_records: &[ExperienceRecord],
) -> Option<u64> {
    let key = duplicate_output_key(&record.lesson)?;
    let canonical_id = existing_records.iter().find_map(|existing| {
        duplicate_output_key(&existing.lesson)
            .filter(|existing_key| existing_key == &key)
            .map(|_| existing.id)
    })?;
    if record
        .process_reward
        .notes
        .iter()
        .any(|note| parse_duplicate_reference_note(note).is_some())
    {
        return Some(canonical_id);
    }

    let original_lesson_chars = record.lesson.chars().count();
    let preview = compact_preview(&record.lesson, DUPLICATE_REFERENCE_PREVIEW_CHARS);
    record.lesson = format!(
        "duplicate_reference: canonical_experience_id={canonical_id}; original_lesson_chars={original_lesson_chars}; preview={preview}"
    );
    record.quality = record.quality.min(DUPLICATE_REFERENCE_QUALITY_CAP);
    record.process_reward.total = record
        .process_reward
        .total
        .min(DUPLICATE_REFERENCE_QUALITY_CAP);
    record.process_reward.notes.push(format!(
        "experience_index:duplicate_reference:canonical_id={canonical_id}:original_lesson_chars={original_lesson_chars}:dedup_key_chars={}",
        key.chars().count()
    ));
    Some(canonical_id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompactIndexSegment {
    text: String,
    compacted: bool,
}

fn compact_index_segment(label: &str, value: &str, max_chars: usize) -> CompactIndexSegment {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return CompactIndexSegment {
            text: format!("{label}:{value}"),
            compacted: false,
        };
    }

    let head_chars = max_chars.saturating_mul(2) / 3;
    let tail_chars = max_chars.saturating_sub(head_chars);
    let head = value.chars().take(head_chars).collect::<String>();
    let tail = value
        .chars()
        .rev()
        .take(tail_chars)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    CompactIndexSegment {
        text: format!(
            "{label}:{head}\n[index_compacted label={label} original_chars={char_count}]\n{tail}"
        ),
        compacted: true,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ExperienceIndexAssessment {
    compacted: bool,
    noise_penalty: f32,
    reason: String,
    duplicate_of: Option<u64>,
    prompt_chars: usize,
    lesson_chars: usize,
    record_chars: usize,
    overlong: bool,
    overlong_without_clean_gist: bool,
}

fn assess_record_index(
    record: &ExperienceRecord,
    duplicate_of: Option<u64>,
) -> ExperienceIndexAssessment {
    let prompt_chars = record.prompt.chars().count();
    let lesson_chars = record.lesson.chars().count();
    let record_chars = prompt_chars + lesson_chars;
    let compacted = prompt_chars > PROMPT_INDEX_CHARS || lesson_chars > LESSON_INDEX_CHARS;
    let retrieval_noise = retrieval_noise(record);
    let duplicate_reference = duplicate_reference_canonical_id(record).is_some();
    let overlong =
        prompt_chars > ADMISSION_NOTE_TRIGGER_CHARS || lesson_chars > ADMISSION_NOTE_TRIGGER_CHARS;
    let overlong_without_clean_gist = overlong && !retrieval_noise.has_clean_gist;
    let unstructured_long = overlong_without_clean_gist;
    let transcript_like = !duplicate_reference
        && (is_transcript_like(&record.prompt) || is_transcript_like(&record.lesson));
    let (noise_penalty, reason) = if unstructured_long {
        let size_penalty = ((prompt_chars + lesson_chars) as f32 / 12_000.0).min(0.10);
        if transcript_like {
            (
                0.08 + size_penalty,
                "unstructured_long_transcript".to_owned(),
            )
        } else {
            (
                0.03 + size_penalty,
                "overlong_single_document_without_clean_gist".to_owned(),
            )
        }
    } else {
        (0.0, "clean".to_owned())
    };
    let (noise_penalty, reason) = merge_index_noise(
        noise_penalty,
        reason,
        if duplicate_reference {
            None
        } else {
            retrieval_index_noise(retrieval_noise)
        },
    );
    let (noise_penalty, reason) = if duplicate_of.is_some() {
        let reason = if noise_penalty > 0.0 {
            format!("duplicate_output+{reason}")
        } else {
            "duplicate_output".to_owned()
        };
        (noise_penalty.max(0.12), reason)
    } else {
        (noise_penalty, reason)
    };

    ExperienceIndexAssessment {
        compacted,
        noise_penalty,
        reason,
        duplicate_of,
        prompt_chars,
        lesson_chars,
        record_chars,
        overlong,
        overlong_without_clean_gist,
    }
}

fn retrieval_index_noise(noise: ExperienceRetrievalNoise) -> Option<(f32, &'static str)> {
    if noise.metadata_lesson_like {
        return if noise.has_clean_gist {
            Some((0.06, "legacy_metadata_lesson_clean_gist_fallback"))
        } else {
            Some((0.28, "legacy_metadata_lesson_missing_clean_gist"))
        };
    }
    if noise.lesson_transcript_like {
        return Some((0.22, "transcript_lesson"));
    }
    if noise.prompt_transcript_like && !noise.clean_lesson_like && !noise.has_clean_gist {
        return Some((0.12, "transcript_prompt_without_clean_lesson"));
    }
    None
}

fn merge_index_noise(
    current_penalty: f32,
    current_reason: String,
    extra: Option<(f32, &'static str)>,
) -> (f32, String) {
    let Some((extra_penalty, extra_reason)) = extra else {
        return (current_penalty, current_reason);
    };
    if current_penalty <= 0.0 {
        return (extra_penalty, extra_reason.to_owned());
    }
    (
        (current_penalty + extra_penalty).min(0.60),
        format!("{current_reason}+{extra_reason}"),
    )
}

fn index_finding(
    record: &ExperienceRecord,
    assessment: &ExperienceIndexAssessment,
) -> ExperienceIndexFinding {
    ExperienceIndexFinding {
        experience_id: record.id,
        reason: assessment.reason.clone(),
        compacted: assessment.compacted,
        noise_penalty: assessment.noise_penalty,
        duplicate_of: assessment.duplicate_of,
        prompt_chars: assessment.prompt_chars,
        lesson_chars: assessment.lesson_chars,
        prompt_preview: compact_preview(&record.prompt, 160),
        lesson_preview: compact_preview(&record.lesson, 160),
    }
}

fn duplicate_output_map(records: &[ExperienceRecord]) -> HashMap<u64, u64> {
    let mut canonical_by_lesson = HashMap::<String, u64>::new();
    let mut duplicates = HashMap::<u64, u64>::new();
    for record in records {
        if duplicate_reference_canonical_id(record).is_some() {
            continue;
        }

        let Some(key) = duplicate_output_key(&record.lesson) else {
            continue;
        };
        if let Some(canonical_id) = canonical_by_lesson.get(&key) {
            duplicates.insert(record.id, *canonical_id);
        } else {
            canonical_by_lesson.insert(key, record.id);
        }
    }
    duplicates
}

fn duplicate_reference_canonical_id(record: &ExperienceRecord) -> Option<u64> {
    record
        .process_reward
        .notes
        .iter()
        .find_map(|note| parse_duplicate_reference_note(note))
        .or_else(|| duplicate_reference_lesson_canonical_id(&record.lesson))
}

fn parse_duplicate_reference_note(note: &str) -> Option<u64> {
    let note = ExperienceEvidenceNote::parse(note)?;
    if !note.is_kind("experience_index") || !note.first_tag_matches("duplicate_reference") {
        return None;
    }
    note.field_normalized_ascii_trimmed("canonical_id")
        .or_else(|| note.field_normalized_ascii_trimmed("canonical_experience_id"))
        .and_then(|value| parse_canonical_id_value(&value))
}

fn experience_index_note_has_tag(note: &str, tag: &str) -> bool {
    ExperienceEvidenceNote::parse(note)
        .is_some_and(|note| note.is_kind("experience_index") && note.first_tag_matches(tag))
}

fn is_experience_index_note(note: &str) -> bool {
    ExperienceEvidenceNote::parse(note).is_some_and(|note| note.is_kind("experience_index"))
}

pub(super) fn duplicate_reference_lesson_canonical_id(lesson: &str) -> Option<u64> {
    let note = ExperienceEvidenceNote::parse(lesson)?;
    if !note.is_kind("duplicate_reference") {
        return None;
    }
    note.field_normalized_ascii_trimmed("canonical_id")
        .or_else(|| note.field_normalized_ascii_trimmed("canonical_experience_id"))
        .and_then(|value| parse_canonical_id_value(&value))
}

fn parse_canonical_id_value(value: &str) -> Option<u64> {
    let value = normalize_full_width_ascii(value);
    value
        .split(';')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .parse()
        .ok()
}

pub(super) fn duplicate_output_key(value: &str) -> Option<String> {
    if duplicate_reference_lesson_canonical_id(value).is_some() {
        return None;
    }

    let normalized = normalize_duplicate_output_key(value);
    (normalized.chars().count() >= 80).then_some(normalized)
}

fn approximate_record_tokens(record: &ExperienceRecord) -> usize {
    record
        .prompt
        .chars()
        .count()
        .saturating_add(record.lesson.chars().count())
        .div_ceil(4)
}

fn normalize_duplicate_output_key(value: &str) -> String {
    let mut normalized = String::new();
    let mut pending_separator = false;
    for ch in value.chars() {
        let ch = normalize_full_width_ascii_char(ch);
        if ch.is_whitespace() || ch.is_ascii_punctuation() {
            pending_separator = !normalized.is_empty();
            continue;
        }
        if is_cjk_punctuation(ch) {
            continue;
        }
        if pending_separator {
            if !normalized
                .chars()
                .last()
                .is_some_and(|previous| is_cjk_text_char(previous) && is_cjk_text_char(ch))
            {
                normalized.push(' ');
            }
            pending_separator = false;
        }
        normalized.extend(ch.to_lowercase());
    }
    normalized
}

fn is_cjk_text_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

fn index_sketch(parts: &[&str]) -> String {
    let mut seen = HashSet::new();
    let mut sketch = String::new();
    for ch in parts
        .iter()
        .flat_map(|part| part.chars())
        .filter(|ch| is_signal_char(*ch))
    {
        if seen.insert(ch) {
            sketch.push(ch);
        }
        if sketch.chars().count() >= INDEX_SKETCH_CHARS {
            break;
        }
    }
    sketch
}

fn is_transcript_like(value: &str) -> bool {
    text_has_transcript_shape(value)
}

fn compact_preview(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in value.chars().take(max_chars) {
        if ch.is_whitespace() {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}
