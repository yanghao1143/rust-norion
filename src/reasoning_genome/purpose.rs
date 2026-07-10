use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

use super::model::{GeneValidationStatus, ReasoningGene, ReasoningGeneKind, ReasoningGeneStatus};
use super::schema::{DnaGeneEvidenceKind, DnaGeneRecord};

pub const GENE_PURPOSE_ONTOLOGY_VERSION: &str = "gene_purpose_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenePurposeEvidenceClass {
    SyntheticDefault,
    Reflection,
    MemoryAdmission,
    RuntimeKv,
    ToolReliability,
    OperatorApproved,
    HealthMetadata,
    FeedbackSignal,
}

impl GenePurposeEvidenceClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SyntheticDefault => "synthetic_default",
            Self::Reflection => "reflection",
            Self::MemoryAdmission => "memory_admission",
            Self::RuntimeKv => "runtime_kv",
            Self::ToolReliability => "tool_reliability",
            Self::OperatorApproved => "operator_approved",
            Self::HealthMetadata => "health_metadata",
            Self::FeedbackSignal => "feedback_signal",
        }
    }

    fn from_dna_kind(kind: DnaGeneEvidenceKind) -> Self {
        match kind {
            DnaGeneEvidenceKind::SyntheticDefault => Self::SyntheticDefault,
            DnaGeneEvidenceKind::Reflection => Self::Reflection,
            DnaGeneEvidenceKind::MemoryAdmission => Self::MemoryAdmission,
            DnaGeneEvidenceKind::RuntimeKv => Self::RuntimeKv,
            DnaGeneEvidenceKind::ToolReliability => Self::ToolReliability,
            DnaGeneEvidenceKind::OperatorApproved => Self::OperatorApproved,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenePurposeFreshness {
    Fresh,
    Aging,
    Stale,
    Malignant,
}

impl GenePurposeFreshness {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Aging => "aging",
            Self::Stale => "stale",
            Self::Malignant => "malignant",
        }
    }

    fn from_gene_status(status: ReasoningGeneStatus, age: u32, fitness: f32, drift: f32) -> Self {
        match status {
            ReasoningGeneStatus::Malignant => Self::Malignant,
            ReasoningGeneStatus::Quarantined | ReasoningGeneStatus::Regenerating => Self::Stale,
            ReasoningGeneStatus::Aging => Self::Aging,
            ReasoningGeneStatus::Active if age >= 12 || fitness < 0.35 || drift > 0.55 => {
                Self::Stale
            }
            ReasoningGeneStatus::Active if age >= 8 || fitness < 0.55 || drift > 0.30 => {
                Self::Aging
            }
            ReasoningGeneStatus::Active => Self::Fresh,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenePurposeRelabelDecision {
    AcceptedPreview,
    Quarantined,
}

impl GenePurposeRelabelDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptedPreview => "accepted_preview",
            Self::Quarantined => "quarantined",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenePurposeRecord {
    pub schema_version: &'static str,
    pub stable_id: String,
    pub gene_id: String,
    pub gene_kind: ReasoningGeneKind,
    pub profile: TaskProfile,
    pub task_family: String,
    pub input_shape: String,
    pub output_shape: String,
    pub tenant_scope: String,
    pub source_evidence_class: GenePurposeEvidenceClass,
    pub freshness: GenePurposeFreshness,
    pub fitness_score: f32,
    pub trust_score: f32,
    pub drift_score: f32,
    pub rollback_anchor_id: String,
    pub provenance_digest: String,
    pub purpose_digest: String,
    pub label: String,
    pub purpose_summary: String,
    pub tags: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl GenePurposeRecord {
    pub fn from_dna_record(record: &DnaGeneRecord) -> Self {
        Self::new(
            record.profile,
            record.gene_id.clone(),
            record.gene_kind,
            record.status,
            record.label.clone(),
            record.purpose.clone(),
            record.tags.clone(),
            record.lineage.tenant_scope.clone(),
            GenePurposeEvidenceClass::from_dna_kind(record.source_evidence.kind),
            record.age,
            record.fitness_score,
            record.trust_score,
            record.drift_score,
            record.rollback_anchor_id.clone(),
            [
                record.source_evidence.source_hash.as_str(),
                record.source_evidence.source_summary.as_str(),
            ],
        )
    }

    pub fn from_reasoning_gene(
        profile: TaskProfile,
        tenant_scope: impl Into<String>,
        source_evidence_class: GenePurposeEvidenceClass,
        rollback_anchor_id: impl Into<String>,
        gene: &ReasoningGene,
    ) -> Self {
        let trust_score = (gene.fitness * (1.0 - gene.drift_score)).clamp(0.0, 1.0);
        Self::new(
            profile,
            gene.id.clone(),
            gene.kind,
            gene.derived_status(),
            gene.label.clone(),
            gene.purpose.clone(),
            gene.tags.clone(),
            tenant_scope,
            source_evidence_class,
            gene.age,
            gene.fitness,
            trust_score,
            gene.drift_score,
            rollback_anchor_id,
            [gene.id.as_str(), gene.kind.as_str()],
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        profile: TaskProfile,
        gene_id: impl Into<String>,
        gene_kind: ReasoningGeneKind,
        status: ReasoningGeneStatus,
        label: impl Into<String>,
        purpose: impl Into<String>,
        tags: impl IntoIterator<Item = impl AsRef<str>>,
        tenant_scope: impl Into<String>,
        source_evidence_class: GenePurposeEvidenceClass,
        age: u32,
        fitness_score: f32,
        trust_score: f32,
        drift_score: f32,
        rollback_anchor_id: impl Into<String>,
        provenance_parts: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        let gene_id = gene_id.into();
        let label = label.into();
        let purpose_summary = purpose.into();
        let tenant_scope = tenant_scope.into();
        let rollback_anchor_id = rollback_anchor_id.into();
        let tags = normalized_tags(tags);
        let provenance = provenance_parts
            .into_iter()
            .map(|part| part.as_ref().to_owned())
            .collect::<Vec<_>>();
        let provenance_refs = provenance.iter().map(String::as_str).collect::<Vec<_>>();
        let purpose_digest = stable_redaction_digest([
            gene_id.as_str(),
            label.as_str(),
            purpose_summary.as_str(),
            &tags.join("|"),
        ]);
        let stable_id = stable_redaction_digest([
            "gene-purpose",
            gene_id.as_str(),
            gene_kind.as_str(),
            profile_to_str(profile),
            purpose_digest.as_str(),
        ]);

        Self {
            schema_version: GENE_PURPOSE_ONTOLOGY_VERSION,
            stable_id,
            gene_id,
            gene_kind,
            profile,
            task_family: task_family(profile).to_owned(),
            input_shape: input_shape(gene_kind).to_owned(),
            output_shape: output_shape(gene_kind).to_owned(),
            tenant_scope,
            source_evidence_class,
            freshness: GenePurposeFreshness::from_gene_status(
                status,
                age,
                fitness_score,
                drift_score,
            ),
            fitness_score: clamp_unit(fitness_score),
            trust_score: clamp_unit(trust_score),
            drift_score: clamp_unit(drift_score),
            rollback_anchor_id,
            provenance_digest: stable_redaction_digest(provenance_refs),
            purpose_digest,
            label,
            purpose_summary,
            tags,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn to_kv_line(&self) -> String {
        let fields = [
            self.schema_version.to_owned(),
            self.stable_id.clone(),
            self.gene_id.clone(),
            self.gene_kind.as_str().to_owned(),
            profile_to_str(self.profile).to_owned(),
            self.task_family.clone(),
            self.input_shape.clone(),
            self.output_shape.clone(),
            self.tenant_scope.clone(),
            self.source_evidence_class.as_str().to_owned(),
            self.freshness.as_str().to_owned(),
            finite_f32_to_field(self.fitness_score),
            finite_f32_to_field(self.trust_score),
            finite_f32_to_field(self.drift_score),
            self.rollback_anchor_id.clone(),
            self.provenance_digest.clone(),
            self.purpose_digest.clone(),
            self.label.clone(),
            self.purpose_summary.clone(),
            serialize_tags(&self.tags),
            bool_to_field(self.read_only).to_owned(),
            bool_to_field(self.write_allowed).to_owned(),
            bool_to_field(self.applied).to_owned(),
        ];
        fields
            .iter()
            .map(|field| escape_field(field))
            .collect::<Vec<_>>()
            .join("\t")
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenePurposeRelabelEvidence {
    pub source_evidence_class: GenePurposeEvidenceClass,
    pub source_digest: String,
    pub evidence_summary: String,
    pub observed_label: String,
    pub observed_purpose: String,
    pub observed_tags: Vec<String>,
    pub freshness: GenePurposeFreshness,
    pub fitness_score: f32,
    pub trust_score: f32,
    pub rollback_anchor_id: String,
    pub privacy_checked: bool,
}

impl GenePurposeRelabelEvidence {
    pub fn new(
        source_evidence_class: GenePurposeEvidenceClass,
        source_digest: impl Into<String>,
        evidence_summary: impl Into<String>,
        observed_label: impl Into<String>,
        observed_purpose: impl Into<String>,
        rollback_anchor_id: impl Into<String>,
    ) -> Self {
        Self {
            source_evidence_class,
            source_digest: source_digest.into(),
            evidence_summary: evidence_summary.into(),
            observed_label: observed_label.into(),
            observed_purpose: observed_purpose.into(),
            observed_tags: Vec::new(),
            freshness: GenePurposeFreshness::Fresh,
            fitness_score: 0.80,
            trust_score: 0.80,
            rollback_anchor_id: rollback_anchor_id.into(),
            privacy_checked: true,
        }
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.observed_tags = normalized_tags(tags);
        self
    }

    pub fn with_health(
        mut self,
        freshness: GenePurposeFreshness,
        fitness_score: f32,
        trust_score: f32,
    ) -> Self {
        self.freshness = freshness;
        self.fitness_score = clamp_unit(fitness_score);
        self.trust_score = clamp_unit(trust_score);
        self
    }

    pub fn without_privacy_check(mut self) -> Self {
        self.privacy_checked = false;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenePurposeRelabelProposal {
    pub target_gene_id: String,
    pub proposed_record: GenePurposeRecord,
    pub proposed_label: String,
    pub proposed_purpose: String,
    pub proposed_tags: Vec<String>,
    pub source_digest: String,
    pub proposal_digest: String,
    pub validation_status: GeneValidationStatus,
    pub decision: GenePurposeRelabelDecision,
    pub reason_codes: Vec<String>,
    pub preview_only: bool,
    pub approval_required: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl GenePurposeRelabelProposal {
    pub fn accepted(&self) -> bool {
        self.decision == GenePurposeRelabelDecision::AcceptedPreview
    }

    pub fn quarantined(&self) -> bool {
        self.decision == GenePurposeRelabelDecision::Quarantined
    }

    pub fn summary_line(&self) -> String {
        format!(
            "gene_purpose_relabel_v1 target={} decision={} validation={} source={} proposal={} preview_only={} approval_required={} write_allowed={} applied={} reasons={}",
            stable_redaction_digest([self.target_gene_id.as_str()]),
            self.decision.as_str(),
            self.validation_status.as_str(),
            self.source_digest,
            self.proposal_digest,
            self.preview_only,
            self.approval_required,
            self.write_allowed,
            self.applied,
            self.reason_codes.join("|")
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenePurposeRelabelPolicy {
    pub min_label_chars: usize,
    pub min_purpose_chars: usize,
    pub min_trust_score: f32,
    pub min_fitness_score: f32,
    pub require_privacy_check: bool,
}

impl Default for GenePurposeRelabelPolicy {
    fn default() -> Self {
        Self {
            min_label_chars: 6,
            min_purpose_chars: 24,
            min_trust_score: 0.35,
            min_fitness_score: 0.30,
            require_privacy_check: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenePurposeRelabelValidator {
    pub policy: GenePurposeRelabelPolicy,
}

impl GenePurposeRelabelValidator {
    pub fn new(policy: GenePurposeRelabelPolicy) -> Self {
        Self { policy }
    }

    pub fn validate(
        &self,
        current: &GenePurposeRecord,
        evidence: &GenePurposeRelabelEvidence,
    ) -> GenePurposeRelabelProposal {
        let mut reason_codes = Vec::new();
        validate_current_record(current, &mut reason_codes);
        validate_evidence(&self.policy, evidence, &mut reason_codes);

        let proposed_label = proposed_label(current, evidence);
        let proposed_purpose = proposed_purpose(current, evidence);
        let proposed_tags = proposed_tags(current, evidence);
        validate_proposal_text(
            &self.policy,
            &proposed_label,
            &proposed_purpose,
            &mut reason_codes,
        );

        if conflicts_with_current(current, &proposed_label, &proposed_purpose) {
            push_code_once(&mut reason_codes, "conflicting_relabel");
        }
        if evidence.freshness == GenePurposeFreshness::Stale {
            push_code_once(&mut reason_codes, "stale_evidence");
        }
        if evidence.freshness == GenePurposeFreshness::Malignant {
            push_code_once(&mut reason_codes, "malignant_evidence");
        }

        let decision = if reason_codes.iter().any(|code| is_blocking_code(code)) {
            GenePurposeRelabelDecision::Quarantined
        } else {
            GenePurposeRelabelDecision::AcceptedPreview
        };
        let validation_status = match decision {
            GenePurposeRelabelDecision::AcceptedPreview => GeneValidationStatus::Pending,
            GenePurposeRelabelDecision::Quarantined => GeneValidationStatus::Failed,
        };
        let proposed_record = GenePurposeRecord::new(
            current.profile,
            current.gene_id.clone(),
            current.gene_kind,
            status_for_freshness(evidence.freshness),
            proposed_label.clone(),
            proposed_purpose.clone(),
            proposed_tags.clone(),
            current.tenant_scope.clone(),
            evidence.source_evidence_class,
            0,
            evidence.fitness_score,
            evidence.trust_score,
            current.drift_score.min(0.20),
            evidence.rollback_anchor_id.clone(),
            [evidence.source_digest.as_str(), current.stable_id.as_str()],
        );
        let proposal_digest = stable_redaction_digest([
            current.stable_id.as_str(),
            proposed_record.stable_id.as_str(),
            evidence.source_digest.as_str(),
            validation_status.as_str(),
            &reason_codes.join("|"),
        ]);

        GenePurposeRelabelProposal {
            target_gene_id: current.gene_id.clone(),
            proposed_record,
            proposed_label,
            proposed_purpose,
            proposed_tags,
            source_digest: evidence.source_digest.clone(),
            proposal_digest,
            validation_status,
            decision,
            reason_codes,
            preview_only: true,
            approval_required: true,
            write_allowed: false,
            applied: false,
        }
    }
}

impl Default for GenePurposeRelabelValidator {
    fn default() -> Self {
        Self::new(GenePurposeRelabelPolicy::default())
    }
}

fn validate_current_record(record: &GenePurposeRecord, reason_codes: &mut Vec<String>) {
    if record.stable_id.trim().is_empty() {
        push_code_once(reason_codes, "missing_stable_id");
    }
    if record.rollback_anchor_id.trim().is_empty() {
        push_code_once(reason_codes, "missing_rollback_anchor");
    }
    if record.tenant_scope.trim().is_empty() {
        push_code_once(reason_codes, "missing_tenant_scope");
    }
    if !record.is_preview_only() {
        push_code_once(reason_codes, "current_record_not_preview_only");
    }
    for value in [record.label.as_str(), record.purpose_summary.as_str()] {
        if contains_private_or_executable_marker(value) {
            push_code_once(reason_codes, "private_payload_marker");
        }
    }
    if record
        .tags
        .iter()
        .any(|tag| contains_private_or_executable_marker(tag))
    {
        push_code_once(reason_codes, "private_payload_marker");
    }
}

fn validate_evidence(
    policy: &GenePurposeRelabelPolicy,
    evidence: &GenePurposeRelabelEvidence,
    reason_codes: &mut Vec<String>,
) {
    if evidence.source_digest.trim().is_empty()
        || !evidence.source_digest.starts_with("redaction-digest:")
    {
        push_code_once(reason_codes, "missing_source_digest");
    }
    if evidence.rollback_anchor_id.trim().is_empty() {
        push_code_once(reason_codes, "missing_rollback_anchor");
    }
    if policy.require_privacy_check && !evidence.privacy_checked {
        push_code_once(reason_codes, "privacy_gate_missing");
    }
    if evidence.trust_score < policy.min_trust_score {
        push_code_once(reason_codes, "low_trust_evidence");
    }
    if evidence.fitness_score < policy.min_fitness_score {
        push_code_once(reason_codes, "low_fitness_evidence");
    }
    for value in [
        evidence.evidence_summary.as_str(),
        evidence.observed_label.as_str(),
        evidence.observed_purpose.as_str(),
    ] {
        if contains_private_or_executable_marker(value) {
            push_code_once(reason_codes, "private_payload_marker");
        }
    }
    if evidence
        .observed_tags
        .iter()
        .any(|tag| contains_private_or_executable_marker(tag))
    {
        push_code_once(reason_codes, "private_payload_marker");
    }
}

fn validate_proposal_text(
    policy: &GenePurposeRelabelPolicy,
    label: &str,
    purpose: &str,
    reason_codes: &mut Vec<String>,
) {
    if label.trim().len() < policy.min_label_chars || is_ambiguous_label(label) {
        push_code_once(reason_codes, "ambiguous_label");
    }
    if purpose.trim().len() < policy.min_purpose_chars {
        push_code_once(reason_codes, "ambiguous_purpose");
    }
    if contains_conflict_marker(label) || contains_conflict_marker(purpose) {
        push_code_once(reason_codes, "contradictory_label");
    }
    if contains_private_or_executable_marker(label)
        || contains_private_or_executable_marker(purpose)
    {
        push_code_once(reason_codes, "private_payload_marker");
    }
}

fn proposed_label(current: &GenePurposeRecord, evidence: &GenePurposeRelabelEvidence) -> String {
    let observed = evidence.observed_label.trim();
    if !observed.is_empty() && !contains_private_or_executable_marker(observed) {
        format!("refreshed {observed}")
    } else if !current.label.trim().is_empty()
        && !is_ambiguous_label(&current.label)
        && !contains_private_or_executable_marker(&current.label)
    {
        format!("refreshed {}", current.label.trim())
    } else {
        canonical_label(current.gene_kind).to_owned()
    }
}

fn proposed_purpose(current: &GenePurposeRecord, evidence: &GenePurposeRelabelEvidence) -> String {
    let observed = evidence.observed_purpose.trim();
    if !observed.is_empty() && !contains_private_or_executable_marker(observed) {
        format!(
            "{observed}; relabel preview preserves ontology id, rollback anchor, and validation gates"
        )
    } else if !current.purpose_summary.trim().is_empty()
        && !contains_private_or_executable_marker(&current.purpose_summary)
    {
        format!(
            "{}; refreshed from redacted ontology evidence before reuse",
            current.purpose_summary.trim()
        )
    } else {
        format!(
            "{}; relabel preview generated from canonical purpose ontology",
            canonical_purpose(current.gene_kind)
        )
    }
}

fn proposed_tags(
    current: &GenePurposeRecord,
    evidence: &GenePurposeRelabelEvidence,
) -> Vec<String> {
    normalized_tags(
        current
            .tags
            .iter()
            .chain(evidence.observed_tags.iter())
            .map(String::as_str)
            .chain([
                current.gene_kind.as_str(),
                "purpose_ontology",
                "relabel",
                "preview_only",
            ]),
    )
}

fn conflicts_with_current(
    current: &GenePurposeRecord,
    proposed_label: &str,
    proposed_purpose: &str,
) -> bool {
    let current_label = current.label.to_ascii_lowercase();
    let proposed_label = proposed_label.to_ascii_lowercase();
    let proposed_purpose = proposed_purpose.to_ascii_lowercase();
    contains_conflict_marker(&current_label)
        || contains_conflict_marker(&proposed_label)
        || contains_conflict_marker(&proposed_purpose)
}

fn is_blocking_code(code: &str) -> bool {
    matches!(
        code,
        "missing_stable_id"
            | "missing_rollback_anchor"
            | "missing_tenant_scope"
            | "current_record_not_preview_only"
            | "missing_source_digest"
            | "privacy_gate_missing"
            | "low_trust_evidence"
            | "low_fitness_evidence"
            | "private_payload_marker"
            | "ambiguous_label"
            | "ambiguous_purpose"
            | "contradictory_label"
            | "conflicting_relabel"
            | "stale_evidence"
            | "malignant_evidence"
    )
}

fn status_for_freshness(freshness: GenePurposeFreshness) -> ReasoningGeneStatus {
    match freshness {
        GenePurposeFreshness::Fresh => ReasoningGeneStatus::Active,
        GenePurposeFreshness::Aging => ReasoningGeneStatus::Aging,
        GenePurposeFreshness::Stale => ReasoningGeneStatus::Quarantined,
        GenePurposeFreshness::Malignant => ReasoningGeneStatus::Malignant,
    }
}

fn is_ambiguous_label(label: &str) -> bool {
    let label = label.trim().to_ascii_lowercase();
    label.is_empty()
        || matches!(
            label.as_str(),
            "misc" | "unknown" | "todo" | "stuff" | "thing" | "unlabelled segment"
        )
}

fn contains_conflict_marker(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("contradict ")
        || value.contains("contradicts ")
        || value.contains("conflicting ")
        || value.contains("conflict with")
        || value.contains("opposite ")
        || value.contains("mutually exclusive")
}

fn normalized_tags(tags: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.as_ref().trim().to_ascii_lowercase().replace(' ', "_");
        if tag.is_empty()
            || contains_private_or_executable_marker(&tag)
            || normalized.contains(&tag)
        {
            continue;
        }
        normalized.push(tag);
    }
    normalized.sort();
    normalized
}

fn task_family(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general_reasoning",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn profile_to_str(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn input_shape(kind: ReasoningGeneKind) -> &'static str {
    match kind {
        ReasoningGeneKind::Retrieval => "semantic_memory_or_runtime_kv",
        ReasoningGeneKind::Routing => "task_hardware_entropy_cache_signals",
        ReasoningGeneKind::Reflection => "draft_trace_and_validation_evidence",
        ReasoningGeneKind::Language => "task_text_and_language_mode",
        ReasoningGeneKind::Safety => "candidate_state_and_privacy_markers",
        ReasoningGeneKind::ToolUse => "tool_request_and_build_evidence",
        ReasoningGeneKind::Budget => "compute_pressure_and_latency_budget",
    }
}

fn output_shape(kind: ReasoningGeneKind) -> &'static str {
    match kind {
        ReasoningGeneKind::Retrieval => "ranked_memory_selection",
        ReasoningGeneKind::Routing => "route_plan_and_attention_thresholds",
        ReasoningGeneKind::Reflection => "reflection_issues_and_repair_actions",
        ReasoningGeneKind::Language => "profile_scoped_language_plan",
        ReasoningGeneKind::Safety => "safety_gate_and_quarantine_reason",
        ReasoningGeneKind::ToolUse => "validated_tool_plan",
        ReasoningGeneKind::Budget => "bounded_compute_schedule",
    }
}

fn canonical_label(kind: ReasoningGeneKind) -> &'static str {
    match kind {
        ReasoningGeneKind::Retrieval => "memory retrieval gene",
        ReasoningGeneKind::Routing => "adaptive routing gene",
        ReasoningGeneKind::Reflection => "closed-loop reflection gene",
        ReasoningGeneKind::Language => "task language gene",
        ReasoningGeneKind::Safety => "drift safety gene",
        ReasoningGeneKind::ToolUse => "local tool-use gene",
        ReasoningGeneKind::Budget => "compute budget gene",
    }
}

fn canonical_purpose(kind: ReasoningGeneKind) -> &'static str {
    match kind {
        ReasoningGeneKind::Retrieval => {
            "select useful semantic, gist, and runtime KV memory with bounded evidence"
        }
        ReasoningGeneKind::Routing => {
            "route attention thresholds using task, hardware, entropy, and cache signals"
        }
        ReasoningGeneKind::Reflection => {
            "surface contradictions, repair actions, and validated memory admission evidence"
        }
        ReasoningGeneKind::Language => {
            "keep English, Chinese, coding, writing, and long-context behavior profile-scoped"
        }
        ReasoningGeneKind::Safety => {
            "block unsafe memory admission, drift, privacy leaks, and unreviewed mutation writes"
        }
        ReasoningGeneKind::ToolUse => {
            "prefer Rust-written local tools behind explicit build and validation gates"
        }
        ReasoningGeneKind::Budget => {
            "reduce wasted compute while preserving rollback and regeneration evidence"
        }
    }
}

fn finite_f32_to_field(value: f32) -> String {
    if value.is_finite() {
        format!("{:.6}", value)
    } else {
        "nan".to_owned()
    }
}

fn bool_to_field(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn serialize_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|tag| escape_field(tag))
        .collect::<Vec<_>>()
        .join("|")
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('|', "\\p")
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn push_code_once(codes: &mut Vec<String>, code: &str) {
    if !codes.iter().any(|existing| existing == code) {
        codes.push(code.to_owned());
    }
}
