use crate::hierarchy::TaskProfile;
use crate::memory_admission::{
    MemoryAdmissionCandidate, MemoryAdmissionDecision, MemoryAdmissionKind, MemoryAdmissionPreview,
};

const SELF_EVOLVING_MEMORY_STORE_TRACE_SCHEMA: &str = "rust-norion-self-evolving-memory-store-v1";

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
}
