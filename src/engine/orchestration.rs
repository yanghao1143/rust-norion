use super::types::InferenceOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoironOrchestrationStageStatus {
    Completed,
    Failed,
    PreviewOnly,
    Gated,
    RolledBack,
}

impl NoironOrchestrationStageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::PreviewOnly => "preview_only",
            Self::Gated => "gated",
            Self::RolledBack => "rolled_back",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoironOrchestrationStage {
    pub name: String,
    pub status: NoironOrchestrationStageStatus,
    pub evidence: Vec<String>,
    pub rollback_records: Vec<String>,
}

impl NoironOrchestrationStage {
    fn new(
        name: impl Into<String>,
        status: NoironOrchestrationStageStatus,
        evidence: Vec<String>,
        rollback_records: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status,
            evidence,
            rollback_records,
        }
    }

    pub fn is_failed(&self) -> bool {
        self.status == NoironOrchestrationStageStatus::Failed
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoironContextTrace {
    pub profile: String,
    pub prompt_tokens: usize,
    pub recursive_chunks: usize,
    pub recursive_merge_rounds: usize,
    pub recursive_execution_waves: usize,
    pub recursive_runtime_calls: usize,
    pub max_parallel_chunks: usize,
    pub memory_matches: usize,
    pub experience_matches: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoironRouteTrace {
    pub route_threshold: f32,
    pub route_attention_tokens: usize,
    pub route_fast_tokens: usize,
    pub adaptive_candidates: usize,
    pub adaptive_include: usize,
    pub adaptive_compress: usize,
    pub adaptive_defer: usize,
    pub adaptive_skip: usize,
    pub adaptive_input_tokens: usize,
    pub adaptive_retained_tokens: usize,
    pub adaptive_saved_tokens: usize,
    pub decision_count_matches: bool,
    pub token_accounting_matches: bool,
    pub anchors_retained: bool,
    pub selected_routes: Vec<String>,
    pub action_summaries: Vec<String>,
    pub score_summaries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoironKvTrace {
    pub used_memory_ids: Vec<u64>,
    pub used_memories: usize,
    pub gist_records: usize,
    pub stored_memory: bool,
    pub stored_gist_memories: usize,
    pub exported_runtime_kv_blocks: usize,
    pub stored_runtime_kv_memories: usize,
    pub memory_admission_candidates: usize,
    pub memory_admission_review_packets: usize,
    pub memory_ledger_records: usize,
    pub memory_ledger_authorized: usize,
    pub memory_ledger_applied: usize,
    pub memory_admission_kinds: Vec<String>,
    pub memory_admission_decisions: Vec<String>,
    pub fusion_score_summaries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoironGenomeTrace {
    pub genome_id: String,
    pub stable_anchor_id: String,
    pub expression_gene_count: usize,
    pub active_genes: usize,
    pub aging_genes: usize,
    pub malignant_genes: usize,
    pub relabel_candidates: usize,
    pub regeneration_candidates: usize,
    pub genome_mutation_plans: usize,
    pub genome_lifecycle_records: usize,
    pub genome_lifecycle_actions: Vec<String>,
    pub splice_segments: usize,
    pub splice_retained: usize,
    pub splice_skipped: usize,
    pub splice_quarantined: usize,
    pub splice_repair_candidates: usize,
    pub splice_findings: usize,
    pub splice_mutation_plans: usize,
    pub splice_lifecycle_records: usize,
    pub splice_finding_kinds: Vec<String>,
    pub splice_dispositions: Vec<String>,
    pub splice_saved_tokens: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoironReflectionTrace {
    pub quality: f32,
    pub issue_count: usize,
    pub critical_issue_count: usize,
    pub contradiction_count: usize,
    pub revision_passes: usize,
    pub revision_action_count: usize,
    pub store_as_memory: bool,
    pub process_reward: f32,
    pub process_reward_action: String,
    pub runtime_error_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoironGateTrace {
    pub memory_admission_read_only_preview: bool,
    pub genome_expression_read_only_preview: bool,
    pub genome_splice_read_only_preview: bool,
    pub compute_budget_read_only: bool,
    pub durable_memory_ledger_records: usize,
    pub durable_memory_ledger_authorized: usize,
    pub durable_memory_ledger_applied: usize,
    pub unauthorized_durable_memory_writes: usize,
    pub unauthorized_genome_writes: usize,
    pub unauthorized_experiment_ledger_writes: usize,
    pub runtime_cache_writes: usize,
    pub evolution_ledger_live_runs: u64,
    pub evolution_ledger_drift_rollbacks: u64,
}

impl NoironGateTrace {
    pub fn all_writes_gated(&self) -> bool {
        self.unauthorized_durable_memory_writes == 0
            && self.unauthorized_genome_writes == 0
            && self.unauthorized_experiment_ledger_writes == 0
            && self.durable_memory_ledger_applied <= self.durable_memory_ledger_authorized
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoironOrchestrationTrace {
    pub schema_version: u8,
    pub stages: Vec<NoironOrchestrationStage>,
    pub context: NoironContextTrace,
    pub route: NoironRouteTrace,
    pub kv: NoironKvTrace,
    pub genome: NoironGenomeTrace,
    pub reflection: NoironReflectionTrace,
    pub gates: NoironGateTrace,
    pub rollback_records: Vec<String>,
}

impl NoironOrchestrationTrace {
    pub fn stage(&self, name: &str) -> Option<&NoironOrchestrationStage> {
        self.stages.iter().find(|stage| stage.name == name)
    }

    pub fn has_stage(&self, name: &str) -> bool {
        self.stage(name).is_some()
    }

    pub fn failed_stages(&self) -> Vec<&NoironOrchestrationStage> {
        self.stages
            .iter()
            .filter(|stage| stage.is_failed())
            .collect()
    }

    pub fn has_actionable_rollback_record(&self) -> bool {
        !self.rollback_records.is_empty()
            || self
                .stages
                .iter()
                .any(|stage| !stage.rollback_records.is_empty())
    }

    pub fn all_writes_gated(&self) -> bool {
        self.gates.all_writes_gated()
    }

    pub fn live_feedback_closed(&self) -> bool {
        self.stage("live_feedback_loop")
            .is_some_and(|stage| stage.status == NoironOrchestrationStageStatus::Completed)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "noiron_orchestration_trace_v{} stages={} failed={} memories={} runtime_kv_exported={} route_candidates={} genome_segments={} durable_ledger={}/{} applied={} writes_gated={} live_feedback_closed={}",
            self.schema_version,
            self.stages.len(),
            self.failed_stages().len(),
            self.kv.used_memories,
            self.kv.exported_runtime_kv_blocks,
            self.route.adaptive_candidates,
            self.genome.splice_segments,
            self.gates.durable_memory_ledger_authorized,
            self.gates.durable_memory_ledger_records,
            self.gates.durable_memory_ledger_applied,
            self.all_writes_gated(),
            self.live_feedback_closed()
        )
    }
}

impl InferenceOutcome {
    pub fn orchestration_trace(&self) -> NoironOrchestrationTrace {
        let context = NoironContextTrace {
            profile: profile_slug(self.task_hierarchy_plan.profile).to_owned(),
            prompt_tokens: self.recursive_schedule.prompt_tokens,
            recursive_chunks: self.recursive_schedule.chunk_count(),
            recursive_merge_rounds: self.recursive_schedule.merge_round_count(),
            recursive_execution_waves: self.recursive_schedule.execution_wave_count(),
            recursive_runtime_calls: self.recursive_runtime_calls,
            max_parallel_chunks: self.recursive_schedule.max_parallel_chunks,
            memory_matches: self.used_memories.len(),
            experience_matches: self.used_experiences.len(),
        };
        let route = NoironRouteTrace {
            route_threshold: self.route_budget.threshold,
            route_attention_tokens: self.route_budget.attention_tokens,
            route_fast_tokens: self.route_budget.fast_tokens,
            adaptive_candidates: self.adaptive_route_plan.candidates,
            adaptive_include: self.adaptive_route_plan.include,
            adaptive_compress: self.adaptive_route_plan.compress,
            adaptive_defer: self.adaptive_route_plan.defer,
            adaptive_skip: self.adaptive_route_plan.skip,
            adaptive_input_tokens: self.adaptive_route_plan.input_tokens,
            adaptive_retained_tokens: self.adaptive_route_plan.retained_tokens,
            adaptive_saved_tokens: self.adaptive_route_plan.saved_tokens,
            decision_count_matches: self.adaptive_route_plan.decision_count_matches(),
            token_accounting_matches: self.adaptive_route_plan.token_accounting_matches(),
            anchors_retained: self.adaptive_route_plan.anchors_retained(),
            selected_routes: self.adaptive_route_plan.selected_route_summaries(),
            action_summaries: self.adaptive_route_plan.action_summaries(),
            score_summaries: self.adaptive_route_plan.score_summaries(4),
        };
        let kv = NoironKvTrace {
            used_memory_ids: self.used_memories.iter().map(|memory| memory.id).collect(),
            used_memories: self.used_memories.len(),
            gist_records: self.gist_records.len(),
            stored_memory: self.stored_memory_id.is_some(),
            stored_gist_memories: self.stored_gist_memory_ids.len(),
            exported_runtime_kv_blocks: self.exported_runtime_kv_blocks,
            stored_runtime_kv_memories: self.stored_runtime_kv_memory_ids.len(),
            memory_admission_candidates: self.memory_admission.candidate_count(),
            memory_admission_review_packets: self.memory_admission.review_packet_count(),
            memory_ledger_records: self.memory_admission.ledger_record_count(),
            memory_ledger_authorized: self.memory_admission.ledger_authorized_count(),
            memory_ledger_applied: self.memory_admission.ledger_applied_count(),
            memory_admission_kinds: self.memory_admission.kind_summaries(),
            memory_admission_decisions: self.memory_admission.decision_summaries(),
            fusion_score_summaries: self.memory_admission.fusion_score_summaries(4),
        };
        let genome = NoironGenomeTrace {
            genome_id: self.reasoning_genome.genome_id.clone(),
            stable_anchor_id: self.reasoning_genome.stable_anchor_id.clone(),
            expression_gene_count: self.reasoning_genome.expression_gene_count,
            active_genes: self.reasoning_genome.active_gene_count(),
            aging_genes: self.reasoning_genome.aged_gene_count(),
            malignant_genes: self.reasoning_genome.malignant_gene_count(),
            relabel_candidates: self.reasoning_genome.relabel_candidate_count(),
            regeneration_candidates: self.reasoning_genome.regeneration_candidate_count(),
            genome_mutation_plans: self.reasoning_genome.scissors_proposal_count(),
            genome_lifecycle_records: self.reasoning_genome.lifecycle_record_count(),
            genome_lifecycle_actions: self.reasoning_genome.lifecycle_action_summaries(),
            splice_segments: self.reasoning_genome_splice.segments.len(),
            splice_retained: self.reasoning_genome_splice.retained_count(),
            splice_skipped: self.reasoning_genome_splice.skipped_count(),
            splice_quarantined: self.reasoning_genome_splice.quarantined_count(),
            splice_repair_candidates: self.reasoning_genome_splice.repair_candidate_count(),
            splice_findings: self.reasoning_genome_splice.findings.len(),
            splice_mutation_plans: self.reasoning_genome_splice.mutation_plans.len(),
            splice_lifecycle_records: self.reasoning_genome_splice.lifecycle_record_count(),
            splice_finding_kinds: self.reasoning_genome_splice.finding_kinds(),
            splice_dispositions: self.reasoning_genome_splice.disposition_summaries(),
            splice_saved_tokens: self.reasoning_genome_splice.estimated_saved_token_count(),
        };
        let runtime_error_notes = runtime_error_notes(self);
        let reflection = NoironReflectionTrace {
            quality: self.report.quality,
            issue_count: self.report.issues.len(),
            critical_issue_count: self.report.critical_issue_count(),
            contradiction_count: self.report.contradictions.len(),
            revision_passes: self.report.revision_passes,
            revision_action_count: self.report.revision_actions.len(),
            store_as_memory: self.report.store_as_memory,
            process_reward: self.process_reward.total,
            process_reward_action: self.process_reward.action.as_str().to_owned(),
            runtime_error_notes: runtime_error_notes.clone(),
        };
        let gates = NoironGateTrace {
            memory_admission_read_only_preview: self.memory_admission.is_read_only_preview(),
            genome_expression_read_only_preview: self.reasoning_genome.is_read_only_preview(),
            genome_splice_read_only_preview: self.reasoning_genome_splice.is_read_only_preview(),
            compute_budget_read_only: self.compute_budget_schedule.read_only
                && !self.compute_budget_schedule.write_allowed
                && !self.compute_budget_schedule.applied,
            durable_memory_ledger_records: self.memory_admission.ledger_record_count(),
            durable_memory_ledger_authorized: self.memory_admission.ledger_authorized_count(),
            durable_memory_ledger_applied: self.memory_admission.ledger_applied_count(),
            unauthorized_durable_memory_writes: unauthorized_memory_writes(self),
            unauthorized_genome_writes: unauthorized_genome_writes(self),
            unauthorized_experiment_ledger_writes: 0,
            runtime_cache_writes: usize::from(self.stored_memory_id.is_some())
                .saturating_add(self.stored_gist_memory_ids.len())
                .saturating_add(self.stored_runtime_kv_memory_ids.len()),
            evolution_ledger_live_runs: self.evolution_ledger.live_inference_runs,
            evolution_ledger_drift_rollbacks: self.evolution_ledger.drift_rollbacks,
        };
        let rollback_records = rollback_records(self, &runtime_error_notes);
        let stages = vec![
            context_stage(&context),
            memory_retrieval_stage(&context, &kv),
            routing_stage(&route, &self.compute_budget_schedule.summary_line()),
            model_adapter_stage(self, &runtime_error_notes),
            reflection_stage(&reflection),
            live_feedback_stage(self),
            genome_stage(&genome, &gates),
            memory_admission_stage(&kv, &gates),
            evolution_stage(self, &gates),
            retention_stage(self),
        ];

        NoironOrchestrationTrace {
            schema_version: 1,
            stages,
            context,
            route,
            kv,
            genome,
            reflection,
            gates,
            rollback_records,
        }
    }
}

fn context_stage(context: &NoironContextTrace) -> NoironOrchestrationStage {
    NoironOrchestrationStage::new(
        "context",
        NoironOrchestrationStageStatus::Completed,
        vec![
            format!("profile={}", context.profile),
            format!("prompt_tokens={}", context.prompt_tokens),
            format!("recursive_chunks={}", context.recursive_chunks),
            format!("recursive_merge_rounds={}", context.recursive_merge_rounds),
            format!("execution_waves={}", context.recursive_execution_waves),
            format!("max_parallel_chunks={}", context.max_parallel_chunks),
        ],
        Vec::new(),
    )
}

fn memory_retrieval_stage(
    context: &NoironContextTrace,
    kv: &NoironKvTrace,
) -> NoironOrchestrationStage {
    NoironOrchestrationStage::new(
        "memory_retrieval",
        NoironOrchestrationStageStatus::Completed,
        vec![
            format!("used_memories={}", context.memory_matches),
            format!("used_experiences={}", context.experience_matches),
            format!("gist_records={}", kv.gist_records),
            format!("runtime_kv_exported={}", kv.exported_runtime_kv_blocks),
        ],
        Vec::new(),
    )
}

fn routing_stage(route: &NoironRouteTrace, compute_summary: &str) -> NoironOrchestrationStage {
    let status =
        if route.decision_count_matches && route.token_accounting_matches && route.anchors_retained
        {
            NoironOrchestrationStageStatus::Completed
        } else {
            NoironOrchestrationStageStatus::Failed
        };
    NoironOrchestrationStage::new(
        "routing",
        status,
        vec![
            format!("route_threshold={:.3}", route.route_threshold),
            format!("adaptive_candidates={}", route.adaptive_candidates),
            format!(
                "adaptive_actions=include:{}|compress:{}|defer:{}|skip:{}",
                route.adaptive_include,
                route.adaptive_compress,
                route.adaptive_defer,
                route.adaptive_skip
            ),
            format!(
                "token_accounting=input:{}|retained:{}|saved:{}",
                route.adaptive_input_tokens,
                route.adaptive_retained_tokens,
                route.adaptive_saved_tokens
            ),
            compute_summary.to_owned(),
        ],
        Vec::new(),
    )
}

fn model_adapter_stage(
    outcome: &InferenceOutcome,
    runtime_error_notes: &[String],
) -> NoironOrchestrationStage {
    let status = if runtime_error_notes.is_empty() {
        NoironOrchestrationStageStatus::Completed
    } else {
        NoironOrchestrationStageStatus::Failed
    };
    NoironOrchestrationStage::new(
        "model_adapter",
        status,
        vec![
            format!(
                "runtime_model_present={}",
                outcome.runtime_diagnostics.model_id.is_some()
            ),
            format!(
                "selected_adapter_present={}",
                outcome.runtime_diagnostics.selected_adapter.is_some()
            ),
            format!(
                "recursive_runtime_calls={}",
                outcome.recursive_runtime_calls
            ),
            format!(
                "exported_runtime_kv_blocks={}",
                outcome.exported_runtime_kv_blocks
            ),
        ],
        runtime_error_notes.to_vec(),
    )
}

fn reflection_stage(reflection: &NoironReflectionTrace) -> NoironOrchestrationStage {
    let status =
        if reflection.critical_issue_count == 0 && reflection.runtime_error_notes.is_empty() {
            NoironOrchestrationStageStatus::Completed
        } else {
            NoironOrchestrationStageStatus::Failed
        };
    NoironOrchestrationStage::new(
        "reflection_validation",
        status,
        vec![
            format!("quality={:.3}", reflection.quality),
            format!("issues={}", reflection.issue_count),
            format!("critical_issues={}", reflection.critical_issue_count),
            format!("revision_passes={}", reflection.revision_passes),
            format!("store_as_memory={}", reflection.store_as_memory),
            format!(
                "process_reward={:.3}:{}",
                reflection.process_reward, reflection.process_reward_action
            ),
        ],
        reflection.runtime_error_notes.clone(),
    )
}

fn live_feedback_stage(outcome: &InferenceOutcome) -> NoironOrchestrationStage {
    let status = if feedback_loop_closed(outcome) {
        NoironOrchestrationStageStatus::Completed
    } else {
        NoironOrchestrationStageStatus::PreviewOnly
    };
    NoironOrchestrationStage::new(
        "live_feedback_loop",
        status,
        vec![
            format!(
                "router_threshold_delta={:.6}",
                outcome.live_evolution.router_threshold_delta
            ),
            format!(
                "hierarchy_weight_delta={:.6}",
                outcome.live_evolution.hierarchy_weight_delta
            ),
            format!(
                "online_reward_feedbacks={}",
                outcome.live_evolution.online_reward_feedbacks
            ),
            format!(
                "memory_feedback=updates:{}|applied:{}",
                outcome.memory_feedback.total_updates(),
                outcome.memory_feedback.applied_updates()
            ),
            format!(
                "reflection_feedback=revisions:{}",
                outcome.report.revision_actions.len()
            ),
        ],
        Vec::new(),
    )
}

fn feedback_loop_closed(outcome: &InferenceOutcome) -> bool {
    (outcome.live_evolution.router_threshold_delta > 0.000001
        || outcome.live_evolution.hierarchy_weight_delta > 0.000001)
        && (outcome.live_evolution.online_reward_feedbacks > 0
            || outcome.memory_feedback.total_updates() > 0)
}

fn genome_stage(genome: &NoironGenomeTrace, gates: &NoironGateTrace) -> NoironOrchestrationStage {
    let status = if gates.genome_expression_read_only_preview
        && gates.genome_splice_read_only_preview
        && gates.unauthorized_genome_writes == 0
    {
        NoironOrchestrationStageStatus::PreviewOnly
    } else {
        NoironOrchestrationStageStatus::Failed
    };
    NoironOrchestrationStage::new(
        "reasoning_genome",
        status,
        vec![
            format!("expression_genes={}", genome.expression_gene_count),
            format!("active_genes={}", genome.active_genes),
            format!("aging_genes={}", genome.aging_genes),
            format!("malignant_genes={}", genome.malignant_genes),
            format!("genome_mutation_plans={}", genome.genome_mutation_plans),
            format!("splice_segments={}", genome.splice_segments),
            format!(
                "splice_dispositions=retained:{}|skipped:{}|quarantined:{}|repair:{}",
                genome.splice_retained,
                genome.splice_skipped,
                genome.splice_quarantined,
                genome.splice_repair_candidates
            ),
            format!("splice_findings={}", genome.splice_findings),
            format!("splice_saved_tokens={}", genome.splice_saved_tokens),
        ],
        Vec::new(),
    )
}

fn memory_admission_stage(kv: &NoironKvTrace, gates: &NoironGateTrace) -> NoironOrchestrationStage {
    let status = if gates.all_writes_gated() && gates.memory_admission_read_only_preview {
        NoironOrchestrationStageStatus::PreviewOnly
    } else {
        NoironOrchestrationStageStatus::Failed
    };
    NoironOrchestrationStage::new(
        "memory_admission",
        status,
        vec![
            format!("candidates={}", kv.memory_admission_candidates),
            format!("review_packets={}", kv.memory_admission_review_packets),
            format!("ledger_records={}", kv.memory_ledger_records),
            format!("ledger_authorized={}", kv.memory_ledger_authorized),
            format!("ledger_applied={}", kv.memory_ledger_applied),
            format!("runtime_cache_writes={}", gates.runtime_cache_writes),
            format!("admission_kinds={}", kv.memory_admission_kinds.join("|")),
            format!(
                "admission_decisions={}",
                kv.memory_admission_decisions.join("|")
            ),
        ],
        Vec::new(),
    )
}

fn evolution_stage(
    outcome: &InferenceOutcome,
    gates: &NoironGateTrace,
) -> NoironOrchestrationStage {
    let status = if outcome.drift_report.rollback_adaptive {
        NoironOrchestrationStageStatus::RolledBack
    } else {
        NoironOrchestrationStageStatus::Gated
    };
    let rollback_records = if outcome.drift_report.rollback_adaptive {
        vec![format!(
            "drift_rollback:severity={:?}:ledger_rollbacks={}",
            outcome.drift_report.severity, gates.evolution_ledger_drift_rollbacks
        )]
    } else {
        Vec::new()
    };
    NoironOrchestrationStage::new(
        "evolution_ledger",
        status,
        vec![
            format!("live_runs={}", gates.evolution_ledger_live_runs),
            format!(
                "durable_experiment_ledger_unauthorized={}",
                gates.unauthorized_experiment_ledger_writes
            ),
            format!("drift_rollbacks={}", gates.evolution_ledger_drift_rollbacks),
        ],
        rollback_records,
    )
}

fn retention_stage(outcome: &InferenceOutcome) -> NoironOrchestrationStage {
    NoironOrchestrationStage::new(
        "retention_compaction",
        NoironOrchestrationStageStatus::Completed,
        vec![
            format!(
                "retention_removed={}",
                outcome.retention_report.removed.len()
            ),
            format!(
                "compaction_merges={}",
                outcome.memory_compaction_report.merged.len()
            ),
        ],
        Vec::new(),
    )
}

fn runtime_error_notes(outcome: &InferenceOutcome) -> Vec<String> {
    outcome
        .process_reward
        .notes
        .iter()
        .filter(|note| note.starts_with("runtime_error:"))
        .cloned()
        .collect()
}

fn rollback_records(outcome: &InferenceOutcome, runtime_error_notes: &[String]) -> Vec<String> {
    let mut records = runtime_error_notes.to_vec();
    if outcome.drift_report.rollback_adaptive {
        records.push(format!(
            "drift_rollback:severity={:?}:router_threshold_delta={:.6}:hierarchy_delta={:.6}",
            outcome.drift_report.severity,
            outcome.evolution_ledger.rollback_router_threshold_delta,
            outcome.evolution_ledger.rollback_hierarchy_weight_delta
        ));
    }
    records
}

fn unauthorized_memory_writes(outcome: &InferenceOutcome) -> usize {
    outcome
        .memory_admission
        .ledger_applied_count()
        .saturating_sub(outcome.memory_admission.ledger_authorized_count())
}

fn unauthorized_genome_writes(outcome: &InferenceOutcome) -> usize {
    usize::from(
        (outcome.reasoning_genome.applied && !outcome.reasoning_genome.write_allowed)
            || (outcome.reasoning_genome_splice.applied
                && !outcome.reasoning_genome_splice.write_allowed),
    )
}

fn profile_slug(profile: crate::hierarchy::TaskProfile) -> &'static str {
    match profile {
        crate::hierarchy::TaskProfile::General => "general",
        crate::hierarchy::TaskProfile::Coding => "coding",
        crate::hierarchy::TaskProfile::Writing => "writing",
        crate::hierarchy::TaskProfile::LongDocument => "long_document",
    }
}
