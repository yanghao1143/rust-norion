use std::collections::{BTreeMap, BTreeSet};

use crate::agent_team::{AgentRole, AgentTeamPlan};
use crate::development_pollution::{
    DevelopmentEvidenceUseSurface, gate_development_evidence_payload_surface,
};
use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

pub const TASK_EXPRESSION_GENE_SCHEMA_VERSION: &str = "task_expression_gene_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskGeneGermLayer {
    Ectoderm,
    Mesoderm,
    Endoderm,
}

impl TaskGeneGermLayer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ectoderm => "ectoderm",
            Self::Mesoderm => "mesoderm",
            Self::Endoderm => "endoderm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskGeneCascadeMode {
    Serial,
    Parallel,
}

impl TaskGeneCascadeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Serial => "serial",
            Self::Parallel => "parallel",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskGeneAdmissionDecision {
    AcceptPreview,
    HoldForEvidence,
    RejectUnboundedSpawn,
    QuarantinePollutedPayload,
    RejectBudgetExhaustion,
    QuarantineConflict,
    RequireMainThreadApproval,
}

impl TaskGeneAdmissionDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptPreview => "accept_preview",
            Self::HoldForEvidence => "hold_for_evidence",
            Self::RejectUnboundedSpawn => "reject_unbounded_spawn",
            Self::QuarantinePollutedPayload => "quarantine_polluted_payload",
            Self::RejectBudgetExhaustion => "reject_budget_exhaustion",
            Self::QuarantineConflict => "quarantine_conflict",
            Self::RequireMainThreadApproval => "require_main_thread_approval",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskGeneSurfaceLink {
    ReasoningGenome,
    TaskSkillGeneCandidate,
    AgentTeamPlan,
    CrossWindowHandoffPacket,
    ThinkingSchedulerPhase,
    EvolutionGoalQueue,
    UnifiedWriterGate,
}

impl TaskGeneSurfaceLink {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReasoningGenome => "reasoning_genome",
            Self::TaskSkillGeneCandidate => "task_skill_gene_candidate",
            Self::AgentTeamPlan => "agent_team_plan",
            Self::CrossWindowHandoffPacket => "cross_window_handoff_packet",
            Self::ThinkingSchedulerPhase => "thinking_scheduler_phase",
            Self::EvolutionGoalQueue => "evolution_goal_queue",
            Self::UnifiedWriterGate => "unified_writer_gate",
        }
    }

    pub fn required() -> Vec<Self> {
        vec![
            Self::ReasoningGenome,
            Self::TaskSkillGeneCandidate,
            Self::AgentTeamPlan,
            Self::CrossWindowHandoffPacket,
            Self::ThinkingSchedulerPhase,
            Self::EvolutionGoalQueue,
            Self::UnifiedWriterGate,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskExpressionGene {
    pub schema_version: &'static str,
    pub gene_id: String,
    pub parent_gene_id: Option<String>,
    pub root_goal_id: String,
    pub task_profile: TaskProfile,
    pub role: AgentRole,
    pub lane: String,
    pub germ_layer: TaskGeneGermLayer,
    pub required_capabilities: Vec<String>,
    pub objective_digest: String,
    pub spawn_reason: String,
    pub dependencies: Vec<String>,
    pub tool_policy: String,
    pub memory_scope: String,
    pub budget_tokens: usize,
    pub spent_tokens: usize,
    pub child_depth: usize,
    pub stop_condition: String,
    pub validation_gate: String,
    pub validation_passed: bool,
    pub rollback_anchor: String,
    pub source_evidence_ids: Vec<String>,
    pub retired_version_block: Option<String>,
    pub blocked_payload: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl TaskExpressionGene {
    pub fn new(
        gene_id: impl Into<String>,
        root_goal_id: impl Into<String>,
        task_profile: TaskProfile,
        role: AgentRole,
        lane: impl Into<String>,
        objective: impl AsRef<str>,
        spawn_reason: impl AsRef<str>,
    ) -> Self {
        let objective = objective.as_ref();
        let spawn_reason = spawn_reason.as_ref();
        let blocked_payload = contains_private_or_executable_marker(objective)
            || contains_private_or_executable_marker(spawn_reason)
            || genome_expression_payload_blocked("task-expression-objective", objective)
            || genome_expression_payload_blocked("task-expression-spawn-reason", spawn_reason);
        let role_layer = germ_layer_for_role(role);
        Self {
            schema_version: TASK_EXPRESSION_GENE_SCHEMA_VERSION,
            gene_id: safe_ref(gene_id.into()),
            parent_gene_id: None,
            root_goal_id: safe_ref(root_goal_id.into()),
            task_profile,
            role,
            lane: safe_ref(lane.into()),
            germ_layer: role_layer,
            required_capabilities: default_capabilities(role),
            objective_digest: stable_redaction_digest(["task-expression-objective", objective]),
            spawn_reason: safe_text("spawn-reason", spawn_reason),
            dependencies: Vec::new(),
            tool_policy: "read_only_local_tools".to_owned(),
            memory_scope: "digest_only_blackboard".to_owned(),
            budget_tokens: 512,
            spent_tokens: 0,
            child_depth: 0,
            stop_condition: "validation_gate_or_budget_stop".to_owned(),
            validation_gate: "focused_validation_required".to_owned(),
            validation_passed: false,
            rollback_anchor: "preview_only_no_durable_write".to_owned(),
            source_evidence_ids: vec![stable_redaction_digest([
                "task-expression-source",
                objective,
                spawn_reason,
            ])],
            retired_version_block: None,
            blocked_payload,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn with_parent(mut self, parent_gene_id: impl Into<String>, child_depth: usize) -> Self {
        self.parent_gene_id = Some(safe_ref(parent_gene_id.into()));
        self.child_depth = child_depth;
        self
    }

    pub fn with_budget(mut self, budget_tokens: usize, spent_tokens: usize) -> Self {
        self.budget_tokens = budget_tokens.max(1);
        self.spent_tokens = spent_tokens;
        self
    }

    pub fn with_dependencies(mut self, dependencies: impl IntoIterator<Item = String>) -> Self {
        self.dependencies = dependencies.into_iter().map(safe_ref).collect();
        self
    }

    pub fn with_validation(mut self, validation_gate: impl Into<String>, passed: bool) -> Self {
        self.validation_gate = safe_ref(validation_gate.into());
        self.validation_passed = passed;
        self
    }

    pub fn with_evidence(mut self, evidence_ids: impl IntoIterator<Item = String>) -> Self {
        self.source_evidence_ids = evidence_ids.into_iter().map(safe_ref).collect();
        self
    }

    pub fn with_germ_layer(mut self, germ_layer: TaskGeneGermLayer, reason: &str) -> Self {
        self.germ_layer = germ_layer;
        self.spawn_reason = stable_redaction_digest([
            "task-expression-germ-layer-override",
            self.spawn_reason.as_str(),
            germ_layer.as_str(),
            reason,
        ]);
        self
    }

    pub fn with_retired_version_block(mut self, retired_version_block: impl Into<String>) -> Self {
        self.retired_version_block = Some(safe_ref(retired_version_block.into()));
        self
    }

    pub fn with_write_flags(mut self, read_only: bool, write_allowed: bool, applied: bool) -> Self {
        self.read_only = read_only;
        self.write_allowed = write_allowed;
        self.applied = applied;
        self
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGeneTraceCounters {
    pub task_gene_spawned: usize,
    pub task_gene_child_count: usize,
    pub task_gene_max_depth: usize,
    pub task_gene_rejected_unbounded_spawn: usize,
    pub task_gene_budget_stop: usize,
    pub task_gene_conflict_quarantine: usize,
    pub task_gene_preview_admission: usize,
    pub ectoderm_count: usize,
    pub mesoderm_count: usize,
    pub endoderm_count: usize,
}

impl TaskGeneTraceCounters {
    fn from_genes(genes: &[TaskExpressionGene]) -> Self {
        Self {
            task_gene_spawned: genes.len(),
            task_gene_child_count: genes
                .iter()
                .filter(|gene| gene.parent_gene_id.is_some())
                .count(),
            task_gene_max_depth: genes.iter().map(|gene| gene.child_depth).max().unwrap_or(0),
            task_gene_rejected_unbounded_spawn: 0,
            task_gene_budget_stop: genes
                .iter()
                .filter(|gene| gene.spent_tokens > gene.budget_tokens)
                .count(),
            task_gene_conflict_quarantine: 0,
            task_gene_preview_admission: 0,
            ectoderm_count: genes
                .iter()
                .filter(|gene| gene.germ_layer == TaskGeneGermLayer::Ectoderm)
                .count(),
            mesoderm_count: genes
                .iter()
                .filter(|gene| gene.germ_layer == TaskGeneGermLayer::Mesoderm)
                .count(),
            endoderm_count: genes
                .iter()
                .filter(|gene| gene.germ_layer == TaskGeneGermLayer::Endoderm)
                .count(),
        }
    }

    pub fn summary_fields(&self) -> String {
        format!(
            "task_gene_spawned={} task_gene_child_count={} task_gene_max_depth={} task_gene_rejected_unbounded_spawn={} task_gene_budget_stop={} task_gene_conflict_quarantine={} task_gene_preview_admission={} ectoderm={} mesoderm={} endoderm={}",
            self.task_gene_spawned,
            self.task_gene_child_count,
            self.task_gene_max_depth,
            self.task_gene_rejected_unbounded_spawn,
            self.task_gene_budget_stop,
            self.task_gene_conflict_quarantine,
            self.task_gene_preview_admission,
            self.ectoderm_count,
            self.mesoderm_count,
            self.endoderm_count
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGeneCascade {
    pub root_goal_id: String,
    pub max_depth: usize,
    pub max_children_per_gene: usize,
    pub serial_or_parallel_mode: TaskGeneCascadeMode,
    pub genes: Vec<TaskExpressionGene>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl TaskGeneCascade {
    pub fn new(
        root_goal_id: impl Into<String>,
        max_depth: usize,
        max_children_per_gene: usize,
        serial_or_parallel_mode: TaskGeneCascadeMode,
    ) -> Self {
        Self {
            root_goal_id: safe_ref(root_goal_id.into()),
            max_depth,
            max_children_per_gene: max_children_per_gene.max(1),
            serial_or_parallel_mode,
            genes: Vec::new(),
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn from_agent_team_plan(
        root_goal_id: impl Into<String>,
        task_profile: TaskProfile,
        plan: &AgentTeamPlan,
    ) -> Self {
        let root_goal_id = safe_ref(root_goal_id.into());
        let mut cascade = Self::new(
            root_goal_id.clone(),
            2,
            plan.aggregation.max_parallel_lanes.max(1),
            if plan.aggregation.max_parallel_lanes > 1 {
                TaskGeneCascadeMode::Parallel
            } else {
                TaskGeneCascadeMode::Serial
            },
        );
        let root = TaskExpressionGene::new(
            format!("{}-root", plan.run_id),
            &root_goal_id,
            task_profile,
            AgentRole::Planner,
            "control",
            &plan.main_thread_goal,
            "agent_team_plan_root",
        )
        .with_validation("agent_team_collision_free", plan.collision_free())
        .with_evidence(vec![plan.run_id.clone()]);
        cascade = cascade.with_gene(root);

        for agent in &plan.agents {
            let objective = format!("{}:{}", agent.role.as_str(), agent.objective);
            let gene = TaskExpressionGene::new(
                agent.id.clone(),
                &root_goal_id,
                task_profile,
                agent.role,
                agent.lane.clone(),
                objective,
                "agent_team_child_lane",
            )
            .with_parent(format!("{}-root", plan.run_id), 1)
            .with_dependencies(agent.dependencies.clone())
            .with_validation("agent_team_collision_free", plan.collision_free())
            .with_write_flags(!agent.writes_allowed, agent.writes_allowed, false)
            .with_evidence(vec![stable_redaction_digest([
                "agent-team-child",
                agent.id.as_str(),
                agent.role.as_str(),
            ])]);
            cascade = cascade.with_gene(gene);
        }

        cascade
    }

    pub fn with_gene(mut self, gene: TaskExpressionGene) -> Self {
        self.genes.push(gene);
        self
    }

    pub fn with_write_flags(mut self, read_only: bool, write_allowed: bool, applied: bool) -> Self {
        self.read_only = read_only;
        self.write_allowed = write_allowed;
        self.applied = applied;
        self
    }

    pub fn counters(&self) -> TaskGeneTraceCounters {
        let mut counters = TaskGeneTraceCounters::from_genes(&self.genes);
        counters.task_gene_rejected_unbounded_spawn = usize::from(self.exceeds_spawn_limits());
        counters.task_gene_conflict_quarantine = usize::from(self.has_lane_conflict());
        counters
    }

    pub fn review(&self) -> TaskGeneAdmissionReview {
        let mut counters = self.counters();
        let mut reason_codes = Vec::new();
        let mut decision = TaskGeneAdmissionDecision::AcceptPreview;

        if !self.read_only
            || self.write_allowed
            || self.applied
            || self.genes.iter().any(|gene| !gene.is_preview_only())
        {
            decision = TaskGeneAdmissionDecision::RequireMainThreadApproval;
            push_unique(&mut reason_codes, "task_gene_single_writer_violation");
        } else if self
            .genes
            .iter()
            .any(|gene| gene.retired_version_block.is_some())
        {
            decision = TaskGeneAdmissionDecision::QuarantinePollutedPayload;
            push_unique(&mut reason_codes, "task_gene_retired_version_block");
        } else if self.genes.iter().any(|gene| gene.blocked_payload) {
            decision = TaskGeneAdmissionDecision::QuarantinePollutedPayload;
            push_unique(&mut reason_codes, "task_gene_polluted_payload");
        } else if self.exceeds_spawn_limits() {
            decision = TaskGeneAdmissionDecision::RejectUnboundedSpawn;
            push_unique(&mut reason_codes, "task_gene_unbounded_spawn");
        } else if counters.task_gene_budget_stop > 0 {
            decision = TaskGeneAdmissionDecision::RejectBudgetExhaustion;
            push_unique(&mut reason_codes, "task_gene_budget_exhausted");
        } else if self.has_lane_conflict() {
            decision = TaskGeneAdmissionDecision::QuarantineConflict;
            counters.task_gene_conflict_quarantine = 1;
            push_unique(&mut reason_codes, "task_gene_conflicting_child_lanes");
        } else if self
            .genes
            .iter()
            .any(|gene| !gene.validation_passed || gene.source_evidence_ids.is_empty())
        {
            decision = TaskGeneAdmissionDecision::HoldForEvidence;
            push_unique(
                &mut reason_codes,
                "task_gene_validation_evidence_missing_or_failed",
            );
        }

        if decision == TaskGeneAdmissionDecision::AcceptPreview {
            counters.task_gene_preview_admission = 1;
        }

        TaskGeneAdmissionReview {
            decision,
            reason_codes,
            counters,
            evidence_digest: self.evidence_digest(decision),
            read_only: true,
            write_allowed: false,
            applied: false,
            operator_approval_required: true,
        }
    }

    fn exceeds_spawn_limits(&self) -> bool {
        if self
            .genes
            .iter()
            .any(|gene| gene.child_depth > self.max_depth)
        {
            return true;
        }

        let mut children_by_parent = BTreeMap::<String, usize>::new();
        for gene in self
            .genes
            .iter()
            .filter(|gene| gene.parent_gene_id.is_some())
        {
            let parent = gene.parent_gene_id.clone().unwrap_or_default();
            *children_by_parent.entry(parent).or_default() += 1;
        }
        children_by_parent
            .values()
            .any(|children| *children > self.max_children_per_gene)
    }

    fn has_lane_conflict(&self) -> bool {
        let mut seen = BTreeSet::new();
        for gene in self
            .genes
            .iter()
            .filter(|gene| gene.parent_gene_id.is_some())
        {
            let key = (
                gene.parent_gene_id.clone().unwrap_or_default(),
                gene.lane.clone(),
                gene.role,
            );
            if !seen.insert(key) {
                return true;
            }
        }
        false
    }

    fn evidence_digest(&self, decision: TaskGeneAdmissionDecision) -> String {
        let gene_digests = self
            .genes
            .iter()
            .map(|gene| gene.objective_digest.as_str())
            .collect::<Vec<_>>()
            .join("|");
        stable_redaction_digest([
            TASK_EXPRESSION_GENE_SCHEMA_VERSION,
            self.root_goal_id.as_str(),
            decision.as_str(),
            self.serial_or_parallel_mode.as_str(),
            gene_digests.as_str(),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGeneAdmissionReview {
    pub decision: TaskGeneAdmissionDecision,
    pub reason_codes: Vec<String>,
    pub counters: TaskGeneTraceCounters,
    pub evidence_digest: String,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub operator_approval_required: bool,
}

impl TaskGeneAdmissionReview {
    pub fn summary_line(&self) -> String {
        format!(
            "task_gene_admission decision={} reasons={} evidence_digest={} {} read_only={} write_allowed={} applied={} operator_approval_required={}",
            self.decision.as_str(),
            self.reason_codes.join("|"),
            self.evidence_digest,
            self.counters.summary_fields(),
            self.read_only,
            self.write_allowed,
            self.applied,
            self.operator_approval_required
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubAgentExpressionTrace {
    pub schema_version: &'static str,
    pub root_goal_id: String,
    pub scheduler_phase_id: String,
    pub agent_team_run_id: String,
    pub linked_surfaces: Vec<TaskGeneSurfaceLink>,
    pub counters: TaskGeneTraceCounters,
    pub admission_decision: TaskGeneAdmissionDecision,
    pub evidence_digest: String,
    pub preview_only_evolution_signal: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SubAgentExpressionTrace {
    pub fn from_agent_team_plan(
        cascade: &TaskGeneCascade,
        plan: &AgentTeamPlan,
        scheduler_phase_id: &str,
    ) -> Self {
        let review = cascade.review();
        let preview_only_evolution_signal = review.decision
            == TaskGeneAdmissionDecision::AcceptPreview
            && plan.evolution_signal_count() > 0
            && plan.collision_free();
        let mut counters = review.counters.clone();
        counters.task_gene_preview_admission = usize::from(preview_only_evolution_signal);
        let evidence_digest = stable_redaction_digest([
            "sub-agent-expression-trace",
            cascade.root_goal_id.as_str(),
            plan.run_id.as_str(),
            scheduler_phase_id,
            review.evidence_digest.as_str(),
        ]);

        Self {
            schema_version: TASK_EXPRESSION_GENE_SCHEMA_VERSION,
            root_goal_id: cascade.root_goal_id.clone(),
            scheduler_phase_id: safe_ref(scheduler_phase_id.to_owned()),
            agent_team_run_id: safe_ref(plan.run_id.clone()),
            linked_surfaces: TaskGeneSurfaceLink::required(),
            counters,
            admission_decision: review.decision,
            evidence_digest,
            preview_only_evolution_signal,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn summary_line(&self) -> String {
        let surfaces = self
            .linked_surfaces
            .iter()
            .map(|surface| surface.as_str())
            .collect::<Vec<_>>()
            .join("|");
        format!(
            "sub_agent_expression_trace schema={} root_goal={} scheduler_phase={} agent_team_run={} surfaces={} decision={} evidence_digest={} preview_only_evolution_signal={} {} read_only={} write_allowed={} applied={}",
            self.schema_version,
            self.root_goal_id,
            self.scheduler_phase_id,
            self.agent_team_run_id,
            surfaces,
            self.admission_decision.as_str(),
            self.evidence_digest,
            self.preview_only_evolution_signal,
            self.counters.summary_fields(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

fn germ_layer_for_role(role: AgentRole) -> TaskGeneGermLayer {
    match role {
        AgentRole::Planner | AgentRole::Researcher => TaskGeneGermLayer::Ectoderm,
        AgentRole::Coder | AgentRole::Tester => TaskGeneGermLayer::Mesoderm,
        AgentRole::Reviewer | AgentRole::MemoryCurator | AgentRole::Aggregator => {
            TaskGeneGermLayer::Endoderm
        }
    }
}

fn default_capabilities(role: AgentRole) -> Vec<String> {
    match role {
        AgentRole::Planner => vec!["decompose".to_owned(), "budget_gate".to_owned()],
        AgentRole::Researcher => vec!["context_recall".to_owned(), "safety_preflight".to_owned()],
        AgentRole::Coder => vec!["rust".to_owned(), "local_tools".to_owned()],
        AgentRole::Reviewer => vec!["risk_review".to_owned(), "quarantine".to_owned()],
        AgentRole::Tester => vec!["tests".to_owned(), "trace_gate".to_owned()],
        AgentRole::MemoryCurator => vec!["digest_memory".to_owned(), "rollback_advice".to_owned()],
        AgentRole::Aggregator => vec!["blackboard_merge".to_owned(), "dedupe".to_owned()],
    }
}

fn safe_ref(value: String) -> String {
    value
        .trim()
        .chars()
        .take(96)
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
}

fn safe_text(label: &str, value: &str) -> String {
    if contains_private_or_executable_marker(value)
        || genome_expression_payload_blocked(label, value)
    {
        stable_redaction_digest([label, value])
    } else {
        safe_ref(value.to_owned())
    }
}

fn genome_expression_payload_blocked(event_id: &str, payload: &str) -> bool {
    !gate_development_evidence_payload_surface(
        event_id,
        "reasoning_genome",
        payload,
        DevelopmentEvidenceUseSurface::GenomeExpression,
    )
    .allowed
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_team::{
        AgentEvolutionSignal, AgentIsolationPolicy, AgentNode, AgentTeamAggregation, AgentTeamPlan,
    };

    #[test]
    fn cascade_accepts_serial_parent_with_child_genes() {
        let cascade = base_cascade(TaskGeneCascadeMode::Serial)
            .with_gene(parent_gene())
            .with_gene(child_gene(
                "child-coder",
                AgentRole::Coder,
                "implementation",
            ))
            .with_gene(child_gene("child-tester", AgentRole::Tester, "validation"));

        let review = cascade.review();

        assert_eq!(review.decision, TaskGeneAdmissionDecision::AcceptPreview);
        assert_eq!(review.counters.task_gene_spawned, 3);
        assert_eq!(review.counters.task_gene_child_count, 2);
        assert_eq!(review.counters.task_gene_preview_admission, 1);
        assert!(review.summary_line().contains("task_gene_spawned=3"));
        assert!(
            review
                .summary_line()
                .contains("evidence_digest=redaction-digest:")
        );
    }

    #[test]
    fn cascade_accepts_nested_child_within_depth_limit() {
        let nested = child_gene("child-reviewer", AgentRole::Reviewer, "review")
            .with_parent("child-coder", 2);
        let cascade = base_cascade(TaskGeneCascadeMode::Serial)
            .with_gene(parent_gene())
            .with_gene(child_gene(
                "child-coder",
                AgentRole::Coder,
                "implementation",
            ))
            .with_gene(nested);

        let review = cascade.review();

        assert_eq!(review.decision, TaskGeneAdmissionDecision::AcceptPreview);
        assert_eq!(review.counters.task_gene_max_depth, 2);
        assert_eq!(review.counters.endoderm_count, 1);
    }

    #[test]
    fn cascade_rejects_spawn_past_depth_limit() {
        let too_deep = child_gene("child-reviewer", AgentRole::Reviewer, "review")
            .with_parent("child-coder", 3);
        let cascade = base_cascade(TaskGeneCascadeMode::Serial)
            .with_gene(parent_gene())
            .with_gene(child_gene(
                "child-coder",
                AgentRole::Coder,
                "implementation",
            ))
            .with_gene(too_deep);

        let review = cascade.review();

        assert_eq!(
            review.decision,
            TaskGeneAdmissionDecision::RejectUnboundedSpawn
        );
        assert_eq!(review.counters.task_gene_rejected_unbounded_spawn, 1);
        assert!(
            review
                .reason_codes
                .contains(&"task_gene_unbounded_spawn".to_owned())
        );
    }

    #[test]
    fn cascade_rejects_child_budget_exhaustion() {
        let cascade = base_cascade(TaskGeneCascadeMode::Serial)
            .with_gene(parent_gene())
            .with_gene(
                child_gene("child-coder", AgentRole::Coder, "implementation").with_budget(16, 24),
            );

        let review = cascade.review();

        assert_eq!(
            review.decision,
            TaskGeneAdmissionDecision::RejectBudgetExhaustion
        );
        assert_eq!(review.counters.task_gene_budget_stop, 1);
    }

    #[test]
    fn cascade_quarantines_polluted_handoff_without_echoing_payload() {
        let polluted = TaskExpressionGene::new(
            "child-researcher",
            "goal-242",
            TaskProfile::Coding,
            AgentRole::Researcher,
            "context",
            "prompt: private user chat with api_key=secret",
            "private chat should not be copied",
        )
        .with_parent("parent", 1)
        .with_validation("privacy_gate", false);
        let cascade = base_cascade(TaskGeneCascadeMode::Serial)
            .with_gene(parent_gene())
            .with_gene(polluted);

        let review = cascade.review();
        let line = review.summary_line();

        assert_eq!(
            review.decision,
            TaskGeneAdmissionDecision::QuarantinePollutedPayload
        );
        assert!(!line.contains("api_key"));
        assert!(!line.contains("private user chat"));
        assert!(
            !crate::privacy_redaction::privacy_redaction_reason_codes(&line)
                .contains(&"secret_or_credential".to_owned())
        );
    }

    #[test]
    fn cascade_blocks_retired_version_block_before_preview_admission() {
        let retired = child_gene("child-coder", AgentRole::Coder, "implementation")
            .with_retired_version_block("0.305.41-issue-305-active-request-preview-gate");
        let cascade = base_cascade(TaskGeneCascadeMode::Serial)
            .with_gene(parent_gene())
            .with_gene(retired);

        let review = cascade.review();

        assert_eq!(
            review.decision,
            TaskGeneAdmissionDecision::QuarantinePollutedPayload
        );
        assert_eq!(review.counters.task_gene_preview_admission, 0);
        assert!(
            review
                .reason_codes
                .contains(&"task_gene_retired_version_block".to_owned())
        );
    }

    #[test]
    fn cascade_quarantines_genome_expression_pollution_marker_before_preview() {
        let polluted = TaskExpressionGene::new(
            "child-coder",
            "goal-242",
            TaskProfile::Coding,
            AgentRole::Coder,
            "implementation",
            "development_evidence_contamination must not enter genome expression",
            "reasoning_genome_hygiene_violation must not stay raw",
        )
        .with_parent("parent", 1)
        .with_validation("focused_validation", true)
        .with_evidence(vec!["evidence:child-coder".to_owned()]);
        assert!(polluted.blocked_payload);
        assert!(polluted.spawn_reason.starts_with("redaction-digest:"));
        assert!(
            !polluted
                .spawn_reason
                .contains("reasoning_genome_hygiene_violation")
        );
        let cascade = base_cascade(TaskGeneCascadeMode::Serial)
            .with_gene(parent_gene())
            .with_gene(polluted);

        let review = cascade.review();
        let line = review.summary_line();

        assert_eq!(
            review.decision,
            TaskGeneAdmissionDecision::QuarantinePollutedPayload
        );
        assert_eq!(review.counters.task_gene_preview_admission, 0);
        assert!(
            review
                .reason_codes
                .contains(&"task_gene_polluted_payload".to_owned())
        );
        assert!(!line.contains("development_evidence_contamination"));
        assert!(!line.contains("reasoning_genome_hygiene_violation"));
    }

    #[test]
    fn cascade_quarantines_conflicting_child_lanes() {
        let first = child_gene("child-coder-a", AgentRole::Coder, "implementation");
        let second = child_gene("child-coder-b", AgentRole::Coder, "implementation");
        let cascade = base_cascade(TaskGeneCascadeMode::Parallel)
            .with_gene(parent_gene())
            .with_gene(first)
            .with_gene(second);

        let review = cascade.review();

        assert_eq!(
            review.decision,
            TaskGeneAdmissionDecision::QuarantineConflict
        );
        assert_eq!(review.counters.task_gene_conflict_quarantine, 1);
    }

    #[test]
    fn cascade_holds_validation_failure_for_more_evidence() {
        let held = child_gene("child-tester", AgentRole::Tester, "validation")
            .with_validation("focused_tests", false);
        let cascade = base_cascade(TaskGeneCascadeMode::Serial)
            .with_gene(parent_gene())
            .with_gene(held);

        let review = cascade.review();

        assert_eq!(review.decision, TaskGeneAdmissionDecision::HoldForEvidence);
        assert!(
            review
                .reason_codes
                .contains(&"task_gene_validation_evidence_missing_or_failed".to_owned())
        );
    }

    #[test]
    fn agent_team_plan_aggregates_into_preview_only_expression_trace() {
        let plan = agent_team_fixture();
        let cascade = TaskGeneCascade::from_agent_team_plan("goal-242", TaskProfile::Coding, &plan);
        let trace = SubAgentExpressionTrace::from_agent_team_plan(
            &cascade,
            &plan,
            "thinking_phase:planning",
        );
        let line = trace.summary_line();

        assert_eq!(
            trace.admission_decision,
            TaskGeneAdmissionDecision::AcceptPreview
        );
        assert!(trace.preview_only_evolution_signal);
        assert!(trace.read_only && !trace.write_allowed && !trace.applied);
        assert_eq!(trace.linked_surfaces, TaskGeneSurfaceLink::required());
        assert!(line.contains("agent_team_plan"));
        assert!(line.contains("thinking_scheduler_phase"));
        assert!(line.contains("unified_writer_gate"));
        assert!(line.contains("task_gene_preview_admission=1"));
        assert!(!line.contains("compile digest-only task"));
    }

    #[test]
    fn cascade_requires_main_thread_approval_for_write_flags() {
        let cascade = base_cascade(TaskGeneCascadeMode::Serial)
            .with_write_flags(true, true, false)
            .with_gene(parent_gene())
            .with_gene(child_gene(
                "child-coder",
                AgentRole::Coder,
                "implementation",
            ));

        let review = cascade.review();

        assert_eq!(
            review.decision,
            TaskGeneAdmissionDecision::RequireMainThreadApproval
        );
        assert!(review.operator_approval_required);
        assert!(!review.write_allowed);
    }

    fn base_cascade(mode: TaskGeneCascadeMode) -> TaskGeneCascade {
        TaskGeneCascade::new("goal-242", 2, 2, mode)
    }

    fn parent_gene() -> TaskExpressionGene {
        TaskExpressionGene::new(
            "parent",
            "goal-242",
            TaskProfile::Coding,
            AgentRole::Planner,
            "control",
            "implement bounded sub-agent cascade",
            "root planning phase",
        )
        .with_validation("planning_gate", true)
    }

    fn child_gene(id: &str, role: AgentRole, lane: &str) -> TaskExpressionGene {
        TaskExpressionGene::new(
            id,
            "goal-242",
            TaskProfile::Coding,
            role,
            lane,
            format!("child objective for {lane}"),
            "bounded parent spawn",
        )
        .with_parent("parent", 1)
        .with_validation("focused_validation", true)
        .with_evidence(vec![format!("evidence:{id}")])
    }

    fn agent_team_fixture() -> AgentTeamPlan {
        let run_id = "agent-team-fixture".to_owned();
        AgentTeamPlan {
            enabled: true,
            run_id: run_id.clone(),
            main_thread_goal: "compile digest-only task through child lanes".to_owned(),
            isolation: AgentIsolationPolicy::default(),
            aggregation: AgentTeamAggregation {
                lane_count: 2,
                max_parallel_lanes: 2,
                budget_scope: "parallel_read_only_lanes_under_main_thread".to_owned(),
                main_thread_writer: "main_thread".to_owned(),
                ..AgentTeamAggregation::default()
            },
            agents: vec![
                AgentNode {
                    id: format!("{run_id}-planner"),
                    role: AgentRole::Planner,
                    objective: "decompose".to_owned(),
                    lane: "control".to_owned(),
                    dependencies: Vec::new(),
                    writes_allowed: false,
                },
                AgentNode {
                    id: format!("{run_id}-coder"),
                    role: AgentRole::Coder,
                    objective: "prepare patch".to_owned(),
                    lane: "implementation".to_owned(),
                    dependencies: vec!["planner".to_owned()],
                    writes_allowed: false,
                },
            ],
            messages: Vec::new(),
            conflicts: Vec::new(),
            evolution_signals: vec![AgentEvolutionSignal {
                target: "task_gene_cascade".to_owned(),
                action: "accept_preview".to_owned(),
                reason: "validated child lanes can inform future scheduler phases".to_owned(),
                score: 0.82,
            }],
            notes: Vec::new(),
        }
    }
}
