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

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryAdmissionCandidate {
    pub id: String,
    pub kind: MemoryAdmissionKind,
    pub decision: MemoryAdmissionDecision,
    pub profile: TaskProfile,
    pub prompt_digest: String,
    pub prompt_chars: usize,
    pub quality: f32,
    pub process_reward: f32,
    pub critical_reflection_issues: usize,
    pub revision_actions: usize,
    pub rollback_anchor_id: String,
    pub evidence: Vec<String>,
    pub privacy_checked: bool,
    pub durable_write_authorized: bool,
    pub applied: bool,
}

impl MemoryAdmissionCandidate {
    pub fn summary(&self) -> String {
        format!(
            "{}:{}:{} q={:.3} reward={:.3} critical={} revisions={} privacy_checked={} durable_write_authorized={} applied={}",
            self.decision.as_str(),
            self.kind.as_str(),
            self.id,
            self.quality,
            self.process_reward,
            self.critical_reflection_issues,
            self.revision_actions,
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
    pub evidence: Vec<String>,
    pub risk_flags: Vec<String>,
    pub next_action: String,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl MemoryAdmissionReviewPacket {
    pub fn summary(&self) -> String {
        format!(
            "{}:{}:{} approval={} next={} risks={} evidence={} rollback={} read_only={} write_allowed={} applied={}",
            self.decision.as_str(),
            self.kind.as_str(),
            self.packet_id,
            self.approval_state.as_str(),
            self.next_action,
            self.risk_flags.join("|"),
            self.evidence.join("|"),
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
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl Default for MemoryAdmissionPreview {
    fn default() -> Self {
        Self {
            candidates: Vec::new(),
            review_packets: Vec::new(),
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
                vec![
                    format!("runtime_kv_exported={}", input.exported_runtime_kv_blocks),
                    format!(
                        "stored_runtime_kv_memories={}",
                        input.stored_runtime_kv_memories
                    ),
                    format!("runtime_kv_hold={}", input.runtime_kv_hold),
                ],
            ));
        }

        let review_packets = candidates
            .iter()
            .map(review_packet_for_candidate)
            .collect::<Vec<_>>();

        Self {
            candidates,
            review_packets,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
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
        evidence: candidate
            .evidence
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
    pub used_memories: usize,
    pub memory_feedback_updates: usize,
    pub runtime_adapter_observations: usize,
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
        prompt_chars,
        quality,
        process_reward,
        critical_reflection_issues: input.report.critical_issue_count(),
        revision_actions: input.report.revision_actions.len(),
        rollback_anchor_id: rollback_anchor_id.to_owned(),
        evidence,
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
            used_memories: 1,
            memory_feedback_updates: 0,
            runtime_adapter_observations: 0,
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
            used_memories: 1,
            memory_feedback_updates: 1,
            runtime_adapter_observations: 0,
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
            used_memories: 2,
            memory_feedback_updates: 1,
            runtime_adapter_observations: 2,
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
            used_memories: 1,
            memory_feedback_updates: 0,
            runtime_adapter_observations: 1,
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
}
