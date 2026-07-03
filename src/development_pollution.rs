use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};
use std::collections::BTreeMap;

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
pub enum DevelopmentPollutionLifecycleStage {
    Heal,
    Quarantine,
    Cut,
    Archive,
    Nutrient,
}

impl DevelopmentPollutionLifecycleStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Heal => "heal",
            Self::Quarantine => "quarantine",
            Self::Cut => "cut",
            Self::Archive => "archive",
            Self::Nutrient => "nutrient",
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
pub enum DevelopmentEvidenceAdmissionDecision {
    UseAsCurrentTruth,
    RequireLiveRevalidation,
    DigestOnlyQuarantine,
    Block,
}

impl DevelopmentEvidenceAdmissionDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UseAsCurrentTruth => "use_as_current_truth",
            Self::RequireLiveRevalidation => "require_live_revalidation",
            Self::DigestOnlyQuarantine => "digest_only_quarantine",
            Self::Block => "block",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentEvidenceUseSurface {
    Prompt,
    Trace,
    Benchmark,
    PullRequestBody,
    ExperienceRetrieval,
    DurableMemory,
    GenomeExpression,
    DigestMarker,
}

impl DevelopmentEvidenceUseSurface {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::Trace => "trace",
            Self::Benchmark => "benchmark",
            Self::PullRequestBody => "pull_request_body",
            Self::ExperienceRetrieval => "experience_retrieval",
            Self::DurableMemory => "durable_memory",
            Self::GenomeExpression => "genome_expression",
            Self::DigestMarker => "digest_marker",
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
pub struct DevelopmentEvidenceAdmission {
    pub event_id: String,
    pub source_digest: String,
    pub decision: DevelopmentEvidenceAdmissionDecision,
    pub can_use_as_current_truth: bool,
    pub can_store_digest_marker: bool,
    pub readmission_gate: String,
    pub validation_required: bool,
    pub privacy_license_required: bool,
    pub rollback_anchor_required: bool,
    pub explicit_approval_required: bool,
}

impl DevelopmentEvidenceAdmission {
    pub fn summary_line(&self) -> String {
        format!(
            "development_evidence_admission event={} digest={} decision={} current_truth={} digest_marker={} readmission_gate={} validation_required={} privacy_license_required={} rollback_anchor_required={} explicit_approval_required={}",
            stable_part(&self.event_id),
            self.source_digest,
            self.decision.as_str(),
            self.can_use_as_current_truth,
            self.can_store_digest_marker,
            stable_part(&self.readmission_gate),
            self.validation_required,
            self.privacy_license_required,
            self.rollback_anchor_required,
            self.explicit_approval_required,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentEvidenceSurfaceGate {
    pub event_id: String,
    pub source_digest: String,
    pub surface: DevelopmentEvidenceUseSurface,
    pub decision: DevelopmentEvidenceAdmissionDecision,
    pub allowed: bool,
    pub reason: String,
}

impl DevelopmentEvidenceSurfaceGate {
    pub fn summary_line(&self) -> String {
        format!(
            "development_evidence_surface_gate event={} digest={} surface={} decision={} allowed={} reason={}",
            stable_part(&self.event_id),
            self.source_digest,
            self.surface.as_str(),
            self.decision.as_str(),
            self.allowed,
            stable_part(&self.reason),
        )
    }
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
pub struct DefenseSpacerActivationGate {
    pub candidate_id: String,
    pub matched_spacer_id: Option<String>,
    pub matched_scope: String,
    pub decision: DefenseSpacerDecision,
    pub allowed: bool,
    pub reason: String,
}

impl DefenseSpacerActivationGate {
    pub fn summary_line(&self) -> String {
        format!(
            "defense_spacer_activation_gate candidate={} spacer={} scope={} decision={} allowed={} reason={}",
            self.candidate_id,
            self.matched_spacer_id.as_deref().unwrap_or("none"),
            stable_part(&self.matched_scope),
            self.decision.as_str(),
            self.allowed,
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
    pub lifecycle_stage: DevelopmentPollutionLifecycleStage,
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
            "development_pollution event={} source={} digest={} class={} action={} lifecycle={} hygiene={} nutrient_target={} proof={} ttl={} reason={} hits={} capability_candidate={}",
            stable_part(&self.event_id),
            stable_part(&self.source_kind),
            self.source_digest,
            self.class.as_str(),
            self.action.as_str(),
            self.lifecycle_stage.as_str(),
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
        let heal = self.lifecycle_count(DevelopmentPollutionLifecycleStage::Heal);
        let quarantine = self.lifecycle_count(DevelopmentPollutionLifecycleStage::Quarantine);
        let cut = self.lifecycle_count(DevelopmentPollutionLifecycleStage::Cut);
        let archive = self.lifecycle_count(DevelopmentPollutionLifecycleStage::Archive);
        let nutrient = self.lifecycle_count(DevelopmentPollutionLifecycleStage::Nutrient);
        format!(
            "development_pollution_report findings={} quarantined={} stale={} lifecycle_heal={} lifecycle_quarantine={} lifecycle_cut={} lifecycle_archive={} lifecycle_nutrient={} capability_candidates={}",
            self.findings.len(),
            quarantined,
            stale,
            heal,
            quarantine,
            cut,
            archive,
            nutrient,
            self.capability_candidates.len()
        )
    }

    fn lifecycle_count(&self, stage: DevelopmentPollutionLifecycleStage) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.lifecycle_stage == stage)
            .count()
    }
}

pub fn classify_development_pollution(
    events: &[DevelopmentPollutionEvent],
) -> DevelopmentPollutionReport {
    let findings = events
        .iter()
        .map(classify_development_pollution_event)
        .collect::<Vec<_>>();
    let capability_candidates = aggregate_capability_candidates(&findings);

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
                DevelopmentNutrientTarget::EvidencePacketTemplate,
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
        lifecycle_stage: lifecycle_stage_for_action(action),
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

pub fn admit_development_evidence_for_current_use(
    finding: &DevelopmentPollutionFinding,
) -> DevelopmentEvidenceAdmission {
    let (decision, readmission_gate) = if finding.class == DevelopmentPollutionClass::ActiveExon
        && finding.hygiene_state == DevelopmentHygieneState::Clean
        && finding.proof == "current_evidence"
    {
        (
            DevelopmentEvidenceAdmissionDecision::UseAsCurrentTruth,
            "current_evidence_present",
        )
    } else if finding.class == DevelopmentPollutionClass::MalignantGene
        || matches!(
            finding.hygiene_state,
            DevelopmentHygieneState::Polluted | DevelopmentHygieneState::Quarantined
        )
    {
        (
            DevelopmentEvidenceAdmissionDecision::DigestOnlyQuarantine,
            "validation_privacy_license_rollback_and_explicit_approval",
        )
    } else if matches!(
        finding.class,
        DevelopmentPollutionClass::DeleteCandidate | DevelopmentPollutionClass::DeadGene
    ) && finding.nutrient_target == DevelopmentNutrientTarget::NoNutrientValue
    {
        (
            DevelopmentEvidenceAdmissionDecision::Block,
            "proof_required_before_cut_or_delete",
        )
    } else {
        (
            DevelopmentEvidenceAdmissionDecision::RequireLiveRevalidation,
            "live_git_source_test_or_runtime_evidence_required",
        )
    };

    let quarantine_readmission =
        decision == DevelopmentEvidenceAdmissionDecision::DigestOnlyQuarantine;
    DevelopmentEvidenceAdmission {
        event_id: finding.event_id.clone(),
        source_digest: finding.source_digest.clone(),
        decision,
        can_use_as_current_truth: decision
            == DevelopmentEvidenceAdmissionDecision::UseAsCurrentTruth,
        can_store_digest_marker: decision != DevelopmentEvidenceAdmissionDecision::Block,
        readmission_gate: readmission_gate.to_owned(),
        validation_required: decision != DevelopmentEvidenceAdmissionDecision::UseAsCurrentTruth,
        privacy_license_required: quarantine_readmission,
        rollback_anchor_required: quarantine_readmission,
        explicit_approval_required: quarantine_readmission,
    }
}

pub fn gate_development_evidence_surface(
    admission: &DevelopmentEvidenceAdmission,
    surface: DevelopmentEvidenceUseSurface,
) -> DevelopmentEvidenceSurfaceGate {
    let (allowed, reason) = if surface == DevelopmentEvidenceUseSurface::DigestMarker {
        if admission.can_store_digest_marker {
            (true, "digest_marker_allowed")
        } else {
            (false, "digest_marker_blocked")
        }
    } else if admission.can_use_as_current_truth {
        (true, "current_truth_allowed")
    } else {
        (
            false,
            match admission.decision {
                DevelopmentEvidenceAdmissionDecision::UseAsCurrentTruth => {
                    "current_truth_not_available"
                }
                DevelopmentEvidenceAdmissionDecision::RequireLiveRevalidation => {
                    "live_revalidation_required"
                }
                DevelopmentEvidenceAdmissionDecision::DigestOnlyQuarantine => {
                    "digest_only_quarantine_required"
                }
                DevelopmentEvidenceAdmissionDecision::Block => "blocked",
            },
        )
    };

    DevelopmentEvidenceSurfaceGate {
        event_id: admission.event_id.clone(),
        source_digest: admission.source_digest.clone(),
        surface,
        decision: admission.decision,
        allowed,
        reason: reason.to_owned(),
    }
}

pub fn gate_development_evidence_payload_surface(
    event_id: impl Into<String>,
    source_kind: impl Into<String>,
    payload: impl Into<String>,
    surface: DevelopmentEvidenceUseSurface,
) -> DevelopmentEvidenceSurfaceGate {
    let payload = payload.into();
    let reason = development_evidence_payload_reason(&payload);
    let mut event = DevelopmentPollutionEvent::new(event_id, source_kind, payload, reason);
    if reason == "current_validated_path" {
        event = event.with_current_proof(true);
    }
    let finding = classify_development_pollution_event(&event);
    let admission = admit_development_evidence_for_current_use(&finding);
    gate_development_evidence_surface(&admission, surface)
}

pub fn gate_defense_spacer_activation(
    spacers: &[DefenseSpacer],
    candidate: &DefenseSpacerCandidate,
) -> DefenseSpacerActivationGate {
    let mut non_blocking_match = None;
    for spacer in spacers {
        let spacer_match = spacer.preview_match(candidate);
        if !spacer_match.matched {
            continue;
        }

        match spacer_match.decision {
            DefenseSpacerDecision::Block => {
                return defense_spacer_activation_gate_from_match(
                    candidate,
                    &spacer_match,
                    false,
                    "matched_blocking_defense_spacer",
                );
            }
            DefenseSpacerDecision::Quarantine => {
                return defense_spacer_activation_gate_from_match(
                    candidate,
                    &spacer_match,
                    false,
                    "matched_quarantine_defense_spacer",
                );
            }
            DefenseSpacerDecision::RequireReview => {
                return defense_spacer_activation_gate_from_match(
                    candidate,
                    &spacer_match,
                    false,
                    "matched_requires_review_defense_spacer",
                );
            }
            DefenseSpacerDecision::Expire | DefenseSpacerDecision::Observe => {
                non_blocking_match.get_or_insert(spacer_match);
            }
        }
    }

    if let Some(spacer_match) = non_blocking_match {
        let reason = if spacer_match.decision == DefenseSpacerDecision::Expire {
            "matched_expired_defense_spacer"
        } else {
            "matched_observe_defense_spacer"
        };
        defense_spacer_activation_gate_from_match(candidate, &spacer_match, true, reason)
    } else {
        DefenseSpacerActivationGate {
            candidate_id: candidate.candidate_id.clone(),
            matched_spacer_id: None,
            matched_scope: candidate.matched_scope.clone(),
            decision: DefenseSpacerDecision::Observe,
            allowed: true,
            reason: "no_matching_defense_spacer".to_owned(),
        }
    }
}

fn defense_spacer_activation_gate_from_match(
    candidate: &DefenseSpacerCandidate,
    spacer_match: &DefenseSpacerMatch,
    allowed: bool,
    reason: &str,
) -> DefenseSpacerActivationGate {
    DefenseSpacerActivationGate {
        candidate_id: candidate.candidate_id.clone(),
        matched_spacer_id: Some(spacer_match.spacer_id.clone()),
        matched_scope: candidate.matched_scope.clone(),
        decision: spacer_match.decision,
        allowed,
        reason: reason.to_owned(),
    }
}

fn aggregate_capability_candidates(
    findings: &[DevelopmentPollutionFinding],
) -> Vec<CapabilityCandidate> {
    let mut totals = BTreeMap::<String, (DevelopmentNutrientTarget, usize)>::new();
    for finding in findings {
        if !has_nutrient_value(finding.nutrient_target) {
            continue;
        }
        let entry = totals
            .entry(finding.reason_code.clone())
            .or_insert((finding.nutrient_target, 0));
        entry.1 = entry.1.saturating_add(finding.hit_count);
    }

    totals
        .into_iter()
        .filter_map(|(reason_code, (target, hit_count))| {
            (hit_count >= 2).then_some(CapabilityCandidate {
                reason_code,
                target,
                hit_count,
            })
        })
        .collect()
}

fn has_nutrient_value(target: DevelopmentNutrientTarget) -> bool {
    !matches!(
        target,
        DevelopmentNutrientTarget::None | DevelopmentNutrientTarget::NoNutrientValue
    )
}

fn lifecycle_stage_for_action(
    action: DevelopmentPollutionAction,
) -> DevelopmentPollutionLifecycleStage {
    match action {
        DevelopmentPollutionAction::Keep => DevelopmentPollutionLifecycleStage::Heal,
        DevelopmentPollutionAction::LowerRank | DevelopmentPollutionAction::ColdStore => {
            DevelopmentPollutionLifecycleStage::Archive
        }
        DevelopmentPollutionAction::ArchiveThenCutCandidate
        | DevelopmentPollutionAction::DeleteAfterProof => DevelopmentPollutionLifecycleStage::Cut,
        DevelopmentPollutionAction::QuarantineImmediately
        | DevelopmentPollutionAction::DryRunQuarantine => {
            DevelopmentPollutionLifecycleStage::Quarantine
        }
        DevelopmentPollutionAction::AdmitAsNutrient => DevelopmentPollutionLifecycleStage::Nutrient,
    }
}

fn is_active_reason(reason: &str) -> bool {
    reason.contains("current") || reason.contains("active") || reason.contains("validated")
}

pub(crate) fn development_evidence_payload_reason(payload: &str) -> &'static str {
    if contains_private_or_executable_marker(payload) {
        return "prompt_injection_marker";
    }

    let lower = payload.to_ascii_lowercase();
    if lower.contains("begin secret") {
        "prompt_injection_marker"
    } else if lower.contains("reasoning_genome_hygiene_violation") {
        "reasoning_genome_hygiene_violation"
    } else if lower.contains("development_evidence_contamination") {
        "development_evidence_contamination"
    } else if lower.contains("stale_or_polluted_claim") || lower.contains("polluted_claim") {
        "stale_or_polluted_claim"
    } else if lower.contains("retired_version_marker")
        || lower.contains("archived_pollution_source")
    {
        "retired_version_marker"
    } else if lower.contains("runtime_manifest") || lower.contains("sha_mismatch") {
        "runtime_manifest_sha_mismatch"
    } else if lower.contains("poisoned_handoff") {
        "poisoned_handoff"
    } else if lower.contains("unsafe_toolsmith") || lower.contains("toolsmith_blueprint") {
        "unsafe_toolsmith_blueprint"
    } else if lower.contains("cross_tenant") {
        "cross_tenant_memory_or_genome"
    } else {
        "current_validated_path"
    }
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
    fn repeated_same_reason_events_become_one_capability_candidate() {
        let report = classify_development_pollution(&[
            DevelopmentPollutionEvent::new(
                "tool-gap-1",
                "thread_summary",
                "raw transcript one",
                "missing_discovery",
            ),
            DevelopmentPollutionEvent::new(
                "tool-gap-2",
                "issue_comment",
                "raw transcript two",
                "missing_discovery",
            ),
        ]);

        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.capability_candidates.len(), 1);
        let candidate = &report.capability_candidates[0];
        assert_eq!(candidate.reason_code, "missing_discovery");
        assert_eq!(candidate.target, DevelopmentNutrientTarget::ToolWrapper);
        assert_eq!(candidate.hit_count, 2);
        assert!(
            !report.findings[0]
                .summary_line()
                .contains("raw transcript one")
        );
        assert!(
            !report.findings[1]
                .summary_line()
                .contains("raw transcript two")
        );
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
    fn repeated_archive_pollution_becomes_evidence_packet_candidate() {
        let report = classify_development_pollution(&[DevelopmentPollutionEvent::new(
            "release-scar",
            "issue_comment",
            "old release evidence body must stay cold",
            "release_evidence_archive",
        )
        .with_hit_count(2)]);

        assert_eq!(report.capability_candidates.len(), 1);
        let finding = &report.findings[0];
        assert_eq!(finding.class, DevelopmentPollutionClass::Archive);
        assert_eq!(finding.action, DevelopmentPollutionAction::ColdStore);
        assert_eq!(
            finding.nutrient_target,
            DevelopmentNutrientTarget::EvidencePacketTemplate
        );
        assert!(finding.capability_candidate.is_some());
        assert!(!finding.summary_line().contains("old release evidence body"));
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
    fn only_clean_current_evidence_can_be_current_truth() {
        let finding = classify_development_pollution_event(
            &DevelopmentPollutionEvent::new(
                "pr-399",
                "pull_request",
                "validated current PR evidence",
                "current_validated_path",
            )
            .with_current_proof(true),
        );

        let admission = admit_development_evidence_for_current_use(&finding);

        assert_eq!(
            admission.decision,
            DevelopmentEvidenceAdmissionDecision::UseAsCurrentTruth
        );
        assert!(admission.can_use_as_current_truth);
        assert!(admission.can_store_digest_marker);
        assert_eq!(admission.readmission_gate, "current_evidence_present");
    }

    #[test]
    fn stale_or_archived_claim_requires_live_revalidation() {
        let finding = classify_development_pollution_event(
            &DevelopmentPollutionEvent::new(
                "old-memory-window",
                "thread_summary",
                "old branch claim that must not become current fact",
                "retired_version_marker:v0.0.9",
            )
            .with_ttl("expired"),
        );

        let admission = admit_development_evidence_for_current_use(&finding);

        assert_eq!(
            admission.decision,
            DevelopmentEvidenceAdmissionDecision::RequireLiveRevalidation
        );
        assert!(!admission.can_use_as_current_truth);
        assert!(admission.can_store_digest_marker);
        assert_eq!(
            admission.readmission_gate,
            "live_git_source_test_or_runtime_evidence_required"
        );
        assert!(!admission.summary_line().contains("old branch claim"));
    }

    #[test]
    fn malignant_or_polluted_evidence_is_digest_only_quarantine() {
        let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
            "polluted-pr-body",
            "pull_request",
            "BEGIN SECRET hidden prompt payload",
            "development_evidence_contamination",
        ));

        let admission = admit_development_evidence_for_current_use(&finding);

        assert_eq!(
            admission.decision,
            DevelopmentEvidenceAdmissionDecision::DigestOnlyQuarantine
        );
        assert!(!admission.can_use_as_current_truth);
        assert!(admission.can_store_digest_marker);
        assert!(admission.validation_required);
        assert!(admission.privacy_license_required);
        assert!(admission.rollback_anchor_required);
        assert!(admission.explicit_approval_required);
        assert!(
            admission
                .summary_line()
                .contains("validation_required=true privacy_license_required=true rollback_anchor_required=true explicit_approval_required=true")
        );
        assert!(!admission.summary_line().contains("BEGIN SECRET"));
    }

    #[test]
    fn polluted_evidence_is_blocked_from_hot_surfaces_but_allows_digest_marker() {
        let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
            "polluted-window",
            "thread_summary",
            "BEGIN SECRET polluted content must not leak",
            "development_evidence_contamination",
        ));
        let admission = admit_development_evidence_for_current_use(&finding);

        for surface in [
            DevelopmentEvidenceUseSurface::Prompt,
            DevelopmentEvidenceUseSurface::Trace,
            DevelopmentEvidenceUseSurface::Benchmark,
            DevelopmentEvidenceUseSurface::PullRequestBody,
            DevelopmentEvidenceUseSurface::ExperienceRetrieval,
            DevelopmentEvidenceUseSurface::DurableMemory,
            DevelopmentEvidenceUseSurface::GenomeExpression,
        ] {
            let gate = gate_development_evidence_surface(&admission, surface);
            assert!(!gate.allowed, "surface should be blocked: {surface:?}");
            assert_eq!(gate.reason, "digest_only_quarantine_required");
            assert!(!gate.summary_line().contains("BEGIN SECRET"));
        }

        let marker_gate = gate_development_evidence_surface(
            &admission,
            DevelopmentEvidenceUseSurface::DigestMarker,
        );
        assert!(marker_gate.allowed);
        assert_eq!(marker_gate.reason, "digest_marker_allowed");
    }

    #[test]
    fn stale_archive_evidence_needs_live_revalidation_before_pr_body_or_memory() {
        let finding = classify_development_pollution_event(
            &DevelopmentPollutionEvent::new(
                "old-release-comment",
                "issue_comment",
                "old release note body",
                "release_evidence_archive",
            )
            .with_hit_count(2),
        );
        let admission = admit_development_evidence_for_current_use(&finding);

        for surface in [
            DevelopmentEvidenceUseSurface::PullRequestBody,
            DevelopmentEvidenceUseSurface::DurableMemory,
        ] {
            let gate = gate_development_evidence_surface(&admission, surface);
            assert!(!gate.allowed);
            assert_eq!(gate.reason, "live_revalidation_required");
            assert!(!gate.summary_line().contains("old release note body"));
        }
    }

    #[test]
    fn clean_current_evidence_can_use_hot_surfaces() {
        let finding = classify_development_pollution_event(
            &DevelopmentPollutionEvent::new(
                "current-pr",
                "pull_request",
                "current validated evidence",
                "current_validated_path",
            )
            .with_current_proof(true),
        );
        let admission = admit_development_evidence_for_current_use(&finding);

        let prompt_gate =
            gate_development_evidence_surface(&admission, DevelopmentEvidenceUseSurface::Prompt);
        assert!(prompt_gate.allowed);
        assert_eq!(prompt_gate.reason, "current_truth_allowed");
        assert!(!admission.validation_required);
        assert!(!admission.privacy_license_required);
        assert!(!admission.rollback_anchor_required);
        assert!(!admission.explicit_approval_required);
    }

    #[test]
    fn payload_surface_gate_blocks_polluted_marker_without_raw_text() {
        let gate = gate_development_evidence_payload_surface(
            "trace-prompt",
            "trace_prompt",
            "BEGIN SECRET polluted prompt must not leak",
            DevelopmentEvidenceUseSurface::Trace,
        );

        assert!(!gate.allowed);
        assert_eq!(
            gate.decision,
            DevelopmentEvidenceAdmissionDecision::DigestOnlyQuarantine
        );
        assert_eq!(gate.reason, "digest_only_quarantine_required");
        assert!(gate.source_digest.starts_with("redaction-digest:"));
        assert!(!gate.summary_line().contains("BEGIN SECRET"));
    }

    #[test]
    fn payload_surface_gate_allows_current_clean_payload() {
        let gate = gate_development_evidence_payload_surface(
            "benchmark-clean",
            "benchmark_case",
            "current validated benchmark prompt",
            DevelopmentEvidenceUseSurface::Benchmark,
        );

        assert!(gate.allowed);
        assert_eq!(
            gate.decision,
            DevelopmentEvidenceAdmissionDecision::UseAsCurrentTruth
        );
        assert_eq!(gate.reason, "current_truth_allowed");
    }

    #[test]
    fn dirty_path_and_output_artifact_sources_are_digest_only_findings() {
        let report = classify_development_pollution(&[
            DevelopmentPollutionEvent::new(
                "dirty-script",
                "dirty_path",
                "tools/smartsteam-forge/scripts/status-forge.ps1 raw diff",
                "missing_cleanup",
            )
            .with_ttl("next_release"),
            DevelopmentPollutionEvent::new(
                "generated-output",
                "output_artifact",
                "output/tmp/generated packet body",
                "reproducible_junk",
            ),
        ]);

        let dirty_path = &report.findings[0];
        assert_eq!(dirty_path.source_kind, "dirty_path");
        assert_eq!(dirty_path.class, DevelopmentPollutionClass::Nutrient);
        assert_eq!(
            dirty_path.action,
            DevelopmentPollutionAction::AdmitAsNutrient
        );
        assert_eq!(
            dirty_path.lifecycle_stage,
            DevelopmentPollutionLifecycleStage::Nutrient
        );
        assert_eq!(
            dirty_path.nutrient_target,
            DevelopmentNutrientTarget::SkillPlaybook
        );
        assert_eq!(dirty_path.proof, "missing");
        assert_eq!(dirty_path.ttl.as_deref(), Some("next_release"));
        assert!(
            !dirty_path
                .summary_line()
                .contains("status-forge.ps1 raw diff")
        );

        let output_artifact = &report.findings[1];
        assert_eq!(output_artifact.source_kind, "output_artifact");
        assert_eq!(
            output_artifact.class,
            DevelopmentPollutionClass::DeleteCandidate
        );
        assert_eq!(
            output_artifact.action,
            DevelopmentPollutionAction::DeleteAfterProof
        );
        assert_eq!(
            output_artifact.lifecycle_stage,
            DevelopmentPollutionLifecycleStage::Cut
        );
        assert_eq!(
            output_artifact.nutrient_target,
            DevelopmentNutrientTarget::NoNutrientValue
        );
        assert_eq!(output_artifact.proof, "missing");
        assert!(
            !output_artifact
                .summary_line()
                .contains("generated packet body")
        );
        assert!(report.summary_line().contains("lifecycle_cut=1"));
        assert!(report.summary_line().contains("lifecycle_nutrient=1"));
    }

    #[test]
    fn report_counts_all_lifecycle_stages() {
        let report = classify_development_pollution(&[
            DevelopmentPollutionEvent::new(
                "current-pr",
                "pull_request",
                "current validated body",
                "current_validated_path",
            )
            .with_current_proof(true),
            DevelopmentPollutionEvent::new(
                "polluted-comment",
                "issue_comment",
                "polluted body",
                "development_evidence_contamination",
            ),
            DevelopmentPollutionEvent::new(
                "old-window",
                "thread_summary",
                "old body",
                "retired_version_marker:v0.0.9",
            ),
            DevelopmentPollutionEvent::new(
                "release-scar",
                "issue_comment",
                "release evidence body",
                "release_evidence_archive",
            ),
            DevelopmentPollutionEvent::new(
                "tool-gap",
                "thread_summary",
                "tool gap body",
                "missing_discovery",
            )
            .with_hit_count(2),
        ]);

        let summary = report.summary_line();

        assert!(summary.contains("lifecycle_heal=1"));
        assert!(summary.contains("lifecycle_quarantine=1"));
        assert!(summary.contains("lifecycle_cut=1"));
        assert!(summary.contains("lifecycle_archive=1"));
        assert!(summary.contains("lifecycle_nutrient=1"));
        assert!(summary.contains("capability_candidates=1"));
    }

    #[test]
    fn defense_spacer_first_tool_gap_observes_without_promotion() {
        let report = classify_development_pollution(&[DevelopmentPollutionEvent::new(
            "tool-gap-first",
            "thread_summary",
            "raw transcript body must stay out of spacer output",
            "missing_discovery",
        )]);
        let finding = &report.findings[0];
        let spacer = DefenseSpacer::from_finding(
            finding,
            "tool_selection",
            "2026-06-30T09:14:41Z",
            "repeat_count_ge_2",
        );

        assert!(report.capability_candidates.is_empty());
        assert_eq!(finding.hit_count, 1);
        assert!(finding.capability_candidate.is_none());
        assert_eq!(spacer.first_seen_at, "2026-06-30T09:14:41Z");
        assert_eq!(spacer.last_seen_at, spacer.first_seen_at);
        assert_eq!(spacer.hit_count, 1);
        assert_eq!(spacer.false_positive_count, 0);
        assert_eq!(
            spacer.threat_class,
            DefenseSpacerThreatClass::ToolAffordanceGap
        );
        assert_eq!(spacer.decision, DefenseSpacerDecision::Observe);
        assert_eq!(spacer.effective_decision(), DefenseSpacerDecision::Observe);
        assert!(!spacer.summary_line().contains("raw transcript body"));
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
    fn defense_spacer_quarantines_repeated_prompt_injection_marker() {
        let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
            "prompt-injection-1",
            "issue_comment",
            "BEGIN SECRET first prompt injection payload",
            "prompt_injection_marker",
        ));
        let spacer = DefenseSpacer::from_finding(
            &finding,
            "prompt",
            "2026-06-30T09:14:41Z",
            "operator_review",
        );
        let candidate = DefenseSpacerCandidate::from_finding(
            &classify_development_pollution_event(&DevelopmentPollutionEvent::new(
                "prompt-injection-2",
                "issue_comment",
                "BEGIN SECRET second prompt injection payload",
                "prompt_injection_marker",
            )),
            "prompt",
        );
        let second_candidate = DefenseSpacerCandidate::from_finding(
            &classify_development_pollution_event(&DevelopmentPollutionEvent::new(
                "prompt-injection-3",
                "issue_comment",
                "BEGIN SECRET third prompt injection payload",
                "prompt_injection_marker",
            )),
            "prompt",
        );

        let matched = spacer.preview_match(&candidate);
        let second_matched = spacer.preview_match(&second_candidate);

        assert_eq!(
            spacer.threat_class,
            DefenseSpacerThreatClass::PromptInjectionOrPrivatePayload
        );
        assert_eq!(finding.class, DevelopmentPollutionClass::MalignantGene);
        assert_eq!(spacer.decision, DefenseSpacerDecision::Quarantine);
        assert!(matched.matched);
        assert_eq!(matched.decision, DefenseSpacerDecision::Quarantine);
        assert!(second_matched.matched);
        assert_eq!(second_matched.decision, DefenseSpacerDecision::Quarantine);
        assert!(!spacer.summary_line().contains("BEGIN SECRET"));
        assert!(
            !matched
                .summary_line()
                .contains("second prompt injection payload")
        );
        assert!(
            !second_matched
                .summary_line()
                .contains("third prompt injection payload")
        );
    }

    #[test]
    fn defense_spacer_unknown_pollution_requires_operator_review() {
        let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
            "unknown-pollution",
            "runtime_manifest",
            "raw unknown manifest body",
            "unmapped_pollution_signal",
        ));
        let spacer = DefenseSpacer::from_finding(
            &finding,
            "process_start",
            "2026-06-30T09:14:41Z",
            "operator_review",
        );
        let candidate = DefenseSpacerCandidate::from_finding(
            &classify_development_pollution_event(&DevelopmentPollutionEvent::new(
                "unknown-pollution-again",
                "runtime_manifest",
                "different raw unknown manifest body",
                "unmapped_pollution_signal",
            )),
            "process_start",
        );

        let matched = spacer.preview_match(&candidate);

        assert_eq!(
            spacer.threat_class,
            DefenseSpacerThreatClass::UnknownDevelopmentPollution
        );
        assert_eq!(finding.hygiene_state, DevelopmentHygieneState::Unknown);
        assert_eq!(spacer.decision, DefenseSpacerDecision::RequireReview);
        assert!(matched.matched);
        assert_eq!(matched.decision, DefenseSpacerDecision::RequireReview);
        assert!(!spacer.summary_line().contains("raw unknown manifest body"));
        assert!(
            !matched
                .summary_line()
                .contains("different raw unknown manifest body")
        );
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
        let candidate = DefenseSpacerCandidate::from_finding(
            &classify_development_pollution_event(&DevelopmentPollutionEvent::new(
                "polluted-evidence-again",
                "issue_comment",
                "different raw polluted evidence body",
                "development_evidence_contamination",
            )),
            "pr_body",
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
        let held_match = spacer
            .clone()
            .with_false_positive_count(1)
            .preview_match(&candidate);
        assert!(held_match.matched);
        assert_eq!(held_match.decision, DefenseSpacerDecision::RequireReview);
        assert_eq!(
            spacer.with_false_positive_count(2).effective_decision(),
            DefenseSpacerDecision::Expire
        );
        assert!(!held_match.summary_line().contains("different raw polluted"));
    }

    #[test]
    fn defense_spacer_activation_gate_blocks_retired_version_before_model_weight_load() {
        let finding = classify_development_pollution_event(
            &DevelopmentPollutionEvent::new(
                "old-runtime",
                "runtime_manifest",
                "raw old model path C:/private/model.bin",
                "retired_version_marker:v0.0.9",
            )
            .with_hit_count(2),
        );
        let spacer = DefenseSpacer::from_finding(
            &finding,
            "model_weight_load",
            "2026-06-30T09:14:41Z",
            "next_release_revalidation",
        );
        let candidate = DefenseSpacerCandidate::from_finding(
            &classify_development_pollution_event(&DevelopmentPollutionEvent::new(
                "future-old-runtime",
                "runtime_manifest",
                "raw future model path C:/private/old.bin",
                "retired_version_marker:v0.0.9",
            )),
            "model_weight_load",
        );

        let gate = gate_defense_spacer_activation(&[spacer], &candidate);

        assert!(!gate.allowed);
        assert_eq!(gate.decision, DefenseSpacerDecision::Block);
        assert_eq!(gate.reason, "matched_blocking_defense_spacer");
        assert!(gate.matched_spacer_id.is_some());
        assert!(!gate.summary_line().contains("C:/private"));
    }

    #[test]
    fn defense_spacer_activation_gate_requires_review_before_process_start() {
        let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
            "unknown-pollution",
            "runtime_manifest",
            "raw unknown manifest body",
            "unmapped_pollution_signal",
        ));
        let spacer = DefenseSpacer::from_finding(
            &finding,
            "process_start",
            "2026-06-30T09:14:41Z",
            "operator_review",
        );
        let candidate = DefenseSpacerCandidate::from_finding(
            &classify_development_pollution_event(&DevelopmentPollutionEvent::new(
                "unknown-pollution-again",
                "runtime_manifest",
                "different raw unknown manifest body",
                "unmapped_pollution_signal",
            )),
            "process_start",
        );

        let gate = gate_defense_spacer_activation(&[spacer], &candidate);

        assert!(!gate.allowed);
        assert_eq!(gate.decision, DefenseSpacerDecision::RequireReview);
        assert_eq!(gate.reason, "matched_requires_review_defense_spacer");
        assert!(!gate.summary_line().contains("raw unknown"));
    }

    #[test]
    fn defense_spacer_activation_gate_allows_expired_false_positive() {
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
        )
        .with_false_positive_count(2);
        let candidate = DefenseSpacerCandidate::from_finding(
            &classify_development_pollution_event(&DevelopmentPollutionEvent::new(
                "polluted-evidence-again",
                "issue_comment",
                "different raw polluted evidence body",
                "development_evidence_contamination",
            )),
            "pr_body",
        );

        let gate = gate_defense_spacer_activation(&[spacer], &candidate);

        assert!(gate.allowed);
        assert_eq!(gate.decision, DefenseSpacerDecision::Expire);
        assert_eq!(gate.reason, "matched_expired_defense_spacer");
        assert!(!gate.summary_line().contains("different raw polluted"));
    }

    #[test]
    fn issue305_acceptance_audit_covers_spacer_classes_and_surfaces() {
        use std::collections::BTreeSet;

        struct SpacerCase {
            id: &'static str,
            source_kind: &'static str,
            payload: &'static str,
            reason: &'static str,
            scope: &'static str,
            threat_class: DefenseSpacerThreatClass,
            decision: DefenseSpacerDecision,
        }

        let cases = [
            SpacerCase {
                id: "retired-runtime",
                source_kind: "runtime_manifest",
                payload: "retired model path C:/private/model.gguf",
                reason: "retired_version_marker:v0.305.0",
                scope: "model_weight_load",
                threat_class: DefenseSpacerThreatClass::RetiredSource,
                decision: DefenseSpacerDecision::Block,
            },
            SpacerCase {
                id: "prompt-marker",
                source_kind: "issue_comment",
                payload: "BEGIN SECRET hidden prompt",
                reason: "prompt_injection_marker",
                scope: "prompt",
                threat_class: DefenseSpacerThreatClass::PromptInjectionOrPrivatePayload,
                decision: DefenseSpacerDecision::Quarantine,
            },
            SpacerCase {
                id: "runtime-manifest",
                source_kind: "runtime_manifest",
                payload: "manifest sha mismatch payload",
                reason: "runtime_manifest_sha_mismatch",
                scope: "process_start",
                threat_class: DefenseSpacerThreatClass::MalformedRuntimeManifest,
                decision: DefenseSpacerDecision::Block,
            },
            SpacerCase {
                id: "unsafe-toolsmith",
                source_kind: "toolsmith_blueprint",
                payload: "unsafe toolsmith blueprint payload",
                reason: "unsafe_toolsmith_blueprint",
                scope: "tool_blueprint_activation",
                threat_class: DefenseSpacerThreatClass::UnsafeToolsmithBlueprint,
                decision: DefenseSpacerDecision::Block,
            },
            SpacerCase {
                id: "poisoned-handoff",
                source_kind: "cross_window_handoff",
                payload: "poisoned handoff packet payload",
                reason: "poisoned_handoff",
                scope: "cross_window_handoff_activation",
                threat_class: DefenseSpacerThreatClass::PoisonedHandoffPacket,
                decision: DefenseSpacerDecision::Block,
            },
            SpacerCase {
                id: "cross-tenant",
                source_kind: "tenant_scope",
                payload: "tenant-a/private-key must not leak",
                reason: "cross_tenant_memory_or_genome",
                scope: "tenant_scope_boundary_activation",
                threat_class: DefenseSpacerThreatClass::CrossTenantContamination,
                decision: DefenseSpacerDecision::Block,
            },
            SpacerCase {
                id: "polluted-evidence",
                source_kind: "pull_request",
                payload: "polluted PR evidence payload",
                reason: "development_evidence_contamination",
                scope: "pull_request_body",
                threat_class: DefenseSpacerThreatClass::DevelopmentEvidenceContamination,
                decision: DefenseSpacerDecision::Quarantine,
            },
            SpacerCase {
                id: "genome-hygiene",
                source_kind: "reasoning_genome",
                payload: "reasoning genome hygiene payload",
                reason: "reasoning_genome_hygiene_violation",
                scope: "genome_expression",
                threat_class: DefenseSpacerThreatClass::ReasoningGenomeHygieneViolation,
                decision: DefenseSpacerDecision::Quarantine,
            },
            SpacerCase {
                id: "stale-claim",
                source_kind: "thread_summary",
                payload: "stale claim payload",
                reason: "stale_or_polluted_claim",
                scope: "experience_retrieval",
                threat_class: DefenseSpacerThreatClass::StaleOrPollutedClaim,
                decision: DefenseSpacerDecision::Quarantine,
            },
            SpacerCase {
                id: "tool-gap",
                source_kind: "thread_summary",
                payload: "missing tool discovery payload",
                reason: "missing_discovery",
                scope: "tool_selection",
                threat_class: DefenseSpacerThreatClass::ToolAffordanceGap,
                decision: DefenseSpacerDecision::Observe,
            },
            SpacerCase {
                id: "operator-review",
                source_kind: "runtime_manifest",
                payload: "unknown manifest payload",
                reason: "unmapped_pollution_signal",
                scope: "process_start",
                threat_class: DefenseSpacerThreatClass::UnknownDevelopmentPollution,
                decision: DefenseSpacerDecision::RequireReview,
            },
        ];

        let mut threat_classes = BTreeSet::new();
        let mut hygiene_states = BTreeSet::new();
        let raw_fragments = [
            "C:/private",
            "BEGIN SECRET",
            "tenant-a/private-key",
            "polluted PR evidence payload",
            "unknown manifest payload",
        ];

        for case in cases {
            let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
                case.id,
                case.source_kind,
                case.payload,
                case.reason,
            ));
            hygiene_states.insert(finding.hygiene_state.as_str());

            let spacer = DefenseSpacer::from_finding(
                &finding,
                case.scope,
                "2026-07-03T13:51:28Z",
                "issue305_acceptance_recheck",
            );
            let candidate = DefenseSpacerCandidate::from_finding(&finding, case.scope);
            let gate = gate_defense_spacer_activation(&[spacer.clone()], &candidate);

            threat_classes.insert(spacer.threat_class.as_str());
            assert_eq!(spacer.threat_class, case.threat_class, "{}", case.id);
            assert_eq!(gate.decision, case.decision, "{}", case.id);
            assert_eq!(
                gate.allowed,
                matches!(case.decision, DefenseSpacerDecision::Observe)
            );
            assert!(spacer.summary_line().contains("redaction-digest:"));
            for fragment in raw_fragments {
                assert!(!finding.summary_line().contains(fragment));
                assert!(!spacer.summary_line().contains(fragment));
                assert!(!gate.summary_line().contains(fragment));
            }
        }

        for required in [
            "retired_source",
            "prompt_injection_or_private_payload",
            "malformed_runtime_manifest",
            "unsafe_toolsmith_blueprint",
            "poisoned_handoff_packet",
            "cross_tenant_contamination",
            "development_evidence_contamination",
            "reasoning_genome_hygiene_violation",
            "stale_or_polluted_claim",
            "tool_affordance_gap",
            "unknown_development_pollution",
        ] {
            assert!(
                threat_classes.contains(required),
                "missing threat {required}"
            );
        }

        for event in [
            DevelopmentPollutionEvent::new(
                "current",
                "pull_request",
                "current validated path",
                "current_validated_path",
            )
            .with_current_proof(true),
            DevelopmentPollutionEvent::new(
                "archive",
                "issue_comment",
                "release evidence archive payload",
                "release_evidence_archive",
            ),
            DevelopmentPollutionEvent::new(
                "delete",
                "output_artifact",
                "reproducible junk payload",
                "reproducible_junk",
            ),
        ] {
            let finding = classify_development_pollution_event(&event);
            hygiene_states.insert(finding.hygiene_state.as_str());
        }

        for required in [
            "clean",
            "suspicious",
            "polluted",
            "stale",
            "unknown",
            "quarantined",
        ] {
            assert!(
                hygiene_states.contains(required),
                "missing hygiene {required}"
            );
        }

        let polluted = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
            "polluted-hot-surface",
            "pull_request",
            "polluted evidence payload",
            "development_evidence_contamination",
        ));
        let admission = admit_development_evidence_for_current_use(&polluted);
        for surface in [
            DevelopmentEvidenceUseSurface::Prompt,
            DevelopmentEvidenceUseSurface::Trace,
            DevelopmentEvidenceUseSurface::Benchmark,
            DevelopmentEvidenceUseSurface::PullRequestBody,
            DevelopmentEvidenceUseSurface::ExperienceRetrieval,
            DevelopmentEvidenceUseSurface::DurableMemory,
            DevelopmentEvidenceUseSurface::GenomeExpression,
        ] {
            let gate = gate_development_evidence_surface(&admission, surface);
            assert!(!gate.allowed, "surface should be blocked: {surface:?}");
            assert_eq!(gate.reason, "digest_only_quarantine_required");
        }
        assert!(
            gate_development_evidence_surface(
                &admission,
                DevelopmentEvidenceUseSurface::DigestMarker
            )
            .allowed
        );
        assert!(admission.validation_required);
        assert!(admission.privacy_license_required);
        assert!(admission.rollback_anchor_required);
        assert!(admission.explicit_approval_required);

        let report = classify_development_pollution(&[
            DevelopmentPollutionEvent::new(
                "gap-1",
                "thread_summary",
                "tool gap one",
                "missing_cleanup",
            ),
            DevelopmentPollutionEvent::new(
                "gap-2",
                "thread_summary",
                "tool gap two",
                "missing_cleanup",
            ),
            DevelopmentPollutionEvent::new(
                "junk",
                "output_artifact",
                "junk body",
                "reproducible_junk",
            )
            .with_hit_count(2),
        ]);
        assert_eq!(report.capability_candidates.len(), 1);
        assert_eq!(
            report.capability_candidates[0].reason_code,
            "missing_cleanup"
        );
        assert!(report.findings.iter().any(|finding| {
            finding.reason_code == "reproducible_junk"
                && finding.nutrient_target == DevelopmentNutrientTarget::NoNutrientValue
                && finding.capability_candidate.is_none()
        }));

        let false_positive = DefenseSpacer::from_finding(
            &classify_development_pollution_event(
                &DevelopmentPollutionEvent::new(
                    "polluted-hot-surface-repeat",
                    "pull_request",
                    "polluted evidence payload",
                    "development_evidence_contamination",
                )
                .with_hit_count(2),
            ),
            "pull_request_body",
            "2026-07-03T13:51:28Z",
            "operator_review",
        );
        assert_eq!(
            false_positive
                .clone()
                .with_false_positive_count(1)
                .effective_decision(),
            DefenseSpacerDecision::RequireReview
        );
        assert_eq!(
            false_positive
                .with_false_positive_count(1_000)
                .effective_decision(),
            DefenseSpacerDecision::Expire
        );
    }
}
