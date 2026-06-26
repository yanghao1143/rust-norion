use crate::hierarchy::TaskProfile;
use crate::memory_admission::{
    MemoryAdmissionCandidate, MemoryAdmissionDecision, MemoryAdmissionKind, MemoryAdmissionPreview,
};
use std::collections::BTreeMap;

const SELF_EVOLVING_MEMORY_STORE_TRACE_SCHEMA: &str = "rust-norion-self-evolving-memory-store-v1";
pub const SELF_EVOLVING_MEMORY_CONSOLIDATION_SCHEMA_VERSION: &str =
    "self_evolving_memory_consolidation_v1";

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemoryApproval {
    pub operator_approved: bool,
    pub privacy_checked: bool,
    pub rollback_anchor_id: String,
    pub validation_evidence: Vec<String>,
}

impl SelfEvolvingMemoryApproval {
    pub fn approved(
        rollback_anchor_id: impl Into<String>,
        validation_evidence: Vec<String>,
    ) -> Self {
        Self {
            operator_approved: true,
            privacy_checked: true,
            rollback_anchor_id: rollback_anchor_id.into(),
            validation_evidence,
        }
    }

    fn blocked_reasons(&self) -> Vec<String> {
        let mut reasons = Vec::new();
        if !self.operator_approved {
            reasons.push("self_evolving_memory_operator_approval_missing".to_owned());
        }
        if !self.privacy_checked {
            reasons.push("self_evolving_memory_privacy_check_missing".to_owned());
        }
        if self.rollback_anchor_id.trim().is_empty() {
            reasons.push("self_evolving_memory_rollback_anchor_missing".to_owned());
        }
        if self.validation_evidence.is_empty() {
            reasons.push("self_evolving_memory_validation_evidence_missing".to_owned());
        }
        reasons
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingEpisodeInput {
    pub problem: String,
    pub solution_path: String,
    pub outcome: String,
    pub key_insights: Vec<String>,
    pub tags: Vec<String>,
    pub profile: TaskProfile,
    pub quality: f32,
    pub token_estimate: usize,
    pub source_case_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingEpisodeRecord {
    pub sequence: u64,
    pub record_id: String,
    pub problem_digest: String,
    pub solution_path_digest: String,
    pub outcome_digest: String,
    pub key_insight_digests: Vec<String>,
    pub tags: Vec<String>,
    pub profile: TaskProfile,
    pub quality: f32,
    pub token_estimate: usize,
    pub source_case_digest: String,
    pub rollback_anchor_id: String,
    pub validation_evidence_count: usize,
    pub active: bool,
    pub merged_into: Option<String>,
    pub append_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingHeuristicInput {
    pub rule: String,
    pub tags: Vec<String>,
    pub profile: TaskProfile,
    pub priority: f32,
    pub confidence: f32,
    pub source_case_id: String,
    pub updated_step: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingHeuristicRecord {
    pub sequence: u64,
    pub record_id: String,
    pub rule_digest: String,
    pub tags: Vec<String>,
    pub profile: TaskProfile,
    pub priority: f32,
    pub confidence: f32,
    pub source_case_digest: String,
    pub last_updated_step: u64,
    pub support_count: usize,
    pub decay_count: usize,
    pub quarantined: bool,
    pub quarantine_reason: Option<String>,
    pub rollback_anchor_id: String,
    pub validation_evidence_count: usize,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolReliabilityObservationInput {
    pub tool_name: String,
    pub profile: TaskProfile,
    pub success: bool,
    pub quality: f32,
    pub source_case_id: String,
    pub observed_step: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolReliabilityObservationRecord {
    pub sequence: u64,
    pub tool_id: String,
    pub tool_digest: String,
    pub profile: TaskProfile,
    pub success: bool,
    pub quality: f32,
    pub source_case_digest: String,
    pub observed_step: u64,
    pub rollback_anchor_id: String,
    pub validation_evidence_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolReliabilityRecord {
    pub tool_id: String,
    pub tool_digest: String,
    pub profile: TaskProfile,
    pub observations: usize,
    pub successes: usize,
    pub success_rate: f32,
    pub avg_quality: f32,
    pub trust_score: f32,
    pub last_used_step: u64,
    pub decay_count: usize,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemoryWriteReport {
    pub accepted: bool,
    pub lane: String,
    pub record_id: Option<String>,
    pub blocked_reasons: Vec<String>,
    pub content_digest: String,
}

impl SelfEvolvingMemoryWriteReport {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolving_memory_write accepted={} lane={} record_id={} blocked_reasons={} digest={}",
            self.accepted,
            self.lane,
            self.record_id.as_deref().unwrap_or("none"),
            self.blocked_reasons.len(),
            self.content_digest
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemoryQuery {
    pub prompt: String,
    pub profile: TaskProfile,
    pub tags: Vec<String>,
    pub record_limit: usize,
    pub token_budget: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingEpisodeContext {
    pub record_id: String,
    pub problem_digest: String,
    pub solution_path_digest: String,
    pub outcome_digest: String,
    pub key_insight_count: usize,
    pub source_case_digest: String,
    pub score: f32,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingHeuristicContext {
    pub record_id: String,
    pub rule_digest: String,
    pub source_case_digest: String,
    pub priority: f32,
    pub confidence: f32,
    pub score: f32,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolReliabilityContext {
    pub tool_id: String,
    pub tool_digest: String,
    pub profile: TaskProfile,
    pub observations: usize,
    pub success_rate: f32,
    pub avg_quality: f32,
    pub trust_score: f32,
    pub score: f32,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemoryRetrievalReport {
    pub requested_limit: usize,
    pub token_budget: usize,
    pub retained_tokens: usize,
    pub skipped_by_budget: usize,
    pub skipped_cross_profile: usize,
    pub episodes: Vec<SelfEvolvingEpisodeContext>,
    pub heuristics: Vec<SelfEvolvingHeuristicContext>,
    pub tool_reliability: Vec<ToolReliabilityContext>,
    pub redacted: bool,
}

impl SelfEvolvingMemoryRetrievalReport {
    pub fn total_contexts(&self) -> usize {
        self.episodes
            .len()
            .saturating_add(self.heuristics.len())
            .saturating_add(self.tool_reliability.len())
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_evolving_memory_retrieval contexts={} episodes={} heuristics={} tools={} retained_tokens={} skipped_by_budget={} skipped_cross_profile={} redacted={}",
            self.total_contexts(),
            self.episodes.len(),
            self.heuristics.len(),
            self.tool_reliability.len(),
            self.retained_tokens,
            self.skipped_by_budget,
            self.skipped_cross_profile,
            self.redacted
        )
    }

    pub fn json_line(&self) -> String {
        let evidence_digest = stable_digest(&format!(
            "retrieval:{}:{}:{}:{}:{}:{}:{}:{}",
            self.requested_limit,
            self.token_budget,
            self.retained_tokens,
            self.skipped_by_budget,
            self.skipped_cross_profile,
            self.episodes.len(),
            self.heuristics.len(),
            self.tool_reliability.len()
        ));
        format!(
            "{{\"schema\":\"{}\",\"operation\":\"retrieval\",\"contexts\":{},\"episodes\":{},\"heuristics\":{},\"tools\":{},\"requested_limit\":{},\"token_budget\":{},\"retained_tokens\":{},\"skipped_by_budget\":{},\"skipped_cross_profile\":{},\"redacted\":{},\"report_only\":true,\"read_only\":true,\"write_allowed\":false,\"durable_write_allowed\":false,\"applied\":false,\"applied_to_disk\":false,\"evidence_digest\":\"{}\"}}",
            SELF_EVOLVING_MEMORY_STORE_TRACE_SCHEMA,
            self.total_contexts(),
            self.episodes.len(),
            self.heuristics.len(),
            self.tool_reliability.len(),
            self.requested_limit,
            self.token_budget,
            self.retained_tokens,
            self.skipped_by_budget,
            self.skipped_cross_profile,
            self.redacted,
            evidence_digest
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemoryMaintenancePolicy {
    pub current_step: u64,
    pub stale_after_steps: u64,
    pub heuristic_decay: f32,
    pub tool_reliability_decay: f32,
    pub quarantine_below_confidence: f32,
    pub merge_duplicate_episodes: bool,
}

impl Default for SelfEvolvingMemoryMaintenancePolicy {
    fn default() -> Self {
        Self {
            current_step: 0,
            stale_after_steps: 10,
            heuristic_decay: 0.90,
            tool_reliability_decay: 0.95,
            quarantine_below_confidence: 0.20,
            merge_duplicate_episodes: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolvingMemoryMaintenanceReport {
    pub decayed_heuristics: usize,
    pub decayed_tool_reliability: usize,
    pub quarantined_heuristics: usize,
    pub merged_duplicate_episodes: usize,
    pub read_only: bool,
    pub durable_write_allowed: bool,
    pub applied_to_disk: bool,
}

impl SelfEvolvingMemoryMaintenanceReport {
    pub fn action_count(&self) -> usize {
        self.decayed_heuristics
            .saturating_add(self.decayed_tool_reliability)
            .saturating_add(self.quarantined_heuristics)
            .saturating_add(self.merged_duplicate_episodes)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_evolving_memory_maintenance decayed_heuristics={} decayed_tool_reliability={} quarantined_heuristics={} merged_duplicate_episodes={} read_only={} durable_write_allowed={} applied_to_disk={}",
            self.decayed_heuristics,
            self.decayed_tool_reliability,
            self.quarantined_heuristics,
            self.merged_duplicate_episodes,
            self.read_only,
            self.durable_write_allowed,
            self.applied_to_disk
        )
    }

    pub fn json_line(&self) -> String {
        let evidence_digest = stable_digest(&format!(
            "maintenance:{}:{}:{}:{}:{}:{}:{}",
            self.decayed_heuristics,
            self.decayed_tool_reliability,
            self.quarantined_heuristics,
            self.merged_duplicate_episodes,
            self.read_only,
            self.durable_write_allowed,
            self.applied_to_disk
        ));
        format!(
            "{{\"schema\":\"{}\",\"operation\":\"maintenance\",\"maintenance_actions\":{},\"decayed_heuristics\":{},\"decayed_tool_reliability\":{},\"quarantined_heuristics\":{},\"merged_duplicate_episodes\":{},\"redacted\":true,\"report_only\":true,\"read_only\":{},\"write_allowed\":false,\"durable_write_allowed\":{},\"applied\":false,\"applied_to_disk\":{},\"evidence_digest\":\"{}\"}}",
            SELF_EVOLVING_MEMORY_STORE_TRACE_SCHEMA,
            self.action_count(),
            self.decayed_heuristics,
            self.decayed_tool_reliability,
            self.quarantined_heuristics,
            self.merged_duplicate_episodes,
            self.read_only,
            self.durable_write_allowed,
            self.applied_to_disk,
            evidence_digest
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryConsolidationEvidenceClass {
    RetrospectiveEpisode,
    ProceduralHeuristic,
    ToolReliabilityObservation,
    GeneSegmentAnchor,
}

impl MemoryConsolidationEvidenceClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RetrospectiveEpisode => "retrospective_episode",
            Self::ProceduralHeuristic => "procedural_heuristic",
            Self::ToolReliabilityObservation => "tool_reliability_observation",
            Self::GeneSegmentAnchor => "gene_segment_anchor",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryConsolidationRecord {
    pub record_id: String,
    pub tenant_scope: String,
    pub evidence_class: MemoryConsolidationEvidenceClass,
    pub source_digest: String,
    pub content_digest: String,
    pub profile: TaskProfile,
    pub confidence: f32,
    pub quality: f32,
    pub last_touched_step: u64,
    pub token_estimate: usize,
    pub rollback_anchor_id: String,
    pub validation_evidence_count: usize,
    pub protected: bool,
}

impl MemoryConsolidationRecord {
    pub fn new(
        record_id: impl Into<String>,
        tenant_scope: impl Into<String>,
        evidence_class: MemoryConsolidationEvidenceClass,
        source_digest: impl Into<String>,
        content_digest: impl Into<String>,
        profile: TaskProfile,
    ) -> Self {
        let record_id = sanitize_identifier(&record_id.into(), "memory-record");
        Self {
            rollback_anchor_id: format!("rollback:{record_id}"),
            record_id,
            tenant_scope: sanitize_identifier(&tenant_scope.into(), "tenant:local"),
            evidence_class,
            source_digest: digest_or_stable(&source_digest.into()),
            content_digest: digest_or_stable(&content_digest.into()),
            profile,
            confidence: 0.50,
            quality: 0.50,
            last_touched_step: 0,
            token_estimate: 1,
            validation_evidence_count: 0,
            protected: false,
        }
    }

    pub fn with_scores(mut self, confidence: f32, quality: f32) -> Self {
        self.confidence = clamp_unit(confidence);
        self.quality = clamp_unit(quality);
        self
    }

    pub fn with_last_touched_step(mut self, last_touched_step: u64) -> Self {
        self.last_touched_step = last_touched_step;
        self
    }

    pub fn with_token_estimate(mut self, token_estimate: usize) -> Self {
        self.token_estimate = token_estimate.max(1);
        self
    }

    pub fn with_rollback_anchor(mut self, rollback_anchor_id: impl Into<String>) -> Self {
        let rollback_anchor_id = sanitize_identifier(&rollback_anchor_id.into(), "rollback");
        if !rollback_anchor_id.trim().is_empty() {
            self.rollback_anchor_id = rollback_anchor_id;
        }
        self
    }

    pub fn with_validation_evidence_count(mut self, validation_evidence_count: usize) -> Self {
        self.validation_evidence_count = validation_evidence_count;
        self
    }

    pub fn with_protected(mut self, protected: bool) -> Self {
        self.protected = protected;
        self
    }

    pub fn record_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{:?}\t{:.3}\t{:.3}\t{}\t{}\t{}\t{}\t{}",
            self.record_id,
            self.tenant_scope,
            self.evidence_class.as_str(),
            self.source_digest,
            self.content_digest,
            self.profile,
            self.confidence,
            self.quality,
            self.last_touched_step,
            self.token_estimate,
            self.rollback_anchor_id,
            self.validation_evidence_count,
            self.protected
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryConsolidationDecisionKind {
    Keep,
    MergePreview,
    DecayPreview,
    TombstonePreview,
    MergeRejected,
}

impl MemoryConsolidationDecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::MergePreview => "merge_preview",
            Self::DecayPreview => "decay_preview",
            Self::TombstonePreview => "tombstone_preview",
            Self::MergeRejected => "merge_rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryConsolidationDecision {
    pub record_id: String,
    pub decision: MemoryConsolidationDecisionKind,
    pub evidence_class: MemoryConsolidationEvidenceClass,
    pub tenant_scope: String,
    pub source_digest: String,
    pub content_digest: String,
    pub primary_record_id: Option<String>,
    pub compacted_summary_digest: String,
    pub reason_codes: Vec<String>,
    pub rollback_anchor_id: String,
    pub tombstone_id: Option<String>,
    pub confidence_before: f32,
    pub confidence_after: f32,
    pub retained_tokens: usize,
    pub saved_tokens: usize,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl MemoryConsolidationDecision {
    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn record_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{}\t{}\t{}\t{}\t{}",
            self.record_id,
            self.decision.as_str(),
            self.evidence_class.as_str(),
            self.tenant_scope,
            self.source_digest,
            self.content_digest,
            self.primary_record_id.as_deref().unwrap_or("none"),
            self.compacted_summary_digest,
            self.confidence_before,
            self.confidence_after,
            self.retained_tokens,
            self.saved_tokens,
            self.rollback_anchor_id,
            self.tombstone_id.as_deref().unwrap_or("none"),
            self.reason_codes.join("|")
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_consolidation_decision id={} decision={} class={} tenant={} source={} content={} primary={} summary={} confidence={:.3}->{:.3} retained={} saved={} rollback={} tombstone={} reasons={} read_only={} write_allowed={} applied={}",
            self.record_id,
            self.decision.as_str(),
            self.evidence_class.as_str(),
            self.tenant_scope,
            self.source_digest,
            self.content_digest,
            self.primary_record_id.as_deref().unwrap_or("none"),
            self.compacted_summary_digest,
            self.confidence_before,
            self.confidence_after,
            self.retained_tokens,
            self.saved_tokens,
            self.rollback_anchor_id,
            self.tombstone_id.as_deref().unwrap_or("none"),
            self.reason_codes.join("|"),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelfEvolvingMemoryConsolidationPolicy {
    pub current_step: u64,
    pub stale_after_steps: u64,
    pub decay_factor: f32,
    pub tombstone_below_confidence: f32,
    pub tombstone_below_quality: f32,
    pub merge_duplicate_records: bool,
}

impl Default for SelfEvolvingMemoryConsolidationPolicy {
    fn default() -> Self {
        Self {
            current_step: 0,
            stale_after_steps: 10,
            decay_factor: 0.90,
            tombstone_below_confidence: 0.18,
            tombstone_below_quality: 0.15,
            merge_duplicate_records: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolvingMemoryConsolidationMetrics {
    pub records_before: usize,
    pub records_after_preview: usize,
    pub token_estimate_before: usize,
    pub token_estimate_after_preview: usize,
    pub retrieval_precision_before_milli: i64,
    pub retrieval_precision_after_milli: i64,
    pub retrieval_precision_delta_milli: i64,
    pub replay_safety_milli: i64,
    pub benchmark_impact_milli: i64,
}

impl SelfEvolvingMemoryConsolidationMetrics {
    pub fn summary_line(&self) -> String {
        format!(
            "memory_consolidation_metrics records_before={} records_after_preview={} token_estimate_before={} token_estimate_after_preview={} retrieval_precision_before_milli={} retrieval_precision_after_milli={} retrieval_precision_delta_milli={} replay_safety_milli={} benchmark_impact_milli={}",
            self.records_before,
            self.records_after_preview,
            self.token_estimate_before,
            self.token_estimate_after_preview,
            self.retrieval_precision_before_milli,
            self.retrieval_precision_after_milli,
            self.retrieval_precision_delta_milli,
            self.replay_safety_milli,
            self.benchmark_impact_milli
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemoryConsolidationReport {
    pub schema_version: &'static str,
    pub snapshot_digest: String,
    pub plan_digest: String,
    pub decisions: Vec<MemoryConsolidationDecision>,
    pub metrics: SelfEvolvingMemoryConsolidationMetrics,
    pub read_only: bool,
    pub write_allowed: bool,
    pub durable_write_allowed: bool,
    pub applied: bool,
    pub applied_to_disk: bool,
}

impl SelfEvolvingMemoryConsolidationReport {
    pub fn merge_count(&self) -> usize {
        self.count_decision(MemoryConsolidationDecisionKind::MergePreview)
    }

    pub fn decay_count(&self) -> usize {
        self.count_decision(MemoryConsolidationDecisionKind::DecayPreview)
    }

    pub fn tombstone_count(&self) -> usize {
        self.count_decision(MemoryConsolidationDecisionKind::TombstonePreview)
    }

    pub fn merge_rejected_count(&self) -> usize {
        self.count_decision(MemoryConsolidationDecisionKind::MergeRejected)
    }

    pub fn count_decision(&self, decision: MemoryConsolidationDecisionKind) -> usize {
        self.decisions
            .iter()
            .filter(|item| item.decision == decision)
            .count()
    }

    pub fn action_count(&self) -> usize {
        self.merge_count()
            .saturating_add(self.decay_count())
            .saturating_add(self.tombstone_count())
            .saturating_add(self.merge_rejected_count())
    }

    pub fn record_lines(&self) -> Vec<String> {
        self.decisions
            .iter()
            .map(MemoryConsolidationDecision::record_line)
            .collect()
    }

    pub fn replay_matches(&self, other: &Self) -> bool {
        self.snapshot_digest == other.snapshot_digest
            && self.plan_digest == other.plan_digest
            && self.record_lines() == other.record_lines()
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.durable_write_allowed
            && !self.applied
            && !self.applied_to_disk
            && self
                .decisions
                .iter()
                .all(MemoryConsolidationDecision::is_preview_only)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_evolving_memory_consolidation schema={} snapshot={} plan={} decisions={} actions={} merges={} decays={} tombstones={} merge_rejected={} {} read_only={} write_allowed={} durable_write_allowed={} applied={} applied_to_disk={}",
            self.schema_version,
            self.snapshot_digest,
            self.plan_digest,
            self.decisions.len(),
            self.action_count(),
            self.merge_count(),
            self.decay_count(),
            self.tombstone_count(),
            self.merge_rejected_count(),
            self.metrics.summary_line(),
            self.read_only,
            self.write_allowed,
            self.durable_write_allowed,
            self.applied,
            self.applied_to_disk
        )
    }

    pub fn json_line(&self) -> String {
        let evidence_digest = stable_digest(&format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.snapshot_digest,
            self.plan_digest,
            self.decisions.len(),
            self.action_count(),
            self.merge_count(),
            self.decay_count(),
            self.tombstone_count(),
            self.metrics.benchmark_impact_milli
        ));
        format!(
            "{{\"schema\":\"{}\",\"operation\":\"consolidation_preview\",\"consolidation_actions\":{},\"records_before\":{},\"records_after_preview\":{},\"token_estimate_before\":{},\"token_estimate_after_preview\":{},\"merge_previews\":{},\"decay_previews\":{},\"tombstone_previews\":{},\"merge_rejections\":{},\"retrieval_precision_before_milli\":{},\"retrieval_precision_after_milli\":{},\"retrieval_precision_delta_milli\":{},\"replay_safety_milli\":{},\"benchmark_impact_milli\":{},\"snapshot_digest\":\"{}\",\"plan_digest\":\"{}\",\"redacted\":true,\"report_only\":true,\"read_only\":{},\"write_allowed\":false,\"durable_write_allowed\":false,\"applied\":false,\"applied_to_disk\":false,\"evidence_digest\":\"{}\"}}",
            SELF_EVOLVING_MEMORY_STORE_TRACE_SCHEMA,
            self.action_count(),
            self.metrics.records_before,
            self.metrics.records_after_preview,
            self.metrics.token_estimate_before,
            self.metrics.token_estimate_after_preview,
            self.merge_count(),
            self.decay_count(),
            self.tombstone_count(),
            self.merge_rejected_count(),
            self.metrics.retrieval_precision_before_milli,
            self.metrics.retrieval_precision_after_milli,
            self.metrics.retrieval_precision_delta_milli,
            self.metrics.replay_safety_milli,
            self.metrics.benchmark_impact_milli,
            self.snapshot_digest,
            self.plan_digest,
            self.read_only,
            evidence_digest
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemoryConsolidationWorker {
    pub policy: SelfEvolvingMemoryConsolidationPolicy,
}

impl Default for SelfEvolvingMemoryConsolidationWorker {
    fn default() -> Self {
        Self::new(SelfEvolvingMemoryConsolidationPolicy::default())
    }
}

impl SelfEvolvingMemoryConsolidationWorker {
    pub fn new(policy: SelfEvolvingMemoryConsolidationPolicy) -> Self {
        Self { policy }
    }

    pub fn plan(
        &self,
        records: &[MemoryConsolidationRecord],
    ) -> SelfEvolvingMemoryConsolidationReport {
        let mut records = records.to_vec();
        records.sort_by(|left, right| left.record_line().cmp(&right.record_line()));
        let snapshot_digest = stable_digest(
            &records
                .iter()
                .map(MemoryConsolidationRecord::record_line)
                .collect::<Vec<_>>()
                .join("\n"),
        );

        let mut decisions_by_id = BTreeMap::<String, MemoryConsolidationDecision>::new();
        for record in &records {
            decisions_by_id.insert(record.record_id.clone(), keep_decision(record));
        }

        if self.policy.merge_duplicate_records {
            for (primary_id, duplicate_ids) in compatible_duplicate_groups(&records) {
                for duplicate_id in duplicate_ids {
                    let Some(duplicate) = records
                        .iter()
                        .find(|record| record.record_id == duplicate_id)
                    else {
                        continue;
                    };
                    decisions_by_id.insert(
                        duplicate.record_id.clone(),
                        merge_decision(duplicate, &primary_id),
                    );
                }
            }
        }

        let mut rejected_cross_tenant = Vec::new();
        if self.policy.merge_duplicate_records {
            rejected_cross_tenant = cross_tenant_merge_rejections(&records);
        }

        for record in &records {
            if decisions_by_id
                .get(&record.record_id)
                .is_some_and(|decision| {
                    decision.decision == MemoryConsolidationDecisionKind::MergePreview
                })
            {
                continue;
            }
            let aged = self
                .policy
                .current_step
                .saturating_sub(record.last_touched_step)
                >= self.policy.stale_after_steps;
            if record.protected {
                if aged {
                    if let Some(decision) = decisions_by_id.get_mut(&record.record_id) {
                        push_unique_reason(&mut decision.reason_codes, "protected_rollback_anchor");
                    }
                }
                continue;
            }

            let confidence_after = if aged {
                clamp_unit(record.confidence * clamp_unit(self.policy.decay_factor))
            } else {
                record.confidence
            };
            let low_confidence = confidence_after < self.policy.tombstone_below_confidence;
            let low_quality = record.quality < self.policy.tombstone_below_quality;

            if low_confidence || low_quality {
                decisions_by_id.insert(
                    record.record_id.clone(),
                    tombstone_decision(record, confidence_after, aged, low_confidence, low_quality),
                );
            } else if aged && confidence_after < record.confidence {
                decisions_by_id.insert(
                    record.record_id.clone(),
                    decay_decision(record, confidence_after),
                );
            }
        }

        let mut decisions = decisions_by_id.into_values().collect::<Vec<_>>();
        decisions.extend(rejected_cross_tenant);
        decisions.sort_by(|left, right| left.record_line().cmp(&right.record_line()));

        let metrics = consolidation_metrics(&records, &decisions);
        let plan_digest = stable_digest(
            &decisions
                .iter()
                .map(MemoryConsolidationDecision::record_line)
                .collect::<Vec<_>>()
                .join("\n"),
        );

        SelfEvolvingMemoryConsolidationReport {
            schema_version: SELF_EVOLVING_MEMORY_CONSOLIDATION_SCHEMA_VERSION,
            snapshot_digest,
            plan_digest,
            decisions,
            metrics,
            read_only: true,
            write_allowed: false,
            durable_write_allowed: false,
            applied: false,
            applied_to_disk: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolvingMemoryAdmissionCandidatePreview {
    pub candidate_id: String,
    pub kind: MemoryAdmissionKind,
    pub source_hash: String,
    pub rollback_anchor_id: String,
    pub validation_evidence_count: usize,
    pub eligible_for_store: bool,
    pub blocked_reasons: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolvingMemoryAdmissionPreview {
    pub candidates: Vec<SelfEvolvingMemoryAdmissionCandidatePreview>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfEvolvingMemoryAdmissionPreview {
    pub fn eligible_count(&self) -> usize {
        self.candidates
            .iter()
            .filter(|candidate| candidate.eligible_for_store)
            .count()
    }

    pub fn blocked_count(&self) -> usize {
        self.candidates.len().saturating_sub(self.eligible_count())
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_evolving_memory_admission_preview candidates={} eligible={} blocked={} read_only={} write_allowed={} applied={}",
            self.candidates.len(),
            self.eligible_count(),
            self.blocked_count(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }

    pub fn blocked_reasons_count(&self) -> usize {
        self.candidates
            .iter()
            .map(|candidate| candidate.blocked_reasons.len())
            .sum()
    }

    pub fn json_line(&self) -> String {
        let evidence_digest = stable_digest(&format!(
            "admission_preview:{}:{}:{}:{}:{}:{}",
            self.candidates.len(),
            self.eligible_count(),
            self.blocked_count(),
            self.blocked_reasons_count(),
            self.write_allowed,
            self.applied
        ));
        format!(
            "{{\"schema\":\"{}\",\"operation\":\"admission_preview\",\"candidates\":{},\"eligible\":{},\"blocked\":{},\"blocked_reasons\":{},\"redacted\":true,\"report_only\":true,\"read_only\":{},\"write_allowed\":{},\"durable_write_allowed\":false,\"applied\":{},\"applied_to_disk\":false,\"evidence_digest\":\"{}\"}}",
            SELF_EVOLVING_MEMORY_STORE_TRACE_SCHEMA,
            self.candidates.len(),
            self.eligible_count(),
            self.blocked_count(),
            self.blocked_reasons_count(),
            self.read_only,
            self.write_allowed,
            self.applied,
            evidence_digest
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SelfEvolvingMemoryStore {
    episodes: Vec<SelfEvolvingEpisodeRecord>,
    heuristics: Vec<SelfEvolvingHeuristicRecord>,
    tool_reliability: Vec<ToolReliabilityRecord>,
    tool_observations: Vec<ToolReliabilityObservationRecord>,
    next_sequence: u64,
}

impl SelfEvolvingMemoryStore {
    pub fn new() -> Self {
        Self {
            next_sequence: 1,
            ..Self::default()
        }
    }

    pub fn episodes(&self) -> &[SelfEvolvingEpisodeRecord] {
        &self.episodes
    }

    pub fn heuristics(&self) -> &[SelfEvolvingHeuristicRecord] {
        &self.heuristics
    }

    pub fn tool_reliability(&self) -> &[ToolReliabilityRecord] {
        &self.tool_reliability
    }

    pub fn tool_observations(&self) -> &[ToolReliabilityObservationRecord] {
        &self.tool_observations
    }

    pub fn append_episode(
        &mut self,
        input: SelfEvolvingEpisodeInput,
        approval: &SelfEvolvingMemoryApproval,
    ) -> SelfEvolvingMemoryWriteReport {
        let blocked_reasons = approval.blocked_reasons();
        if !blocked_reasons.is_empty() {
            return blocked_write_report("episode", blocked_reasons);
        }

        let sequence = self.next_sequence();
        let record_id = format!("sem:episode:{sequence}");
        let record = SelfEvolvingEpisodeRecord {
            sequence,
            record_id: record_id.clone(),
            problem_digest: stable_digest(&input.problem),
            solution_path_digest: stable_digest(&input.solution_path),
            outcome_digest: stable_digest(&input.outcome),
            key_insight_digests: input
                .key_insights
                .iter()
                .map(|insight| stable_digest(insight))
                .collect(),
            tags: sanitize_tags(&input.tags),
            profile: input.profile,
            quality: clamp_unit(input.quality),
            token_estimate: input.token_estimate.max(1),
            source_case_digest: stable_digest(&input.source_case_id),
            rollback_anchor_id: approval.rollback_anchor_id.clone(),
            validation_evidence_count: approval.validation_evidence.len(),
            active: true,
            merged_into: None,
            append_only: true,
        };
        let content_digest = stable_digest(&format!(
            "{}:{}:{}",
            record.record_id, record.problem_digest, record.outcome_digest
        ));
        self.episodes.push(record);

        accepted_write_report("episode", record_id, content_digest)
    }

    pub fn append_heuristic(
        &mut self,
        input: SelfEvolvingHeuristicInput,
        approval: &SelfEvolvingMemoryApproval,
    ) -> SelfEvolvingMemoryWriteReport {
        let blocked_reasons = approval.blocked_reasons();
        if !blocked_reasons.is_empty() {
            return blocked_write_report("heuristic", blocked_reasons);
        }

        let sequence = self.next_sequence();
        let record_id = format!("sem:heuristic:{sequence}");
        let record = SelfEvolvingHeuristicRecord {
            sequence,
            record_id: record_id.clone(),
            rule_digest: stable_digest(&input.rule),
            tags: sanitize_tags(&input.tags),
            profile: input.profile,
            priority: clamp_unit(input.priority),
            confidence: clamp_unit(input.confidence),
            source_case_digest: stable_digest(&input.source_case_id),
            last_updated_step: input.updated_step,
            support_count: 1,
            decay_count: 0,
            quarantined: false,
            quarantine_reason: None,
            rollback_anchor_id: approval.rollback_anchor_id.clone(),
            validation_evidence_count: approval.validation_evidence.len(),
            version: 1,
        };
        let content_digest = stable_digest(&format!(
            "{}:{}:{}",
            record.record_id, record.rule_digest, record.confidence
        ));
        self.heuristics.push(record);

        accepted_write_report("heuristic", record_id, content_digest)
    }

    pub fn observe_tool(
        &mut self,
        input: ToolReliabilityObservationInput,
        approval: &SelfEvolvingMemoryApproval,
    ) -> SelfEvolvingMemoryWriteReport {
        let blocked_reasons = approval.blocked_reasons();
        if !blocked_reasons.is_empty() {
            return blocked_write_report("tool_reliability", blocked_reasons);
        }

        let sequence = self.next_sequence();
        let tool_id = sanitize_identifier(&input.tool_name, "tool");
        let tool_digest = stable_digest(&tool_id);
        let quality = clamp_unit(input.quality);
        self.tool_observations
            .push(ToolReliabilityObservationRecord {
                sequence,
                tool_id: tool_id.clone(),
                tool_digest: tool_digest.clone(),
                profile: input.profile,
                success: input.success,
                quality,
                source_case_digest: stable_digest(&input.source_case_id),
                observed_step: input.observed_step,
                rollback_anchor_id: approval.rollback_anchor_id.clone(),
                validation_evidence_count: approval.validation_evidence.len(),
            });

        match self
            .tool_reliability
            .iter_mut()
            .find(|record| record.tool_id == tool_id && record.profile == input.profile)
        {
            Some(record) => {
                let old_observations = record.observations as f32;
                record.observations = record.observations.saturating_add(1);
                record.successes = record.successes.saturating_add(usize::from(input.success));
                record.success_rate = record.successes as f32 / record.observations as f32;
                record.avg_quality = ((record.avg_quality * old_observations) + quality)
                    / record.observations as f32;
                record.trust_score = trust_score(record.success_rate, record.avg_quality);
                record.last_used_step = input.observed_step;
                record.version = record.version.saturating_add(1);
            }
            None => {
                let success_rate = if input.success { 1.0 } else { 0.0 };
                self.tool_reliability.push(ToolReliabilityRecord {
                    tool_id: tool_id.clone(),
                    tool_digest: tool_digest.clone(),
                    profile: input.profile,
                    observations: 1,
                    successes: usize::from(input.success),
                    success_rate,
                    avg_quality: quality,
                    trust_score: trust_score(success_rate, quality),
                    last_used_step: input.observed_step,
                    decay_count: 0,
                    version: 1,
                });
            }
        }

        accepted_write_report(
            "tool_reliability",
            format!("sem:tool-observation:{sequence}"),
            stable_digest(&format!("{tool_digest}:{sequence}:{quality:.3}")),
        )
    }

    pub fn retrieve_context(
        &self,
        query: &SelfEvolvingMemoryQuery,
    ) -> SelfEvolvingMemoryRetrievalReport {
        let mut retained_tokens = 0usize;
        let mut skipped_by_budget = 0usize;
        let mut skipped_cross_profile = 0usize;
        let record_limit = query.record_limit.max(1);
        let token_budget = query.token_budget.max(1);
        let query_tags = sanitize_tags(&query.tags);
        let query_tokens = query_tokens(&query.prompt);

        let mut episodes = self
            .episodes
            .iter()
            .filter(|record| record.active)
            .filter_map(|record| {
                if record.profile != query.profile {
                    skipped_cross_profile = skipped_cross_profile.saturating_add(1);
                    return None;
                }
                Some((
                    retrieval_score(record.quality, &record.tags, &query_tags, &query_tokens),
                    record,
                ))
            })
            .collect::<Vec<_>>();
        episodes.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut episode_contexts = Vec::new();
        for (score, record) in episodes {
            if episode_contexts.len() >= record_limit {
                break;
            }
            if retained_tokens.saturating_add(record.token_estimate) > token_budget {
                skipped_by_budget = skipped_by_budget.saturating_add(1);
                continue;
            }
            retained_tokens = retained_tokens.saturating_add(record.token_estimate);
            episode_contexts.push(SelfEvolvingEpisodeContext {
                record_id: record.record_id.clone(),
                problem_digest: record.problem_digest.clone(),
                solution_path_digest: record.solution_path_digest.clone(),
                outcome_digest: record.outcome_digest.clone(),
                key_insight_count: record.key_insight_digests.len(),
                source_case_digest: record.source_case_digest.clone(),
                score,
                token_estimate: record.token_estimate,
            });
        }

        let mut heuristics = self
            .heuristics
            .iter()
            .filter(|record| !record.quarantined)
            .filter_map(|record| {
                if record.profile != query.profile {
                    skipped_cross_profile = skipped_cross_profile.saturating_add(1);
                    return None;
                }
                let score = ((record.priority + record.confidence) * 0.5)
                    + tag_overlap_score(&record.tags, &query_tags, &query_tokens) * 0.25;
                Some((score.clamp(0.0, 1.0), record))
            })
            .collect::<Vec<_>>();
        heuristics.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut heuristic_contexts = Vec::new();
        for (score, record) in heuristics {
            if episode_contexts
                .len()
                .saturating_add(heuristic_contexts.len())
                >= record_limit
            {
                break;
            }
            let token_estimate = 32usize;
            if retained_tokens.saturating_add(token_estimate) > token_budget {
                skipped_by_budget = skipped_by_budget.saturating_add(1);
                continue;
            }
            retained_tokens = retained_tokens.saturating_add(token_estimate);
            heuristic_contexts.push(SelfEvolvingHeuristicContext {
                record_id: record.record_id.clone(),
                rule_digest: record.rule_digest.clone(),
                source_case_digest: record.source_case_digest.clone(),
                priority: record.priority,
                confidence: record.confidence,
                score,
                token_estimate,
            });
        }

        let mut tools = self
            .tool_reliability
            .iter()
            .filter_map(|record| {
                if record.profile != query.profile {
                    skipped_cross_profile = skipped_cross_profile.saturating_add(1);
                    return None;
                }
                Some((record.trust_score, record))
            })
            .collect::<Vec<_>>();
        tools.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut tool_contexts = Vec::new();
        for (score, record) in tools {
            if episode_contexts
                .len()
                .saturating_add(heuristic_contexts.len())
                .saturating_add(tool_contexts.len())
                >= record_limit
            {
                break;
            }
            let token_estimate = 16usize;
            if retained_tokens.saturating_add(token_estimate) > token_budget {
                skipped_by_budget = skipped_by_budget.saturating_add(1);
                continue;
            }
            retained_tokens = retained_tokens.saturating_add(token_estimate);
            tool_contexts.push(ToolReliabilityContext {
                tool_id: record.tool_id.clone(),
                tool_digest: record.tool_digest.clone(),
                profile: record.profile,
                observations: record.observations,
                success_rate: record.success_rate,
                avg_quality: record.avg_quality,
                trust_score: record.trust_score,
                score,
                token_estimate,
            });
        }

        SelfEvolvingMemoryRetrievalReport {
            requested_limit: record_limit,
            token_budget,
            retained_tokens,
            skipped_by_budget,
            skipped_cross_profile,
            episodes: episode_contexts,
            heuristics: heuristic_contexts,
            tool_reliability: tool_contexts,
            redacted: true,
        }
    }

    pub fn maintain(
        &mut self,
        policy: &SelfEvolvingMemoryMaintenancePolicy,
    ) -> SelfEvolvingMemoryMaintenanceReport {
        let mut decayed_heuristics = 0usize;
        let mut decayed_tool_reliability = 0usize;
        let mut quarantined_heuristics = 0usize;
        let decay = clamp_unit(policy.heuristic_decay);
        let tool_decay = clamp_unit(policy.tool_reliability_decay);
        for heuristic in &mut self.heuristics {
            if heuristic.quarantined {
                continue;
            }
            let age = policy
                .current_step
                .saturating_sub(heuristic.last_updated_step);
            if age >= policy.stale_after_steps {
                heuristic.confidence = (heuristic.confidence * decay).clamp(0.0, 1.0);
                heuristic.decay_count = heuristic.decay_count.saturating_add(1);
                heuristic.version = heuristic.version.saturating_add(1);
                decayed_heuristics = decayed_heuristics.saturating_add(1);
            }
            if heuristic.confidence < policy.quarantine_below_confidence {
                heuristic.quarantined = true;
                heuristic.quarantine_reason =
                    Some("self_evolving_memory_low_confidence".to_owned());
                heuristic.version = heuristic.version.saturating_add(1);
                quarantined_heuristics = quarantined_heuristics.saturating_add(1);
            }
        }

        for record in &mut self.tool_reliability {
            let age = policy.current_step.saturating_sub(record.last_used_step);
            if age >= policy.stale_after_steps {
                record.trust_score = (record.trust_score * tool_decay).clamp(0.0, 1.0);
                record.decay_count = record.decay_count.saturating_add(1);
                record.version = record.version.saturating_add(1);
                decayed_tool_reliability = decayed_tool_reliability.saturating_add(1);
            }
        }

        let merged_duplicate_episodes = if policy.merge_duplicate_episodes {
            self.merge_duplicate_episodes()
        } else {
            0
        };

        SelfEvolvingMemoryMaintenanceReport {
            decayed_heuristics,
            decayed_tool_reliability,
            quarantined_heuristics,
            merged_duplicate_episodes,
            read_only: false,
            durable_write_allowed: false,
            applied_to_disk: false,
        }
    }

    pub fn preview_from_memory_admission(
        &self,
        preview: &MemoryAdmissionPreview,
    ) -> SelfEvolvingMemoryAdmissionPreview {
        let candidates = preview
            .candidates
            .iter()
            .map(memory_admission_candidate_preview)
            .collect();

        SelfEvolvingMemoryAdmissionPreview {
            candidates,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn consolidation_snapshot(
        &self,
        tenant_scope: impl Into<String>,
        current_step: u64,
    ) -> Vec<MemoryConsolidationRecord> {
        let tenant_scope = tenant_scope.into();
        let mut records = Vec::new();

        records.extend(self.episodes.iter().map(|episode| {
            MemoryConsolidationRecord::new(
                episode.record_id.clone(),
                tenant_scope.clone(),
                MemoryConsolidationEvidenceClass::RetrospectiveEpisode,
                episode.source_case_digest.clone(),
                stable_digest(&format!(
                    "{}:{}:{}",
                    episode.problem_digest,
                    episode.outcome_digest,
                    episode.key_insight_digests.len()
                )),
                episode.profile,
            )
            .with_scores(episode.quality, episode.quality)
            .with_last_touched_step(episode.sequence.min(current_step))
            .with_token_estimate(episode.token_estimate)
            .with_rollback_anchor(episode.rollback_anchor_id.clone())
            .with_validation_evidence_count(episode.validation_evidence_count)
            .with_protected(!episode.active)
        }));

        records.extend(self.heuristics.iter().map(|heuristic| {
            MemoryConsolidationRecord::new(
                heuristic.record_id.clone(),
                tenant_scope.clone(),
                MemoryConsolidationEvidenceClass::ProceduralHeuristic,
                heuristic.source_case_digest.clone(),
                heuristic.rule_digest.clone(),
                heuristic.profile,
            )
            .with_scores(heuristic.confidence, heuristic.priority)
            .with_last_touched_step(heuristic.last_updated_step.min(current_step))
            .with_token_estimate(32)
            .with_rollback_anchor(heuristic.rollback_anchor_id.clone())
            .with_validation_evidence_count(heuristic.validation_evidence_count)
            .with_protected(heuristic.quarantined)
        }));

        records.extend(self.tool_reliability.iter().map(|tool| {
            MemoryConsolidationRecord::new(
                format!("sem:tool-reliability:{}", tool.tool_id),
                tenant_scope.clone(),
                MemoryConsolidationEvidenceClass::ToolReliabilityObservation,
                tool.tool_digest.clone(),
                stable_digest(&format!(
                    "{}:{:?}:{}:{:.3}:{:.3}",
                    tool.tool_id,
                    tool.profile,
                    tool.observations,
                    tool.success_rate,
                    tool.avg_quality
                )),
                tool.profile,
            )
            .with_scores(tool.trust_score, tool.avg_quality)
            .with_last_touched_step(tool.last_used_step.min(current_step))
            .with_token_estimate(24)
            .with_rollback_anchor(format!("rollback:tool:{}", tool.tool_id))
            .with_validation_evidence_count(tool.observations)
        }));

        records
    }

    fn next_sequence(&mut self) -> u64 {
        if self.next_sequence == 0 {
            self.next_sequence = 1;
        }
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        sequence
    }

    fn merge_duplicate_episodes(&mut self) -> usize {
        let mut merged = 0usize;
        for index in 0..self.episodes.len() {
            if !self.episodes[index].active {
                continue;
            }
            let duplicate_of = (0..index).find(|previous| {
                self.episodes[*previous].active
                    && self.episodes[*previous].profile == self.episodes[index].profile
                    && self.episodes[*previous].problem_digest
                        == self.episodes[index].problem_digest
            });
            if let Some(previous) = duplicate_of {
                let keep_previous = self.episodes[previous].quality >= self.episodes[index].quality;
                if keep_previous {
                    self.episodes[index].active = false;
                    self.episodes[index].merged_into =
                        Some(self.episodes[previous].record_id.clone());
                } else {
                    self.episodes[previous].active = false;
                    self.episodes[previous].merged_into =
                        Some(self.episodes[index].record_id.clone());
                }
                merged = merged.saturating_add(1);
            }
        }
        merged
    }
}

fn compatible_duplicate_groups(
    records: &[MemoryConsolidationRecord],
) -> Vec<(String, Vec<String>)> {
    let mut groups = BTreeMap::<String, Vec<&MemoryConsolidationRecord>>::new();
    for record in records {
        groups
            .entry(compatible_merge_key(record))
            .or_default()
            .push(record);
    }

    groups
        .into_values()
        .filter_map(|mut group| {
            if group.len() < 2 {
                return None;
            }
            group.sort_by(|left, right| {
                candidate_rank(right)
                    .cmp(&candidate_rank(left))
                    .then_with(|| left.record_id.cmp(&right.record_id))
            });
            let primary = group[0].record_id.clone();
            let duplicate_ids = group
                .into_iter()
                .skip(1)
                .filter(|record| !record.protected)
                .map(|record| record.record_id.clone())
                .collect::<Vec<_>>();
            if duplicate_ids.is_empty() {
                None
            } else {
                Some((primary, duplicate_ids))
            }
        })
        .collect()
}

fn compatible_merge_key(record: &MemoryConsolidationRecord) -> String {
    format!(
        "{}:{}:{}:{}:{:?}",
        record.tenant_scope,
        record.evidence_class.as_str(),
        record.source_digest,
        record.content_digest,
        record.profile
    )
}

fn cross_tenant_merge_rejections(
    records: &[MemoryConsolidationRecord],
) -> Vec<MemoryConsolidationDecision> {
    let mut rejections = Vec::new();
    for left_index in 0..records.len() {
        for right in records.iter().skip(left_index + 1) {
            let left = &records[left_index];
            if left.tenant_scope == right.tenant_scope
                || left.evidence_class != right.evidence_class
                || left.source_digest != right.source_digest
                || left.content_digest != right.content_digest
                || left.profile != right.profile
            {
                continue;
            }
            let record_id = sanitize_identifier(
                &format!("merge-rejected:{}:{}", left.record_id, right.record_id),
                "merge-rejected",
            );
            rejections.push(MemoryConsolidationDecision {
                record_id,
                decision: MemoryConsolidationDecisionKind::MergeRejected,
                evidence_class: left.evidence_class,
                tenant_scope: sanitize_identifier(
                    &format!("{}_vs_{}", left.tenant_scope, right.tenant_scope),
                    "tenant-conflict",
                ),
                source_digest: left.source_digest.clone(),
                content_digest: left.content_digest.clone(),
                primary_record_id: Some(left.record_id.clone()),
                compacted_summary_digest: compacted_summary_digest([left, right]),
                reason_codes: vec![
                    "cross_tenant_merge_rejected".to_owned(),
                    "tenant_scope_incompatible".to_owned(),
                ],
                rollback_anchor_id: sanitize_identifier(
                    &format!(
                        "rollback:cross-tenant:{}:{}",
                        left.record_id, right.record_id
                    ),
                    "rollback",
                ),
                tombstone_id: None,
                confidence_before: right.confidence,
                confidence_after: right.confidence,
                retained_tokens: 0,
                saved_tokens: 0,
                read_only: true,
                write_allowed: false,
                applied: false,
            });
        }
    }
    rejections
}

fn keep_decision(record: &MemoryConsolidationRecord) -> MemoryConsolidationDecision {
    MemoryConsolidationDecision {
        record_id: record.record_id.clone(),
        decision: MemoryConsolidationDecisionKind::Keep,
        evidence_class: record.evidence_class,
        tenant_scope: record.tenant_scope.clone(),
        source_digest: record.source_digest.clone(),
        content_digest: record.content_digest.clone(),
        primary_record_id: None,
        compacted_summary_digest: compacted_summary_digest([record]),
        reason_codes: vec!["retained_without_change".to_owned()],
        rollback_anchor_id: record.rollback_anchor_id.clone(),
        tombstone_id: None,
        confidence_before: record.confidence,
        confidence_after: record.confidence,
        retained_tokens: record.token_estimate,
        saved_tokens: 0,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn merge_decision(
    record: &MemoryConsolidationRecord,
    primary_record_id: &str,
) -> MemoryConsolidationDecision {
    MemoryConsolidationDecision {
        record_id: record.record_id.clone(),
        decision: MemoryConsolidationDecisionKind::MergePreview,
        evidence_class: record.evidence_class,
        tenant_scope: record.tenant_scope.clone(),
        source_digest: record.source_digest.clone(),
        content_digest: record.content_digest.clone(),
        primary_record_id: Some(primary_record_id.to_owned()),
        compacted_summary_digest: stable_digest(&format!(
            "merge:{}:{}:{}:{}",
            record.evidence_class.as_str(),
            record.tenant_scope,
            record.source_digest,
            record.content_digest
        )),
        reason_codes: vec![
            "compatible_duplicate".to_owned(),
            "same_tenant_scope".to_owned(),
            "same_source_digest".to_owned(),
            "same_evidence_class".to_owned(),
        ],
        rollback_anchor_id: record.rollback_anchor_id.clone(),
        tombstone_id: None,
        confidence_before: record.confidence,
        confidence_after: record.confidence,
        retained_tokens: 0,
        saved_tokens: record.token_estimate,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn decay_decision(
    record: &MemoryConsolidationRecord,
    confidence_after: f32,
) -> MemoryConsolidationDecision {
    MemoryConsolidationDecision {
        record_id: record.record_id.clone(),
        decision: MemoryConsolidationDecisionKind::DecayPreview,
        evidence_class: record.evidence_class,
        tenant_scope: record.tenant_scope.clone(),
        source_digest: record.source_digest.clone(),
        content_digest: record.content_digest.clone(),
        primary_record_id: None,
        compacted_summary_digest: compacted_summary_digest([record]),
        reason_codes: vec!["stale_record_decay_preview".to_owned()],
        rollback_anchor_id: record.rollback_anchor_id.clone(),
        tombstone_id: None,
        confidence_before: record.confidence,
        confidence_after,
        retained_tokens: record.token_estimate,
        saved_tokens: 0,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn tombstone_decision(
    record: &MemoryConsolidationRecord,
    confidence_after: f32,
    aged: bool,
    low_confidence: bool,
    low_quality: bool,
) -> MemoryConsolidationDecision {
    let mut reason_codes = Vec::new();
    if aged {
        reason_codes.push("stale_record".to_owned());
    }
    if low_confidence {
        reason_codes.push("low_confidence".to_owned());
    }
    if low_quality {
        reason_codes.push("low_quality".to_owned());
    }
    reason_codes.push("tombstone_requires_operator_approval".to_owned());

    MemoryConsolidationDecision {
        record_id: record.record_id.clone(),
        decision: MemoryConsolidationDecisionKind::TombstonePreview,
        evidence_class: record.evidence_class,
        tenant_scope: record.tenant_scope.clone(),
        source_digest: record.source_digest.clone(),
        content_digest: record.content_digest.clone(),
        primary_record_id: None,
        compacted_summary_digest: compacted_summary_digest([record]),
        reason_codes,
        rollback_anchor_id: record.rollback_anchor_id.clone(),
        tombstone_id: Some(sanitize_identifier(
            &format!("tombstone:{}", record.record_id),
            "tombstone",
        )),
        confidence_before: record.confidence,
        confidence_after,
        retained_tokens: 0,
        saved_tokens: record.token_estimate,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn compacted_summary_digest<'a>(
    records: impl IntoIterator<Item = &'a MemoryConsolidationRecord>,
) -> String {
    stable_digest(
        &records
            .into_iter()
            .map(|record| {
                format!(
                    "{}:{}:{}:{}:{:.3}:{:.3}:{}",
                    record.evidence_class.as_str(),
                    record.tenant_scope,
                    record.source_digest,
                    record.content_digest,
                    record.confidence,
                    record.quality,
                    record.validation_evidence_count
                )
            })
            .collect::<Vec<_>>()
            .join("|"),
    )
}

fn consolidation_metrics(
    records: &[MemoryConsolidationRecord],
    decisions: &[MemoryConsolidationDecision],
) -> SelfEvolvingMemoryConsolidationMetrics {
    let token_estimate_before = records
        .iter()
        .map(|record| record.token_estimate)
        .sum::<usize>();
    let removed_record_ids = decisions
        .iter()
        .filter(|decision| {
            matches!(
                decision.decision,
                MemoryConsolidationDecisionKind::MergePreview
                    | MemoryConsolidationDecisionKind::TombstonePreview
            )
        })
        .map(|decision| decision.record_id.as_str())
        .collect::<Vec<_>>();
    let removed_records = records
        .iter()
        .filter(|record| removed_record_ids.iter().any(|id| *id == record.record_id))
        .count();
    let token_estimate_after_preview = token_estimate_before.saturating_sub(
        decisions
            .iter()
            .filter(|decision| {
                matches!(
                    decision.decision,
                    MemoryConsolidationDecisionKind::MergePreview
                        | MemoryConsolidationDecisionKind::TombstonePreview
                )
            })
            .map(|decision| decision.saved_tokens)
            .sum::<usize>(),
    );

    let precision_before = average_precision_milli(
        records
            .iter()
            .map(|record| (record.confidence, record.quality)),
    );
    let precision_after = average_precision_milli(records.iter().filter_map(|record| {
        if removed_record_ids.iter().any(|id| *id == record.record_id) {
            return None;
        }
        let confidence = decisions
            .iter()
            .find(|decision| decision.record_id == record.record_id)
            .map(|decision| decision.confidence_after)
            .unwrap_or(record.confidence);
        Some((confidence, record.quality))
    }));
    let saved_tokens = token_estimate_before.saturating_sub(token_estimate_after_preview);
    let retrieval_precision_delta_milli = precision_after - precision_before;

    SelfEvolvingMemoryConsolidationMetrics {
        records_before: records.len(),
        records_after_preview: records.len().saturating_sub(removed_records),
        token_estimate_before,
        token_estimate_after_preview,
        retrieval_precision_before_milli: precision_before,
        retrieval_precision_after_milli: precision_after,
        retrieval_precision_delta_milli,
        replay_safety_milli: if decisions
            .iter()
            .all(MemoryConsolidationDecision::is_preview_only)
        {
            1000
        } else {
            0
        },
        benchmark_impact_milli: saved_tokens as i64 * 10 + retrieval_precision_delta_milli,
    }
}

fn average_precision_milli(values: impl IntoIterator<Item = (f32, f32)>) -> i64 {
    let values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        return 0;
    }
    let total = values
        .iter()
        .map(|(confidence, quality)| {
            (clamp_unit(*confidence) * 0.45 + clamp_unit(*quality) * 0.55) * 1000.0
        })
        .sum::<f32>();
    (total / values.len() as f32).round() as i64
}

fn candidate_rank(record: &MemoryConsolidationRecord) -> (u8, i64, usize, u64) {
    (
        u8::from(record.protected),
        average_precision_milli([(record.confidence, record.quality)]),
        record.validation_evidence_count,
        record.last_touched_step,
    )
}

fn push_unique_reason(reason_codes: &mut Vec<String>, reason: &str) {
    if !reason_codes.iter().any(|item| item == reason) {
        reason_codes.push(reason.to_owned());
    }
}

fn memory_admission_candidate_preview(
    candidate: &MemoryAdmissionCandidate,
) -> SelfEvolvingMemoryAdmissionCandidatePreview {
    let mut blocked_reasons = Vec::new();
    if !matches!(
        candidate.kind,
        MemoryAdmissionKind::RetrospectiveEpisode
            | MemoryAdmissionKind::ProceduralHeuristic
            | MemoryAdmissionKind::ToolReliabilityObservation
    ) {
        blocked_reasons.push("self_evolving_memory_unsupported_store_lane".to_owned());
    }
    if candidate.decision != MemoryAdmissionDecision::Ready {
        blocked_reasons.push(format!(
            "self_evolving_memory_candidate_decision_{}",
            candidate.decision.as_str()
        ));
    }
    if !candidate.privacy_checked {
        blocked_reasons.push("self_evolving_memory_privacy_check_missing".to_owned());
    }
    if candidate.validation_evidence.is_empty() {
        blocked_reasons.push("self_evolving_memory_validation_evidence_missing".to_owned());
    }
    if candidate.rollback_anchor_id.trim().is_empty() {
        blocked_reasons.push("self_evolving_memory_rollback_anchor_missing".to_owned());
    }
    if candidate.durable_write_authorized || candidate.applied {
        blocked_reasons.push("self_evolving_memory_unsafe_write_or_apply_flag".to_owned());
    }

    SelfEvolvingMemoryAdmissionCandidatePreview {
        candidate_id: sanitize_identifier(&candidate.id, "candidate"),
        kind: candidate.kind,
        source_hash: sanitize_identifier(&candidate.source_hash, "source"),
        rollback_anchor_id: sanitize_identifier(&candidate.rollback_anchor_id, "rollback"),
        validation_evidence_count: candidate.validation_evidence.len(),
        eligible_for_store: blocked_reasons.is_empty(),
        blocked_reasons,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn accepted_write_report(
    lane: &'static str,
    record_id: String,
    content_digest: String,
) -> SelfEvolvingMemoryWriteReport {
    SelfEvolvingMemoryWriteReport {
        accepted: true,
        lane: lane.to_owned(),
        record_id: Some(record_id),
        blocked_reasons: Vec::new(),
        content_digest,
    }
}

fn blocked_write_report(
    lane: &'static str,
    blocked_reasons: Vec<String>,
) -> SelfEvolvingMemoryWriteReport {
    SelfEvolvingMemoryWriteReport {
        accepted: false,
        lane: lane.to_owned(),
        record_id: None,
        content_digest: stable_digest(&format!("{lane}:{blocked_reasons:?}")),
        blocked_reasons,
    }
}

fn retrieval_score(
    quality: f32,
    record_tags: &[String],
    query_tags: &[String],
    query_tokens: &[String],
) -> f32 {
    (quality * 0.70 + tag_overlap_score(record_tags, query_tags, query_tokens) * 0.30)
        .clamp(0.0, 1.0)
}

fn tag_overlap_score(
    record_tags: &[String],
    query_tags: &[String],
    query_tokens: &[String],
) -> f32 {
    if record_tags.is_empty() {
        return 0.0;
    }
    let hits = record_tags
        .iter()
        .filter(|tag| {
            query_tags.iter().any(|query_tag| query_tag == *tag)
                || query_tokens.iter().any(|query_token| query_token == *tag)
        })
        .count();
    (hits as f32 / record_tags.len().max(1) as f32).clamp(0.0, 1.0)
}

fn query_tokens(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(sanitize_tag)
        .filter(|token| !token.is_empty())
        .collect()
}

fn sanitize_tags(tags: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        let tag = sanitize_tag(tag);
        if !tag.is_empty() && !out.iter().any(|seen| seen == &tag) {
            out.push(tag);
        }
    }
    out
}

fn sanitize_tag(tag: &str) -> String {
    tag.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .flat_map(char::to_lowercase)
        .take(48)
        .collect()
}

fn sanitize_identifier(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .take(96)
        .collect::<String>();
    if sanitized.trim().is_empty() {
        fallback.to_owned()
    } else {
        sanitized
    }
}

fn trust_score(success_rate: f32, avg_quality: f32) -> f32 {
    (success_rate.clamp(0.0, 1.0) * 0.55 + avg_quality.clamp(0.0, 1.0) * 0.45).clamp(0.0, 1.0)
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn stable_digest(value: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("fnv64:{hash:016x}")
}

fn digest_or_stable(value: &str) -> String {
    let value = value.trim();
    if value.starts_with("fnv64:")
        || value.starts_with("sha256:")
        || value.starts_with("blake3:")
        || value.starts_with("redaction-digest:")
    {
        sanitize_identifier(value, "digest")
    } else {
        stable_digest(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_admission::MemoryPrivacyClassification;

    fn approval() -> SelfEvolvingMemoryApproval {
        SelfEvolvingMemoryApproval::approved(
            "rollback:self-evolving-memory:test",
            vec!["cargo-test:self-evolving-memory".to_owned()],
        )
    }

    fn episode_input(problem: &str, quality: f32, tags: &[&str]) -> SelfEvolvingEpisodeInput {
        SelfEvolvingEpisodeInput {
            problem: problem.to_owned(),
            solution_path: "run cargo test and keep the failing assertion as evidence".to_owned(),
            outcome: "tests passed after targeted fix".to_owned(),
            key_insights: vec!["prefer focused regression before broad refactor".to_owned()],
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            profile: TaskProfile::Coding,
            quality,
            token_estimate: 48,
            source_case_id: format!("case:{problem}"),
        }
    }

    #[test]
    fn retrieval_ranks_redacted_episodes_under_budget() {
        let mut store = SelfEvolvingMemoryStore::new();
        let approval = approval();
        let low = store.append_episode(
            episode_input("debug UI layout overflow", 0.40, &["ui", "layout"]),
            &approval,
        );
        let high = store.append_episode(
            episode_input("debug rust panic in trace tests", 0.92, &["rust", "test"]),
            &approval,
        );
        assert!(low.accepted);
        assert!(high.accepted);

        let report = store.retrieve_context(&SelfEvolvingMemoryQuery {
            prompt: "rust test panic".to_owned(),
            profile: TaskProfile::Coding,
            tags: vec!["rust".to_owned()],
            record_limit: 2,
            token_budget: 64,
        });

        assert_eq!(report.episodes.len(), 1);
        assert_eq!(report.skipped_by_budget, 1);
        assert!(report.redacted);
        assert!(report.episodes[0].score > 0.90);
        assert!(report.episodes[0].problem_digest.starts_with("fnv64:"));
        assert!(!report.summary_line().contains("rust test panic"));
        assert_eq!(report.retained_tokens, 48);
    }

    #[test]
    fn maintenance_decays_and_quarantines_low_confidence_heuristics() {
        let mut store = SelfEvolvingMemoryStore::new();
        let approval = approval();
        let write = store.append_heuristic(
            SelfEvolvingHeuristicInput {
                rule: "When trace gates change, add a schema rejection test.".to_owned(),
                tags: vec!["trace".to_owned(), "schema".to_owned()],
                profile: TaskProfile::Coding,
                priority: 0.80,
                confidence: 0.30,
                source_case_id: "case:trace-schema".to_owned(),
                updated_step: 1,
            },
            &approval,
        );
        assert!(write.accepted);

        let report = store.maintain(&SelfEvolvingMemoryMaintenancePolicy {
            current_step: 20,
            stale_after_steps: 5,
            heuristic_decay: 0.50,
            tool_reliability_decay: 0.95,
            quarantine_below_confidence: 0.20,
            merge_duplicate_episodes: false,
        });

        assert_eq!(report.decayed_heuristics, 1);
        assert_eq!(report.quarantined_heuristics, 1);
        assert!(!report.durable_write_allowed);
        assert!(!report.applied_to_disk);
        assert!(store.heuristics()[0].quarantined);
        assert_eq!(store.heuristics()[0].decay_count, 1);
        assert!(store.heuristics()[0].confidence < 0.20);
    }

    #[test]
    fn maintenance_merges_duplicate_episodes_without_deleting_records() {
        let mut store = SelfEvolvingMemoryStore::new();
        let approval = approval();
        store.append_episode(
            episode_input("same rust compiler failure", 0.65, &["rust"]),
            &approval,
        );
        store.append_episode(
            episode_input("same rust compiler failure", 0.90, &["rust"]),
            &approval,
        );

        let report = store.maintain(&SelfEvolvingMemoryMaintenancePolicy {
            merge_duplicate_episodes: true,
            ..SelfEvolvingMemoryMaintenancePolicy::default()
        });

        assert_eq!(report.merged_duplicate_episodes, 1);
        assert_eq!(store.episodes().len(), 2);
        assert_eq!(
            store
                .episodes()
                .iter()
                .filter(|record| record.active)
                .count(),
            1
        );
        assert!(
            store
                .episodes()
                .iter()
                .any(|record| record.merged_into.is_some())
        );
    }

    #[test]
    fn tool_reliability_tracks_versioned_observations() {
        let mut store = SelfEvolvingMemoryStore::new();
        let approval = approval();

        store.observe_tool(
            ToolReliabilityObservationInput {
                tool_name: "cargo-test".to_owned(),
                profile: TaskProfile::Coding,
                success: true,
                quality: 0.90,
                source_case_id: "case:green".to_owned(),
                observed_step: 2,
            },
            &approval,
        );
        store.observe_tool(
            ToolReliabilityObservationInput {
                tool_name: "cargo-test".to_owned(),
                profile: TaskProfile::Coding,
                success: false,
                quality: 0.20,
                source_case_id: "case:red".to_owned(),
                observed_step: 3,
            },
            &approval,
        );

        let record = &store.tool_reliability()[0];
        assert_eq!(store.tool_observations().len(), 2);
        assert_eq!(record.observations, 2);
        assert_eq!(record.successes, 1);
        assert_eq!(record.version, 2);
        assert!((record.success_rate - 0.5).abs() < 0.001);
        assert!((record.avg_quality - 0.55).abs() < 0.001);
    }

    #[test]
    fn maintenance_decays_stale_tool_reliability_without_disk_apply() {
        let mut store = SelfEvolvingMemoryStore::new();
        let approval = approval();

        store.observe_tool(
            ToolReliabilityObservationInput {
                tool_name: "cargo-test".to_owned(),
                profile: TaskProfile::Coding,
                success: true,
                quality: 0.80,
                source_case_id: "case:old-tool".to_owned(),
                observed_step: 1,
            },
            &approval,
        );

        let before = store.tool_reliability()[0].trust_score;
        let report = store.maintain(&SelfEvolvingMemoryMaintenancePolicy {
            current_step: 20,
            stale_after_steps: 5,
            heuristic_decay: 0.90,
            tool_reliability_decay: 0.50,
            quarantine_below_confidence: 0.20,
            merge_duplicate_episodes: false,
        });

        let record = &store.tool_reliability()[0];
        assert_eq!(report.decayed_tool_reliability, 1);
        assert!(record.trust_score < before);
        assert_eq!(record.decay_count, 1);
        assert_eq!(record.version, 2);
        assert!(!report.durable_write_allowed);
        assert!(!report.applied_to_disk);
    }

    #[test]
    fn admission_preview_blocks_unsafe_writes_and_keeps_read_only() {
        let store = SelfEvolvingMemoryStore::new();
        let candidate = MemoryAdmissionCandidate {
            id: "candidate/raw prompt should be sanitized".to_owned(),
            kind: MemoryAdmissionKind::RetrospectiveEpisode,
            decision: MemoryAdmissionDecision::Ready,
            profile: TaskProfile::Coding,
            prompt_digest: "fnv64:prompt".to_owned(),
            source_hash: "sha256:source".to_owned(),
            privacy_classification: MemoryPrivacyClassification::DigestOnly,
            prompt_chars: 128,
            quality: 0.91,
            process_reward: 0.88,
            critical_reflection_issues: 0,
            revision_actions: 0,
            runtime_kv_influence: None,
            runtime_kv_segment_yield: None,
            runtime_kv_budget_pressure: None,
            rollback_anchor_id: "rollback:self-evolving-memory".to_owned(),
            evidence: vec!["redacted-evidence".to_owned()],
            validation_evidence: vec!["cargo-test:self-evolving-memory".to_owned()],
            privacy_checked: true,
            durable_write_authorized: true,
            applied: true,
        };
        let preview = MemoryAdmissionPreview {
            candidates: vec![candidate],
            read_only: true,
            write_allowed: false,
            applied: false,
            ..MemoryAdmissionPreview::default()
        };

        let report = store.preview_from_memory_admission(&preview);

        assert!(report.read_only);
        assert!(!report.write_allowed);
        assert!(!report.applied);
        assert_eq!(report.eligible_count(), 0);
        assert_eq!(report.blocked_count(), 1);
        assert!(
            report.candidates[0]
                .blocked_reasons
                .contains(&"self_evolving_memory_unsafe_write_or_apply_flag".to_owned())
        );
        assert!(!report.candidates[0].candidate_id.contains(' '));
        assert!(report.summary_line().contains("eligible=0"));
    }

    #[test]
    fn missing_approval_blocks_store_mutation() {
        let mut store = SelfEvolvingMemoryStore::new();
        let report = store.append_episode(
            episode_input("private prompt never reaches store", 0.90, &["rust"]),
            &SelfEvolvingMemoryApproval {
                operator_approved: false,
                privacy_checked: true,
                rollback_anchor_id: "rollback:test".to_owned(),
                validation_evidence: vec!["cargo-test".to_owned()],
            },
        );

        assert!(!report.accepted);
        assert_eq!(store.episodes().len(), 0);
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolving_memory_operator_approval_missing".to_owned())
        );
        assert!(!report.summary_line().contains("private prompt"));
    }

    #[test]
    fn store_reports_emit_digest_only_trace_json_lines() {
        let mut store = SelfEvolvingMemoryStore::new();
        let approval = approval();
        let raw_problem = "private prompt should never appear in JSONL";
        store.append_episode(
            episode_input(raw_problem, 0.91, &["rust", "trace"]),
            &approval,
        );
        store.append_heuristic(
            SelfEvolvingHeuristicInput {
                rule: "private heuristic rule should be digested".to_owned(),
                tags: vec!["rust".to_owned(), "trace".to_owned()],
                profile: TaskProfile::Coding,
                priority: 0.80,
                confidence: 0.30,
                source_case_id: "case:trace-json".to_owned(),
                updated_step: 1,
            },
            &approval,
        );

        let retrieval = store.retrieve_context(&SelfEvolvingMemoryQuery {
            prompt: "private prompt query should be tokenized only".to_owned(),
            profile: TaskProfile::Coding,
            tags: vec!["rust".to_owned()],
            record_limit: 4,
            token_budget: 96,
        });
        let maintenance = store.maintain(&SelfEvolvingMemoryMaintenancePolicy {
            current_step: 20,
            stale_after_steps: 5,
            heuristic_decay: 0.50,
            tool_reliability_decay: 0.95,
            quarantine_below_confidence: 0.20,
            merge_duplicate_episodes: false,
        });
        let admission = SelfEvolvingMemoryAdmissionPreview {
            candidates: vec![SelfEvolvingMemoryAdmissionCandidatePreview {
                candidate_id: "sem_candidate_digest_only".to_owned(),
                kind: MemoryAdmissionKind::RetrospectiveEpisode,
                source_hash: "sha256:source".to_owned(),
                rollback_anchor_id: "rollback:self-evolving-memory".to_owned(),
                validation_evidence_count: 1,
                eligible_for_store: false,
                blocked_reasons: vec!["self_evolving_memory_unsafe_write_or_apply_flag".to_owned()],
                read_only: true,
                write_allowed: false,
                applied: false,
            }],
            read_only: true,
            write_allowed: false,
            applied: false,
        };

        for line in [
            retrieval.json_line(),
            maintenance.json_line(),
            admission.json_line(),
        ] {
            assert!(line.contains("\"schema\":\"rust-norion-self-evolving-memory-store-v1\""));
            assert!(line.contains("\"redacted\":true"));
            assert!(line.contains("\"write_allowed\":false"));
            assert!(line.contains("\"applied\":false"));
            assert!(line.contains("\"evidence_digest\":\"fnv64:"));
            assert!(!line.contains(raw_problem));
            assert!(!line.contains("private heuristic rule"));
            assert!(!line.contains("private prompt query"));
            assert!(
                crate::trace::evaluate_trace_schema_line(&line).is_empty(),
                "{line}"
            );
        }
    }

    #[test]
    fn consolidation_worker_replays_same_snapshot_and_merges_compatible_records() {
        let worker =
            SelfEvolvingMemoryConsolidationWorker::new(SelfEvolvingMemoryConsolidationPolicy {
                current_step: 20,
                stale_after_steps: 50,
                ..SelfEvolvingMemoryConsolidationPolicy::default()
            });
        let records = vec![
            consolidation_record(
                "episode:a",
                "tenant:alpha",
                MemoryConsolidationEvidenceClass::RetrospectiveEpisode,
                "source:compiler-fix",
                "content:borrow-check",
                0.90,
                0.92,
                18,
                64,
            ),
            consolidation_record(
                "episode:b",
                "tenant:alpha",
                MemoryConsolidationEvidenceClass::RetrospectiveEpisode,
                "source:compiler-fix",
                "content:borrow-check",
                0.60,
                0.66,
                17,
                48,
            ),
        ];

        let first = worker.plan(&records);
        let second = worker.plan(&records);

        assert!(first.replay_matches(&second));
        assert!(first.is_preview_only());
        assert_eq!(first.merge_count(), 1);
        assert_eq!(first.metrics.records_before, 2);
        assert_eq!(first.metrics.records_after_preview, 1);
        assert_eq!(first.metrics.token_estimate_before, 112);
        assert_eq!(first.metrics.token_estimate_after_preview, 64);
        assert!(
            first.decisions.iter().any(|decision| {
                decision.decision == MemoryConsolidationDecisionKind::MergePreview
                    && decision
                        .reason_codes
                        .contains(&"same_tenant_scope".to_owned())
                    && decision.primary_record_id.as_deref() == Some("episode:a")
            }),
            "{:?}",
            first.decisions
        );
    }

    #[test]
    fn consolidation_worker_does_not_merge_incompatible_evidence_classes() {
        let worker = SelfEvolvingMemoryConsolidationWorker::default();
        let records = vec![
            consolidation_record(
                "heuristic:a",
                "tenant:alpha",
                MemoryConsolidationEvidenceClass::ProceduralHeuristic,
                "source:shared",
                "content:shared",
                0.80,
                0.80,
                1,
                32,
            ),
            consolidation_record(
                "gene:a",
                "tenant:alpha",
                MemoryConsolidationEvidenceClass::GeneSegmentAnchor,
                "source:shared",
                "content:shared",
                0.82,
                0.82,
                1,
                32,
            ),
        ];

        let report = worker.plan(&records);

        assert_eq!(report.merge_count(), 0);
        assert_eq!(report.merge_rejected_count(), 0);
        assert_eq!(
            report.count_decision(MemoryConsolidationDecisionKind::Keep),
            2
        );
    }

    #[test]
    fn consolidation_worker_decays_stale_records_without_applying_mutation() {
        let worker =
            SelfEvolvingMemoryConsolidationWorker::new(SelfEvolvingMemoryConsolidationPolicy {
                current_step: 20,
                stale_after_steps: 5,
                decay_factor: 0.50,
                tombstone_below_confidence: 0.10,
                tombstone_below_quality: 0.10,
                merge_duplicate_records: false,
            });
        let records = vec![consolidation_record(
            "heuristic:old",
            "tenant:alpha",
            MemoryConsolidationEvidenceClass::ProceduralHeuristic,
            "source:trace",
            "content:schema",
            0.80,
            0.70,
            1,
            32,
        )];

        let report = worker.plan(&records);
        let decision = &report.decisions[0];

        assert_eq!(report.decay_count(), 1);
        assert_eq!(
            decision.decision,
            MemoryConsolidationDecisionKind::DecayPreview
        );
        assert!((decision.confidence_after - 0.40).abs() < 0.001);
        assert!(decision.is_preview_only());
        assert!(!report.applied);
    }

    #[test]
    fn consolidation_worker_proposes_tombstone_with_reason_codes_and_rollback_anchor() {
        let worker =
            SelfEvolvingMemoryConsolidationWorker::new(SelfEvolvingMemoryConsolidationPolicy {
                current_step: 30,
                stale_after_steps: 5,
                decay_factor: 0.50,
                tombstone_below_confidence: 0.20,
                tombstone_below_quality: 0.15,
                merge_duplicate_records: false,
            });
        let records = vec![
            consolidation_record(
                "tool:bad",
                "tenant:alpha",
                MemoryConsolidationEvidenceClass::ToolReliabilityObservation,
                "source:tool",
                "content:tool",
                0.30,
                0.10,
                1,
                24,
            )
            .with_rollback_anchor("rollback:tool:bad"),
        ];

        let report = worker.plan(&records);
        let decision = &report.decisions[0];

        assert_eq!(report.tombstone_count(), 1);
        assert_eq!(
            decision.decision,
            MemoryConsolidationDecisionKind::TombstonePreview
        );
        assert_eq!(decision.rollback_anchor_id, "rollback:tool:bad");
        assert!(decision.tombstone_id.is_some());
        assert!(decision.reason_codes.contains(&"low_quality".to_owned()));
        assert!(
            decision
                .reason_codes
                .contains(&"tombstone_requires_operator_approval".to_owned())
        );
        assert_eq!(report.metrics.records_after_preview, 0);
    }

    #[test]
    fn consolidation_worker_rejects_unsafe_cross_tenant_merge() {
        let worker = SelfEvolvingMemoryConsolidationWorker::default();
        let records = vec![
            consolidation_record(
                "episode:tenant-a",
                "tenant:a",
                MemoryConsolidationEvidenceClass::RetrospectiveEpisode,
                "source:shared",
                "content:shared",
                0.90,
                0.90,
                1,
                64,
            ),
            consolidation_record(
                "episode:tenant-b",
                "tenant:b",
                MemoryConsolidationEvidenceClass::RetrospectiveEpisode,
                "source:shared",
                "content:shared",
                0.91,
                0.91,
                1,
                64,
            ),
        ];

        let report = worker.plan(&records);

        assert_eq!(report.merge_count(), 0);
        assert_eq!(report.merge_rejected_count(), 1);
        assert!(
            report.decisions.iter().any(|decision| {
                decision.decision == MemoryConsolidationDecisionKind::MergeRejected
                    && decision
                        .reason_codes
                        .contains(&"cross_tenant_merge_rejected".to_owned())
                    && decision.is_preview_only()
            }),
            "{:?}",
            report.decisions
        );
        assert_eq!(report.metrics.records_after_preview, 2);
    }

    #[test]
    fn consolidation_worker_exports_metric_and_trace_gate_output() {
        let worker =
            SelfEvolvingMemoryConsolidationWorker::new(SelfEvolvingMemoryConsolidationPolicy {
                current_step: 20,
                stale_after_steps: 5,
                decay_factor: 0.50,
                tombstone_below_confidence: 0.20,
                tombstone_below_quality: 0.15,
                merge_duplicate_records: true,
            });
        let records = vec![
            consolidation_record(
                "gene:old",
                "tenant:alpha",
                MemoryConsolidationEvidenceClass::GeneSegmentAnchor,
                "source:gene-anchor",
                "content:route-strategy",
                0.90,
                0.90,
                1,
                40,
            ),
            consolidation_record(
                "gene:duplicate",
                "tenant:alpha",
                MemoryConsolidationEvidenceClass::GeneSegmentAnchor,
                "source:gene-anchor",
                "content:route-strategy",
                0.80,
                0.82,
                2,
                30,
            ),
            consolidation_record(
                "heuristic:stale",
                "tenant:alpha",
                MemoryConsolidationEvidenceClass::ProceduralHeuristic,
                "source:heuristic",
                "content:threshold",
                0.60,
                0.70,
                1,
                32,
            ),
        ];

        let report = worker.plan(&records);
        let json = report.json_line();

        assert_eq!(report.metrics.records_before, 3);
        assert!(report.metrics.records_after_preview < report.metrics.records_before);
        assert!(report.metrics.benchmark_impact_milli > 0);
        assert!(
            report
                .summary_line()
                .contains("memory_consolidation_metrics")
        );
        assert!(json.contains("\"operation\":\"consolidation_preview\""));
        assert!(
            crate::trace::evaluate_trace_schema_line(&json).is_empty(),
            "{json}"
        );
    }

    #[test]
    fn store_projects_episode_heuristic_and_tool_reliability_into_consolidation_snapshot() {
        let mut store = SelfEvolvingMemoryStore::new();
        let approval = approval();
        store.append_episode(
            episode_input("snapshot episode", 0.80, &["rust", "snapshot"]),
            &approval,
        );
        store.append_heuristic(
            SelfEvolvingHeuristicInput {
                rule: "Prefer digest-only maintenance previews.".to_owned(),
                tags: vec!["memory".to_owned()],
                profile: TaskProfile::Coding,
                priority: 0.70,
                confidence: 0.72,
                source_case_id: "case:heuristic-snapshot".to_owned(),
                updated_step: 2,
            },
            &approval,
        );
        store.observe_tool(
            ToolReliabilityObservationInput {
                tool_name: "cargo-test".to_owned(),
                profile: TaskProfile::Coding,
                success: true,
                quality: 0.86,
                source_case_id: "case:tool-snapshot".to_owned(),
                observed_step: 3,
            },
            &approval,
        );

        let snapshot = store.consolidation_snapshot("tenant:alpha", 10);

        assert_eq!(snapshot.len(), 3);
        assert!(
            snapshot
                .iter()
                .all(|record| record.tenant_scope == "tenant:alpha")
        );
        assert!(snapshot.iter().any(|record| {
            record.evidence_class == MemoryConsolidationEvidenceClass::RetrospectiveEpisode
        }));
        assert!(snapshot.iter().any(|record| {
            record.evidence_class == MemoryConsolidationEvidenceClass::ProceduralHeuristic
        }));
        assert!(snapshot.iter().any(|record| {
            record.evidence_class == MemoryConsolidationEvidenceClass::ToolReliabilityObservation
        }));
        assert!(snapshot.iter().all(|record| {
            record.source_digest.starts_with("fnv64:")
                && record.content_digest.starts_with("fnv64:")
                && !record.record_line().contains("snapshot episode")
        }));
    }

    fn consolidation_record(
        record_id: &str,
        tenant_scope: &str,
        evidence_class: MemoryConsolidationEvidenceClass,
        source: &str,
        content: &str,
        confidence: f32,
        quality: f32,
        last_touched_step: u64,
        token_estimate: usize,
    ) -> MemoryConsolidationRecord {
        MemoryConsolidationRecord::new(
            record_id,
            tenant_scope,
            evidence_class,
            source,
            content,
            TaskProfile::Coding,
        )
        .with_scores(confidence, quality)
        .with_last_touched_step(last_touched_step)
        .with_token_estimate(token_estimate)
        .with_rollback_anchor(format!("rollback:{record_id}"))
        .with_validation_evidence_count(1)
    }
}
