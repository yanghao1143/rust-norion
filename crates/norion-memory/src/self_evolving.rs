use std::collections::{BTreeMap, BTreeSet};

use crate::{MemoryError, MemoryResult, MemoryScope, Metadata, clamp01, stable_hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaseOutcome {
    Success,
    Partial,
    Failure,
}

impl CaseOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Partial => "partial",
            Self::Failure => "failure",
        }
    }

    fn score(self) -> f32 {
        match self {
            Self::Success => 1.0,
            Self::Partial => 0.62,
            Self::Failure => 0.28,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeInput {
    pub problem_description: String,
    pub solution_path: String,
    pub outcome: CaseOutcome,
    pub key_insights: Vec<String>,
    pub embedding: Vec<f32>,
    pub tags: Vec<String>,
    pub metadata: Metadata,
    pub scope: MemoryScope,
    pub strength: f32,
}

impl EpisodeInput {
    pub fn new(
        problem_description: impl Into<String>,
        solution_path: impl Into<String>,
        outcome: CaseOutcome,
    ) -> Self {
        Self {
            problem_description: problem_description.into(),
            solution_path: solution_path.into(),
            outcome,
            key_insights: Vec::new(),
            embedding: Vec::new(),
            tags: Vec::new(),
            metadata: Metadata::new(),
            scope: MemoryScope::default(),
            strength: outcome.score(),
        }
    }

    pub fn with_key_insights(mut self, key_insights: Vec<String>) -> Self {
        self.key_insights = key_insights;
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = embedding;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_scope(mut self, scope: MemoryScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = clamp01(strength);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrospectiveEpisode {
    pub id: u64,
    pub problem_description: String,
    pub solution_path: String,
    pub outcome: CaseOutcome,
    pub key_insights: Vec<String>,
    pub embedding: Vec<f32>,
    pub tags: Vec<String>,
    pub metadata: Metadata,
    pub scope: MemoryScope,
    pub strength: f32,
    pub reuse_count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeQuery {
    pub text: String,
    pub embedding: Vec<f32>,
    pub tags: Vec<String>,
    pub limit: usize,
    pub scope: Option<MemoryScope>,
}

impl EpisodeQuery {
    pub fn by_text(text: impl Into<String>, limit: usize) -> Self {
        Self {
            text: text.into(),
            embedding: Vec::new(),
            tags: Vec::new(),
            limit,
            scope: None,
        }
    }

    pub fn by_embedding(embedding: Vec<f32>, limit: usize) -> Self {
        Self {
            text: String::new(),
            embedding,
            tags: Vec::new(),
            limit,
            scope: None,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_scope(mut self, scope: MemoryScope) -> Self {
        self.scope = Some(scope);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeMatch {
    pub id: u64,
    pub score: f32,
    pub outcome: CaseOutcome,
    pub strength: f32,
    pub tags: Vec<String>,
    pub scope: MemoryScope,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveHeuristicInput {
    pub rule: String,
    pub category: String,
    pub priority: f32,
    pub confidence: f32,
    pub source_episode_id: Option<u64>,
    pub tags: Vec<String>,
    pub metadata: Metadata,
}

impl AdaptiveHeuristicInput {
    pub fn new(rule: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            category: category.into(),
            priority: 0.5,
            confidence: 0.5,
            source_episode_id: None,
            tags: Vec::new(),
            metadata: Metadata::new(),
        }
    }

    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = clamp01(priority);
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = clamp01(confidence);
        self
    }

    pub fn with_source_episode_id(mut self, source_episode_id: u64) -> Self {
        self.source_episode_id = Some(source_episode_id);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveHeuristic {
    pub id: u64,
    pub rule: String,
    pub category: String,
    pub priority: f32,
    pub confidence: f32,
    pub source_episode_id: Option<u64>,
    pub tags: Vec<String>,
    pub metadata: Metadata,
    pub version: u64,
    pub last_updated_step: u64,
    pub usage_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeuristicQuery {
    pub text: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub limit: usize,
}

impl HeuristicQuery {
    pub fn new(text: impl Into<String>, limit: usize) -> Self {
        Self {
            text: text.into(),
            category: None,
            tags: Vec::new(),
            limit,
        }
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeuristicMatch {
    pub id: u64,
    pub score: f32,
    pub rule: String,
    pub category: String,
    pub priority: f32,
    pub confidence: f32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolReliabilityUpdate {
    pub tool_name: String,
    pub succeeded: bool,
    pub quality: f32,
    pub metadata: Metadata,
}

impl ToolReliabilityUpdate {
    pub fn new(tool_name: impl Into<String>, succeeded: bool, quality: f32) -> Self {
        Self {
            tool_name: tool_name.into(),
            succeeded,
            quality: clamp01(quality),
            metadata: Metadata::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolReliabilityRecord {
    pub tool_name: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_quality: f32,
    pub reliability_score: f32,
    pub last_used_step: u64,
    pub metadata: Metadata,
}

impl ToolReliabilityRecord {
    pub fn total_count(&self) -> u64 {
        self.success_count.saturating_add(self.failure_count)
    }

    pub fn success_rate(&self) -> f32 {
        let total = self.total_count();
        if total == 0 {
            return 0.0;
        }
        self.success_count as f32 / total as f32
    }

    pub fn summary_line(&self) -> String {
        format!(
            "tool_reliability tool={} total={} success_rate={:.3} avg_quality={:.3} reliability={:.3} last_used_step={}",
            stable_code(&self.tool_name),
            self.total_count(),
            self.success_rate(),
            self.avg_quality,
            self.reliability_score,
            self.last_used_step,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingCaseReflection {
    pub episode: EpisodeInput,
    pub heuristics: Vec<AdaptiveHeuristicInput>,
    pub tool_updates: Vec<ToolReliabilityUpdate>,
}

impl SelfEvolvingCaseReflection {
    pub fn new(episode: EpisodeInput) -> Self {
        Self {
            episode,
            heuristics: Vec::new(),
            tool_updates: Vec::new(),
        }
    }

    pub fn with_heuristics(mut self, heuristics: Vec<AdaptiveHeuristicInput>) -> Self {
        self.heuristics = heuristics;
        self
    }

    pub fn with_tool_updates(mut self, tool_updates: Vec<ToolReliabilityUpdate>) -> Self {
        self.tool_updates = tool_updates;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolvingReflectionReport {
    pub episode_id: u64,
    pub heuristic_ids: Vec<u64>,
    pub tool_codes: Vec<String>,
    pub outcome: CaseOutcome,
}

impl SelfEvolvingReflectionReport {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolving_reflection_report episode_id={} outcome={} heuristics={} tools={} tool_codes={}",
            self.episode_id,
            self.outcome.as_str(),
            self.heuristic_ids.len(),
            self.tool_codes.len(),
            join_codes(self.tool_codes.clone()),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemorySnapshot {
    pub episodes: usize,
    pub heuristics: usize,
    pub tools: usize,
    pub top_tool_code: Option<String>,
    pub average_tool_reliability: f32,
}

impl SelfEvolvingMemorySnapshot {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolving_memory_snapshot episodes={} heuristics={} tools={} top_tool={} average_tool_reliability={:.3} persistent_writes_allowed=false",
            self.episodes,
            self.heuristics,
            self.tools,
            self.top_tool_code.as_deref().unwrap_or("none"),
            self.average_tool_reliability,
        )
    }
}

pub trait SelfEvolvingMemory {
    fn add_episode(&mut self, input: EpisodeInput) -> MemoryResult<u64>;
    fn get_episode(&self, id: u64) -> MemoryResult<Option<RetrospectiveEpisode>>;
    fn search_episodes(&self, query: EpisodeQuery) -> MemoryResult<Vec<EpisodeMatch>>;
    fn upsert_heuristic(&mut self, input: AdaptiveHeuristicInput) -> MemoryResult<u64>;
    fn search_heuristics(&self, query: HeuristicQuery) -> MemoryResult<Vec<HeuristicMatch>>;
    fn record_tool_result(
        &mut self,
        update: ToolReliabilityUpdate,
    ) -> MemoryResult<ToolReliabilityRecord>;
    fn tool_reliability(&self, tool_name: &str) -> MemoryResult<Option<ToolReliabilityRecord>>;
    fn rank_tools(&self, limit: usize) -> MemoryResult<Vec<ToolReliabilityRecord>>;
    fn record_reflection(
        &mut self,
        reflection: SelfEvolvingCaseReflection,
    ) -> MemoryResult<SelfEvolvingReflectionReport>;
    fn snapshot(&self) -> SelfEvolvingMemorySnapshot;
}

#[derive(Debug, Clone)]
pub struct InMemorySelfEvolvingMemory {
    episodes: BTreeMap<u64, RetrospectiveEpisode>,
    heuristics: BTreeMap<u64, AdaptiveHeuristic>,
    heuristic_index: BTreeMap<String, u64>,
    tool_reliability: BTreeMap<String, ToolReliabilityRecord>,
    next_episode_id: u64,
    next_heuristic_id: u64,
    logical_step: u64,
}

impl Default for InMemorySelfEvolvingMemory {
    fn default() -> Self {
        Self {
            episodes: BTreeMap::new(),
            heuristics: BTreeMap::new(),
            heuristic_index: BTreeMap::new(),
            tool_reliability: BTreeMap::new(),
            next_episode_id: 1,
            next_heuristic_id: 1,
            logical_step: 0,
        }
    }
}

impl InMemorySelfEvolvingMemory {
    pub fn new() -> Self {
        Self::default()
    }

    fn advance_step(&mut self) -> u64 {
        self.logical_step = self.logical_step.saturating_add(1);
        self.logical_step
    }
}

impl SelfEvolvingMemory for InMemorySelfEvolvingMemory {
    fn add_episode(&mut self, input: EpisodeInput) -> MemoryResult<u64> {
        validate_episode_input(&input)?;
        let id = self.next_episode_id;
        self.next_episode_id = self.next_episode_id.saturating_add(1);
        self.episodes.insert(
            id,
            RetrospectiveEpisode {
                id,
                problem_description: input.problem_description,
                solution_path: input.solution_path,
                outcome: input.outcome,
                key_insights: clean_strings(input.key_insights),
                embedding: input.embedding,
                tags: clean_tags(input.tags),
                metadata: input.metadata,
                scope: input.scope,
                strength: clamp01(input.strength),
                reuse_count: 0,
            },
        );
        Ok(id)
    }

    fn get_episode(&self, id: u64) -> MemoryResult<Option<RetrospectiveEpisode>> {
        Ok(self.episodes.get(&id).cloned())
    }

    fn search_episodes(&self, query: EpisodeQuery) -> MemoryResult<Vec<EpisodeMatch>> {
        let limit = query.limit.max(1);
        let query_tokens = tokens(&query.text);
        let query_tags = clean_tags(query.tags)
            .into_iter()
            .map(|tag| tag.to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        let mut matches = self
            .episodes
            .values()
            .filter(|episode| scope_matches(query.scope.as_ref(), &episode.scope))
            .filter_map(|episode| {
                let text = format!(
                    "{} {} {}",
                    episode.problem_description,
                    episode.solution_path,
                    episode.key_insights.join(" ")
                );
                let text_score = overlap(&query_tokens, &tokens(&text));
                let vector_score = cosine_similarity(&query.embedding, &episode.embedding);
                let tag_score = tag_overlap(&query_tags, &episode.tags);
                let score = (text_score.max(vector_score) * 0.62
                    + tag_score * 0.16
                    + episode.strength * 0.12
                    + episode.outcome.score() * 0.10)
                    .clamp(0.0, 1.0);
                (score > 0.0).then_some(EpisodeMatch {
                    id: episode.id,
                    score,
                    outcome: episode.outcome,
                    strength: episode.strength,
                    tags: episode.tags.clone(),
                    scope: episode.scope.clone(),
                })
            })
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

    fn upsert_heuristic(&mut self, input: AdaptiveHeuristicInput) -> MemoryResult<u64> {
        validate_heuristic_input(&input)?;
        let key = heuristic_key(&input.category, &input.rule);
        let step = self.advance_step();
        if let Some(id) = self.heuristic_index.get(&key).copied() {
            let Some(record) = self.heuristics.get_mut(&id) else {
                return Err(MemoryError::NotFound(format!(
                    "heuristic index pointed to missing id {id}"
                )));
            };
            record.priority = clamp01((record.priority + input.priority) / 2.0);
            record.confidence = clamp01((record.confidence + input.confidence) / 2.0);
            record.source_episode_id = input.source_episode_id.or(record.source_episode_id);
            record.tags = merge_tags(&record.tags, input.tags);
            record.metadata.extend(input.metadata);
            record.version = record.version.saturating_add(1);
            record.last_updated_step = step;
            return Ok(id);
        }

        let id = self.next_heuristic_id;
        self.next_heuristic_id = self.next_heuristic_id.saturating_add(1);
        self.heuristic_index.insert(key, id);
        self.heuristics.insert(
            id,
            AdaptiveHeuristic {
                id,
                rule: input.rule,
                category: input.category,
                priority: clamp01(input.priority),
                confidence: clamp01(input.confidence),
                source_episode_id: input.source_episode_id,
                tags: clean_tags(input.tags),
                metadata: input.metadata,
                version: 1,
                last_updated_step: step,
                usage_count: 0,
            },
        );
        Ok(id)
    }

    fn search_heuristics(&self, query: HeuristicQuery) -> MemoryResult<Vec<HeuristicMatch>> {
        let limit = query.limit.max(1);
        let query_tokens = tokens(&query.text);
        let query_category = query.category.as_deref().map(normalize_token);
        let query_tags = clean_tags(query.tags)
            .into_iter()
            .map(|tag| tag.to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        let mut matches = self
            .heuristics
            .values()
            .filter_map(|heuristic| {
                let text = format!("{} {}", heuristic.category, heuristic.rule);
                let text_score = overlap(&query_tokens, &tokens(&text));
                let category_score = query_category.as_ref().map_or(0.0, |category| {
                    (normalize_token(&heuristic.category) == *category) as u8 as f32
                });
                let tag_score = tag_overlap(&query_tags, &heuristic.tags);
                let score = (text_score * 0.45
                    + category_score * 0.20
                    + tag_score * 0.15
                    + heuristic.priority * 0.10
                    + heuristic.confidence * 0.10)
                    .clamp(0.0, 1.0);
                (score > 0.0).then_some(HeuristicMatch {
                    id: heuristic.id,
                    score,
                    rule: heuristic.rule.clone(),
                    category: heuristic.category.clone(),
                    priority: heuristic.priority,
                    confidence: heuristic.confidence,
                    tags: heuristic.tags.clone(),
                })
            })
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

    fn record_tool_result(
        &mut self,
        update: ToolReliabilityUpdate,
    ) -> MemoryResult<ToolReliabilityRecord> {
        validate_tool_update(&update)?;
        let step = self.advance_step();
        let key = normalize_token(&update.tool_name);
        let record = self
            .tool_reliability
            .entry(key)
            .or_insert_with(|| ToolReliabilityRecord {
                tool_name: update.tool_name.clone(),
                success_count: 0,
                failure_count: 0,
                avg_quality: 0.0,
                reliability_score: 0.0,
                last_used_step: 0,
                metadata: Metadata::new(),
            });
        let total_before = record.total_count();
        record.avg_quality = if total_before == 0 {
            clamp01(update.quality)
        } else {
            clamp01(
                ((record.avg_quality * total_before as f32) + update.quality)
                    / (total_before.saturating_add(1) as f32),
            )
        };
        if update.succeeded {
            record.success_count = record.success_count.saturating_add(1);
        } else {
            record.failure_count = record.failure_count.saturating_add(1);
        }
        record.metadata.extend(update.metadata);
        record.last_used_step = step;
        record.reliability_score =
            (record.success_rate() * 0.62 + record.avg_quality * 0.38).clamp(0.0, 1.0);
        Ok(record.clone())
    }

    fn tool_reliability(&self, tool_name: &str) -> MemoryResult<Option<ToolReliabilityRecord>> {
        Ok(self
            .tool_reliability
            .get(&normalize_token(tool_name))
            .cloned())
    }

    fn rank_tools(&self, limit: usize) -> MemoryResult<Vec<ToolReliabilityRecord>> {
        let mut records = self.tool_reliability.values().cloned().collect::<Vec<_>>();
        records.sort_by(|left, right| {
            right
                .reliability_score
                .partial_cmp(&left.reliability_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.last_used_step.cmp(&left.last_used_step))
                .then_with(|| left.tool_name.cmp(&right.tool_name))
        });
        records.truncate(limit.max(1));
        Ok(records)
    }

    fn record_reflection(
        &mut self,
        reflection: SelfEvolvingCaseReflection,
    ) -> MemoryResult<SelfEvolvingReflectionReport> {
        validate_case_reflection(&reflection)?;

        let outcome = reflection.episode.outcome;
        let episode_id = self.add_episode(reflection.episode)?;
        let mut heuristic_ids = Vec::with_capacity(reflection.heuristics.len());
        for heuristic in reflection.heuristics {
            heuristic_ids.push(self.upsert_heuristic(heuristic)?);
        }
        let mut tool_codes = Vec::with_capacity(reflection.tool_updates.len());
        for update in reflection.tool_updates {
            tool_codes.push(stable_code(&update.tool_name));
            self.record_tool_result(update)?;
        }
        tool_codes.sort();
        tool_codes.dedup();
        Ok(SelfEvolvingReflectionReport {
            episode_id,
            heuristic_ids,
            tool_codes,
            outcome,
        })
    }

    fn snapshot(&self) -> SelfEvolvingMemorySnapshot {
        let ranked = self.rank_tools(1).unwrap_or_default();
        let average_tool_reliability = if self.tool_reliability.is_empty() {
            0.0
        } else {
            self.tool_reliability
                .values()
                .map(|record| record.reliability_score)
                .sum::<f32>()
                / self.tool_reliability.len() as f32
        };
        SelfEvolvingMemorySnapshot {
            episodes: self.episodes.len(),
            heuristics: self.heuristics.len(),
            tools: self.tool_reliability.len(),
            top_tool_code: ranked.first().map(|record| stable_code(&record.tool_name)),
            average_tool_reliability,
        }
    }
}

fn validate_episode_input(input: &EpisodeInput) -> MemoryResult<()> {
    if input.problem_description.trim().is_empty() {
        return Err(MemoryError::InvalidInput(
            "episode problem description cannot be empty".to_owned(),
        ));
    }
    if input.solution_path.trim().is_empty() {
        return Err(MemoryError::InvalidInput(
            "episode solution path cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_heuristic_input(input: &AdaptiveHeuristicInput) -> MemoryResult<()> {
    if input.rule.trim().is_empty() {
        return Err(MemoryError::InvalidInput(
            "heuristic rule cannot be empty".to_owned(),
        ));
    }
    if input.category.trim().is_empty() {
        return Err(MemoryError::InvalidInput(
            "heuristic category cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_tool_update(update: &ToolReliabilityUpdate) -> MemoryResult<()> {
    if update.tool_name.trim().is_empty() {
        return Err(MemoryError::InvalidInput(
            "tool name cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_case_reflection(reflection: &SelfEvolvingCaseReflection) -> MemoryResult<()> {
    validate_episode_input(&reflection.episode)?;
    for heuristic in &reflection.heuristics {
        validate_heuristic_input(heuristic)?;
    }
    for update in &reflection.tool_updates {
        validate_tool_update(update)?;
    }
    Ok(())
}

fn scope_matches(query_scope: Option<&MemoryScope>, record_scope: &MemoryScope) -> bool {
    let Some(query_scope) = query_scope else {
        return false;
    };
    query_scope
        .scope_mismatch_reason(record_scope, false)
        .is_none()
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

fn overlap(left: &BTreeSet<String>, right: &BTreeSet<String>) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let shared = left.intersection(right).count() as f32;
    (shared / left.len().min(right.len()) as f32).clamp(0.0, 1.0)
}

fn tag_overlap(query_tags: &BTreeSet<String>, record_tags: &[String]) -> f32 {
    if query_tags.is_empty() {
        return 0.0;
    }
    let record_tags = record_tags
        .iter()
        .map(|tag| tag.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    query_tags.intersection(&record_tags).count() as f32 / query_tags.len() as f32
}

fn tokens(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn clean_strings(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect()
}

fn clean_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn merge_tags(left: &[String], right: Vec<String>) -> Vec<String> {
    left.iter()
        .cloned()
        .chain(clean_tags(right))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn heuristic_key(category: &str, rule: &str) -> String {
    format!("{}:{}", normalize_token(category), normalize_token(rule))
}

fn normalize_token(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_ascii_lowercase()
}

fn stable_code(value: &str) -> String {
    format!("{:016x}", stable_hash(normalize_token(value).as_bytes()))
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_evolving_memory_records_and_retrieves_episodes() {
        let mut memory = InMemorySelfEvolvingMemory::new();
        let rust_episode_id = memory
            .add_episode(
                EpisodeInput::new(
                    "Rust compiler rejected the generated adapter",
                    "Run cargo test, inspect borrow error, then repair ownership boundary",
                    CaseOutcome::Success,
                )
                .with_key_insights(vec![
                    "Prefer borrowed slices before cloning adapter payloads".to_owned(),
                ])
                .with_embedding(vec![1.0, 0.0, 0.0])
                .with_tags(vec!["rust".to_owned(), "compiler".to_owned()])
                .with_scope(MemoryScope::for_task("runtime")),
            )
            .unwrap();
        memory
            .add_episode(
                EpisodeInput::new(
                    "Markdown roadmap update needed review",
                    "Collect roadmap evidence and avoid runtime code changes",
                    CaseOutcome::Partial,
                )
                .with_embedding(vec![0.0, 1.0, 0.0])
                .with_tags(vec!["docs".to_owned()])
                .with_scope(MemoryScope::for_task("docs")),
            )
            .unwrap();

        let matches = memory
            .search_episodes(
                EpisodeQuery::by_text("borrow compiler adapter", 5)
                    .with_tags(vec!["rust".to_owned()])
                    .with_scope(MemoryScope::for_task("runtime")),
            )
            .unwrap();

        assert_eq!(matches[0].id, rust_episode_id);
        assert_eq!(matches[0].outcome, CaseOutcome::Success);
        assert!(matches[0].score > 0.70, "score={}", matches[0].score);

        let vector_matches = memory
            .search_episodes(
                EpisodeQuery::by_embedding(vec![0.9, 0.1, 0.0], 1)
                    .with_scope(MemoryScope::for_task("runtime")),
            )
            .unwrap();
        assert_eq!(vector_matches[0].id, rust_episode_id);
    }

    #[test]
    fn episode_query_without_request_scope_returns_no_matches() {
        let mut memory = InMemorySelfEvolvingMemory::new();
        memory
            .add_episode(EpisodeInput::new(
                "Rust compiler rejected the generated adapter",
                "Run cargo test and repair ownership boundary",
                CaseOutcome::Success,
            ))
            .unwrap();

        let matches = memory
            .search_episodes(EpisodeQuery::by_text("compiler adapter", 10))
            .unwrap();

        assert!(matches.is_empty());
    }

    #[test]
    fn adaptive_heuristics_upsert_and_rank_by_relevance() {
        let mut memory = InMemorySelfEvolvingMemory::new();
        let first = memory
            .upsert_heuristic(
                AdaptiveHeuristicInput::new(
                    "Run focused cargo tests before proposing a memory promotion",
                    "validation",
                )
                .with_priority(0.9)
                .with_confidence(0.8)
                .with_tags(vec!["rust".to_owned(), "gate".to_owned()]),
            )
            .unwrap();
        let duplicate = memory
            .upsert_heuristic(
                AdaptiveHeuristicInput::new(
                    "Run focused cargo tests before proposing a memory promotion",
                    "validation",
                )
                .with_priority(0.7)
                .with_confidence(0.9)
                .with_tags(vec!["compiler".to_owned()]),
            )
            .unwrap();

        assert_eq!(first, duplicate);
        let record = memory.heuristics.get(&first).unwrap();
        assert_eq!(record.version, 2);
        assert_eq!(
            record.tags,
            vec!["compiler".to_owned(), "gate".to_owned(), "rust".to_owned()]
        );

        let matches = memory
            .search_heuristics(
                HeuristicQuery::new("cargo tests memory gate", 3)
                    .with_category("validation")
                    .with_tags(vec!["rust".to_owned()]),
            )
            .unwrap();

        assert_eq!(matches[0].id, first);
        assert!(matches[0].score > 0.65, "score={}", matches[0].score);
    }

    #[test]
    fn tool_reliability_tracks_quality_and_ranks_tools() {
        let mut memory = InMemorySelfEvolvingMemory::new();
        memory
            .record_tool_result(ToolReliabilityUpdate::new("cargo-test", true, 0.95))
            .unwrap();
        memory
            .record_tool_result(ToolReliabilityUpdate::new("cargo-test", true, 0.85))
            .unwrap();
        memory
            .record_tool_result(ToolReliabilityUpdate::new("remote-shell", false, 0.20))
            .unwrap();

        let cargo = memory.tool_reliability("cargo-test").unwrap().unwrap();
        assert_eq!(cargo.total_count(), 2);
        assert!((cargo.success_rate() - 1.0).abs() < f32::EPSILON);
        assert!(cargo.reliability_score > 0.90);
        assert!(cargo.summary_line().contains("tool="));
        assert!(!cargo.summary_line().contains("cargo-test"));

        let ranked = memory.rank_tools(2).unwrap();
        assert_eq!(ranked[0].tool_name, "cargo-test");
        assert_eq!(ranked[1].tool_name, "remote-shell");
    }

    #[test]
    fn reflection_records_episode_heuristics_and_tool_reliability_without_raw_summary() {
        let mut memory = InMemorySelfEvolvingMemory::new();
        let secret = "PRIVATE_PROMPT_PAYLOAD_DO_NOT_LOG";
        let report = memory
            .record_reflection(
                SelfEvolvingCaseReflection::new(
                    EpisodeInput::new(
                        format!("Rust failure with {secret}"),
                        "Use clean gist and run focused tests",
                        CaseOutcome::Success,
                    )
                    .with_tags(vec!["rust".to_owned()]),
                )
                .with_heuristics(vec![
                    AdaptiveHeuristicInput::new(
                        "When a clean gist exists, never persist raw transcript text",
                        "privacy",
                    )
                    .with_priority(0.95)
                    .with_confidence(0.9),
                ])
                .with_tool_updates(vec![ToolReliabilityUpdate::new("cargo-test", true, 0.92)]),
            )
            .unwrap();

        assert_eq!(report.episode_id, 1);
        assert_eq!(report.heuristic_ids, vec![1]);
        assert_eq!(memory.snapshot().episodes, 1);
        assert_eq!(memory.snapshot().heuristics, 1);
        assert_eq!(memory.snapshot().tools, 1);
        assert!(report.summary_line().contains("outcome=success"));
        assert!(!report.summary_line().contains(secret));
        let snapshot_line = memory.snapshot().summary_line();
        assert!(snapshot_line.contains("persistent_writes_allowed=false"));
        assert!(!snapshot_line.contains("cargo-test"));
    }

    #[test]
    fn reflection_rejects_invalid_batch_without_partial_memory_writes() {
        let mut memory = InMemorySelfEvolvingMemory::new();
        let result = memory.record_reflection(
            SelfEvolvingCaseReflection::new(EpisodeInput::new(
                "valid episode before invalid tool update",
                "reject the whole reflection batch",
                CaseOutcome::Success,
            ))
            .with_heuristics(vec![AdaptiveHeuristicInput::new(
                "valid heuristic must not be written when tool input is invalid",
                "atomicity",
            )])
            .with_tool_updates(vec![ToolReliabilityUpdate::new("", true, 0.9)]),
        );

        assert!(matches!(result, Err(MemoryError::InvalidInput(_))));
        let snapshot = memory.snapshot();
        assert_eq!(snapshot.episodes, 0);
        assert_eq!(snapshot.heuristics, 0);
        assert_eq!(snapshot.tools, 0);
    }

    #[test]
    fn self_evolving_memory_rejects_empty_inputs() {
        let mut memory = InMemorySelfEvolvingMemory::new();

        assert!(matches!(
            memory.add_episode(EpisodeInput::new("", "repair", CaseOutcome::Failure)),
            Err(MemoryError::InvalidInput(_))
        ));
        assert!(matches!(
            memory.upsert_heuristic(AdaptiveHeuristicInput::new("", "validation")),
            Err(MemoryError::InvalidInput(_))
        ));
        assert!(matches!(
            memory.record_tool_result(ToolReliabilityUpdate::new("", true, 1.0)),
            Err(MemoryError::InvalidInput(_))
        ));
    }
}
