use crate::hierarchy::TaskProfile;
use crate::kv_exchange::RuntimeKvBlock;

use super::model::{GeneScissorsIntent, MutationPlan};

const MAX_SEGMENT_DECAY_AGE: u32 = 16;
const GENE_SCISSORS_READMISSION_HOLD_GATE: &str = "hold_until_verifier_and_operator_approval";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneSegmentSource {
    Prompt,
    SemanticMemory,
    GistMemory,
    RuntimeKv,
    GenomeLedger,
    ToolOutput,
}

impl GeneSegmentSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::SemanticMemory => "semantic_memory",
            Self::GistMemory => "gist_memory",
            Self::RuntimeKv => "runtime_kv",
            Self::GenomeLedger => "genome_ledger",
            Self::ToolOutput => "tool_output",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneKvResidency {
    None,
    Sink,
    HotRecent,
    PackedSynopsis,
    ColdEvidence,
}

impl GeneKvResidency {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sink => "sink",
            Self::HotRecent => "hot_recent",
            Self::PackedSynopsis => "packed_synopsis",
            Self::ColdEvidence => "cold_evidence",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneSegmentClass {
    Exon,
    Intron,
    Variant,
}

impl GeneSegmentClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exon => "exon",
            Self::Intron => "intron",
            Self::Variant => "variant",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneSegmentDisposition {
    Retained,
    Skipped,
    Quarantined,
    RepairCandidate,
}

impl GeneSegmentDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Retained => "retained",
            Self::Skipped => "skipped",
            Self::Quarantined => "quarantined",
            Self::RepairCandidate => "repair_candidate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneVariantKind {
    Insertion,
    Deletion,
    Truncation,
    StaleLabel,
    Contradiction,
    LowFitnessRepetition,
    Drift,
    Privacy,
    KvShape,
    Schema,
    EmptyRange,
    MissingSourceHash,
}

impl GeneVariantKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Insertion => "insertion",
            Self::Deletion => "deletion",
            Self::Truncation => "truncation",
            Self::StaleLabel => "stale_label",
            Self::Contradiction => "contradiction",
            Self::LowFitnessRepetition => "low_fitness_repetition",
            Self::Drift => "drift",
            Self::Privacy => "privacy",
            Self::KvShape => "kv_shape",
            Self::Schema => "schema",
            Self::EmptyRange => "empty_range",
            Self::MissingSourceHash => "missing_source_hash",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneVariantSeverity {
    Watch,
    Repair,
    Quarantine,
}

impl GeneVariantSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Watch => "watch",
            Self::Repair => "repair",
            Self::Quarantine => "quarantine",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneSegment {
    pub id: String,
    pub profile: TaskProfile,
    pub source: GeneSegmentSource,
    pub source_hash: String,
    pub tenant_scope: String,
    pub start_token: usize,
    pub end_token: usize,
    pub label: String,
    pub purpose: String,
    pub last_confirmed_purpose: String,
    pub semantic_gist: String,
    pub age: u32,
    pub kv_residency: GeneKvResidency,
    pub fitness: f32,
    pub drift_score: f32,
    pub privacy_risk: f32,
    pub schema_valid: bool,
    pub kv_shape_valid: bool,
}

impl GeneSegment {
    pub fn new(
        id: impl Into<String>,
        profile: TaskProfile,
        source: GeneSegmentSource,
        start_token: usize,
        end_token: usize,
    ) -> Self {
        Self {
            id: id.into(),
            profile,
            source,
            source_hash: String::new(),
            tenant_scope: "local".to_owned(),
            start_token,
            end_token,
            label: "unlabelled segment".to_owned(),
            purpose: "carry bounded reasoning evidence".to_owned(),
            last_confirmed_purpose: "carry bounded reasoning evidence".to_owned(),
            semantic_gist: String::new(),
            age: 0,
            kv_residency: GeneKvResidency::ColdEvidence,
            fitness: 1.0,
            drift_score: 0.0,
            privacy_risk: 0.0,
            schema_valid: true,
            kv_shape_valid: true,
        }
    }

    pub fn with_source_hash(mut self, source_hash: impl Into<String>) -> Self {
        self.source_hash = source_hash.into();
        self
    }

    pub fn with_scope(mut self, tenant_scope: impl Into<String>) -> Self {
        self.tenant_scope = tenant_scope.into();
        self
    }

    pub fn with_metadata(
        mut self,
        label: impl Into<String>,
        purpose: impl Into<String>,
        semantic_gist: impl Into<String>,
    ) -> Self {
        self.label = label.into();
        self.purpose = purpose.into();
        self.last_confirmed_purpose = self.purpose.clone();
        self.semantic_gist = semantic_gist.into();
        self
    }

    pub fn with_age(mut self, age: u32) -> Self {
        self.age = age;
        self
    }

    pub fn with_last_confirmed_purpose(
        mut self,
        last_confirmed_purpose: impl Into<String>,
    ) -> Self {
        self.last_confirmed_purpose = last_confirmed_purpose.into();
        self
    }

    pub fn with_kv_residency(mut self, kv_residency: GeneKvResidency) -> Self {
        self.kv_residency = kv_residency;
        self
    }

    pub fn with_health(mut self, fitness: f32, drift_score: f32, privacy_risk: f32) -> Self {
        self.fitness = clamp_unit(fitness);
        self.drift_score = clamp_unit(drift_score);
        self.privacy_risk = clamp_unit(privacy_risk);
        self
    }

    pub fn with_schema(mut self, schema_valid: bool, kv_shape_valid: bool) -> Self {
        self.schema_valid = schema_valid;
        self.kv_shape_valid = kv_shape_valid;
        self
    }

    pub fn from_runtime_kv_block(
        id: impl Into<String>,
        profile: TaskProfile,
        source_hash: impl Into<String>,
        block: &RuntimeKvBlock,
    ) -> Self {
        let kv_shape_valid = block.validate_shape(usize::MAX, usize::MAX, None).is_ok();
        Self::new(
            id,
            profile,
            GeneSegmentSource::RuntimeKv,
            block.token_start,
            block.token_end,
        )
        .with_source_hash(source_hash)
        .with_metadata(
            format!("runtime KV l{}h{}", block.layer, block.head),
            "carry bounded runtime KV evidence through genome splicing preview",
            format!(
                "runtime KV tokens {}..{} key_dims={} value_dims={}",
                block.token_start,
                block.token_end,
                block.key.len(),
                block.value.len()
            ),
        )
        .with_kv_residency(GeneKvResidency::PackedSynopsis)
        .with_schema(true, kv_shape_valid)
    }

    pub fn token_count(&self) -> usize {
        self.end_token.saturating_sub(self.start_token)
    }

    pub fn decay_score(&self) -> f32 {
        let age_pressure =
            (self.age.min(MAX_SEGMENT_DECAY_AGE) as f32 / MAX_SEGMENT_DECAY_AGE as f32) * 0.40;
        let fitness_pressure = (1.0 - clamp_unit(self.fitness)) * 0.35;
        let drift_pressure = clamp_unit(self.drift_score) * 0.25;
        clamp_unit(age_pressure + fitness_pressure + drift_pressure)
    }

    pub fn has_stale_label(&self, max_segment_age: u32) -> bool {
        self.label.trim().is_empty()
            || self.purpose.trim().is_empty()
            || self.last_confirmed_purpose.trim().is_empty()
            || self.age >= max_segment_age
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DnaSplicerPolicy {
    pub min_exon_fitness: f32,
    pub max_exon_drift: f32,
    pub max_exon_privacy_risk: f32,
    pub max_segment_tokens: usize,
    pub max_planned_overlap_tokens: usize,
    pub max_segment_age: u32,
    pub require_source_hash: bool,
}

impl Default for DnaSplicerPolicy {
    fn default() -> Self {
        Self {
            min_exon_fitness: 0.55,
            max_exon_drift: 0.35,
            max_exon_privacy_risk: 0.20,
            max_segment_tokens: 512,
            max_planned_overlap_tokens: 256,
            max_segment_age: 8,
            require_source_hash: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationFinding {
    pub id: String,
    pub segment_id: String,
    pub kind: GeneVariantKind,
    pub severity: GeneVariantSeverity,
    pub suggested_intent: GeneScissorsIntent,
    pub reason: String,
}

impl MutationFinding {
    fn new(
        segment_id: &str,
        kind: GeneVariantKind,
        severity: GeneVariantSeverity,
        suggested_intent: GeneScissorsIntent,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("finding:{segment_id}:{}", kind.as_str()),
            segment_id: segment_id.to_owned(),
            kind,
            severity,
            suggested_intent,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneScissorsLifecycleState {
    Detected,
    Quarantined,
    RepairCandidate,
    Validated,
    Cut,
    Held,
    Rejected,
}

impl GeneScissorsLifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Detected => "detected",
            Self::Quarantined => "quarantined",
            Self::RepairCandidate => "repair_candidate",
            Self::Validated => "validated",
            Self::Cut => "cut",
            Self::Held => "held",
            Self::Rejected => "rejected",
        }
    }

    pub fn control_lifecycle_state(self) -> &'static str {
        match self {
            Self::Detected => "suspect",
            Self::Quarantined => "quarantined",
            Self::RepairCandidate | Self::Validated => "repaired_candidate",
            Self::Cut => "tombstone_preview",
            Self::Held => "recycle_candidate",
            Self::Rejected => "rejected_final",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneScissorsValidationStatus {
    Pending,
    Passed,
    Failed,
}

impl GeneScissorsValidationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneScissorsLifecycleRecord {
    pub id: String,
    pub target_segment_id: String,
    pub finding_ids: Vec<String>,
    pub finding_kinds: Vec<GeneVariantKind>,
    pub mutation_plan_ids: Vec<String>,
    pub state: GeneScissorsLifecycleState,
    pub validation_status: GeneScissorsValidationStatus,
    pub confidence: f32,
    pub reason_code: String,
    pub source_digest: String,
    pub parent_lineage: String,
    pub rollback_anchor_id: String,
    pub affected_scope: String,
    pub readmission_gate: String,
    pub operator_approval_required: bool,
    pub stable_anchor_sources: Vec<String>,
    pub next_action: String,
    pub admission_write_authorized: bool,
    pub applied: bool,
}

impl GeneScissorsLifecycleRecord {
    fn preview(
        target_segment_id: impl Into<String>,
        findings: &[&MutationFinding],
        mutation_plans: &[MutationPlan],
        segment: Option<&GeneSegment>,
        stable_anchor_id: &str,
    ) -> Self {
        let target_segment_id = target_segment_id.into();
        let state = initial_lifecycle_state(findings);
        let mutation_plan_ids = mutation_plans
            .iter()
            .filter(|plan| plan.target_gene_id == target_segment_id)
            .map(|plan| plan.id.clone())
            .collect::<Vec<_>>();
        let finding_ids = findings
            .iter()
            .map(|finding| finding.id.clone())
            .collect::<Vec<_>>();
        let mut finding_kinds = Vec::new();
        for finding in findings {
            if !finding_kinds.contains(&finding.kind) {
                finding_kinds.push(finding.kind);
            }
        }
        let confidence = lifecycle_confidence(findings);
        let reason_code = findings
            .first()
            .map(|finding| finding.kind.as_str())
            .unwrap_or("detected")
            .to_owned();
        let source_digest = segment_source_digest(segment);
        let affected_scope = segment_affected_scope(segment);
        let parent_lineage = format!("{stable_anchor_id}:{target_segment_id}");

        Self {
            id: format!("gene_scissors:{target_segment_id}:{}", state.as_str()),
            target_segment_id,
            finding_ids,
            finding_kinds,
            mutation_plan_ids,
            state,
            validation_status: GeneScissorsValidationStatus::Pending,
            confidence,
            reason_code,
            source_digest,
            parent_lineage,
            rollback_anchor_id: stable_anchor_id.to_owned(),
            affected_scope,
            readmission_gate: GENE_SCISSORS_READMISSION_HOLD_GATE.to_owned(),
            operator_approval_required: true,
            stable_anchor_sources: vec![stable_anchor_id.to_owned()],
            next_action: next_action_for_state(state).to_owned(),
            admission_write_authorized: false,
            applied: false,
        }
    }

    pub fn with_validation_status(
        mut self,
        validation_status: GeneScissorsValidationStatus,
    ) -> Self {
        self.validation_status = validation_status;
        match validation_status {
            GeneScissorsValidationStatus::Pending => {}
            GeneScissorsValidationStatus::Passed => {
                self.state = GeneScissorsLifecycleState::Validated;
                self.next_action = "await_operator_approval_before_apply".to_owned();
            }
            GeneScissorsValidationStatus::Failed => {
                if self.state == GeneScissorsLifecycleState::Quarantined {
                    self.state = GeneScissorsLifecycleState::Rejected;
                    self.next_action =
                        "reject_candidate_keep_quarantine_and_rollback_anchor".to_owned();
                } else {
                    self.state = GeneScissorsLifecycleState::Held;
                    self.next_action = "hold_candidate_for_more_evidence".to_owned();
                }
                self.admission_write_authorized = false;
                self.applied = false;
            }
        }
        self
    }

    pub fn with_cut_preview(mut self) -> Self {
        self.state = GeneScissorsLifecycleState::Cut;
        self.next_action = "cut_from_active_expression_after_operator_approval".to_owned();
        self.admission_write_authorized = false;
        self.applied = false;
        self
    }

    pub fn is_read_only_preview(&self) -> bool {
        !self.admission_write_authorized && !self.applied
    }

    pub fn summary(&self) -> String {
        format!(
            "target_present={} state={} control_lifecycle_state={} validation={} confidence={:.3} reason_present={} source_digest_present={} parent_lineage_present={} affected_scope_present={} readmission_gate_present={} operator_approval_required={} findings={} plans={} rollback_present={} stable_anchors={} next_present={} write_allowed={} applied={}",
            !self.target_segment_id.trim().is_empty(),
            self.state.as_str(),
            self.state.control_lifecycle_state(),
            self.validation_status.as_str(),
            self.confidence,
            !self.reason_code.trim().is_empty(),
            !self.source_digest.trim().is_empty(),
            !self.parent_lineage.trim().is_empty(),
            !self.affected_scope.trim().is_empty(),
            !self.readmission_gate.trim().is_empty(),
            self.operator_approval_required,
            self.finding_kinds.len(),
            self.mutation_plan_ids.len(),
            !self.rollback_anchor_id.trim().is_empty(),
            self.stable_anchor_sources.len(),
            !self.next_action.trim().is_empty(),
            self.admission_write_authorized,
            self.applied
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassifiedGeneSegment {
    pub segment: GeneSegment,
    pub class: GeneSegmentClass,
    pub disposition: GeneSegmentDisposition,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DnaSplicePreview {
    pub profile: TaskProfile,
    pub stable_anchor_id: String,
    pub segments: Vec<ClassifiedGeneSegment>,
    pub findings: Vec<MutationFinding>,
    pub mutation_plans: Vec<MutationPlan>,
    pub lifecycle_records: Vec<GeneScissorsLifecycleRecord>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl DnaSplicePreview {
    pub fn exon_count(&self) -> usize {
        self.count_class(GeneSegmentClass::Exon)
    }

    pub fn intron_count(&self) -> usize {
        self.count_class(GeneSegmentClass::Intron)
    }

    pub fn variant_count(&self) -> usize {
        self.count_class(GeneSegmentClass::Variant)
    }

    pub fn retained_count(&self) -> usize {
        self.count_disposition(GeneSegmentDisposition::Retained)
    }

    pub fn skipped_count(&self) -> usize {
        self.count_disposition(GeneSegmentDisposition::Skipped)
    }

    pub fn quarantined_count(&self) -> usize {
        self.count_disposition(GeneSegmentDisposition::Quarantined)
    }

    pub fn repair_candidate_count(&self) -> usize {
        self.count_disposition(GeneSegmentDisposition::RepairCandidate)
    }

    pub fn lifecycle_record_count(&self) -> usize {
        self.lifecycle_records.len()
    }

    pub fn quarantined_lifecycle_count(&self) -> usize {
        self.count_lifecycle_state(GeneScissorsLifecycleState::Quarantined)
    }

    pub fn held_lifecycle_count(&self) -> usize {
        self.count_lifecycle_state(GeneScissorsLifecycleState::Held)
    }

    pub fn rejected_lifecycle_count(&self) -> usize {
        self.count_lifecycle_state(GeneScissorsLifecycleState::Rejected)
    }

    pub fn total_token_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.segment.token_count())
            .sum()
    }

    pub fn retained_token_count(&self) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.disposition == GeneSegmentDisposition::Retained)
            .map(|segment| segment.segment.token_count())
            .sum()
    }

    pub fn estimated_saved_token_count(&self) -> usize {
        self.total_token_count()
            .saturating_sub(self.retained_token_count())
    }

    pub fn disposition_summaries(&self) -> Vec<String> {
        let mut dispositions = Vec::new();
        for segment in &self.segments {
            let disposition = segment.disposition.as_str().to_owned();
            if !dispositions.contains(&disposition) {
                dispositions.push(disposition);
            }
        }
        dispositions
    }

    pub fn lifecycle_state_summaries(&self) -> Vec<String> {
        let mut states = Vec::new();
        for record in &self.lifecycle_records {
            let state = record.state.as_str().to_owned();
            if !states.contains(&state) {
                states.push(state);
            }
        }
        states
    }

    pub fn control_lifecycle_state_summaries(&self) -> Vec<String> {
        let mut states = Vec::new();
        for record in &self.lifecycle_records {
            let state = record.state.control_lifecycle_state().to_owned();
            if !states.contains(&state) {
                states.push(state);
            }
        }
        states
    }

    pub fn lifecycle_summaries(&self, limit: usize) -> Vec<String> {
        self.lifecycle_records
            .iter()
            .take(limit)
            .map(GeneScissorsLifecycleRecord::summary)
            .collect()
    }

    pub fn segment_reason_summaries(&self, limit: usize) -> Vec<String> {
        self.segments
            .iter()
            .take(limit)
            .map(segment_reason_summary)
            .collect()
    }

    pub fn mutation_intents(&self) -> Vec<String> {
        let mut intents = Vec::new();
        for plan in &self.mutation_plans {
            let intent = plan.intent.as_str().to_owned();
            if !intents.contains(&intent) {
                intents.push(intent);
            }
        }
        intents
    }

    pub fn finding_kinds(&self) -> Vec<String> {
        let mut kinds = Vec::new();
        for finding in &self.findings {
            let kind = finding.kind.as_str().to_owned();
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
        kinds
    }

    pub fn proposal_ids(&self) -> Vec<String> {
        self.mutation_plans
            .iter()
            .map(|plan| plan.id.clone())
            .collect()
    }

    pub fn is_read_only_preview(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .mutation_plans
                .iter()
                .all(MutationPlan::is_read_only_preview)
            && self
                .lifecycle_records
                .iter()
                .all(GeneScissorsLifecycleRecord::is_read_only_preview)
    }

    fn count_class(&self, class: GeneSegmentClass) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.class == class)
            .count()
    }

    fn count_disposition(&self, disposition: GeneSegmentDisposition) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.disposition == disposition)
            .count()
    }

    fn count_lifecycle_state(&self, state: GeneScissorsLifecycleState) -> usize {
        self.lifecycle_records
            .iter()
            .filter(|record| record.state == state)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutDetector {
    policy: DnaSplicerPolicy,
}

impl MutDetector {
    pub fn new(policy: DnaSplicerPolicy) -> Self {
        Self { policy }
    }

    pub fn detect(&self, segments: &[GeneSegment]) -> Vec<MutationFinding> {
        let mut findings = Vec::new();

        for segment in segments {
            if segment.start_token >= segment.end_token {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::EmptyRange,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Repair,
                    "segment token range is empty or reversed",
                ));
            }
            if self.policy.require_source_hash && segment.source_hash.trim().is_empty() {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::MissingSourceHash,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Repair,
                    "segment is missing a source hash for audit and rollback",
                ));
            }
            if segment.token_count() > self.policy.max_segment_tokens {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::Truncation,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Splice,
                    "segment exceeds the splicer token budget and should be re-sliced",
                ));
            }
            if segment.has_stale_label(self.policy.max_segment_age) {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::StaleLabel,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Relabel,
                    "segment label, purpose, or age metadata is stale and cannot explain its function",
                ));
            }
            if has_contradictory_metadata(segment) {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::Contradiction,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Repair,
                    "segment metadata contains contradictory or conflicting rule evidence",
                ));
            }
            if segment.drift_score > self.policy.max_exon_drift {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::Drift,
                    GeneVariantSeverity::Quarantine,
                    GeneScissorsIntent::Quarantine,
                    "segment drift exceeds the safe exon threshold",
                ));
            }
            if segment.privacy_risk > self.policy.max_exon_privacy_risk {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::Privacy,
                    GeneVariantSeverity::Quarantine,
                    GeneScissorsIntent::Quarantine,
                    "segment privacy risk exceeds the safe exon threshold",
                ));
            }
            if !segment.schema_valid {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::Schema,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Repair,
                    "segment schema validation failed",
                ));
            }
            if !segment.kv_shape_valid {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::KvShape,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Repair,
                    "segment KV shape is not valid for runtime import",
                ));
            }
        }

        let mut low_fitness_signatures: Vec<(String, &GeneSegment)> = Vec::new();
        for segment in segments {
            if segment.fitness >= self.policy.min_exon_fitness {
                continue;
            }
            let signature = low_fitness_signature(segment);
            if low_fitness_signatures
                .iter()
                .any(|(existing, _)| existing == &signature)
            {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::LowFitnessRepetition,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Repair,
                    "repeated low-fitness segment pattern should be merged, decayed, or repaired",
                ));
            }
            low_fitness_signatures.push((signature, segment));
        }

        let mut ordered = segments.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|segment| {
            (
                segment.source.as_str(),
                segment.source_hash.as_str(),
                segment.start_token,
                segment.end_token,
            )
        });
        for window in ordered.windows(2) {
            let left = window[0];
            let right = window[1];
            if left.source != right.source || left.source_hash != right.source_hash {
                continue;
            }
            if right.start_token > left.end_token {
                findings.push(MutationFinding::new(
                    &right.id,
                    GeneVariantKind::Deletion,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Splice,
                    format!(
                        "token gap {}..{} between adjacent segments requires splice repair",
                        left.end_token, right.start_token
                    ),
                ));
            } else if right.start_token < left.end_token {
                let overlap_tokens = left.end_token.saturating_sub(right.start_token);
                if overlap_tokens <= self.policy.max_planned_overlap_tokens {
                    continue;
                }
                findings.push(MutationFinding::new(
                    &right.id,
                    GeneVariantKind::Insertion,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Splice,
                    format!(
                        "token overlap {}..{} exceeds planned overlap budget and requires splice repair",
                        right.start_token, left.end_token
                    ),
                ));
            }
        }

        findings
    }
}

impl Default for MutDetector {
    fn default() -> Self {
        Self::new(DnaSplicerPolicy::default())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MutFixer;

impl MutFixer {
    pub fn mutation_plans(
        &self,
        findings: &[MutationFinding],
        stable_anchor_id: impl Into<String>,
    ) -> Vec<MutationPlan> {
        let stable_anchor_id = stable_anchor_id.into();
        let mut plans = Vec::new();

        for finding in findings {
            match finding.suggested_intent {
                GeneScissorsIntent::Quarantine => {
                    push_plan_once(
                        &mut plans,
                        MutationPlan::preview(
                            format!("mutation:{}:quarantine", finding.segment_id),
                            GeneScissorsIntent::Quarantine,
                            finding.segment_id.clone(),
                            finding.reason.clone(),
                            "isolate the variant segment before it can influence expression or KV prefill",
                            stable_anchor_id.clone(),
                        )
                        .with_sources([finding.segment_id.clone()]),
                    );
                    push_plan_once(
                        &mut plans,
                        MutationPlan::preview(
                            format!("mutation:{}:regenerate", finding.segment_id),
                            GeneScissorsIntent::Regenerate,
                            finding.segment_id.clone(),
                            "regenerate a young replacement segment from the stable genome anchor",
                            "replace the quarantined segment only after validation gates pass",
                            stable_anchor_id.clone(),
                        )
                        .with_sources([stable_anchor_id.clone()])
                        .with_replacement(regenerated_segment_id(&finding.segment_id))
                        .with_repair_payload(
                            format!("regenerated {}", finding.segment_id),
                            "young replacement segment rebuilt from the stable genome anchor",
                            ["regenerate", "stable_anchor", finding.kind.as_str()],
                        ),
                    );
                    push_plan_once(
                        &mut plans,
                        MutationPlan::preview(
                            format!("mutation:{}:cut", finding.segment_id),
                            GeneScissorsIntent::Cut,
                            finding.segment_id.clone(),
                            "quarantined variant requires a reversible cut candidate",
                            "remove the variant from active expression only after replay validation, rollback evidence, and operator approval",
                            stable_anchor_id.clone(),
                        )
                        .with_sources([stable_anchor_id.clone()]),
                    );
                }
                GeneScissorsIntent::Relabel => {
                    push_plan_once(
                        &mut plans,
                        MutationPlan::preview(
                            format!("mutation:{}:relabel", finding.segment_id),
                            GeneScissorsIntent::Relabel,
                            finding.segment_id.clone(),
                            finding.reason.clone(),
                            "refresh segment label and purpose while preserving the stable anchor",
                            stable_anchor_id.clone(),
                        )
                        .with_sources([finding.segment_id.clone()])
                        .with_repair_payload(
                            format!("refreshed {}", finding.segment_id),
                            "restore the segment function label before reuse in expression or KV prefill",
                            ["relabel", "youth_renewal", finding.kind.as_str()],
                        ),
                    );
                }
                GeneScissorsIntent::Splice => push_plan_once(
                    &mut plans,
                    MutationPlan::preview(
                        format!("mutation:{}:splice", finding.segment_id),
                        GeneScissorsIntent::Splice,
                        finding.segment_id.clone(),
                        finding.reason.clone(),
                        "re-slice adjacent segments with bounded overlap and KV-safe anchors",
                        stable_anchor_id.clone(),
                    )
                    .with_sources([finding.segment_id.clone()]),
                ),
                _ => push_plan_once(
                    &mut plans,
                    MutationPlan::preview(
                        format!("mutation:{}:repair", finding.segment_id),
                        GeneScissorsIntent::Repair,
                        finding.segment_id.clone(),
                        finding.reason.clone(),
                        "repair segment metadata before any expression or KV import",
                        stable_anchor_id.clone(),
                    )
                    .with_sources([finding.segment_id.clone()]),
                ),
            }
        }

        plans
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DnaSplicer {
    policy: DnaSplicerPolicy,
}

impl DnaSplicer {
    pub fn new(policy: DnaSplicerPolicy) -> Self {
        Self { policy }
    }

    pub fn preview(
        &self,
        profile: TaskProfile,
        stable_anchor_id: impl Into<String>,
        segments: Vec<GeneSegment>,
    ) -> DnaSplicePreview {
        let stable_anchor_id = stable_anchor_id.into();
        let detector = MutDetector::new(self.policy.clone());
        let findings = detector.detect(&segments);
        let fixer = MutFixer;
        let mutation_plans = fixer.mutation_plans(&findings, stable_anchor_id.clone());
        let lifecycle_records = lifecycle_records_for_findings(
            &findings,
            &mutation_plans,
            &segments,
            &stable_anchor_id,
        );
        let classified_segments = segments
            .into_iter()
            .map(|segment| {
                let segment_findings = findings
                    .iter()
                    .filter(|finding| finding.segment_id == segment.id)
                    .collect::<Vec<_>>();
                let class = classify_segment(&self.policy, &segment, &segment_findings);
                let disposition = disposition_for_class(&class, &segment_findings);
                let reasons = segment_findings
                    .iter()
                    .map(|finding| finding.reason.clone())
                    .collect();
                ClassifiedGeneSegment {
                    segment,
                    class,
                    disposition,
                    reasons,
                }
            })
            .collect();

        DnaSplicePreview {
            profile,
            stable_anchor_id,
            segments: classified_segments,
            findings,
            mutation_plans,
            lifecycle_records,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }
}

impl Default for DnaSplicer {
    fn default() -> Self {
        Self::new(DnaSplicerPolicy::default())
    }
}

fn disposition_for_class(
    class: &GeneSegmentClass,
    findings: &[&MutationFinding],
) -> GeneSegmentDisposition {
    match class {
        GeneSegmentClass::Exon => GeneSegmentDisposition::Retained,
        GeneSegmentClass::Intron => GeneSegmentDisposition::Skipped,
        GeneSegmentClass::Variant
            if findings
                .iter()
                .any(|finding| finding.severity == GeneVariantSeverity::Quarantine) =>
        {
            GeneSegmentDisposition::Quarantined
        }
        GeneSegmentClass::Variant => GeneSegmentDisposition::RepairCandidate,
    }
}

fn lifecycle_records_for_findings(
    findings: &[MutationFinding],
    mutation_plans: &[MutationPlan],
    segments: &[GeneSegment],
    stable_anchor_id: &str,
) -> Vec<GeneScissorsLifecycleRecord> {
    let mut records = Vec::new();
    for finding in findings {
        if records.iter().any(|record: &GeneScissorsLifecycleRecord| {
            record.target_segment_id == finding.segment_id
        }) {
            continue;
        }
        let segment_findings = findings
            .iter()
            .filter(|candidate| candidate.segment_id == finding.segment_id)
            .collect::<Vec<_>>();
        records.push(GeneScissorsLifecycleRecord::preview(
            finding.segment_id.clone(),
            &segment_findings,
            mutation_plans,
            segments
                .iter()
                .find(|segment| segment.id == finding.segment_id),
            stable_anchor_id,
        ));
    }
    records
}

fn segment_source_digest(segment: Option<&GeneSegment>) -> String {
    segment
        .and_then(|segment| {
            (!segment.source_hash.trim().is_empty()).then(|| segment.source_hash.clone())
        })
        .unwrap_or_else(|| "missing_source_digest".to_owned())
}

fn segment_affected_scope(segment: Option<&GeneSegment>) -> String {
    segment
        .map(|segment| {
            format!(
                "{}:{}..{}",
                segment.source.as_str(),
                segment.start_token,
                segment.end_token
            )
        })
        .unwrap_or_else(|| "gene_segment".to_owned())
}

fn initial_lifecycle_state(findings: &[&MutationFinding]) -> GeneScissorsLifecycleState {
    if findings.is_empty() {
        return GeneScissorsLifecycleState::Detected;
    }
    if findings
        .iter()
        .any(|finding| finding.severity == GeneVariantSeverity::Quarantine)
    {
        GeneScissorsLifecycleState::Quarantined
    } else {
        GeneScissorsLifecycleState::RepairCandidate
    }
}

fn lifecycle_confidence(findings: &[&MutationFinding]) -> f32 {
    let mut confidence = 0.45_f32;
    for finding in findings {
        confidence = confidence.max(match finding.severity {
            GeneVariantSeverity::Watch => 0.55,
            GeneVariantSeverity::Repair => 0.74,
            GeneVariantSeverity::Quarantine => 0.92,
        });
    }
    confidence.min(0.99)
}

fn next_action_for_state(state: GeneScissorsLifecycleState) -> &'static str {
    match state {
        GeneScissorsLifecycleState::Detected => "collect_more_evidence",
        GeneScissorsLifecycleState::Quarantined => {
            "keep_isolated_generate_stable_anchor_replacement"
        }
        GeneScissorsLifecycleState::RepairCandidate => "validate_repair_candidate",
        GeneScissorsLifecycleState::Validated => "await_operator_approval_before_apply",
        GeneScissorsLifecycleState::Cut => "cut_from_active_expression_after_operator_approval",
        GeneScissorsLifecycleState::Held => "hold_candidate_for_more_evidence",
        GeneScissorsLifecycleState::Rejected => "reject_candidate_keep_rollback_anchor",
    }
}

fn segment_reason_summary(segment: &ClassifiedGeneSegment) -> String {
    let mut finding_kinds = Vec::new();
    for reason in &segment.reasons {
        let normalized = reason_kind_hint(reason);
        if !finding_kinds.contains(&normalized) {
            finding_kinds.push(normalized);
        }
    }
    if finding_kinds.is_empty() {
        finding_kinds.push("none".to_owned());
    }

    format!(
        "source={} class={} disposition={} tokens={} age={} decay={:.3} last_purpose_present={} kv={} hash_present={} findings={}",
        segment.segment.source.as_str(),
        segment.class.as_str(),
        segment.disposition.as_str(),
        segment.segment.token_count(),
        segment.segment.age,
        segment.segment.decay_score(),
        !segment.segment.last_confirmed_purpose.trim().is_empty(),
        segment.segment.kv_residency.as_str(),
        !segment.segment.source_hash.trim().is_empty(),
        finding_kinds.join("|")
    )
}

fn reason_kind_hint(reason: &str) -> String {
    for (needle, kind) in [
        ("token range", "empty_range"),
        ("source hash", "missing_source_hash"),
        ("token budget", "truncation"),
        ("label or purpose", "stale_label"),
        ("contradictory", "contradiction"),
        ("low-fitness", "low_fitness_repetition"),
        ("drift", "drift"),
        ("privacy", "privacy"),
        ("schema", "schema"),
        ("KV shape", "kv_shape"),
        ("token gap", "deletion"),
        ("token overlap", "insertion"),
    ] {
        if reason.contains(needle) {
            return kind.to_owned();
        }
    }
    "repair".to_owned()
}

fn has_contradictory_metadata(segment: &GeneSegment) -> bool {
    let label = segment.label.to_ascii_lowercase();
    let purpose = segment.purpose.to_ascii_lowercase();
    label.contains("contradict")
        || label.contains("conflict")
        || purpose.contains("contradict")
        || purpose.contains("conflict")
}

fn low_fitness_signature(segment: &GeneSegment) -> String {
    format!(
        "{}:{}:{}",
        segment.source.as_str(),
        segment.label.trim().to_ascii_lowercase(),
        segment.purpose.trim().to_ascii_lowercase()
    )
}

fn classify_segment(
    policy: &DnaSplicerPolicy,
    segment: &GeneSegment,
    findings: &[&MutationFinding],
) -> GeneSegmentClass {
    if !findings.is_empty() {
        return GeneSegmentClass::Variant;
    }
    if segment.fitness >= policy.min_exon_fitness
        && segment.drift_score <= policy.max_exon_drift
        && segment.privacy_risk <= policy.max_exon_privacy_risk
        && segment.token_count() <= policy.max_segment_tokens
        && segment.schema_valid
        && segment.kv_shape_valid
    {
        GeneSegmentClass::Exon
    } else {
        GeneSegmentClass::Intron
    }
}

fn push_plan_once(plans: &mut Vec<MutationPlan>, plan: MutationPlan) {
    if plans.iter().any(|existing| {
        existing.target_gene_id == plan.target_gene_id && existing.intent == plan.intent
    }) {
        return;
    }
    plans.push(plan);
}

fn regenerated_segment_id(segment_id: &str) -> String {
    format!("{segment_id}:young")
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}
