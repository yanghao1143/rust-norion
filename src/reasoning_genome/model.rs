use crate::hierarchy::TaskProfile;

const AGING_AGE_THRESHOLD: u32 = 8;
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

    pub fn derived_status(&self) -> ReasoningGeneStatus {
        if self.drift_score >= MALIGNANT_DRIFT_THRESHOLD {
            ReasoningGeneStatus::Malignant
        } else if self.age >= AGING_AGE_THRESHOLD || self.fitness < LOW_FITNESS_THRESHOLD {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationPlan {
    pub id: String,
    pub intent: GeneScissorsIntent,
    pub target_gene_id: String,
    pub source_gene_ids: Vec<String>,
    pub replacement_gene_id: Option<String>,
    pub reason: String,
    pub expected_effect: String,
    pub rollback_anchor_id: String,
    pub validation_gates: Vec<String>,
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
        Self {
            id: id.into(),
            intent,
            target_gene_id: target_gene_id.into(),
            source_gene_ids: Vec::new(),
            replacement_gene_id: None,
            reason: reason.into(),
            expected_effect: expected_effect.into(),
            rollback_anchor_id: rollback_anchor_id.into(),
            validation_gates: default_validation_gates(),
            admission_write_authorized: false,
            applied: false,
        }
    }

    pub fn with_sources(mut self, sources: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.source_gene_ids = sources.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_replacement(mut self, replacement_gene_id: impl Into<String>) -> Self {
        self.replacement_gene_id = Some(replacement_gene_id.into());
        self
    }

    pub fn is_read_only_preview(&self) -> bool {
        !self.admission_write_authorized && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReasoningGenome {
    pub id: String,
    pub profile: TaskProfile,
    pub stable_anchor_id: String,
    pub genes: Vec<ReasoningGene>,
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

    pub fn express(&self, input: GenomeExpressionInput) -> GenomeExpression {
        let mut active_gene_ids = Vec::new();
        let mut aged_gene_ids = Vec::new();
        let mut malignant_gene_ids = Vec::new();
        let mut relabel_candidate_ids = Vec::new();
        let mut regeneration_candidate_ids = Vec::new();
        let mut mutation_plans = Vec::new();

        for gene in &self.genes {
            match gene.derived_status() {
                ReasoningGeneStatus::Active => active_gene_ids.push(gene.id.clone()),
                ReasoningGeneStatus::Aging => {
                    active_gene_ids.push(gene.id.clone());
                    aged_gene_ids.push(gene.id.clone());
                    if gene.needs_relabel() {
                        relabel_candidate_ids.push(gene.id.clone());
                        mutation_plans.push(MutationPlan::preview(
                            format!("mutation:{}:relabel", gene.id),
                            GeneScissorsIntent::Relabel,
                            gene.id.clone(),
                            "gene label or purpose is aging and needs refreshed function metadata",
                            "refresh label and purpose while preserving the stable gene anchor",
                            self.stable_anchor_id.clone(),
                        ));
                    }
                }
                ReasoningGeneStatus::Malignant => {
                    malignant_gene_ids.push(gene.id.clone());
                    regeneration_candidate_ids.push(gene.id.clone());
                    mutation_plans.push(
                        MutationPlan::preview(
                            format!("mutation:{}:quarantine", gene.id),
                            GeneScissorsIntent::Quarantine,
                            gene.id.clone(),
                            "gene drift crossed malignant threshold and must be isolated before reuse",
                            "stop expression of the contaminated strategy while preserving audit evidence",
                            self.stable_anchor_id.clone(),
                        )
                        .with_sources([gene.id.clone()]),
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
                        .with_sources([self.stable_anchor_id.clone()]),
                    );
                }
                ReasoningGeneStatus::Quarantined | ReasoningGeneStatus::Regenerating => {}
            }
        }

        if input.drift_rollback || input.critical_reflection_issue_count > 0 {
            let target = self
                .genes
                .iter()
                .find(|gene| gene.kind == ReasoningGeneKind::Safety)
                .or_else(|| self.genes.first());
            if let Some(gene) = target {
                if !malignant_gene_ids.contains(&gene.id) {
                    malignant_gene_ids.push(gene.id.clone());
                    regeneration_candidate_ids.push(gene.id.clone());
                    mutation_plans.push(
                        MutationPlan::preview(
                            format!("mutation:{}:quarantine", gene.id),
                            GeneScissorsIntent::Quarantine,
                            gene.id.clone(),
                            "runtime drift marked this safety gene as unsafe to express",
                            "isolate the unstable strategy before rollback or regeneration",
                            self.stable_anchor_id.clone(),
                        )
                        .with_sources([gene.id.clone()]),
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
                        .with_sources([self.stable_anchor_id.clone()]),
                    );
                }
                mutation_plans.push(MutationPlan::preview(
                    format!("mutation:{}:rollback", gene.id),
                    GeneScissorsIntent::Rollback,
                    gene.id.clone(),
                    "runtime drift or critical reflection issue requires stable genome rollback",
                    "restore the previous stable genome before any durable mutation is admitted",
                    self.stable_anchor_id.clone(),
                ));
            }
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
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub youth_pressure: f32,
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
            drift_rollback: false,
            runtime_kv_hold: false,
        })
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

    pub fn is_read_only_preview(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .mutation_plans
                .iter()
                .all(MutationPlan::is_read_only_preview)
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
