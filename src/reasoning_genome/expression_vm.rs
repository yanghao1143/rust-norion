use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::stable_redaction_digest;

use super::model::{
    GenomeExpression, GenomeOpcode, PreReasoningGenomeIsa, ReasoningFrame, ReasoningFrameAction,
    ReasoningFrameBudget, ReasoningFrameCapability, ReasoningFrameContextPolicy,
    ReasoningFrameEnvironmentMatch, ReasoningFrameEnvironmentSignal,
    ReasoningFrameEvidenceRequirement, ReasoningFrameGate, ReasoningFrameMemoryPolicy,
    ReasoningFrameMemoryTier, ReasoningFrameMutationPreview, ReasoningFrameObservation,
    ReasoningFrameRiskLimit, ReasoningFrameRoutingBias, ReasoningFrameSignalKind,
    ReasoningFrameValidationRequirement,
};
use super::task_expression::TaskExpressionGene;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenomeExpressionEnvironment {
    pub stimulus_digest: String,
    pub task_state_digest: String,
    pub memory_state_digest: String,
    pub runtime_health_digest: String,
}

impl GenomeExpressionEnvironment {
    pub fn preview(stimulus: &str) -> Self {
        Self {
            stimulus_digest: digest_or_hash("stimulus", stimulus),
            task_state_digest: stable_redaction_digest(["expression-vm", "task-state", stimulus]),
            memory_state_digest: stable_redaction_digest([
                "expression-vm",
                "memory-state",
                stimulus,
            ]),
            runtime_health_digest: stable_redaction_digest([
                "expression-vm",
                "runtime-health",
                stimulus,
            ]),
        }
    }

    pub fn new(
        stimulus_digest: impl AsRef<str>,
        task_state_digest: impl AsRef<str>,
        memory_state_digest: impl AsRef<str>,
        runtime_health_digest: impl AsRef<str>,
    ) -> Self {
        Self {
            stimulus_digest: digest_or_hash("stimulus", stimulus_digest.as_ref()),
            task_state_digest: digest_or_hash("task-state", task_state_digest.as_ref()),
            memory_state_digest: digest_or_hash("memory-state", memory_state_digest.as_ref()),
            runtime_health_digest: digest_or_hash("runtime-health", runtime_health_digest.as_ref()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenomeExpressionBudget {
    pub compute_budget: String,
    pub max_tokens: usize,
    pub threshold_milli: u16,
    pub route_fanout: usize,
    pub reflection_passes: usize,
    pub validation_runs: usize,
    pub memory_records: usize,
}

impl GenomeExpressionBudget {
    pub fn preview(profile: TaskProfile) -> Self {
        let (compute_budget, max_tokens, route_fanout) = match profile {
            TaskProfile::Coding => ("normal", 1024, 2),
            TaskProfile::Writing => ("normal", 1024, 2),
            TaskProfile::LongDocument => ("high", 2048, 3),
            TaskProfile::General => ("low", 512, 1),
        };
        Self {
            compute_budget: compute_budget.to_owned(),
            max_tokens,
            threshold_milli: 520,
            route_fanout,
            reflection_passes: 1,
            validation_runs: 1,
            memory_records: 4,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GenomeExpressionVmInput<'a> {
    pub expression: &'a GenomeExpression,
    pub task_gene: Option<&'a TaskExpressionGene>,
    pub environment: &'a GenomeExpressionEnvironment,
    pub budget: &'a GenomeExpressionBudget,
}

impl<'a> GenomeExpressionVmInput<'a> {
    pub fn new(
        expression: &'a GenomeExpression,
        environment: &'a GenomeExpressionEnvironment,
        budget: &'a GenomeExpressionBudget,
    ) -> Self {
        Self {
            expression,
            task_gene: None,
            environment,
            budget,
        }
    }

    pub fn with_task_gene(mut self, task_gene: Option<&'a TaskExpressionGene>) -> Self {
        self.task_gene = task_gene;
        self
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GenomeExpressionVm;

impl GenomeExpressionVm {
    pub fn execute(&self, input: GenomeExpressionVmInput<'_>) -> ReasoningFrame {
        let isa = PreReasoningGenomeIsa::preview();
        let mut executed_opcodes = Vec::with_capacity(isa.opcodes.len());
        let mut environment_signals = Vec::new();
        let mut environment_matches = Vec::new();
        let mut selected_gene_ids = Vec::new();
        let mut allowed_observations = Vec::new();
        let mut action_vocab = Vec::new();
        let mut routing_bias = default_routing_bias(input);
        let mut memory_policy = default_memory_policy(input);
        let mut budget = frame_budget(input.budget);
        let mut context_policy = default_context_policy();
        let mut suppressed_capabilities = Vec::new();
        let mut evidence_requirements = Vec::new();
        let mut validation_requirements = Vec::new();
        let mut gates = Vec::new();
        let mut mutation_preview = Vec::new();

        for opcode in &isa.opcodes {
            match opcode {
                GenomeOpcode::BindStimulus => {
                    environment_signals = bind_environment(input.environment);
                }
                GenomeOpcode::LoadGene => {
                    selected_gene_ids.clone_from(&input.expression.active_gene_ids);
                }
                GenomeOpcode::MatchEnv => {
                    environment_matches.push(ReasoningFrameEnvironmentMatch {
                        profile: input.expression.profile,
                        matched_gene_count: selected_gene_ids.len(),
                        matched_signal_count: environment_signals.len(),
                        task_gene_compatible: input
                            .task_gene
                            .is_none_or(|gene| gene.task_profile == input.expression.profile),
                    });
                }
                GenomeOpcode::ExpressTrait => {
                    routing_bias = default_routing_bias(input);
                }
                GenomeOpcode::SetBudget => {
                    budget = frame_budget(input.budget);
                }
                GenomeOpcode::SelectMemory => {
                    memory_policy = default_memory_policy(input);
                }
                GenomeOpcode::PackContext => {
                    context_policy.selected_gene_count = selected_gene_ids.len();
                    context_policy.selected_memory_count =
                        input.budget.memory_records.min(memory_policy.max_records);
                }
                GenomeOpcode::FocusSignal => {
                    allowed_observations = vec![
                        ReasoningFrameObservation::RepoIssueTerminalRuntimeState,
                        ReasoningFrameObservation::TaskConstraints,
                        ReasoningFrameObservation::MemoryState,
                        ReasoningFrameObservation::RuntimeHealth,
                    ];
                    context_policy.focus_signals = focused_signals(input.expression.profile);
                }
                GenomeOpcode::MaskSignal => {
                    context_policy.masked_signals = vec![
                        ReasoningFrameSignalKind::RawPayload,
                        ReasoningFrameSignalKind::UntrustedExternalPayload,
                    ];
                }
                GenomeOpcode::DeclareActionVocab => {
                    action_vocab = ReasoningFrameAction::bounded_vocab();
                }
                GenomeOpcode::SuppressCapability => {
                    suppressed_capabilities =
                        ReasoningFrameCapability::forbidden_preview_capabilities().to_vec();
                }
                GenomeOpcode::RequireEvidence => {
                    evidence_requirements = vec![
                        ReasoningFrameEvidenceRequirement::DigestOnlyFrameId,
                        ReasoningFrameEvidenceRequirement::NoRawPayload,
                    ];
                    validation_requirements = vec![
                        ReasoningFrameValidationRequirement::PreviewOnly,
                        ReasoningFrameValidationRequirement::NoWrite,
                        ReasoningFrameValidationRequirement::NoApply,
                        ReasoningFrameValidationRequirement::SuppressForbiddenCapabilities,
                    ];
                }
                GenomeOpcode::DeclareGate => {
                    gates = vec![
                        ReasoningFrameGate::ToolAction,
                        ReasoningFrameGate::Network,
                        ReasoningFrameGate::Writer,
                        ReasoningFrameGate::MemoryAdmission,
                        ReasoningFrameGate::GenomeWriter,
                        ReasoningFrameGate::Process,
                        ReasoningFrameGate::Repository,
                        ReasoningFrameGate::Rollback,
                    ];
                }
                GenomeOpcode::PreviewMutation => {
                    mutation_preview = input
                        .expression
                        .mutation_plans
                        .iter()
                        .map(|plan| ReasoningFrameMutationPreview {
                            plan_id_digest: stable_redaction_digest([
                                "expression-vm-plan",
                                plan.id.as_str(),
                            ]),
                            intent: plan.intent,
                            target_gene_digest: stable_redaction_digest([
                                "expression-vm-target",
                                plan.target_gene_id.as_str(),
                            ]),
                            rollback_anchor_digest: stable_redaction_digest([
                                "expression-vm-rollback",
                                plan.rollback_anchor_id.as_str(),
                            ]),
                        })
                        .collect();
                }
                GenomeOpcode::EmitFrame => {}
            }
            executed_opcodes.push(*opcode);
        }

        let frame_id = stable_redaction_digest([
            "genome-expression-vm-v1",
            input.expression.genome_id.as_str(),
            input.environment.stimulus_digest.as_str(),
            routing_bias.compute_budget.as_str(),
            memory_policy.scope.as_str(),
            input
                .task_gene
                .map(|gene| gene.objective_digest.as_str())
                .unwrap_or("task-gene:none"),
        ]);

        ReasoningFrame {
            frame_id,
            genome_isa: isa,
            environment_signals_present: !environment_signals.is_empty(),
            environment_signals,
            environment_matches,
            allowed_observations,
            selected_gene_ids,
            executed_opcodes,
            action_vocab,
            suppressed_capabilities,
            granted_capabilities: Vec::new(),
            routing_bias,
            memory_policy,
            budget,
            context_policy,
            gates,
            mutation_preview,
            risk_limits: vec![
                ReasoningFrameRiskLimit::PreviewOnly,
                ReasoningFrameRiskLimit::DigestOnly,
            ],
            evidence_requirements,
            validation_requirements,
            efficiency_snapshot: None,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }
}

impl ReasoningFrame {
    pub fn issue375_preview(body_state_id: &str) -> Self {
        let expression = GenomeExpression::empty(TaskProfile::General);
        let environment = GenomeExpressionEnvironment::preview(body_state_id);
        let budget = GenomeExpressionBudget::preview(TaskProfile::General);
        GenomeExpressionVm.execute(GenomeExpressionVmInput::new(
            &expression,
            &environment,
            &budget,
        ))
    }
}

fn bind_environment(
    environment: &GenomeExpressionEnvironment,
) -> Vec<ReasoningFrameEnvironmentSignal> {
    vec![
        ReasoningFrameEnvironmentSignal {
            kind: ReasoningFrameSignalKind::UserConstraint,
            digest: environment.stimulus_digest.clone(),
        },
        ReasoningFrameEnvironmentSignal {
            kind: ReasoningFrameSignalKind::TaskState,
            digest: environment.task_state_digest.clone(),
        },
        ReasoningFrameEnvironmentSignal {
            kind: ReasoningFrameSignalKind::MemoryState,
            digest: environment.memory_state_digest.clone(),
        },
        ReasoningFrameEnvironmentSignal {
            kind: ReasoningFrameSignalKind::RuntimeHealth,
            digest: environment.runtime_health_digest.clone(),
        },
    ]
}

fn default_routing_bias(input: GenomeExpressionVmInput<'_>) -> ReasoningFrameRoutingBias {
    ReasoningFrameRoutingBias {
        profile: input.expression.profile,
        compute_budget: input.budget.compute_budget.clone(),
        threshold_milli: input.budget.threshold_milli.min(1000),
        max_fanout: input.budget.route_fanout.max(1),
    }
}

fn default_memory_policy(input: GenomeExpressionVmInput<'_>) -> ReasoningFrameMemoryPolicy {
    let tiers = match input.expression.profile {
        TaskProfile::Coding => vec![
            ReasoningFrameMemoryTier::Semantic,
            ReasoningFrameMemoryTier::RuntimeKv,
            ReasoningFrameMemoryTier::Experience,
            ReasoningFrameMemoryTier::ToolReliability,
        ],
        TaskProfile::LongDocument => vec![
            ReasoningFrameMemoryTier::Semantic,
            ReasoningFrameMemoryTier::Gist,
            ReasoningFrameMemoryTier::RuntimeKv,
            ReasoningFrameMemoryTier::Experience,
        ],
        TaskProfile::Writing => vec![
            ReasoningFrameMemoryTier::Semantic,
            ReasoningFrameMemoryTier::Gist,
            ReasoningFrameMemoryTier::Experience,
        ],
        TaskProfile::General => vec![
            ReasoningFrameMemoryTier::Semantic,
            ReasoningFrameMemoryTier::Gist,
            ReasoningFrameMemoryTier::Experience,
        ],
    };
    ReasoningFrameMemoryPolicy {
        scope: input
            .task_gene
            .map(|gene| gene.memory_scope.clone())
            .unwrap_or_else(|| "digest_only_profile_memory".to_owned()),
        tiers,
        max_records: input.budget.memory_records,
        read_only: true,
    }
}

fn frame_budget(budget: &GenomeExpressionBudget) -> ReasoningFrameBudget {
    ReasoningFrameBudget {
        max_tokens: budget.max_tokens.max(1),
        route_fanout: budget.route_fanout.max(1),
        reflection_passes: budget.reflection_passes,
        validation_runs: budget.validation_runs,
    }
}

fn default_context_policy() -> ReasoningFrameContextPolicy {
    ReasoningFrameContextPolicy {
        selected_gene_count: 0,
        selected_memory_count: 0,
        focus_signals: Vec::new(),
        masked_signals: Vec::new(),
        digest_only: true,
    }
}

fn focused_signals(profile: TaskProfile) -> Vec<ReasoningFrameSignalKind> {
    match profile {
        TaskProfile::Coding => vec![
            ReasoningFrameSignalKind::TaskState,
            ReasoningFrameSignalKind::RuntimeHealth,
            ReasoningFrameSignalKind::MemoryState,
            ReasoningFrameSignalKind::GenomeState,
        ],
        TaskProfile::LongDocument => vec![
            ReasoningFrameSignalKind::MemoryState,
            ReasoningFrameSignalKind::TaskState,
            ReasoningFrameSignalKind::RuntimeHealth,
            ReasoningFrameSignalKind::GenomeState,
        ],
        TaskProfile::Writing => vec![
            ReasoningFrameSignalKind::TaskState,
            ReasoningFrameSignalKind::MemoryState,
            ReasoningFrameSignalKind::GenomeState,
        ],
        TaskProfile::General => vec![
            ReasoningFrameSignalKind::UserConstraint,
            ReasoningFrameSignalKind::TaskState,
            ReasoningFrameSignalKind::GenomeState,
        ],
    }
}

fn digest_or_hash(label: &str, value: &str) -> String {
    if value.starts_with("redaction-digest:") {
        value.to_owned()
    } else {
        stable_redaction_digest(["expression-vm", label, value])
    }
}
