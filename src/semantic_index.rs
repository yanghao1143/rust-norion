use std::collections::BTreeSet;

use norion_memory::{
    DefaultMemorySemanticRetriever, MemoryIndexDocument, MemoryIndexSource, MemoryScope,
    MemorySemanticQuery, MemorySemanticRetriever, Metadata, memory_index_digest,
};

use crate::experience::ExperienceRecord;
use crate::hierarchy::TaskProfile;
use crate::reasoning_genome::{
    ClassifiedGeneSegment, DnaSplicePreview, GeneSegment, GeneSegmentDisposition,
};
use crate::self_evolving_memory::{
    SelfEvolvingEpisodeRecord, SelfEvolvingHeuristicRecord, SelfEvolvingMemoryStore,
    ToolReliabilityRecord,
};

const MOCK_EMBEDDING_DIMS: usize = 8;
const PRIVACY_BLOCK_THRESHOLD: f32 = 0.20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticIndexLane {
    Experience,
    Episode,
    Heuristic,
    ToolReliability,
    GeneSegment,
}

impl SemanticIndexLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Experience => "experience",
            Self::Episode => "episode",
            Self::Heuristic => "heuristic",
            Self::ToolReliability => "tool_reliability",
            Self::GeneSegment => "gene_segment",
        }
    }

    fn source(self) -> MemoryIndexSource {
        match self {
            Self::Experience | Self::Episode => MemoryIndexSource::Experience,
            Self::Heuristic | Self::ToolReliability => MemoryIndexSource::LongTerm,
            Self::GeneSegment => MemoryIndexSource::GeneSegment,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticIndexRecord {
    pub id: String,
    pub lane: SemanticIndexLane,
    pub profile: TaskProfile,
    pub tenant_scope: String,
    pub source_anchor: String,
    pub content_digest: String,
    pub token_estimate: usize,
    pub document: MemoryIndexDocument,
}

impl SemanticIndexRecord {
    pub fn from_experience(record: &ExperienceRecord, tenant_scope: impl Into<String>) -> Self {
        let tenant_scope = tenant_scope.into();
        let content = experience_content(record);
        let tags = [
            "experience".to_owned(),
            format!("profile:{}", profile_slug(record.profile)),
        ]
        .into_iter()
        .chain(runtime_tags(record))
        .collect::<Vec<_>>();
        let source_anchor = format!("experience:{}", record.id);
        Self::from_projected_content(
            format!("experience:{}", record.id),
            SemanticIndexLane::Experience,
            record.profile,
            tenant_scope,
            source_anchor,
            content,
            tags,
            record.quality,
            record.quality,
            1.0,
            None,
        )
    }

    pub fn from_episode(
        record: &SelfEvolvingEpisodeRecord,
        tenant_scope: impl Into<String>,
        current_step: u64,
    ) -> Self {
        let tenant_scope = tenant_scope.into();
        let content = format!(
            "episode profile={} tags={} problem_digest={} outcome_digest={} insights={} source_case={}",
            profile_slug(record.profile),
            join_sorted(&record.tags),
            record.problem_digest,
            record.outcome_digest,
            record.key_insight_digests.len(),
            record.source_case_digest
        );
        let tags = lane_tags(SemanticIndexLane::Episode, record.profile, &record.tags);
        let freshness = freshness_from_age(current_step.saturating_sub(record.sequence));
        let mut metadata_flags = Vec::new();
        if !record.active {
            metadata_flags.push(("quarantined", "true"));
        }
        Self::from_projected_content(
            record.record_id.clone(),
            SemanticIndexLane::Episode,
            record.profile,
            tenant_scope,
            record.source_case_digest.clone(),
            content,
            tags,
            record.quality,
            record.quality,
            freshness,
            Some(metadata_flags),
        )
    }

    pub fn from_heuristic(
        record: &SelfEvolvingHeuristicRecord,
        tenant_scope: impl Into<String>,
        current_step: u64,
    ) -> Self {
        let tenant_scope = tenant_scope.into();
        let content = format!(
            "heuristic profile={} tags={} rule_digest={} source_case={} support={} decay={}",
            profile_slug(record.profile),
            join_sorted(&record.tags),
            record.rule_digest,
            record.source_case_digest,
            record.support_count,
            record.decay_count
        );
        let tags = lane_tags(SemanticIndexLane::Heuristic, record.profile, &record.tags);
        let freshness = freshness_from_age(current_step.saturating_sub(record.last_updated_step));
        let mut metadata_flags = Vec::new();
        if record.quarantined {
            metadata_flags.push(("quarantined", "true"));
            if let Some(reason) = &record.quarantine_reason {
                metadata_flags.push(("quarantine_reason", reason.as_str()));
            }
        }
        Self::from_projected_content(
            record.record_id.clone(),
            SemanticIndexLane::Heuristic,
            record.profile,
            tenant_scope,
            record.source_case_digest.clone(),
            content,
            tags,
            record.priority,
            record.confidence,
            freshness,
            Some(metadata_flags),
        )
    }

    pub fn from_tool_reliability(
        record: &ToolReliabilityRecord,
        tenant_scope: impl Into<String>,
        current_step: u64,
    ) -> Self {
        let tenant_scope = tenant_scope.into();
        let content = format!(
            "tool reliability profile={} tool={} tool_digest={} observations={} success_rate={:.3} avg_quality={:.3}",
            profile_slug(record.profile),
            record.tool_id,
            record.tool_digest,
            record.observations,
            record.success_rate,
            record.avg_quality
        );
        let tags = lane_tags(
            SemanticIndexLane::ToolReliability,
            record.profile,
            &[record.tool_id.clone()],
        );
        let freshness = freshness_from_age(current_step.saturating_sub(record.last_used_step));
        Self::from_projected_content(
            format!("sem:tool-reliability:{}", record.tool_id),
            SemanticIndexLane::ToolReliability,
            record.profile,
            tenant_scope,
            record.tool_digest.clone(),
            content,
            tags,
            record.trust_score,
            record.trust_score,
            freshness,
            None,
        )
    }

    pub fn from_gene_segment(classified: &ClassifiedGeneSegment) -> Self {
        let segment = &classified.segment;
        let content = gene_segment_content(segment);
        let mut tags = lane_tags(SemanticIndexLane::GeneSegment, segment.profile, &[]);
        tags.extend([
            format!("source:{}", segment.source.as_str()),
            format!("disposition:{}", classified.disposition.as_str()),
            format!("kv:{}", segment.kv_residency.as_str()),
        ]);

        let mut metadata_flags = Vec::new();
        if segment.privacy_risk > PRIVACY_BLOCK_THRESHOLD {
            metadata_flags.push(("privacy", "blocked"));
            metadata_flags.push(("privacy_blocked", "true"));
        }
        match classified.disposition {
            GeneSegmentDisposition::Quarantined => {
                metadata_flags.push(("quarantined", "true"));
                metadata_flags.push(("gene_status", "quarantined"));
            }
            GeneSegmentDisposition::RepairCandidate => {
                metadata_flags.push(("gene_status", "corrupt"));
            }
            GeneSegmentDisposition::Retained | GeneSegmentDisposition::Skipped => {}
        }
        if !segment.schema_valid || !segment.kv_shape_valid {
            metadata_flags.push(("gene_status", "corrupt"));
        }

        let source_anchor = if segment.source_hash.trim().is_empty() {
            format!("gene:{}", segment.id)
        } else {
            segment.source_hash.clone()
        };
        let strength = (segment.fitness * (1.0 - segment.drift_score)).clamp(0.0, 1.0);
        Self::from_projected_content(
            segment.id.clone(),
            SemanticIndexLane::GeneSegment,
            segment.profile,
            segment.tenant_scope.clone(),
            source_anchor,
            content,
            tags,
            strength,
            segment.fitness,
            freshness_from_segment_age(segment.age),
            Some(metadata_flags),
        )
    }

    pub fn retained_gene_segment(segment: &GeneSegment) -> Self {
        Self::from_gene_segment(&ClassifiedGeneSegment {
            segment: segment.clone(),
            class: crate::reasoning_genome::GeneSegmentClass::Exon,
            disposition: GeneSegmentDisposition::Retained,
            reasons: Vec::new(),
        })
    }

    fn from_projected_content(
        id: String,
        lane: SemanticIndexLane,
        profile: TaskProfile,
        tenant_scope: String,
        source_anchor: String,
        content: String,
        tags: Vec<String>,
        strength: f32,
        confidence: f32,
        freshness: f32,
        metadata_flags: Option<Vec<(&str, &str)>>,
    ) -> Self {
        let content_digest = stable_digest(&content);
        let token_estimate = estimate_tokens(&content);
        let mut metadata = Metadata::new();
        metadata.insert("lane".to_owned(), lane.as_str().to_owned());
        metadata.insert("profile".to_owned(), profile_slug(profile).to_owned());
        metadata.insert("tenant_scope".to_owned(), tenant_scope.clone());
        metadata.insert("source_anchor".to_owned(), source_anchor.clone());
        metadata.insert("content_digest".to_owned(), content_digest.clone());
        metadata.insert("content_basis".to_owned(), "redacted_projection".to_owned());
        metadata.insert("tags".to_owned(), join_sorted(&tags));
        metadata.insert("confidence".to_owned(), format_unit(confidence));
        metadata.insert("freshness".to_owned(), format_unit(freshness));
        metadata.insert("token_estimate".to_owned(), token_estimate.to_string());
        metadata.insert("redacted".to_owned(), "true".to_owned());
        for (key, value) in metadata_flags.unwrap_or_default() {
            metadata.insert(key.to_owned(), value.to_owned());
        }

        let document = MemoryIndexDocument::new(id.clone(), lane.source(), content.clone())
            .with_embedding(mock_embedding(&content))
            .with_scope(memory_scope(profile, &tenant_scope))
            .with_metadata(metadata)
            .with_strength(strength);

        Self {
            id,
            lane,
            profile,
            tenant_scope,
            source_anchor,
            content_digest,
            token_estimate,
            document,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SemanticIndex {
    records: Vec<SemanticIndexRecord>,
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_self_evolving_store(
        store: &SelfEvolvingMemoryStore,
        tenant_scope: impl Into<String>,
        current_step: u64,
    ) -> Self {
        Self::new().with_self_evolving_store(store, tenant_scope, current_step)
    }

    pub fn from_splice_preview(preview: &DnaSplicePreview) -> Self {
        Self::new().with_splice_preview(preview)
    }

    pub fn with_experience_records(
        mut self,
        records: &[ExperienceRecord],
        tenant_scope: impl Into<String>,
    ) -> Self {
        let tenant_scope = tenant_scope.into();
        self.records.extend(
            records
                .iter()
                .map(|record| SemanticIndexRecord::from_experience(record, tenant_scope.clone())),
        );
        self
    }

    pub fn with_self_evolving_store(
        mut self,
        store: &SelfEvolvingMemoryStore,
        tenant_scope: impl Into<String>,
        current_step: u64,
    ) -> Self {
        let tenant_scope = tenant_scope.into();
        self.records.extend(store.episodes().iter().map(|record| {
            SemanticIndexRecord::from_episode(record, tenant_scope.clone(), current_step)
        }));
        self.records.extend(store.heuristics().iter().map(|record| {
            SemanticIndexRecord::from_heuristic(record, tenant_scope.clone(), current_step)
        }));
        self.records
            .extend(store.tool_reliability().iter().map(|record| {
                SemanticIndexRecord::from_tool_reliability(
                    record,
                    tenant_scope.clone(),
                    current_step,
                )
            }));
        self
    }

    pub fn with_splice_preview(mut self, preview: &DnaSplicePreview) -> Self {
        self.records.extend(
            preview
                .segments
                .iter()
                .map(SemanticIndexRecord::from_gene_segment),
        );
        self
    }

    pub fn push_record(&mut self, record: SemanticIndexRecord) {
        self.records.push(record);
    }

    pub fn records(&self) -> &[SemanticIndexRecord] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn rebuild_digest(&self) -> u64 {
        let documents = self.documents();
        memory_index_digest(&documents)
    }

    pub fn retrieve(&self, query: &SemanticIndexQuery) -> SemanticIndexRetrievalReport {
        let documents = self.documents();
        let memory_query = query.to_memory_query();
        let plan = DefaultMemorySemanticRetriever.retrieve(&documents, &memory_query);
        SemanticIndexRetrievalReport::from_plan(query, &plan, &self.records)
    }

    fn documents(&self) -> Vec<MemoryIndexDocument> {
        self.records
            .iter()
            .map(|record| record.document.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticIndexQuery {
    pub text: String,
    pub profile: TaskProfile,
    pub tenant_scope: String,
    pub record_limit: usize,
    pub token_budget: usize,
    pub min_score: f32,
    pub include_repair_candidates: bool,
    pub allow_cross_profile: bool,
}

impl SemanticIndexQuery {
    pub fn new(
        text: impl Into<String>,
        profile: TaskProfile,
        tenant_scope: impl Into<String>,
    ) -> Self {
        Self {
            text: text.into(),
            profile,
            tenant_scope: tenant_scope.into(),
            record_limit: 8,
            token_budget: 1_024,
            min_score: 0.05,
            include_repair_candidates: false,
            allow_cross_profile: false,
        }
    }

    pub fn with_record_limit(mut self, record_limit: usize) -> Self {
        self.record_limit = record_limit.max(1);
        self
    }

    pub fn with_token_budget(mut self, token_budget: usize) -> Self {
        self.token_budget = token_budget.max(1);
        self
    }

    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = min_score.clamp(0.0, 1.0);
        self
    }

    pub fn include_repair_candidates(mut self, include_repair_candidates: bool) -> Self {
        self.include_repair_candidates = include_repair_candidates;
        self
    }

    pub fn allow_cross_profile(mut self, allow_cross_profile: bool) -> Self {
        self.allow_cross_profile = allow_cross_profile;
        self
    }

    fn to_memory_query(&self) -> MemorySemanticQuery {
        MemorySemanticQuery::new(self.text.clone(), self.record_limit)
            .with_embedding(mock_embedding(&self.text))
            .with_scope(memory_scope(self.profile, &self.tenant_scope))
            .with_token_budget(self.token_budget)
            .with_min_score(self.min_score)
            .include_quarantined(self.include_repair_candidates)
            .allow_cross_task(self.allow_cross_profile)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticIndexMatch {
    pub id: String,
    pub lane: SemanticIndexLane,
    pub profile: TaskProfile,
    pub tenant_scope: String,
    pub source_anchor: String,
    pub content_digest: String,
    pub score: f32,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticIndexSkip {
    pub id: String,
    pub lane: SemanticIndexLane,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticIndexRetrievalReport {
    pub requested_limit: usize,
    pub token_budget: usize,
    pub used_tokens: usize,
    pub source_digest: u64,
    pub record_count: usize,
    pub matches: Vec<SemanticIndexMatch>,
    pub skipped: Vec<SemanticIndexSkip>,
    pub redacted: bool,
    pub read_only: bool,
    pub write_allowed: bool,
}

impl SemanticIndexRetrievalReport {
    fn from_plan(
        query: &SemanticIndexQuery,
        plan: &norion_memory::MemorySemanticRetrievalPlan,
        records: &[SemanticIndexRecord],
    ) -> Self {
        let matches = plan
            .matches
            .iter()
            .map(|item| {
                let record = find_record(records, &item.id, item.source);
                SemanticIndexMatch {
                    id: item.id.clone(),
                    lane: record
                        .map(|record| record.lane)
                        .unwrap_or_else(|| lane_from_source(item.source)),
                    profile: record.map(|record| record.profile).unwrap_or(query.profile),
                    tenant_scope: record
                        .map(|record| record.tenant_scope.clone())
                        .unwrap_or_else(|| query.tenant_scope.clone()),
                    source_anchor: record
                        .map(|record| record.source_anchor.clone())
                        .unwrap_or_else(|| "missing-anchor".to_owned()),
                    content_digest: record
                        .map(|record| record.content_digest.clone())
                        .unwrap_or_else(|| stable_digest(&item.id)),
                    score: item.score,
                    estimated_tokens: item.estimated_tokens,
                }
            })
            .collect::<Vec<_>>();

        let skipped = plan
            .skipped
            .iter()
            .map(|item| {
                let record = find_record(records, &item.id, item.source);
                SemanticIndexSkip {
                    id: item.id.clone(),
                    lane: record
                        .map(|record| record.lane)
                        .unwrap_or_else(|| lane_from_source(item.source)),
                    reason: item.reason.clone(),
                }
            })
            .collect::<Vec<_>>();

        Self {
            requested_limit: query.record_limit,
            token_budget: query.token_budget,
            used_tokens: plan.used_tokens,
            source_digest: plan.source_digest,
            record_count: records.len(),
            matches,
            skipped,
            redacted: true,
            read_only: true,
            write_allowed: false,
        }
    }

    pub fn matched_ids(&self) -> Vec<String> {
        self.matches.iter().map(|item| item.id.clone()).collect()
    }

    pub fn matched_source_anchors(&self) -> Vec<String> {
        self.matches
            .iter()
            .map(|item| item.source_anchor.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn matched_gene_segment_ids(&self) -> Vec<String> {
        self.matches
            .iter()
            .filter(|item| item.lane == SemanticIndexLane::GeneSegment)
            .map(|item| item.id.clone())
            .collect()
    }

    pub fn skipped_ids_for_reason(&self, reason: &str) -> Vec<String> {
        self.skipped
            .iter()
            .filter(|item| item.reason == reason)
            .map(|item| item.id.clone())
            .collect()
    }

    pub fn lane_codes(&self) -> Vec<String> {
        self.matches
            .iter()
            .map(|item| item.lane.as_str().to_owned())
            .collect::<BTreeSet<_>>()
            .into_iter()
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

    pub fn evidence_digest(&self) -> String {
        stable_digest(&format!(
            "{}:{}:{}:{}:{}:{}",
            self.requested_limit,
            self.token_budget,
            self.used_tokens,
            self.source_digest,
            self.matched_ids().join("|"),
            self.reason_codes().join("|")
        ))
    }

    pub fn summary_line(&self) -> String {
        format!(
            "semantic_index_retrieval records={} matches={} skipped={} used_tokens={} source_digest={:016x} lanes={} reasons={} redacted={} read_only={} write_allowed={} evidence_digest={}",
            self.record_count,
            self.matches.len(),
            self.skipped.len(),
            self.used_tokens,
            self.source_digest,
            self.lane_codes().join("|"),
            self.reason_codes().join("|"),
            self.redacted,
            self.read_only,
            self.write_allowed,
            self.evidence_digest()
        )
    }
}

fn find_record<'a>(
    records: &'a [SemanticIndexRecord],
    id: &str,
    source: MemoryIndexSource,
) -> Option<&'a SemanticIndexRecord> {
    records
        .iter()
        .find(|record| record.id == id && record.document.source == source)
}

fn lane_from_source(source: MemoryIndexSource) -> SemanticIndexLane {
    match source {
        MemoryIndexSource::Experience => SemanticIndexLane::Experience,
        MemoryIndexSource::GeneSegment => SemanticIndexLane::GeneSegment,
        MemoryIndexSource::LongTerm | MemoryIndexSource::Skill | MemoryIndexSource::RuntimeKv => {
            SemanticIndexLane::Heuristic
        }
    }
}

fn memory_scope(profile: TaskProfile, tenant_scope: &str) -> MemoryScope {
    MemoryScope::for_task(profile_slug(profile)).with_agent(tenant_scope)
}

fn lane_tags(lane: SemanticIndexLane, profile: TaskProfile, tags: &[String]) -> Vec<String> {
    let mut values = vec![
        lane.as_str().to_owned(),
        format!("profile:{}", profile_slug(profile)),
    ];
    values.extend(tags.iter().map(|tag| safe_projection_text(tag, 64)));
    values
}

fn experience_content(record: &ExperienceRecord) -> String {
    let gist = record
        .gist_records
        .iter()
        .take(3)
        .map(|gist| {
            format!(
                "{} {}",
                safe_projection_text(&gist.title, 80),
                safe_projection_text(&gist.summary, 180)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let reflection_codes = record
        .reflection_issues
        .iter()
        .map(|issue| safe_projection_text(&issue.code, 64))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "experience profile={} gist={} reflection={} prompt_digest={} lesson_digest={} quality={:.3}",
        profile_slug(record.profile),
        gist,
        reflection_codes,
        stable_digest(&record.prompt),
        stable_digest(&record.lesson),
        record.quality
    )
}

fn runtime_tags(record: &ExperienceRecord) -> Vec<String> {
    [
        record.runtime_diagnostics.model_id.as_deref(),
        record.runtime_diagnostics.selected_adapter.as_deref(),
        record.runtime_diagnostics.device_profile.as_deref(),
        record.runtime_diagnostics.primary_lane.as_deref(),
        record.runtime_diagnostics.fallback_lane.as_deref(),
        record.runtime_diagnostics.memory_mode.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(|value| safe_projection_text(value, 64))
    .collect()
}

fn gene_segment_content(segment: &GeneSegment) -> String {
    format!(
        "gene_segment profile={} source={} residency={} label={} purpose={} confirmed={} gist={}",
        profile_slug(segment.profile),
        segment.source.as_str(),
        segment.kv_residency.as_str(),
        safe_projection_text(&segment.label, 96),
        safe_projection_text(&segment.purpose, 192),
        safe_projection_text(&segment.last_confirmed_purpose, 192),
        safe_projection_text(&segment.semantic_gist, 260)
    )
}

fn mock_embedding(text: &str) -> Vec<f32> {
    let mut embedding = vec![0.0_f32; MOCK_EMBEDDING_DIMS];
    for token in normalized_tokens(text) {
        let index = stable_hash(token.as_bytes()) as usize % MOCK_EMBEDDING_DIMS;
        embedding[index] += 1.0;
    }
    let norm = embedding
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if norm > 0.0 {
        for value in &mut embedding {
            *value /= norm;
        }
    }
    embedding
}

fn normalized_tokens(value: &str) -> Vec<String> {
    value
        .split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn safe_projection_text(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn join_sorted(values: &[String]) -> String {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(",")
}

fn freshness_from_age(age: u64) -> f32 {
    (1.0 / (1.0 + age as f32 / 16.0)).clamp(0.0, 1.0)
}

fn freshness_from_segment_age(age: u32) -> f32 {
    (1.0 / (1.0 + age as f32 / 8.0)).clamp(0.0, 1.0)
}

fn estimate_tokens(value: &str) -> usize {
    (value.chars().count() / 4).max(1)
}

fn format_unit(value: f32) -> String {
    format!("{:.6}", value.clamp(0.0, 1.0))
}

fn stable_digest(value: &str) -> String {
    format!("digest:{:016x}", stable_hash(value.as_bytes()))
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn profile_slug(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning_genome::{
        ClassifiedGeneSegment, GeneSegment, GeneSegmentClass, GeneSegmentDisposition,
        GeneSegmentSource,
    };
    use crate::self_evolving_memory::{
        SelfEvolvingEpisodeInput, SelfEvolvingHeuristicInput, SelfEvolvingMemoryApproval,
        ToolReliabilityObservationInput,
    };

    #[test]
    fn semantic_index_builds_lanes_and_retrieves_redacted_store_records() {
        let secret = "SECRET_PROMPT_PAYLOAD";
        let mut store = SelfEvolvingMemoryStore::new();
        let approval = approval();
        store.append_episode(
            SelfEvolvingEpisodeInput {
                problem: format!("private problem {secret}"),
                solution_path: "private solution path".to_owned(),
                outcome: "validated runtime routing outcome".to_owned(),
                key_insights: vec!["keep chunked kv anchors".to_owned()],
                tags: vec!["rust".to_owned(), "router".to_owned(), "kv".to_owned()],
                profile: TaskProfile::Coding,
                quality: 0.92,
                token_estimate: 24,
                source_case_id: "case-runtime-router".to_owned(),
            },
            &approval,
        );
        store.append_heuristic(
            SelfEvolvingHeuristicInput {
                rule: format!("do not leak {secret}"),
                tags: vec!["router".to_owned(), "threshold".to_owned()],
                profile: TaskProfile::Coding,
                priority: 0.9,
                confidence: 0.86,
                source_case_id: "case-router-heuristic".to_owned(),
                updated_step: 8,
            },
            &approval,
        );
        store.observe_tool(
            ToolReliabilityObservationInput {
                tool_name: "cargo-test".to_owned(),
                profile: TaskProfile::Coding,
                success: true,
                quality: 0.95,
                source_case_id: "case-tool".to_owned(),
                observed_step: 9,
            },
            &approval,
        );

        let index = SemanticIndex::from_self_evolving_store(&store, "tenant-a", 10);
        let report = index.retrieve(
            &SemanticIndexQuery::new(
                "rust router threshold cargo",
                TaskProfile::Coding,
                "tenant-a",
            )
            .with_record_limit(4),
        );

        assert_eq!(index.len(), 3);
        assert!(report.matches.len() >= 2);
        assert!(report.lane_codes().contains(&"episode".to_owned()));
        assert!(report.lane_codes().contains(&"heuristic".to_owned()));
        assert!(report.redacted);
        assert!(!report.summary_line().contains(secret));
        assert!(report.summary_line().contains("write_allowed=false"));
    }

    #[test]
    fn semantic_index_ranks_bounds_and_isolates_tenant_scope() {
        let approval = approval();
        let mut tenant_a = SelfEvolvingMemoryStore::new();
        tenant_a.append_episode(
            SelfEvolvingEpisodeInput {
                problem: "rust router problem".to_owned(),
                solution_path: "router fix".to_owned(),
                outcome: "good".to_owned(),
                key_insights: vec!["adaptive threshold".to_owned()],
                tags: vec!["rust".to_owned(), "router".to_owned()],
                profile: TaskProfile::Coding,
                quality: 0.95,
                token_estimate: 16,
                source_case_id: "tenant-a-case".to_owned(),
            },
            &approval,
        );
        let mut tenant_b = SelfEvolvingMemoryStore::new();
        tenant_b.append_episode(
            SelfEvolvingEpisodeInput {
                problem: "rust router other tenant".to_owned(),
                solution_path: "router fix".to_owned(),
                outcome: "good".to_owned(),
                key_insights: vec!["adaptive threshold".to_owned()],
                tags: vec!["rust".to_owned(), "router".to_owned()],
                profile: TaskProfile::Coding,
                quality: 0.99,
                token_estimate: 16,
                source_case_id: "tenant-b-case".to_owned(),
            },
            &approval,
        );

        let index = SemanticIndex::from_self_evolving_store(&tenant_a, "tenant-a", 2)
            .with_self_evolving_store(&tenant_b, "tenant-b", 2);
        let report = index.retrieve(
            &SemanticIndexQuery::new("rust router", TaskProfile::Coding, "tenant-a")
                .with_record_limit(2)
                .with_token_budget(64),
        );

        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].tenant_scope, "tenant-a");
        assert_eq!(
            report.skipped_ids_for_reason("cross_agent_scope"),
            vec!["sem:episode:1".to_owned()]
        );
        assert_eq!(report.source_digest, index.rebuild_digest());
    }

    #[test]
    fn semantic_index_applies_freshness_decay_to_ranking() {
        let fresh = retained_gene("fresh", 0, "router adapter cache repair");
        let stale = retained_gene("stale", 32, "router adapter cache repair stale");
        let mut index = SemanticIndex::new();
        index.push_record(SemanticIndexRecord::from_gene_segment(&stale));
        index.push_record(SemanticIndexRecord::from_gene_segment(&fresh));

        let report = index.retrieve(
            &SemanticIndexQuery::new(
                "router adapter cache repair",
                TaskProfile::Coding,
                "tenant-a",
            )
            .with_record_limit(2),
        );

        assert_eq!(report.matched_ids()[0], "fresh");
        assert_eq!(report.matched_ids()[1], "stale");
    }

    #[test]
    fn semantic_index_suppresses_duplicates_and_respects_token_budget() {
        let duplicate_a = retained_gene("dup-a", 0, "chunked kv semantic anchor");
        let duplicate_b = retained_gene("dup-b", 0, "chunked kv semantic anchor");
        let long_segment =
            retained_gene("long", 0, &format!("chunked kv {}", "budget ".repeat(80)));
        let mut index = SemanticIndex::new();
        index.push_record(SemanticIndexRecord::from_gene_segment(&duplicate_a));
        index.push_record(SemanticIndexRecord::from_gene_segment(&duplicate_b));
        index.push_record(SemanticIndexRecord::from_gene_segment(&long_segment));

        let report = index.retrieve(
            &SemanticIndexQuery::new(
                "chunked kv semantic anchor",
                TaskProfile::Coding,
                "tenant-a",
            )
            .with_record_limit(5)
            .with_token_budget(96),
        );

        assert!(report.matched_ids().contains(&"dup-a".to_owned()));
        assert_eq!(
            report.skipped_ids_for_reason("duplicate"),
            vec!["dup-b".to_owned()]
        );
        assert_eq!(
            report.skipped_ids_for_reason("token_budget"),
            vec!["long".to_owned()]
        );
        assert!(report.used_tokens <= 96);
    }

    #[test]
    fn semantic_index_skips_privacy_and_repair_candidates_by_default() {
        let active = retained_gene("active", 0, "splice repair safe anchor");
        let private = classified_gene(
            GeneSegment::new(
                "private",
                TaskProfile::Coding,
                GeneSegmentSource::SemanticMemory,
                0,
                16,
            )
            .with_scope("tenant-a")
            .with_source_hash("sha256:private")
            .with_metadata(
                "splice repair private anchor",
                "private memory anchor",
                "splice repair private anchor",
            )
            .with_health(0.9, 0.1, 0.95),
            GeneSegmentDisposition::Retained,
        );
        let corrupt = classified_gene(
            GeneSegment::new(
                "corrupt",
                TaskProfile::Coding,
                GeneSegmentSource::SemanticMemory,
                0,
                16,
            )
            .with_scope("tenant-a")
            .with_source_hash("sha256:corrupt")
            .with_metadata(
                "splice repair corrupt anchor",
                "repair candidate",
                "splice repair corrupt anchor",
            )
            .with_schema(false, true),
            GeneSegmentDisposition::RepairCandidate,
        );
        let mut index = SemanticIndex::new();
        index.push_record(SemanticIndexRecord::from_gene_segment(&active));
        index.push_record(SemanticIndexRecord::from_gene_segment(&private));
        index.push_record(SemanticIndexRecord::from_gene_segment(&corrupt));

        let guarded = index.retrieve(&SemanticIndexQuery::new(
            "splice repair anchor",
            TaskProfile::Coding,
            "tenant-a",
        ));

        assert_eq!(guarded.matched_ids(), vec!["active".to_owned()]);
        assert_eq!(
            guarded.skipped_ids_for_reason("privacy_blocked"),
            vec!["private".to_owned()]
        );
        assert_eq!(
            guarded.skipped_ids_for_reason("gene_segment_corrupt"),
            vec!["corrupt".to_owned()]
        );

        let repair = index.retrieve(
            &SemanticIndexQuery::new("splice repair anchor", TaskProfile::Coding, "tenant-a")
                .include_repair_candidates(true),
        );

        assert!(repair.matched_ids().contains(&"corrupt".to_owned()));
        assert!(!repair.matched_ids().contains(&"private".to_owned()));
    }

    #[test]
    fn semantic_index_rebuild_digest_is_stable_across_ordering() {
        let left = retained_gene("left", 0, "semantic index stable digest");
        let right = retained_gene("right", 1, "semantic index stable digest alternate");
        let mut first = SemanticIndex::new();
        first.push_record(SemanticIndexRecord::from_gene_segment(&left));
        first.push_record(SemanticIndexRecord::from_gene_segment(&right));

        let mut second = SemanticIndex::new();
        second.push_record(SemanticIndexRecord::from_gene_segment(&right));
        second.push_record(SemanticIndexRecord::from_gene_segment(&left));

        assert_eq!(first.rebuild_digest(), second.rebuild_digest());
        let report = first.retrieve(&SemanticIndexQuery::new(
            "semantic index stable digest",
            TaskProfile::Coding,
            "tenant-a",
        ));
        assert_eq!(report.source_digest, first.rebuild_digest());
        assert!(!report.evidence_digest().is_empty());
    }

    fn approval() -> SelfEvolvingMemoryApproval {
        SelfEvolvingMemoryApproval::approved("rollback:semantic-index", vec!["cargo:test".into()])
    }

    fn retained_gene(id: &str, age: u32, gist: &str) -> ClassifiedGeneSegment {
        classified_gene(
            GeneSegment::new(
                id,
                TaskProfile::Coding,
                GeneSegmentSource::SemanticMemory,
                0,
                24,
            )
            .with_scope("tenant-a")
            .with_source_hash(format!("sha256:{id}"))
            .with_metadata(gist, "carry semantic retrieval anchor", gist)
            .with_age(age)
            .with_health(0.9, 0.05, 0.0),
            GeneSegmentDisposition::Retained,
        )
    }

    fn classified_gene(
        segment: GeneSegment,
        disposition: GeneSegmentDisposition,
    ) -> ClassifiedGeneSegment {
        ClassifiedGeneSegment {
            segment,
            class: GeneSegmentClass::Exon,
            disposition,
            reasons: Vec::new(),
        }
    }
}
