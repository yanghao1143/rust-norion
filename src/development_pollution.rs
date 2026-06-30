use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentPollutionClass {
    ActiveExon,
    InactiveIntron,
    DeadGene,
    MalignantGene,
    Quarantine,
    Archive,
    DeleteCandidate,
    Nutrient,
}

impl DevelopmentPollutionClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ActiveExon => "active_exon",
            Self::InactiveIntron => "inactive_intron",
            Self::DeadGene => "dead_gene",
            Self::MalignantGene => "malignant_gene",
            Self::Quarantine => "quarantine",
            Self::Archive => "archive",
            Self::DeleteCandidate => "delete_candidate",
            Self::Nutrient => "nutrient",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentPollutionAction {
    Keep,
    LowerRank,
    ArchiveThenCutCandidate,
    QuarantineImmediately,
    DryRunQuarantine,
    ColdStore,
    DeleteAfterProof,
    AdmitAsNutrient,
}

impl DevelopmentPollutionAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::LowerRank => "lower_rank",
            Self::ArchiveThenCutCandidate => "archive_then_cut_candidate",
            Self::QuarantineImmediately => "quarantine_immediately",
            Self::DryRunQuarantine => "dry_run_quarantine",
            Self::ColdStore => "cold_store",
            Self::DeleteAfterProof => "delete_after_proof",
            Self::AdmitAsNutrient => "admit_as_nutrient",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentHygieneState {
    Clean,
    Suspicious,
    Polluted,
    Stale,
    Unknown,
    Quarantined,
}

impl DevelopmentHygieneState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Suspicious => "suspicious",
            Self::Polluted => "polluted",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
            Self::Quarantined => "quarantined",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentNutrientTarget {
    None,
    ToolWrapper,
    SkillPlaybook,
    TrainingReplayFixture,
    CiPrGate,
    EvidencePacketTemplate,
    MemoryTombstone,
    NoNutrientValue,
}

impl DevelopmentNutrientTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ToolWrapper => "tool_wrapper",
            Self::SkillPlaybook => "skill_playbook",
            Self::TrainingReplayFixture => "training_sample_replay_fixture",
            Self::CiPrGate => "ci_or_pr_gate",
            Self::EvidencePacketTemplate => "evidence_packet_template",
            Self::MemoryTombstone => "memory_tombstone_stale_fact_marker",
            Self::NoNutrientValue => "no_nutrient_value",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefenseSpacerThreatClass {
    RetiredSource,
    PromptInjectionOrPrivatePayload,
    MalformedRuntimeManifest,
    UnsafeToolsmithBlueprint,
    PoisonedHandoffPacket,
    CrossTenantContamination,
    DevelopmentEvidenceContamination,
    ReasoningGenomeHygieneViolation,
    StaleOrPollutedClaim,
    ToolAffordanceGap,
    UnknownDevelopmentPollution,
}

impl DefenseSpacerThreatClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RetiredSource => "retired_source",
            Self::PromptInjectionOrPrivatePayload => "prompt_injection_or_private_payload",
            Self::MalformedRuntimeManifest => "malformed_runtime_manifest",
            Self::UnsafeToolsmithBlueprint => "unsafe_toolsmith_blueprint",
            Self::PoisonedHandoffPacket => "poisoned_handoff_packet",
            Self::CrossTenantContamination => "cross_tenant_contamination",
            Self::DevelopmentEvidenceContamination => "development_evidence_contamination",
            Self::ReasoningGenomeHygieneViolation => "reasoning_genome_hygiene_violation",
            Self::StaleOrPollutedClaim => "stale_or_polluted_claim",
            Self::ToolAffordanceGap => "tool_affordance_gap",
            Self::UnknownDevelopmentPollution => "unknown_development_pollution",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefenseSpacerDecision {
    Observe,
    Block,
    Quarantine,
    Expire,
    RequireReview,
}

impl DefenseSpacerDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Block => "block",
            Self::Quarantine => "quarantine",
            Self::Expire => "expire",
            Self::RequireReview => "require_review",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentPollutionEvent {
    pub event_id: String,
    pub source_kind: String,
    pub payload: String,
    pub reason_code: String,
    pub hit_count: usize,
    pub has_current_proof: bool,
    pub ttl: Option<String>,
}

impl DevelopmentPollutionEvent {
    pub fn new(
        event_id: impl Into<String>,
        source_kind: impl Into<String>,
        payload: impl Into<String>,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            source_kind: source_kind.into(),
            payload: payload.into(),
            reason_code: reason_code.into(),
            hit_count: 1,
            has_current_proof: false,
            ttl: None,
        }
    }

    pub fn with_hit_count(mut self, hit_count: usize) -> Self {
        self.hit_count = hit_count;
        self
    }

    pub fn with_current_proof(mut self, has_current_proof: bool) -> Self {
        self.has_current_proof = has_current_proof;
        self
    }

    pub fn with_ttl(mut self, ttl: impl Into<String>) -> Self {
        self.ttl = Some(ttl.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityCandidate {
    pub reason_code: String,
    pub target: DevelopmentNutrientTarget,
    pub hit_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefenseSpacer {
    pub spacer_id: String,
    pub source_event_id: String,
    pub source_digest: String,
    pub threat_class: DefenseSpacerThreatClass,
    pub matched_scope: String,
    pub blocked_payload_marker_digest: String,
    pub retired_version_marker: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub hit_count: usize,
    pub false_positive_count: usize,
    pub expiry_or_revalidation_gate: String,
    pub decision: DefenseSpacerDecision,
}

impl DefenseSpacer {
    pub fn from_finding(
        finding: &DevelopmentPollutionFinding,
        matched_scope: impl Into<String>,
        observed_at: impl Into<String>,
        expiry_or_revalidation_gate: impl Into<String>,
    ) -> Self {
        let matched_scope = matched_scope.into();
        let observed_at = observed_at.into();
        let threat_class = defense_spacer_threat_class_for_finding(finding);
        let blocked_payload_marker_digest = defense_spacer_marker_digest(
            threat_class,
            finding.source_kind.as_str(),
            finding.reason_code.as_str(),
        );
        let retired_version_marker = retired_version_marker_digest(finding.reason_code.as_str());
        let spacer_id = stable_redaction_digest([
            "defense-spacer",
            threat_class.as_str(),
            matched_scope.as_str(),
            blocked_payload_marker_digest.as_str(),
        ]);

        Self {
            spacer_id,
            source_event_id: stable_part(&finding.event_id),
            source_digest: finding.source_digest.clone(),
            threat_class,
            matched_scope,
            blocked_payload_marker_digest,
            retired_version_marker,
            first_seen_at: observed_at.clone(),
            last_seen_at: observed_at,
            hit_count: finding.hit_count,
            false_positive_count: 0,
            expiry_or_revalidation_gate: expiry_or_revalidation_gate.into(),
            decision: defense_spacer_decision_for_finding(finding, threat_class),
        }
    }

    pub fn with_false_positive_count(mut self, false_positive_count: usize) -> Self {
        self.false_positive_count = false_positive_count;
        self
    }

    pub fn effective_decision(&self) -> DefenseSpacerDecision {
        if self.false_positive_count >= self.hit_count && self.false_positive_count > 0 {
            DefenseSpacerDecision::Expire
        } else if self.false_positive_count > 0 {
            DefenseSpacerDecision::RequireReview
        } else {
            self.decision
        }
    }

    pub fn preview_match(&self, candidate: &DefenseSpacerCandidate) -> DefenseSpacerMatch {
        let marker_matched = self.blocked_payload_marker_digest == candidate.payload_marker_digest
            || self
                .retired_version_marker
                .as_ref()
                .is_some_and(|marker| Some(marker) == candidate.retired_version_marker.as_ref());
        let matched = self.threat_class == candidate.threat_class
            && self.matched_scope == candidate.matched_scope
            && marker_matched;
        let decision = if matched {
            self.effective_decision()
        } else {
            DefenseSpacerDecision::Observe
        };

        DefenseSpacerMatch {
            spacer_id: self.spacer_id.clone(),
            candidate_id: candidate.candidate_id.clone(),
            matched,
            decision,
            reason: if matched {
                "matched_defense_spacer".to_owned()
            } else {
                "no_matching_defense_spacer".to_owned()
            },
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "defense_spacer id={} source_event={} source_digest={} threat_class={} scope={} marker_digest={} retired_marker={} hits={} false_positives={} gate={} decision={} effective_decision={}",
            self.spacer_id,
            self.source_event_id,
            self.source_digest,
            self.threat_class.as_str(),
            stable_part(&self.matched_scope),
            self.blocked_payload_marker_digest,
            self.retired_version_marker.as_deref().unwrap_or("none"),
            self.hit_count,
            self.false_positive_count,
            stable_part(&self.expiry_or_revalidation_gate),
            self.decision.as_str(),
            self.effective_decision().as_str(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefenseSpacerCandidate {
    pub candidate_id: String,
    pub threat_class: DefenseSpacerThreatClass,
    pub matched_scope: String,
    pub payload_marker_digest: String,
    pub retired_version_marker: Option<String>,
}

impl DefenseSpacerCandidate {
    pub fn from_finding(
        finding: &DevelopmentPollutionFinding,
        matched_scope: impl Into<String>,
    ) -> Self {
        let threat_class = defense_spacer_threat_class_for_finding(finding);
        Self {
            candidate_id: stable_part(&finding.event_id),
            threat_class,
            matched_scope: matched_scope.into(),
            payload_marker_digest: defense_spacer_marker_digest(
                threat_class,
                finding.source_kind.as_str(),
                finding.reason_code.as_str(),
            ),
            retired_version_marker: retired_version_marker_digest(finding.reason_code.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefenseSpacerMatch {
    pub spacer_id: String,
    pub candidate_id: String,
    pub matched: bool,
    pub decision: DefenseSpacerDecision,
    pub reason: String,
}

impl DefenseSpacerMatch {
    pub fn summary_line(&self) -> String {
        format!(
            "defense_spacer_match spacer={} candidate={} matched={} decision={} reason={}",
            self.spacer_id,
            self.candidate_id,
            self.matched,
            self.decision.as_str(),
            stable_part(&self.reason),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentPollutionFinding {
    pub event_id: String,
    pub source_kind: String,
    pub source_digest: String,
    pub class: DevelopmentPollutionClass,
    pub action: DevelopmentPollutionAction,
    pub hygiene_state: DevelopmentHygieneState,
    pub nutrient_target: DevelopmentNutrientTarget,
    pub proof: String,
    pub ttl: Option<String>,
    pub reason_code: String,
    pub hit_count: usize,
    pub capability_candidate: Option<CapabilityCandidate>,
}

impl DevelopmentPollutionFinding {
    pub fn summary_line(&self) -> String {
        format!(
            "development_pollution event={} source={} digest={} class={} action={} hygiene={} nutrient_target={} proof={} ttl={} reason={} hits={} capability_candidate={}",
            stable_part(&self.event_id),
            stable_part(&self.source_kind),
            self.source_digest,
            self.class.as_str(),
            self.action.as_str(),
            self.hygiene_state.as_str(),
            self.nutrient_target.as_str(),
            self.proof,
            self.ttl.as_deref().unwrap_or("missing"),
            stable_part(&self.reason_code),
            self.hit_count,
            self.capability_candidate.is_some()
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DevelopmentPollutionReport {
    pub findings: Vec<DevelopmentPollutionFinding>,
    pub capability_candidates: Vec<CapabilityCandidate>,
}

impl DevelopmentPollutionReport {
    pub fn summary_line(&self) -> String {
        let quarantined = self
            .findings
            .iter()
            .filter(|finding| finding.hygiene_state == DevelopmentHygieneState::Quarantined)
            .count();
        let stale = self
            .findings
            .iter()
            .filter(|finding| finding.hygiene_state == DevelopmentHygieneState::Stale)
            .count();
        format!(
            "development_pollution_report findings={} quarantined={} stale={} capability_candidates={}",
            self.findings.len(),
            quarantined,
            stale,
            self.capability_candidates.len()
        )
    }
}

pub fn classify_development_pollution(
    events: &[DevelopmentPollutionEvent],
) -> DevelopmentPollutionReport {
    let findings = events
        .iter()
        .map(classify_development_pollution_event)
        .collect::<Vec<_>>();
    let capability_candidates = findings
        .iter()
        .filter_map(|finding| finding.capability_candidate.clone())
        .collect();

    DevelopmentPollutionReport {
        findings,
        capability_candidates,
    }
}

pub fn classify_development_pollution_event(
    event: &DevelopmentPollutionEvent,
) -> DevelopmentPollutionFinding {
    let reason = event.reason_code.as_str();
    let (class, action, hygiene_state, nutrient_target) =
        if is_active_reason(reason) && event.has_current_proof {
            (
                DevelopmentPollutionClass::ActiveExon,
                DevelopmentPollutionAction::Keep,
                DevelopmentHygieneState::Clean,
                DevelopmentNutrientTarget::None,
            )
        } else if is_malignant(event) {
            (
                DevelopmentPollutionClass::MalignantGene,
                DevelopmentPollutionAction::QuarantineImmediately,
                DevelopmentHygieneState::Quarantined,
                DevelopmentNutrientTarget::TrainingReplayFixture,
            )
        } else if reason.contains("development_evidence_contamination") {
            (
                DevelopmentPollutionClass::Quarantine,
                DevelopmentPollutionAction::DryRunQuarantine,
                DevelopmentHygieneState::Polluted,
                DevelopmentNutrientTarget::EvidencePacketTemplate,
            )
        } else if reason.contains("stale") || reason.contains("old") || reason.contains("retired") {
            (
                DevelopmentPollutionClass::DeadGene,
                DevelopmentPollutionAction::ArchiveThenCutCandidate,
                DevelopmentHygieneState::Stale,
                DevelopmentNutrientTarget::MemoryTombstone,
            )
        } else if reason.contains("archive") || reason.contains("release_evidence") {
            (
                DevelopmentPollutionClass::Archive,
                DevelopmentPollutionAction::ColdStore,
                DevelopmentHygieneState::Stale,
                DevelopmentNutrientTarget::None,
            )
        } else if is_tool_affordance_gap(reason) {
            (
                DevelopmentPollutionClass::Nutrient,
                DevelopmentPollutionAction::AdmitAsNutrient,
                DevelopmentHygieneState::Suspicious,
                nutrient_target_for_tool_gap(reason),
            )
        } else if reason.contains("delete") || reason.contains("reproducible_junk") {
            (
                DevelopmentPollutionClass::DeleteCandidate,
                DevelopmentPollutionAction::DeleteAfterProof,
                DevelopmentHygieneState::Suspicious,
                DevelopmentNutrientTarget::NoNutrientValue,
            )
        } else if reason.contains("inactive") || reason.contains("low_priority") {
            (
                DevelopmentPollutionClass::InactiveIntron,
                DevelopmentPollutionAction::LowerRank,
                DevelopmentHygieneState::Suspicious,
                DevelopmentNutrientTarget::SkillPlaybook,
            )
        } else {
            (
                DevelopmentPollutionClass::Quarantine,
                DevelopmentPollutionAction::DryRunQuarantine,
                DevelopmentHygieneState::Unknown,
                DevelopmentNutrientTarget::NoNutrientValue,
            )
        };

    let capability_candidate = (event.hit_count >= 2
        && nutrient_target != DevelopmentNutrientTarget::None
        && nutrient_target != DevelopmentNutrientTarget::NoNutrientValue)
        .then(|| CapabilityCandidate {
            reason_code: event.reason_code.clone(),
            target: nutrient_target,
            hit_count: event.hit_count,
        });

    DevelopmentPollutionFinding {
        event_id: event.event_id.clone(),
        source_kind: event.source_kind.clone(),
        source_digest: stable_redaction_digest([
            event.event_id.as_str(),
            event.source_kind.as_str(),
            event.reason_code.as_str(),
            event.payload.as_str(),
        ]),
        class,
        action,
        hygiene_state,
        nutrient_target,
        proof: if event.has_current_proof {
            "current_evidence".to_owned()
        } else {
            "missing".to_owned()
        },
        ttl: event.ttl.clone(),
        reason_code: event.reason_code.clone(),
        hit_count: event.hit_count,
        capability_candidate,
    }
}

fn is_active_reason(reason: &str) -> bool {
    reason.contains("current") || reason.contains("active") || reason.contains("validated")
}

fn is_malignant(event: &DevelopmentPollutionEvent) -> bool {
    let reason = event.reason_code.as_str();
    reason.contains("malignant")
        || reason.contains("injection")
        || reason.contains("polluted_claim")
        || reason.contains("raw_private_payload")
        || reason.contains("reasoning_genome_hygiene_violation")
        || contains_private_or_executable_marker(&event.payload)
}

fn is_tool_affordance_gap(reason: &str) -> bool {
    matches!(
        reason,
        "missing_discovery"
            | "missing_invocation"
            | "missing_input_template"
            | "missing_evidence"
            | "missing_cleanup"
            | "missing_explanation"
    ) || reason.contains("tool_affordance_gap")
}

fn nutrient_target_for_tool_gap(reason: &str) -> DevelopmentNutrientTarget {
    match reason {
        "missing_discovery" | "missing_invocation" => DevelopmentNutrientTarget::ToolWrapper,
        "missing_input_template" => DevelopmentNutrientTarget::EvidencePacketTemplate,
        "missing_evidence" => DevelopmentNutrientTarget::CiPrGate,
        "missing_cleanup" => DevelopmentNutrientTarget::SkillPlaybook,
        "missing_explanation" => DevelopmentNutrientTarget::TrainingReplayFixture,
        _ => DevelopmentNutrientTarget::ToolWrapper,
    }
}

fn defense_spacer_threat_class_for_finding(
    finding: &DevelopmentPollutionFinding,
) -> DefenseSpacerThreatClass {
    let reason = finding.reason_code.as_str();
    if reason.contains("retired") || reason.contains("archived_pollution_source") {
        DefenseSpacerThreatClass::RetiredSource
    } else if reason.contains("runtime_manifest") || reason.contains("sha_mismatch") {
        DefenseSpacerThreatClass::MalformedRuntimeManifest
    } else if reason.contains("unsafe_toolsmith") || reason.contains("toolsmith_blueprint") {
        DefenseSpacerThreatClass::UnsafeToolsmithBlueprint
    } else if reason.contains("poisoned_handoff") {
        DefenseSpacerThreatClass::PoisonedHandoffPacket
    } else if reason.contains("cross_tenant")
        || reason.contains("tenant_memory")
        || reason.contains("tenant_genome")
    {
        DefenseSpacerThreatClass::CrossTenantContamination
    } else if reason.contains("development_evidence_contamination") {
        DefenseSpacerThreatClass::DevelopmentEvidenceContamination
    } else if reason.contains("reasoning_genome_hygiene_violation") {
        DefenseSpacerThreatClass::ReasoningGenomeHygieneViolation
    } else if reason.contains("stale_or_polluted_claim") || reason.contains("polluted_claim") {
        DefenseSpacerThreatClass::StaleOrPollutedClaim
    } else if is_tool_affordance_gap(reason) {
        DefenseSpacerThreatClass::ToolAffordanceGap
    } else if finding.class == DevelopmentPollutionClass::MalignantGene {
        DefenseSpacerThreatClass::PromptInjectionOrPrivatePayload
    } else {
        DefenseSpacerThreatClass::UnknownDevelopmentPollution
    }
}

fn defense_spacer_decision_for_finding(
    finding: &DevelopmentPollutionFinding,
    threat_class: DefenseSpacerThreatClass,
) -> DefenseSpacerDecision {
    match threat_class {
        DefenseSpacerThreatClass::RetiredSource
        | DefenseSpacerThreatClass::MalformedRuntimeManifest
        | DefenseSpacerThreatClass::UnsafeToolsmithBlueprint
        | DefenseSpacerThreatClass::PoisonedHandoffPacket
        | DefenseSpacerThreatClass::CrossTenantContamination => DefenseSpacerDecision::Block,
        DefenseSpacerThreatClass::PromptInjectionOrPrivatePayload
        | DefenseSpacerThreatClass::DevelopmentEvidenceContamination
        | DefenseSpacerThreatClass::ReasoningGenomeHygieneViolation
        | DefenseSpacerThreatClass::StaleOrPollutedClaim => DefenseSpacerDecision::Quarantine,
        DefenseSpacerThreatClass::ToolAffordanceGap => DefenseSpacerDecision::Observe,
        DefenseSpacerThreatClass::UnknownDevelopmentPollution => {
            if finding.hygiene_state == DevelopmentHygieneState::Unknown {
                DefenseSpacerDecision::RequireReview
            } else {
                DefenseSpacerDecision::Observe
            }
        }
    }
}

fn defense_spacer_marker_digest(
    threat_class: DefenseSpacerThreatClass,
    source_kind: &str,
    reason_code: &str,
) -> String {
    stable_redaction_digest([
        "defense-spacer-marker",
        threat_class.as_str(),
        source_kind,
        reason_code,
    ])
}

fn retired_version_marker_digest(reason_code: &str) -> Option<String> {
    (reason_code.contains("retired") || reason_code.contains("archived_pollution_source"))
        .then(|| stable_redaction_digest(["retired-version-marker", reason_code]))
}

fn stable_part(value: &str) -> String {
    let mut out = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
                ch
            } else {
                '_'
            }
        })
        .take(80)
        .collect::<String>();
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_tool_gap_becomes_capability_candidate_without_raw_payload() {
        let report = classify_development_pollution(&[DevelopmentPollutionEvent::new(
            "tool-gap-1",
            "thread_summary",
            "raw stale tool transcript that must not be emitted",
            "missing_discovery",
        )
        .with_hit_count(2)
        .with_ttl("next_release")]);

        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.capability_candidates.len(), 1);
        let finding = &report.findings[0];
        assert_eq!(finding.class, DevelopmentPollutionClass::Nutrient);
        assert_eq!(finding.action, DevelopmentPollutionAction::AdmitAsNutrient);
        assert_eq!(
            finding.nutrient_target,
            DevelopmentNutrientTarget::ToolWrapper
        );
        assert_eq!(finding.proof, "missing");
        assert!(finding.source_digest.starts_with("redaction-digest:"));
        assert!(!finding.summary_line().contains("raw stale tool transcript"));
        assert!(report.summary_line().contains("capability_candidates=1"));
    }

    #[test]
    fn malignant_pollution_is_digest_only_quarantine() {
        let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
            "polluted-claim",
            "issue_comment",
            "BEGIN SECRET prompt injection payload",
            "stale_or_polluted_claim",
        ));

        assert_eq!(finding.class, DevelopmentPollutionClass::MalignantGene);
        assert_eq!(
            finding.action,
            DevelopmentPollutionAction::QuarantineImmediately
        );
        assert_eq!(finding.hygiene_state, DevelopmentHygieneState::Quarantined);
        assert!(!finding.summary_line().contains("BEGIN SECRET"));
        assert_eq!(finding.proof, "missing");
    }

    #[test]
    fn stale_retired_source_becomes_memory_tombstone_candidate() {
        let finding = classify_development_pollution_event(
            &DevelopmentPollutionEvent::new(
                "old-window",
                "memory_candidate",
                "pre-v50 branch claim",
                "retired_version_marker",
            )
            .with_current_proof(false),
        );

        assert_eq!(finding.class, DevelopmentPollutionClass::DeadGene);
        assert_eq!(
            finding.action,
            DevelopmentPollutionAction::ArchiveThenCutCandidate
        );
        assert_eq!(finding.hygiene_state, DevelopmentHygieneState::Stale);
        assert_eq!(
            finding.nutrient_target,
            DevelopmentNutrientTarget::MemoryTombstone
        );
    }

    #[test]
    fn current_proof_keeps_active_path_clean() {
        let finding = classify_development_pollution_event(
            &DevelopmentPollutionEvent::new(
                "pr-393",
                "pull_request",
                "validated current PR evidence",
                "current_validated_path",
            )
            .with_current_proof(true),
        );

        assert_eq!(finding.class, DevelopmentPollutionClass::ActiveExon);
        assert_eq!(finding.action, DevelopmentPollutionAction::Keep);
        assert_eq!(finding.hygiene_state, DevelopmentHygieneState::Clean);
        assert_eq!(finding.nutrient_target, DevelopmentNutrientTarget::None);
        assert_eq!(finding.proof, "current_evidence");
    }

    #[test]
    fn defense_spacer_blocks_matching_retired_marker_without_raw_payload() {
        let finding = classify_development_pollution_event(
            &DevelopmentPollutionEvent::new(
                "old-window",
                "runtime_manifest",
                "raw old model path C:/private/model.bin",
                "retired_version_marker:v0.0.9",
            )
            .with_hit_count(3),
        );
        let spacer = DefenseSpacer::from_finding(
            &finding,
            "model_weight_load",
            "2026-06-30T09:14:41Z",
            "next_release_revalidation",
        );
        let candidate = DefenseSpacerCandidate::from_finding(
            &classify_development_pollution_event(&DevelopmentPollutionEvent::new(
                "future-old-window",
                "runtime_manifest",
                "different raw path that must not be emitted",
                "retired_version_marker:v0.0.9",
            )),
            "model_weight_load",
        );

        let matched = spacer.preview_match(&candidate);

        assert_eq!(spacer.threat_class, DefenseSpacerThreatClass::RetiredSource);
        assert_eq!(spacer.decision, DefenseSpacerDecision::Block);
        assert!(matched.matched);
        assert_eq!(matched.decision, DefenseSpacerDecision::Block);
        assert!(spacer.retired_version_marker.is_some());
        assert!(spacer.summary_line().contains("redaction-digest:"));
        assert!(!spacer.summary_line().contains("C:/private/model.bin"));
        assert!(!matched.summary_line().contains("different raw path"));
    }

    #[test]
    fn defense_spacer_false_positive_holds_then_expires() {
        let finding = classify_development_pollution_event(
            &DevelopmentPollutionEvent::new(
                "polluted-evidence",
                "issue_comment",
                "raw polluted evidence body",
                "development_evidence_contamination",
            )
            .with_hit_count(2),
        );
        let spacer = DefenseSpacer::from_finding(
            &finding,
            "pr_body",
            "2026-06-30T09:14:41Z",
            "operator_review",
        );

        assert_eq!(
            spacer.threat_class,
            DefenseSpacerThreatClass::DevelopmentEvidenceContamination
        );
        assert_eq!(finding.hygiene_state, DevelopmentHygieneState::Polluted);
        assert_eq!(
            finding.nutrient_target,
            DevelopmentNutrientTarget::EvidencePacketTemplate
        );
        assert!(finding.capability_candidate.is_some());
        assert_eq!(spacer.decision, DefenseSpacerDecision::Quarantine);
        assert_eq!(
            spacer
                .clone()
                .with_false_positive_count(1)
                .effective_decision(),
            DefenseSpacerDecision::RequireReview
        );
        assert_eq!(
            spacer.with_false_positive_count(2).effective_decision(),
            DefenseSpacerDecision::Expire
        );
    }
}
