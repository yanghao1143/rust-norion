use std::collections::{HashMap, HashSet};

use super::evidence::ExperienceEvidenceNote;
use super::hygiene;
use super::index;
use super::model::ExperienceRecord;
use super::noise::{
    strip_at_case_insensitive_marker, strip_reflection_lesson_suffix, strip_reusable_text_prefixes,
    strip_reuse_response_prefix, text_has_metadata_lesson_shape,
    text_has_rejected_metadata_lesson_shape, text_has_role_labeled_lesson_residue,
    text_has_transcript_shape,
};
use super::relevance::is_signal_char;
use super::ExperienceStore;
use crate::process_reward::RewardAction;

const PREVIEW_CHARS: usize = 160;
const REPAIR_LESSON_CHARS: usize = 420;
const DUPLICATE_REFERENCE_QUALITY_CAP: f32 = 0.72;
const RUNTIME_BACKEND_ERROR_QUALITY_CAP: f32 = 0.62;
const GENERATED_RESPONSE_CLEAN_GIST_MIN_IMPORTANCE: f32 = 0.72;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperienceRepairAction {
    ReuseResponse,
    ReviseResponse,
    DedupeReference,
    StripTranscriptContext,
    AddCleanGist,
}

impl ExperienceRepairAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReuseResponse => "reuse_response",
            Self::ReviseResponse => "revise_response",
            Self::DedupeReference => "dedupe_reference",
            Self::StripTranscriptContext => "strip_transcript_context",
            Self::AddCleanGist => "add_clean_gist",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceRepairItem {
    pub experience_id: u64,
    pub action: ExperienceRepairAction,
    pub source: String,
    pub old_lesson_preview: String,
    pub proposed_lesson_preview: String,
    pub source_gist_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceRepairSkippedItem {
    pub experience_id: u64,
    pub reason: String,
    pub old_lesson_preview: String,
    pub prompt_preview: String,
    pub gist_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExperienceRepairPlan {
    pub total_records: usize,
    pub legacy_metadata_lesson_count: usize,
    pub repairable_legacy_metadata_lesson_count: usize,
    pub index_noisy_record_count: usize,
    pub index_duplicate_output_count: usize,
    pub repairable_index_record_count: usize,
    pub skipped_quarantine_candidate_count: usize,
    pub skipped_missing_clean_gist_count: usize,
    pub projected_after_repair: ExperienceRepairProjection,
    pub listed_repairs: Vec<ExperienceRepairItem>,
    pub listed_skipped_quarantine_candidates: Vec<ExperienceRepairSkippedItem>,
    pub listed_skipped_missing_clean_gist: Vec<ExperienceRepairSkippedItem>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExperienceRepairProjection {
    pub total_records: usize,
    pub hygiene_finding_count: usize,
    pub hygiene_watch_count: usize,
    pub hygiene_quarantine_candidate_count: usize,
    pub legacy_metadata_lesson_count: usize,
    pub legacy_metadata_without_clean_gist_count: usize,
    pub index_quality_score: f32,
    pub index_noisy_record_count: usize,
    pub index_duplicate_output_count: usize,
    pub index_retrieval_ready: bool,
    pub index_risk_level: String,
}

impl ExperienceRepairPlan {
    pub fn is_empty(&self) -> bool {
        self.repairable_legacy_metadata_lesson_count == 0 && self.repairable_index_record_count == 0
    }

    pub fn remaining_legacy_metadata_lesson_count_after_repair(&self) -> usize {
        self.projected_after_repair.legacy_metadata_lesson_count
    }

    pub fn remaining_watch_count_after_repair(&self) -> usize {
        self.projected_after_repair.hygiene_watch_count
    }

    pub fn remaining_quarantine_candidate_count_after_repair(&self) -> usize {
        self.projected_after_repair
            .hygiene_quarantine_candidate_count
    }
}

impl ExperienceStore {
    pub fn legacy_metadata_repair_plan(&self, limit: usize) -> ExperienceRepairPlan {
        legacy_metadata_repair_plan(&self.records, limit)
    }

    pub fn repaired_legacy_metadata_store(&self, limit: usize) -> (Self, ExperienceRepairPlan) {
        let plan = self.legacy_metadata_repair_plan(limit);
        if plan.is_empty() {
            return (self.clone(), plan);
        }

        let quarantine_ids = quarantine_candidate_ids(self);
        let repaired_records = repaired_records(&self.records, &quarantine_ids);

        (
            Self {
                records: repaired_records,
                next_id: self.next_id,
            },
            plan,
        )
    }
}

fn legacy_metadata_repair_plan(records: &[ExperienceRecord], limit: usize) -> ExperienceRepairPlan {
    let quarantine_ids = quarantine_candidate_ids_for_records(records);
    let index_repairs = index_repair_items(records, &quarantine_ids);
    let index_report = index::inspect_records(records, limit);
    let mut plan = ExperienceRepairPlan {
        total_records: records.len(),
        index_noisy_record_count: index_report.noisy_record_count,
        index_duplicate_output_count: index_report.duplicate_output_count,
        repairable_index_record_count: index_repairs.len(),
        ..ExperienceRepairPlan::default()
    };

    for record in records {
        if !text_has_metadata_lesson_shape(&record.lesson) {
            continue;
        }

        plan.legacy_metadata_lesson_count += 1;
        if quarantine_ids.contains(&record.id) {
            plan.skipped_quarantine_candidate_count += 1;
            push_skipped(
                &mut plan.listed_skipped_quarantine_candidates,
                skipped_item(record, "quarantine_candidate"),
                limit,
            );
            continue;
        }

        let Some(repair) = repair_item(record) else {
            plan.skipped_missing_clean_gist_count += 1;
            push_skipped(
                &mut plan.listed_skipped_missing_clean_gist,
                skipped_item(record, "missing_clean_gist"),
                limit,
            );
            continue;
        };

        plan.repairable_legacy_metadata_lesson_count += 1;
        if plan.listed_repairs.len() < limit {
            plan.listed_repairs.push(repair);
        }
    }

    for repair in index_repairs {
        if plan.listed_repairs.len() < limit
            && !plan
                .listed_repairs
                .iter()
                .any(|listed| listed.experience_id == repair.experience_id)
        {
            plan.listed_repairs.push(repair);
        }
    }

    plan.projected_after_repair = projected_after_repair(records, &quarantine_ids, limit);
    plan
}

fn projected_after_repair(
    records: &[ExperienceRecord],
    quarantine_ids: &HashSet<u64>,
    limit: usize,
) -> ExperienceRepairProjection {
    let repaired_records = repaired_records(records, quarantine_ids);

    let hygiene_report = hygiene::inspect_records(&repaired_records, limit);
    let index_report = index::inspect_records(&repaired_records, limit);
    ExperienceRepairProjection {
        total_records: hygiene_report.total_records,
        hygiene_finding_count: hygiene_report.finding_count,
        hygiene_watch_count: hygiene_report.watch_count,
        hygiene_quarantine_candidate_count: hygiene_report.quarantine_candidate_count,
        legacy_metadata_lesson_count: hygiene_report.legacy_metadata_lesson_count,
        legacy_metadata_without_clean_gist_count: hygiene_report
            .legacy_metadata_without_clean_gist_count,
        index_quality_score: index_report.quality_score,
        index_noisy_record_count: index_report.noisy_record_count,
        index_duplicate_output_count: index_report.duplicate_output_count,
        index_retrieval_ready: index_report.retrieval_ready,
        index_risk_level: index_report.risk_level,
    }
}

fn repaired_records(
    records: &[ExperienceRecord],
    quarantine_ids: &HashSet<u64>,
) -> Vec<ExperienceRecord> {
    let mut repaired_records = records.to_vec();
    let index_repairs = index_repair_items(records, quarantine_ids)
        .into_iter()
        .map(|repair| (repair.experience_id, repair))
        .collect::<HashMap<_, _>>();

    for record in &mut repaired_records {
        if quarantine_ids.contains(&record.id) {
            continue;
        }
        if let Some(repair) = repair_item(record) {
            apply_repair(
                record,
                &repair,
                "experience_repair:legacy_metadata_lesson:source=clean_gist",
            );
        }
        if let Some(repair) = index_repairs.get(&record.id) {
            apply_repair(record, repair, "experience_repair:index_quality");
        }
    }
    repaired_records
}

fn apply_repair(record: &mut ExperienceRecord, repair: &ExperienceRepairItem, note_prefix: &str) {
    record.lesson = match repair.action {
        ExperienceRepairAction::ReuseResponse | ExperienceRepairAction::ReviseResponse => {
            proposed_lesson(repair.action, &repair.source_gist_preview)
        }
        ExperienceRepairAction::AddCleanGist => {
            index::ensure_clean_gist(
                record,
                clean_gist_repair_title(repair),
                &repair.source_gist_preview,
                clean_gist_repair_importance(record, repair),
            );
            proposed_lesson(repair.action, &repair.source_gist_preview)
        }
        ExperienceRepairAction::DedupeReference
        | ExperienceRepairAction::StripTranscriptContext => repair.source_gist_preview.clone(),
    };
    if repair.action == ExperienceRepairAction::DedupeReference {
        record.quality = record.quality.min(DUPLICATE_REFERENCE_QUALITY_CAP);
        record.process_reward.total = record
            .process_reward
            .total
            .min(DUPLICATE_REFERENCE_QUALITY_CAP);
    }
    if repair.action == ExperienceRepairAction::StripTranscriptContext {
        record.process_reward.action = RewardAction::Hold;
    }
    if repair.action == ExperienceRepairAction::AddCleanGist
        && clean_gist_repair_caps_quality(repair)
    {
        record.quality = record.quality.min(RUNTIME_BACKEND_ERROR_QUALITY_CAP);
        record.process_reward.total = record
            .process_reward
            .total
            .min(RUNTIME_BACKEND_ERROR_QUALITY_CAP);
        record.process_reward.action = RewardAction::Hold;
    }
    if !record
        .process_reward
        .notes
        .iter()
        .any(|note| repair_note_matches_prefix(note, note_prefix))
    {
        record
            .process_reward
            .notes
            .push(format!("{note_prefix}:action={}", repair.action.as_str()));
    }
}

fn repair_note_matches_prefix(note: &str, note_prefix: &str) -> bool {
    let Some(note) = ExperienceEvidenceNote::parse(note) else {
        return false;
    };
    if !note.is_kind("experience_repair") {
        return false;
    }
    match note_prefix {
        "experience_repair:index_quality" => note.first_tag_matches("index_quality"),
        "experience_repair:legacy_metadata_lesson:source=clean_gist" => {
            note.first_tag_matches("legacy_metadata_lesson")
                && note_field_token_matches(&note, "source", "clean_gist")
        }
        _ => false,
    }
}

fn note_field_token_matches(note: &ExperienceEvidenceNote, key: &str, expected: &str) -> bool {
    note.field_normalized_ascii_trimmed(key)
        .and_then(|value| value.split(';').next().map(str::trim).map(str::to_owned))
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

fn quarantine_candidate_ids(store: &ExperienceStore) -> HashSet<u64> {
    store
        .hygiene_quarantine_plan(store.records.len().max(1))
        .candidate_ids
        .into_iter()
        .collect()
}

fn quarantine_candidate_ids_for_records(records: &[ExperienceRecord]) -> HashSet<u64> {
    ExperienceStore {
        records: records.to_vec(),
        next_id: records
            .iter()
            .map(|record| record.id + 1)
            .max()
            .unwrap_or(1),
    }
    .hygiene_quarantine_plan(records.len().max(1))
    .candidate_ids
    .into_iter()
    .collect()
}

fn repair_item(record: &ExperienceRecord) -> Option<ExperienceRepairItem> {
    if !text_has_metadata_lesson_shape(&record.lesson) {
        return None;
    }

    let action = repair_action(&record.lesson);
    let clean_gist = best_clean_gist(record)?;
    let proposed = proposed_lesson(action, &clean_gist);

    Some(ExperienceRepairItem {
        experience_id: record.id,
        action,
        source: "clean_gist".to_owned(),
        old_lesson_preview: compact_preview(&record.lesson, PREVIEW_CHARS),
        proposed_lesson_preview: compact_preview(&proposed, PREVIEW_CHARS),
        source_gist_preview: clean_gist,
    })
}

fn index_repair_items(
    records: &[ExperienceRecord],
    quarantine_ids: &HashSet<u64>,
) -> Vec<ExperienceRepairItem> {
    let duplicate_canonical = duplicate_output_canonical_map(records);
    let mut items = Vec::new();
    for record in records {
        if quarantine_ids.contains(&record.id) {
            continue;
        }
        if let Some(clean_gist) = index::runtime_backend_error_index_repair_clean_gist(record) {
            items.push(clean_gist_repair_item(
                record,
                "runtime_backend_error_without_clean_gist",
                clean_gist,
            ));
            continue;
        }
        if let Some(clean_gist) = index::generated_response_index_repair_clean_gist(record) {
            items.push(clean_gist_repair_item(
                record,
                "generated_response_without_clean_gist",
                clean_gist,
            ));
            continue;
        }
        if index::duplicate_reference_lesson_canonical_id(&record.lesson).is_some() {
            continue;
        }
        if let Some(canonical_id) = duplicate_canonical.get(&record.id) {
            let proposed = duplicate_reference_lesson(*canonical_id, &record.lesson);
            items.push(ExperienceRepairItem {
                experience_id: record.id,
                action: ExperienceRepairAction::DedupeReference,
                source: format!("canonical_experience_id={canonical_id}"),
                old_lesson_preview: compact_preview(&record.lesson, PREVIEW_CHARS),
                proposed_lesson_preview: compact_preview(&proposed, PREVIEW_CHARS),
                source_gist_preview: proposed,
            });
            continue;
        }
        if (text_has_transcript_shape(&record.lesson)
            || text_has_role_labeled_lesson_residue(&record.lesson))
            && let Some(clean_lesson) = transcript_lesson_repair(record)
        {
            items.push(ExperienceRepairItem {
                experience_id: record.id,
                action: ExperienceRepairAction::StripTranscriptContext,
                source: "reuse_response_without_reflection_transcript".to_owned(),
                old_lesson_preview: compact_preview(&record.lesson, PREVIEW_CHARS),
                proposed_lesson_preview: compact_preview(&clean_lesson, PREVIEW_CHARS),
                source_gist_preview: clean_lesson,
            });
        }
    }
    items
}

fn clean_gist_repair_item(
    record: &ExperienceRecord,
    source: &str,
    clean_gist: String,
) -> ExperienceRepairItem {
    ExperienceRepairItem {
        experience_id: record.id,
        action: ExperienceRepairAction::AddCleanGist,
        source: source.to_owned(),
        old_lesson_preview: compact_preview(&record.lesson, PREVIEW_CHARS),
        proposed_lesson_preview: compact_preview(
            &proposed_lesson(ExperienceRepairAction::AddCleanGist, &clean_gist),
            PREVIEW_CHARS,
        ),
        source_gist_preview: clean_gist,
    }
}

fn clean_gist_repair_title(repair: &ExperienceRepairItem) -> &'static str {
    if repair.source == "generated_response_without_clean_gist" {
        "Generated response clean gist"
    } else {
        "Runtime backend error guard"
    }
}

fn clean_gist_repair_importance(record: &ExperienceRecord, repair: &ExperienceRepairItem) -> f32 {
    if repair.source == "generated_response_without_clean_gist" {
        record
            .quality
            .max(GENERATED_RESPONSE_CLEAN_GIST_MIN_IMPORTANCE)
    } else {
        RUNTIME_BACKEND_ERROR_QUALITY_CAP
    }
}

fn clean_gist_repair_caps_quality(repair: &ExperienceRepairItem) -> bool {
    repair.source == "runtime_backend_error_without_clean_gist"
}

fn duplicate_output_canonical_map(records: &[ExperienceRecord]) -> HashMap<u64, u64> {
    let mut canonical_by_key = HashMap::<String, u64>::new();
    let mut duplicates = HashMap::<u64, u64>::new();
    for record in records {
        if index::duplicate_reference_lesson_canonical_id(&record.lesson).is_some() {
            continue;
        }
        let Some(key) = index::duplicate_output_key(&record.lesson) else {
            continue;
        };
        if let Some(canonical_id) = canonical_by_key.get(&key) {
            duplicates.insert(record.id, *canonical_id);
        } else {
            canonical_by_key.insert(key, record.id);
        }
    }
    duplicates
}

fn duplicate_reference_lesson(canonical_id: u64, lesson: &str) -> String {
    let original_lesson_chars = lesson.chars().count();
    format!(
        "duplicate_reference: canonical_experience_id={canonical_id}; original_lesson_chars={original_lesson_chars}; source_redacted=true"
    )
}

fn transcript_lesson_repair(record: &ExperienceRecord) -> Option<String> {
    let lesson = record.lesson.trim();
    let reuse_response = strip_reuse_response_prefix(lesson);
    let response = reuse_response.unwrap_or(lesson);
    let clean = strip_reflection_repair_suffix(response);
    let clean = strip_reusable_text_prefixes(clean);
    let clean = if reuse_response.is_some() {
        clean_reuse_response_text(clean)
    } else {
        clean_lesson_text(clean)
    }?;
    Some(format!("reuse_response: {clean}"))
}

fn strip_reflection_repair_suffix(value: &str) -> &str {
    strip_at_case_insensitive_marker(value, " reflection repair:")
}

fn push_skipped(
    items: &mut Vec<ExperienceRepairSkippedItem>,
    item: ExperienceRepairSkippedItem,
    limit: usize,
) {
    if items.len() < limit {
        items.push(item);
    }
}

fn skipped_item(record: &ExperienceRecord, reason: &str) -> ExperienceRepairSkippedItem {
    ExperienceRepairSkippedItem {
        experience_id: record.id,
        reason: reason.to_owned(),
        old_lesson_preview: compact_preview(&record.lesson, PREVIEW_CHARS),
        prompt_preview: compact_preview(&record.prompt, PREVIEW_CHARS),
        gist_count: record.gist_records.len(),
    }
}

fn repair_action(lesson: &str) -> ExperienceRepairAction {
    if text_has_rejected_metadata_lesson_shape(lesson) {
        ExperienceRepairAction::ReviseResponse
    } else {
        ExperienceRepairAction::ReuseResponse
    }
}

fn proposed_lesson(action: ExperienceRepairAction, clean_gist: &str) -> String {
    format!("{}: {}", action.as_str(), clean_gist)
}

fn best_clean_gist(record: &ExperienceRecord) -> Option<String> {
    record
        .gist_records
        .iter()
        .filter_map(|gist| {
            clean_lesson_text(&gist.summary).map(|summary| (gist.importance, summary))
        })
        .max_by(|left, right| {
            left.0
                .partial_cmp(&right.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, summary)| summary)
}

fn clean_lesson_text(value: &str) -> Option<String> {
    let trimmed = strip_reflection_lesson_suffix(value.trim());
    let trimmed = strip_reusable_text_prefixes(trimmed);
    if trimmed.is_empty()
        || text_has_metadata_lesson_shape(trimmed)
        || text_has_transcript_shape(trimmed)
    {
        return None;
    }

    let compact = compact_preview(trimmed, REPAIR_LESSON_CHARS);
    let signal_chars = compact
        .chars()
        .filter(|ch| is_signal_char(*ch))
        .take(12)
        .count();
    if signal_chars < 12 {
        return None;
    }

    Some(compact)
}

fn clean_reuse_response_text(value: &str) -> Option<String> {
    let trimmed = strip_reflection_lesson_suffix(value.trim());
    let trimmed = strip_reusable_text_prefixes(trimmed);
    if trimmed.is_empty()
        || text_has_metadata_lesson_shape(trimmed)
        || text_has_transcript_shape(trimmed)
    {
        return None;
    }

    let compact = compact_preview(trimmed, REPAIR_LESSON_CHARS);
    compact.chars().any(is_signal_char).then_some(compact)
}

fn compact_preview(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut previous_space = false;
    for ch in value.chars().take(max_chars) {
        if ch.is_whitespace() {
            if !previous_space {
                out.push(' ');
                previous_space = true;
            }
        } else {
            out.push(ch);
            previous_space = false;
        }
    }
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out.trim().to_owned()
}
