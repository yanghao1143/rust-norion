use crate::hierarchy::TaskProfile;

use super::model::{GeneScissorsIntent, MutationPlan};

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
pub enum GeneVariantKind {
    Insertion,
    Deletion,
    Truncation,
    StaleLabel,
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
    pub semantic_gist: String,
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
            semantic_gist: String::new(),
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
        self.semantic_gist = semantic_gist.into();
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

    pub fn token_count(&self) -> usize {
        self.end_token.saturating_sub(self.start_token)
    }

    pub fn has_stale_label(&self) -> bool {
        self.label.trim().is_empty() || self.purpose.trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DnaSplicerPolicy {
    pub min_exon_fitness: f32,
    pub max_exon_drift: f32,
    pub max_exon_privacy_risk: f32,
    pub max_segment_tokens: usize,
    pub max_planned_overlap_tokens: usize,
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

#[derive(Debug, Clone, PartialEq)]
pub struct ClassifiedGeneSegment {
    pub segment: GeneSegment,
    pub class: GeneSegmentClass,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DnaSplicePreview {
    pub profile: TaskProfile,
    pub stable_anchor_id: String,
    pub segments: Vec<ClassifiedGeneSegment>,
    pub findings: Vec<MutationFinding>,
    pub mutation_plans: Vec<MutationPlan>,
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
    }

    fn count_class(&self, class: GeneSegmentClass) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.class == class)
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
            if segment.has_stale_label() {
                findings.push(MutationFinding::new(
                    &segment.id,
                    GeneVariantKind::StaleLabel,
                    GeneVariantSeverity::Repair,
                    GeneScissorsIntent::Relabel,
                    "segment label or purpose is stale and cannot explain its function",
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
                        .with_sources([stable_anchor_id.clone(), finding.segment_id.clone()]),
                    );
                }
                GeneScissorsIntent::Relabel => push_plan_once(
                    &mut plans,
                    MutationPlan::preview(
                        format!("mutation:{}:relabel", finding.segment_id),
                        GeneScissorsIntent::Relabel,
                        finding.segment_id.clone(),
                        finding.reason.clone(),
                        "refresh segment label and purpose while preserving the stable anchor",
                        stable_anchor_id.clone(),
                    )
                    .with_sources([finding.segment_id.clone()]),
                ),
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
        let classified_segments = segments
            .into_iter()
            .map(|segment| {
                let segment_findings = findings
                    .iter()
                    .filter(|finding| finding.segment_id == segment.id)
                    .collect::<Vec<_>>();
                let class = classify_segment(&self.policy, &segment, &segment_findings);
                let reasons = segment_findings
                    .iter()
                    .map(|finding| finding.reason.clone())
                    .collect();
                ClassifiedGeneSegment {
                    segment,
                    class,
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

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}
