use std::collections::{BTreeMap, BTreeSet};

use crate::{MemoryError, MemoryResult, MemoryScope, Metadata, clamp01};

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryDocumentInput {
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: Metadata,
    pub scope: MemoryScope,
    pub strength: f32,
}

impl MemoryDocumentInput {
    pub fn new(content: impl Into<String>, embedding: Vec<f32>) -> Self {
        Self {
            content: content.into(),
            embedding,
            metadata: Metadata::new(),
            scope: MemoryScope::default(),
            strength: 0.5,
        }
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

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryDocument {
    pub id: u64,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: Metadata,
    pub scope: MemoryScope,
    pub strength: f32,
    pub hits: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LongTermQuery {
    pub embedding: Vec<f32>,
    pub text_hint: String,
    pub limit: usize,
    pub scope: Option<MemoryScope>,
    pub metadata_filters: Metadata,
}

impl LongTermQuery {
    pub fn by_embedding(embedding: Vec<f32>, limit: usize) -> Self {
        Self {
            embedding,
            text_hint: String::new(),
            limit,
            scope: None,
            metadata_filters: Metadata::new(),
        }
    }

    pub fn by_text(text_hint: impl Into<String>, limit: usize) -> Self {
        Self {
            embedding: Vec::new(),
            text_hint: text_hint.into(),
            limit,
            scope: None,
            metadata_filters: Metadata::new(),
        }
    }

    pub fn with_scope(mut self, scope: MemoryScope) -> Self {
        self.scope = Some(scope);
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
pub struct LongTermMatch {
    pub id: u64,
    pub content: String,
    pub score: f32,
    pub strength: f32,
    pub metadata: Metadata,
    pub scope: MemoryScope,
}

pub trait LongTermMemory {
    fn remember(&mut self, input: MemoryDocumentInput) -> MemoryResult<u64>;
    fn get(&self, id: u64) -> MemoryResult<Option<MemoryDocument>>;
    fn search(&self, query: LongTermQuery) -> MemoryResult<Vec<LongTermMatch>>;
    fn reinforce(&mut self, id: u64, amount: f32) -> MemoryResult<bool>;
    fn penalize(&mut self, id: u64, amount: f32) -> MemoryResult<bool>;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone)]
pub struct InMemoryLongTermMemory {
    records: BTreeMap<u64, MemoryDocument>,
    next_id: u64,
}

impl Default for InMemoryLongTermMemory {
    fn default() -> Self {
        Self {
            records: BTreeMap::new(),
            next_id: 1,
        }
    }
}

impl InMemoryLongTermMemory {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LongTermMemory for InMemoryLongTermMemory {
    fn remember(&mut self, input: MemoryDocumentInput) -> MemoryResult<u64> {
        if input.content.trim().is_empty() {
            return Err(MemoryError::InvalidInput(
                "long-term memory content cannot be empty".to_owned(),
            ));
        }
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.records.insert(
            id,
            MemoryDocument {
                id,
                content: input.content,
                embedding: input.embedding,
                metadata: input.metadata,
                scope: input.scope,
                strength: clamp01(input.strength),
                hits: 0,
            },
        );
        Ok(id)
    }

    fn get(&self, id: u64) -> MemoryResult<Option<MemoryDocument>> {
        Ok(self.records.get(&id).cloned())
    }

    fn search(&self, query: LongTermQuery) -> MemoryResult<Vec<LongTermMatch>> {
        let limit = query.limit.max(1);
        let mut matches = self
            .records
            .values()
            .filter(|record| scope_matches(query.scope.as_ref(), &record.scope))
            .filter(|record| metadata_matches(&query.metadata_filters, &record.metadata))
            .map(|record| {
                let vector_score = cosine_similarity(&query.embedding, &record.embedding);
                let text_score = lexical_overlap(&query.text_hint, &record.content);
                let base_score =
                    (vector_score.max(text_score) * 0.82 + record.strength * 0.18).clamp(0.0, 1.0);
                let score =
                    (base_score * index_quality_multiplier(&record.metadata)).clamp(0.0, 1.0);
                LongTermMatch {
                    id: record.id,
                    content: record.content.clone(),
                    score,
                    strength: record.strength,
                    metadata: record.metadata.clone(),
                    scope: record.scope.clone(),
                }
            })
            .filter(|item| item.score > 0.0)
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.id.cmp(&right.id))
        });
        matches.truncate(limit);
        Ok(matches)
    }

    fn reinforce(&mut self, id: u64, amount: f32) -> MemoryResult<bool> {
        let Some(record) = self.records.get_mut(&id) else {
            return Ok(false);
        };
        record.strength = clamp01(record.strength + amount.max(0.0));
        record.hits = record.hits.saturating_add(1);
        Ok(true)
    }

    fn penalize(&mut self, id: u64, amount: f32) -> MemoryResult<bool> {
        let Some(record) = self.records.get_mut(&id) else {
            return Ok(false);
        };
        record.strength = clamp01(record.strength - amount.max(0.0));
        record.hits = record.hits.saturating_add(1);
        Ok(true)
    }

    fn len(&self) -> usize {
        self.records.len()
    }
}

fn scope_matches(query_scope: Option<&MemoryScope>, record_scope: &MemoryScope) -> bool {
    let Some(query_scope) = query_scope else {
        return true;
    };
    query_scope.same_task_as(record_scope).unwrap_or(true)
}

fn metadata_matches(filters: &Metadata, metadata: &Metadata) -> bool {
    filters
        .iter()
        .all(|(key, expected)| metadata.get(key) == Some(expected))
}

fn index_quality_multiplier(metadata: &Metadata) -> f32 {
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

fn lexical_overlap(left: &str, right: &str) -> f32 {
    let left = normalized_tokens(left);
    let right = normalized_tokens(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let shared = left.intersection(&right).count() as f32;
    (shared / left.len().min(right.len()) as f32).clamp(0.0, 1.0)
}

fn normalized_tokens(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_term_memory_retrieves_by_vector_and_text() {
        let mut memory = InMemoryLongTermMemory::new();
        let rust_id = memory
            .remember(
                MemoryDocumentInput::new(
                    "Rust ownership borrow checker lesson",
                    vec![1.0, 0.0, 0.0],
                )
                .with_strength(0.7),
            )
            .unwrap();
        memory
            .remember(MemoryDocumentInput::new(
                "Gemma runtime health readiness",
                vec![0.0, 1.0, 0.0],
            ))
            .unwrap();

        let vector_matches = memory
            .search(LongTermQuery::by_embedding(vec![0.9, 0.1, 0.0], 1))
            .unwrap();
        assert_eq!(vector_matches[0].id, rust_id);

        let text_matches = memory
            .search(LongTermQuery::by_text("borrow checker", 1))
            .unwrap();
        assert_eq!(text_matches[0].id, rust_id);

        assert!(memory.reinforce(rust_id, 0.2).unwrap());
        assert!(memory.get(rust_id).unwrap().unwrap().strength > 0.8);
        assert!(memory.penalize(rust_id, 0.4).unwrap());
        assert!(memory.get(rust_id).unwrap().unwrap().strength < 0.7);
    }

    #[test]
    fn long_term_query_filters_by_scope_and_metadata() {
        let mut memory = InMemoryLongTermMemory::new();
        let mut rust_metadata = Metadata::new();
        rust_metadata.insert("domain".to_owned(), "runtime".to_owned());
        let rust_id = memory
            .remember(
                MemoryDocumentInput::new("Rust runtime adapter memory", vec![1.0, 0.0])
                    .with_scope(MemoryScope::for_task("runtime"))
                    .with_metadata(rust_metadata),
            )
            .unwrap();
        memory
            .remember(
                MemoryDocumentInput::new("GitLab merge transcript memory", vec![1.0, 0.0])
                    .with_scope(MemoryScope::for_task("gitlab"))
                    .with_metadata({
                        let mut metadata = Metadata::new();
                        metadata.insert("domain".to_owned(), "ops".to_owned());
                        metadata
                    }),
            )
            .unwrap();
        let global_id = memory
            .remember(MemoryDocumentInput::new(
                "Global adapter pattern for clean memory",
                vec![1.0, 0.0],
            ))
            .unwrap();

        let matches = memory
            .search(
                LongTermQuery::by_text("adapter memory", 10)
                    .with_scope(MemoryScope::for_task("runtime")),
            )
            .unwrap();
        let ids = matches.iter().map(|item| item.id).collect::<Vec<_>>();
        assert!(ids.contains(&rust_id));
        assert!(ids.contains(&global_id));
        assert!(!matches.iter().any(|item| item.content.contains("GitLab")));

        let filtered = memory
            .search(
                LongTermQuery::by_text("runtime adapter", 10)
                    .with_metadata_filter("domain", "runtime"),
            )
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, rust_id);
        assert_eq!(filtered[0].scope.task_id.as_deref(), Some("runtime"));
    }

    #[test]
    fn search_penalizes_raw_fallback_and_truncated_index_content() {
        let mut memory = InMemoryLongTermMemory::new();
        let clean_id = memory
            .remember(
                MemoryDocumentInput::new("adapter recall lesson", vec![1.0, 0.0]).with_metadata({
                    let mut metadata = Metadata::new();
                    metadata.insert("content_basis".to_owned(), "clean_gist".to_owned());
                    metadata.insert("content_truncated".to_owned(), "false".to_owned());
                    metadata
                }),
            )
            .unwrap();
        let raw_id = memory
            .remember(
                MemoryDocumentInput::new("adapter recall lesson", vec![1.0, 0.0]).with_metadata({
                    let mut metadata = Metadata::new();
                    metadata.insert("content_basis".to_owned(), "raw_fallback".to_owned());
                    metadata.insert("content_truncated".to_owned(), "true".to_owned());
                    metadata
                }),
            )
            .unwrap();

        let matches = memory
            .search(LongTermQuery::by_text("adapter recall lesson", 2))
            .unwrap();

        assert_eq!(matches[0].id, clean_id);
        assert_eq!(matches[1].id, raw_id);
        assert!(matches[0].score > matches[1].score);
        assert!(matches[1].score < 0.70);
    }
}
