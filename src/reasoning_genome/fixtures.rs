use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

use super::model::{GeneScissorsIntent, GeneValidationStatus, MutationPlan};
use super::splicing::{
    DnaSplicePreview, DnaSplicer, DnaSplicerPolicy, GeneKvResidency, GeneScissorsLifecycleState,
    GeneSegment, GeneSegmentDisposition, GeneSegmentSource, GeneVariantKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationFixtureKind {
    BenignExon,
    Insertion,
    Deletion,
    Truncation,
    SchemaDrift,
    ContradictoryPolicy,
    StaleLabel,
    MaliciousInstruction,
}

impl MutationFixtureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BenignExon => "benign_exon",
            Self::Insertion => "insertion",
            Self::Deletion => "deletion",
            Self::Truncation => "truncation",
            Self::SchemaDrift => "schema_drift",
            Self::ContradictoryPolicy => "contradictory_policy",
            Self::StaleLabel => "stale_label",
            Self::MaliciousInstruction => "malicious_instruction",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutationRepairFixture {
    pub id: String,
    pub kind: MutationFixtureKind,
    pub profile: TaskProfile,
    pub stable_anchor_id: String,
    pub mutated_segment_id: Option<String>,
    pub expected_finding_kinds: Vec<GeneVariantKind>,
    pub expected_disposition: Option<GeneSegmentDisposition>,
    pub expected_lifecycle_state: Option<GeneScissorsLifecycleState>,
    pub expected_reason_fragments: Vec<String>,
    pub protected_segment_ids: Vec<String>,
    pub sanitized_payload_summary: String,
    pub payload_digest: String,
    pub segments: Vec<GeneSegment>,
}

impl MutationRepairFixture {
    pub fn is_malicious_fixture(&self) -> bool {
        self.kind == MutationFixtureKind::MaliciousInstruction
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationRepairCandidateFixture {
    pub fixture_id: String,
    pub plan_id: String,
    pub target_segment_id: String,
    pub intent: GeneScissorsIntent,
    pub before_digest: String,
    pub after_digest: String,
    pub rollback_anchor_id: String,
    pub validation_gates: Vec<String>,
    pub validation_status: GeneValidationStatus,
    pub preview_only: bool,
    pub admission_write_authorized: bool,
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutationRepairFixtureResult {
    pub fixture_id: String,
    pub kind: MutationFixtureKind,
    pub mutated_segment_id: Option<String>,
    pub finding_kinds: Vec<GeneVariantKind>,
    pub reason_matches: Vec<String>,
    pub mutated_disposition: Option<GeneSegmentDisposition>,
    pub lifecycle_state: Option<GeneScissorsLifecycleState>,
    pub protected_segment_ids: Vec<String>,
    pub protected_segments_retained: bool,
    pub payload_digest: String,
    pub sanitized_payload_summary: String,
    pub before_digest: Option<String>,
    pub repair_candidates: Vec<MutationRepairCandidateFixture>,
    pub review_packet_lines: Vec<String>,
    pub preview_only: bool,
    pub failures: Vec<String>,
}

impl MutationRepairFixtureResult {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn has_finding_kind(&self, kind: GeneVariantKind) -> bool {
        self.finding_kinds.contains(&kind)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutationRepairFixtureReport {
    pub results: Vec<MutationRepairFixtureResult>,
    pub covered_fixture_kinds: Vec<MutationFixtureKind>,
    pub missing_fixture_kinds: Vec<MutationFixtureKind>,
    pub total_repair_candidate_count: usize,
    pub total_review_packet_line_count: usize,
    pub preview_only: bool,
    pub passed: bool,
    pub failures: Vec<String>,
}

impl MutationRepairFixtureReport {
    pub fn passed(&self) -> bool {
        self.passed
    }

    pub fn result_for_kind(
        &self,
        kind: MutationFixtureKind,
    ) -> Option<&MutationRepairFixtureResult> {
        self.results.iter().find(|result| result.kind == kind)
    }

    pub fn gate_report(&self) -> MutationRepairFixtureGateReport {
        MutationRepairFixtureGateReport {
            passed: self.passed,
            covered_fixture_kinds: self.covered_fixture_kinds.clone(),
            missing_fixture_kinds: self.missing_fixture_kinds.clone(),
            failures: self.failures.clone(),
        }
    }

    pub fn summary(&self) -> String {
        let covered = self
            .covered_fixture_kinds
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join("|");
        let missing = self
            .missing_fixture_kinds
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join("|");
        format!(
            "mutation_fixture_corpus passed={} fixtures={} repair_candidates={} review_lines={} preview_only={} covered={} missing={}",
            self.passed,
            self.results.len(),
            self.total_repair_candidate_count,
            self.total_review_packet_line_count,
            self.preview_only,
            covered,
            missing
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationRepairFixtureGateReport {
    pub passed: bool,
    pub covered_fixture_kinds: Vec<MutationFixtureKind>,
    pub missing_fixture_kinds: Vec<MutationFixtureKind>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalignantGeneDrillKind {
    MaliciousInstructionSegment,
    FalseMemory,
    BadRoutingThreshold,
    ContradictoryRule,
    StaleLabel,
    IrreversibleDeleteAttempt,
}

impl MalignantGeneDrillKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MaliciousInstructionSegment => "malicious_instruction_segment",
            Self::FalseMemory => "false_memory",
            Self::BadRoutingThreshold => "bad_routing_threshold",
            Self::ContradictoryRule => "contradictory_rule",
            Self::StaleLabel => "stale_label",
            Self::IrreversibleDeleteAttempt => "irreversible_delete_attempt",
        }
    }

    pub fn expected_kinds() -> [Self; 6] {
        [
            Self::MaliciousInstructionSegment,
            Self::FalseMemory,
            Self::BadRoutingThreshold,
            Self::ContradictoryRule,
            Self::StaleLabel,
            Self::IrreversibleDeleteAttempt,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MalignantGeneRecoveryDrill {
    pub id: String,
    pub kind: MalignantGeneDrillKind,
    pub profile: TaskProfile,
    pub stable_anchor_id: String,
    pub target_segment_id: String,
    pub payload_digest: String,
    pub sanitized_payload_summary: String,
    pub replay_validation_status: GeneValidationStatus,
    pub expected_hold_reasons: Vec<String>,
    pub protected_segment_ids: Vec<String>,
    pub segments: Vec<GeneSegment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MalignantGeneRecoveryResult {
    pub fixture_id: String,
    pub kind: MalignantGeneDrillKind,
    pub target_segment_id: String,
    pub classification: String,
    pub confidence: f32,
    pub rollback_anchor_id: String,
    pub validation_status: GeneValidationStatus,
    pub redaction_status: String,
    pub payload_digest: String,
    pub quarantine_plan_present: bool,
    pub cut_candidate_present: bool,
    pub regeneration_candidate_present: bool,
    pub trusted_regeneration_sources: Vec<String>,
    pub copied_bad_payload_source: bool,
    pub tombstone_id: Option<String>,
    pub approval_decision: String,
    pub hold_reasons: Vec<String>,
    pub protected_segments_retained: bool,
    pub evidence_packet_lines: Vec<String>,
    pub preview_only: bool,
    pub failures: Vec<String>,
}

impl MalignantGeneRecoveryResult {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MalignantGeneRecoveryDrillReport {
    pub results: Vec<MalignantGeneRecoveryResult>,
    pub covered_fixture_kinds: Vec<MalignantGeneDrillKind>,
    pub missing_fixture_kinds: Vec<MalignantGeneDrillKind>,
    pub quarantined_count: usize,
    pub cut_candidate_count: usize,
    pub regeneration_candidate_count: usize,
    pub failed_replay_count: usize,
    pub preview_only: bool,
    pub passed: bool,
    pub failures: Vec<String>,
}

impl MalignantGeneRecoveryDrillReport {
    pub fn passed(&self) -> bool {
        self.passed
    }

    pub fn result_for_kind(
        &self,
        kind: MalignantGeneDrillKind,
    ) -> Option<&MalignantGeneRecoveryResult> {
        self.results.iter().find(|result| result.kind == kind)
    }

    pub fn summary(&self) -> String {
        let covered = self
            .covered_fixture_kinds
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join("|");
        let missing = self
            .missing_fixture_kinds
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join("|");
        format!(
            "malignant_gene_recovery_drills passed={} fixtures={} quarantined={} cut_candidates={} regeneration_candidates={} failed_replay={} preview_only={} covered={} missing={}",
            self.passed,
            self.results.len(),
            self.quarantined_count,
            self.cut_candidate_count,
            self.regeneration_candidate_count,
            self.failed_replay_count,
            self.preview_only,
            covered,
            missing
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MalignantGeneRecoveryDrillCorpus {
    pub policy: DnaSplicerPolicy,
    pub fixtures: Vec<MalignantGeneRecoveryDrill>,
}

impl MalignantGeneRecoveryDrillCorpus {
    pub fn new(policy: DnaSplicerPolicy, fixtures: Vec<MalignantGeneRecoveryDrill>) -> Self {
        Self { policy, fixtures }
    }

    pub fn default_corpus() -> Self {
        Self::new(
            DnaSplicerPolicy::default(),
            default_malignant_gene_recovery_drills(),
        )
    }

    pub fn evaluate(&self) -> MalignantGeneRecoveryDrillReport {
        let results = self
            .fixtures
            .iter()
            .map(|fixture| evaluate_malignant_drill(&self.policy, fixture))
            .collect::<Vec<_>>();
        let mut covered_fixture_kinds = Vec::new();
        for result in &results {
            push_drill_kind_once(&mut covered_fixture_kinds, result.kind);
        }
        let missing_fixture_kinds = MalignantGeneDrillKind::expected_kinds()
            .iter()
            .copied()
            .filter(|kind| !covered_fixture_kinds.contains(kind))
            .collect::<Vec<_>>();
        let quarantined_count = results
            .iter()
            .filter(|result| result.classification == "malignant_quarantined")
            .count();
        let cut_candidate_count = results
            .iter()
            .filter(|result| result.cut_candidate_present)
            .count();
        let regeneration_candidate_count = results
            .iter()
            .filter(|result| result.regeneration_candidate_present)
            .count();
        let failed_replay_count = results
            .iter()
            .filter(|result| result.validation_status == GeneValidationStatus::Failed)
            .count();
        let preview_only = results.iter().all(|result| result.preview_only);
        let mut failures = Vec::new();
        for result in &results {
            failures.extend(result.failures.iter().cloned());
        }
        for kind in &missing_fixture_kinds {
            failures.push(format!(
                "malignant_gene_drill_coverage_missing:{}",
                kind.as_str()
            ));
        }
        if !preview_only {
            failures.push("malignant_gene_drill_preview_only_gate_failed".to_owned());
        }
        let passed = failures.is_empty();

        MalignantGeneRecoveryDrillReport {
            results,
            covered_fixture_kinds,
            missing_fixture_kinds,
            quarantined_count,
            cut_candidate_count,
            regeneration_candidate_count,
            failed_replay_count,
            preview_only,
            passed,
            failures,
        }
    }
}

impl Default for MalignantGeneRecoveryDrillCorpus {
    fn default() -> Self {
        Self::default_corpus()
    }
}

pub fn default_malignant_gene_recovery_drill_corpus() -> MalignantGeneRecoveryDrillCorpus {
    MalignantGeneRecoveryDrillCorpus::default()
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutationRepairFixtureCorpus {
    pub policy: DnaSplicerPolicy,
    pub fixtures: Vec<MutationRepairFixture>,
}

impl MutationRepairFixtureCorpus {
    pub fn new(policy: DnaSplicerPolicy, fixtures: Vec<MutationRepairFixture>) -> Self {
        Self { policy, fixtures }
    }

    pub fn default_corpus() -> Self {
        Self::new(
            DnaSplicerPolicy::default(),
            default_mutation_repair_fixtures(),
        )
    }

    pub fn evaluate(&self) -> MutationRepairFixtureReport {
        let results = self
            .fixtures
            .iter()
            .map(|fixture| evaluate_fixture(&self.policy, fixture))
            .collect::<Vec<_>>();
        let mut covered_fixture_kinds = Vec::new();
        for result in &results {
            push_kind_once(&mut covered_fixture_kinds, result.kind);
        }
        let missing_fixture_kinds = required_fixture_kinds()
            .iter()
            .copied()
            .filter(|kind| !covered_fixture_kinds.contains(kind))
            .collect::<Vec<_>>();
        let total_repair_candidate_count = results
            .iter()
            .map(|result| result.repair_candidates.len())
            .sum();
        let total_review_packet_line_count = results
            .iter()
            .map(|result| result.review_packet_lines.len())
            .sum();
        let preview_only = results.iter().all(|result| result.preview_only);
        let mut failures = Vec::new();
        for result in &results {
            failures.extend(result.failures.iter().cloned());
        }
        for kind in &missing_fixture_kinds {
            failures.push(format!(
                "mutation_fixture_coverage_missing:{}",
                kind.as_str()
            ));
        }
        if !preview_only {
            failures.push("mutation_fixture_preview_only_gate_failed".to_owned());
        }
        let passed = failures.is_empty();

        MutationRepairFixtureReport {
            results,
            covered_fixture_kinds,
            missing_fixture_kinds,
            total_repair_candidate_count,
            total_review_packet_line_count,
            preview_only,
            passed,
            failures,
        }
    }
}

impl Default for MutationRepairFixtureCorpus {
    fn default() -> Self {
        Self::default_corpus()
    }
}

pub fn default_mutation_repair_fixture_corpus() -> MutationRepairFixtureCorpus {
    MutationRepairFixtureCorpus::default()
}

fn evaluate_malignant_drill(
    policy: &DnaSplicerPolicy,
    fixture: &MalignantGeneRecoveryDrill,
) -> MalignantGeneRecoveryResult {
    let preview = DnaSplicer::new(policy.clone()).preview(
        fixture.profile,
        fixture.stable_anchor_id.clone(),
        fixture.segments.clone(),
    );
    let target_id = fixture.target_segment_id.as_str();
    let target_findings = target_findings(&preview, Some(target_id));
    let target_disposition = preview
        .segments
        .iter()
        .find(|segment| segment.segment.id == target_id)
        .map(|segment| segment.disposition);
    let lifecycle_record = preview
        .lifecycle_records
        .iter()
        .find(|record| record.target_segment_id == target_id);
    let target_plans = preview
        .mutation_plans
        .iter()
        .filter(|plan| plan.target_gene_id == target_id)
        .collect::<Vec<_>>();
    let quarantine_plan_present = target_plans
        .iter()
        .any(|plan| plan.intent == GeneScissorsIntent::Quarantine);
    let cut_candidate_present = target_plans
        .iter()
        .any(|plan| plan.intent == GeneScissorsIntent::Cut);
    let regeneration_plan = target_plans
        .iter()
        .find(|plan| plan.intent == GeneScissorsIntent::Regenerate);
    let regeneration_candidate_present = regeneration_plan.is_some();
    let trusted_regeneration_sources = regeneration_plan
        .map(|plan| plan.source_gene_ids.clone())
        .unwrap_or_default();
    let copied_bad_payload_source = trusted_regeneration_sources
        .iter()
        .any(|source| source == target_id);
    let tombstone_id = cut_candidate_present.then(|| format!("tombstone:{target_id}"));
    let confidence = lifecycle_record
        .map(|record| record.confidence)
        .unwrap_or_default();
    let classification = if target_disposition == Some(GeneSegmentDisposition::Quarantined)
        && target_findings
            .iter()
            .any(|finding| finding.severity.as_str() == "quarantine")
    {
        "malignant_quarantined"
    } else {
        "not_quarantined"
    }
    .to_owned();
    let hold_reasons = hold_reasons_for_drill(fixture, &preview, &target_plans);
    let approval_decision = approval_decision_for_validation(fixture.replay_validation_status);
    let protected_segments_retained = fixture.protected_segment_ids.iter().all(|id| {
        preview.segments.iter().any(|segment| {
            segment.segment.id == *id && segment.disposition == GeneSegmentDisposition::Retained
        })
    });
    let preview_only = preview.is_read_only_preview()
        && target_plans
            .iter()
            .all(|plan| plan.is_read_only_preview() && !plan.admission_write_authorized);
    let mut evidence_packet_lines = malignant_drill_evidence_lines(
        fixture,
        &classification,
        confidence,
        quarantine_plan_present,
        cut_candidate_present,
        regeneration_candidate_present,
        &trusted_regeneration_sources,
        tombstone_id.as_deref(),
        &approval_decision,
        &hold_reasons,
        preview_only,
    );
    let redaction_status = if evidence_packet_lines
        .iter()
        .any(|line| contains_private_or_executable_marker(line))
    {
        "leaked"
    } else {
        "redacted"
    }
    .to_owned();
    for line in &mut evidence_packet_lines {
        line.push_str(&format!(" redaction_status={redaction_status}"));
    }
    let failures = malignant_drill_failures(
        fixture,
        &preview,
        &target_findings,
        target_disposition,
        lifecycle_record,
        quarantine_plan_present,
        cut_candidate_present,
        regeneration_plan.copied(),
        &trusted_regeneration_sources,
        copied_bad_payload_source,
        tombstone_id.as_deref(),
        &approval_decision,
        &hold_reasons,
        protected_segments_retained,
        &evidence_packet_lines,
        preview_only,
        &redaction_status,
    );

    MalignantGeneRecoveryResult {
        fixture_id: fixture.id.clone(),
        kind: fixture.kind,
        target_segment_id: fixture.target_segment_id.clone(),
        classification,
        confidence,
        rollback_anchor_id: fixture.stable_anchor_id.clone(),
        validation_status: fixture.replay_validation_status,
        redaction_status,
        payload_digest: fixture.payload_digest.clone(),
        quarantine_plan_present,
        cut_candidate_present,
        regeneration_candidate_present,
        trusted_regeneration_sources,
        copied_bad_payload_source,
        tombstone_id,
        approval_decision,
        hold_reasons,
        protected_segments_retained,
        evidence_packet_lines,
        preview_only,
        failures,
    }
}

fn evaluate_fixture(
    policy: &DnaSplicerPolicy,
    fixture: &MutationRepairFixture,
) -> MutationRepairFixtureResult {
    let preview = DnaSplicer::new(policy.clone()).preview(
        fixture.profile,
        fixture.stable_anchor_id.clone(),
        fixture.segments.clone(),
    );
    let target_id = fixture.mutated_segment_id.as_deref();
    let finding_kinds = target_findings(&preview, target_id)
        .iter()
        .map(|finding| finding.kind)
        .collect::<Vec<_>>();
    let reason_matches = expected_reason_matches(&preview, target_id, fixture);
    let mutated_disposition = target_id.and_then(|id| {
        preview
            .segments
            .iter()
            .find(|segment| segment.segment.id == id)
            .map(|segment| segment.disposition)
    });
    let lifecycle_state = target_id.and_then(|id| {
        preview
            .lifecycle_records
            .iter()
            .find(|record| record.target_segment_id == id)
            .map(|record| record.state)
    });
    let protected_segments_retained = protected_segments_retained(&preview, fixture);
    let before_digest = target_id
        .and_then(|id| fixture.segments.iter().find(|segment| segment.id == id))
        .map(segment_digest);
    let repair_candidates = repair_candidate_fixtures(&preview, fixture, before_digest.as_deref());
    let preview_only = preview.is_read_only_preview()
        && repair_candidates
            .iter()
            .all(|candidate| candidate.preview_only && !candidate.admission_write_authorized);
    let review_packet_lines = review_packet_lines(
        fixture,
        &finding_kinds,
        mutated_disposition,
        lifecycle_state,
        &repair_candidates,
        &before_digest,
        preview_only,
    );
    let failures = fixture_failures(
        fixture,
        &preview,
        &finding_kinds,
        mutated_disposition,
        lifecycle_state,
        protected_segments_retained,
        &repair_candidates,
        preview_only,
        &reason_matches,
    );

    MutationRepairFixtureResult {
        fixture_id: fixture.id.clone(),
        kind: fixture.kind,
        mutated_segment_id: fixture.mutated_segment_id.clone(),
        finding_kinds,
        reason_matches,
        mutated_disposition,
        lifecycle_state,
        protected_segment_ids: fixture.protected_segment_ids.clone(),
        protected_segments_retained,
        payload_digest: fixture.payload_digest.clone(),
        sanitized_payload_summary: fixture.sanitized_payload_summary.clone(),
        before_digest,
        repair_candidates,
        review_packet_lines,
        preview_only,
        failures,
    }
}

fn default_mutation_repair_fixtures() -> Vec<MutationRepairFixture> {
    vec![
        benign_exon_fixture(),
        insertion_fixture(),
        deletion_fixture(),
        truncation_fixture(),
        schema_drift_fixture(),
        contradictory_policy_fixture(),
        stale_label_fixture(),
        malicious_instruction_fixture(),
    ]
}

fn default_malignant_gene_recovery_drills() -> Vec<MalignantGeneRecoveryDrill> {
    vec![
        malignant_drill_fixture(
            "drill:malicious-instruction",
            MalignantGeneDrillKind::MaliciousInstructionSegment,
            "segment:drill-malicious",
            GeneSegmentSource::ToolOutput,
            "redacted adversarial instruction marker",
            "quarantine digest-only tool instruction marker before expression",
            "digest-only malicious instruction simulation",
            GeneValidationStatus::Pending,
            ["replay_validation_pending", "operator_approval_required"],
            0.31,
            0.94,
            0.90,
            2,
        ),
        malignant_drill_fixture(
            "drill:false-memory",
            MalignantGeneDrillKind::FalseMemory,
            "segment:drill-false-memory",
            GeneSegmentSource::SemanticMemory,
            "false memory conflict",
            "contradict stable memory validation evidence",
            "digest-only false memory simulation",
            GeneValidationStatus::Pending,
            ["replay_validation_pending", "operator_approval_required"],
            0.28,
            0.88,
            0.24,
            3,
        ),
        malignant_drill_fixture(
            "drill:bad-routing-threshold",
            MalignantGeneDrillKind::BadRoutingThreshold,
            "segment:drill-bad-routing",
            GeneSegmentSource::GenomeLedger,
            "bad routing threshold conflict",
            "contradict stable route budget and force unsafe attention fanout",
            "digest-only bad routing threshold simulation",
            GeneValidationStatus::Pending,
            ["replay_validation_pending", "operator_approval_required"],
            0.34,
            0.86,
            0.22,
            4,
        ),
        malignant_drill_fixture(
            "drill:contradictory-rule",
            MalignantGeneDrillKind::ContradictoryRule,
            "segment:drill-contradictory-rule",
            GeneSegmentSource::ToolOutput,
            "conflicting safety rule",
            "contradict rollback and validation requirements",
            "digest-only contradictory rule simulation",
            GeneValidationStatus::Pending,
            ["replay_validation_pending", "operator_approval_required"],
            0.37,
            0.82,
            0.23,
            5,
        ),
        malignant_drill_fixture(
            "drill:stale-label",
            MalignantGeneDrillKind::StaleLabel,
            "segment:drill-stale-label",
            GeneSegmentSource::SemanticMemory,
            "",
            "",
            "digest-only stale label simulation",
            GeneValidationStatus::Pending,
            ["replay_validation_pending", "operator_approval_required"],
            0.42,
            0.74,
            0.21,
            14,
        ),
        malignant_drill_fixture(
            "drill:irreversible-delete-attempt",
            MalignantGeneDrillKind::IrreversibleDeleteAttempt,
            "segment:drill-irreversible-delete",
            GeneSegmentSource::ToolOutput,
            "irreversible delete attempt",
            "conflict with rollback anchors by requesting destructive removal",
            "digest-only irreversible delete simulation",
            GeneValidationStatus::Failed,
            [
                "destructive_intent_blocked",
                "replay_validation_failed",
                "operator_approval_required",
            ],
            0.20,
            0.96,
            0.92,
            1,
        ),
    ]
}

fn benign_exon_fixture() -> MutationRepairFixture {
    let id = "fixture:benign-exon";
    let protected = vec![
        "segment:benign-express".to_owned(),
        "segment:benign-memory".to_owned(),
    ];
    MutationRepairFixture {
        id: id.to_owned(),
        kind: MutationFixtureKind::BenignExon,
        profile: TaskProfile::Coding,
        stable_anchor_id: "genome:fixture:stable".to_owned(),
        mutated_segment_id: None,
        expected_finding_kinds: Vec::new(),
        expected_disposition: None,
        expected_lifecycle_state: None,
        expected_reason_fragments: Vec::new(),
        protected_segment_ids: protected,
        sanitized_payload_summary: "synthetic benign compiler and memory evidence".to_owned(),
        payload_digest: digest_for_fixture(id, "synthetic benign compiler and memory evidence"),
        segments: vec![
            healthy_segment(
                "segment:benign-express",
                GeneSegmentSource::GenomeLedger,
                "sha256:fixture-benign-express",
                0,
                64,
                "benign express compiler evidence",
                "preserve validated compiler repair posture",
            ),
            healthy_segment(
                "segment:benign-memory",
                GeneSegmentSource::SemanticMemory,
                "sha256:fixture-benign-memory",
                64,
                128,
                "benign memory evidence",
                "preserve bounded memory retrieval posture",
            ),
        ],
    }
}

fn insertion_fixture() -> MutationRepairFixture {
    let id = "fixture:insertion";
    MutationRepairFixture {
        id: id.to_owned(),
        kind: MutationFixtureKind::Insertion,
        profile: TaskProfile::LongDocument,
        stable_anchor_id: "genome:fixture:stable".to_owned(),
        mutated_segment_id: Some("segment:insertion-overlap".to_owned()),
        expected_finding_kinds: vec![GeneVariantKind::Insertion],
        expected_disposition: Some(GeneSegmentDisposition::RepairCandidate),
        expected_lifecycle_state: Some(GeneScissorsLifecycleState::RepairCandidate),
        expected_reason_fragments: vec!["token overlap".to_owned()],
        protected_segment_ids: vec![
            "segment:insertion-left".to_owned(),
            "segment:insertion-memory".to_owned(),
        ],
        sanitized_payload_summary: "synthetic oversized overlap fixture".to_owned(),
        payload_digest: digest_for_fixture(id, "synthetic oversized overlap fixture"),
        segments: vec![
            healthy_segment(
                "segment:insertion-left",
                GeneSegmentSource::Prompt,
                "sha256:fixture-insertion-chain",
                0,
                400,
                "left source exon",
                "preserve source ordering before overlap repair",
            ),
            healthy_segment(
                "segment:insertion-overlap",
                GeneSegmentSource::Prompt,
                "sha256:fixture-insertion-chain",
                128,
                384,
                "overlap candidate",
                "repair accidental duplicate span before KV import",
            ),
            healthy_segment(
                "segment:insertion-memory",
                GeneSegmentSource::SemanticMemory,
                "sha256:fixture-insertion-memory",
                0,
                64,
                "independent memory exon",
                "stay isolated from overlap repair",
            ),
        ],
    }
}

fn deletion_fixture() -> MutationRepairFixture {
    let id = "fixture:deletion";
    MutationRepairFixture {
        id: id.to_owned(),
        kind: MutationFixtureKind::Deletion,
        profile: TaskProfile::LongDocument,
        stable_anchor_id: "genome:fixture:stable".to_owned(),
        mutated_segment_id: Some("segment:deletion-gap-right".to_owned()),
        expected_finding_kinds: vec![GeneVariantKind::Deletion],
        expected_disposition: Some(GeneSegmentDisposition::RepairCandidate),
        expected_lifecycle_state: Some(GeneScissorsLifecycleState::RepairCandidate),
        expected_reason_fragments: vec!["token gap".to_owned()],
        protected_segment_ids: vec![
            "segment:deletion-left".to_owned(),
            "segment:deletion-memory".to_owned(),
        ],
        sanitized_payload_summary: "synthetic missing middle span fixture".to_owned(),
        payload_digest: digest_for_fixture(id, "synthetic missing middle span fixture"),
        segments: vec![
            healthy_segment(
                "segment:deletion-left",
                GeneSegmentSource::Prompt,
                "sha256:fixture-deletion-chain",
                0,
                64,
                "left source exon",
                "preserve source ordering before deletion repair",
            ),
            healthy_segment(
                "segment:deletion-gap-right",
                GeneSegmentSource::Prompt,
                "sha256:fixture-deletion-chain",
                96,
                160,
                "gap repair candidate",
                "repair missing source span before context prefill",
            ),
            healthy_segment(
                "segment:deletion-memory",
                GeneSegmentSource::GistMemory,
                "sha256:fixture-deletion-memory",
                0,
                64,
                "independent gist exon",
                "stay isolated from deletion repair",
            ),
        ],
    }
}

fn truncation_fixture() -> MutationRepairFixture {
    let id = "fixture:truncation";
    MutationRepairFixture {
        id: id.to_owned(),
        kind: MutationFixtureKind::Truncation,
        profile: TaskProfile::LongDocument,
        stable_anchor_id: "genome:fixture:stable".to_owned(),
        mutated_segment_id: Some("segment:truncation-oversize".to_owned()),
        expected_finding_kinds: vec![GeneVariantKind::Truncation],
        expected_disposition: Some(GeneSegmentDisposition::RepairCandidate),
        expected_lifecycle_state: Some(GeneScissorsLifecycleState::RepairCandidate),
        expected_reason_fragments: vec!["token budget".to_owned()],
        protected_segment_ids: vec![
            "segment:truncation-express".to_owned(),
            "segment:truncation-memory".to_owned(),
        ],
        sanitized_payload_summary: "synthetic over-budget segment fixture".to_owned(),
        payload_digest: digest_for_fixture(id, "synthetic over-budget segment fixture"),
        segments: vec![
            healthy_segment(
                "segment:truncation-express",
                GeneSegmentSource::GenomeLedger,
                "sha256:fixture-truncation-express",
                0,
                48,
                "healthy express exon",
                "stay retained while oversize neighbor is sliced",
            ),
            healthy_segment(
                "segment:truncation-oversize",
                GeneSegmentSource::ToolOutput,
                "sha256:fixture-truncation-oversize",
                0,
                700,
                "oversize evidence candidate",
                "split over-budget tool evidence before reuse",
            ),
            healthy_segment(
                "segment:truncation-memory",
                GeneSegmentSource::SemanticMemory,
                "sha256:fixture-truncation-memory",
                48,
                96,
                "healthy memory exon",
                "stay retained while oversize neighbor is sliced",
            ),
        ],
    }
}

fn schema_drift_fixture() -> MutationRepairFixture {
    let id = "fixture:schema-drift";
    MutationRepairFixture {
        id: id.to_owned(),
        kind: MutationFixtureKind::SchemaDrift,
        profile: TaskProfile::Coding,
        stable_anchor_id: "genome:fixture:stable".to_owned(),
        mutated_segment_id: Some("segment:schema-drift".to_owned()),
        expected_finding_kinds: vec![GeneVariantKind::Schema, GeneVariantKind::Drift],
        expected_disposition: Some(GeneSegmentDisposition::Quarantined),
        expected_lifecycle_state: Some(GeneScissorsLifecycleState::Quarantined),
        expected_reason_fragments: vec!["schema".to_owned(), "drift".to_owned()],
        protected_segment_ids: vec![
            "segment:schema-drift-express".to_owned(),
            "segment:schema-drift-memory".to_owned(),
        ],
        sanitized_payload_summary: "synthetic malformed schema with high drift fixture".to_owned(),
        payload_digest: digest_for_fixture(
            id,
            "synthetic malformed schema with high drift fixture",
        ),
        segments: vec![
            healthy_segment(
                "segment:schema-drift-express",
                GeneSegmentSource::GenomeLedger,
                "sha256:fixture-schema-express",
                0,
                64,
                "healthy express exon",
                "preserve stable policy while schema drift is isolated",
            ),
            healthy_segment(
                "segment:schema-drift",
                GeneSegmentSource::RuntimeKv,
                "sha256:fixture-schema-drift",
                64,
                96,
                "schema drift variant",
                "quarantine malformed runtime KV schema before import",
            )
            .with_schema(false, true)
            .with_health(0.78, 0.82, 0.01),
            healthy_segment(
                "segment:schema-drift-memory",
                GeneSegmentSource::SemanticMemory,
                "sha256:fixture-schema-memory",
                96,
                160,
                "healthy memory exon",
                "preserve unrelated memory while schema drift is isolated",
            ),
        ],
    }
}

fn contradictory_policy_fixture() -> MutationRepairFixture {
    let id = "fixture:contradictory-policy";
    MutationRepairFixture {
        id: id.to_owned(),
        kind: MutationFixtureKind::ContradictoryPolicy,
        profile: TaskProfile::Coding,
        stable_anchor_id: "genome:fixture:stable".to_owned(),
        mutated_segment_id: Some("segment:contradictory-policy".to_owned()),
        expected_finding_kinds: vec![GeneVariantKind::Contradiction],
        expected_disposition: Some(GeneSegmentDisposition::RepairCandidate),
        expected_lifecycle_state: Some(GeneScissorsLifecycleState::RepairCandidate),
        expected_reason_fragments: vec!["contradictory".to_owned()],
        protected_segment_ids: vec![
            "segment:contradictory-express".to_owned(),
            "segment:contradictory-memory".to_owned(),
        ],
        sanitized_payload_summary: "synthetic conflicting policy metadata fixture".to_owned(),
        payload_digest: digest_for_fixture(id, "synthetic conflicting policy metadata fixture"),
        segments: vec![
            healthy_segment(
                "segment:contradictory-express",
                GeneSegmentSource::GenomeLedger,
                "sha256:fixture-contradict-express",
                0,
                64,
                "healthy express exon",
                "preserve stable policy while target metadata is repaired",
            ),
            healthy_segment(
                "segment:contradictory-policy",
                GeneSegmentSource::ToolOutput,
                "sha256:fixture-contradict-policy",
                64,
                96,
                "conflicting tool rule",
                "contradict stable compiler validation evidence",
            ),
            healthy_segment(
                "segment:contradictory-memory",
                GeneSegmentSource::SemanticMemory,
                "sha256:fixture-contradict-memory",
                96,
                160,
                "healthy memory exon",
                "preserve unrelated memory while target metadata is repaired",
            ),
        ],
    }
}

fn stale_label_fixture() -> MutationRepairFixture {
    let id = "fixture:stale-label";
    MutationRepairFixture {
        id: id.to_owned(),
        kind: MutationFixtureKind::StaleLabel,
        profile: TaskProfile::General,
        stable_anchor_id: "genome:fixture:stable".to_owned(),
        mutated_segment_id: Some("segment:stale-label".to_owned()),
        expected_finding_kinds: vec![GeneVariantKind::StaleLabel],
        expected_disposition: Some(GeneSegmentDisposition::RepairCandidate),
        expected_lifecycle_state: Some(GeneScissorsLifecycleState::RepairCandidate),
        expected_reason_fragments: vec!["stale".to_owned()],
        protected_segment_ids: vec![
            "segment:stale-express".to_owned(),
            "segment:stale-memory".to_owned(),
        ],
        sanitized_payload_summary: "synthetic aged label fixture".to_owned(),
        payload_digest: digest_for_fixture(id, "synthetic aged label fixture"),
        segments: vec![
            healthy_segment(
                "segment:stale-express",
                GeneSegmentSource::GenomeLedger,
                "sha256:fixture-stale-express",
                0,
                64,
                "healthy express exon",
                "preserve stable anchor while aged label is refreshed",
            ),
            healthy_segment(
                "segment:stale-label",
                GeneSegmentSource::SemanticMemory,
                "sha256:fixture-stale-label",
                64,
                96,
                "aged heuristic label",
                "carry a validated memory heuristic",
            )
            .with_last_confirmed_purpose("validated memory heuristic")
            .with_age(12),
            healthy_segment(
                "segment:stale-memory",
                GeneSegmentSource::GistMemory,
                "sha256:fixture-stale-memory",
                96,
                160,
                "healthy gist exon",
                "preserve unrelated gist while aged label is refreshed",
            ),
        ],
    }
}

fn malicious_instruction_fixture() -> MutationRepairFixture {
    let id = "fixture:malicious-instruction";
    let payload_summary = "redacted inert malicious-instruction marker with digest-only payload";
    MutationRepairFixture {
        id: id.to_owned(),
        kind: MutationFixtureKind::MaliciousInstruction,
        profile: TaskProfile::Coding,
        stable_anchor_id: "genome:fixture:stable".to_owned(),
        mutated_segment_id: Some("segment:malicious-instruction".to_owned()),
        expected_finding_kinds: vec![GeneVariantKind::Drift, GeneVariantKind::Privacy],
        expected_disposition: Some(GeneSegmentDisposition::Quarantined),
        expected_lifecycle_state: Some(GeneScissorsLifecycleState::Quarantined),
        expected_reason_fragments: vec!["drift".to_owned(), "privacy".to_owned()],
        protected_segment_ids: vec![
            "segment:malicious-express".to_owned(),
            "segment:malicious-memory".to_owned(),
        ],
        sanitized_payload_summary: payload_summary.to_owned(),
        payload_digest: digest_for_fixture(id, payload_summary),
        segments: vec![
            healthy_segment(
                "segment:malicious-express",
                GeneSegmentSource::GenomeLedger,
                "sha256:fixture-malicious-express",
                0,
                64,
                "healthy express safety exon",
                "preserve stable safety rule while variant is quarantined",
            ),
            healthy_segment(
                "segment:malicious-instruction",
                GeneSegmentSource::ToolOutput,
                "sha256:fixture-malicious-redacted",
                64,
                96,
                "redacted malicious instruction variant",
                "quarantine digest-only adversarial instruction marker before reuse",
            )
            .with_health(0.70, 0.92, 0.88),
            healthy_segment(
                "segment:malicious-memory",
                GeneSegmentSource::SemanticMemory,
                "sha256:fixture-malicious-memory",
                96,
                160,
                "healthy memory safety exon",
                "preserve unrelated safety memory while variant is quarantined",
            ),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn malignant_drill_fixture(
    id: &str,
    kind: MalignantGeneDrillKind,
    target_segment_id: &str,
    source: GeneSegmentSource,
    label: &str,
    purpose: &str,
    payload_summary: &str,
    replay_validation_status: GeneValidationStatus,
    expected_hold_reasons: impl IntoIterator<Item = &'static str>,
    fitness: f32,
    drift_score: f32,
    privacy_risk: f32,
    age: u32,
) -> MalignantGeneRecoveryDrill {
    let stable_anchor_id = "genome:malignant-drill:stable";
    let protected_segment_ids = vec![
        format!("{target_segment_id}:healthy-left"),
        format!("{target_segment_id}:healthy-right"),
    ];
    let mut target = GeneSegment::new(target_segment_id, TaskProfile::Coding, source, 64, 96)
        .with_source_hash(format!("sha256:{}", id.replace(':', "-")))
        .with_metadata(label, purpose, format!("synthetic drill summary for {id}"))
        .with_kv_residency(GeneKvResidency::Sink)
        .with_age(age)
        .with_health(fitness, drift_score, privacy_risk);
    if label.is_empty() || purpose.is_empty() {
        target = target.with_last_confirmed_purpose("");
    }

    MalignantGeneRecoveryDrill {
        id: id.to_owned(),
        kind,
        profile: TaskProfile::Coding,
        stable_anchor_id: stable_anchor_id.to_owned(),
        target_segment_id: target_segment_id.to_owned(),
        payload_digest: stable_redaction_digest([id, payload_summary]),
        sanitized_payload_summary: payload_summary.to_owned(),
        replay_validation_status,
        expected_hold_reasons: expected_hold_reasons
            .into_iter()
            .map(str::to_owned)
            .collect(),
        protected_segment_ids: protected_segment_ids.clone(),
        segments: vec![
            healthy_segment(
                &protected_segment_ids[0],
                GeneSegmentSource::GenomeLedger,
                &format!("sha256:{}-left", id.replace(':', "-")),
                0,
                64,
                "healthy recovery anchor",
                "preserve stable recovery behavior while target is isolated",
            ),
            target,
            healthy_segment(
                &protected_segment_ids[1],
                GeneSegmentSource::SemanticMemory,
                &format!("sha256:{}-right", id.replace(':', "-")),
                96,
                160,
                "healthy recovery memory",
                "preserve unrelated useful memory while target is isolated",
            ),
        ],
    }
}

fn healthy_segment(
    id: &str,
    source: GeneSegmentSource,
    source_hash: &str,
    start_token: usize,
    end_token: usize,
    label: &str,
    purpose: &str,
) -> GeneSegment {
    GeneSegment::new(id, TaskProfile::Coding, source, start_token, end_token)
        .with_source_hash(source_hash)
        .with_metadata(
            label,
            purpose,
            format!("synthetic fixture summary for {id}"),
        )
        .with_kv_residency(GeneKvResidency::ColdEvidence)
        .with_health(0.91, 0.03, 0.01)
}

fn required_fixture_kinds() -> &'static [MutationFixtureKind] {
    &[
        MutationFixtureKind::BenignExon,
        MutationFixtureKind::Insertion,
        MutationFixtureKind::Deletion,
        MutationFixtureKind::Truncation,
        MutationFixtureKind::SchemaDrift,
        MutationFixtureKind::ContradictoryPolicy,
        MutationFixtureKind::StaleLabel,
        MutationFixtureKind::MaliciousInstruction,
    ]
}

fn target_findings<'a>(
    preview: &'a DnaSplicePreview,
    target_id: Option<&str>,
) -> Vec<&'a super::splicing::MutationFinding> {
    match target_id {
        Some(id) => preview
            .findings
            .iter()
            .filter(|finding| finding.segment_id == id)
            .collect(),
        None => preview.findings.iter().collect(),
    }
}

fn expected_reason_matches(
    preview: &DnaSplicePreview,
    target_id: Option<&str>,
    fixture: &MutationRepairFixture,
) -> Vec<String> {
    let target_findings = target_findings(preview, target_id);
    fixture
        .expected_reason_fragments
        .iter()
        .filter(|fragment| {
            let fragment = fragment.to_ascii_lowercase();
            target_findings
                .iter()
                .any(|finding| finding.reason.to_ascii_lowercase().contains(&fragment))
        })
        .cloned()
        .collect()
}

fn protected_segments_retained(
    preview: &DnaSplicePreview,
    fixture: &MutationRepairFixture,
) -> bool {
    fixture.protected_segment_ids.iter().all(|id| {
        preview.segments.iter().any(|segment| {
            segment.segment.id == *id && segment.disposition == GeneSegmentDisposition::Retained
        })
    })
}

fn repair_candidate_fixtures(
    preview: &DnaSplicePreview,
    fixture: &MutationRepairFixture,
    before_digest: Option<&str>,
) -> Vec<MutationRepairCandidateFixture> {
    let Some(target_id) = fixture.mutated_segment_id.as_deref() else {
        return Vec::new();
    };
    let before_digest = before_digest.unwrap_or("fixture-digest:missing-before");
    preview
        .mutation_plans
        .iter()
        .filter(|plan| plan.target_gene_id == target_id)
        .map(|plan| repair_candidate_fixture(fixture, plan, before_digest))
        .collect()
}

fn repair_candidate_fixture(
    fixture: &MutationRepairFixture,
    plan: &MutationPlan,
    before_digest: &str,
) -> MutationRepairCandidateFixture {
    MutationRepairCandidateFixture {
        fixture_id: fixture.id.clone(),
        plan_id: plan.id.clone(),
        target_segment_id: plan.target_gene_id.clone(),
        intent: plan.intent,
        before_digest: before_digest.to_owned(),
        after_digest: plan_digest(plan),
        rollback_anchor_id: plan.rollback_anchor_id.clone(),
        validation_gates: plan.validation_gates.clone(),
        validation_status: plan.validation_status,
        preview_only: plan.is_read_only_preview(),
        admission_write_authorized: plan.admission_write_authorized,
        applied: plan.applied,
    }
}

fn fixture_failures(
    fixture: &MutationRepairFixture,
    preview: &DnaSplicePreview,
    finding_kinds: &[GeneVariantKind],
    mutated_disposition: Option<GeneSegmentDisposition>,
    lifecycle_state: Option<GeneScissorsLifecycleState>,
    protected_segments_retained: bool,
    repair_candidates: &[MutationRepairCandidateFixture],
    preview_only: bool,
    reason_matches: &[String],
) -> Vec<String> {
    let mut failures = Vec::new();

    if fixture.expected_finding_kinds.is_empty() {
        if !preview.findings.is_empty() {
            failures.push(format!(
                "{}:unexpected_findings:{}",
                fixture.id,
                preview.findings.len()
            ));
        }
    } else {
        for expected in &fixture.expected_finding_kinds {
            if !finding_kinds.contains(expected) {
                failures.push(format!(
                    "{}:missing_finding:{}",
                    fixture.id,
                    expected.as_str()
                ));
            }
        }
    }

    if mutated_disposition != fixture.expected_disposition {
        failures.push(format!(
            "{}:disposition_mismatch:expected={:?}:actual={:?}",
            fixture.id, fixture.expected_disposition, mutated_disposition
        ));
    }
    if lifecycle_state != fixture.expected_lifecycle_state {
        failures.push(format!(
            "{}:lifecycle_mismatch:expected={:?}:actual={:?}",
            fixture.id, fixture.expected_lifecycle_state, lifecycle_state
        ));
    }
    for expected in &fixture.expected_reason_fragments {
        if !reason_matches.contains(expected) {
            failures.push(format!(
                "{}:missing_reason_fragment:{}",
                fixture.id, expected
            ));
        }
    }
    if !protected_segments_retained {
        failures.push(format!("{}:neighbor_isolation_failed", fixture.id));
    }
    if fixture.mutated_segment_id.is_some() && repair_candidates.is_empty() {
        failures.push(format!("{}:repair_candidate_missing", fixture.id));
    }
    for candidate in repair_candidates {
        if candidate.rollback_anchor_id != fixture.stable_anchor_id {
            failures.push(format!(
                "{}:rollback_anchor_mismatch:{}",
                fixture.id, candidate.plan_id
            ));
        }
        if candidate.before_digest == candidate.after_digest {
            failures.push(format!(
                "{}:repair_digest_not_changed:{}",
                fixture.id, candidate.plan_id
            ));
        }
        if !candidate.preview_only
            || candidate.admission_write_authorized
            || candidate.applied
            || candidate.validation_status != GeneValidationStatus::Pending
        {
            failures.push(format!(
                "{}:repair_candidate_not_preview_only:{}",
                fixture.id, candidate.plan_id
            ));
        }
    }
    if !preview_only {
        failures.push(format!("{}:preview_only_gate_failed", fixture.id));
    }
    if fixture.is_malicious_fixture() && !fixture.sanitized_payload_summary.contains("digest-only")
    {
        failures.push(format!("{}:malicious_fixture_not_digest_only", fixture.id));
    }

    failures
}

fn review_packet_lines(
    fixture: &MutationRepairFixture,
    finding_kinds: &[GeneVariantKind],
    disposition: Option<GeneSegmentDisposition>,
    lifecycle_state: Option<GeneScissorsLifecycleState>,
    repair_candidates: &[MutationRepairCandidateFixture],
    before_digest: &Option<String>,
    preview_only: bool,
) -> Vec<String> {
    let finding_summary = finding_kinds
        .iter()
        .map(|kind| kind.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let before = before_digest
        .as_deref()
        .unwrap_or("fixture-digest:no-target");
    let disposition = disposition
        .map(|value| value.as_str().to_owned())
        .unwrap_or_else(|| "none".to_owned());
    let lifecycle = lifecycle_state
        .map(|value| value.as_str().to_owned())
        .unwrap_or_else(|| "none".to_owned());
    let mut lines = vec![format!(
        "mutation_fixture_review fixture={} kind={} target={} findings={} disposition={} lifecycle={} payload_digest={} before_digest={} preview_only={} summary={}",
        fixture.id,
        fixture.kind.as_str(),
        fixture.mutated_segment_id.as_deref().unwrap_or("none"),
        finding_summary,
        disposition,
        lifecycle,
        fixture.payload_digest,
        before,
        preview_only,
        fixture.sanitized_payload_summary
    )];
    for candidate in repair_candidates {
        lines.push(format!(
            "mutation_fixture_repair fixture={} plan={} intent={} before={} after={} rollback={} validation={} preview_only={} write_allowed={} applied={}",
            fixture.id,
            candidate.plan_id,
            candidate.intent.as_str(),
            candidate.before_digest,
            candidate.after_digest,
            candidate.rollback_anchor_id,
            candidate.validation_status.as_str(),
            candidate.preview_only,
            candidate.admission_write_authorized,
            candidate.applied
        ));
    }
    lines
}

#[allow(clippy::too_many_arguments)]
fn malignant_drill_evidence_lines(
    fixture: &MalignantGeneRecoveryDrill,
    classification: &str,
    confidence: f32,
    quarantine_plan_present: bool,
    cut_candidate_present: bool,
    regeneration_candidate_present: bool,
    trusted_regeneration_sources: &[String],
    tombstone_id: Option<&str>,
    approval_decision: &str,
    hold_reasons: &[String],
    preview_only: bool,
) -> Vec<String> {
    vec![
        format!(
            "malignant_drill fixture={} kind={} target_present=true classification={} confidence={:.3} rollback={} validation={} payload_digest={} redacted=true preview_only={} quarantine={} cut_candidate={} regenerate={}",
            fixture.id,
            fixture.kind.as_str(),
            classification,
            confidence,
            fixture.stable_anchor_id,
            fixture.replay_validation_status.as_str(),
            fixture.payload_digest,
            preview_only,
            quarantine_plan_present,
            cut_candidate_present,
            regeneration_candidate_present
        ),
        format!(
            "malignant_drill_recovery fixture={} trusted_sources={} tombstone={} approval={} hold_reasons={} protected_neighbors={} summary={}",
            fixture.id,
            trusted_regeneration_sources.join("|"),
            tombstone_id.unwrap_or("none"),
            approval_decision,
            hold_reasons.join("|"),
            fixture.protected_segment_ids.len(),
            fixture.sanitized_payload_summary
        ),
    ]
}

fn hold_reasons_for_drill(
    fixture: &MalignantGeneRecoveryDrill,
    preview: &DnaSplicePreview,
    target_plans: &[&MutationPlan],
) -> Vec<String> {
    let mut reasons = Vec::new();
    for expected in &fixture.expected_hold_reasons {
        push_string_once(&mut reasons, expected);
    }
    if fixture.replay_validation_status == GeneValidationStatus::Pending {
        push_string_once(&mut reasons, "replay_validation_pending");
    }
    if fixture.replay_validation_status == GeneValidationStatus::Failed {
        push_string_once(&mut reasons, "replay_validation_failed");
    }
    if preview.read_only {
        push_string_once(&mut reasons, "preview_only");
    }
    if target_plans
        .iter()
        .any(|plan| !plan.admission_write_authorized)
    {
        push_string_once(&mut reasons, "operator_approval_required");
    }
    reasons
}

fn approval_decision_for_validation(validation_status: GeneValidationStatus) -> String {
    match validation_status {
        GeneValidationStatus::NotRequired => "not_required".to_owned(),
        GeneValidationStatus::Pending => "held_pending_replay_validation".to_owned(),
        GeneValidationStatus::Passed => "approval_held_pending_operator".to_owned(),
        GeneValidationStatus::Failed => "rejected_hold".to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn malignant_drill_failures(
    fixture: &MalignantGeneRecoveryDrill,
    preview: &DnaSplicePreview,
    findings: &[&super::splicing::MutationFinding],
    target_disposition: Option<GeneSegmentDisposition>,
    lifecycle_record: Option<&super::splicing::GeneScissorsLifecycleRecord>,
    quarantine_plan_present: bool,
    cut_candidate_present: bool,
    regeneration_plan: Option<&MutationPlan>,
    trusted_regeneration_sources: &[String],
    copied_bad_payload_source: bool,
    tombstone_id: Option<&str>,
    approval_decision: &str,
    hold_reasons: &[String],
    protected_segments_retained: bool,
    evidence_packet_lines: &[String],
    preview_only: bool,
    redaction_status: &str,
) -> Vec<String> {
    let mut failures = Vec::new();

    if findings.is_empty() {
        failures.push(format!("{}:malignant_finding_missing", fixture.id));
    }
    if !findings
        .iter()
        .any(|finding| finding.kind == GeneVariantKind::Drift)
    {
        failures.push(format!("{}:drift_finding_missing", fixture.id));
    }
    if target_disposition != Some(GeneSegmentDisposition::Quarantined) {
        failures.push(format!(
            "{}:target_not_quarantined:{target_disposition:?}",
            fixture.id
        ));
    }
    if lifecycle_record.map(|record| record.state) != Some(GeneScissorsLifecycleState::Quarantined)
    {
        failures.push(format!("{}:quarantine_lifecycle_missing", fixture.id));
    }
    if lifecycle_record.map(|record| record.rollback_anchor_id.as_str())
        != Some(fixture.stable_anchor_id.as_str())
    {
        failures.push(format!("{}:lifecycle_rollback_anchor_missing", fixture.id));
    }
    if !quarantine_plan_present {
        failures.push(format!("{}:quarantine_plan_missing", fixture.id));
    }
    if !cut_candidate_present {
        failures.push(format!("{}:cut_candidate_missing", fixture.id));
    }
    if regeneration_plan.is_none() {
        failures.push(format!("{}:regeneration_candidate_missing", fixture.id));
    }
    if let Some(plan) = regeneration_plan {
        if plan.rollback_anchor_id != fixture.stable_anchor_id {
            failures.push(format!("{}:regeneration_rollback_mismatch", fixture.id));
        }
        if plan.validation_status != GeneValidationStatus::Pending
            || !plan.is_read_only_preview()
            || plan.admission_write_authorized
            || plan.applied
        {
            failures.push(format!("{}:regeneration_not_preview_only", fixture.id));
        }
        if !plan.has_regeneration_payload() {
            failures.push(format!("{}:regeneration_payload_missing", fixture.id));
        }
    }
    if !trusted_regeneration_sources.contains(&fixture.stable_anchor_id) {
        failures.push(format!("{}:stable_anchor_not_used", fixture.id));
    }
    if copied_bad_payload_source {
        failures.push(format!(
            "{}:bad_payload_used_as_regeneration_source",
            fixture.id
        ));
    }
    if tombstone_id.is_none() {
        failures.push(format!("{}:tombstone_candidate_missing", fixture.id));
    }
    if fixture.replay_validation_status == GeneValidationStatus::Failed
        && approval_decision != "rejected_hold"
    {
        failures.push(format!("{}:failed_validation_not_rejected", fixture.id));
    }
    for reason in &fixture.expected_hold_reasons {
        if !hold_reasons.contains(reason) {
            failures.push(format!("{}:hold_reason_missing:{reason}", fixture.id));
        }
    }
    if !protected_segments_retained {
        failures.push(format!("{}:protected_neighbor_not_retained", fixture.id));
    }
    if redaction_status != "redacted" {
        failures.push(format!("{}:evidence_redaction_failed", fixture.id));
    }
    if evidence_packet_lines
        .iter()
        .any(|line| contains_private_or_executable_marker(line))
    {
        failures.push(format!("{}:evidence_contains_private_marker", fixture.id));
    }
    if evidence_packet_lines
        .iter()
        .any(|line| line.contains("write_allowed=true") || line.contains("applied=true"))
    {
        failures.push(format!("{}:evidence_claims_write_or_apply", fixture.id));
    }
    if !preview_only || !preview.is_read_only_preview() {
        failures.push(format!("{}:preview_only_gate_failed", fixture.id));
    }

    failures
}

fn segment_digest(segment: &GeneSegment) -> String {
    stable_digest([
        segment.id.as_str(),
        segment.source.as_str(),
        segment.source_hash.as_str(),
        &segment.start_token.to_string(),
        &segment.end_token.to_string(),
        segment.label.as_str(),
        segment.purpose.as_str(),
        segment.last_confirmed_purpose.as_str(),
        segment.kv_residency.as_str(),
        &format!("{:.3}", segment.fitness),
        &format!("{:.3}", segment.drift_score),
        &format!("{:.3}", segment.privacy_risk),
        if segment.schema_valid {
            "schema:ok"
        } else {
            "schema:bad"
        },
        if segment.kv_shape_valid {
            "kv_shape:ok"
        } else {
            "kv_shape:bad"
        },
    ])
}

fn plan_digest(plan: &MutationPlan) -> String {
    stable_digest([
        plan.id.as_str(),
        plan.intent.as_str(),
        plan.target_gene_id.as_str(),
        plan.replacement_gene_id
            .as_deref()
            .unwrap_or("replacement:none"),
        plan.proposed_label.as_deref().unwrap_or("label:none"),
        plan.proposed_purpose.as_deref().unwrap_or("purpose:none"),
        plan.reason.as_str(),
        plan.expected_effect.as_str(),
        plan.rollback_anchor_id.as_str(),
        &plan.validation_gates.join("|"),
        plan.validation_status.as_str(),
    ])
}

fn digest_for_fixture(id: &str, payload_summary: &str) -> String {
    stable_digest([id, payload_summary])
}

fn stable_digest<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fixture-digest:{hash:016x}")
}

fn push_kind_once(kinds: &mut Vec<MutationFixtureKind>, kind: MutationFixtureKind) {
    if !kinds.contains(&kind) {
        kinds.push(kind);
    }
}

fn push_drill_kind_once(kinds: &mut Vec<MalignantGeneDrillKind>, kind: MalignantGeneDrillKind) {
    if !kinds.contains(&kind) {
        kinds.push(kind);
    }
}

fn push_string_once(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}
