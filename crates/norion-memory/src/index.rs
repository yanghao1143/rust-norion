use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ExperienceEnvelope, IndexRebuildPlan, MemoryAdapter, MemoryAdapterCapability,
    MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryResult, MemoryScope, Metadata, clamp01,
};

const CLEAN_GIST_INDEX_MAX_CHARS: usize = 420;
const RAW_FALLBACK_INDEX_MAX_CHARS: usize = 1_200;
const RAW_FALLBACK_PROMPT_MAX_CHARS: usize = 420;
const RAW_FALLBACK_LESSON_MAX_CHARS: usize = 780;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryIndexSource {
    Experience,
    LongTerm,
    Skill,
    RuntimeKv,
}

impl MemoryIndexSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Experience => "experience",
            Self::LongTerm => "long_term",
            Self::Skill => "skill",
            Self::RuntimeKv => "runtime_kv",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryIndexDocument {
    pub id: String,
    pub source: MemoryIndexSource,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: Metadata,
    pub scope: MemoryScope,
    pub strength: f32,
}

impl MemoryIndexDocument {
    pub fn new(
        id: impl Into<String>,
        source: MemoryIndexSource,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source,
            content: content.into(),
            embedding: Vec::new(),
            metadata: Metadata::new(),
            scope: MemoryScope::default(),
            strength: 0.5,
        }
    }

    pub fn from_experience(envelope: &ExperienceEnvelope) -> Self {
        let projected = project_experience_index_content(envelope);
        let tags = projected_index_tags(envelope);
        let mut metadata = Metadata::new();
        if !tags.is_empty() {
            metadata.insert("tags".to_owned(), tags.join(","));
        }
        metadata.insert(
            "source".to_owned(),
            MemoryIndexSource::Experience.as_str().to_owned(),
        );
        metadata.insert("content_basis".to_owned(), projected.basis.to_owned());
        metadata.insert(
            "content_truncated".to_owned(),
            projected.truncated.to_string(),
        );
        if projected.basis == "raw_fallback" {
            metadata.insert(
                "prompt_chars".to_owned(),
                envelope.prompt.chars().count().to_string(),
            );
            metadata.insert(
                "lesson_chars".to_owned(),
                envelope.lesson.chars().count().to_string(),
            );
        }

        Self {
            id: envelope.id.clone(),
            source: MemoryIndexSource::Experience,
            content: projected.content,
            embedding: Vec::new(),
            metadata,
            scope: envelope.scope.clone(),
            strength: clamp01(envelope.quality),
        }
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = embedding;
        self
    }

    pub fn with_scope(mut self, scope: MemoryScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = clamp01(strength);
        self
    }
}

fn projected_index_tags(envelope: &ExperienceEnvelope) -> Vec<String> {
    let mut tags = envelope.tags.clone();
    let combined_chars = envelope.prompt.chars().count() + envelope.lesson.chars().count();
    let risky_without_gist = has_transcript_shape(&envelope.prompt)
        || has_transcript_shape(&envelope.lesson)
        || has_metadata_lesson_shape(&envelope.lesson)
        || combined_chars > RAW_FALLBACK_INDEX_MAX_CHARS;

    match envelope.clean_gist.as_deref() {
        Some(gist) if !is_index_clean_gist(gist) => tags.push("risk:dirty_clean_gist".to_owned()),
        None if risky_without_gist => tags.push("risk:missing_clean_gist".to_owned()),
        _ => {}
    }

    tags.sort();
    tags.dedup();
    tags
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExperienceIndexContent {
    content: String,
    basis: &'static str,
    truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryIndexOperationKind {
    Upsert,
    RefreshEmbedding,
    Compact,
    Quarantine,
    DeleteDuplicate,
}

impl MemoryIndexOperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upsert => "upsert",
            Self::RefreshEmbedding => "refresh_embedding",
            Self::Compact => "compact",
            Self::Quarantine => "quarantine",
            Self::DeleteDuplicate => "delete_duplicate",
        }
    }

    fn priority(self) -> u8 {
        match self {
            Self::Quarantine => 0,
            Self::DeleteDuplicate => 1,
            Self::Compact => 2,
            Self::RefreshEmbedding => 3,
            Self::Upsert => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryIndexOperation {
    pub source_id: String,
    pub source: MemoryIndexSource,
    pub kind: MemoryIndexOperationKind,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryIndexPlan {
    pub operations: Vec<MemoryIndexOperation>,
    pub skipped_ids: Vec<String>,
    pub reasons: Vec<String>,
}

impl MemoryIndexPlan {
    pub fn requires_rebuild(&self) -> bool {
        self.operations.iter().any(|operation| {
            operation.kind != MemoryIndexOperationKind::Upsert
                || self
                    .reasons
                    .iter()
                    .any(|reason| reason == "full_rebuild_requested")
        })
    }

    pub fn operations_by_kind(&self, kind: MemoryIndexOperationKind) -> Vec<&MemoryIndexOperation> {
        self.operations
            .iter()
            .filter(|operation| operation.kind == kind)
            .collect()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.reasons
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.operations
            .iter()
            .filter(|operation| operation.kind != MemoryIndexOperationKind::Upsert)
            .map(|operation| {
                format!(
                    "{}:{}:{}",
                    operation.kind.as_str(),
                    detail_reason(&operation.reason),
                    hex_id(&operation.source_id)
                )
            })
            .chain(
                self.skipped_ids
                    .iter()
                    .map(|id| format!("skipped:{}", hex_id(id))),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn operation_detail_codes_for_kind(&self, kind: MemoryIndexOperationKind) -> Vec<String> {
        let prefix = format!("{}:", kind.as_str());
        self.detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn operation_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{}:", detail_reason(reason));
        self.detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn skipped_detail_codes(&self) -> Vec<String> {
        self.detail_codes()
            .into_iter()
            .filter(|code| code.starts_with("skipped:"))
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_index_plan rebuild={} operations={} upsert={} refresh={} compact={} quarantine={} delete_duplicate={} skipped={} reasons={} reason_codes={} detail_codes={}",
            self.requires_rebuild(),
            self.operations.len(),
            self.operations_by_kind(MemoryIndexOperationKind::Upsert)
                .len(),
            self.operations_by_kind(MemoryIndexOperationKind::RefreshEmbedding)
                .len(),
            self.operations_by_kind(MemoryIndexOperationKind::Compact)
                .len(),
            self.operations_by_kind(MemoryIndexOperationKind::Quarantine)
                .len(),
            self.operations_by_kind(MemoryIndexOperationKind::DeleteDuplicate)
                .len(),
            self.skipped_ids.len(),
            self.reasons.len(),
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceIndexFindingProjection {
    pub source_id: String,
    pub root_reason_codes: Vec<String>,
    pub memory_index_rebuild_reason_codes: Vec<String>,
    pub memory_index_operation_reason_codes: Vec<String>,
    pub context_injection_reason_codes: Vec<String>,
}

impl ExperienceIndexFindingProjection {
    pub fn new(source_id: impl Into<String>, root_reason: &str) -> Self {
        let source_id = source_id.into();
        let root_reason_codes = split_root_index_reasons(root_reason);
        let mut memory_index_rebuild_reason_codes = Vec::new();
        let mut memory_index_operation_reason_codes = Vec::new();
        let mut context_injection_reason_codes = Vec::new();

        for reason in &root_reason_codes {
            memory_index_rebuild_reason_codes.extend(root_index_rebuild_reason_codes(reason));
            memory_index_operation_reason_codes.extend(root_index_operation_reason_codes(reason));
            context_injection_reason_codes.extend(root_index_context_reason_codes(reason));
        }

        sort_dedup_strings(&mut memory_index_rebuild_reason_codes);
        sort_dedup_strings(&mut memory_index_operation_reason_codes);
        sort_dedup_strings(&mut context_injection_reason_codes);

        Self {
            source_id,
            root_reason_codes,
            memory_index_rebuild_reason_codes,
            memory_index_operation_reason_codes,
            context_injection_reason_codes,
        }
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let source_id = hex_id(&self.source_id);
        self.root_reason_codes
            .iter()
            .map(|reason| format!("root_index:{reason}:{source_id}"))
            .chain(
                self.memory_index_rebuild_reason_codes
                    .iter()
                    .map(|reason| format!("index_rebuild:{reason}:{source_id}")),
            )
            .chain(
                self.memory_index_operation_reason_codes
                    .iter()
                    .map(|reason| format!("index_operation:{reason}:{source_id}")),
            )
            .chain(
                self.context_injection_reason_codes
                    .iter()
                    .map(|reason| format!("context:{reason}:{source_id}")),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "experience_index_projection source_id_hex={} root_reason_codes={} index_rebuild_reason_codes={} index_operation_reason_codes={} context_reason_codes={} detail_codes={}",
            hex_id(&self.source_id),
            join_codes(self.root_reason_codes.clone()),
            join_codes(self.memory_index_rebuild_reason_codes.clone()),
            join_codes(self.memory_index_operation_reason_codes.clone()),
            join_codes(self.context_injection_reason_codes.clone()),
            join_codes(self.detail_codes()),
        )
    }
}

pub trait MemoryIndexPlanner {
    fn plan(
        &self,
        documents: &[MemoryIndexDocument],
        rebuild: &IndexRebuildPlan,
    ) -> MemoryIndexPlan;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DefaultMemoryIndexPlanner;

impl MemoryAdapter for DefaultMemoryIndexPlanner {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_memory_index_planner",
            vec![MemoryAdapterCapability::MemoryIndex],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl MemoryIndexPlanner for DefaultMemoryIndexPlanner {
    fn plan(
        &self,
        documents: &[MemoryIndexDocument],
        rebuild: &IndexRebuildPlan,
    ) -> MemoryIndexPlan {
        let duplicate_ids = rebuild
            .deduplicate_groups
            .iter()
            .flat_map(|group| group.duplicate_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        let refresh_ids = rebuild
            .refresh_embedding_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let compact_ids = rebuild.compact_ids.iter().cloned().collect::<BTreeSet<_>>();
        let quarantine_ids = rebuild
            .quarantine_candidate_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let dirty_gist_ids = rebuild
            .dirty_gist_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let missing_clean_gist_ids = rebuild
            .missing_clean_gist_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let dirty_clean_gist_ids = rebuild
            .dirty_clean_gist_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let document_ids = documents
            .iter()
            .map(|document| document.id.clone())
            .collect::<BTreeSet<_>>();
        let mut operations_by_id = BTreeMap::<String, MemoryIndexOperation>::new();

        for document in documents {
            let (kind, reason) = if quarantine_ids.contains(&document.id) {
                (
                    MemoryIndexOperationKind::Quarantine,
                    "governance_quarantine_candidate",
                )
            } else if duplicate_ids.contains(&document.id) {
                (
                    MemoryIndexOperationKind::DeleteDuplicate,
                    "deduplicate_exact_fingerprint",
                )
            } else if compact_ids.contains(&document.id) {
                (
                    MemoryIndexOperationKind::Compact,
                    "compact_long_context_without_gist",
                )
            } else if refresh_ids.contains(&document.id) || dirty_gist_ids.contains(&document.id) {
                (
                    MemoryIndexOperationKind::RefreshEmbedding,
                    refresh_reason(
                        &document.id,
                        &refresh_ids,
                        &dirty_gist_ids,
                        &missing_clean_gist_ids,
                        &dirty_clean_gist_ids,
                    ),
                )
            } else {
                (MemoryIndexOperationKind::Upsert, "current_document")
            };
            operations_by_id.insert(
                document.id.clone(),
                MemoryIndexOperation {
                    source_id: document.id.clone(),
                    source: document.source,
                    kind,
                    reason: reason.to_owned(),
                },
            );
        }

        let mut skipped_ids = BTreeSet::new();
        skipped_ids.extend(refresh_ids.difference(&document_ids).cloned());
        skipped_ids.extend(compact_ids.difference(&document_ids).cloned());
        skipped_ids.extend(quarantine_ids.difference(&document_ids).cloned());
        skipped_ids.extend(dirty_gist_ids.difference(&document_ids).cloned());
        skipped_ids.extend(duplicate_ids.difference(&document_ids).cloned());

        let mut operations = operations_by_id.into_values().collect::<Vec<_>>();
        operations.sort_by(|left, right| {
            left.kind
                .priority()
                .cmp(&right.kind.priority())
                .then_with(|| left.source.cmp(&right.source))
                .then_with(|| left.source_id.cmp(&right.source_id))
        });

        let mut reasons = rebuild.reasons.clone();
        if rebuild.rebuild_required {
            reasons.push("full_rebuild_requested".to_owned());
        }
        reasons.sort();
        reasons.dedup();

        MemoryIndexPlan {
            operations,
            skipped_ids: skipped_ids.into_iter().collect(),
            reasons,
        }
    }
}

fn project_experience_index_content(envelope: &ExperienceEnvelope) -> ExperienceIndexContent {
    if let Some(gist) = envelope
        .clean_gist
        .as_deref()
        .map(str::trim)
        .filter(|gist| !gist.is_empty())
    {
        let (content, truncated) = truncate_chars(gist, CLEAN_GIST_INDEX_MAX_CHARS);
        return ExperienceIndexContent {
            content,
            basis: "clean_gist",
            truncated,
        };
    }

    let combined = format!("{}\n{}", envelope.prompt, envelope.lesson);
    if combined.chars().count() <= RAW_FALLBACK_INDEX_MAX_CHARS {
        return ExperienceIndexContent {
            content: combined,
            basis: "raw_fallback",
            truncated: false,
        };
    }

    let (prompt, prompt_truncated) =
        truncate_chars(envelope.prompt.trim(), RAW_FALLBACK_PROMPT_MAX_CHARS);
    let (lesson, lesson_truncated) =
        truncate_chars(envelope.lesson.trim(), RAW_FALLBACK_LESSON_MAX_CHARS);
    ExperienceIndexContent {
        content: format!("prompt_excerpt: {prompt}\nlesson_excerpt: {lesson}"),
        basis: "raw_fallback",
        truncated: prompt_truncated || lesson_truncated,
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> (String, bool) {
    let mut chars = value.chars();
    let truncated = value.chars().count() > max_chars;
    if !truncated {
        return (value.to_owned(), false);
    }
    let mut text = chars.by_ref().take(max_chars).collect::<String>();
    text.push_str("...<truncated>");
    (text, true)
}

fn is_index_clean_gist(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= CLEAN_GIST_INDEX_MAX_CHARS
        && !has_transcript_shape(trimmed)
        && !has_metadata_lesson_shape(trimmed)
        && trimmed
            .chars()
            .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
            .take(12)
            .count()
            >= 12
}

fn has_transcript_shape(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("conversation transcript:")
        || (value.contains("user:") && value.contains("assistant:"))
}

fn has_metadata_lesson_shape(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.starts_with("accepted_pattern ")
        || value.starts_with("rejected_pattern ")
        || ((value.contains("quality=") || value.contains("overlap="))
            && value.contains("max_severity="))
}

fn refresh_reason(
    id: &str,
    refresh_ids: &BTreeSet<String>,
    dirty_gist_ids: &BTreeSet<String>,
    missing_clean_gist_ids: &BTreeSet<String>,
    dirty_clean_gist_ids: &BTreeSet<String>,
) -> &'static str {
    if dirty_clean_gist_ids.contains(id) {
        "refresh_dirty_clean_gist"
    } else if missing_clean_gist_ids.contains(id) {
        "refresh_missing_clean_gist"
    } else if dirty_gist_ids.contains(id) {
        "refresh_noisy_or_dirty_gist"
    } else if refresh_ids.contains(id) {
        "refresh_noisy_or_rotting_index"
    } else {
        "refresh_embedding"
    }
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn detail_reason(reason: &str) -> &str {
    if reason.is_empty() {
        "unspecified"
    } else {
        reason
    }
}

fn split_root_index_reasons(reason: &str) -> Vec<String> {
    let mut reasons = reason
        .split('+')
        .map(stable_reason_code)
        .filter(|reason| !reason.is_empty() && reason != "clean")
        .collect::<Vec<_>>();
    sort_dedup_strings(&mut reasons);
    reasons
}

fn root_index_rebuild_reason_codes(reason: &str) -> Vec<String> {
    match reason {
        "duplicate_output" => vec!["deduplicate_exact_fingerprints".to_owned()],
        "unstructured_long_transcript" => vec![
            "compact_long_context_without_gist".to_owned(),
            "quarantine_high_noise_records".to_owned(),
            "refresh_noisy_or_rotting_index".to_owned(),
            "repair_missing_or_dirty_clean_gist".to_owned(),
        ],
        "overlong_single_document_without_clean_gist" => vec![
            "compact_long_context_without_gist".to_owned(),
            "refresh_noisy_or_rotting_index".to_owned(),
            "repair_missing_or_dirty_clean_gist".to_owned(),
        ],
        "legacy_metadata_lesson_missing_clean_gist" => vec![
            "refresh_noisy_or_rotting_index".to_owned(),
            "repair_missing_or_dirty_clean_gist".to_owned(),
        ],
        "legacy_metadata_lesson_clean_gist_fallback"
        | "transcript_lesson"
        | "transcript_prompt_without_clean_lesson" => {
            vec!["refresh_noisy_or_rotting_index".to_owned()]
        }
        _ => vec!["refresh_noisy_or_rotting_index".to_owned()],
    }
}

fn root_index_operation_reason_codes(reason: &str) -> Vec<String> {
    match reason {
        "duplicate_output" => vec!["deduplicate_exact_fingerprint".to_owned()],
        "unstructured_long_transcript" => vec![
            "compact_long_context_without_gist".to_owned(),
            "governance_quarantine_candidate".to_owned(),
            "refresh_missing_clean_gist".to_owned(),
        ],
        "overlong_single_document_without_clean_gist" => vec![
            "compact_long_context_without_gist".to_owned(),
            "refresh_missing_clean_gist".to_owned(),
        ],
        "legacy_metadata_lesson_missing_clean_gist" | "transcript_prompt_without_clean_lesson" => {
            vec!["refresh_missing_clean_gist".to_owned()]
        }
        "legacy_metadata_lesson_clean_gist_fallback" | "transcript_lesson" => {
            vec!["refresh_noisy_or_rotting_index".to_owned()]
        }
        _ => vec!["refresh_noisy_or_rotting_index".to_owned()],
    }
}

fn root_index_context_reason_codes(reason: &str) -> Vec<String> {
    match reason {
        "unstructured_long_transcript" => vec![
            "missing_clean_gist".to_owned(),
            "raw_fallback_index_content".to_owned(),
            "transcript_anchor_risk".to_owned(),
            "truncated_index_content".to_owned(),
        ],
        "overlong_single_document_without_clean_gist" => vec![
            "missing_clean_gist".to_owned(),
            "raw_fallback_index_content".to_owned(),
            "truncated_index_content".to_owned(),
        ],
        "legacy_metadata_lesson_missing_clean_gist" | "transcript_prompt_without_clean_lesson" => {
            vec![
                "missing_clean_gist".to_owned(),
                "raw_fallback_index_content".to_owned(),
            ]
        }
        "transcript_lesson" => vec!["transcript_anchor_risk".to_owned()],
        _ => Vec::new(),
    }
}

fn stable_reason_code(reason: &str) -> String {
    reason
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == ':' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

fn sort_dedup_strings(codes: &mut Vec<String>) {
    codes.sort();
    codes.dedup();
}

fn hex_id(id: &str) -> String {
    id.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ContextCandidate, ContextDecisionKind, ContextInjectionGate, DefaultContextInjectionGate,
        DefaultExperienceGovernance, ExperienceGovernance, MemoryAccessPurpose,
        MemoryRequestContext,
    };

    #[test]
    fn experience_envelope_projects_to_index_document() {
        let envelope = ExperienceEnvelope::new("7", "prompt text", "lesson text")
            .with_clean_gist("Clean gist with enough signal for recall.")
            .with_quality(1.8)
            .with_tags(vec!["runtime".to_owned(), "repair".to_owned()])
            .with_scope(MemoryScope::for_task("runtime"));

        let document = MemoryIndexDocument::from_experience(&envelope);
        assert_eq!(document.id, "7");
        assert_eq!(document.source, MemoryIndexSource::Experience);
        assert_eq!(
            document.content,
            "Clean gist with enough signal for recall."
        );
        assert_eq!(document.strength, 1.0);
        assert_eq!(
            document.metadata.get("tags").map(String::as_str),
            Some("repair,runtime")
        );
        assert_eq!(
            document.metadata.get("content_basis").map(String::as_str),
            Some("clean_gist")
        );
        assert_eq!(
            document
                .metadata
                .get("content_truncated")
                .map(String::as_str),
            Some("false")
        );
        assert_eq!(document.scope.task_id.as_deref(), Some("runtime"));
    }

    #[test]
    fn raw_experience_projection_is_bounded_for_index_quality() {
        let envelope = ExperienceEnvelope::new(
            "raw-long",
            format!("prompt {}", "p".repeat(2_000)),
            format!("lesson {}", "l".repeat(2_000)),
        );

        let document = MemoryIndexDocument::from_experience(&envelope);

        assert_eq!(
            document.metadata.get("content_basis").map(String::as_str),
            Some("raw_fallback")
        );
        assert_eq!(
            document
                .metadata
                .get("content_truncated")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(
            document.metadata.get("prompt_chars").map(String::as_str),
            Some("2007")
        );
        assert_eq!(
            document.metadata.get("lesson_chars").map(String::as_str),
            Some("2007")
        );
        assert!(document.content.starts_with("prompt_excerpt: prompt "));
        assert!(document.content.contains("\nlesson_excerpt: lesson "));
        assert!(document.content.contains("...<truncated>"));
        assert!(document.content.chars().count() <= 1_280);
    }

    #[test]
    fn clean_gist_projection_is_bounded_even_if_adapter_sends_long_text() {
        let envelope = ExperienceEnvelope::new("gist-long", "prompt", "lesson")
            .with_clean_gist(format!("stable gist {}", "g".repeat(900)));

        let document = MemoryIndexDocument::from_experience(&envelope);

        assert_eq!(
            document.metadata.get("content_basis").map(String::as_str),
            Some("clean_gist")
        );
        assert_eq!(
            document
                .metadata
                .get("content_truncated")
                .map(String::as_str),
            Some("true")
        );
        assert!(document.content.starts_with("stable gist "));
        assert!(document.content.ends_with("...<truncated>"));
        assert!(document.content.chars().count() <= 435);
    }

    #[test]
    fn dirty_clean_gist_projection_carries_context_rejection_risk() {
        let envelope = ExperienceEnvelope::new("dirty", "prompt", "lesson")
            .with_clean_gist("Conversation Transcript: User: stale Assistant: stale")
            .with_scope(MemoryScope::for_task("runtime"));
        let document = MemoryIndexDocument::from_experience(&envelope);

        assert_eq!(
            document.metadata.get("tags").map(String::as_str),
            Some("risk:dirty_clean_gist")
        );

        let candidate = ContextCandidate::from_index_document(&document);
        assert_eq!(candidate.risk_reasons, vec!["dirty_clean_gist".to_owned()]);

        let request = MemoryRequestContext::new(
            MemoryScope::for_task("runtime"),
            MemoryAccessPurpose::Recall,
        );
        let plan = DefaultContextInjectionGate::new().plan(&[candidate], &request);
        assert_eq!(plan.decisions[0].kind, ContextDecisionKind::RejectRisk);
        assert_eq!(plan.reason_codes(), vec!["dirty_clean_gist".to_owned()]);
    }

    #[test]
    fn missing_clean_gist_raw_fallback_is_rejected_before_context_injection() {
        let envelope = ExperienceEnvelope::new(
            "missing",
            format!(
                "Conversation Transcript:\nUser: bash command {}\nAssistant: ok",
                "x".repeat(1_300)
            ),
            "accepted_pattern quality=0.2 max_severity=critical",
        )
        .with_scope(MemoryScope::for_task("runtime"));
        let document = MemoryIndexDocument::from_experience(&envelope);

        assert_eq!(
            document.metadata.get("content_basis").map(String::as_str),
            Some("raw_fallback")
        );
        assert_eq!(
            document.metadata.get("tags").map(String::as_str),
            Some("risk:missing_clean_gist")
        );

        let candidate = ContextCandidate::from_index_document(&document);
        assert!(
            candidate
                .risk_reasons
                .contains(&"missing_clean_gist".to_owned())
        );
        assert!(
            candidate
                .risk_reasons
                .contains(&"raw_fallback_index_content".to_owned())
        );

        let request = MemoryRequestContext::new(
            MemoryScope::for_task("runtime"),
            MemoryAccessPurpose::Recall,
        );
        let plan = DefaultContextInjectionGate::new().plan(&[candidate], &request);
        assert_eq!(plan.decisions[0].kind, ContextDecisionKind::RejectRisk);
        assert_eq!(
            plan.reason_codes(),
            vec![
                "missing_clean_gist".to_owned(),
                "raw_fallback_index_content".to_owned(),
                "truncated_index_content".to_owned(),
            ]
        );
        assert_eq!(plan.injected_context(), Vec::<&str>::new());
    }

    #[test]
    fn raw_fallback_index_evidence_uses_stable_codes_without_payloads() {
        let prompt_secret = "INDEX_PROMPT_SECRET_DO_NOT_LOG";
        let lesson_secret = "INDEX_LESSON_SECRET_DO_NOT_LOG";
        let envelope = ExperienceEnvelope::new(
            "raw-secret",
            format!("Conversation Transcript:\nUser: {prompt_secret}\nAssistant: ok"),
            format!("accepted_pattern quality=0.1 max_severity=critical {lesson_secret}"),
        )
        .with_scope(MemoryScope::for_task("runtime"));
        let document = MemoryIndexDocument::from_experience(&envelope);

        assert_eq!(
            document.metadata.get("content_basis").map(String::as_str),
            Some("raw_fallback")
        );
        assert_eq!(
            document.metadata.get("tags").map(String::as_str),
            Some("risk:missing_clean_gist")
        );
        for forbidden in [prompt_secret, lesson_secret] {
            assert!(
                !document
                    .metadata
                    .values()
                    .any(|value| value.contains(forbidden)),
                "index metadata leaked raw fallback payload: {forbidden}"
            );
        }

        let rebuild = IndexRebuildPlan {
            rebuild_required: true,
            refresh_embedding_ids: vec!["raw-secret".to_owned()],
            missing_clean_gist_ids: vec!["raw-secret".to_owned()],
            dirty_gist_ids: vec!["raw-secret".to_owned()],
            reasons: vec!["repair_missing_or_dirty_clean_gist".to_owned()],
            ..IndexRebuildPlan::default()
        };
        let plan = DefaultMemoryIndexPlanner.plan(&[document.clone()], &rebuild);
        let summary_line = plan.summary_line();
        let detail_codes = plan.detail_codes();

        assert_eq!(
            plan.operations_by_kind(MemoryIndexOperationKind::RefreshEmbedding)[0].reason,
            "refresh_missing_clean_gist"
        );
        assert_eq!(
            detail_codes,
            vec!["refresh_embedding:refresh_missing_clean_gist:7261772d736563726574".to_owned()]
        );
        assert_eq!(
            plan.operation_detail_codes_for_kind(MemoryIndexOperationKind::RefreshEmbedding),
            vec!["refresh_embedding:refresh_missing_clean_gist:7261772d736563726574".to_owned()]
        );
        assert_eq!(
            plan.operation_detail_codes_for_reason("refresh_missing_clean_gist"),
            vec!["refresh_embedding:refresh_missing_clean_gist:7261772d736563726574".to_owned()]
        );
        assert_eq!(plan.skipped_detail_codes(), Vec::<String>::new());
        assert!(summary_line.contains("refresh_missing_clean_gist"));
        for forbidden in [prompt_secret, lesson_secret] {
            assert!(
                !summary_line.contains(forbidden),
                "index summary leaked raw fallback payload: {forbidden}"
            );
            assert!(
                !detail_codes.iter().any(|code| code.contains(forbidden)),
                "index detail codes leaked raw fallback payload: {forbidden}"
            );
        }

        let candidate = ContextCandidate::from_index_document(&document);
        let request = MemoryRequestContext::new(
            MemoryScope::for_task("runtime"),
            MemoryAccessPurpose::Recall,
        );
        let context = DefaultContextInjectionGate::new().plan(&[candidate], &request);
        assert_eq!(context.decisions[0].kind, ContextDecisionKind::RejectRisk);
        assert_eq!(context.injected_context(), Vec::<&str>::new());
        assert_eq!(
            context.detail_codes(),
            vec![
                "reject_risk:missing_clean_gist:7261772d736563726574".to_owned(),
                "reject_risk:raw_fallback_index_content:7261772d736563726574".to_owned(),
            ]
        );
        for forbidden in [prompt_secret, lesson_secret] {
            assert!(
                !context.summary_line().contains(forbidden),
                "context summary leaked raw fallback payload: {forbidden}"
            );
            assert!(
                !context
                    .detail_codes()
                    .iter()
                    .any(|code| code.contains(forbidden)),
                "context detail codes leaked raw fallback payload: {forbidden}"
            );
        }
    }

    #[test]
    fn root_experience_index_finding_projection_maps_to_index_and_context_codes() {
        let projection = ExperienceIndexFindingProjection::new(
            "polluted",
            "duplicate_output+unstructured_long_transcript",
        );

        assert_eq!(
            projection.root_reason_codes,
            vec![
                "duplicate_output".to_owned(),
                "unstructured_long_transcript".to_owned()
            ]
        );
        assert_eq!(
            projection.memory_index_rebuild_reason_codes,
            vec![
                "compact_long_context_without_gist".to_owned(),
                "deduplicate_exact_fingerprints".to_owned(),
                "quarantine_high_noise_records".to_owned(),
                "refresh_noisy_or_rotting_index".to_owned(),
                "repair_missing_or_dirty_clean_gist".to_owned(),
            ]
        );
        assert_eq!(
            projection.memory_index_operation_reason_codes,
            vec![
                "compact_long_context_without_gist".to_owned(),
                "deduplicate_exact_fingerprint".to_owned(),
                "governance_quarantine_candidate".to_owned(),
                "refresh_missing_clean_gist".to_owned(),
            ]
        );
        assert_eq!(
            projection.context_injection_reason_codes,
            vec![
                "missing_clean_gist".to_owned(),
                "raw_fallback_index_content".to_owned(),
                "transcript_anchor_risk".to_owned(),
                "truncated_index_content".to_owned(),
            ]
        );
        assert!(
            projection
                .summary_line()
                .contains("detail_codes=context:missing_clean_gist:706f6c6c75746564")
        );

        let document = MemoryIndexDocument::from_experience(
            &ExperienceEnvelope::new(
                "polluted",
                format!(
                    "Conversation Transcript:\nUser: cargo test {}\nAssistant: ok",
                    "x".repeat(1_300)
                ),
                "accepted_pattern quality=0.2 max_severity=critical",
            )
            .with_tags(vec!["risk:transcript_anchor_risk".to_owned()])
            .with_scope(MemoryScope::for_task("runtime")),
        );
        let rebuild = IndexRebuildPlan {
            rebuild_required: true,
            deduplicate_groups: vec![crate::DuplicateGroup {
                canonical_id: "canonical".to_owned(),
                duplicate_ids: vec!["polluted".to_owned()],
                fingerprint: "fingerprint".to_owned(),
            }],
            refresh_embedding_ids: vec!["polluted".to_owned()],
            compact_ids: vec!["polluted".to_owned()],
            quarantine_candidate_ids: vec!["polluted".to_owned()],
            missing_clean_gist_ids: vec!["polluted".to_owned()],
            dirty_gist_ids: vec!["polluted".to_owned()],
            reasons: projection.memory_index_rebuild_reason_codes.clone(),
            ..IndexRebuildPlan::default()
        };
        let index_plan = DefaultMemoryIndexPlanner.plan(&[document.clone()], &rebuild);
        let operation = &index_plan.operations[0];

        assert_eq!(operation.kind, MemoryIndexOperationKind::Quarantine);
        assert!(
            projection
                .memory_index_operation_reason_codes
                .contains(&operation.reason)
        );
        assert_eq!(
            index_plan.operation_detail_codes_for_reason(&operation.reason),
            vec!["quarantine:governance_quarantine_candidate:706f6c6c75746564".to_owned()]
        );

        let candidate = ContextCandidate::from_index_document(&document);
        let request = MemoryRequestContext::new(
            MemoryScope::for_task("runtime"),
            MemoryAccessPurpose::Recall,
        );
        let context_plan = DefaultContextInjectionGate::new().plan(&[candidate], &request);

        assert_eq!(
            context_plan.decisions[0].kind,
            ContextDecisionKind::RejectRisk
        );
        assert_eq!(
            context_plan.reason_codes(),
            projection.context_injection_reason_codes
        );
        assert_eq!(
            context_plan.detail_codes(),
            vec![
                "reject_risk:missing_clean_gist:706f6c6c75746564".to_owned(),
                "reject_risk:raw_fallback_index_content:706f6c6c75746564".to_owned(),
                "reject_risk:transcript_anchor_risk:706f6c6c75746564".to_owned(),
                "reject_risk:truncated_index_content:706f6c6c75746564".to_owned(),
            ]
        );
    }

    #[test]
    fn index_plan_prioritizes_quarantine_duplicate_compact_refresh() {
        let records = vec![
            ExperienceEnvelope::new("keep", "prompt", "lesson")
                .with_clean_gist("Clean stable lesson content for recall."),
            ExperienceEnvelope::new("dupe-a", "same prompt", "same lesson"),
            ExperienceEnvelope::new("dupe-b", "same prompt", "same lesson"),
            ExperienceEnvelope::new(
                "long",
                format!(
                    "Conversation Transcript:\nUser: bash command {}\nAssistant: ok",
                    "x".repeat(2_700)
                ),
                "accepted_pattern quality=0.1 max_severity=critical",
            ),
        ];
        let rebuild = DefaultExperienceGovernance::default().rebuild_plan(&records);
        let documents = records
            .iter()
            .map(MemoryIndexDocument::from_experience)
            .collect::<Vec<_>>();

        let plan = DefaultMemoryIndexPlanner.plan(&documents, &rebuild);
        assert!(plan.requires_rebuild());
        assert_eq!(
            plan.operations_by_kind(MemoryIndexOperationKind::Quarantine)[0].source_id,
            "long"
        );
        assert_eq!(
            plan.operations_by_kind(MemoryIndexOperationKind::DeleteDuplicate)[0].source_id,
            "dupe-b"
        );
        assert!(plan.operations.iter().any(|operation| operation.kind
            == MemoryIndexOperationKind::Upsert
            && operation.source_id == "keep"));
        assert_eq!(
            plan.summary_line(),
            "memory_index_plan rebuild=true operations=4 upsert=2 refresh=0 compact=0 quarantine=1 delete_duplicate=1 skipped=0 reasons=6 reason_codes=compact_long_context_without_gist|deduplicate_exact_fingerprints|full_rebuild_requested|quarantine_high_noise_records|refresh_noisy_or_rotting_index|repair_missing_or_dirty_clean_gist detail_codes=delete_duplicate:deduplicate_exact_fingerprint:647570652d62|quarantine:governance_quarantine_candidate:6c6f6e67"
        );
        assert_eq!(
            plan.reason_codes(),
            vec![
                "compact_long_context_without_gist".to_owned(),
                "deduplicate_exact_fingerprints".to_owned(),
                "full_rebuild_requested".to_owned(),
                "quarantine_high_noise_records".to_owned(),
                "refresh_noisy_or_rotting_index".to_owned(),
                "repair_missing_or_dirty_clean_gist".to_owned(),
            ]
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "delete_duplicate:deduplicate_exact_fingerprint:647570652d62".to_owned(),
                "quarantine:governance_quarantine_candidate:6c6f6e67".to_owned(),
            ]
        );
        assert_eq!(
            plan.operation_detail_codes_for_kind(MemoryIndexOperationKind::DeleteDuplicate),
            vec!["delete_duplicate:deduplicate_exact_fingerprint:647570652d62".to_owned()]
        );
        assert_eq!(
            plan.operation_detail_codes_for_reason("governance_quarantine_candidate"),
            vec!["quarantine:governance_quarantine_candidate:6c6f6e67".to_owned()]
        );
    }

    #[test]
    fn index_plan_reports_missing_rebuild_targets_without_writing() {
        let rebuild = IndexRebuildPlan {
            rebuild_required: true,
            refresh_embedding_ids: vec!["missing-refresh".to_owned()],
            compact_ids: vec!["missing-compact".to_owned()],
            quarantine_candidate_ids: vec!["missing-quarantine".to_owned()],
            dirty_gist_ids: vec!["missing-gist".to_owned()],
            reasons: vec!["manual_test".to_owned()],
            ..IndexRebuildPlan::default()
        };
        let documents = vec![MemoryIndexDocument::new(
            "present",
            MemoryIndexSource::Experience,
            "present content",
        )];

        let plan = DefaultMemoryIndexPlanner.plan(&documents, &rebuild);
        assert_eq!(
            plan.skipped_ids,
            vec![
                "missing-compact".to_owned(),
                "missing-gist".to_owned(),
                "missing-quarantine".to_owned(),
                "missing-refresh".to_owned(),
            ]
        );
        assert_eq!(
            plan.operations_by_kind(MemoryIndexOperationKind::Upsert)[0].source_id,
            "present"
        );
        assert_eq!(
            plan.summary_line(),
            "memory_index_plan rebuild=true operations=1 upsert=1 refresh=0 compact=0 quarantine=0 delete_duplicate=0 skipped=4 reasons=2 reason_codes=full_rebuild_requested|manual_test detail_codes=skipped:6d697373696e672d636f6d70616374|skipped:6d697373696e672d67697374|skipped:6d697373696e672d71756172616e74696e65|skipped:6d697373696e672d72656672657368"
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "skipped:6d697373696e672d636f6d70616374".to_owned(),
                "skipped:6d697373696e672d67697374".to_owned(),
                "skipped:6d697373696e672d71756172616e74696e65".to_owned(),
                "skipped:6d697373696e672d72656672657368".to_owned(),
            ]
        );
        assert_eq!(
            plan.skipped_detail_codes(),
            vec![
                "skipped:6d697373696e672d636f6d70616374".to_owned(),
                "skipped:6d697373696e672d67697374".to_owned(),
                "skipped:6d697373696e672d71756172616e74696e65".to_owned(),
                "skipped:6d697373696e672d72656672657368".to_owned(),
            ]
        );
        assert_eq!(
            plan.operation_detail_codes_for_reason("refresh_missing_clean_gist"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn index_plan_distinguishes_clean_gist_refresh_reasons() {
        let rebuild = IndexRebuildPlan {
            rebuild_required: true,
            refresh_embedding_ids: vec!["noisy".to_owned()],
            missing_clean_gist_ids: vec!["missing".to_owned()],
            dirty_clean_gist_ids: vec!["dirty".to_owned()],
            dirty_gist_ids: vec!["dirty".to_owned(), "missing".to_owned()],
            reasons: vec!["repair_missing_or_dirty_clean_gist".to_owned()],
            ..IndexRebuildPlan::default()
        };
        let documents = vec![
            MemoryIndexDocument::new("missing", MemoryIndexSource::Experience, "missing gist"),
            MemoryIndexDocument::new("dirty", MemoryIndexSource::Experience, "dirty gist"),
            MemoryIndexDocument::new("noisy", MemoryIndexSource::Experience, "noisy record"),
        ];

        let plan = DefaultMemoryIndexPlanner.plan(&documents, &rebuild);
        let refresh = plan.operations_by_kind(MemoryIndexOperationKind::RefreshEmbedding);

        assert_eq!(refresh.len(), 3);
        assert!(refresh.iter().any(|operation| {
            operation.source_id == "missing" && operation.reason == "refresh_missing_clean_gist"
        }));
        assert!(refresh.iter().any(|operation| {
            operation.source_id == "dirty" && operation.reason == "refresh_dirty_clean_gist"
        }));
        assert!(refresh.iter().any(|operation| {
            operation.source_id == "noisy" && operation.reason == "refresh_noisy_or_rotting_index"
        }));
        assert_eq!(
            plan.detail_codes(),
            vec![
                "refresh_embedding:refresh_dirty_clean_gist:6469727479".to_owned(),
                "refresh_embedding:refresh_missing_clean_gist:6d697373696e67".to_owned(),
                "refresh_embedding:refresh_noisy_or_rotting_index:6e6f697379".to_owned(),
            ]
        );
        assert_eq!(
            plan.operation_detail_codes_for_kind(MemoryIndexOperationKind::RefreshEmbedding),
            vec![
                "refresh_embedding:refresh_dirty_clean_gist:6469727479".to_owned(),
                "refresh_embedding:refresh_missing_clean_gist:6d697373696e67".to_owned(),
                "refresh_embedding:refresh_noisy_or_rotting_index:6e6f697379".to_owned(),
            ]
        );
        assert_eq!(
            plan.operation_detail_codes_for_reason("refresh_dirty_clean_gist"),
            vec!["refresh_embedding:refresh_dirty_clean_gist:6469727479".to_owned()]
        );
        assert_eq!(
            plan.operation_detail_codes_for_reason("refresh_missing_clean_gist"),
            vec!["refresh_embedding:refresh_missing_clean_gist:6d697373696e67".to_owned()]
        );
    }

    #[test]
    fn index_planner_is_read_only_adapter() {
        let descriptor = DefaultMemoryIndexPlanner.descriptor();
        assert_eq!(descriptor.name, "default_memory_index_planner");
        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::MemoryIndex)
        );
        assert!(DefaultMemoryIndexPlanner.health().unwrap().ready);
    }
}
