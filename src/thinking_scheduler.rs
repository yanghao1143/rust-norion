use crate::hierarchy::{TaskAwareHierarchyPlan, TaskProfile};
use crate::privacy_redaction::stable_redaction_digest;
use crate::reasoning_genome::{DnaChainKind, DnaGeneChain, DnaGeneRecord, ReasoningGeneKind};
use crate::router::{AdaptiveRouteAction, AdaptiveRouteDecision, AdaptiveRoutingPlan};
use crate::router::{ComputeBudgetSchedule, Route};

pub const THINKING_SCHEDULER_SCHEMA_VERSION: &str = "thinking_scheduler_v1";
pub const REASONING_GENOME_PLAN_SCHEMA_VERSION: &str = "reasoning_genome_plan_v1";

const PHASE_ORDER: [ThinkingPhase; 7] = [
    ThinkingPhase::Planning,
    ThinkingPhase::MemoryRecall,
    ThinkingPhase::GenomeExpression,
    ThinkingPhase::RouteSelection,
    ThinkingPhase::AnswerSynthesis,
    ThinkingPhase::Verification,
    ThinkingPhase::Reflection,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingPhase {
    Planning,
    MemoryRecall,
    GenomeExpression,
    RouteSelection,
    AnswerSynthesis,
    Verification,
    Reflection,
}

impl ThinkingPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::MemoryRecall => "memory_recall",
            Self::GenomeExpression => "genome_expression",
            Self::RouteSelection => "route_selection",
            Self::AnswerSynthesis => "answer_synthesis",
            Self::Verification => "verification",
            Self::Reflection => "reflection",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingPhaseStatus {
    Planned,
    Skipped,
    Fallback,
    BudgetExhausted,
    Disabled,
}

impl ThinkingPhaseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Skipped => "skipped",
            Self::Fallback => "fallback",
            Self::BudgetExhausted => "budget_exhausted",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThinkingPhaseBudget {
    pub max_tokens: usize,
    pub spent_tokens: usize,
    pub max_steps: usize,
    pub spent_steps: usize,
}

impl ThinkingPhaseBudget {
    pub fn new(
        max_tokens: usize,
        spent_tokens: usize,
        max_steps: usize,
        spent_steps: usize,
    ) -> Self {
        Self {
            max_tokens,
            spent_tokens,
            max_steps,
            spent_steps,
        }
    }

    pub fn exhausted(self) -> bool {
        self.spent_tokens > self.max_tokens || self.spent_steps > self.max_steps
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingGeneSelection {
    pub phase: ThinkingPhase,
    pub chain_kind: DnaChainKind,
    pub gene_digest: String,
    pub gene_kind: ReasoningGeneKind,
    pub reason_code: String,
}

impl ThinkingGeneSelection {
    pub fn summary_line(&self) -> String {
        format!(
            "phase={} chain={} gene={} kind={} reason={}",
            self.phase.as_str(),
            self.chain_kind.as_str(),
            self.gene_digest,
            self.gene_kind.as_str(),
            self.reason_code
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingRouteSelection {
    pub phase: ThinkingPhase,
    pub candidate_digest: String,
    pub source: String,
    pub action: String,
    pub route: String,
    pub retained_tokens: usize,
    pub saved_tokens: usize,
    pub reason_code: String,
}

impl ThinkingRouteSelection {
    pub fn summary_line(&self) -> String {
        format!(
            "phase={} candidate={} source={} action={} route={} retained={} saved={} reason={}",
            self.phase.as_str(),
            self.candidate_digest,
            self.source,
            self.action,
            self.route,
            self.retained_tokens,
            self.saved_tokens,
            self.reason_code
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneThoughtFrame {
    pub phase: ThinkingPhase,
    pub task_genes: usize,
    pub constraint_genes: usize,
    pub routing_genes: usize,
    pub retrieval_genes: usize,
    pub reflection_genes: usize,
    pub output_policy_genes: usize,
    pub selected_gene_digests: Vec<String>,
    pub rejected_intron_count: usize,
    pub splice_window_count: usize,
    pub route_candidate_digests: Vec<String>,
    pub attention_budget_tokens: usize,
    pub routing_budget_decisions: usize,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl GeneThoughtFrame {
    pub fn summary_line(&self) -> String {
        format!(
            "gene_thought_frame phase={} task_genes={} constraint_genes={} routing_genes={} retrieval_genes={} reflection_genes={} output_policy_genes={} selected_genes={} rejected_introns={} splice_windows={} route_candidates={} attention_budget={} routing_budget_decisions={} read_only={} write_allowed={} applied={}",
            self.phase.as_str(),
            self.task_genes,
            self.constraint_genes,
            self.routing_genes,
            self.retrieval_genes,
            self.reflection_genes,
            self.output_policy_genes,
            self.selected_gene_digests.len(),
            self.rejected_intron_count,
            self.splice_window_count,
            self.route_candidate_digests.len(),
            self.attention_budget_tokens,
            self.routing_budget_decisions,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningGenomePlan {
    pub schema_version: &'static str,
    pub prompt_digest: String,
    pub profile: TaskProfile,
    pub frames: Vec<GeneThoughtFrame>,
    pub selected_gene_count: usize,
    pub rejected_intron_count: usize,
    pub splice_window_count: usize,
    pub routing_budget_decisions: usize,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl ReasoningGenomePlan {
    pub fn validate_preview(&self) -> Result<(), &'static str> {
        if !self.prompt_digest.starts_with("redaction-digest:") {
            return Err("prompt_digest_not_redacted");
        }
        if !self.read_only || self.write_allowed || self.applied {
            return Err("reasoning_genome_plan_not_preview_only");
        }
        if self
            .frames
            .iter()
            .any(|frame| !frame.read_only || frame.write_allowed || frame.applied)
        {
            return Err("gene_thought_frame_not_preview_only");
        }
        Ok(())
    }

    pub fn summary_line(&self) -> String {
        format!(
            "reasoning_genome_plan schema={} profile={} frames={} selected_genes={} rejected_introns={} splice_windows={} routing_budget_decisions={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            profile_slug(self.profile),
            self.frames.len(),
            self.selected_gene_count,
            self.rejected_intron_count,
            self.splice_window_count,
            self.routing_budget_decisions,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingPhaseTrace {
    pub phase: ThinkingPhase,
    pub status: ThinkingPhaseStatus,
    pub budget: ThinkingPhaseBudget,
    pub selected_gene_digests: Vec<String>,
    pub route_candidate_digests: Vec<String>,
    pub fallback_reasons: Vec<String>,
    pub skip_reasons: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl ThinkingPhaseTrace {
    pub fn summary_line(&self) -> String {
        format!(
            "thinking_phase phase={} status={} spent={}/{} steps={}/{} genes={} routes={} fallback={} skip={} read_only={} write_allowed={} applied={}",
            self.phase.as_str(),
            self.status.as_str(),
            self.budget.spent_tokens,
            self.budget.max_tokens,
            self.budget.spent_steps,
            self.budget.max_steps,
            self.selected_gene_digests.join("|"),
            self.route_candidate_digests.join("|"),
            self.fallback_reasons.join("|"),
            self.skip_reasons.join("|"),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThinkingSchedulerPolicy {
    pub enabled: bool,
    pub phase_token_budget: usize,
    pub phase_step_budget: usize,
    pub max_express_genes: usize,
    pub max_memory_genes: usize,
    pub max_route_decisions: usize,
    pub min_gene_trust: f32,
    pub max_gene_drift: f32,
}

impl Default for ThinkingSchedulerPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            phase_token_budget: 512,
            phase_step_budget: 8,
            max_express_genes: 3,
            max_memory_genes: 3,
            max_route_decisions: 4,
            min_gene_trust: 0.40,
            max_gene_drift: 0.62,
        }
    }
}

impl ThinkingSchedulerPolicy {
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn with_phase_token_budget(mut self, phase_token_budget: usize) -> Self {
        self.phase_token_budget = phase_token_budget.max(1);
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThinkingSchedulerInput<'a> {
    pub prompt: &'a str,
    pub task_plan: &'a TaskAwareHierarchyPlan,
    pub dna_chain: &'a DnaGeneChain,
    pub routing_plan: &'a AdaptiveRoutingPlan,
    pub compute_budget: &'a ComputeBudgetSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingScheduleReport {
    pub schema_version: &'static str,
    pub prompt_digest: String,
    pub profile: TaskProfile,
    pub disabled: bool,
    pub adapter_passthrough: bool,
    pub adapter_behavior_changed: bool,
    pub active_generation_without_durable_writes: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub phases: Vec<ThinkingPhaseTrace>,
    pub gene_selections: Vec<ThinkingGeneSelection>,
    pub route_selections: Vec<ThinkingRouteSelection>,
    pub reasoning_plan: ReasoningGenomePlan,
    pub fallback_reasons: Vec<String>,
    pub skip_reasons: Vec<String>,
}

impl ThinkingScheduleReport {
    pub fn phase(&self, phase: ThinkingPhase) -> Option<&ThinkingPhaseTrace> {
        self.phases.iter().find(|trace| trace.phase == phase)
    }

    pub fn phase_status(&self, phase: ThinkingPhase) -> Option<ThinkingPhaseStatus> {
        self.phase(phase).map(|trace| trace.status)
    }

    pub fn is_read_only_preview(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self.active_generation_without_durable_writes
            && !self.adapter_behavior_changed
            && self.reasoning_plan.validate_preview().is_ok()
            && self
                .phases
                .iter()
                .all(|phase| phase.read_only && !phase.write_allowed && !phase.applied)
    }

    pub fn selected_gene_digests(&self) -> Vec<String> {
        let mut digests = Vec::new();
        for selection in &self.gene_selections {
            push_unique(&mut digests, selection.gene_digest.clone());
        }
        digests
    }

    pub fn summary_line(&self) -> String {
        format!(
            "thinking_scheduler schema={} profile={} disabled={} phases={} genes={} routes={} fallback={} skip={} passthrough={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            profile_slug(self.profile),
            self.disabled,
            self.phases.len(),
            self.gene_selections.len(),
            self.route_selections.len(),
            self.fallback_reasons.join("|"),
            self.skip_reasons.join("|"),
            self.adapter_passthrough,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone)]
pub struct ThinkingScheduler {
    pub policy: ThinkingSchedulerPolicy,
}

impl Default for ThinkingScheduler {
    fn default() -> Self {
        Self {
            policy: ThinkingSchedulerPolicy::default(),
        }
    }
}

impl ThinkingScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: ThinkingSchedulerPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn schedule(&self, input: ThinkingSchedulerInput<'_>) -> ThinkingScheduleReport {
        if !self.policy.enabled {
            return disabled_report(input);
        }

        let threshold = gene_threshold(input.task_plan, input.routing_plan);
        let memory_genes = select_gene_records(
            ThinkingPhase::MemoryRecall,
            DnaChainKind::Memory,
            &input.dna_chain.memory_chain,
            self.policy.max_memory_genes,
            threshold,
            self.policy,
        );
        let genome_genes = select_gene_records(
            ThinkingPhase::GenomeExpression,
            DnaChainKind::Express,
            &input.dna_chain.express_chain,
            self.policy.max_express_genes,
            threshold,
            self.policy,
        );
        let synthesis_genes = select_gene_records(
            ThinkingPhase::AnswerSynthesis,
            DnaChainKind::Express,
            &input.dna_chain.express_chain,
            self.policy.max_express_genes,
            threshold,
            self.policy,
        );
        let verification_genes = select_gene_records(
            ThinkingPhase::Verification,
            DnaChainKind::Express,
            &input.dna_chain.express_chain,
            self.policy.max_express_genes,
            threshold,
            self.policy,
        );
        let reflection_genes = select_gene_records(
            ThinkingPhase::Reflection,
            DnaChainKind::Express,
            &input.dna_chain.express_chain,
            self.policy.max_express_genes,
            threshold,
            self.policy,
        );
        let route_selections = select_route_decisions(
            input.routing_plan,
            self.policy.max_route_decisions,
            ThinkingPhase::RouteSelection,
        );
        let route_fallbacks = route_fallback_reasons(input.routing_plan, input.compute_budget);

        let mut gene_selections = Vec::new();
        gene_selections.extend(memory_genes.clone());
        gene_selections.extend(genome_genes.clone());
        gene_selections.extend(synthesis_genes.clone());
        gene_selections.extend(verification_genes.clone());
        gene_selections.extend(reflection_genes.clone());

        let phase_inputs = [
            phase_input(
                ThinkingPhase::Planning,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                planning_skip_reasons(input.task_plan),
                planning_spent_tokens(input.task_plan),
                1,
            ),
            phase_input(
                ThinkingPhase::MemoryRecall,
                memory_genes.clone(),
                Vec::new(),
                Vec::new(),
                empty_selection_skip(memory_genes.is_empty(), "no_memory_chain_segments_selected"),
                memory_genes.len().saturating_mul(24),
                memory_genes.len().max(1),
            ),
            phase_input(
                ThinkingPhase::GenomeExpression,
                genome_genes.clone(),
                Vec::new(),
                empty_selection_fallback(
                    genome_genes.is_empty(),
                    !input.dna_chain.express_chain.is_empty(),
                    "no_express_chain_segment_passed_threshold",
                ),
                Vec::new(),
                genome_genes.len().saturating_mul(32),
                genome_genes.len().max(1),
            ),
            phase_input(
                ThinkingPhase::RouteSelection,
                Vec::new(),
                route_selections.clone(),
                route_fallbacks.clone(),
                empty_route_skip(input.routing_plan.decisions.is_empty()),
                route_selections.len().saturating_mul(16),
                route_selections.len().max(1),
            ),
            phase_input(
                ThinkingPhase::AnswerSynthesis,
                synthesis_genes.clone(),
                Vec::new(),
                answer_fallbacks(&route_fallbacks),
                Vec::new(),
                32usize.saturating_add(synthesis_genes.len().saturating_mul(16)),
                1,
            ),
            phase_input(
                ThinkingPhase::Verification,
                verification_genes.clone(),
                Vec::new(),
                Vec::new(),
                verification_skip_reasons(input.compute_budget),
                input.compute_budget.validation_cost_tokens,
                input.compute_budget.validation_run_budget.max(1),
            ),
            phase_input(
                ThinkingPhase::Reflection,
                reflection_genes.clone(),
                Vec::new(),
                Vec::new(),
                reflection_skip_reasons(input.compute_budget),
                input
                    .compute_budget
                    .reflection_pass_budget
                    .saturating_mul(64),
                input.compute_budget.reflection_pass_budget.max(1),
            ),
        ];

        let phases = phase_inputs
            .into_iter()
            .map(|input| self.build_phase_trace(input))
            .collect::<Vec<_>>();
        let fallback_reasons = collect_unique_reasons(&phases, ReasonKind::Fallback);
        let skip_reasons = collect_unique_reasons(&phases, ReasonKind::Skip);
        let reasoning_plan = build_reasoning_genome_plan(
            input.prompt,
            input.task_plan.profile,
            input.dna_chain,
            &phases,
            &gene_selections,
            &route_selections,
        );

        ThinkingScheduleReport {
            schema_version: THINKING_SCHEDULER_SCHEMA_VERSION,
            prompt_digest: stable_redaction_digest(["thinking-scheduler", input.prompt]),
            profile: input.task_plan.profile,
            disabled: false,
            adapter_passthrough: false,
            adapter_behavior_changed: false,
            active_generation_without_durable_writes: true,
            read_only: true,
            write_allowed: false,
            applied: false,
            phases,
            gene_selections,
            route_selections,
            reasoning_plan,
            fallback_reasons,
            skip_reasons,
        }
    }

    fn build_phase_trace(&self, input: PhaseBuildInput) -> ThinkingPhaseTrace {
        let budget = ThinkingPhaseBudget::new(
            self.policy.phase_token_budget,
            input.spent_tokens,
            self.policy.phase_step_budget,
            input.spent_steps,
        );
        let status = if budget.exhausted() {
            ThinkingPhaseStatus::BudgetExhausted
        } else if !input.fallback_reasons.is_empty() {
            ThinkingPhaseStatus::Fallback
        } else if !input.skip_reasons.is_empty() {
            ThinkingPhaseStatus::Skipped
        } else {
            ThinkingPhaseStatus::Planned
        };

        ThinkingPhaseTrace {
            phase: input.phase,
            status,
            budget,
            selected_gene_digests: input
                .gene_selections
                .iter()
                .map(|selection| selection.gene_digest.clone())
                .collect(),
            route_candidate_digests: input
                .route_selections
                .iter()
                .map(|selection| selection.candidate_digest.clone())
                .collect(),
            fallback_reasons: input.fallback_reasons,
            skip_reasons: input.skip_reasons,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }
}

#[derive(Debug, Clone)]
struct PhaseBuildInput {
    phase: ThinkingPhase,
    gene_selections: Vec<ThinkingGeneSelection>,
    route_selections: Vec<ThinkingRouteSelection>,
    fallback_reasons: Vec<String>,
    skip_reasons: Vec<String>,
    spent_tokens: usize,
    spent_steps: usize,
}

#[derive(Debug, Clone, Copy)]
enum ReasonKind {
    Fallback,
    Skip,
}

fn phase_input(
    phase: ThinkingPhase,
    gene_selections: Vec<ThinkingGeneSelection>,
    route_selections: Vec<ThinkingRouteSelection>,
    fallback_reasons: Vec<String>,
    skip_reasons: Vec<String>,
    spent_tokens: usize,
    spent_steps: usize,
) -> PhaseBuildInput {
    PhaseBuildInput {
        phase,
        gene_selections,
        route_selections,
        fallback_reasons,
        skip_reasons,
        spent_tokens,
        spent_steps,
    }
}

fn disabled_report(input: ThinkingSchedulerInput<'_>) -> ThinkingScheduleReport {
    let phases = PHASE_ORDER
        .iter()
        .map(|phase| ThinkingPhaseTrace {
            phase: *phase,
            status: ThinkingPhaseStatus::Disabled,
            budget: ThinkingPhaseBudget::new(0, 0, 0, 0),
            selected_gene_digests: Vec::new(),
            route_candidate_digests: Vec::new(),
            fallback_reasons: vec!["scheduler_disabled_adapter_passthrough".to_owned()],
            skip_reasons: vec!["feature_flag_disabled".to_owned()],
            read_only: true,
            write_allowed: false,
            applied: false,
        })
        .collect::<Vec<_>>();
    let reasoning_plan = build_reasoning_genome_plan(
        input.prompt,
        input.task_plan.profile,
        input.dna_chain,
        &phases,
        &[],
        &[],
    );

    ThinkingScheduleReport {
        schema_version: THINKING_SCHEDULER_SCHEMA_VERSION,
        prompt_digest: stable_redaction_digest(["thinking-scheduler", input.prompt]),
        profile: input.task_plan.profile,
        disabled: true,
        adapter_passthrough: true,
        adapter_behavior_changed: false,
        active_generation_without_durable_writes: true,
        read_only: true,
        write_allowed: false,
        applied: false,
        phases,
        gene_selections: Vec::new(),
        route_selections: Vec::new(),
        reasoning_plan,
        fallback_reasons: vec!["scheduler_disabled_adapter_passthrough".to_owned()],
        skip_reasons: vec!["feature_flag_disabled".to_owned()],
    }
}

fn build_reasoning_genome_plan(
    prompt: &str,
    profile: TaskProfile,
    dna_chain: &DnaGeneChain,
    phases: &[ThinkingPhaseTrace],
    gene_selections: &[ThinkingGeneSelection],
    route_selections: &[ThinkingRouteSelection],
) -> ReasoningGenomePlan {
    let frames = phases
        .iter()
        .map(|phase| gene_thought_frame(phase, dna_chain, gene_selections, route_selections))
        .collect::<Vec<_>>();
    let selected_gene_count = gene_selections.len();
    let rejected_intron_count = frames.iter().map(|frame| frame.rejected_intron_count).sum();
    let splice_window_count = frames.iter().map(|frame| frame.splice_window_count).sum();
    let routing_budget_decisions = frames
        .iter()
        .map(|frame| frame.routing_budget_decisions)
        .sum();

    ReasoningGenomePlan {
        schema_version: REASONING_GENOME_PLAN_SCHEMA_VERSION,
        prompt_digest: stable_redaction_digest(["reasoning-genome-plan", prompt]),
        profile,
        frames,
        selected_gene_count,
        rejected_intron_count,
        splice_window_count,
        routing_budget_decisions,
        read_only: true,
        write_allowed: false,
        applied: false,
    }
}

fn gene_thought_frame(
    phase: &ThinkingPhaseTrace,
    dna_chain: &DnaGeneChain,
    gene_selections: &[ThinkingGeneSelection],
    route_selections: &[ThinkingRouteSelection],
) -> GeneThoughtFrame {
    let phase_genes = gene_selections
        .iter()
        .filter(|selection| selection.phase == phase.phase)
        .collect::<Vec<_>>();
    let phase_routes = route_selections
        .iter()
        .filter(|selection| selection.phase == phase.phase)
        .collect::<Vec<_>>();
    let selected_gene_digests = phase_genes
        .iter()
        .map(|selection| selection.gene_digest.clone())
        .collect::<Vec<_>>();

    GeneThoughtFrame {
        phase: phase.phase,
        task_genes: phase_genes
            .iter()
            .filter(|selection| {
                matches!(
                    selection.gene_kind,
                    ReasoningGeneKind::Language | ReasoningGeneKind::ToolUse
                )
            })
            .count(),
        constraint_genes: phase_genes
            .iter()
            .filter(|selection| {
                matches!(
                    selection.gene_kind,
                    ReasoningGeneKind::Safety | ReasoningGeneKind::Budget
                )
            })
            .count(),
        routing_genes: phase_genes
            .iter()
            .filter(|selection| selection.gene_kind == ReasoningGeneKind::Routing)
            .count(),
        retrieval_genes: phase_genes
            .iter()
            .filter(|selection| selection.gene_kind == ReasoningGeneKind::Retrieval)
            .count(),
        reflection_genes: phase_genes
            .iter()
            .filter(|selection| selection.gene_kind == ReasoningGeneKind::Reflection)
            .count(),
        output_policy_genes: phase_genes
            .iter()
            .filter(|selection| {
                matches!(
                    selection.phase,
                    ThinkingPhase::AnswerSynthesis | ThinkingPhase::Verification
                )
            })
            .count(),
        rejected_intron_count: rejected_intron_count(
            phase.phase,
            dna_chain,
            &selected_gene_digests,
        ),
        splice_window_count: splice_window_count(phase.phase, dna_chain),
        route_candidate_digests: phase_routes
            .iter()
            .map(|selection| selection.candidate_digest.clone())
            .collect(),
        attention_budget_tokens: phase.budget.max_tokens,
        routing_budget_decisions: phase_routes.len(),
        read_only: phase.read_only,
        write_allowed: phase.write_allowed,
        applied: phase.applied,
        selected_gene_digests,
    }
}

fn rejected_intron_count(
    phase: ThinkingPhase,
    dna_chain: &DnaGeneChain,
    selected_gene_digests: &[String],
) -> usize {
    chain_records_for_phase(phase, dna_chain)
        .into_iter()
        .filter(|record| {
            let digest = gene_digest(record.chain_kind, record);
            !selected_gene_digests
                .iter()
                .any(|selected| selected == &digest)
        })
        .count()
}

fn splice_window_count(phase: ThinkingPhase, dna_chain: &DnaGeneChain) -> usize {
    usize::from(!chain_records_for_phase(phase, dna_chain).is_empty())
}

fn chain_records_for_phase<'a>(
    phase: ThinkingPhase,
    dna_chain: &'a DnaGeneChain,
) -> Vec<&'a DnaGeneRecord> {
    match phase {
        ThinkingPhase::MemoryRecall => dna_chain.memory_chain.iter().collect(),
        ThinkingPhase::GenomeExpression
        | ThinkingPhase::AnswerSynthesis
        | ThinkingPhase::Verification
        | ThinkingPhase::Reflection => dna_chain.express_chain.iter().collect(),
        ThinkingPhase::Planning | ThinkingPhase::RouteSelection => Vec::new(),
    }
}

fn select_gene_records(
    phase: ThinkingPhase,
    chain_kind: DnaChainKind,
    records: &[DnaGeneRecord],
    limit: usize,
    threshold: f32,
    policy: ThinkingSchedulerPolicy,
) -> Vec<ThinkingGeneSelection> {
    let mut scored = records
        .iter()
        .filter(|record| record.chain_kind == chain_kind)
        .filter(|record| record.trust_score >= policy.min_gene_trust)
        .filter(|record| record.drift_score <= policy.max_gene_drift)
        .filter_map(|record| {
            let score = gene_score(record, phase);
            (score >= threshold).then_some((record, score))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.gene_id.cmp(&right.0.gene_id))
    });

    scored
        .into_iter()
        .take(limit.max(1))
        .map(|(record, _)| ThinkingGeneSelection {
            phase,
            chain_kind,
            gene_digest: gene_digest(chain_kind, record),
            gene_kind: record.gene_kind,
            reason_code: gene_reason_code(phase, record.gene_kind),
        })
        .collect()
}

fn select_route_decisions(
    routing_plan: &AdaptiveRoutingPlan,
    limit: usize,
    phase: ThinkingPhase,
) -> Vec<ThinkingRouteSelection> {
    let mut retained = routing_plan
        .decisions
        .iter()
        .filter(|decision| decision.action.retains_tokens())
        .collect::<Vec<_>>();
    retained.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });

    retained
        .into_iter()
        .take(limit.max(1))
        .map(|decision| route_selection(phase, decision))
        .collect()
}

fn route_selection(
    phase: ThinkingPhase,
    decision: &AdaptiveRouteDecision,
) -> ThinkingRouteSelection {
    ThinkingRouteSelection {
        phase,
        candidate_digest: stable_redaction_digest([
            "route-candidate",
            decision.candidate_id.as_str(),
            decision.source.as_str(),
        ]),
        source: decision.source.as_str().to_owned(),
        action: decision.action.as_str().to_owned(),
        route: decision.route.as_str().to_owned(),
        retained_tokens: decision.retained_tokens,
        saved_tokens: decision.saved_tokens(),
        reason_code: route_reason_code(decision.action, decision.route),
    }
}

fn gene_score(record: &DnaGeneRecord, phase: ThinkingPhase) -> f32 {
    let trust = finite_unit(record.trust_score);
    let fitness = finite_unit(record.fitness_score);
    let drift_relief = 1.0 - finite_unit(record.drift_score);
    let decay_relief = 1.0 - finite_unit(record.decay_score);
    let affinity = phase_affinity(phase, record.gene_kind);
    (trust * 0.42 + fitness * 0.18 + drift_relief * 0.18 + decay_relief * 0.08 + affinity * 0.14)
        .clamp(0.0, 1.0)
}

fn phase_affinity(phase: ThinkingPhase, kind: ReasoningGeneKind) -> f32 {
    match phase {
        ThinkingPhase::Planning => match kind {
            ReasoningGeneKind::Budget | ReasoningGeneKind::Safety => 1.0,
            ReasoningGeneKind::Routing => 0.8,
            _ => 0.35,
        },
        ThinkingPhase::MemoryRecall => match kind {
            ReasoningGeneKind::Retrieval => 1.0,
            ReasoningGeneKind::Budget | ReasoningGeneKind::Safety => 0.65,
            _ => 0.30,
        },
        ThinkingPhase::GenomeExpression => 0.80,
        ThinkingPhase::RouteSelection => match kind {
            ReasoningGeneKind::Routing | ReasoningGeneKind::Budget => 1.0,
            ReasoningGeneKind::Safety => 0.75,
            _ => 0.30,
        },
        ThinkingPhase::AnswerSynthesis => match kind {
            ReasoningGeneKind::Language | ReasoningGeneKind::ToolUse => 1.0,
            ReasoningGeneKind::Reflection => 0.75,
            _ => 0.35,
        },
        ThinkingPhase::Verification => match kind {
            ReasoningGeneKind::Safety | ReasoningGeneKind::Budget => 1.0,
            ReasoningGeneKind::Reflection => 0.70,
            _ => 0.25,
        },
        ThinkingPhase::Reflection => match kind {
            ReasoningGeneKind::Reflection => 1.0,
            ReasoningGeneKind::Safety => 0.75,
            _ => 0.30,
        },
    }
}

fn gene_digest(chain_kind: DnaChainKind, record: &DnaGeneRecord) -> String {
    stable_redaction_digest([
        "dna-gene",
        chain_kind.as_str(),
        record.gene_id.as_str(),
        record.gene_kind.as_str(),
        record.rollback_anchor_id.as_str(),
    ])
}

fn gene_reason_code(phase: ThinkingPhase, kind: ReasoningGeneKind) -> String {
    format!("{}_{}_gene_selected", phase.as_str(), kind.as_str())
}

fn route_reason_code(action: AdaptiveRouteAction, route: Route) -> String {
    format!("{}_{}_route_selected", action.as_str(), route.as_str())
}

fn gene_threshold(task_plan: &TaskAwareHierarchyPlan, routing_plan: &AdaptiveRoutingPlan) -> f32 {
    let hierarchy_threshold = finite_unit(task_plan.threshold_after);
    let route_threshold = finite_unit(routing_plan.threshold);
    hierarchy_threshold
        .min(route_threshold)
        .mul_add(0.55, 0.20)
        .clamp(0.20, 0.78)
}

fn route_fallback_reasons(
    routing_plan: &AdaptiveRoutingPlan,
    compute_budget: &ComputeBudgetSchedule,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if compute_budget.fallback_triggered {
        push_unique(
            &mut reasons,
            "compute_budget_requested_router_fallback".to_owned(),
        );
    }
    if !routing_plan.anchors_retained() {
        push_unique(&mut reasons, "correctness_anchor_not_retained".to_owned());
    }
    if routing_plan
        .decisions
        .iter()
        .all(|decision| !decision.action.retains_tokens())
    {
        push_unique(&mut reasons, "router_no_retained_decisions".to_owned());
    }
    for note in &compute_budget.notes {
        if note.contains("fallback") {
            push_unique(&mut reasons, note.clone());
        }
    }
    reasons
}

fn answer_fallbacks(route_fallbacks: &[String]) -> Vec<String> {
    if route_fallbacks.is_empty() {
        Vec::new()
    } else {
        vec!["answer_synthesis_uses_adapter_passthrough_after_route_fallback".to_owned()]
    }
}

fn planning_skip_reasons(task_plan: &TaskAwareHierarchyPlan) -> Vec<String> {
    let mut reasons = Vec::new();
    if task_plan.selected_lanes.is_empty() {
        reasons.push("task_hierarchy_selected_no_lanes".to_owned());
    }
    reasons
}

fn verification_skip_reasons(compute_budget: &ComputeBudgetSchedule) -> Vec<String> {
    if compute_budget.validation_run_budget == 0 {
        vec!["validation_not_requested_by_task_mode".to_owned()]
    } else {
        Vec::new()
    }
}

fn reflection_skip_reasons(compute_budget: &ComputeBudgetSchedule) -> Vec<String> {
    if compute_budget.reflection_pass_budget == 0 {
        vec!["reflection_budget_not_allocated".to_owned()]
    } else {
        Vec::new()
    }
}

fn empty_selection_skip(condition: bool, reason: &str) -> Vec<String> {
    if condition {
        vec![reason.to_owned()]
    } else {
        Vec::new()
    }
}

fn empty_selection_fallback(condition: bool, input_present: bool, reason: &str) -> Vec<String> {
    if condition && input_present {
        vec![reason.to_owned()]
    } else {
        Vec::new()
    }
}

fn empty_route_skip(condition: bool) -> Vec<String> {
    if condition {
        vec!["router_candidates_missing".to_owned()]
    } else {
        Vec::new()
    }
}

fn planning_spent_tokens(task_plan: &TaskAwareHierarchyPlan) -> usize {
    8usize
        .saturating_add(task_plan.selected_lanes.len().saturating_mul(2))
        .saturating_add(task_plan.memory_lanes.len().saturating_mul(2))
}

fn collect_unique_reasons(phases: &[ThinkingPhaseTrace], kind: ReasonKind) -> Vec<String> {
    let mut reasons = Vec::new();
    for phase in phases {
        let source = match kind {
            ReasonKind::Fallback => &phase.fallback_reasons,
            ReasonKind::Skip => &phase.skip_reasons,
        };
        for reason in source {
            push_unique(&mut reasons, reason.clone());
        }
        if phase.status == ThinkingPhaseStatus::BudgetExhausted {
            push_unique(
                &mut reasons,
                format!("{}_phase_budget_exhausted", phase.phase.as_str()),
            );
        }
    }
    reasons
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn profile_slug(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::{HierarchyWeights, TaskAwareHierarchyInput, TaskAwareHierarchyPlanner};
    use crate::reasoning_genome::{
        DnaGeneEvidenceKind, DnaGeneLineage, DnaGeneSourceEvidence, ReasoningFrame,
        ReasoningFrameEfficiencySnapshot, ReasoningGene, ReasoningGenome,
    };
    use crate::router::{
        AdaptiveRouteCandidate, AdaptiveRouteScoreComponents, AdaptiveRouteSource,
        AdaptiveRoutingPlanner, ComputeBudgetContext, RoutingContext,
    };

    #[test]
    fn thinking_scheduler_orders_phases_deterministically() {
        let fixture = fixture("fix Rust compile error with cargo test");
        let scheduler = ThinkingScheduler::new();

        let first = scheduler.schedule(fixture.input());
        let second = scheduler.schedule(fixture.input());

        assert_eq!(first.summary_line(), second.summary_line());
        assert_eq!(first.phases.len(), PHASE_ORDER.len());
        assert_eq!(
            first
                .phases
                .iter()
                .map(|phase| phase.phase)
                .collect::<Vec<_>>(),
            PHASE_ORDER
        );
        assert!(first.is_read_only_preview());
        assert!(!first.adapter_behavior_changed);
    }

    #[test]
    fn thinking_scheduler_reports_budget_exhaustion_per_phase() {
        let fixture = fixture("benchmark long Rust compiler repair with validation");
        let scheduler = ThinkingScheduler::new()
            .with_policy(ThinkingSchedulerPolicy::default().with_phase_token_budget(8));

        let report = scheduler.schedule(fixture.input());

        assert!(
            report
                .phases
                .iter()
                .any(|phase| phase.status == ThinkingPhaseStatus::BudgetExhausted)
        );
        assert!(
            report
                .fallback_reasons
                .iter()
                .any(|reason| reason.ends_with("_phase_budget_exhausted"))
        );
        assert!(report.is_read_only_preview());
    }

    #[test]
    fn thinking_scheduler_emits_digest_only_segment_trace() {
        let fixture = fixture("use memory and genome chain for Rust answer synthesis");
        let report = ThinkingScheduler::new().schedule(fixture.input());

        assert!(!report.gene_selections.is_empty());
        assert!(
            report
                .selected_gene_digests()
                .iter()
                .all(|digest| digest.starts_with("redaction-digest:"))
        );
        let summaries = report
            .gene_selections
            .iter()
            .map(ThinkingGeneSelection::summary_line)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!summaries.contains("profile retrieval posture"));
        assert!(!summaries.contains("memory purpose"));
        assert!(summaries.contains("redaction-digest:"));
    }

    #[test]
    fn thinking_scheduler_builds_reasoning_genome_plan_preview() {
        let fixture = fixture("用 gene chain 规划 Rust 修复并保留验证证据");
        let report = ThinkingScheduler::new().schedule(fixture.input());
        let plan = &report.reasoning_plan;

        assert_eq!(plan.schema_version, REASONING_GENOME_PLAN_SCHEMA_VERSION);
        assert_eq!(plan.frames.len(), PHASE_ORDER.len());
        assert!(plan.selected_gene_count > 0);
        assert!(plan.splice_window_count > 0);
        assert!(plan.routing_budget_decisions > 0);
        assert!(plan.validate_preview().is_ok());
        assert!(plan.prompt_digest.starts_with("redaction-digest:"));
        assert!(!plan.summary_line().contains("Rust 修复"));
        assert!(plan.frames.iter().any(|frame| frame.retrieval_genes > 0));
        assert!(
            plan.frames
                .iter()
                .any(|frame| frame.routing_budget_decisions > 0)
        );
        assert!(
            plan.frames
                .iter()
                .all(|frame| frame.read_only && !frame.write_allowed && !frame.applied)
        );
    }

    #[test]
    fn reasoning_frame_efficiency_snapshot_uses_scheduler_feedback() {
        let fixture = fixture("用 gene chain 规划 Rust 修复并跑 cargo test");
        let report = ThinkingScheduler::new().schedule(fixture.input());
        let plan = &report.reasoning_plan;
        let budget = &fixture.compute_budget;
        let snapshot = ReasoningFrameEfficiencySnapshot::preview(
            plan.selected_gene_count,
            plan.rejected_intron_count,
            plan.splice_window_count,
            plan.routing_budget_decisions,
            budget.compute_budget.as_str(),
            budget.input_tokens,
            budget.retained_tokens,
            budget.saved_tokens,
            budget.validation_cost_tokens,
            0.87,
            0.91,
        );
        let frame = ReasoningFrame::issue375_preview(&plan.prompt_digest)
            .with_efficiency_snapshot(snapshot);
        let snapshot = frame.efficiency_snapshot.as_ref().expect("snapshot");

        assert!(frame.validate_preview().is_ok());
        assert!(snapshot.has_feedback_signal());
        assert!(snapshot.selected_gene_count > 0);
        assert!(snapshot.saved_tokens > 0 || snapshot.validation_cost_tokens > 0);
        assert!(snapshot.quality_milli > 0 || snapshot.process_reward_milli > 0);
        assert!(snapshot.read_only && !snapshot.write_allowed && !snapshot.applied);
        assert!(!format!("{snapshot:?}").contains("cargo test"));
    }

    #[test]
    fn reasoning_genome_plan_rejects_unredacted_or_write_enabled_preview() {
        let fixture = fixture("secret prompt must not leak");
        let report = ThinkingScheduler::new().schedule(fixture.input());
        let mut unredacted = report.reasoning_plan.clone();
        unredacted.prompt_digest = "secret prompt must not leak".to_owned();
        let mut write_enabled = report.reasoning_plan.clone();
        write_enabled.write_allowed = true;

        assert_eq!(
            unredacted.validate_preview(),
            Err("prompt_digest_not_redacted")
        );
        assert_eq!(
            write_enabled.validate_preview(),
            Err("reasoning_genome_plan_not_preview_only")
        );
    }

    #[test]
    fn thinking_scheduler_disabled_mode_keeps_adapter_passthrough() {
        let fixture = fixture("normal request");
        let scheduler =
            ThinkingScheduler::new().with_policy(ThinkingSchedulerPolicy::default().disabled());

        let report = scheduler.schedule(fixture.input());

        assert!(report.disabled);
        assert!(report.adapter_passthrough);
        assert!(!report.adapter_behavior_changed);
        assert!(report.gene_selections.is_empty());
        assert!(
            report
                .phases
                .iter()
                .all(|phase| phase.status == ThinkingPhaseStatus::Disabled)
        );
        assert!(report.is_read_only_preview());
    }

    #[test]
    fn thinking_scheduler_reports_router_fallback_behavior() {
        let mut fixture = fixture("low budget prompt should preserve anchor fallback");
        fixture.routing_plan = AdaptiveRoutingPlan::from_decisions(
            TaskProfile::Coding,
            0.90,
            vec![skipped_route_decision("prompt-fragment-with-private-text")],
        );
        fixture.compute_budget.fallback_triggered = true;
        fixture
            .compute_budget
            .notes
            .push("fallback_fast_projection_or_anchor_hold".to_owned());

        let report = ThinkingScheduler::new().schedule(fixture.input());

        assert_eq!(
            report.phase_status(ThinkingPhase::RouteSelection),
            Some(ThinkingPhaseStatus::Fallback)
        );
        assert!(
            report
                .fallback_reasons
                .contains(&"router_no_retained_decisions".to_owned())
        );
        assert!(
            report
                .fallback_reasons
                .contains(&"compute_budget_requested_router_fallback".to_owned())
        );
        assert!(report.is_read_only_preview());
    }

    struct Fixture {
        prompt: String,
        task_plan: TaskAwareHierarchyPlan,
        dna_chain: DnaGeneChain,
        routing_plan: AdaptiveRoutingPlan,
        compute_budget: ComputeBudgetSchedule,
    }

    impl Fixture {
        fn input(&self) -> ThinkingSchedulerInput<'_> {
            ThinkingSchedulerInput {
                prompt: &self.prompt,
                task_plan: &self.task_plan,
                dna_chain: &self.dna_chain,
                routing_plan: &self.routing_plan,
                compute_budget: &self.compute_budget,
            }
        }
    }

    fn fixture(prompt: &str) -> Fixture {
        let profile = TaskProfile::Coding;
        let task_plan = TaskAwareHierarchyPlanner::new().plan(TaskAwareHierarchyInput {
            prompt,
            profile,
            max_tokens: Some(512),
            prompt_tokens: 96,
            used_memories: 2,
            threshold_before: 0.48,
            hierarchy_before: HierarchyWeights::default(),
        });
        let mut dna_chain = DnaGeneChain::preview_from_genome(
            &ReasoningGenome::default_for_profile(profile),
            "tenant-local",
            "session-1",
            DnaGeneSourceEvidence::new(
                DnaGeneEvidenceKind::SyntheticDefault,
                "fixture-source",
                "scheduler fixture source",
            )
            .with_privacy_gate(),
        );
        let memory_gene = ReasoningGene::new(
            "memory:retrieval:compiler",
            ReasoningGeneKind::Retrieval,
            "memory retrieval",
            "memory purpose",
        )
        .with_tags(["memory", "compiler"])
        .with_health(1, 0.92, 0.04);
        dna_chain.push_memory_record(DnaGeneRecord::from_reasoning_gene(
            DnaChainKind::Memory,
            profile,
            "memory-anchor",
            DnaGeneLineage::new("tenant-local", "session-1"),
            DnaGeneSourceEvidence::new(
                DnaGeneEvidenceKind::MemoryAdmission,
                "memory-source",
                "admitted memory digest",
            )
            .with_privacy_gate(),
            &memory_gene,
        ));

        let routing_context = RoutingContext {
            profile,
            context_tokens: 96,
            cache_hit_rate: 0.60,
            latency_budget_ms: Some(600),
            hardware_pressure: 0.25,
            compute_headroom: 0.80,
            hierarchy: task_plan.hierarchy_after,
        };
        let candidates = vec![
            AdaptiveRouteCandidate::new(
                "prompt:anchor",
                AdaptiveRouteSource::PromptChunk,
                96,
                AdaptiveRouteScoreComponents::new(0.90, 0.80, 0.95, 0.82, 0.70, 0.90, 0.20, 0.85),
            )
            .with_anchor_required(true),
            AdaptiveRouteCandidate::new(
                "genome:reasoning",
                AdaptiveRouteSource::ReasoningGenome,
                72,
                AdaptiveRouteScoreComponents::new(0.85, 0.70, 0.88, 0.90, 0.60, 0.86, 0.25, 0.80),
            ),
        ];
        let budget_context =
            ComputeBudgetContext::from_task_plan(&task_plan, 96).with_max_tokens(Some(512));
        let budgeted = AdaptiveRoutingPlanner::new().plan_with_compute_budget(
            profile,
            0.42,
            routing_context,
            budget_context,
            candidates,
        );

        Fixture {
            prompt: prompt.to_owned(),
            task_plan,
            dna_chain,
            routing_plan: budgeted.routing_plan,
            compute_budget: budgeted.schedule,
        }
    }

    fn skipped_route_decision(id: &str) -> AdaptiveRouteDecision {
        AdaptiveRouteDecision {
            candidate_id: id.to_owned(),
            source: AdaptiveRouteSource::PromptChunk,
            estimated_tokens: 80,
            retained_tokens: 0,
            anchor_required: false,
            components: AdaptiveRouteScoreComponents::new(
                0.10, 0.10, 0.10, 0.10, 0.10, 0.10, 0.95, 0.10,
            ),
            score: 0.05,
            threshold: 0.90,
            route: Route::FastProjection,
            action: AdaptiveRouteAction::Skip,
            compute_pressure: 0.90,
            reason: "low score fallback fixture".to_owned(),
        }
    }
}
