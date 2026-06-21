use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ExperienceEnvelope, IndexRebuildPlan, MemoryAdapter, MemoryAdapterCapability,
    MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryResult, MemoryScope, Metadata, clamp01,
    stable_hash,
};

const CLEAN_GIST_INDEX_MAX_CHARS: usize = 420;
const RAW_FALLBACK_INDEX_MAX_CHARS: usize = 1_200;
const RAW_FALLBACK_PROMPT_MAX_CHARS: usize = 420;
const RAW_FALLBACK_LESSON_MAX_CHARS: usize = 780;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryIndexSource {
    Experience,
    GeneSegment,
    LongTerm,
    Skill,
    RuntimeKv,
}

impl MemoryIndexSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Experience => "experience",
            Self::GeneSegment => "gene_segment",
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

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySemanticQuery {
    pub text: String,
    pub embedding: Vec<f32>,
    pub scope: Option<MemoryScope>,
    pub limit: usize,
    pub token_budget: usize,
    pub min_score: f32,
    pub allow_cross_task: bool,
    pub include_quarantined: bool,
    pub source_filters: BTreeSet<MemoryIndexSource>,
    pub metadata_filters: Metadata,
}

impl MemorySemanticQuery {
    pub fn new(text: impl Into<String>, limit: usize) -> Self {
        Self {
            text: text.into(),
            embedding: Vec::new(),
            scope: None,
            limit: limit.max(1),
            token_budget: 1_024,
            min_score: 0.05,
            allow_cross_task: false,
            include_quarantined: false,
            source_filters: BTreeSet::new(),
            metadata_filters: Metadata::new(),
        }
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = embedding;
        self
    }

    pub fn with_scope(mut self, scope: MemoryScope) -> Self {
        self.scope = Some(scope);
        self
    }

    pub fn with_token_budget(mut self, token_budget: usize) -> Self {
        self.token_budget = token_budget.max(1);
        self
    }

    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = clamp01(min_score);
        self
    }

    pub fn allow_cross_task(mut self, allow_cross_task: bool) -> Self {
        self.allow_cross_task = allow_cross_task;
        self
    }

    pub fn include_quarantined(mut self, include_quarantined: bool) -> Self {
        self.include_quarantined = include_quarantined;
        self
    }

    pub fn with_source(mut self, source: MemoryIndexSource) -> Self {
        self.source_filters.insert(source);
        self
    }

    pub fn with_metadata_filter(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.metadata_filters.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySemanticMatch {
    pub id: String,
    pub source: MemoryIndexSource,
    pub content: String,
    pub score: f32,
    pub strength: f32,
    pub estimated_tokens: usize,
    pub metadata: Metadata,
    pub scope: MemoryScope,
}

impl MemorySemanticMatch {
    pub fn detail_code(&self) -> String {
        format!(
            "match:{}:{:.3}:{}",
            self.source.as_str(),
            self.score,
            hex_id(&self.id)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySemanticSkip {
    pub id: String,
    pub source: MemoryIndexSource,
    pub reason: String,
}

impl MemorySemanticSkip {
    pub fn detail_code(&self) -> String {
        format!(
            "skip:{}:{}:{}",
            self.source.as_str(),
            detail_reason(&self.reason),
            hex_id(&self.id)
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemorySemanticRetrievalPlan {
    pub matches: Vec<MemorySemanticMatch>,
    pub skipped: Vec<MemorySemanticSkip>,
    pub used_tokens: usize,
    pub source_digest: u64,
}

impl MemorySemanticRetrievalPlan {
    pub fn matched_ids(&self) -> Vec<String> {
        self.matches.iter().map(|item| item.id.clone()).collect()
    }

    pub fn skipped_ids_for_reason(&self, reason: &str) -> Vec<String> {
        self.skipped
            .iter()
            .filter(|item| item.reason == reason)
            .map(|item| item.id.clone())
            .collect()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.skipped
            .iter()
            .map(|item| item.reason.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn match_detail_codes(&self) -> Vec<String> {
        self.matches
            .iter()
            .map(MemorySemanticMatch::detail_code)
            .collect()
    }

    pub fn skip_detail_codes(&self) -> Vec<String> {
        self.skipped
            .iter()
            .map(MemorySemanticSkip::detail_code)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.match_detail_codes()
            .into_iter()
            .chain(self.skip_detail_codes())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_semantic_retrieval matches={} skipped={} used_tokens={} source_digest={:016x} reason_codes={} detail_codes={}",
            self.matches.len(),
            self.skipped.len(),
            self.used_tokens,
            self.source_digest,
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

pub trait MemorySemanticRetriever {
    fn retrieve(
        &self,
        documents: &[MemoryIndexDocument],
        query: &MemorySemanticQuery,
    ) -> MemorySemanticRetrievalPlan;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DefaultMemorySemanticRetriever;

impl MemoryAdapter for DefaultMemorySemanticRetriever {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_memory_semantic_retriever",
            vec![
                MemoryAdapterCapability::MemoryIndex,
                MemoryAdapterCapability::SemanticRetrieval,
            ],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl MemorySemanticRetriever for DefaultMemorySemanticRetriever {
    fn retrieve(
        &self,
        documents: &[MemoryIndexDocument],
        query: &MemorySemanticQuery,
    ) -> MemorySemanticRetrievalPlan {
        let mut skipped = Vec::new();
        let query_tokens = normalized_tokens(&query.text);
        let mut scored = Vec::new();

        for document in documents {
            if !query.source_filters.is_empty() && !query.source_filters.contains(&document.source)
            {
                skipped.push(skip(document, "source_filter"));
                continue;
            }
            if !metadata_matches(&query.metadata_filters, &document.metadata) {
                skipped.push(skip(document, "metadata_filter"));
                continue;
            }
            if let Some(scope_reason) = scope_skip_reason(query, &document.scope) {
                skipped.push(skip(document, scope_reason));
                continue;
            }
            if let Some(risk_reason) = retrieval_risk_reason(document, query.include_quarantined) {
                skipped.push(skip(document, risk_reason));
                continue;
            }

            let vector_score = cosine_similarity(&query.embedding, &document.embedding);
            let text_score = lexical_score(&query_tokens, document);
            let confidence = metadata_float(&document.metadata, "confidence").unwrap_or(0.5);
            let freshness = metadata_float(&document.metadata, "freshness").unwrap_or(0.5);
            let quality = retrieval_quality_multiplier(&document.metadata);
            let score = ((vector_score.max(text_score) * 0.64)
                + (document.strength * 0.16)
                + (confidence * 0.12)
                + (freshness * 0.08))
                .clamp(0.0, 1.0)
                * quality;

            if score < query.min_score {
                skipped.push(skip(document, "below_min_score"));
                continue;
            }

            scored.push((
                MemorySemanticMatch {
                    id: document.id.clone(),
                    source: document.source,
                    content: document.content.clone(),
                    score: score.clamp(0.0, 1.0),
                    strength: document.strength,
                    estimated_tokens: estimate_tokens(&document.content),
                    metadata: document.metadata.clone(),
                    scope: document.scope.clone(),
                },
                normalized_content_key(&document.content),
            ));
        }

        scored.sort_by(|left, right| {
            right
                .0
                .score
                .partial_cmp(&left.0.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.0.source.cmp(&right.0.source))
                .then_with(|| left.0.id.cmp(&right.0.id))
        });

        let mut matches = Vec::new();
        let mut used_tokens = 0usize;
        let mut seen_ids = BTreeSet::new();
        let mut seen_content = BTreeSet::new();
        for (item, content_key) in scored {
            let source_id_key = format!("{}:{}", item.source.as_str(), item.id);
            if !seen_ids.insert(source_id_key) || !seen_content.insert(content_key) {
                skipped.push(MemorySemanticSkip {
                    id: item.id,
                    source: item.source,
                    reason: "duplicate".to_owned(),
                });
                continue;
            }
            if matches.len() >= query.limit {
                skipped.push(MemorySemanticSkip {
                    id: item.id,
                    source: item.source,
                    reason: "result_limit".to_owned(),
                });
                continue;
            }
            if used_tokens.saturating_add(item.estimated_tokens) > query.token_budget {
                skipped.push(MemorySemanticSkip {
                    id: item.id,
                    source: item.source,
                    reason: "token_budget".to_owned(),
                });
                continue;
            }
            used_tokens = used_tokens.saturating_add(item.estimated_tokens);
            matches.push(item);
        }

        MemorySemanticRetrievalPlan {
            matches,
            skipped,
            used_tokens,
            source_digest: memory_index_digest(documents),
        }
    }
}

pub fn memory_index_digest(documents: &[MemoryIndexDocument]) -> u64 {
    let mut lines = documents
        .iter()
        .map(|document| {
            let metadata = document
                .metadata
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{}\t{}\t{}\t{}\t{:.6}\t{}",
                document.source.as_str(),
                document.id,
                stable_hash(document.content.as_bytes()),
                document.embedding.len(),
                document.strength,
                metadata
            )
        })
        .collect::<Vec<_>>();
    lines.sort();
    stable_hash(lines.join("\n").as_bytes())
}

fn skip(document: &MemoryIndexDocument, reason: impl Into<String>) -> MemorySemanticSkip {
    MemorySemanticSkip {
        id: document.id.clone(),
        source: document.source,
        reason: reason.into(),
    }
}

fn metadata_matches(filters: &Metadata, metadata: &Metadata) -> bool {
    filters
        .iter()
        .all(|(key, expected)| metadata.get(key) == Some(expected))
}

fn scope_skip_reason(
    query: &MemorySemanticQuery,
    document_scope: &MemoryScope,
) -> Option<&'static str> {
    let scope = query.scope.as_ref()?;
    if let (Some(left), Some(right)) = (&scope.agent_id, &document_scope.agent_id) {
        if left != right {
            return Some("cross_agent_scope");
        }
    }
    if let (Some(left), Some(right)) = (&scope.session_id, &document_scope.session_id) {
        if left != right {
            return Some("cross_session_scope");
        }
    }
    if !query.allow_cross_task {
        if let (Some(left), Some(right)) = (&scope.task_id, &document_scope.task_id) {
            if left != right {
                return Some("cross_task_scope");
            }
        }
    }
    None
}

fn retrieval_risk_reason(
    document: &MemoryIndexDocument,
    include_quarantined: bool,
) -> Option<&'static str> {
    let tags = metadata_tags(&document.metadata);
    if metadata_bool(&document.metadata, "privacy_blocked")
        || document
            .metadata
            .get("privacy")
            .is_some_and(|value| value == "blocked")
        || tags.contains("risk:privacy_blocked")
    {
        return Some("privacy_blocked");
    }

    if !include_quarantined {
        if metadata_bool(&document.metadata, "quarantined")
            || tags.contains("risk:quarantined")
            || tags.contains("risk:quarantine_high_noise_records")
        {
            return Some("quarantined");
        }
        if document.source == MemoryIndexSource::GeneSegment {
            match document.metadata.get("gene_status").map(String::as_str) {
                Some("corrupt") => return Some("gene_segment_corrupt"),
                Some("malignant") => return Some("gene_segment_malignant"),
                Some("quarantined") => return Some("gene_segment_quarantined"),
                _ => {}
            }
        }
    }

    None
}

fn metadata_tags(metadata: &Metadata) -> BTreeSet<&str> {
    metadata
        .get("tags")
        .map(|tags| {
            tags.split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default()
}

fn metadata_bool(metadata: &Metadata, key: &str) -> bool {
    metadata
        .get(key)
        .is_some_and(|value| matches!(value.as_str(), "true" | "1" | "yes"))
}

fn metadata_float(metadata: &Metadata, key: &str) -> Option<f32> {
    metadata.get(key)?.parse::<f32>().ok().map(clamp01)
}

fn retrieval_quality_multiplier(metadata: &Metadata) -> f32 {
    let mut multiplier = 1.0_f32;
    if metadata
        .get("content_basis")
        .is_some_and(|value| value == "raw_fallback")
    {
        multiplier *= 0.86;
    }
    if metadata
        .get("content_truncated")
        .is_some_and(|value| value == "true")
    {
        multiplier *= 0.78;
    }
    multiplier
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let dot = left.iter().zip(right).map(|(l, r)| l * r).sum::<f32>();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }
    (dot / (left_norm * right_norm)).clamp(0.0, 1.0)
}

fn lexical_score(query_tokens: &BTreeSet<String>, document: &MemoryIndexDocument) -> f32 {
    if query_tokens.is_empty() {
        return 0.0;
    }
    let mut content = document.content.clone();
    if let Some(tags) = document.metadata.get("tags") {
        content.push(' ');
        content.push_str(tags);
    }
    let document_tokens = normalized_tokens(&content);
    if document_tokens.is_empty() {
        return 0.0;
    }
    let shared = query_tokens.intersection(&document_tokens).count() as f32;
    (shared / query_tokens.len().min(document_tokens.len()) as f32).clamp(0.0, 1.0)
}

fn normalized_tokens(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn normalized_content_key(value: &str) -> u64 {
    let normalized = normalized_tokens(value)
        .into_iter()
        .collect::<Vec<_>>()
        .join(" ");
    stable_hash(normalized.as_bytes())
}

fn estimate_tokens(value: &str) -> usize {
    (value.chars().count() / 4).max(1)
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
    fn semantic_retrieval_ranks_bounds_and_isolates_scope() {
        let mut runtime_metadata = Metadata::new();
        runtime_metadata.insert("tags".to_owned(), "rust,runtime".to_owned());
        runtime_metadata.insert("confidence".to_owned(), "0.9".to_owned());
        runtime_metadata.insert("freshness".to_owned(), "0.8".to_owned());
        let runtime_scope = MemoryScope::for_task("runtime").with_agent("tenant-a");
        let documents = vec![
            MemoryIndexDocument::new(
                "runtime-rust",
                MemoryIndexSource::LongTerm,
                "Rust borrow checker lesson for runtime adapter repair",
            )
            .with_embedding(vec![1.0, 0.0])
            .with_scope(runtime_scope.clone())
            .with_metadata(runtime_metadata)
            .with_strength(0.8),
            MemoryIndexDocument::new(
                "runtime-large",
                MemoryIndexSource::LongTerm,
                format!("runtime adapter {}", "budget ".repeat(260)),
            )
            .with_scope(runtime_scope.clone())
            .with_strength(0.3),
            MemoryIndexDocument::new(
                "other-task",
                MemoryIndexSource::LongTerm,
                "Rust borrow checker lesson for a different task",
            )
            .with_scope(MemoryScope::for_task("gitlab").with_agent("tenant-a"))
            .with_strength(0.9),
            MemoryIndexDocument::new(
                "other-agent",
                MemoryIndexSource::LongTerm,
                "Rust borrow checker lesson for another tenant",
            )
            .with_scope(MemoryScope::for_task("runtime").with_agent("tenant-b"))
            .with_strength(0.9),
        ];

        let plan = DefaultMemorySemanticRetriever.retrieve(
            &documents,
            &MemorySemanticQuery::new("rust borrow checker runtime adapter", 3)
                .with_embedding(vec![0.9, 0.1])
                .with_scope(runtime_scope)
                .with_token_budget(32),
        );

        assert_eq!(plan.matched_ids(), vec!["runtime-rust".to_owned()]);
        assert!(plan.used_tokens <= 32);
        assert_eq!(
            plan.skipped_ids_for_reason("cross_task_scope"),
            vec!["other-task".to_owned()]
        );
        assert_eq!(
            plan.skipped_ids_for_reason("cross_agent_scope"),
            vec!["other-agent".to_owned()]
        );
        assert_eq!(
            plan.skipped_ids_for_reason("token_budget"),
            vec!["runtime-large".to_owned()]
        );
        assert_eq!(plan.source_digest, memory_index_digest(&documents));
        assert!(
            plan.summary_line()
                .contains("reason_codes=cross_agent_scope|cross_task_scope|token_budget")
        );
    }

    #[test]
    fn semantic_retrieval_skips_privacy_and_quarantined_gene_segments() {
        let mut corrupt_metadata = Metadata::new();
        corrupt_metadata.insert("gene_status".to_owned(), "corrupt".to_owned());
        let mut malignant_metadata = Metadata::new();
        malignant_metadata.insert("gene_status".to_owned(), "malignant".to_owned());
        let mut privacy_metadata = Metadata::new();
        privacy_metadata.insert("privacy".to_owned(), "blocked".to_owned());

        let documents = vec![
            MemoryIndexDocument::new(
                "active-gene",
                MemoryIndexSource::GeneSegment,
                "splice repair anchor for safe memory recall",
            )
            .with_strength(0.8),
            MemoryIndexDocument::new(
                "corrupt-gene",
                MemoryIndexSource::GeneSegment,
                "splice repair anchor with corrupt payload",
            )
            .with_metadata(corrupt_metadata)
            .with_strength(0.9),
            MemoryIndexDocument::new(
                "malignant-gene",
                MemoryIndexSource::GeneSegment,
                "splice repair anchor with malignant payload",
            )
            .with_metadata(malignant_metadata)
            .with_strength(0.9),
            MemoryIndexDocument::new(
                "private-episode",
                MemoryIndexSource::Experience,
                "splice repair anchor blocked by privacy",
            )
            .with_metadata(privacy_metadata)
            .with_strength(0.9),
        ];

        let guarded = DefaultMemorySemanticRetriever.retrieve(
            &documents,
            &MemorySemanticQuery::new("splice repair anchor", 5),
        );

        assert_eq!(guarded.matched_ids(), vec!["active-gene".to_owned()]);
        assert_eq!(
            guarded.skipped_ids_for_reason("gene_segment_corrupt"),
            vec!["corrupt-gene".to_owned()]
        );
        assert_eq!(
            guarded.skipped_ids_for_reason("gene_segment_malignant"),
            vec!["malignant-gene".to_owned()]
        );
        assert_eq!(
            guarded.skipped_ids_for_reason("privacy_blocked"),
            vec!["private-episode".to_owned()]
        );

        let repair = DefaultMemorySemanticRetriever.retrieve(
            &documents,
            &MemorySemanticQuery::new("splice repair anchor", 5).include_quarantined(true),
        );
        assert!(repair.matched_ids().contains(&"corrupt-gene".to_owned()));
        assert!(repair.matched_ids().contains(&"malignant-gene".to_owned()));
        assert_eq!(
            repair.skipped_ids_for_reason("privacy_blocked"),
            vec!["private-episode".to_owned()]
        );
    }

    #[test]
    fn semantic_retrieval_suppresses_duplicates_and_redacts_evidence() {
        let secret = "SEMANTIC_SECRET_DO_NOT_LOG";
        let documents = vec![
            MemoryIndexDocument::new(
                "clean-a",
                MemoryIndexSource::Experience,
                format!("rust adapter repair {secret}"),
            )
            .with_strength(0.9),
            MemoryIndexDocument::new(
                "clean-b",
                MemoryIndexSource::Experience,
                format!("rust adapter repair {secret}"),
            )
            .with_strength(0.8),
        ];

        let plan = DefaultMemorySemanticRetriever.retrieve(
            &documents,
            &MemorySemanticQuery::new("rust adapter repair", 5),
        );

        assert_eq!(plan.matched_ids(), vec!["clean-a".to_owned()]);
        assert_eq!(
            plan.skipped_ids_for_reason("duplicate"),
            vec!["clean-b".to_owned()]
        );
        assert_eq!(plan.source_digest, memory_index_digest(&documents));
        assert!(!plan.summary_line().contains(secret));
        assert!(!plan.detail_codes().iter().any(|code| code.contains(secret)));
    }

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
