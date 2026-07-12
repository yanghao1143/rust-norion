use crate::{hierarchy::TaskProfile, privacy_redaction::stable_redaction_digest};

const AGING_AGE_THRESHOLD: u32 = 8;
const MAX_DECAY_AGE: u32 = 16;
const LOW_FITNESS_THRESHOLD: f32 = 0.45;
const MALIGNANT_DRIFT_THRESHOLD: f32 = 0.70;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningGeneKind {
    Retrieval,
    Routing,
    Reflection,
    Language,
    Safety,
    ToolUse,
    Budget,
}

impl ReasoningGeneKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Retrieval => "retrieval",
            Self::Routing => "routing",
            Self::Reflection => "reflection",
            Self::Language => "language",
            Self::Safety => "safety",
            Self::ToolUse => "tool_use",
            Self::Budget => "budget",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningGeneStatus {
    Active,
    Aging,
    Malignant,
    Quarantined,
    Regenerating,
}

impl ReasoningGeneStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Aging => "aging",
            Self::Malignant => "malignant",
            Self::Quarantined => "quarantined",
            Self::Regenerating => "regenerating",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneScissorsIntent {
    Relabel,
    Cut,
    Splice,
    Quarantine,
    Repair,
    Crossover,
    Rollback,
    Regenerate,
}

impl GeneScissorsIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Relabel => "relabel",
            Self::Cut => "cut",
            Self::Splice => "splice",
            Self::Quarantine => "quarantine",
            Self::Repair => "repair",
            Self::Crossover => "crossover",
            Self::Rollback => "rollback",
            Self::Regenerate => "regenerate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneLifecycleAction {
    Keep,
    Relabel,
    Quarantine,
    Regenerate,
    Rollback,
    Cut,
}

impl GeneLifecycleAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Relabel => "relabel",
            Self::Quarantine => "quarantine",
            Self::Regenerate => "regenerate",
            Self::Rollback => "rollback",
            Self::Cut => "cut",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneValidationStatus {
    NotRequired,
    Pending,
    Passed,
    Failed,
}

impl GeneValidationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotRequired => "not_required",
            Self::Pending => "pending",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneLifecycleSourceKind {
    HealthMetadata,
    FeedbackSignal,
    StableAnchor,
    HighFitnessSibling,
    DriftRollback,
}

impl GeneLifecycleSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HealthMetadata => "health_metadata",
            Self::FeedbackSignal => "feedback_signal",
            Self::StableAnchor => "stable_anchor",
            Self::HighFitnessSibling => "high_fitness_sibling",
            Self::DriftRollback => "drift_rollback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneLifecycleSourceEvidence {
    pub kind: GeneLifecycleSourceKind,
    pub source_id: String,
    pub summary: String,
}

impl GeneLifecycleSourceEvidence {
    pub fn new(
        kind: GeneLifecycleSourceKind,
        source_id: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            source_id: source_id.into(),
            summary: summary.into(),
        }
    }

    pub fn health_metadata(gene: &ReasoningGene) -> Self {
        Self::new(
            GeneLifecycleSourceKind::HealthMetadata,
            gene.id.clone(),
            format!(
                "age={} fitness={:.3} drift={:.3} decay={:.3}",
                gene.age,
                gene.fitness,
                gene.drift_score,
                gene.decay_score()
            ),
        )
    }

    pub fn stable_anchor(anchor_id: impl Into<String>) -> Self {
        let anchor_id = anchor_id.into();
        Self::new(
            GeneLifecycleSourceKind::StableAnchor,
            anchor_id.clone(),
            format!("stable rollback anchor {anchor_id}"),
        )
    }

    pub fn high_fitness_sibling(sibling_id: impl Into<String>) -> Self {
        let sibling_id = sibling_id.into();
        Self::new(
            GeneLifecycleSourceKind::HighFitnessSibling,
            sibling_id.clone(),
            format!("high-fitness sibling source {sibling_id}"),
        )
    }

    pub fn drift_rollback(anchor_id: impl Into<String>) -> Self {
        let anchor_id = anchor_id.into();
        Self::new(
            GeneLifecycleSourceKind::DriftRollback,
            anchor_id.clone(),
            format!("runtime drift rollback evidence anchored at {anchor_id}"),
        )
    }

    pub fn summary(&self) -> String {
        format!("{}:{}:{}", self.kind.as_str(), self.source_id, self.summary)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReasoningGene {
    pub id: String,
    pub kind: ReasoningGeneKind,
    pub label: String,
    pub purpose: String,
    pub tags: Vec<String>,
    pub age: u32,
    pub fitness: f32,
    pub drift_score: f32,
    pub status: ReasoningGeneStatus,
}

impl ReasoningGene {
    pub fn new(
        id: impl Into<String>,
        kind: ReasoningGeneKind,
        label: impl Into<String>,
        purpose: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            label: label.into(),
            purpose: purpose.into(),
            tags: Vec::new(),
            age: 0,
            fitness: 1.0,
            drift_score: 0.0,
            status: ReasoningGeneStatus::Active,
        }
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_health(mut self, age: u32, fitness: f32, drift_score: f32) -> Self {
        self.age = age;
        self.fitness = clamp_unit(fitness);
        self.drift_score = clamp_unit(drift_score);
        self.status = self.derived_status();
        self
    }

    pub fn with_status(mut self, status: ReasoningGeneStatus) -> Self {
        self.status = status;
        self
    }

    pub fn derived_status(&self) -> ReasoningGeneStatus {
        match self.status {
            ReasoningGeneStatus::Quarantined
            | ReasoningGeneStatus::Regenerating
            | ReasoningGeneStatus::Malignant => return self.status,
            ReasoningGeneStatus::Active | ReasoningGeneStatus::Aging => {}
        }

        if self.drift_score >= MALIGNANT_DRIFT_THRESHOLD {
            ReasoningGeneStatus::Malignant
        } else if self.age >= AGING_AGE_THRESHOLD || self.fitness < LOW_FITNESS_THRESHOLD {
            ReasoningGeneStatus::Aging
        } else if self.status == ReasoningGeneStatus::Aging {
            ReasoningGeneStatus::Aging
        } else {
            ReasoningGeneStatus::Active
        }
    }

    pub fn needs_relabel(&self) -> bool {
        self.derived_status() == ReasoningGeneStatus::Aging
            || self.label.trim().is_empty()
            || self.purpose.trim().is_empty()
    }

    pub fn is_malignant(&self) -> bool {
        self.derived_status() == ReasoningGeneStatus::Malignant
    }

    pub fn decay_score(&self) -> f32 {
        let age_pressure = (self.age.min(MAX_DECAY_AGE) as f32 / MAX_DECAY_AGE as f32) * 0.40;
        let fitness_pressure = (1.0 - clamp_unit(self.fitness)) * 0.35;
        let drift_pressure = clamp_unit(self.drift_score) * 0.25;
        clamp_unit(age_pressure + fitness_pressure + drift_pressure)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationPlan {
    pub id: String,
    pub intent: GeneScissorsIntent,
    pub target_gene_id: String,
    pub source_gene_ids: Vec<String>,
    pub replacement_gene_id: Option<String>,
    pub proposed_label: Option<String>,
    pub proposed_purpose: Option<String>,
    pub proposed_tags: Vec<String>,
    pub reason: String,
    pub expected_effect: String,
    pub rollback_anchor_id: String,
    pub validation_gates: Vec<String>,
    pub validation_status: GeneValidationStatus,
    pub source_evidence: Vec<GeneLifecycleSourceEvidence>,
    pub admission_write_authorized: bool,
    pub applied: bool,
}

impl MutationPlan {
    pub fn preview(
        id: impl Into<String>,
        intent: GeneScissorsIntent,
        target_gene_id: impl Into<String>,
        reason: impl Into<String>,
        expected_effect: impl Into<String>,
        rollback_anchor_id: impl Into<String>,
    ) -> Self {
        let target_gene_id = target_gene_id.into();
        let rollback_anchor_id = rollback_anchor_id.into();
        Self {
            id: id.into(),
            intent,
            target_gene_id: target_gene_id.clone(),
            source_gene_ids: Vec::new(),
            replacement_gene_id: None,
            proposed_label: None,
            proposed_purpose: None,
            proposed_tags: Vec::new(),
            reason: reason.into(),
            expected_effect: expected_effect.into(),
            rollback_anchor_id: rollback_anchor_id.clone(),
            validation_gates: default_validation_gates(),
            validation_status: GeneValidationStatus::Pending,
            source_evidence: Vec::new(),
            admission_write_authorized: false,
            applied: false,
        }
        .with_source_evidence([
            GeneLifecycleSourceEvidence::new(
                GeneLifecycleSourceKind::HealthMetadata,
                target_gene_id,
                "mutation candidate health metadata",
            ),
            GeneLifecycleSourceEvidence::stable_anchor(rollback_anchor_id),
        ])
    }

    pub fn with_sources(mut self, sources: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.source_gene_ids = sources.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_replacement(mut self, replacement_gene_id: impl Into<String>) -> Self {
        self.replacement_gene_id = Some(replacement_gene_id.into());
        self
    }

    pub fn with_source_evidence(
        mut self,
        evidence: impl IntoIterator<Item = GeneLifecycleSourceEvidence>,
    ) -> Self {
        for item in evidence {
            if !self
                .source_evidence
                .iter()
                .any(|existing| existing.kind == item.kind && existing.source_id == item.source_id)
            {
                self.source_evidence.push(item);
            }
        }
        self
    }

    pub fn with_validation_status(mut self, validation_status: GeneValidationStatus) -> Self {
        self.validation_status = validation_status;
        self
    }

    pub fn with_repair_payload(
        mut self,
        label: impl Into<String>,
        purpose: impl Into<String>,
        tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.proposed_label = Some(label.into());
        self.proposed_purpose = Some(purpose.into());
        self.proposed_tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn has_repair_payload(&self) -> bool {
        self.proposed_label
            .as_deref()
            .is_some_and(|label| !label.trim().is_empty())
            && self
                .proposed_purpose
                .as_deref()
                .is_some_and(|purpose| !purpose.trim().is_empty())
    }

    pub fn has_regeneration_payload(&self) -> bool {
        self.intent == GeneScissorsIntent::Regenerate
            && self.replacement_gene_id.is_some()
            && self.has_repair_payload()
    }

    pub fn is_read_only_preview(&self) -> bool {
        !self.admission_write_authorized && !self.applied
    }

    pub fn has_source_evidence(&self) -> bool {
        !self.source_evidence.is_empty()
            && self
                .source_evidence
                .iter()
                .all(|evidence| !evidence.source_id.trim().is_empty())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneLifecycleRecord {
    pub id: String,
    pub gene_id: String,
    pub action: GeneLifecycleAction,
    pub status: ReasoningGeneStatus,
    pub age: u32,
    pub last_confirmed_purpose: String,
    pub decay_score: f32,
    pub fitness_score: f32,
    pub drift_score: f32,
    pub validation_status: GeneValidationStatus,
    pub source_evidence: Vec<GeneLifecycleSourceEvidence>,
    pub rollback_anchor_id: String,
    pub stable_anchor_sources: Vec<String>,
    pub replacement_gene_id: Option<String>,
    pub tombstone_id: Option<String>,
    pub reason: String,
    pub next_action: String,
    pub admission_write_authorized: bool,
    pub applied: bool,
}

impl GeneLifecycleRecord {
    fn preview(
        gene: &ReasoningGene,
        action: GeneLifecycleAction,
        stable_anchor_id: &str,
        reason: impl Into<String>,
        next_action: impl Into<String>,
    ) -> Self {
        let validation_status = match action {
            GeneLifecycleAction::Keep => GeneValidationStatus::NotRequired,
            GeneLifecycleAction::Relabel
            | GeneLifecycleAction::Quarantine
            | GeneLifecycleAction::Regenerate
            | GeneLifecycleAction::Rollback
            | GeneLifecycleAction::Cut => GeneValidationStatus::Pending,
        };
        Self {
            id: format!("gene_lifecycle:{}:{}", gene.id, action.as_str()),
            gene_id: gene.id.clone(),
            action,
            status: gene.derived_status(),
            age: gene.age,
            last_confirmed_purpose: gene.purpose.clone(),
            decay_score: gene.decay_score(),
            fitness_score: gene.fitness,
            drift_score: gene.drift_score,
            validation_status,
            source_evidence: vec![
                GeneLifecycleSourceEvidence::health_metadata(gene),
                GeneLifecycleSourceEvidence::stable_anchor(stable_anchor_id),
            ],
            rollback_anchor_id: stable_anchor_id.to_owned(),
            stable_anchor_sources: vec![stable_anchor_id.to_owned()],
            replacement_gene_id: None,
            tombstone_id: None,
            reason: reason.into(),
            next_action: next_action.into(),
            admission_write_authorized: false,
            applied: false,
        }
    }

    fn with_source_evidence(
        mut self,
        evidence: impl IntoIterator<Item = GeneLifecycleSourceEvidence>,
    ) -> Self {
        for item in evidence {
            if !self
                .source_evidence
                .iter()
                .any(|existing| existing.kind == item.kind && existing.source_id == item.source_id)
            {
                self.source_evidence.push(item);
            }
        }
        self
    }

    fn with_replacement(mut self, replacement_gene_id: impl Into<String>) -> Self {
        self.replacement_gene_id = Some(replacement_gene_id.into());
        self
    }

    fn with_tombstone(mut self, tombstone_id: impl Into<String>) -> Self {
        self.tombstone_id = Some(tombstone_id.into());
        self
    }

    pub fn is_read_only_preview(&self) -> bool {
        !self.admission_write_authorized && !self.applied
    }

    pub fn is_tombstone_candidate(&self) -> bool {
        self.action == GeneLifecycleAction::Cut && self.tombstone_id.is_some()
    }

    pub fn has_source_evidence(&self) -> bool {
        !self.source_evidence.is_empty()
            && self
                .source_evidence
                .iter()
                .all(|evidence| !evidence.source_id.trim().is_empty())
    }

    pub fn summary(&self) -> String {
        let replacement = self.replacement_gene_id.as_deref().unwrap_or("none");
        let tombstone = self.tombstone_id.as_deref().unwrap_or("none");
        format!(
            "{}:{} status={} age={} decay={:.3} validation={} replacement={} tombstone={} next={}",
            self.action.as_str(),
            self.gene_id,
            self.status.as_str(),
            self.age,
            self.decay_score,
            self.validation_status.as_str(),
            replacement,
            tombstone,
            self.next_action
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReasoningGenome {
    pub id: String,
    pub profile: TaskProfile,
    pub stable_anchor_id: String,
    pub genes: Vec<ReasoningGene>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningGenomeStrategy {
    English,
    Chinese,
    RustCoding,
    LongContext,
    LocalTool,
}

impl ReasoningGenomeStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::Chinese => "chinese",
            Self::RustCoding => "rust_coding",
            Self::LongContext => "long_context",
            Self::LocalTool => "local_tool",
        }
    }

    pub fn select(profile: TaskProfile, language: &str, local_tool_workflow: bool) -> Self {
        if profile == TaskProfile::Coding {
            Self::RustCoding
        } else if profile == TaskProfile::LongDocument {
            Self::LongContext
        } else if local_tool_workflow {
            Self::LocalTool
        } else if matches!(language, "chinese" | "mixed") {
            Self::Chinese
        } else {
            Self::English
        }
    }
}

impl ReasoningGenome {
    pub fn new(
        id: impl Into<String>,
        profile: TaskProfile,
        stable_anchor_id: impl Into<String>,
        genes: Vec<ReasoningGene>,
    ) -> Self {
        Self {
            id: id.into(),
            profile,
            stable_anchor_id: stable_anchor_id.into(),
            genes,
        }
    }

    pub fn default_for_profile(profile: TaskProfile) -> Self {
        let slug = profile_slug(profile);
        Self::new(
            format!("genome:{slug}:v1"),
            profile,
            format!("genome:{slug}:stable"),
            vec![
                ReasoningGene::new(
                    format!("gene:{slug}:retrieval"),
                    ReasoningGeneKind::Retrieval,
                    "profile retrieval posture",
                    "select useful semantic, gist, and runtime KV memory without raw prompt leakage",
                )
                .with_tags(["memory", "kv", slug]),
                ReasoningGene::new(
                    format!("gene:{slug}:routing"),
                    ReasoningGeneKind::Routing,
                    "adaptive route pressure",
                    "bias attention thresholds using profile, hardware, entropy, and cache signals",
                )
                .with_tags(["router", "threshold", slug]),
                ReasoningGene::new(
                    format!("gene:{slug}:reflection"),
                    ReasoningGeneKind::Reflection,
                    "closed-loop reflection checklist",
                    "surface contradictions, repair actions, and memory admission evidence",
                )
                .with_tags(["reflection", "reward", slug]),
                ReasoningGene::new(
                    format!("gene:{slug}:language"),
                    ReasoningGeneKind::Language,
                    "task language mode",
                    "keep English, Chinese, coding, writing, and long-context behavior profile-scoped",
                )
                .with_tags(["language", slug]),
                ReasoningGene::new(
                    format!("gene:{slug}:safety"),
                    ReasoningGeneKind::Safety,
                    "drift and privacy guard",
                    "block unsafe memory admission, raw prompt leakage, and unreviewed mutation writes",
                )
                .with_tags(["safety", "drift", slug]),
                ReasoningGene::new(
                    format!("gene:{slug}:tool-use"),
                    ReasoningGeneKind::ToolUse,
                    "Rust-only tool posture",
                    "prefer Rust-written local tools with explicit build and validation gates",
                )
                .with_tags(["toolsmith", "rust", slug]),
                ReasoningGene::new(
                    format!("gene:{slug}:budget"),
                    ReasoningGeneKind::Budget,
                    "compute youth pressure",
                    "reduce wasted compute while preserving rollback and regeneration evidence",
                )
                .with_tags(["budget", "youth", slug]),
            ],
        )
    }

    pub fn default_for_strategy(strategy: ReasoningGenomeStrategy, profile: TaskProfile) -> Self {
        let slug = strategy.as_str();
        let genes = match strategy {
            ReasoningGenomeStrategy::English => vec![
                strategy_gene(
                    slug,
                    "language",
                    ReasoningGeneKind::Language,
                    "English response contract",
                    "preserve English terminology, concise structure, and explicit evidence boundaries",
                    ["english", "language", "evidence"],
                ),
                strategy_gene(
                    slug,
                    "reflection",
                    ReasoningGeneKind::Reflection,
                    "English reflection contract",
                    "verify claims and summarize uncertainty before admitting reusable experience",
                    ["english", "reflection", "verification"],
                ),
                strategy_gene(
                    slug,
                    "retrieval",
                    ReasoningGeneKind::Retrieval,
                    "English semantic retrieval",
                    "prefer English semantic and experience memory while keeping recall digest-only",
                    ["english", "semantic", "memory"],
                ),
            ],
            ReasoningGenomeStrategy::Chinese => vec![
                strategy_gene(
                    slug,
                    "language",
                    ReasoningGeneKind::Language,
                    "Chinese response contract",
                    "preserve Chinese instructions and keep technical terms bilingual when ambiguity matters",
                    ["chinese", "language", "bilingual"],
                ),
                strategy_gene(
                    slug,
                    "reflection",
                    ReasoningGeneKind::Reflection,
                    "Chinese reflection contract",
                    "verify conclusions in compact Chinese and separate evidence from missing proof",
                    ["chinese", "reflection", "evidence"],
                ),
                strategy_gene(
                    slug,
                    "retrieval",
                    ReasoningGeneKind::Retrieval,
                    "Chinese gist retrieval",
                    "prefer Chinese gist and semantic memory without importing raw conversation payloads",
                    ["chinese", "gist", "memory"],
                ),
            ],
            ReasoningGenomeStrategy::RustCoding => vec![
                strategy_gene(
                    slug,
                    "tool",
                    ReasoningGeneKind::ToolUse,
                    "Rust compiler tool contract",
                    "prefer cargo fmt, check, test, and compiler evidence before promotion",
                    ["rust", "cargo", "compiler"],
                ),
                strategy_gene(
                    slug,
                    "reflection",
                    ReasoningGeneKind::Reflection,
                    "Rust repair reflection",
                    "convert compiler and test failures into bounded repair evidence and reusable lessons",
                    ["rust", "reflection", "repair"],
                ),
                strategy_gene(
                    slug,
                    "safety",
                    ReasoningGeneKind::Safety,
                    "Rust safety boundary",
                    "hold unsafe, untested, or non-rollbackable code changes",
                    ["rust", "safety", "rollback"],
                ),
            ],
            ReasoningGenomeStrategy::LongContext => vec![
                strategy_gene(
                    slug,
                    "retrieval",
                    ReasoningGeneKind::Retrieval,
                    "long-context tiered retrieval",
                    "combine semantic, gist, and runtime KV tiers under bounded context packing",
                    ["long_context", "gist", "runtime_kv"],
                ),
                strategy_gene(
                    slug,
                    "routing",
                    ReasoningGeneKind::Routing,
                    "long-context recursive routing",
                    "route chunk and merge work while preserving anchors and segment provenance",
                    ["long_context", "recursive", "routing"],
                ),
                strategy_gene(
                    slug,
                    "budget",
                    ReasoningGeneKind::Budget,
                    "long-context compute budget",
                    "reduce redundant tokens through splice windows, summaries, and KV reuse",
                    ["long_context", "budget", "splice"],
                ),
            ],
            ReasoningGenomeStrategy::LocalTool => vec![
                strategy_gene(
                    slug,
                    "tool",
                    ReasoningGeneKind::ToolUse,
                    "local Rust tool contract",
                    "prefer Rust-native local tools with explicit IO, build, and validation gates",
                    ["local_tool", "rust", "toolsmith"],
                ),
                strategy_gene(
                    slug,
                    "safety",
                    ReasoningGeneKind::Safety,
                    "local tool capability boundary",
                    "suppress shell, network, process, and writes until downstream gates approve them",
                    ["local_tool", "capability", "gate"],
                ),
                strategy_gene(
                    slug,
                    "reflection",
                    ReasoningGeneKind::Reflection,
                    "local tool validation reflection",
                    "admit tool experience only after build and focused validation evidence",
                    ["local_tool", "validation", "reflection"],
                ),
            ],
        };
        Self::new(
            format!("genome:strategy:{slug}:v1"),
            profile,
            format!("genome:strategy:{slug}:stable"),
            genes,
        )
    }

    pub fn with_feedback_health(mut self, input: &GenomeExpressionInput) -> Self {
        let quality = clamp_unit(input.quality);
        let process_reward = clamp_unit(input.process_reward);
        let low_quality = quality < 0.55;
        let low_reward = process_reward < 0.45;
        let revision_pressure = input.revision_action_count > 0 || input.contradiction_count > 0;
        let safety_pressure = input.drift_rollback || input.critical_reflection_issue_count > 0;
        let memory_pressure = input.memory_feedback_updates > 0
            && (!input.drift_memory_write_allowed || low_quality || low_reward);
        let runtime_kv_pressure =
            input.runtime_kv_hold && (!input.drift_memory_write_allowed || low_quality);
        let route_pressure = input.route_attention_fraction > 0.72
            && (low_quality || low_reward || revision_pressure);

        for gene in &mut self.genes {
            match gene.kind {
                ReasoningGeneKind::Safety if safety_pressure => {
                    mark_feedback_malignant(gene, quality.min(process_reward), 0.82);
                }
                ReasoningGeneKind::Reflection if low_quality || low_reward || revision_pressure => {
                    mark_feedback_aging(
                        gene,
                        quality.min(process_reward),
                        feedback_drift_score(input, 0.34),
                    );
                }
                ReasoningGeneKind::Retrieval if memory_pressure || runtime_kv_pressure => {
                    mark_feedback_aging(
                        gene,
                        (quality + process_reward) * 0.5,
                        feedback_drift_score(input, 0.28),
                    );
                }
                ReasoningGeneKind::Routing
                    if route_pressure || !input.agent_team_collision_free =>
                {
                    mark_feedback_aging(gene, process_reward, feedback_drift_score(input, 0.30));
                }
                ReasoningGeneKind::ToolUse if !input.toolsmith_gate_passed => {
                    mark_feedback_aging(gene, process_reward, feedback_drift_score(input, 0.32));
                }
                ReasoningGeneKind::Budget
                    if runtime_kv_pressure
                        || !input.agent_team_collision_free
                        || input.route_attention_fraction > 0.86 =>
                {
                    mark_feedback_aging(gene, process_reward, feedback_drift_score(input, 0.26));
                }
                ReasoningGeneKind::Language if input.contradiction_count > 1 && low_quality => {
                    mark_feedback_aging(gene, quality, feedback_drift_score(input, 0.24));
                }
                _ => {}
            }
        }

        self
    }

    pub fn express(&self, input: GenomeExpressionInput) -> GenomeExpression {
        let mut active_gene_ids = Vec::new();
        let mut aged_gene_ids = Vec::new();
        let mut malignant_gene_ids = Vec::new();
        let mut relabel_candidate_ids = Vec::new();
        let mut regeneration_candidate_ids = Vec::new();
        let mut mutation_plans = Vec::new();
        let mut lifecycle_records = Vec::new();

        for gene in &self.genes {
            match gene.derived_status() {
                ReasoningGeneStatus::Active => {
                    active_gene_ids.push(gene.id.clone());
                    lifecycle_records.push(GeneLifecycleRecord::preview(
                        gene,
                        GeneLifecycleAction::Keep,
                        &self.stable_anchor_id,
                        "gene health is within active expression thresholds",
                        "keep_current_expression",
                    ));
                }
                ReasoningGeneStatus::Aging => {
                    active_gene_ids.push(gene.id.clone());
                    aged_gene_ids.push(gene.id.clone());
                    if gene.needs_relabel() {
                        let (label, purpose, tags) = relabel_payload(gene);
                        let relabel_evidence = relabel_source_evidence(gene);
                        relabel_candidate_ids.push(gene.id.clone());
                        mutation_plans.push(MutationPlan::preview(
                            format!("mutation:{}:relabel", gene.id),
                            GeneScissorsIntent::Relabel,
                            gene.id.clone(),
                            "gene label or purpose is aging and needs refreshed function metadata",
                            "refresh label and purpose while preserving the stable gene anchor",
                            self.stable_anchor_id.clone(),
                        )
                        .with_source_evidence(relabel_evidence.clone())
                        .with_repair_payload(label, purpose, tags));
                        lifecycle_records.push(
                            GeneLifecycleRecord::preview(
                                gene,
                                GeneLifecycleAction::Relabel,
                                &self.stable_anchor_id,
                                "aging metadata requires refreshed purpose labels",
                                "validate_relabel_candidate_before_any_write",
                            )
                            .with_source_evidence(relabel_evidence),
                        );
                    }
                }
                ReasoningGeneStatus::Malignant => {
                    malignant_gene_ids.push(gene.id.clone());
                    regeneration_candidate_ids.push(gene.id.clone());
                    let (label, purpose, tags) = regeneration_payload(gene);
                    let regeneration_evidence =
                        regeneration_source_evidence(&self.genes, gene, &self.stable_anchor_id);
                    let regeneration_source_ids =
                        regeneration_source_ids(&self.stable_anchor_id, &regeneration_evidence);
                    mutation_plans.push(
                        MutationPlan::preview(
                            format!("mutation:{}:quarantine", gene.id),
                            GeneScissorsIntent::Quarantine,
                            gene.id.clone(),
                            "gene drift crossed malignant threshold and must be isolated before reuse",
                            "stop expression of the contaminated strategy while preserving audit evidence",
                            self.stable_anchor_id.clone(),
                        )
                        .with_sources([gene.id.clone()])
                        .with_source_evidence(regeneration_evidence.clone()),
                    );
                    mutation_plans.push(
                        MutationPlan::preview(
                            format!("mutation:{}:regenerate", gene.id),
                            GeneScissorsIntent::Regenerate,
                            gene.id.clone(),
                            "replace malignant behavior from stable anchor and high-fitness siblings",
                            "regenerate a young strategy candidate after validation gates pass",
                            self.stable_anchor_id.clone(),
                        )
                        .with_sources(regeneration_source_ids)
                        .with_source_evidence(regeneration_evidence.clone())
                        .with_replacement(regenerated_gene_id(gene))
                        .with_repair_payload(label, purpose, tags),
                    );
                    mutation_plans.push(
                        MutationPlan::preview(
                            format!("mutation:{}:cut", gene.id),
                            GeneScissorsIntent::Cut,
                            gene.id.clone(),
                            "malignant gene requires a tombstone candidate before removal from active expression",
                            "cut only after validation, rollback anchor, and operator approval are present",
                            self.stable_anchor_id.clone(),
                        )
                        .with_sources([gene.id.clone()])
                        .with_source_evidence(regeneration_evidence.clone()),
                    );
                    lifecycle_records.push(
                        GeneLifecycleRecord::preview(
                            gene,
                            GeneLifecycleAction::Quarantine,
                            &self.stable_anchor_id,
                            "malignant drift crossed quarantine threshold",
                            "hold_gene_out_of_active_expression",
                        )
                        .with_source_evidence(regeneration_evidence.clone()),
                    );
                    lifecycle_records.push(
                        GeneLifecycleRecord::preview(
                            gene,
                            GeneLifecycleAction::Regenerate,
                            &self.stable_anchor_id,
                            "regenerate from stable anchor and high-fitness siblings",
                            "validate_regeneration_candidate_before_any_write",
                        )
                        .with_source_evidence(regeneration_evidence.clone())
                        .with_replacement(regenerated_gene_id(gene)),
                    );
                    lifecycle_records.push(
                        GeneLifecycleRecord::preview(
                            gene,
                            GeneLifecycleAction::Cut,
                            &self.stable_anchor_id,
                            "cut malignant active expression through a reversible tombstone preview",
                            "await_operator_approval_before_tombstone_apply",
                        )
                        .with_source_evidence(regeneration_evidence)
                        .with_tombstone(tombstone_id(gene)),
                    );
                }
                ReasoningGeneStatus::Quarantined | ReasoningGeneStatus::Regenerating => {
                    lifecycle_records.push(
                        GeneLifecycleRecord::preview(
                            gene,
                            GeneLifecycleAction::Cut,
                            &self.stable_anchor_id,
                            "gene is already isolated from active expression pending validated regeneration",
                            "keep_tombstone_preview_until_regeneration_passes",
                        )
                        .with_tombstone(tombstone_id(gene)),
                    );
                }
            }
        }

        if input.drift_rollback || input.critical_reflection_issue_count > 0 {
            let target = self
                .genes
                .iter()
                .find(|gene| gene.kind == ReasoningGeneKind::Safety)
                .or_else(|| self.genes.first());
            if let Some(gene) = target {
                let rollback_evidence = vec![GeneLifecycleSourceEvidence::drift_rollback(
                    &self.stable_anchor_id,
                )];
                if !malignant_gene_ids.contains(&gene.id) {
                    malignant_gene_ids.push(gene.id.clone());
                    regeneration_candidate_ids.push(gene.id.clone());
                    let (label, purpose, tags) = regeneration_payload(gene);
                    let regeneration_evidence =
                        regeneration_source_evidence(&self.genes, gene, &self.stable_anchor_id)
                            .into_iter()
                            .chain(rollback_evidence.clone())
                            .collect::<Vec<_>>();
                    let regeneration_source_ids =
                        regeneration_source_ids(&self.stable_anchor_id, &regeneration_evidence);
                    mutation_plans.push(
                        MutationPlan::preview(
                            format!("mutation:{}:quarantine", gene.id),
                            GeneScissorsIntent::Quarantine,
                            gene.id.clone(),
                            "runtime drift marked this safety gene as unsafe to express",
                            "isolate the unstable strategy before rollback or regeneration",
                            self.stable_anchor_id.clone(),
                        )
                        .with_sources([gene.id.clone()])
                        .with_source_evidence(regeneration_evidence.clone()),
                    );
                    mutation_plans.push(
                        MutationPlan::preview(
                            format!("mutation:{}:regenerate", gene.id),
                            GeneScissorsIntent::Regenerate,
                            gene.id.clone(),
                            "runtime drift requires a fresh safety strategy from the stable anchor",
                            "produce a young replacement candidate after validation gates pass",
                            self.stable_anchor_id.clone(),
                        )
                        .with_sources(regeneration_source_ids)
                        .with_source_evidence(regeneration_evidence.clone())
                        .with_replacement(regenerated_gene_id(gene))
                        .with_repair_payload(label, purpose, tags),
                    );
                    mutation_plans.push(
                        MutationPlan::preview(
                            format!("mutation:{}:cut", gene.id),
                            GeneScissorsIntent::Cut,
                            gene.id.clone(),
                            "rollback pressure requires a reversible tombstone candidate",
                            "cut only after validation, rollback anchor, and operator approval are present",
                            self.stable_anchor_id.clone(),
                        )
                        .with_sources([gene.id.clone()])
                        .with_source_evidence(regeneration_evidence.clone()),
                    );
                    lifecycle_records.push(
                        GeneLifecycleRecord::preview(
                            gene,
                            GeneLifecycleAction::Quarantine,
                            &self.stable_anchor_id,
                            "runtime drift rollback isolated this gene",
                            "hold_gene_out_of_active_expression",
                        )
                        .with_source_evidence(regeneration_evidence.clone()),
                    );
                    lifecycle_records.push(
                        GeneLifecycleRecord::preview(
                            gene,
                            GeneLifecycleAction::Regenerate,
                            &self.stable_anchor_id,
                            "runtime drift rollback requires a regenerated candidate",
                            "validate_regeneration_candidate_before_any_write",
                        )
                        .with_source_evidence(regeneration_evidence.clone())
                        .with_replacement(regenerated_gene_id(gene)),
                    );
                    lifecycle_records.push(
                        GeneLifecycleRecord::preview(
                            gene,
                            GeneLifecycleAction::Cut,
                            &self.stable_anchor_id,
                            "runtime drift rollback produced a reversible tombstone preview",
                            "await_operator_approval_before_tombstone_apply",
                        )
                        .with_source_evidence(regeneration_evidence)
                        .with_tombstone(tombstone_id(gene)),
                    );
                }
                mutation_plans.push(MutationPlan::preview(
                    format!("mutation:{}:rollback", gene.id),
                    GeneScissorsIntent::Rollback,
                    gene.id.clone(),
                    "runtime drift or critical reflection issue requires stable genome rollback",
                    "restore the previous stable genome before any durable mutation is admitted",
                    self.stable_anchor_id.clone(),
                )
                .with_source_evidence(rollback_evidence.clone()));
                lifecycle_records.push(
                    GeneLifecycleRecord::preview(
                        gene,
                        GeneLifecycleAction::Rollback,
                        &self.stable_anchor_id,
                        "runtime drift or critical reflection issue requires rollback evidence",
                        "replay_stable_anchor_before_any_admission",
                    )
                    .with_source_evidence(rollback_evidence),
                );
            }
        }

        if !input.genome_mutation_allowed {
            relabel_candidate_ids.clear();
            regeneration_candidate_ids.clear();
            mutation_plans.clear();
        }

        let youth_pressure = compute_youth_pressure(&input, &aged_gene_ids, &malignant_gene_ids);

        GenomeExpression {
            genome_id: self.id.clone(),
            profile: self.profile,
            stable_anchor_id: self.stable_anchor_id.clone(),
            expression_gene_count: self.genes.len(),
            active_gene_ids,
            aged_gene_ids,
            malignant_gene_ids,
            relabel_candidate_ids,
            regeneration_candidate_ids,
            mutation_plans,
            lifecycle_records,
            read_only: true,
            write_allowed: false,
            applied: false,
            youth_pressure,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenomeExpressionInput {
    pub profile: TaskProfile,
    pub quality: f32,
    pub process_reward: f32,
    pub contradiction_count: usize,
    pub critical_reflection_issue_count: usize,
    pub revision_action_count: usize,
    pub used_memories: usize,
    pub memory_feedback_updates: usize,
    pub route_attention_fraction: f32,
    pub agent_team_collision_free: bool,
    pub toolsmith_gate_passed: bool,
    pub drift_memory_write_allowed: bool,
    pub genome_mutation_allowed: bool,
    pub drift_rollback: bool,
    pub runtime_kv_hold: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenomeExpression {
    pub genome_id: String,
    pub profile: TaskProfile,
    pub stable_anchor_id: String,
    pub expression_gene_count: usize,
    pub active_gene_ids: Vec<String>,
    pub aged_gene_ids: Vec<String>,
    pub malignant_gene_ids: Vec<String>,
    pub relabel_candidate_ids: Vec<String>,
    pub regeneration_candidate_ids: Vec<String>,
    pub mutation_plans: Vec<MutationPlan>,
    pub lifecycle_records: Vec<GeneLifecycleRecord>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub youth_pressure: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpigeneticExpressionCacheMarker {
    pub marker_id: String,
    pub cache_candidate_digest: String,
    pub cache_key_digest: String,
    pub observation_window: usize,
    pub min_success_rate_milli: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenomeOpcode {
    BindStimulus,
    LoadGene,
    MatchEnv,
    ExpressTrait,
    SetBudget,
    SelectMemory,
    PackContext,
    FocusSignal,
    MaskSignal,
    DeclareActionVocab,
    SuppressCapability,
    RequireEvidence,
    DeclareGate,
    PreviewMutation,
    EmitFrame,
}

impl GenomeOpcode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BindStimulus => "bind_stimulus",
            Self::LoadGene => "load_gene",
            Self::MatchEnv => "match_env",
            Self::ExpressTrait => "express_trait",
            Self::SetBudget => "set_budget",
            Self::SelectMemory => "select_memory",
            Self::PackContext => "pack_context",
            Self::FocusSignal => "focus_signal",
            Self::MaskSignal => "mask_signal",
            Self::DeclareActionVocab => "declare_action_vocab",
            Self::SuppressCapability => "suppress_capability",
            Self::RequireEvidence => "require_evidence",
            Self::DeclareGate => "declare_gate",
            Self::PreviewMutation => "preview_mutation",
            Self::EmitFrame => "emit_frame",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::BindStimulus,
            Self::LoadGene,
            Self::MatchEnv,
            Self::ExpressTrait,
            Self::SetBudget,
            Self::SelectMemory,
            Self::PackContext,
            Self::FocusSignal,
            Self::MaskSignal,
            Self::DeclareActionVocab,
            Self::SuppressCapability,
            Self::RequireEvidence,
            Self::DeclareGate,
            Self::PreviewMutation,
            Self::EmitFrame,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenomeExpressionVmSideEffect {
    ReadOnly,
}

impl GenomeExpressionVmSideEffect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreReasoningGenomeIsa {
    pub name: &'static str,
    pub opcodes: Vec<GenomeOpcode>,
    pub expression_vm_side_effect: GenomeExpressionVmSideEffect,
    pub apply_allowed: bool,
}

impl PreReasoningGenomeIsa {
    pub fn preview() -> Self {
        Self {
            name: "PreReasoningGenomeIsa",
            opcodes: GenomeOpcode::all(),
            expression_vm_side_effect: GenomeExpressionVmSideEffect::ReadOnly,
            apply_allowed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFrameObservation {
    RepoIssueTerminalRuntimeState,
    TaskConstraints,
    MemoryState,
    RuntimeHealth,
}

impl ReasoningFrameObservation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RepoIssueTerminalRuntimeState => "repo_issue_terminal_runtime_state",
            Self::TaskConstraints => "task_constraints",
            Self::MemoryState => "memory_state",
            Self::RuntimeHealth => "runtime_health",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFrameAction {
    Observe,
    Inspect,
    Compare,
    Summarize,
    Propose,
    Simulate,
    Gate,
    Verify,
    Quarantine,
    Rollback,
}

impl ReasoningFrameAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Inspect => "inspect",
            Self::Compare => "compare",
            Self::Summarize => "summarize",
            Self::Propose => "propose",
            Self::Simulate => "simulate",
            Self::Gate => "gate",
            Self::Verify => "verify",
            Self::Quarantine => "quarantine",
            Self::Rollback => "rollback",
        }
    }

    pub fn bounded_vocab() -> Vec<Self> {
        vec![
            Self::Observe,
            Self::Inspect,
            Self::Compare,
            Self::Summarize,
            Self::Propose,
            Self::Simulate,
            Self::Gate,
            Self::Verify,
            Self::Quarantine,
            Self::Rollback,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFrameSignalKind {
    UserConstraint,
    GenomeState,
    TaskState,
    MemoryState,
    RuntimeHealth,
    RawPayload,
    UntrustedExternalPayload,
}

impl ReasoningFrameSignalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserConstraint => "user_constraint",
            Self::GenomeState => "genome_state",
            Self::TaskState => "task_state",
            Self::MemoryState => "memory_state",
            Self::RuntimeHealth => "runtime_health",
            Self::RawPayload => "raw_payload",
            Self::UntrustedExternalPayload => "untrusted_external_payload",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningFrameEnvironmentSignal {
    pub kind: ReasoningFrameSignalKind,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningFrameEnvironmentMatch {
    pub profile: TaskProfile,
    pub matched_gene_count: usize,
    pub matched_signal_count: usize,
    pub task_gene_compatible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningFrameRoutingBias {
    pub profile: TaskProfile,
    pub compute_budget: String,
    pub threshold_milli: u16,
    pub max_fanout: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFrameMemoryTier {
    Semantic,
    Gist,
    RuntimeKv,
    Experience,
    ToolReliability,
}

impl ReasoningFrameMemoryTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Semantic => "semantic",
            Self::Gist => "gist",
            Self::RuntimeKv => "runtime_kv",
            Self::Experience => "experience",
            Self::ToolReliability => "tool_reliability",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningFrameMemoryPolicy {
    pub scope: String,
    pub tiers: Vec<ReasoningFrameMemoryTier>,
    pub max_records: usize,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningFrameBudget {
    pub max_tokens: usize,
    pub route_fanout: usize,
    pub reflection_passes: usize,
    pub validation_runs: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningFrameContextPolicy {
    pub selected_gene_count: usize,
    pub selected_memory_count: usize,
    pub focus_signals: Vec<ReasoningFrameSignalKind>,
    pub masked_signals: Vec<ReasoningFrameSignalKind>,
    pub digest_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFrameGate {
    ToolAction,
    Network,
    Writer,
    MemoryAdmission,
    GenomeWriter,
    Process,
    Repository,
    Rollback,
}

impl ReasoningFrameGate {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToolAction => "tool_action_gate",
            Self::Network => "network_gate",
            Self::Writer => "writer_gate",
            Self::MemoryAdmission => "memory_admission_gate",
            Self::GenomeWriter => "genome_writer_gate",
            Self::Process => "process_gate",
            Self::Repository => "repository_gate",
            Self::Rollback => "rollback_gate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningFrameMutationPreview {
    pub plan_id_digest: String,
    pub intent: GeneScissorsIntent,
    pub target_gene_digest: String,
    pub rollback_anchor_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFrameCapability {
    Write,
    Shell,
    Browser,
    Network,
    Process,
    FileWrite,
    MemoryWrite,
    GenomeWrite,
    IssuePrWrite,
    RuntimeWrite,
}

impl ReasoningFrameCapability {
    pub fn forbidden_preview_capabilities() -> &'static [Self] {
        &[
            Self::Write,
            Self::Shell,
            Self::Browser,
            Self::Network,
            Self::Process,
            Self::FileWrite,
            Self::MemoryWrite,
            Self::GenomeWrite,
            Self::IssuePrWrite,
            Self::RuntimeWrite,
        ]
    }

    pub fn is_forbidden_preview(self) -> bool {
        Self::forbidden_preview_capabilities().contains(&self)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Shell => "shell",
            Self::Browser => "browser",
            Self::Network => "network",
            Self::Process => "process",
            Self::FileWrite => "file_write",
            Self::MemoryWrite => "memory_write",
            Self::GenomeWrite => "genome_write",
            Self::IssuePrWrite => "issue_pr_write",
            Self::RuntimeWrite => "runtime_write",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFrameRiskLimit {
    PreviewOnly,
    DigestOnly,
}

impl ReasoningFrameRiskLimit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreviewOnly => "preview_only",
            Self::DigestOnly => "digest_only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFrameEvidenceRequirement {
    DigestOnlyFrameId,
    NoRawPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFrameValidationRequirement {
    PreviewOnly,
    NoWrite,
    NoApply,
    SuppressForbiddenCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningFrameEfficiencySnapshot {
    pub selected_gene_count: usize,
    pub rejected_intron_count: usize,
    pub splice_window_count: usize,
    pub routing_budget_decisions: usize,
    pub compute_budget_class: &'static str,
    pub input_tokens: usize,
    pub retained_tokens: usize,
    pub saved_tokens: usize,
    pub validation_cost_tokens: usize,
    pub quality_milli: u16,
    pub process_reward_milli: u16,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl ReasoningFrameEfficiencySnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn preview(
        selected_gene_count: usize,
        rejected_intron_count: usize,
        splice_window_count: usize,
        routing_budget_decisions: usize,
        compute_budget_class: &'static str,
        input_tokens: usize,
        retained_tokens: usize,
        saved_tokens: usize,
        validation_cost_tokens: usize,
        quality: f32,
        process_reward: f32,
    ) -> Self {
        Self {
            selected_gene_count,
            rejected_intron_count,
            splice_window_count,
            routing_budget_decisions,
            compute_budget_class,
            input_tokens,
            retained_tokens,
            saved_tokens,
            validation_cost_tokens,
            quality_milli: bounded_unit_milli(quality),
            process_reward_milli: bounded_unit_milli(process_reward),
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn has_feedback_signal(&self) -> bool {
        self.selected_gene_count > 0
            && (self.saved_tokens > 0 || self.validation_cost_tokens > 0)
            && (self.quality_milli > 0 || self.process_reward_milli > 0)
    }

    fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningFrame {
    pub frame_id: String,
    pub genome_isa: PreReasoningGenomeIsa,
    pub environment_signals_present: bool,
    pub environment_signals: Vec<ReasoningFrameEnvironmentSignal>,
    pub environment_matches: Vec<ReasoningFrameEnvironmentMatch>,
    pub allowed_observations: Vec<ReasoningFrameObservation>,
    pub selected_gene_ids: Vec<String>,
    pub executed_opcodes: Vec<GenomeOpcode>,
    pub action_vocab: Vec<ReasoningFrameAction>,
    pub suppressed_capabilities: Vec<ReasoningFrameCapability>,
    pub granted_capabilities: Vec<ReasoningFrameCapability>,
    pub routing_bias: ReasoningFrameRoutingBias,
    pub memory_policy: ReasoningFrameMemoryPolicy,
    pub budget: ReasoningFrameBudget,
    pub context_policy: ReasoningFrameContextPolicy,
    pub gates: Vec<ReasoningFrameGate>,
    pub mutation_preview: Vec<ReasoningFrameMutationPreview>,
    pub risk_limits: Vec<ReasoningFrameRiskLimit>,
    pub evidence_requirements: Vec<ReasoningFrameEvidenceRequirement>,
    pub validation_requirements: Vec<ReasoningFrameValidationRequirement>,
    pub efficiency_snapshot: Option<ReasoningFrameEfficiencySnapshot>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningFrameValidationError {
    FrameIdNotDigestOnly,
    MissingEnvironmentSignals,
    MissingObservationBoundary,
    MissingOpcode,
    OpcodeNotExecuted(GenomeOpcode),
    EnvironmentSignalNotDigestOnly,
    MissingEnvironmentMatch,
    MissingActionVocab,
    MemoryPolicyNotReadOnly,
    MissingGate,
    MissingRiskLimit(ReasoningFrameRiskLimit),
    MissingEvidenceRequirement(ReasoningFrameEvidenceRequirement),
    MissingValidationRequirement(ReasoningFrameValidationRequirement),
    MissingSuppressedCapability(ReasoningFrameCapability),
    ForbiddenCapabilityGranted(ReasoningFrameCapability),
    ExpressionVmNotReadOnly,
    GenomeIsaApplyAllowed,
    EfficiencySnapshotNotPreviewOnly,
    NotReadOnly,
    WriteAllowed,
    Applied,
}

impl ReasoningFrame {
    pub fn with_efficiency_snapshot(mut self, snapshot: ReasoningFrameEfficiencySnapshot) -> Self {
        self.efficiency_snapshot = Some(snapshot);
        self
    }

    pub fn validate_preview(&self) -> Result<(), ReasoningFrameValidationError> {
        if !self.frame_id.starts_with("redaction-digest:") {
            return Err(ReasoningFrameValidationError::FrameIdNotDigestOnly);
        }
        if !self.environment_signals_present {
            return Err(ReasoningFrameValidationError::MissingEnvironmentSignals);
        }
        if self.environment_signals.is_empty()
            || self
                .environment_signals
                .iter()
                .any(|signal| !signal.digest.starts_with("redaction-digest:"))
        {
            return Err(ReasoningFrameValidationError::EnvironmentSignalNotDigestOnly);
        }
        if self.environment_matches.is_empty() {
            return Err(ReasoningFrameValidationError::MissingEnvironmentMatch);
        }
        if !self
            .allowed_observations
            .contains(&ReasoningFrameObservation::RepoIssueTerminalRuntimeState)
        {
            return Err(ReasoningFrameValidationError::MissingObservationBoundary);
        }
        if self.genome_isa.opcodes.is_empty() {
            return Err(ReasoningFrameValidationError::MissingOpcode);
        }
        for opcode in &self.genome_isa.opcodes {
            if !self.executed_opcodes.contains(opcode) {
                return Err(ReasoningFrameValidationError::OpcodeNotExecuted(*opcode));
            }
        }
        if self.action_vocab.is_empty() {
            return Err(ReasoningFrameValidationError::MissingActionVocab);
        }
        if !self.memory_policy.read_only {
            return Err(ReasoningFrameValidationError::MemoryPolicyNotReadOnly);
        }
        if self.gates.is_empty() {
            return Err(ReasoningFrameValidationError::MissingGate);
        }
        if self.genome_isa.expression_vm_side_effect != GenomeExpressionVmSideEffect::ReadOnly {
            return Err(ReasoningFrameValidationError::ExpressionVmNotReadOnly);
        }
        if self.genome_isa.apply_allowed {
            return Err(ReasoningFrameValidationError::GenomeIsaApplyAllowed);
        }
        if !self.read_only {
            return Err(ReasoningFrameValidationError::NotReadOnly);
        }
        if self.write_allowed {
            return Err(ReasoningFrameValidationError::WriteAllowed);
        }
        if self.applied {
            return Err(ReasoningFrameValidationError::Applied);
        }
        if self
            .efficiency_snapshot
            .as_ref()
            .is_some_and(|snapshot| !snapshot.is_preview_only())
        {
            return Err(ReasoningFrameValidationError::EfficiencySnapshotNotPreviewOnly);
        }
        for capability in &self.granted_capabilities {
            if capability.is_forbidden_preview() {
                return Err(ReasoningFrameValidationError::ForbiddenCapabilityGranted(
                    *capability,
                ));
            }
        }
        for capability in ReasoningFrameCapability::forbidden_preview_capabilities() {
            if !self.suppressed_capabilities.contains(capability) {
                return Err(ReasoningFrameValidationError::MissingSuppressedCapability(
                    *capability,
                ));
            }
        }
        for limit in [
            ReasoningFrameRiskLimit::PreviewOnly,
            ReasoningFrameRiskLimit::DigestOnly,
        ] {
            if !self.risk_limits.contains(&limit) {
                return Err(ReasoningFrameValidationError::MissingRiskLimit(limit));
            }
        }
        for requirement in [
            ReasoningFrameEvidenceRequirement::DigestOnlyFrameId,
            ReasoningFrameEvidenceRequirement::NoRawPayload,
        ] {
            if !self.evidence_requirements.contains(&requirement) {
                return Err(ReasoningFrameValidationError::MissingEvidenceRequirement(
                    requirement,
                ));
            }
        }
        for requirement in [
            ReasoningFrameValidationRequirement::PreviewOnly,
            ReasoningFrameValidationRequirement::NoWrite,
            ReasoningFrameValidationRequirement::NoApply,
            ReasoningFrameValidationRequirement::SuppressForbiddenCapabilities,
        ] {
            if !self.validation_requirements.contains(&requirement) {
                return Err(ReasoningFrameValidationError::MissingValidationRequirement(
                    requirement,
                ));
            }
        }
        Ok(())
    }

    pub fn issue375_evidence_fields(&self) -> String {
        format!(
            "issue375_pre_reasoning_genome_isa_present=true issue375_reasoning_frame_id={} issue375_reasoning_frame_environment_signals_present={} issue375_reasoning_frame_allowed_observations={} issue375_reasoning_frame_action_vocab={} issue375_reasoning_frame_executed_opcodes={} issue375_reasoning_frame_routing_bias={} issue375_reasoning_frame_memory_policy={} issue375_reasoning_frame_mutation_previews={} issue375_reasoning_frame_suppressed_capabilities={} issue375_reasoning_frame_risk_limits={} issue375_expression_vm_side_effect={} issue375_genome_isa_apply_allowed={}",
            self.frame_id,
            self.environment_signals_present,
            self.allowed_observations_evidence_value(),
            self.action_vocab_evidence_value(),
            self.executed_opcodes_evidence_value(),
            self.routing_bias_evidence_value(),
            self.memory_policy_evidence_value(),
            self.mutation_preview.len(),
            self.suppressed_capabilities_evidence_value(),
            self.risk_limits_evidence_value(),
            self.genome_isa.expression_vm_side_effect.as_str(),
            self.genome_isa.apply_allowed,
        )
    }

    fn allowed_observations_evidence_value(&self) -> String {
        self.allowed_observations
            .iter()
            .map(|observation| observation.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }

    fn action_vocab_evidence_value(&self) -> String {
        self.action_vocab
            .iter()
            .map(|action| action.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }

    pub fn executed_opcodes_evidence_value(&self) -> String {
        self.executed_opcodes
            .iter()
            .map(|opcode| opcode.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }

    pub fn routing_bias_evidence_value(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            profile_slug(self.routing_bias.profile),
            self.routing_bias.compute_budget,
            self.routing_bias.threshold_milli,
            self.routing_bias.max_fanout
        )
    }

    pub fn memory_policy_evidence_value(&self) -> String {
        format!(
            "{}:{}:{}",
            self.memory_policy.scope,
            self.memory_policy
                .tiers
                .iter()
                .map(|tier| tier.as_str())
                .collect::<Vec<_>>()
                .join("-"),
            self.memory_policy.max_records
        )
    }

    fn suppressed_capabilities_evidence_value(&self) -> String {
        self.suppressed_capabilities
            .iter()
            .map(|capability| capability.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }

    fn risk_limits_evidence_value(&self) -> String {
        self.risk_limits
            .iter()
            .map(|limit| limit.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }
}

fn bounded_unit_milli(value: f32) -> u16 {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as u16
    } else {
        0
    }
}

impl GenomeExpression {
    pub fn empty(profile: TaskProfile) -> Self {
        ReasoningGenome::default_for_profile(profile).express(GenomeExpressionInput {
            profile,
            quality: 1.0,
            process_reward: 1.0,
            contradiction_count: 0,
            critical_reflection_issue_count: 0,
            revision_action_count: 0,
            used_memories: 0,
            memory_feedback_updates: 0,
            route_attention_fraction: 0.0,
            agent_team_collision_free: true,
            toolsmith_gate_passed: true,
            drift_memory_write_allowed: true,
            genome_mutation_allowed: true,
            drift_rollback: false,
            runtime_kv_hold: false,
        })
    }

    pub fn compose_read_only(&self, strategy: &Self) -> Self {
        let mut composed = self.clone();
        composed.genome_id = format!("{}+{}", self.genome_id, strategy.genome_id);
        composed.expression_gene_count = self
            .expression_gene_count
            .saturating_add(strategy.expression_gene_count);
        extend_unique_strings(&mut composed.active_gene_ids, &strategy.active_gene_ids);
        extend_unique_strings(&mut composed.aged_gene_ids, &strategy.aged_gene_ids);
        extend_unique_strings(
            &mut composed.malignant_gene_ids,
            &strategy.malignant_gene_ids,
        );
        extend_unique_strings(
            &mut composed.relabel_candidate_ids,
            &strategy.relabel_candidate_ids,
        );
        extend_unique_strings(
            &mut composed.regeneration_candidate_ids,
            &strategy.regeneration_candidate_ids,
        );
        for plan in &strategy.mutation_plans {
            if !composed
                .mutation_plans
                .iter()
                .any(|existing| existing.id == plan.id)
            {
                composed.mutation_plans.push(plan.clone());
            }
        }
        for record in &strategy.lifecycle_records {
            if !composed
                .lifecycle_records
                .iter()
                .any(|existing| existing.id == record.id)
            {
                composed.lifecycle_records.push(record.clone());
            }
        }
        composed.read_only = self.read_only && strategy.read_only;
        composed.write_allowed = false;
        composed.applied = false;
        composed.youth_pressure = self.youth_pressure.max(strategy.youth_pressure);
        composed
    }

    pub fn active_gene_count(&self) -> usize {
        self.active_gene_ids.len()
    }

    pub fn aged_gene_count(&self) -> usize {
        self.aged_gene_ids.len()
    }

    pub fn malignant_gene_count(&self) -> usize {
        self.malignant_gene_ids.len()
    }

    pub fn relabel_candidate_count(&self) -> usize {
        self.relabel_candidate_ids.len()
    }

    pub fn regeneration_candidate_count(&self) -> usize {
        self.regeneration_candidate_ids.len()
    }

    pub fn scissors_proposal_count(&self) -> usize {
        self.mutation_plans.len()
    }

    pub fn lifecycle_record_count(&self) -> usize {
        self.lifecycle_records.len()
    }

    pub fn lifecycle_source_evidence_count(&self) -> usize {
        self.lifecycle_records
            .iter()
            .map(|record| record.source_evidence.len())
            .sum()
    }

    pub fn pending_lifecycle_validation_count(&self) -> usize {
        self.lifecycle_records
            .iter()
            .filter(|record| record.validation_status == GeneValidationStatus::Pending)
            .count()
    }

    pub fn tombstone_candidate_count(&self) -> usize {
        self.lifecycle_records
            .iter()
            .filter(|record| record.is_tombstone_candidate())
            .count()
    }

    pub fn repair_payload_count(&self) -> usize {
        self.mutation_plans
            .iter()
            .filter(|plan| plan.has_repair_payload())
            .count()
    }

    pub fn regeneration_payload_count(&self) -> usize {
        self.mutation_plans
            .iter()
            .filter(|plan| plan.has_regeneration_payload())
            .count()
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

    pub fn proposal_ids(&self) -> Vec<String> {
        self.mutation_plans
            .iter()
            .map(|plan| plan.id.clone())
            .collect()
    }

    pub fn lifecycle_action_summaries(&self) -> Vec<String> {
        let mut actions = Vec::new();
        for record in &self.lifecycle_records {
            let action = record.action.as_str().to_owned();
            if !actions.contains(&action) {
                actions.push(action);
            }
        }
        actions
    }

    pub fn lifecycle_summaries(&self, limit: usize) -> Vec<String> {
        self.lifecycle_records
            .iter()
            .take(limit)
            .map(GeneLifecycleRecord::summary)
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
                .mutation_plans
                .iter()
                .all(MutationPlan::has_source_evidence)
            && self
                .lifecycle_records
                .iter()
                .all(GeneLifecycleRecord::is_read_only_preview)
            && self
                .lifecycle_records
                .iter()
                .all(GeneLifecycleRecord::has_source_evidence)
    }

    pub fn epigenetic_expression_cache_marker(&self) -> Option<EpigeneticExpressionCacheMarker> {
        let stable = self.is_read_only_preview()
            && self.active_gene_count() == self.expression_gene_count
            && self.aged_gene_count() == 0
            && self.malignant_gene_count() == 0
            && self.scissors_proposal_count() == 0
            && self.lifecycle_record_count() >= self.expression_gene_count
            && self.youth_pressure <= 0.03;
        if !stable {
            return None;
        }
        let marker_id = stable_redaction_digest([
            "issue-496",
            "epigenetic-expression-marker",
            self.genome_id.as_str(),
            self.stable_anchor_id.as_str(),
        ]);
        let cache_candidate_digest = stable_redaction_digest([
            "issue-496",
            "mrna-cache-candidate",
            marker_id.as_str(),
            profile_slug(self.profile),
        ]);
        let expression_gene_count = self.expression_gene_count.to_string();
        let cache_key_digest = stable_redaction_digest([
            "issue-496",
            "expression-cache-key",
            marker_id.as_str(),
            expression_gene_count.as_str(),
        ]);
        Some(EpigeneticExpressionCacheMarker {
            marker_id,
            cache_candidate_digest,
            cache_key_digest,
            observation_window: 100,
            min_success_rate_milli: 980,
        })
    }
}

fn extend_unique_strings(target: &mut Vec<String>, source: &[String]) {
    for value in source {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}

pub(crate) fn profile_slug(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn relabel_source_evidence(gene: &ReasoningGene) -> Vec<GeneLifecycleSourceEvidence> {
    vec![
        GeneLifecycleSourceEvidence::health_metadata(gene),
        GeneLifecycleSourceEvidence::new(
            GeneLifecycleSourceKind::FeedbackSignal,
            gene.id.clone(),
            "age, decay, or stale purpose metadata triggered relabel preview",
        ),
    ]
}

fn regeneration_source_evidence(
    genes: &[ReasoningGene],
    target: &ReasoningGene,
    stable_anchor_id: &str,
) -> Vec<GeneLifecycleSourceEvidence> {
    let mut evidence = vec![
        GeneLifecycleSourceEvidence::health_metadata(target),
        GeneLifecycleSourceEvidence::stable_anchor(stable_anchor_id),
    ];
    for sibling in genes
        .iter()
        .filter(|gene| {
            gene.id != target.id
                && gene.fitness >= 0.75
                && gene.drift_score <= 0.20
                && !gene.is_malignant()
        })
        .take(3)
    {
        evidence.push(GeneLifecycleSourceEvidence::high_fitness_sibling(
            sibling.id.clone(),
        ));
    }
    evidence
}

fn regeneration_source_ids(
    stable_anchor_id: &str,
    evidence: &[GeneLifecycleSourceEvidence],
) -> Vec<String> {
    let mut source_ids = vec![stable_anchor_id.to_owned()];
    for item in evidence {
        if item.kind == GeneLifecycleSourceKind::HighFitnessSibling
            && !source_ids.contains(&item.source_id)
        {
            source_ids.push(item.source_id.clone());
        }
    }
    source_ids
}

fn compute_youth_pressure(
    input: &GenomeExpressionInput,
    aged_gene_ids: &[String],
    malignant_gene_ids: &[String],
) -> f32 {
    let mut pressure = 0.0_f32;
    pressure += (1.0 - clamp_unit(input.quality)) * 0.25;
    pressure += (1.0 - clamp_unit(input.process_reward)) * 0.20;
    pressure += input.contradiction_count.min(4) as f32 * 0.08;
    pressure += input.critical_reflection_issue_count.min(3) as f32 * 0.14;
    pressure += input.revision_action_count.min(4) as f32 * 0.04;
    pressure += aged_gene_ids.len().min(4) as f32 * 0.06;
    pressure += malignant_gene_ids.len().min(3) as f32 * 0.16;
    if !input.agent_team_collision_free {
        pressure += 0.08;
    }
    if !input.toolsmith_gate_passed {
        pressure += 0.06;
    }
    if !input.drift_memory_write_allowed {
        pressure += 0.08;
    }
    if input.drift_rollback {
        pressure += 0.18;
    }
    if input.runtime_kv_hold {
        pressure += 0.04;
    }
    clamp_unit(pressure)
}

fn mark_feedback_aging(gene: &mut ReasoningGene, fitness: f32, drift_score: f32) {
    if matches!(
        gene.status,
        ReasoningGeneStatus::Quarantined
            | ReasoningGeneStatus::Regenerating
            | ReasoningGeneStatus::Malignant
    ) {
        return;
    }

    gene.age = gene.age.max(AGING_AGE_THRESHOLD);
    gene.fitness = gene.fitness.min(clamp_unit(fitness));
    gene.drift_score = gene
        .drift_score
        .max(clamp_unit(drift_score).min(MALIGNANT_DRIFT_THRESHOLD - 0.01));
    gene.status = gene.derived_status();
}

fn mark_feedback_malignant(gene: &mut ReasoningGene, fitness: f32, drift_score: f32) {
    if matches!(
        gene.status,
        ReasoningGeneStatus::Quarantined | ReasoningGeneStatus::Regenerating
    ) {
        return;
    }

    gene.age = gene.age.max(AGING_AGE_THRESHOLD);
    gene.fitness = gene.fitness.min(clamp_unit(fitness));
    gene.drift_score = gene
        .drift_score
        .max(clamp_unit(drift_score).max(MALIGNANT_DRIFT_THRESHOLD));
    gene.status = ReasoningGeneStatus::Malignant;
}

fn feedback_drift_score(input: &GenomeExpressionInput, base: f32) -> f32 {
    let contradiction_pressure = input.contradiction_count.min(3) as f32 * 0.06;
    let critical_pressure = input.critical_reflection_issue_count.min(2) as f32 * 0.08;
    let revision_pressure = input.revision_action_count.min(4) as f32 * 0.03;
    let gate_pressure = if input.toolsmith_gate_passed {
        0.0
    } else {
        0.06
    };
    let coordination_pressure = if input.agent_team_collision_free {
        0.0
    } else {
        0.06
    };
    let memory_pressure = if input.drift_memory_write_allowed {
        0.0
    } else {
        0.05
    };
    let runtime_pressure = if input.runtime_kv_hold { 0.04 } else { 0.0 };

    clamp_unit(
        base + contradiction_pressure
            + critical_pressure
            + revision_pressure
            + gate_pressure
            + coordination_pressure
            + memory_pressure
            + runtime_pressure,
    )
}

fn relabel_payload(gene: &ReasoningGene) -> (String, String, Vec<String>) {
    let label = if gene.label.trim().is_empty() {
        canonical_label(gene.kind).to_owned()
    } else {
        format!("refreshed {}", gene.label.trim())
    };
    let purpose = if gene.purpose.trim().is_empty() {
        canonical_purpose(gene.kind).to_owned()
    } else {
        format!(
            "{}; refreshed to preserve its current function after aging evidence",
            gene.purpose.trim()
        )
    };
    let mut tags = gene.tags.clone();
    push_tag_once(&mut tags, gene.kind.as_str());
    push_tag_once(&mut tags, "relabel");
    push_tag_once(&mut tags, "youth_renewal");
    (label, purpose, tags)
}

fn regeneration_payload(gene: &ReasoningGene) -> (String, String, Vec<String>) {
    let label = format!("regenerated {}", canonical_label(gene.kind));
    let purpose = format!(
        "{}; young candidate rebuilt from the stable anchor after malignant drift isolation",
        canonical_purpose(gene.kind)
    );
    let mut tags = gene.tags.clone();
    push_tag_once(&mut tags, gene.kind.as_str());
    push_tag_once(&mut tags, "quarantine");
    push_tag_once(&mut tags, "regenerate");
    push_tag_once(&mut tags, "stable_anchor");
    (label, purpose, tags)
}

fn regenerated_gene_id(gene: &ReasoningGene) -> String {
    format!("{}:young", gene.id)
}

fn tombstone_id(gene: &ReasoningGene) -> String {
    format!("tombstone:{}", gene.id)
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

fn strategy_gene<const N: usize>(
    strategy: &str,
    lane: &str,
    kind: ReasoningGeneKind,
    label: &str,
    purpose: &str,
    tags: [&str; N],
) -> ReasoningGene {
    ReasoningGene::new(
        format!("gene:strategy:{strategy}:{lane}"),
        kind,
        label,
        purpose,
    )
    .with_tags(tags)
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

fn push_tag_once(tags: &mut Vec<String>, tag: &str) {
    if !tags.iter().any(|existing| existing == tag) {
        tags.push(tag.to_owned());
    }
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn default_validation_gates() -> Vec<String> {
    vec![
        "cargo_fmt".to_owned(),
        "focused_rust_tests".to_owned(),
        "trace_schema_gate".to_owned(),
        "benchmark_gate".to_owned(),
        "rollback_anchor_present".to_owned(),
        "operator_approval_required".to_owned(),
    ]
}
