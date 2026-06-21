use crate::hierarchy::TaskProfile;

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
