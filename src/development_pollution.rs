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
}
