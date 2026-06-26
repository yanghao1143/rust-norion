use crate::drift::DriftReport;
use crate::hierarchy::TaskProfile;
use crate::process_reward::{ProcessRewardReport, RewardAction};
use crate::reflection::ReflectionReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAdmissionKind {
    RetrospectiveEpisode,
    ProceduralHeuristic,
    ToolReliabilityObservation,
    GistEvidence,
    RuntimeKvEvidence,
}

impl MemoryAdmissionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RetrospectiveEpisode => "retrospective_episode",
            Self::ProceduralHeuristic => "procedural_heuristic",
            Self::ToolReliabilityObservation => "tool_reliability_observation",
            Self::GistEvidence => "gist_evidence",
            Self::RuntimeKvEvidence => "runtime_kv_evidence",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAdmissionDecision {
    Ready,
    Hold,
    Reject,
    Quarantine,
}

impl MemoryAdmissionDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Hold => "hold",
            Self::Reject => "reject",
            Self::Quarantine => "quarantine",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAdmissionApprovalState {
    PendingApproval,
    HeldForEvidence,
    Rejected,
    Quarantined,
    Admitted,
}

impl MemoryAdmissionApprovalState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PendingApproval => "pending_approval",
            Self::HeldForEvidence => "held_for_evidence",
            Self::Rejected => "rejected",
            Self::Quarantined => "quarantined",
            Self::Admitted => "admitted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPrivacyClassification {
    DigestOnly,
    PublicSafe,
    SensitiveBlocked,
}

impl MemoryPrivacyClassification {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DigestOnly => "digest_only",
            Self::PublicSafe => "public_safe",
            Self::SensitiveBlocked => "sensitive_blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKvLedgerWriteDecision {
    PreviewOnly,
    Admitted,
    Held,
    Rejected,
    Duplicate,
    Decayed,
    Merged,
    Rollback,
}

impl MemoryKvLedgerWriteDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreviewOnly => "preview_only",
            Self::Admitted => "admitted",
            Self::Held => "held",
            Self::Rejected => "rejected",
            Self::Duplicate => "duplicate",
            Self::Decayed => "decayed",
            Self::Merged => "merged",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryKvLedgerWritePolicy {
    pub durable_writes_enabled: bool,
    pub operator_approved: bool,
    pub rollback_requested: bool,
    pub duplicate_source_hashes: Vec<String>,
    pub decayed_candidate_ids: Vec<String>,
    pub merged_candidate_ids: Vec<(String, String)>,
}

impl Default for MemoryKvLedgerWritePolicy {
    fn default() -> Self {
        Self {
            durable_writes_enabled: false,
            operator_approved: false,
            rollback_requested: false,
            duplicate_source_hashes: Vec::new(),
            decayed_candidate_ids: Vec::new(),
            merged_candidate_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryKvLedgerRecord {
    pub ledger_key: String,
    pub candidate_id: String,
    pub kind: MemoryAdmissionKind,
    pub admission_decision: MemoryAdmissionDecision,
    pub approval_state: MemoryAdmissionApprovalState,
    pub write_decision: MemoryKvLedgerWriteDecision,
    pub source_hash: String,
    pub privacy_classification: MemoryPrivacyClassification,
    pub rollback_anchor_id: String,
    pub validation_evidence: Vec<String>,
    pub rejection_reasons: Vec<String>,
    pub duplicate_of: Option<String>,
    pub merged_into: Option<String>,
    pub append_only: bool,
    pub durable_write_authorized: bool,
    pub applied: bool,
}

impl MemoryKvLedgerRecord {
    fn from_candidate(
        candidate: &MemoryAdmissionCandidate,
        packet: Option<&MemoryAdmissionReviewPacket>,
        policy: &MemoryKvLedgerWritePolicy,
    ) -> Self {
        let approval_state = packet
            .map(|packet| packet.approval_state)
            .unwrap_or(MemoryAdmissionApprovalState::HeldForEvidence);
        let mut rejection_reasons = writer_gate_failures(candidate, packet);
        let duplicate_of = policy
            .duplicate_source_hashes
            .iter()
            .find(|hash| *hash == &candidate.source_hash)
            .cloned();
        let merged_into = policy
            .merged_candidate_ids
            .iter()
            .find(|(candidate_id, _)| candidate_id == &candidate.id)
            .map(|(_, merged_into)| merged_into.clone());
        let write_decision = if !rejection_reasons.is_empty() {
            MemoryKvLedgerWriteDecision::Rejected
        } else if policy.rollback_requested
            || candidate.decision == MemoryAdmissionDecision::Quarantine
            || approval_state == MemoryAdmissionApprovalState::Quarantined
        {
            rejection_reasons.push("rollback_or_quarantine_required".to_owned());
            MemoryKvLedgerWriteDecision::Rollback
        } else if candidate.decision == MemoryAdmissionDecision::Reject
            || approval_state == MemoryAdmissionApprovalState::Rejected
        {
            rejection_reasons.push("candidate_rejected".to_owned());
            MemoryKvLedgerWriteDecision::Rejected
        } else if candidate.decision == MemoryAdmissionDecision::Hold
            || approval_state == MemoryAdmissionApprovalState::HeldForEvidence
        {
            rejection_reasons.push("held_for_more_evidence".to_owned());
            MemoryKvLedgerWriteDecision::Held
        } else if duplicate_of.is_some() {
            rejection_reasons.push("duplicate_source_hash".to_owned());
            MemoryKvLedgerWriteDecision::Duplicate
        } else if policy
            .decayed_candidate_ids
            .iter()
            .any(|candidate_id| candidate_id == &candidate.id)
        {
            rejection_reasons.push("candidate_decayed_before_write".to_owned());
            MemoryKvLedgerWriteDecision::Decayed
        } else if merged_into.is_some() {
            rejection_reasons.push("candidate_merged_before_write".to_owned());
            MemoryKvLedgerWriteDecision::Merged
        } else if !policy.durable_writes_enabled {
            rejection_reasons.push("durable_writes_disabled".to_owned());
            MemoryKvLedgerWriteDecision::PreviewOnly
        } else if !policy.operator_approved {
            rejection_reasons.push("operator_approval_missing".to_owned());
            MemoryKvLedgerWriteDecision::Held
        } else {
            MemoryKvLedgerWriteDecision::Admitted
        };
        let durable_write_authorized = write_decision == MemoryKvLedgerWriteDecision::Admitted
            && policy.durable_writes_enabled
            && policy.operator_approved;

        Self {
            ledger_key: ledger_key_for_candidate(candidate),
            candidate_id: candidate.id.clone(),
            kind: candidate.kind,
            admission_decision: candidate.decision,
            approval_state,
            write_decision,
            source_hash: candidate.source_hash.clone(),
            privacy_classification: candidate.privacy_classification,
            rollback_anchor_id: candidate.rollback_anchor_id.clone(),
            validation_evidence: candidate.validation_evidence.clone(),
            rejection_reasons,
            duplicate_of,
            merged_into,
            append_only: true,
            durable_write_authorized,
            applied: false,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "{}:{}:{} approval={} authorized={} applied={} rollback={} source_hash={} privacy={} validation={} reasons={}",
            self.write_decision.as_str(),
            self.kind.as_str(),
            self.candidate_id,
            self.approval_state.as_str(),
            self.durable_write_authorized,
            self.applied,
            self.rollback_anchor_id,
            self.source_hash,
            self.privacy_classification.as_str(),
            self.validation_evidence.len(),
            self.rejection_reasons.join("|")
        )
    }

    pub fn serialized_value(&self) -> Vec<u8> {
        format!(
            "memory_kv_ledger_v1\tcandidate={}\tkind={}\tdecision={}\tapproval={}\tsource_hash={}\tprivacy={}\trollback={}\tvalidation={}\treasons={}\tappend_only={}\tauthorized={}\tapplied={}",
            sanitize_review_text(&self.candidate_id),
            self.kind.as_str(),
            self.write_decision.as_str(),
            self.approval_state.as_str(),
            sanitize_review_text(&self.source_hash),
            self.privacy_classification.as_str(),
            sanitize_review_text(&self.rollback_anchor_id),
            self.validation_evidence.len(),
            self.rejection_reasons.join("|"),
            self.append_only,
            self.durable_write_authorized,
            self.applied
        )
        .into_bytes()
    }

    pub fn is_read_only_preview(&self) -> bool {
        !self.durable_write_authorized && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryKvLedgerWritePlan {
    pub records: Vec<MemoryKvLedgerRecord>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl Default for MemoryKvLedgerWritePlan {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }
}

impl MemoryKvLedgerWritePlan {
    pub fn from_preview(
        preview: &MemoryAdmissionPreview,
        policy: MemoryKvLedgerWritePolicy,
    ) -> Self {
        let records = preview
            .candidates
            .iter()
            .map(|candidate| {
                let packet = preview
                    .review_packets
                    .iter()
                    .find(|packet| packet.candidate_id == candidate.id);
                MemoryKvLedgerRecord::from_candidate(candidate, packet, &policy)
            })
            .collect::<Vec<_>>();
        let write_allowed = records.iter().any(|record| record.durable_write_authorized);

        Self {
            records,
            read_only: !write_allowed,
            write_allowed,
            applied: false,
        }
    }

    pub fn append_authorized_records(
        &mut self,
        store: &mut crate::disk_kv::DiskKvStore,
    ) -> std::io::Result<usize> {
        let mut applied = 0;
        for record in &mut self.records {
            if !record.durable_write_authorized || record.applied {
                continue;
            }
            let mut committed = record.clone();
            committed.applied = true;
            store.put(&record.ledger_key, committed.serialized_value())?;
            record.applied = true;
            applied += 1;
        }
        if applied > 0 {
            self.applied = true;
        }
        Ok(applied)
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    pub fn authorized_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.durable_write_authorized)
            .count()
    }

    pub fn applied_count(&self) -> usize {
        self.records.iter().filter(|record| record.applied).count()
    }

    pub fn count_decision(&self, decision: MemoryKvLedgerWriteDecision) -> usize {
        self.records
            .iter()
            .filter(|record| record.write_decision == decision)
            .count()
    }

    pub fn summary_lines(&self) -> Vec<String> {
        self.records
            .iter()
            .map(MemoryKvLedgerRecord::summary)
            .collect()
    }

    pub fn is_read_only_preview(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .records
                .iter()
                .all(MemoryKvLedgerRecord::is_read_only_preview)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReinforcedKvFusionSource {
    SemanticMemory,
    GistMemory,
    RuntimeKv,
    ColdEvidence,
    GenomeSegment,
}

impl ReinforcedKvFusionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SemanticMemory => "semantic_memory",
            Self::GistMemory => "gist_memory",
            Self::RuntimeKv => "runtime_kv",
            Self::ColdEvidence => "cold_evidence",
            Self::GenomeSegment => "genome_segment",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReinforcedKvFusionDecision {
    Fuse,
    Compress,
    Skip,
    Hold,
    Reject,
    ApprovalBlocked,
}

impl ReinforcedKvFusionDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fuse => "fuse",
            Self::Compress => "compress",
            Self::Skip => "skip",
            Self::Hold => "hold",
            Self::Reject => "reject",
            Self::ApprovalBlocked => "approval_blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReinforcedKvFusionPolicy {
    pub fuse_threshold: f32,
    pub compress_threshold: f32,
    pub hold_threshold: f32,
    pub compression_fraction: f32,
    pub max_full_fusion_tokens: usize,
}

impl Default for ReinforcedKvFusionPolicy {
    fn default() -> Self {
        Self {
            fuse_threshold: 0.72,
            compress_threshold: 0.50,
            hold_threshold: 0.34,
            compression_fraction: 0.42,
            max_full_fusion_tokens: 192,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReinforcedKvFusionCandidate {
    pub id: String,
    pub source: ReinforcedKvFusionSource,
    pub estimated_tokens: usize,
    pub trust: f32,
    pub freshness: f32,
    pub fitness: f32,
    pub task_relevance: f32,
    pub reinforcement: f32,
    pub privacy_classification: MemoryPrivacyClassification,
    pub rollback_anchor_id: String,
    pub source_hash: String,
    pub duplicate_of: Option<String>,
    pub contradictory: bool,
    pub required_anchor: bool,
    pub requires_approval: bool,
}

impl ReinforcedKvFusionCandidate {
    pub fn new(
        id: impl Into<String>,
        source: ReinforcedKvFusionSource,
        estimated_tokens: usize,
    ) -> Self {
        Self {
            id: sanitize_review_text(&id.into()),
            source,
            estimated_tokens: estimated_tokens.max(1),
            trust: 0.50,
            freshness: 0.50,
            fitness: 0.50,
            task_relevance: 0.50,
            reinforcement: 0.0,
            privacy_classification: MemoryPrivacyClassification::DigestOnly,
            rollback_anchor_id: "kv_fusion:stable".to_owned(),
            source_hash: "sha256:kv_fusion".to_owned(),
            duplicate_of: None,
            contradictory: false,
            required_anchor: false,
            requires_approval: false,
        }
    }

    pub fn with_scores(
        mut self,
        trust: f32,
        freshness: f32,
        fitness: f32,
        task_relevance: f32,
        reinforcement: f32,
    ) -> Self {
        self.trust = clamp_unit(trust);
        self.freshness = clamp_unit(freshness);
        self.fitness = clamp_unit(fitness);
        self.task_relevance = clamp_unit(task_relevance);
        self.reinforcement = clamp_reward(reinforcement);
        self
    }

    pub fn with_privacy(mut self, privacy_classification: MemoryPrivacyClassification) -> Self {
        self.privacy_classification = privacy_classification;
        self
    }

    pub fn with_rollback_anchor(mut self, rollback_anchor_id: impl Into<String>) -> Self {
        self.rollback_anchor_id = sanitize_review_text(&rollback_anchor_id.into());
        self
    }

    pub fn with_source_hash(mut self, source_hash: impl Into<String>) -> Self {
        self.source_hash = sanitize_review_text(&source_hash.into());
        self
    }

    pub fn with_duplicate_of(mut self, duplicate_of: impl Into<String>) -> Self {
        self.duplicate_of = Some(sanitize_review_text(&duplicate_of.into()));
        self
    }

    pub fn with_contradictory(mut self, contradictory: bool) -> Self {
        self.contradictory = contradictory;
        self
    }

    pub fn with_required_anchor(mut self, required_anchor: bool) -> Self {
        self.required_anchor = required_anchor;
        self
    }

    pub fn with_requires_approval(mut self, requires_approval: bool) -> Self {
        self.requires_approval = requires_approval;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReinforcedKvFusionScoreComponents {
    pub trust: f32,
    pub freshness: f32,
    pub fitness: f32,
    pub task_relevance: f32,
    pub reinforcement: f32,
    pub privacy: f32,
    pub token_cost: f32,
}

impl ReinforcedKvFusionScoreComponents {
    fn from_candidate(candidate: &ReinforcedKvFusionCandidate) -> Self {
        Self {
            trust: clamp_unit(candidate.trust),
            freshness: clamp_unit(candidate.freshness),
            fitness: clamp_unit(candidate.fitness),
            task_relevance: clamp_unit(candidate.task_relevance),
            reinforcement: clamp_reward(candidate.reinforcement),
            privacy: match candidate.privacy_classification {
                MemoryPrivacyClassification::SensitiveBlocked => 0.0,
                MemoryPrivacyClassification::DigestOnly => 0.82,
                MemoryPrivacyClassification::PublicSafe => 1.0,
            },
            token_cost: (candidate.estimated_tokens as f32 / 512.0).clamp(0.0, 1.0),
        }
    }

    fn score(self) -> f32 {
        let positive_reinforcement = self.reinforcement.max(0.0);
        let negative_reinforcement = (-self.reinforcement).max(0.0);
        (self.task_relevance * 0.24
            + self.fitness * 0.20
            + self.trust * 0.20
            + self.freshness * 0.12
            + positive_reinforcement * 0.16
            + self.privacy * 0.08
            - negative_reinforcement * 0.22
            - self.token_cost * 0.10)
            .clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReinforcedKvFusionDecisionRecord {
    pub candidate_id: String,
    pub source: ReinforcedKvFusionSource,
    pub decision: ReinforcedKvFusionDecision,
    pub estimated_tokens: usize,
    pub retained_tokens: usize,
    pub score: f32,
    pub components: ReinforcedKvFusionScoreComponents,
    pub privacy_classification: MemoryPrivacyClassification,
    pub rollback_anchor_id: String,
    pub duplicate_of: Option<String>,
    pub reason: String,
}

impl ReinforcedKvFusionDecisionRecord {
    pub fn saved_tokens(&self) -> usize {
        self.estimated_tokens.saturating_sub(self.retained_tokens)
    }

    pub fn summary(&self) -> String {
        format!(
            "id={} source={} decision={} score={:.3} components=trust:{:.3}|freshness:{:.3}|fitness:{:.3}|task:{:.3}|reinforcement:{:.3}|privacy_score:{:.3}|token_cost:{:.3} retained={} saved={} privacy={} rollback={} duplicate={} reason={}",
            self.candidate_id,
            self.source.as_str(),
            self.decision.as_str(),
            self.score,
            self.components.trust,
            self.components.freshness,
            self.components.fitness,
            self.components.task_relevance,
            self.components.reinforcement,
            self.components.privacy,
            self.components.token_cost,
            self.retained_tokens,
            self.saved_tokens(),
            self.privacy_classification.as_str(),
            self.rollback_anchor_id,
            self.duplicate_of.as_deref().unwrap_or("none"),
            self.reason
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReinforcedKvFusionPlan {
    pub candidates: usize,
    pub fused: usize,
    pub compressed: usize,
    pub skipped: usize,
    pub held: usize,
    pub rejected: usize,
    pub approval_blocked: usize,
    pub input_tokens: usize,
    pub retained_tokens: usize,
    pub saved_tokens: usize,
    pub min_score: f32,
    pub max_score: f32,
    pub average_score: f32,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub decisions: Vec<ReinforcedKvFusionDecisionRecord>,
}

impl Default for ReinforcedKvFusionPlan {
    fn default() -> Self {
        Self::from_candidates(ReinforcedKvFusionPolicy::default(), Vec::new())
    }
}

impl ReinforcedKvFusionPlan {
    pub fn from_admission_candidates(candidates: &[MemoryAdmissionCandidate]) -> Self {
        let candidates = candidates
            .iter()
            .map(fusion_candidate_from_admission)
            .collect::<Vec<_>>();
        Self::from_candidates(ReinforcedKvFusionPolicy::default(), candidates)
    }

    pub fn from_candidates(
        policy: ReinforcedKvFusionPolicy,
        candidates: Vec<ReinforcedKvFusionCandidate>,
    ) -> Self {
        let mut decisions = candidates
            .iter()
            .map(|candidate| decide_reinforced_kv_fusion(policy, candidate))
            .collect::<Vec<_>>();
        decisions.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

        let mut fused = 0usize;
        let mut compressed = 0usize;
        let mut skipped = 0usize;
        let mut held = 0usize;
        let mut rejected = 0usize;
        let mut approval_blocked = 0usize;
        let mut input_tokens = 0usize;
        let mut retained_tokens = 0usize;
        let mut score_sum = 0.0f32;
        let mut min_score = f32::INFINITY;
        let mut max_score = f32::NEG_INFINITY;

        for decision in &decisions {
            match decision.decision {
                ReinforcedKvFusionDecision::Fuse => fused = fused.saturating_add(1),
                ReinforcedKvFusionDecision::Compress => compressed = compressed.saturating_add(1),
                ReinforcedKvFusionDecision::Skip => skipped = skipped.saturating_add(1),
                ReinforcedKvFusionDecision::Hold => held = held.saturating_add(1),
                ReinforcedKvFusionDecision::Reject => rejected = rejected.saturating_add(1),
                ReinforcedKvFusionDecision::ApprovalBlocked => {
                    approval_blocked = approval_blocked.saturating_add(1)
                }
            }
            input_tokens = input_tokens.saturating_add(decision.estimated_tokens);
            retained_tokens = retained_tokens.saturating_add(decision.retained_tokens);
            score_sum += decision.score;
            min_score = min_score.min(decision.score);
            max_score = max_score.max(decision.score);
        }

        let candidates = decisions.len();
        Self {
            candidates,
            fused,
            compressed,
            skipped,
            held,
            rejected,
            approval_blocked,
            input_tokens,
            retained_tokens,
            saved_tokens: input_tokens.saturating_sub(retained_tokens),
            min_score: if candidates == 0 { 0.0 } else { min_score },
            max_score: if candidates == 0 { 0.0 } else { max_score },
            average_score: if candidates == 0 {
                0.0
            } else {
                score_sum / candidates as f32
            },
            read_only: true,
            write_allowed: false,
            applied: false,
            decisions,
        }
    }

    pub fn decision_count_matches(&self) -> bool {
        self.fused
            .saturating_add(self.compressed)
            .saturating_add(self.skipped)
            .saturating_add(self.held)
            .saturating_add(self.rejected)
            .saturating_add(self.approval_blocked)
            == self.candidates
    }

    pub fn token_accounting_matches(&self) -> bool {
        self.retained_tokens.saturating_add(self.saved_tokens) == self.input_tokens
    }

    pub fn is_read_only_preview(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn score_summaries(&self, limit: usize) -> Vec<String> {
        self.decisions
            .iter()
            .take(limit)
            .map(ReinforcedKvFusionDecisionRecord::summary)
            .collect()
    }
}

fn decide_reinforced_kv_fusion(
    policy: ReinforcedKvFusionPolicy,
    candidate: &ReinforcedKvFusionCandidate,
) -> ReinforcedKvFusionDecisionRecord {
    let components = ReinforcedKvFusionScoreComponents::from_candidate(candidate);
    let score = components.score();
    let (decision, reason) = if candidate.rollback_anchor_id.trim().is_empty() {
        (
            ReinforcedKvFusionDecision::ApprovalBlocked,
            "rollback_anchor_missing",
        )
    } else if candidate.privacy_classification == MemoryPrivacyClassification::SensitiveBlocked {
        (ReinforcedKvFusionDecision::Reject, "privacy_rejected")
    } else if candidate.contradictory {
        (ReinforcedKvFusionDecision::Hold, "contradiction_hold")
    } else if candidate.duplicate_of.is_some() {
        (ReinforcedKvFusionDecision::Skip, "duplicate_merge_skip")
    } else if candidate.requires_approval && score >= policy.compress_threshold {
        (
            ReinforcedKvFusionDecision::ApprovalBlocked,
            "operator_approval_required",
        )
    } else if candidate.required_anchor {
        if score >= policy.compress_threshold {
            (ReinforcedKvFusionDecision::Fuse, "required_anchor_fused")
        } else {
            (
                ReinforcedKvFusionDecision::Compress,
                "required_anchor_compressed",
            )
        }
    } else if score >= policy.fuse_threshold
        && candidate.estimated_tokens <= policy.max_full_fusion_tokens
    {
        (ReinforcedKvFusionDecision::Fuse, "score_fuse")
    } else if score >= policy.compress_threshold {
        (ReinforcedKvFusionDecision::Compress, "score_compress")
    } else if score >= policy.hold_threshold {
        (ReinforcedKvFusionDecision::Hold, "score_hold")
    } else {
        (ReinforcedKvFusionDecision::Skip, "low_score_skip")
    };
    let retained_tokens = match decision {
        ReinforcedKvFusionDecision::Fuse => candidate.estimated_tokens,
        ReinforcedKvFusionDecision::Compress => (candidate.estimated_tokens as f32
            * policy.compression_fraction.clamp(0.10, 0.90))
        .ceil() as usize,
        ReinforcedKvFusionDecision::Skip
        | ReinforcedKvFusionDecision::Hold
        | ReinforcedKvFusionDecision::Reject
        | ReinforcedKvFusionDecision::ApprovalBlocked => 0,
    }
    .min(candidate.estimated_tokens);

    ReinforcedKvFusionDecisionRecord {
        candidate_id: sanitize_review_text(&candidate.id),
        source: candidate.source,
        decision,
        estimated_tokens: candidate.estimated_tokens,
        retained_tokens,
        score,
        components,
        privacy_classification: candidate.privacy_classification,
        rollback_anchor_id: sanitize_review_text(&candidate.rollback_anchor_id),
        duplicate_of: candidate.duplicate_of.clone(),
        reason: reason.to_owned(),
    }
}

pub(crate) fn fusion_candidate_from_admission(
    candidate: &MemoryAdmissionCandidate,
) -> ReinforcedKvFusionCandidate {
    let source = match candidate.kind {
        MemoryAdmissionKind::RetrospectiveEpisode | MemoryAdmissionKind::ProceduralHeuristic => {
            ReinforcedKvFusionSource::SemanticMemory
        }
        MemoryAdmissionKind::ToolReliabilityObservation => ReinforcedKvFusionSource::ColdEvidence,
        MemoryAdmissionKind::GistEvidence => ReinforcedKvFusionSource::GistMemory,
        MemoryAdmissionKind::RuntimeKvEvidence => ReinforcedKvFusionSource::RuntimeKv,
    };
    let reinforcement = match candidate.decision {
        MemoryAdmissionDecision::Ready => 0.45,
        MemoryAdmissionDecision::Hold => -0.05,
        MemoryAdmissionDecision::Reject => -0.65,
        MemoryAdmissionDecision::Quarantine => -0.90,
    };
    let estimated_tokens = match candidate.kind {
        MemoryAdmissionKind::RetrospectiveEpisode => 96,
        MemoryAdmissionKind::ProceduralHeuristic => 48,
        MemoryAdmissionKind::ToolReliabilityObservation => 32,
        MemoryAdmissionKind::GistEvidence => 64,
        MemoryAdmissionKind::RuntimeKvEvidence => 128,
    };

    let (trust, freshness, fitness, task_relevance, reinforcement) =
        fusion_scores_from_admission(candidate, reinforcement);

    ReinforcedKvFusionCandidate::new(candidate.id.clone(), source, estimated_tokens)
        .with_scores(trust, freshness, fitness, task_relevance, reinforcement)
        .with_privacy(candidate.privacy_classification)
        .with_rollback_anchor(candidate.rollback_anchor_id.clone())
        .with_source_hash(candidate.source_hash.clone())
        .with_requires_approval(candidate.decision == MemoryAdmissionDecision::Ready)
        .with_contradictory(candidate.critical_reflection_issues > 0)
}

fn fusion_scores_from_admission(
    candidate: &MemoryAdmissionCandidate,
    reinforcement: f32,
) -> (f32, f32, f32, f32, f32) {
    if candidate.kind != MemoryAdmissionKind::RuntimeKvEvidence {
        return (
            candidate.quality,
            candidate.process_reward,
            (candidate.quality + candidate.process_reward) * 0.5,
            task_relevance_for_admission_kind(candidate.kind),
            reinforcement,
        );
    }

    let influence = candidate.runtime_kv_influence.unwrap_or(0.0);
    let influence = candidate
        .runtime_kv_segment_yield
        .map(|segment_yield| influence * segment_yield)
        .unwrap_or(influence);
    let budget_pressure = candidate.runtime_kv_budget_pressure.unwrap_or(0.0);
    let influence = influence * (1.0 - budget_pressure * 0.30).clamp(0.70, 1.0);
    let signal_multiplier = 0.70 + influence * 0.30;
    (
        candidate.quality * signal_multiplier,
        candidate.process_reward * signal_multiplier,
        ((candidate.quality + candidate.process_reward) * 0.5 * 0.55 + influence * 0.45)
            .clamp(0.0, 1.0),
        (0.52 + influence * 0.38 - budget_pressure * 0.10).clamp(0.35, 0.92),
        (reinforcement + (influence - 0.50) * 0.50 - budget_pressure * 0.20).clamp(-1.0, 1.0),
    )
}

fn runtime_kv_segment_yield(input: MemoryAdmissionInput<'_>) -> Option<f32> {
    let total = input
        .runtime_kv_segments_included
        .saturating_add(input.runtime_kv_segments_skipped)
        .saturating_add(input.runtime_kv_segments_rejected);
    if total == 0 {
        return None;
    }

    let total = total as f32;
    let included = input.runtime_kv_segments_included as f32 / total;
    let skipped = input.runtime_kv_segments_skipped as f32 / total;
    let rejected = input.runtime_kv_segments_rejected as f32 / total;
    Some((included - skipped * 0.25 - rejected * 0.75).clamp(0.0, 1.0))
}

fn runtime_kv_budget_pressure(input: MemoryAdmissionInput<'_>) -> Option<f32> {
    if input.budget_limited_runtime_kv_imports_skipped == 0 {
        return None;
    }

    let total = input
        .exported_runtime_kv_blocks
        .saturating_add(input.budget_limited_runtime_kv_imports_skipped);

    Some((input.budget_limited_runtime_kv_imports_skipped as f32 / total as f32).clamp(0.0, 1.0))
}

fn task_relevance_for_admission_kind(kind: MemoryAdmissionKind) -> f32 {
    match kind {
        MemoryAdmissionKind::RetrospectiveEpisode => 0.86,
        MemoryAdmissionKind::ProceduralHeuristic => 0.78,
        MemoryAdmissionKind::ToolReliabilityObservation => 0.58,
        MemoryAdmissionKind::GistEvidence => 0.74,
        MemoryAdmissionKind::RuntimeKvEvidence => 0.82,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryAdmissionCandidate {
    pub id: String,
    pub kind: MemoryAdmissionKind,
    pub decision: MemoryAdmissionDecision,
    pub profile: TaskProfile,
    pub prompt_digest: String,
    pub source_hash: String,
    pub privacy_classification: MemoryPrivacyClassification,
    pub prompt_chars: usize,
    pub quality: f32,
    pub process_reward: f32,
    pub critical_reflection_issues: usize,
    pub revision_actions: usize,
    pub runtime_kv_influence: Option<f32>,
    pub runtime_kv_segment_yield: Option<f32>,
    pub runtime_kv_budget_pressure: Option<f32>,
    pub rollback_anchor_id: String,
    pub evidence: Vec<String>,
    pub validation_evidence: Vec<String>,
    pub privacy_checked: bool,
    pub durable_write_authorized: bool,
    pub applied: bool,
}

impl MemoryAdmissionCandidate {
    pub fn summary(&self) -> String {
        format!(
            "{}:{}:{} q={:.3} reward={:.3} runtime_kv_influence={} runtime_kv_segment_yield={} runtime_kv_budget_pressure={} critical={} revisions={} source_hash={} privacy={} validation={} privacy_checked={} durable_write_authorized={} applied={}",
            self.decision.as_str(),
            self.kind.as_str(),
            self.id,
            self.quality,
            self.process_reward,
            option_score(self.runtime_kv_influence),
            option_score(self.runtime_kv_segment_yield),
            option_score(self.runtime_kv_budget_pressure),
            self.critical_reflection_issues,
            self.revision_actions,
            self.source_hash,
            self.privacy_classification.as_str(),
            self.validation_evidence.len(),
            self.privacy_checked,
            self.durable_write_authorized,
            self.applied
        )
    }

    pub fn is_read_only_preview(&self) -> bool {
        self.privacy_checked && !self.durable_write_authorized && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryAdmissionReviewPacket {
    pub packet_id: String,
    pub candidate_id: String,
    pub kind: MemoryAdmissionKind,
    pub decision: MemoryAdmissionDecision,
    pub approval_state: MemoryAdmissionApprovalState,
    pub rollback_anchor_id: String,
    pub source_hash: String,
    pub privacy_classification: MemoryPrivacyClassification,
    pub evidence: Vec<String>,
    pub validation_evidence: Vec<String>,
    pub risk_flags: Vec<String>,
    pub next_action: String,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl MemoryAdmissionReviewPacket {
    pub fn summary(&self) -> String {
        format!(
            "{}:{}:{} approval={} next={} risks={} evidence={} validation={} source_hash={} privacy={} rollback={} read_only={} write_allowed={} applied={}",
            self.decision.as_str(),
            self.kind.as_str(),
            self.packet_id,
            self.approval_state.as_str(),
            self.next_action,
            self.risk_flags.join("|"),
            self.evidence.join("|"),
            self.validation_evidence.len(),
            self.source_hash,
            self.privacy_classification.as_str(),
            self.rollback_anchor_id,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }

    pub fn is_read_only_preview(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryAdmissionPreview {
    pub candidates: Vec<MemoryAdmissionCandidate>,
    pub review_packets: Vec<MemoryAdmissionReviewPacket>,
    pub ledger_plan: MemoryKvLedgerWritePlan,
    pub fusion_plan: ReinforcedKvFusionPlan,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl Default for MemoryAdmissionPreview {
    fn default() -> Self {
        Self {
            candidates: Vec::new(),
            review_packets: Vec::new(),
            ledger_plan: MemoryKvLedgerWritePlan {
                read_only: true,
                ..MemoryKvLedgerWritePlan::default()
            },
            fusion_plan: ReinforcedKvFusionPlan::default(),
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }
}

impl MemoryAdmissionPreview {
    pub fn from_feedback(input: MemoryAdmissionInput<'_>) -> Self {
        let mut candidates = Vec::new();
        let prompt_digest = prompt_digest(input.prompt);
        let prompt_chars = input.prompt.chars().count();
        let profile_slug = profile_slug(input.profile);
        let rollback_anchor_id = format!("memory_admission:{profile_slug}:stable");
        let quality = clamp_unit(input.report.quality);
        let process_reward = clamp_unit(input.process_reward.total);

        candidates.push(candidate(
            format!("memory_admission:{profile_slug}:episode:{prompt_digest}"),
            MemoryAdmissionKind::RetrospectiveEpisode,
            episode_decision(input.report, input.process_reward, input.drift_report),
            input,
            &prompt_digest,
            prompt_chars,
            quality,
            process_reward,
            &rollback_anchor_id,
            episode_evidence(input),
        ));

        if !input.report.issues.is_empty() || !input.report.revision_actions.is_empty() {
            candidates.push(candidate(
                format!("memory_admission:{profile_slug}:heuristic:{prompt_digest}"),
                MemoryAdmissionKind::ProceduralHeuristic,
                heuristic_decision(input.report, input.drift_report),
                input,
                &prompt_digest,
                prompt_chars,
                quality,
                process_reward,
                &rollback_anchor_id,
                heuristic_evidence(input.report),
            ));
        }

        if input.has_tool_reliability_signal() {
            candidates.push(candidate(
                format!("memory_admission:{profile_slug}:tool-reliability:{prompt_digest}"),
                MemoryAdmissionKind::ToolReliabilityObservation,
                tool_reliability_decision(input),
                input,
                &prompt_digest,
                prompt_chars,
                quality,
                process_reward,
                &rollback_anchor_id,
                tool_reliability_evidence(input),
            ));
        }

        if input.gist_records > 0 || input.stored_gist_memories > 0 {
            candidates.push(candidate(
                format!("memory_admission:{profile_slug}:gist:{prompt_digest}"),
                MemoryAdmissionKind::GistEvidence,
                evidence_decision(
                    input.stored_gist_memories,
                    input.drift_report.allow_memory_write,
                ),
                input,
                &prompt_digest,
                prompt_chars,
                quality,
                process_reward,
                &rollback_anchor_id,
                vec![
                    format!("gist_records={}", input.gist_records),
                    format!("stored_gist_memories={}", input.stored_gist_memories),
                ],
            ));
        }

        if input.exported_runtime_kv_blocks > 0 {
            let mut runtime_kv_evidence = vec![
                format!("runtime_kv_exported={}", input.exported_runtime_kv_blocks),
                format!(
                    "stored_runtime_kv_memories={}",
                    input.stored_runtime_kv_memories
                ),
                format!("runtime_kv_hold={}", input.runtime_kv_hold),
                format!(
                    "runtime_kv_influence={}",
                    option_score(input.runtime_kv_influence)
                ),
                format!(
                    "runtime_kv_segments=yield:{}:included={}:skipped={}:rejected={}",
                    option_score(runtime_kv_segment_yield(input)),
                    input.runtime_kv_segments_included,
                    input.runtime_kv_segments_skipped,
                    input.runtime_kv_segments_rejected
                ),
            ];
            if let Some(budget_pressure) = runtime_kv_budget_pressure(input) {
                runtime_kv_evidence.push(format!(
                    "runtime_kv_budget=pressure:{}:skipped={}",
                    option_score(Some(budget_pressure)),
                    input.budget_limited_runtime_kv_imports_skipped
                ));
            }

            candidates.push(candidate(
                format!("memory_admission:{profile_slug}:runtime-kv:{prompt_digest}"),
                MemoryAdmissionKind::RuntimeKvEvidence,
                runtime_kv_decision(input),
                input,
                &prompt_digest,
                prompt_chars,
                quality,
                process_reward,
                &rollback_anchor_id,
                runtime_kv_evidence,
            ));
        }

        let review_packets = candidates
            .iter()
            .map(review_packet_for_candidate)
            .collect::<Vec<_>>();

        let mut preview = Self {
            candidates,
            review_packets,
            ledger_plan: MemoryKvLedgerWritePlan {
                read_only: true,
                ..MemoryKvLedgerWritePlan::default()
            },
            fusion_plan: ReinforcedKvFusionPlan::default(),
            read_only: true,
            write_allowed: false,
            applied: false,
        };
        preview.ledger_plan =
            MemoryKvLedgerWritePlan::from_preview(&preview, MemoryKvLedgerWritePolicy::default());
        preview.fusion_plan =
            ReinforcedKvFusionPlan::from_admission_candidates(&preview.candidates);
        preview
    }

    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    pub fn ready_count(&self) -> usize {
        self.count_decision(MemoryAdmissionDecision::Ready)
    }

    pub fn hold_count(&self) -> usize {
        self.count_decision(MemoryAdmissionDecision::Hold)
    }

    pub fn reject_count(&self) -> usize {
        self.count_decision(MemoryAdmissionDecision::Reject)
    }

    pub fn quarantine_count(&self) -> usize {
        self.count_decision(MemoryAdmissionDecision::Quarantine)
    }

    pub fn blocked_count(&self) -> usize {
        self.hold_count().saturating_add(self.quarantine_count())
    }

    pub fn admitted_count(&self) -> usize {
        self.review_packets
            .iter()
            .filter(|packet| packet.approval_state == MemoryAdmissionApprovalState::Admitted)
            .count()
    }

    pub fn review_packet_count(&self) -> usize {
        self.review_packets.len()
    }

    pub fn ledger_record_count(&self) -> usize {
        self.ledger_plan.record_count()
    }

    pub fn ledger_authorized_count(&self) -> usize {
        self.ledger_plan.authorized_count()
    }

    pub fn ledger_applied_count(&self) -> usize {
        self.ledger_plan.applied_count()
    }

    pub fn ledger_preview_only_count(&self) -> usize {
        self.ledger_plan
            .count_decision(MemoryKvLedgerWriteDecision::PreviewOnly)
    }

    pub fn ledger_held_count(&self) -> usize {
        self.ledger_plan
            .count_decision(MemoryKvLedgerWriteDecision::Held)
    }

    pub fn ledger_rejected_count(&self) -> usize {
        self.ledger_plan
            .count_decision(MemoryKvLedgerWriteDecision::Rejected)
    }

    pub fn ledger_duplicate_count(&self) -> usize {
        self.ledger_plan
            .count_decision(MemoryKvLedgerWriteDecision::Duplicate)
    }

    pub fn ledger_decayed_count(&self) -> usize {
        self.ledger_plan
            .count_decision(MemoryKvLedgerWriteDecision::Decayed)
    }

    pub fn ledger_merged_count(&self) -> usize {
        self.ledger_plan
            .count_decision(MemoryKvLedgerWriteDecision::Merged)
    }

    pub fn ledger_rollback_count(&self) -> usize {
        self.ledger_plan
            .count_decision(MemoryKvLedgerWriteDecision::Rollback)
    }

    pub fn kind_summaries(&self) -> Vec<String> {
        unique_strings(
            self.candidates
                .iter()
                .map(|candidate| candidate.kind.as_str().to_owned()),
        )
    }

    pub fn decision_summaries(&self) -> Vec<String> {
        unique_strings(
            self.candidates
                .iter()
                .map(|candidate| candidate.decision.as_str().to_owned()),
        )
    }

    pub fn candidate_summaries(&self) -> Vec<String> {
        self.candidates
            .iter()
            .map(MemoryAdmissionCandidate::summary)
            .collect()
    }

    pub fn review_packet_summaries(&self) -> Vec<String> {
        self.review_packets
            .iter()
            .map(MemoryAdmissionReviewPacket::summary)
            .collect()
    }

    pub fn ledger_summaries(&self) -> Vec<String> {
        self.ledger_plan.summary_lines()
    }

    pub fn fusion_score_summaries(&self, limit: usize) -> Vec<String> {
        self.fusion_plan.score_summaries(limit)
    }

    pub fn is_read_only_preview(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .candidates
                .iter()
                .all(MemoryAdmissionCandidate::is_read_only_preview)
            && self
                .review_packets
                .iter()
                .all(MemoryAdmissionReviewPacket::is_read_only_preview)
            && self.ledger_plan.is_read_only_preview()
            && self.fusion_plan.is_read_only_preview()
    }

    fn count_decision(&self, decision: MemoryAdmissionDecision) -> usize {
        self.candidates
            .iter()
            .filter(|candidate| candidate.decision == decision)
            .count()
    }
}

fn review_packet_for_candidate(
    candidate: &MemoryAdmissionCandidate,
) -> MemoryAdmissionReviewPacket {
    let approval_state = approval_state_for_candidate(candidate);
    MemoryAdmissionReviewPacket {
        packet_id: format!("review:{}", candidate.id),
        candidate_id: candidate.id.clone(),
        kind: candidate.kind,
        decision: candidate.decision,
        approval_state,
        rollback_anchor_id: candidate.rollback_anchor_id.clone(),
        source_hash: candidate.source_hash.clone(),
        privacy_classification: candidate.privacy_classification,
        evidence: candidate
            .evidence
            .iter()
            .map(|evidence| sanitize_review_text(evidence))
            .collect(),
        validation_evidence: candidate
            .validation_evidence
            .iter()
            .map(|evidence| sanitize_review_text(evidence))
            .collect(),
        risk_flags: risk_flags_for_candidate(candidate),
        next_action: next_action_for_state(approval_state).to_owned(),
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn approval_state_for_candidate(
    candidate: &MemoryAdmissionCandidate,
) -> MemoryAdmissionApprovalState {
    if candidate.applied {
        MemoryAdmissionApprovalState::Admitted
    } else {
        match candidate.decision {
            MemoryAdmissionDecision::Ready => MemoryAdmissionApprovalState::PendingApproval,
            MemoryAdmissionDecision::Hold => MemoryAdmissionApprovalState::HeldForEvidence,
            MemoryAdmissionDecision::Reject => MemoryAdmissionApprovalState::Rejected,
            MemoryAdmissionDecision::Quarantine => MemoryAdmissionApprovalState::Quarantined,
        }
    }
}

fn risk_flags_for_candidate(candidate: &MemoryAdmissionCandidate) -> Vec<String> {
    let mut risks = Vec::new();
    if candidate.critical_reflection_issues > 0 {
        risks.push("critical_reflection".to_owned());
    }
    if candidate.quality < 0.35 {
        risks.push("low_quality".to_owned());
    }
    if candidate.process_reward < 0.35 {
        risks.push("low_process_reward".to_owned());
    }
    match candidate.decision {
        MemoryAdmissionDecision::Ready => risks.push("requires_approval_gate".to_owned()),
        MemoryAdmissionDecision::Hold => risks.push("needs_more_evidence".to_owned()),
        MemoryAdmissionDecision::Reject => risks.push("reject_without_write".to_owned()),
        MemoryAdmissionDecision::Quarantine => risks.push("quarantine_required".to_owned()),
    }
    if !candidate.privacy_checked {
        risks.push("privacy_unchecked".to_owned());
    }
    if candidate.durable_write_authorized || candidate.applied {
        risks.push("durable_write_attempt".to_owned());
    }
    risks
}

fn next_action_for_state(state: MemoryAdmissionApprovalState) -> &'static str {
    match state {
        MemoryAdmissionApprovalState::PendingApproval => "review_for_durable_write_gate",
        MemoryAdmissionApprovalState::HeldForEvidence => "collect_more_evidence",
        MemoryAdmissionApprovalState::Rejected => "do_not_store",
        MemoryAdmissionApprovalState::Quarantined => "quarantine_and_require_repair",
        MemoryAdmissionApprovalState::Admitted => "audit_admitted_memory",
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryAdmissionInput<'a> {
    pub prompt: &'a str,
    pub profile: TaskProfile,
    pub report: &'a ReflectionReport,
    pub process_reward: &'a ProcessRewardReport,
    pub drift_report: &'a DriftReport,
    pub stored_memory: bool,
    pub gist_records: usize,
    pub stored_gist_memories: usize,
    pub exported_runtime_kv_blocks: usize,
    pub stored_runtime_kv_memories: usize,
    pub runtime_kv_hold: bool,
    pub runtime_kv_influence: Option<f32>,
    pub budget_limited_runtime_kv_imports_skipped: usize,
    pub runtime_kv_segments_included: usize,
    pub runtime_kv_segments_skipped: usize,
    pub runtime_kv_segments_rejected: usize,
    pub used_memories: usize,
    pub memory_feedback_updates: usize,
    pub runtime_adapter_observations: usize,
    pub runtime_adapter_current_signal: bool,
    pub runtime_adapter_selection_mismatch: bool,
    pub runtime_adapter_best_score: Option<f32>,
    pub runtime_adapter_best_reward: Option<f32>,
    pub runtime_adapter_best_quality: Option<f32>,
    pub toolsmith_blueprints: usize,
    pub toolsmith_ready: usize,
    pub toolsmith_held: usize,
    pub toolsmith_rejected: usize,
    pub toolsmith_gate_passed: bool,
}

impl MemoryAdmissionInput<'_> {
    fn has_tool_reliability_signal(&self) -> bool {
        self.runtime_adapter_observations > 0
            || self.runtime_adapter_current_signal
            || self.toolsmith_blueprints > 0
            || self.toolsmith_rejected > 0
            || self.runtime_adapter_selection_mismatch
    }
}

fn candidate(
    id: String,
    kind: MemoryAdmissionKind,
    decision: MemoryAdmissionDecision,
    input: MemoryAdmissionInput<'_>,
    prompt_digest: &str,
    prompt_chars: usize,
    quality: f32,
    process_reward: f32,
    rollback_anchor_id: &str,
    evidence: Vec<String>,
) -> MemoryAdmissionCandidate {
    MemoryAdmissionCandidate {
        id,
        kind,
        decision,
        profile: input.profile,
        prompt_digest: prompt_digest.to_owned(),
        source_hash: format!("sha256:{prompt_digest}"),
        privacy_classification: MemoryPrivacyClassification::DigestOnly,
        prompt_chars,
        quality,
        process_reward,
        critical_reflection_issues: input.report.critical_issue_count(),
        revision_actions: input.report.revision_actions.len(),
        runtime_kv_influence: (kind == MemoryAdmissionKind::RuntimeKvEvidence)
            .then(|| input.runtime_kv_influence)
            .flatten()
            .map(clamp_unit),
        runtime_kv_segment_yield: (kind == MemoryAdmissionKind::RuntimeKvEvidence)
            .then(|| runtime_kv_segment_yield(input))
            .flatten(),
        runtime_kv_budget_pressure: (kind == MemoryAdmissionKind::RuntimeKvEvidence)
            .then(|| runtime_kv_budget_pressure(input))
            .flatten(),
        rollback_anchor_id: rollback_anchor_id.to_owned(),
        evidence,
        validation_evidence: validation_evidence_for_candidate(
            decision,
            quality,
            process_reward,
            input.drift_report,
        ),
        privacy_checked: true,
        durable_write_authorized: false,
        applied: false,
    }
}

fn episode_decision(
    report: &ReflectionReport,
    reward: &ProcessRewardReport,
    drift: &DriftReport,
) -> MemoryAdmissionDecision {
    if drift.rollback_adaptive || report.critical_issue_count() > 0 {
        MemoryAdmissionDecision::Quarantine
    } else if !report.store_as_memory || reward.action == RewardAction::Penalize {
        MemoryAdmissionDecision::Reject
    } else if !drift.allow_memory_write {
        MemoryAdmissionDecision::Hold
    } else {
        MemoryAdmissionDecision::Ready
    }
}

fn heuristic_decision(report: &ReflectionReport, drift: &DriftReport) -> MemoryAdmissionDecision {
    if report.critical_issue_count() > 0 {
        MemoryAdmissionDecision::Hold
    } else if drift.rollback_adaptive {
        MemoryAdmissionDecision::Quarantine
    } else {
        MemoryAdmissionDecision::Ready
    }
}

fn evidence_decision(stored: usize, allow_memory_write: bool) -> MemoryAdmissionDecision {
    if stored > 0 {
        MemoryAdmissionDecision::Ready
    } else if allow_memory_write {
        MemoryAdmissionDecision::Hold
    } else {
        MemoryAdmissionDecision::Reject
    }
}

fn runtime_kv_decision(input: MemoryAdmissionInput<'_>) -> MemoryAdmissionDecision {
    if input.stored_runtime_kv_memories > 0 {
        MemoryAdmissionDecision::Ready
    } else if input.drift_report.rollback_adaptive || input.report.critical_issue_count() > 0 {
        MemoryAdmissionDecision::Quarantine
    } else if input.runtime_kv_hold || !input.drift_report.allow_runtime_kv_write {
        MemoryAdmissionDecision::Hold
    } else {
        MemoryAdmissionDecision::Ready
    }
}

fn tool_reliability_decision(input: MemoryAdmissionInput<'_>) -> MemoryAdmissionDecision {
    if input.drift_report.rollback_adaptive || input.report.critical_issue_count() > 0 {
        MemoryAdmissionDecision::Quarantine
    } else if input.process_reward.action == RewardAction::Penalize
        || input.runtime_adapter_selection_mismatch
        || input.toolsmith_rejected > 0
    {
        MemoryAdmissionDecision::Hold
    } else if input
        .runtime_adapter_best_score
        .filter(|score| *score >= 0.70)
        .is_some()
        || (input.runtime_adapter_current_signal
            && input.process_reward.total >= 0.70
            && input.report.quality >= 0.70)
        || (input.toolsmith_ready > 0 && input.toolsmith_gate_passed)
    {
        MemoryAdmissionDecision::Ready
    } else {
        MemoryAdmissionDecision::Hold
    }
}

fn episode_evidence(input: MemoryAdmissionInput<'_>) -> Vec<String> {
    vec![
        format!("store_as_memory={}", input.report.store_as_memory),
        format!("stored_memory={}", input.stored_memory),
        format!("used_memories={}", input.used_memories),
        format!("memory_feedback_updates={}", input.memory_feedback_updates),
        format!("reward_action={}", input.process_reward.action.as_str()),
        format!(
            "drift_memory_write_allowed={}",
            input.drift_report.allow_memory_write
        ),
    ]
}

fn heuristic_evidence(report: &ReflectionReport) -> Vec<String> {
    let mut evidence = Vec::new();
    evidence.push(format!("reflection_issues={}", report.issues.len()));
    evidence.push(format!(
        "critical_reflection_issues={}",
        report.critical_issue_count()
    ));
    evidence.push(format!(
        "revision_actions={}",
        report.revision_actions.len()
    ));
    for action in report.revision_actions.iter().take(4) {
        evidence.push(format!("revision_action={action}"));
    }
    evidence
}

fn tool_reliability_evidence(input: MemoryAdmissionInput<'_>) -> Vec<String> {
    vec![
        format!(
            "runtime_adapter_observations={}",
            input.runtime_adapter_observations
        ),
        format!(
            "runtime_adapter_selection_mismatch={}",
            input.runtime_adapter_selection_mismatch
        ),
        format!(
            "runtime_adapter_current_signal={}",
            input.runtime_adapter_current_signal
        ),
        format!(
            "runtime_adapter_best_score={}",
            option_score(input.runtime_adapter_best_score)
        ),
        format!(
            "runtime_adapter_best_reward={}",
            option_score(input.runtime_adapter_best_reward)
        ),
        format!(
            "runtime_adapter_best_quality={}",
            option_score(input.runtime_adapter_best_quality)
        ),
        format!("toolsmith_blueprints={}", input.toolsmith_blueprints),
        format!("toolsmith_ready={}", input.toolsmith_ready),
        format!("toolsmith_held={}", input.toolsmith_held),
        format!("toolsmith_rejected={}", input.toolsmith_rejected),
        format!("toolsmith_gate_passed={}", input.toolsmith_gate_passed),
    ]
}

fn option_score(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{:.3}", value.clamp(0.0, 1.0)))
        .unwrap_or_else(|| "none".to_owned())
}

fn validation_evidence_for_candidate(
    decision: MemoryAdmissionDecision,
    quality: f32,
    process_reward: f32,
    drift: &DriftReport,
) -> Vec<String> {
    vec![
        format!("admission_decision={}", decision.as_str()),
        format!("quality={quality:.3}"),
        format!("process_reward={process_reward:.3}"),
        format!("drift_memory_write_allowed={}", drift.allow_memory_write),
        format!(
            "drift_runtime_kv_write_allowed={}",
            drift.allow_runtime_kv_write
        ),
        format!("drift_rollback={}", drift.rollback_adaptive),
        "privacy_checked=true".to_owned(),
        "rollback_anchor_present=true".to_owned(),
    ]
}

fn writer_gate_failures(
    candidate: &MemoryAdmissionCandidate,
    packet: Option<&MemoryAdmissionReviewPacket>,
) -> Vec<String> {
    let mut failures = Vec::new();
    if packet.is_none() {
        failures.push("review_packet_missing".to_owned());
    }
    if candidate.rollback_anchor_id.trim().is_empty() {
        failures.push("rollback_anchor_missing".to_owned());
    }
    if candidate.source_hash.trim().is_empty() {
        failures.push("source_hash_missing".to_owned());
    }
    if !candidate.privacy_checked
        || candidate.privacy_classification == MemoryPrivacyClassification::SensitiveBlocked
    {
        failures.push("privacy_gate_failed".to_owned());
    }
    if candidate.validation_evidence.is_empty() {
        failures.push("validation_evidence_missing".to_owned());
    }
    if candidate.durable_write_authorized || candidate.applied {
        failures.push("candidate_already_attempted_write".to_owned());
    }
    if let Some(packet) = packet {
        if packet.rollback_anchor_id != candidate.rollback_anchor_id {
            failures.push("review_packet_rollback_mismatch".to_owned());
        }
        if packet.source_hash != candidate.source_hash {
            failures.push("review_packet_source_hash_mismatch".to_owned());
        }
        if packet.write_allowed || packet.applied {
            failures.push("review_packet_write_attempt".to_owned());
        }
    }
    failures
}

fn ledger_key_for_candidate(candidate: &MemoryAdmissionCandidate) -> String {
    format!(
        "memory-ledger/{}/{}/{}",
        candidate.kind.as_str(),
        sanitize_review_text(&candidate.source_hash),
        sanitize_review_text(&candidate.id)
    )
}

fn prompt_digest(prompt: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in prompt.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn sanitize_review_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len().min(96));
    for ch in value.chars().take(96) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '=' | ':' | '.' | '/') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}

fn unique_strings(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

fn profile_slug(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn clamp_reward(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drift::DriftSeverity;
    use crate::process_reward::ProcessRewardComponents;
    use crate::reflection::ReflectionIssue;
    use crate::reflection::ReflectionSeverity;

    #[test]
    fn clean_feedback_creates_ready_episode_without_prompt_leak() {
        let report = ReflectionReport {
            quality: 0.82,
            contradictions: Vec::new(),
            issues: Vec::new(),
            revision_actions: Vec::new(),
            revision_passes: 0,
            revised_answer: "safe answer".to_owned(),
            store_as_memory: true,
            lesson: "reuse safe answer".to_owned(),
        };
        let reward = ProcessRewardReport {
            total: 0.84,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Reinforce,
            notes: Vec::new(),
        };
        let drift = DriftReport {
            severity: DriftSeverity::Stable,
            allow_memory_write: true,
            allow_runtime_kv_write: true,
            penalize_used_memory: false,
            rollback_adaptive: false,
            notes: Vec::new(),
        };

        let preview = MemoryAdmissionPreview::from_feedback(MemoryAdmissionInput {
            prompt: "secret prompt text should not appear in summaries",
            profile: TaskProfile::Coding,
            report: &report,
            process_reward: &reward,
            drift_report: &drift,
            stored_memory: true,
            gist_records: 0,
            stored_gist_memories: 0,
            exported_runtime_kv_blocks: 0,
            stored_runtime_kv_memories: 0,
            runtime_kv_hold: false,
            runtime_kv_influence: None,
            budget_limited_runtime_kv_imports_skipped: 0,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            used_memories: 1,
            memory_feedback_updates: 0,
            runtime_adapter_observations: 0,
            runtime_adapter_current_signal: false,
            runtime_adapter_selection_mismatch: false,
            runtime_adapter_best_score: None,
            runtime_adapter_best_reward: None,
            runtime_adapter_best_quality: None,
            toolsmith_blueprints: 0,
            toolsmith_ready: 0,
            toolsmith_held: 0,
            toolsmith_rejected: 0,
            toolsmith_gate_passed: true,
        });

        assert_eq!(preview.candidate_count(), 1);
        assert_eq!(preview.ready_count(), 1);
        assert_eq!(preview.blocked_count(), 0);
        assert_eq!(preview.admitted_count(), 0);
        assert_eq!(preview.review_packet_count(), 1);
        assert_eq!(preview.ledger_record_count(), 1);
        assert_eq!(preview.ledger_preview_only_count(), 1);
        assert_eq!(preview.ledger_authorized_count(), 0);
        assert_eq!(preview.fusion_plan.candidates, 1);
        assert_eq!(preview.fusion_plan.approval_blocked, 1);
        assert!(preview.fusion_plan.decision_count_matches());
        assert!(preview.fusion_plan.token_accounting_matches());
        assert!(preview.is_read_only_preview());
        assert_eq!(
            preview.review_packets[0].approval_state,
            MemoryAdmissionApprovalState::PendingApproval
        );
        assert_eq!(
            preview.review_packets[0].next_action,
            "review_for_durable_write_gate"
        );
        assert!(
            !preview
                .candidate_summaries()
                .iter()
                .any(|summary| summary.contains("secret prompt text"))
        );
        assert!(
            !preview
                .review_packet_summaries()
                .iter()
                .any(|summary| summary.contains("secret prompt text"))
        );
    }

    #[test]
    fn reinforced_kv_fusion_scores_positive_and_penalty_signals() {
        let plan = ReinforcedKvFusionPlan::from_candidates(
            ReinforcedKvFusionPolicy::default(),
            vec![
                ReinforcedKvFusionCandidate::new(
                    "runtime-positive",
                    ReinforcedKvFusionSource::RuntimeKv,
                    64,
                )
                .with_scores(0.95, 0.90, 0.92, 0.95, 0.80)
                .with_rollback_anchor("anchor:runtime:positive"),
                ReinforcedKvFusionCandidate::new(
                    "runtime-harmful",
                    ReinforcedKvFusionSource::RuntimeKv,
                    256,
                )
                .with_scores(0.20, 0.20, 0.15, 0.20, -0.90)
                .with_rollback_anchor("anchor:runtime:harmful"),
            ],
        );

        let positive = plan
            .decisions
            .iter()
            .find(|decision| decision.candidate_id == "runtime-positive")
            .unwrap();
        let harmful = plan
            .decisions
            .iter()
            .find(|decision| decision.candidate_id == "runtime-harmful")
            .unwrap();

        assert_eq!(plan.fused, 1);
        assert_eq!(plan.skipped, 1);
        assert!(plan.saved_tokens > 0);
        assert!(plan.decision_count_matches());
        assert!(plan.token_accounting_matches());
        assert_eq!(positive.decision, ReinforcedKvFusionDecision::Fuse);
        assert_eq!(positive.retained_tokens, 64);
        assert_eq!(harmful.decision, ReinforcedKvFusionDecision::Skip);
        assert_eq!(harmful.retained_tokens, 0);
        assert!(positive.score > harmful.score);
        assert!(plan.is_read_only_preview());
    }

    #[test]
    fn runtime_kv_influence_reweights_fusion_decisions() {
        let high = runtime_kv_preview_with_influence(0.92);
        let low = runtime_kv_preview_with_influence(0.05);
        let high_runtime = runtime_kv_fusion_decision(&high);
        let low_runtime = runtime_kv_fusion_decision(&low);

        assert_eq!(
            high_runtime.decision,
            ReinforcedKvFusionDecision::ApprovalBlocked
        );
        assert_eq!(low_runtime.decision, ReinforcedKvFusionDecision::Hold);
        assert!(high_runtime.score > low_runtime.score);
        assert!(high_runtime.components.fitness > low_runtime.components.fitness);
        assert!(high.candidates.iter().any(|candidate| {
            candidate.kind == MemoryAdmissionKind::RuntimeKvEvidence
                && candidate.runtime_kv_influence == Some(0.92)
                && candidate
                    .evidence
                    .iter()
                    .any(|item| item == "runtime_kv_influence=0.920")
        }));
        assert!(high.fusion_plan.token_accounting_matches());
        assert!(low.fusion_plan.token_accounting_matches());
    }

    #[test]
    fn runtime_kv_segment_yield_downweights_low_value_fusion() {
        let efficient = runtime_kv_preview_with_influence_and_segments(0.92, 2, 0, 0);
        let wasteful = runtime_kv_preview_with_influence_and_segments(0.92, 0, 3, 2);
        let efficient_runtime = runtime_kv_fusion_decision(&efficient);
        let wasteful_runtime = runtime_kv_fusion_decision(&wasteful);

        assert_eq!(
            efficient_runtime.decision,
            ReinforcedKvFusionDecision::ApprovalBlocked
        );
        assert_eq!(wasteful_runtime.decision, ReinforcedKvFusionDecision::Hold);
        assert!(efficient_runtime.score > wasteful_runtime.score);
        assert!(efficient_runtime.components.fitness > wasteful_runtime.components.fitness);
        assert!(
            efficient_runtime.components.reinforcement > wasteful_runtime.components.reinforcement
        );
        assert!(wasteful.candidates.iter().any(|candidate| {
            candidate.kind == MemoryAdmissionKind::RuntimeKvEvidence
                && candidate.runtime_kv_influence == Some(0.92)
                && candidate.runtime_kv_segment_yield == Some(0.0)
                && candidate.evidence.iter().any(|item| {
                    item == "runtime_kv_segments=yield:0.000:included=0:skipped=3:rejected=2"
                })
        }));
        assert!(efficient.fusion_plan.token_accounting_matches());
        assert!(wasteful.fusion_plan.token_accounting_matches());
    }

    #[test]
    fn runtime_kv_budget_pressure_downweights_fusion() {
        let unconstrained = runtime_kv_preview_with_influence_segments_and_budget(0.92, 1, 0, 0, 0);
        let budget_limited =
            runtime_kv_preview_with_influence_segments_and_budget(0.92, 1, 0, 0, 4);
        let unconstrained_runtime = runtime_kv_fusion_decision(&unconstrained);
        let budget_limited_runtime = runtime_kv_fusion_decision(&budget_limited);

        assert_eq!(
            unconstrained_runtime.decision,
            ReinforcedKvFusionDecision::ApprovalBlocked
        );
        assert_eq!(
            budget_limited_runtime.decision,
            ReinforcedKvFusionDecision::ApprovalBlocked
        );
        assert!(unconstrained_runtime.score > budget_limited_runtime.score);
        assert!(
            unconstrained_runtime.components.fitness > budget_limited_runtime.components.fitness
        );
        assert!(
            unconstrained_runtime.components.task_relevance
                > budget_limited_runtime.components.task_relevance
        );
        assert!(
            unconstrained_runtime.components.reinforcement
                > budget_limited_runtime.components.reinforcement
        );
        assert!(unconstrained.candidates.iter().any(|candidate| {
            candidate.kind == MemoryAdmissionKind::RuntimeKvEvidence
                && candidate.runtime_kv_budget_pressure.is_none()
                && !candidate
                    .evidence
                    .iter()
                    .any(|item| item.starts_with("runtime_kv_budget="))
        }));
        assert!(budget_limited.candidates.iter().any(|candidate| {
            candidate.kind == MemoryAdmissionKind::RuntimeKvEvidence
                && candidate.runtime_kv_influence == Some(0.92)
                && candidate.runtime_kv_budget_pressure == Some(0.8)
                && candidate
                    .evidence
                    .iter()
                    .any(|item| item == "runtime_kv_budget=pressure:0.800:skipped=4")
        }));
        assert!(unconstrained.fusion_plan.token_accounting_matches());
        assert!(budget_limited.fusion_plan.token_accounting_matches());
    }

    #[test]
    fn reinforced_kv_fusion_merges_duplicates_and_blocks_bad_candidates() {
        let plan = ReinforcedKvFusionPlan::from_candidates(
            ReinforcedKvFusionPolicy::default(),
            vec![
                ReinforcedKvFusionCandidate::new(
                    "semantic-duplicate",
                    ReinforcedKvFusionSource::SemanticMemory,
                    80,
                )
                .with_scores(0.90, 0.88, 0.86, 0.90, 0.40)
                .with_duplicate_of("semantic-primary")
                .with_rollback_anchor("anchor:semantic"),
                ReinforcedKvFusionCandidate::new(
                    "gist-contradiction",
                    ReinforcedKvFusionSource::GistMemory,
                    72,
                )
                .with_scores(0.88, 0.80, 0.84, 0.90, 0.20)
                .with_contradictory(true)
                .with_rollback_anchor("anchor:gist"),
                ReinforcedKvFusionCandidate::new(
                    "runtime-private",
                    ReinforcedKvFusionSource::RuntimeKv,
                    96,
                )
                .with_scores(0.95, 0.92, 0.90, 0.90, 0.55)
                .with_privacy(MemoryPrivacyClassification::SensitiveBlocked)
                .with_rollback_anchor("anchor:runtime"),
                ReinforcedKvFusionCandidate::new(
                    "genome-missing-anchor",
                    ReinforcedKvFusionSource::GenomeSegment,
                    32,
                )
                .with_scores(0.95, 0.95, 0.95, 0.95, 0.60)
                .with_rollback_anchor(""),
            ],
        );

        assert_eq!(plan.candidates, 4);
        assert_eq!(plan.skipped, 1);
        assert_eq!(plan.held, 1);
        assert_eq!(plan.rejected, 1);
        assert_eq!(plan.approval_blocked, 1);
        assert_eq!(plan.retained_tokens, 0);
        assert_eq!(plan.saved_tokens, plan.input_tokens);
        assert!(plan.decision_count_matches());
        assert!(plan.token_accounting_matches());
        assert!(
            plan.score_summaries(4)
                .iter()
                .any(|summary| summary.contains("duplicate=semantic-primary"))
        );
        assert!(
            plan.score_summaries(4)
                .iter()
                .any(|summary| summary.contains("reason=contradiction_hold"))
        );
        assert!(
            plan.score_summaries(4)
                .iter()
                .any(|summary| summary.contains("reason=privacy_rejected"))
        );
        assert!(
            plan.score_summaries(4)
                .iter()
                .any(|summary| summary.contains("reason=rollback_anchor_missing"))
        );
    }

    #[test]
    fn reinforced_kv_fusion_seeded_inputs_are_deterministic_and_preserve_required_anchors() {
        let policy = ReinforcedKvFusionPolicy::default();
        let candidates = seeded_kv_fusion_candidates(0x25F0_CAFE, 8);

        let first = ReinforcedKvFusionPlan::from_candidates(policy, candidates.clone());
        let second = ReinforcedKvFusionPlan::from_candidates(policy, candidates.clone());
        let mut reversed = candidates;
        reversed.reverse();
        let reversed = ReinforcedKvFusionPlan::from_candidates(policy, reversed);

        assert_eq!(first.decisions, second.decisions);
        assert_eq!(first.decisions, reversed.decisions);
        assert!(first.decision_count_matches());
        assert!(first.token_accounting_matches());
        assert!(first.is_read_only_preview());
        assert!(
            first
                .score_summaries(8)
                .iter()
                .all(|summary| summary.contains("decision=")
                    && summary.contains("score=")
                    && summary.contains("components=")
                    && summary.contains("retained=")
                    && summary.contains("saved=")
                    && summary.contains("rollback="))
        );

        let required_anchor = first
            .decisions
            .iter()
            .find(|decision| decision.candidate_id == "seeded-fusion-00")
            .unwrap();

        assert!(matches!(
            required_anchor.decision,
            ReinforcedKvFusionDecision::Fuse | ReinforcedKvFusionDecision::Compress
        ));
        assert!(required_anchor.retained_tokens > 0);
        assert!(required_anchor.reason.starts_with("required_anchor_"));
    }

    fn seeded_kv_fusion_candidates(seed: u32, count: usize) -> Vec<ReinforcedKvFusionCandidate> {
        let mut state = seed;
        (0..count)
            .map(|index| {
                let source = match index % 5 {
                    0 => ReinforcedKvFusionSource::SemanticMemory,
                    1 => ReinforcedKvFusionSource::GistMemory,
                    2 => ReinforcedKvFusionSource::RuntimeKv,
                    3 => ReinforcedKvFusionSource::ColdEvidence,
                    _ => ReinforcedKvFusionSource::GenomeSegment,
                };
                let estimated_tokens = 32 + (seeded_u32(&mut state) as usize % 192);
                let reinforcement = seeded_unit(&mut state) * 2.0 - 1.0;

                ReinforcedKvFusionCandidate::new(
                    format!("seeded-fusion-{index:02}"),
                    source,
                    estimated_tokens,
                )
                .with_scores(
                    seeded_unit(&mut state),
                    seeded_unit(&mut state),
                    seeded_unit(&mut state),
                    seeded_unit(&mut state),
                    reinforcement,
                )
                .with_rollback_anchor(format!("anchor:seeded:{index:02}"))
                .with_source_hash(format!("sha256:seeded:{index:02}"))
                .with_required_anchor(index == 0)
            })
            .collect()
    }

    fn seeded_unit(state: &mut u32) -> f32 {
        (seeded_u32(state) % 1000) as f32 / 1000.0
    }

    fn seeded_u32(state: &mut u32) -> u32 {
        *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *state
    }

    fn runtime_kv_preview_with_influence(influence: f32) -> MemoryAdmissionPreview {
        runtime_kv_preview_with_influence_and_segments(influence, 1, 0, 0)
    }

    fn runtime_kv_preview_with_influence_and_segments(
        influence: f32,
        included_segments: usize,
        skipped_segments: usize,
        rejected_segments: usize,
    ) -> MemoryAdmissionPreview {
        runtime_kv_preview_with_influence_segments_and_budget(
            influence,
            included_segments,
            skipped_segments,
            rejected_segments,
            0,
        )
    }

    fn runtime_kv_preview_with_influence_segments_and_budget(
        influence: f32,
        included_segments: usize,
        skipped_segments: usize,
        rejected_segments: usize,
        budget_limited_imports_skipped: usize,
    ) -> MemoryAdmissionPreview {
        let report = ReflectionReport {
            quality: 0.82,
            contradictions: Vec::new(),
            issues: Vec::new(),
            revision_actions: Vec::new(),
            revision_passes: 0,
            revised_answer: "runtime kv answer".to_owned(),
            store_as_memory: true,
            lesson: "reuse runtime kv".to_owned(),
        };
        let reward = ProcessRewardReport {
            total: 0.84,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Reinforce,
            notes: Vec::new(),
        };
        let drift = DriftReport {
            severity: DriftSeverity::Stable,
            allow_memory_write: true,
            allow_runtime_kv_write: true,
            penalize_used_memory: false,
            rollback_adaptive: false,
            notes: Vec::new(),
        };

        MemoryAdmissionPreview::from_feedback(MemoryAdmissionInput {
            prompt: "runtime kv influence prompt",
            profile: TaskProfile::Coding,
            report: &report,
            process_reward: &reward,
            drift_report: &drift,
            stored_memory: true,
            gist_records: 0,
            stored_gist_memories: 0,
            exported_runtime_kv_blocks: 1,
            stored_runtime_kv_memories: 1,
            runtime_kv_hold: false,
            runtime_kv_influence: Some(influence),
            budget_limited_runtime_kv_imports_skipped: budget_limited_imports_skipped,
            runtime_kv_segments_included: included_segments,
            runtime_kv_segments_skipped: skipped_segments,
            runtime_kv_segments_rejected: rejected_segments,
            used_memories: 1,
            memory_feedback_updates: 0,
            runtime_adapter_observations: 0,
            runtime_adapter_current_signal: false,
            runtime_adapter_selection_mismatch: false,
            runtime_adapter_best_score: None,
            runtime_adapter_best_reward: None,
            runtime_adapter_best_quality: None,
            toolsmith_blueprints: 0,
            toolsmith_ready: 0,
            toolsmith_held: 0,
            toolsmith_rejected: 0,
            toolsmith_gate_passed: true,
        })
    }

    fn runtime_kv_fusion_decision(
        preview: &MemoryAdmissionPreview,
    ) -> &ReinforcedKvFusionDecisionRecord {
        preview
            .fusion_plan
            .decisions
            .iter()
            .find(|decision| decision.source == ReinforcedKvFusionSource::RuntimeKv)
            .expect("runtime kv fusion decision")
    }

    #[test]
    fn critical_feedback_quarantines_episode_and_holds_repair_heuristic() {
        let report = ReflectionReport {
            quality: 0.18,
            contradictions: vec!["empty_answer".to_owned()],
            issues: vec![ReflectionIssue::new(
                "empty_answer",
                ReflectionSeverity::Critical,
                "draft answer is empty",
            )],
            revision_actions: vec!["reject_empty_answer".to_owned()],
            revision_passes: 0,
            revised_answer: "[empty draft]".to_owned(),
            store_as_memory: false,
            lesson: "revise empty answer".to_owned(),
        };
        let reward = ProcessRewardReport {
            total: 0.20,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Penalize,
            notes: Vec::new(),
        };
        let drift = DriftReport {
            severity: DriftSeverity::Rollback,
            allow_memory_write: false,
            allow_runtime_kv_write: false,
            penalize_used_memory: true,
            rollback_adaptive: true,
            notes: Vec::new(),
        };

        let preview = MemoryAdmissionPreview::from_feedback(MemoryAdmissionInput {
            prompt: "bad answer",
            profile: TaskProfile::Coding,
            report: &report,
            process_reward: &reward,
            drift_report: &drift,
            stored_memory: false,
            gist_records: 0,
            stored_gist_memories: 0,
            exported_runtime_kv_blocks: 1,
            stored_runtime_kv_memories: 0,
            runtime_kv_hold: true,
            runtime_kv_influence: Some(0.12),
            budget_limited_runtime_kv_imports_skipped: 0,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 1,
            runtime_kv_segments_rejected: 0,
            used_memories: 1,
            memory_feedback_updates: 1,
            runtime_adapter_observations: 0,
            runtime_adapter_current_signal: false,
            runtime_adapter_selection_mismatch: false,
            runtime_adapter_best_score: None,
            runtime_adapter_best_reward: None,
            runtime_adapter_best_quality: None,
            toolsmith_blueprints: 0,
            toolsmith_ready: 0,
            toolsmith_held: 0,
            toolsmith_rejected: 0,
            toolsmith_gate_passed: true,
        });

        assert_eq!(preview.candidate_count(), 3);
        assert_eq!(preview.quarantine_count(), 2);
        assert_eq!(preview.hold_count(), 1);
        assert_eq!(preview.blocked_count(), 3);
        assert_eq!(preview.admitted_count(), 0);
        assert_eq!(preview.review_packet_count(), 3);
        assert!(preview.is_read_only_preview());
        assert!(
            preview
                .review_packet_summaries()
                .iter()
                .any(|summary| summary.contains("approval=held_for_evidence"))
        );
        assert!(
            preview
                .review_packet_summaries()
                .iter()
                .any(|summary| summary.contains("risk") || summary.contains("quarantine_required"))
        );
        assert!(
            preview
                .candidates
                .iter()
                .all(|candidate| candidate.rollback_anchor_id == "memory_admission:coding:stable")
        );
    }

    #[test]
    fn tool_reliability_signal_creates_approval_packet_without_runtime_payload_leak() {
        let report = ReflectionReport {
            quality: 0.78,
            contradictions: Vec::new(),
            issues: Vec::new(),
            revision_actions: Vec::new(),
            revision_passes: 0,
            revised_answer: "runtime adapter completed safely".to_owned(),
            store_as_memory: true,
            lesson: "prefer reliable runtime adapter".to_owned(),
        };
        let reward = ProcessRewardReport {
            total: 0.81,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Reinforce,
            notes: Vec::new(),
        };
        let drift = DriftReport {
            severity: DriftSeverity::Stable,
            allow_memory_write: true,
            allow_runtime_kv_write: true,
            penalize_used_memory: false,
            rollback_adaptive: false,
            notes: Vec::new(),
        };

        let preview = MemoryAdmissionPreview::from_feedback(MemoryAdmissionInput {
            prompt: "runtime adapter reliability prompt should stay private",
            profile: TaskProfile::Coding,
            report: &report,
            process_reward: &reward,
            drift_report: &drift,
            stored_memory: true,
            gist_records: 0,
            stored_gist_memories: 0,
            exported_runtime_kv_blocks: 0,
            stored_runtime_kv_memories: 0,
            runtime_kv_hold: false,
            runtime_kv_influence: None,
            budget_limited_runtime_kv_imports_skipped: 0,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            used_memories: 2,
            memory_feedback_updates: 1,
            runtime_adapter_observations: 2,
            runtime_adapter_current_signal: true,
            runtime_adapter_selection_mismatch: false,
            runtime_adapter_best_score: Some(0.82),
            runtime_adapter_best_reward: Some(0.81),
            runtime_adapter_best_quality: Some(0.78),
            toolsmith_blueprints: 1,
            toolsmith_ready: 1,
            toolsmith_held: 0,
            toolsmith_rejected: 0,
            toolsmith_gate_passed: true,
        });

        let tool_candidate = preview
            .candidates
            .iter()
            .find(|candidate| candidate.kind == MemoryAdmissionKind::ToolReliabilityObservation)
            .expect("tool reliability candidate");

        assert_eq!(preview.candidate_count(), 2);
        assert_eq!(tool_candidate.decision, MemoryAdmissionDecision::Ready);
        assert!(
            tool_candidate
                .evidence
                .iter()
                .any(|item| item == "runtime_adapter_best_score=0.820")
        );
        assert!(preview.review_packet_summaries().iter().any(|summary| {
            summary.contains("tool_reliability_observation")
                && summary.contains("approval=pending_approval")
                && summary.contains("requires_approval_gate")
        }));
        assert!(
            !preview
                .review_packet_summaries()
                .iter()
                .any(|summary| summary.contains("runtime adapter reliability prompt"))
        );
    }

    #[test]
    fn tool_reliability_conflict_is_held_for_more_evidence() {
        let report = ReflectionReport {
            quality: 0.66,
            contradictions: Vec::new(),
            issues: Vec::new(),
            revision_actions: Vec::new(),
            revision_passes: 0,
            revised_answer: "adapter mismatch needs review".to_owned(),
            store_as_memory: true,
            lesson: "review runtime adapter mismatch".to_owned(),
        };
        let reward = ProcessRewardReport {
            total: 0.67,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Hold,
            notes: Vec::new(),
        };
        let drift = DriftReport {
            severity: DriftSeverity::Watch,
            allow_memory_write: true,
            allow_runtime_kv_write: false,
            penalize_used_memory: false,
            rollback_adaptive: false,
            notes: Vec::new(),
        };

        let preview = MemoryAdmissionPreview::from_feedback(MemoryAdmissionInput {
            prompt: "tool mismatch",
            profile: TaskProfile::Coding,
            report: &report,
            process_reward: &reward,
            drift_report: &drift,
            stored_memory: false,
            gist_records: 0,
            stored_gist_memories: 0,
            exported_runtime_kv_blocks: 0,
            stored_runtime_kv_memories: 0,
            runtime_kv_hold: false,
            runtime_kv_influence: None,
            budget_limited_runtime_kv_imports_skipped: 0,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            used_memories: 1,
            memory_feedback_updates: 0,
            runtime_adapter_observations: 1,
            runtime_adapter_current_signal: true,
            runtime_adapter_selection_mismatch: true,
            runtime_adapter_best_score: Some(0.91),
            runtime_adapter_best_reward: Some(0.67),
            runtime_adapter_best_quality: Some(0.66),
            toolsmith_blueprints: 1,
            toolsmith_ready: 0,
            toolsmith_held: 0,
            toolsmith_rejected: 1,
            toolsmith_gate_passed: false,
        });

        let tool_candidate = preview
            .candidates
            .iter()
            .find(|candidate| candidate.kind == MemoryAdmissionKind::ToolReliabilityObservation)
            .expect("tool reliability candidate");

        assert_eq!(tool_candidate.decision, MemoryAdmissionDecision::Hold);
        assert_eq!(preview.hold_count(), 1);
        assert!(
            preview
                .review_packet_summaries()
                .iter()
                .any(|summary| summary.contains("approval=held_for_evidence")
                    && summary.contains("needs_more_evidence"))
        );
    }

    #[test]
    fn writer_gate_accepts_and_appends_only_after_approval() {
        let preview = ready_preview();
        let mut plan = MemoryKvLedgerWritePlan::from_preview(
            &preview,
            MemoryKvLedgerWritePolicy {
                durable_writes_enabled: true,
                operator_approved: true,
                ..MemoryKvLedgerWritePolicy::default()
            },
        );
        let path = temp_ledger_path("accepted");
        let mut store = crate::disk_kv::DiskKvStore::open(&path).unwrap();

        assert_eq!(plan.record_count(), 1);
        assert_eq!(plan.authorized_count(), 1);
        assert_eq!(
            plan.records[0].write_decision,
            MemoryKvLedgerWriteDecision::Admitted
        );

        let applied = plan.append_authorized_records(&mut store).unwrap();
        let value = store.get(&plan.records[0].ledger_key).unwrap().unwrap();
        let value = String::from_utf8(value).unwrap();

        assert_eq!(applied, 1);
        assert_eq!(plan.applied_count(), 1);
        assert!(value.contains("memory_kv_ledger_v1"));
        assert!(value.contains("decision=admitted"));
        assert!(value.contains("authorized=true"));
        assert!(value.contains("applied=true"));
        assert!(!value.contains("approved memory prompt"));
        cleanup_ledger(path);
    }

    #[test]
    fn writer_gate_refuses_missing_review_privacy_source_rollback_validation_or_operator_approval()
    {
        let mut missing_review_packet = ready_preview();
        missing_review_packet.review_packets.clear();
        let missing_review_packet_plan =
            MemoryKvLedgerWritePlan::from_preview(&missing_review_packet, approved_writer_policy());

        let mut missing_source_hash = ready_preview();
        missing_source_hash.candidates[0].source_hash.clear();
        let missing_source_hash_plan =
            MemoryKvLedgerWritePlan::from_preview(&missing_source_hash, approved_writer_policy());

        let mut missing_privacy = ready_preview();
        missing_privacy.candidates[0].privacy_checked = false;
        let privacy_plan =
            MemoryKvLedgerWritePlan::from_preview(&missing_privacy, approved_writer_policy());

        let mut missing_rollback = ready_preview();
        missing_rollback.candidates[0].rollback_anchor_id.clear();
        let rollback_plan =
            MemoryKvLedgerWritePlan::from_preview(&missing_rollback, approved_writer_policy());

        let mut missing_validation = ready_preview();
        missing_validation.candidates[0].validation_evidence.clear();
        let validation_plan =
            MemoryKvLedgerWritePlan::from_preview(&missing_validation, approved_writer_policy());

        let missing_operator_approval_plan = MemoryKvLedgerWritePlan::from_preview(
            &ready_preview(),
            MemoryKvLedgerWritePolicy {
                durable_writes_enabled: true,
                operator_approved: false,
                ..MemoryKvLedgerWritePolicy::default()
            },
        );

        for (plan, decision, marker) in [
            (
                missing_review_packet_plan,
                MemoryKvLedgerWriteDecision::Rejected,
                "review_packet_missing",
            ),
            (
                missing_source_hash_plan,
                MemoryKvLedgerWriteDecision::Rejected,
                "source_hash_missing",
            ),
            (
                privacy_plan,
                MemoryKvLedgerWriteDecision::Rejected,
                "privacy_gate_failed",
            ),
            (
                rollback_plan,
                MemoryKvLedgerWriteDecision::Rejected,
                "rollback_anchor_missing",
            ),
            (
                validation_plan,
                MemoryKvLedgerWriteDecision::Rejected,
                "validation_evidence_missing",
            ),
            (
                missing_operator_approval_plan,
                MemoryKvLedgerWriteDecision::Held,
                "operator_approval_missing",
            ),
        ] {
            assert_eq!(plan.authorized_count(), 0);
            assert_eq!(plan.records[0].write_decision, decision);
            assert!(
                plan.records[0]
                    .rejection_reasons
                    .iter()
                    .any(|reason| reason == marker),
                "{:?}",
                plan.records[0].rejection_reasons
            );
        }
    }

    #[test]
    fn writer_gate_defaults_to_preview_only_when_durable_writes_are_disabled() {
        let plan = MemoryKvLedgerWritePlan::from_preview(
            &ready_preview(),
            MemoryKvLedgerWritePolicy::default(),
        );

        assert_eq!(plan.authorized_count(), 0);
        assert_eq!(plan.applied_count(), 0);
        assert!(plan.is_read_only_preview());
        assert_eq!(
            plan.records[0].write_decision,
            MemoryKvLedgerWriteDecision::PreviewOnly
        );
        assert!(
            plan.records[0]
                .rejection_reasons
                .iter()
                .any(|reason| reason == "durable_writes_disabled"),
            "{:?}",
            plan.records[0].rejection_reasons
        );
    }

    #[test]
    fn writer_gate_classifies_held_rejected_duplicate_decayed_merged_and_rollback() {
        let held_preview = tool_conflict_preview();
        let held_plan =
            MemoryKvLedgerWritePlan::from_preview(&held_preview, approved_writer_policy());
        assert!(
            held_plan
                .records
                .iter()
                .any(|record| record.write_decision == MemoryKvLedgerWriteDecision::Held)
        );

        let rejected_preview = rejected_preview();
        let rejected_plan =
            MemoryKvLedgerWritePlan::from_preview(&rejected_preview, approved_writer_policy());
        assert!(
            rejected_plan
                .records
                .iter()
                .any(|record| record.write_decision == MemoryKvLedgerWriteDecision::Rejected)
        );

        let duplicate_preview = ready_preview();
        let duplicate_plan = MemoryKvLedgerWritePlan::from_preview(
            &duplicate_preview,
            MemoryKvLedgerWritePolicy {
                duplicate_source_hashes: vec![duplicate_preview.candidates[0].source_hash.clone()],
                ..approved_writer_policy()
            },
        );
        assert_eq!(
            duplicate_plan.records[0].write_decision,
            MemoryKvLedgerWriteDecision::Duplicate
        );

        let decayed_preview = ready_preview();
        let decayed_plan = MemoryKvLedgerWritePlan::from_preview(
            &decayed_preview,
            MemoryKvLedgerWritePolicy {
                decayed_candidate_ids: vec![decayed_preview.candidates[0].id.clone()],
                ..approved_writer_policy()
            },
        );
        assert_eq!(
            decayed_plan.records[0].write_decision,
            MemoryKvLedgerWriteDecision::Decayed
        );

        let merged_preview = ready_preview();
        let merged_plan = MemoryKvLedgerWritePlan::from_preview(
            &merged_preview,
            MemoryKvLedgerWritePolicy {
                merged_candidate_ids: vec![(
                    merged_preview.candidates[0].id.clone(),
                    "memory_admission:coding:merged".to_owned(),
                )],
                ..approved_writer_policy()
            },
        );
        assert_eq!(
            merged_plan.records[0].write_decision,
            MemoryKvLedgerWriteDecision::Merged
        );

        let rollback_preview = critical_preview();
        let rollback_plan = MemoryKvLedgerWritePlan::from_preview(
            &rollback_preview,
            MemoryKvLedgerWritePolicy {
                rollback_requested: true,
                ..approved_writer_policy()
            },
        );
        assert!(
            rollback_plan
                .records
                .iter()
                .any(|record| record.write_decision == MemoryKvLedgerWriteDecision::Rollback)
        );
    }

    fn approved_writer_policy() -> MemoryKvLedgerWritePolicy {
        MemoryKvLedgerWritePolicy {
            durable_writes_enabled: true,
            operator_approved: true,
            ..MemoryKvLedgerWritePolicy::default()
        }
    }

    fn ready_preview() -> MemoryAdmissionPreview {
        let report = ReflectionReport {
            quality: 0.86,
            contradictions: Vec::new(),
            issues: Vec::new(),
            revision_actions: Vec::new(),
            revision_passes: 0,
            revised_answer: "approved memory answer".to_owned(),
            store_as_memory: true,
            lesson: "store approved memory safely".to_owned(),
        };
        let reward = ProcessRewardReport {
            total: 0.88,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Reinforce,
            notes: Vec::new(),
        };
        let drift = stable_drift();
        preview_from_parts(
            "approved memory prompt should stay private",
            &report,
            &reward,
            &drift,
            true,
            false,
        )
    }

    fn rejected_preview() -> MemoryAdmissionPreview {
        let report = ReflectionReport {
            quality: 0.58,
            contradictions: Vec::new(),
            issues: Vec::new(),
            revision_actions: Vec::new(),
            revision_passes: 0,
            revised_answer: "do not store".to_owned(),
            store_as_memory: false,
            lesson: "rejected memory".to_owned(),
        };
        let reward = ProcessRewardReport {
            total: 0.30,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Penalize,
            notes: Vec::new(),
        };
        let drift = stable_drift();
        preview_from_parts("rejected prompt", &report, &reward, &drift, false, false)
    }

    fn tool_conflict_preview() -> MemoryAdmissionPreview {
        let report = ReflectionReport {
            quality: 0.70,
            contradictions: Vec::new(),
            issues: Vec::new(),
            revision_actions: Vec::new(),
            revision_passes: 0,
            revised_answer: "adapter mismatch needs review".to_owned(),
            store_as_memory: true,
            lesson: "review runtime adapter mismatch".to_owned(),
        };
        let reward = ProcessRewardReport {
            total: 0.66,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Hold,
            notes: Vec::new(),
        };
        let drift = stable_drift();
        preview_from_parts("held prompt", &report, &reward, &drift, false, true)
    }

    fn critical_preview() -> MemoryAdmissionPreview {
        let report = ReflectionReport {
            quality: 0.20,
            contradictions: vec!["critical".to_owned()],
            issues: vec![ReflectionIssue::new(
                "critical",
                ReflectionSeverity::Critical,
                "critical failure",
            )],
            revision_actions: vec!["rollback".to_owned()],
            revision_passes: 0,
            revised_answer: "[critical]".to_owned(),
            store_as_memory: false,
            lesson: "rollback unsafe memory".to_owned(),
        };
        let reward = ProcessRewardReport {
            total: 0.20,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Penalize,
            notes: Vec::new(),
        };
        let drift = DriftReport {
            severity: DriftSeverity::Rollback,
            allow_memory_write: false,
            allow_runtime_kv_write: false,
            penalize_used_memory: true,
            rollback_adaptive: true,
            notes: Vec::new(),
        };
        preview_from_parts("critical prompt", &report, &reward, &drift, false, false)
    }

    fn preview_from_parts(
        prompt: &str,
        report: &ReflectionReport,
        reward: &ProcessRewardReport,
        drift: &DriftReport,
        stored_memory: bool,
        tool_conflict: bool,
    ) -> MemoryAdmissionPreview {
        MemoryAdmissionPreview::from_feedback(MemoryAdmissionInput {
            prompt,
            profile: TaskProfile::Coding,
            report,
            process_reward: reward,
            drift_report: drift,
            stored_memory,
            gist_records: 0,
            stored_gist_memories: 0,
            exported_runtime_kv_blocks: 0,
            stored_runtime_kv_memories: 0,
            runtime_kv_hold: false,
            runtime_kv_influence: None,
            budget_limited_runtime_kv_imports_skipped: 0,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            used_memories: 1,
            memory_feedback_updates: 0,
            runtime_adapter_observations: usize::from(tool_conflict),
            runtime_adapter_current_signal: tool_conflict,
            runtime_adapter_selection_mismatch: tool_conflict,
            runtime_adapter_best_score: Some(0.91),
            runtime_adapter_best_reward: Some(reward.total),
            runtime_adapter_best_quality: Some(report.quality),
            toolsmith_blueprints: usize::from(tool_conflict),
            toolsmith_ready: 0,
            toolsmith_held: usize::from(tool_conflict),
            toolsmith_rejected: usize::from(tool_conflict),
            toolsmith_gate_passed: !tool_conflict,
        })
    }

    fn stable_drift() -> DriftReport {
        DriftReport {
            severity: DriftSeverity::Stable,
            allow_memory_write: true,
            allow_runtime_kv_write: true,
            penalize_used_memory: false,
            rollback_adaptive: false,
            notes: Vec::new(),
        }
    }

    fn temp_ledger_path(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-memory-ledger-{label}-{}-{nanos}.ndkv",
            std::process::id()
        ))
    }

    fn cleanup_ledger(path: std::path::PathBuf) {
        let _ = std::fs::remove_file(path);
    }
}
