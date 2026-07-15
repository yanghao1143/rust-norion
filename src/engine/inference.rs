use crate::adaptive_state::LiveInferenceEvolution;
use crate::agent_team::AgentTeamInput;
use crate::drift::DriftInput;
use crate::experience::{ExperienceInput, ExperienceRecord};
use crate::gist_memory::GistRecord;
use crate::hardware::RuntimeAdapterHint;
use crate::hierarchy::{TaskAwareHierarchyInput, TaskAwareHierarchyPlanner};
use crate::homeostasis::AllostaticLoadCounters;
use crate::kv_cache::{MemoryEntry, MemoryMatch};
use crate::memory_admission::{
    GeneSegmentKvAdmissionRecord, MemoryAdmissionInput, MemoryAdmissionPreview,
    MemoryPrivacyClassification, ReinforcedKvFusionCandidate, ReinforcedKvFusionPlan,
    ReinforcedKvFusionPolicy, ReinforcedKvFusionSource, fusion_candidate_from_admission,
};
use crate::process_reward::{ProcessRewardInput, RewardAction};
use crate::reasoning_genome::{
    DnaEvolutionApplyDecision, DnaEvolutionController, DnaEvolutionValidationEvidence,
    DnaGeneChain, DnaGeneEvidenceKind, DnaGeneSourceEvidence, DnaSplicePreview, DnaSplicer,
    GeneKvResidency, GenePurposeEvidenceClass, GenePurposeRecord, GenePurposeRelabelDecision,
    GenePurposeRelabelEvidence, GenePurposeRelabelProposal, GenePurposeRelabelValidator,
    GeneScissorsIntent, GeneScissorsOperatorDecision, GeneScissorsTransactionJournal, GeneSegment,
    GeneSegmentDisposition, GeneSegmentSource, GeneValidationStatus, GenomeExpression,
    GenomeExpressionBudget, GenomeExpressionEnvironment, GenomeExpressionInput, GenomeExpressionVm,
    GenomeExpressionVmInput, MutationPlan, ReasoningFrame, ReasoningFrameEfficiencySnapshot,
    ReasoningGenome, ReasoningGenomeStrategy, TaskGeneAdmissionReview, TaskGeneCascade,
    TaskSkillGeneCandidate, TaskSkillGeneEvidence, TaskSkillGeneInput, TaskSkillGeneScorer,
};
use crate::recursive_scheduler::{RecursiveSchedule, RecursiveScheduler};
use crate::reflection::{
    DraftToken, ReasoningStep, ReflectionIssue, ReflectionReport, ReflectionSeverity,
};
use crate::router::{
    AdaptiveRouteCandidate, AdaptiveRouteScoreComponents, AdaptiveRouteSource, AdaptiveRoutingPlan,
    AdaptiveRoutingPlanner, ComputeBudgetContext, ComputeBudgetSchedule, RoutingContext,
};
use crate::runtime::{RuntimeAdapterObservation, RuntimeError};
use crate::tenant_scope::{TenantResourceLane, TenantScope};
use crate::toolsmith::ToolsmithInput;
use crate::writer_gate::{
    UnifiedWriterGate, UnifiedWriterGateCandidate, UnifiedWriterGateDecision,
    UnifiedWriterGatePolicy,
};

use super::NoironEngine;
use super::memory_keys::{
    format_gist_key, format_runtime_kv_key, protected_memory_ids, retention_protected_memory_ids,
    summarize_key,
};
use super::metrics::{hierarchy_weight_delta, metrics_from_report, runtime_error_note_from_trace};
use super::recursive::{
    generate_with_recursive_schedule, generate_with_recursive_schedule_cancelable,
    generate_with_recursive_schedule_stream_checked,
    generate_with_recursive_schedule_stream_checked_cancelable,
};
use super::replay_feedback::*;
use super::types::{
    EmbeddingCall, EmbeddingCallDiagnostics, EmbeddingDiagnostics, EmbeddingSource,
    GenerationContext, GenomeEvolutionAuthorization, GenomeEvolutionPreview, InferenceBackend,
    InferenceOutcome, InferenceRequest, MemoryFeedbackReport, RuntimeTokenMetrics,
    generated_code_behavior_validation_required, generation_cancelled_draft,
};

impl NoironEngine {
    pub fn infer<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
    ) -> InferenceOutcome {
        self.infer_with_stream_observer(request, backend, None, None)
    }

    pub fn infer_cancelable<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
        should_cancel: &mut dyn FnMut() -> bool,
    ) -> InferenceOutcome {
        self.infer_with_stream_observer(request, backend, None, Some(should_cancel))
    }

    pub fn infer_stream<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
        on_token: &mut dyn FnMut(&DraftToken),
    ) -> InferenceOutcome {
        let mut checked = |token: &DraftToken| {
            on_token(token);
            Ok(())
        };
        self.infer_with_stream_observer(request, backend, Some(&mut checked), None)
    }

    pub fn infer_stream_checked<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
    ) -> InferenceOutcome {
        self.infer_with_stream_observer(request, backend, Some(on_token), None)
    }

    pub fn infer_stream_checked_cancelable<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
        should_cancel: &mut dyn FnMut() -> bool,
    ) -> InferenceOutcome {
        self.infer_with_stream_observer(request, backend, Some(on_token), Some(should_cancel))
    }

    fn infer_with_stream_observer<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
        mut on_token: Option<&mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>>,
        mut should_cancel: Option<&mut dyn FnMut() -> bool>,
    ) -> InferenceOutcome {
        let request_scope = request
            .tenant_scope
            .clone()
            .unwrap_or_else(TenantScope::local_single_user);
        let defer_auto_replay = backend.defer_auto_replay_until_generation_result();
        let mut auto_replay_report = if defer_auto_replay {
            None
        } else {
            self.maybe_auto_replay(&request_scope)
        };
        let adaptive_before_inference = self.adaptive_state();
        let query_embedding = self.embed_for_backend(backend, &request.prompt);
        let mut embedding_diagnostics =
            EmbeddingDiagnostics::from_query(query_embedding.diagnostics);
        let genome_scope = request_scope.clone();
        let genome_generation_before = self.genome_runtime_state.generation(request.profile);
        let active_genome = self.genome_runtime_state.active(request.profile).clone();
        let genome_authorization = request.genome_evolution_authorization.as_ref();
        let tenant_scope = Some(&request_scope);
        let used_memories =
            lookup_request_memories(&self.cache, tenant_scope, &query_embedding.vector, 4);
        let scoped_cache_entries = tenant_scope.map(|scope| self.cache.entries_scoped(scope));
        let cache_entries = scoped_cache_entries
            .as_deref()
            .unwrap_or_else(|| self.cache.entries());
        let used_experiences = lookup_request_experiences(
            &self.experience,
            scoped_cache_entries.as_deref(),
            &request.prompt,
            request.profile,
            3,
        );
        let recursive_scheduler =
            self.scheduler_for_backend_window(backend.runtime_native_context_window());
        let recursive_schedule = recursive_scheduler.plan(&request.prompt);
        let base_hierarchy = self.hierarchy.adapt_to_profile(request.profile);
        let task_hierarchy_plan = TaskAwareHierarchyPlanner::new().plan(TaskAwareHierarchyInput {
            prompt: &request.prompt,
            profile: request.profile,
            max_tokens: request.max_tokens,
            prompt_tokens: recursive_schedule.prompt_tokens,
            used_memories: used_memories.len(),
            threshold_before: self.router.threshold_for(request.profile),
            hierarchy_before: base_hierarchy,
        });
        let base_hierarchy = task_hierarchy_plan.hierarchy_after;
        let hardware_plan = self.hardware_allocator.plan(
            self.hardware_snapshot,
            request.profile,
            recursive_schedule.prompt_tokens,
            base_hierarchy,
        );
        let toolsmith_plan = self.toolsmith_planner.plan(ToolsmithInput {
            prompt: &request.prompt,
            profile: request.profile,
            memories: &used_memories,
            experiences: &used_experiences,
            hardware_plan: &hardware_plan,
        });
        let genome_strategy = ReasoningGenomeStrategy::select(
            request.profile,
            task_hierarchy_plan.signals.language.as_str(),
            !toolsmith_plan.blueprints.is_empty() || !toolsmith_plan.rejected_requests.is_empty(),
        );
        let strategy_genome =
            ReasoningGenome::default_for_strategy(genome_strategy, request.profile);
        let mut recursive_schedule =
            recursive_schedule.with_parallel_budget(hardware_plan.execution.max_parallel_chunks);
        let homeostatic_gate = self.homeostatic_setpoints.evaluate(AllostaticLoadCounters {
            runtime_memory_pressure_milli: pressure_milli(self.hardware_snapshot.ram_load),
            device_pressure_milli: pressure_milli(self.hardware_snapshot.pressure()),
            // This path has no queued admission candidates; cache size is retained memory, not backlog.
            memory_candidate_backlog: 0,
            consecutive_high_load_windows: usize::from(
                self.hardware_snapshot.ram_load > 0.85 || hardware_plan.pressure > 0.85,
            ),
            recovery_stable_windows: usize::from(
                self.hardware_snapshot.ram_load <= 0.85 && hardware_plan.pressure <= 0.85,
            ),
            ..AllostaticLoadCounters::default()
        });
        if !homeostatic_gate.recursive_spawn_allowed {
            recursive_schedule = recursive_schedule.without_recursion();
        }
        let tier_plan = self.tiered_cache.plan(cache_entries, &used_memories);
        let tier_migrations = tier_plan.migrations_from(&self.last_tier_plan);
        let infini_memory_planner = self.infini_memory_planner.clone().with_token_budgets(
            hardware_plan.local_kv_token_budget,
            hardware_plan.global_kv_token_budget,
        );
        let infini_memory_plan = infini_memory_planner.plan(cache_entries, &used_memories);
        let routing_context = RoutingContext {
            profile: request.profile,
            context_tokens: recursive_schedule.prompt_tokens,
            cache_hit_rate: used_memories.len() as f32 / 4.0,
            latency_budget_ms: hardware_plan.latency_budget_ms,
            hardware_pressure: hardware_plan.pressure,
            compute_headroom: hardware_plan.compute_headroom(),
            hierarchy: hardware_plan.hierarchy,
        };
        let initial_route_budget = self.router.budget_for_prompt_with_context_threshold(
            &request.prompt,
            routing_context,
            task_hierarchy_plan.threshold_after,
        );
        let pre_reasoning_input = GenomeExpressionInput {
            profile: request.profile,
            quality: genome_fitness(&active_genome),
            process_reward: genome_fitness(&active_genome),
            contradiction_count: 0,
            critical_reflection_issue_count: 0,
            revision_action_count: 0,
            used_memories: used_memories.len(),
            memory_feedback_updates: 0,
            route_attention_fraction: initial_route_budget.attention_fraction,
            agent_team_collision_free: true,
            toolsmith_gate_passed: true,
            drift_memory_write_allowed: true,
            genome_mutation_allowed: true,
            drift_rollback: false,
            runtime_kv_hold: false,
        };
        let strategy_genome = strategy_genome.express(pre_reasoning_input);
        let pre_reasoning_genome = active_genome
            .express(pre_reasoning_input)
            .compose_read_only(&strategy_genome);
        let pre_reasoning_genome_chain = DnaGeneChain::preview_from_genome(
            &active_genome,
            genome_scope.lineage_tenant_scope(),
            genome_scope.session_id.clone(),
            DnaGeneSourceEvidence::new(
                if genome_generation_before == 0 {
                    DnaGeneEvidenceKind::SyntheticDefault
                } else {
                    DnaGeneEvidenceKind::OperatorApproved
                },
                genome_scope.scope_digest(),
                "active genome loaded before generation",
            )
            .with_privacy_gate(),
        );
        let pre_reasoning_genome_splice = reasoning_genome_splice_preview(
            request.profile,
            &recursive_schedule,
            &used_memories,
            &[],
            0,
            genome_fitness(&active_genome),
            false,
            false,
            false,
            false,
            active_genome.stable_anchor_id.clone(),
            &genome_scope,
        );
        let (adaptive_route_plan, compute_budget_schedule) =
            adaptive_route_plan_from_runtime_evidence(
                request.profile,
                initial_route_budget.threshold,
                routing_context,
                ComputeBudgetContext::from_task_plan(
                    &task_hierarchy_plan,
                    recursive_schedule.prompt_tokens,
                )
                .with_max_tokens(request.max_tokens),
                &pre_reasoning_genome,
                &pre_reasoning_genome_splice,
                genome_fitness(&active_genome),
                None,
                0.0,
            );
        let route_budget = self.router.budget_for_prompt_with_context_threshold(
            &request.prompt,
            routing_context,
            compute_budget_schedule.threshold_after,
        );
        recursive_schedule = recursive_schedule.with_parallel_budget(
            hardware_plan
                .execution
                .max_parallel_chunks
                .min(compute_budget_schedule.route_fanout_after.max(1)),
        );
        let execution_hierarchy = hardware_plan.hierarchy;
        let transformer_plan =
            self.transformer_planner
                .plan(request.profile, execution_hierarchy, route_budget);
        let agent_team_plan = self.agent_team_planner.plan(AgentTeamInput {
            prompt: &request.prompt,
            profile: request.profile,
            memories: &used_memories,
            experiences: &used_experiences,
            hardware_plan: &hardware_plan,
            route_budget,
            recursive_schedule: &recursive_schedule,
            toolsmith_plan: &toolsmith_plan,
            layer_b_route_proof: request.agent_team_route_proof.as_ref(),
        });
        let task_gene_cascade = TaskGeneCascade::from_agent_team_plan(
            active_genome.id.clone(),
            request.profile,
            &agent_team_plan,
        );
        let task_gene_review = task_gene_cascade.review();
        let task_skill_gene = TaskSkillGeneScorer::default().score_candidate(
            &[],
            TaskSkillGeneInput::new(
                request.profile,
                task_hierarchy_plan.signals.language.as_str(),
                task_hierarchy_plan.mode.as_str(),
                format!(
                    "active {} strategy genome task policy",
                    genome_strategy.as_str()
                ),
            )
            .with_validation_expectations([
                "reasoning_frame_preview",
                "task_gene_admission",
                "rollback_anchor",
            ])
            .with_failure_modes(["reflection_regression", "route_budget_waste"])
            .with_clean_room_provenance("runtime-active-genome")
            .with_rollback_anchor(active_genome.stable_anchor_id.clone())
            .with_evidence(
                genome_authorization
                    .filter(|authorization| authorization.is_valid())
                    .map(|authorization| authorization.task_skill_evidence().clone())
                    .unwrap_or_else(TaskSkillGeneEvidence::default),
            )
            .with_operator_decision(
                if genome_authorization.is_some_and(GenomeEvolutionAuthorization::is_valid) {
                    GeneScissorsOperatorDecision::Approved
                } else {
                    GeneScissorsOperatorDecision::Pending
                },
            ),
        );
        let task_state_digest = crate::privacy_redaction::stable_redaction_digest([
            "pre-reasoning-task-state",
            task_hierarchy_plan.mode.as_str(),
            task_hierarchy_plan.signals.language.as_str(),
            task_gene_review.decision.as_str(),
            task_skill_gene.decision.as_str(),
        ]);
        let used_memory_count = used_memories.len().to_string();
        let memory_state_digest = crate::privacy_redaction::stable_redaction_digest([
            "pre-reasoning-memory-state",
            used_memory_count.as_str(),
            genome_scope.scope_digest().as_str(),
        ]);
        let runtime_pressure = format!("{:.3}", hardware_plan.pressure);
        let runtime_health_digest = crate::privacy_redaction::stable_redaction_digest([
            "pre-reasoning-runtime-health",
            hardware_plan.execution.primary_lane.as_str(),
            hardware_plan.execution.fallback_lane.as_str(),
            runtime_pressure.as_str(),
        ]);
        let stimulus_digest = crate::privacy_redaction::stable_redaction_digest([
            "pre-reasoning-stimulus",
            request.prompt.as_str(),
            active_genome.id.as_str(),
        ]);
        let expression_environment = GenomeExpressionEnvironment::new(
            stimulus_digest,
            task_state_digest,
            memory_state_digest,
            runtime_health_digest,
        );
        let expression_budget = GenomeExpressionBudget {
            compute_budget: compute_budget_schedule.compute_budget.as_str().to_owned(),
            max_tokens: request
                .max_tokens
                .unwrap_or(compute_budget_schedule.estimated_budget_tokens.max(1)),
            threshold_milli: bounded_milli(compute_budget_schedule.threshold_after),
            route_fanout: compute_budget_schedule.route_fanout_after.max(1),
            reflection_passes: compute_budget_schedule.reflection_pass_budget,
            validation_runs: compute_budget_schedule.validation_run_budget,
            memory_records: compute_budget_schedule.kv_lookups_planned,
        };
        let reasoning_frame = GenomeExpressionVm
            .execute(
                GenomeExpressionVmInput::new(
                    &pre_reasoning_genome,
                    &expression_environment,
                    &expression_budget,
                )
                .with_task_gene(task_gene_cascade.genes.first()),
            )
            .with_efficiency_snapshot(ReasoningFrameEfficiencySnapshot::preview(
                pre_reasoning_genome.active_gene_count(),
                pre_reasoning_genome_splice.intron_count(),
                pre_reasoning_genome_splice.segments.len(),
                adaptive_route_plan.candidates,
                compute_budget_schedule.compute_budget.as_str(),
                adaptive_route_plan.input_tokens,
                adaptive_route_plan.retained_tokens,
                adaptive_route_plan.saved_tokens,
                compute_budget_schedule.validation_cost_tokens,
                genome_fitness(&active_genome),
                genome_fitness(&active_genome),
            ));
        let reasoning_frame_valid = reasoning_frame.validate_preview().is_ok();
        let generation_prompt = dna_control_prompt(
            &request.prompt,
            &active_genome,
            genome_generation_before,
            genome_strategy,
            &strategy_genome,
            &reasoning_frame,
            reasoning_frame_valid,
            &task_gene_review,
            &task_skill_gene,
            &compute_budget_schedule,
        );
        let recalibrated_schedule = recursive_scheduler
            .plan(&generation_prompt)
            .with_parallel_budget(hardware_plan.execution.max_parallel_chunks);
        if recalibrated_schedule.prompt_tokens > recursive_schedule.prompt_tokens {
            recursive_schedule = if homeostatic_gate.recursive_spawn_allowed {
                recalibrated_schedule
            } else {
                recalibrated_schedule.without_recursion()
            };
        }

        backend.configure_generation(request.max_tokens);
        let generation_context = GenerationContext {
            prompt: &generation_prompt,
            profile: request.profile,
            tenant_scope,
            memories: &used_memories,
            route_budget,
            hierarchy: execution_hierarchy,
            tier_plan: &tier_plan,
            infini_memory_plan: &infini_memory_plan,
            recursive_schedule: &recursive_schedule,
            hardware_plan: &hardware_plan,
            experiences: &used_experiences,
            toolsmith_plan: &toolsmith_plan,
            agent_team_plan: &agent_team_plan,
            transformer_plan: &transformer_plan,
        };
        let streamed_during_generation = on_token.is_some();
        let (mut draft, mut recursive_runtime_calls) =
            match (on_token.as_mut(), should_cancel.as_mut()) {
                (Some(on_token), Some(should_cancel)) => {
                    generate_with_recursive_schedule_stream_checked_cancelable(
                        backend,
                        generation_context,
                        *on_token,
                        *should_cancel,
                    )
                }
                (Some(on_token), None) => generate_with_recursive_schedule_stream_checked(
                    backend,
                    generation_context,
                    *on_token,
                ),
                (None, Some(should_cancel)) => generate_with_recursive_schedule_cancelable(
                    backend,
                    generation_context,
                    *should_cancel,
                ),
                (None, None) => generate_with_recursive_schedule(backend, generation_context),
            };
        let cancellation_trace = |draft: &crate::reflection::InferenceDraft| {
            draft.trace.iter().any(|step| {
                matches!(
                    step.label.as_str(),
                    "runtime_generation_cancelled_error" | "newapi_fallback_cancelled"
                )
            })
        };
        let non_persistent_abort_trace = |draft: &crate::reflection::InferenceDraft| {
            draft
                .trace
                .iter()
                .any(|step| step.label == "runtime_model_pool_call_blocked_error")
        };
        let non_persistent_abort = non_persistent_abort_trace(&draft);
        let mut generation_cancelled =
            should_cancel.as_mut().is_some_and(|cancel| cancel()) || cancellation_trace(&draft);
        if generation_cancelled && !cancellation_trace(&draft) {
            draft = generation_cancelled_draft();
        }
        let mut report = self.reflector.reflect(&request.prompt, &draft);
        if !non_persistent_abort
            && request.profile == crate::hierarchy::TaskProfile::Coding
            && let Some(validation) = validate_rust_answer(&report.revised_answer)
            && !validation.passed
        {
            if !streamed_during_generation {
                let retry_prompt = format!(
                    "{generation_prompt}\n\n[noiron-rust-validation] The previous answer failed rustc:\n{}\nReturn corrected Rust code that compiles. Do not claim the failed code is correct.",
                    compact_diagnostic(&validation.diagnostic, 1_200)
                );
                let retry_context = GenerationContext {
                    prompt: &retry_prompt,
                    profile: request.profile,
                    tenant_scope,
                    memories: &used_memories,
                    route_budget,
                    hierarchy: execution_hierarchy,
                    tier_plan: &tier_plan,
                    infini_memory_plan: &infini_memory_plan,
                    recursive_schedule: &recursive_schedule,
                    hardware_plan: &hardware_plan,
                    experiences: &used_experiences,
                    toolsmith_plan: &toolsmith_plan,
                    agent_team_plan: &agent_team_plan,
                    transformer_plan: &transformer_plan,
                };
                let (mut retry_draft, retry_calls) =
                    if let Some(should_cancel) = should_cancel.as_mut() {
                        generate_with_recursive_schedule_cancelable(
                            backend,
                            retry_context,
                            *should_cancel,
                        )
                    } else {
                        generate_with_recursive_schedule(backend, retry_context)
                    };
                recursive_runtime_calls = recursive_runtime_calls.saturating_add(retry_calls);
                retry_draft.trace.push(ReasoningStep::new(
                    "rust_validation_retry",
                    "regenerated after rustc rejected the first coding answer",
                    0.92,
                ));
                draft = retry_draft;
                report = self.reflector.reflect(&request.prompt, &draft);
            }

            match validate_rust_answer(&report.revised_answer) {
                Some(retry_validation) if retry_validation.passed => {}
                Some(retry_validation) => apply_critical_validation_issue(
                    &mut report,
                    "rust_validation_failed",
                    "reject_uncompilable_rust_answer",
                    format!(
                        "rustc rejected generated Rust: {}",
                        compact_diagnostic(&retry_validation.diagnostic, 600)
                    ),
                ),
                None => apply_critical_validation_issue(
                    &mut report,
                    "rust_validation_missing_code",
                    "reject_missing_rust_retry",
                    "Rust retry did not contain a compilable code candidate".to_owned(),
                ),
            }
        }
        let deterministic_code_validation_passed = request.profile
            == crate::hierarchy::TaskProfile::Coding
            && validate_rust_answer(&report.revised_answer).is_some_and(|report| report.passed);
        if generated_code_behavior_validation_required(&request.prompt)
            && !deterministic_code_validation_passed
        {
            apply_unverified_behavior_gate(&mut report);
        }
        apply_memory_grounding_gate(&request.prompt, &used_memories, &mut report);
        if report.issues.iter().any(|issue| {
            matches!(
                issue.code.as_str(),
                "rust_validation_failed"
                    | "rust_validation_missing_code"
                    | "memory_grounding_contradiction"
            )
        }) {
            draft.answer.clone_from(&report.revised_answer);
            draft.tokens = draft
                .answer
                .split_whitespace()
                .map(DraftToken::new)
                .collect();
        }
        let cancel_requested = should_cancel.as_mut().is_some_and(|cancel| cancel());
        let cancellation_draft = cancellation_trace(&draft);
        if cancel_requested && !cancellation_draft {
            draft = generation_cancelled_draft();
            report = self.reflector.reflect(&request.prompt, &draft);
        }
        generation_cancelled |= cancel_requested || cancellation_draft;
        // ponytail: abort paths pay one state clone; replace with transactional deltas if abort latency becomes material.
        let discard_post_generation_state =
            generation_cancelled || non_persistent_abort || non_persistent_abort_trace(&draft);
        if discard_post_generation_state {
            self.restore_adaptive_state(adaptive_before_inference.clone());
        }
        let mut shadow_engine = discard_post_generation_state.then(|| self.clone());
        let engine = match shadow_engine.as_mut() {
            Some(engine) => engine,
            None => self,
        };
        let runtime_token_metrics = RuntimeTokenMetrics::from_draft(&draft);
        let runtime_diagnostics = draft.runtime_diagnostics.clone();
        let runtime_adapter_observations = RuntimeAdapterObservation::from_experiences_for_hardware(
            &used_experiences,
            runtime_diagnostics.model_id.as_deref().unwrap_or_default(),
            &hardware_plan,
        );
        let metrics = metrics_from_report(&draft, &report, route_budget, runtime_token_metrics);
        let gist_records =
            engine
                .gist_generator
                .generate(&request.prompt, &report.revised_answer, report.quality);
        let stream_reports = engine.stream_monitor.observe_draft_with_profile(
            &mut engine.router,
            request.profile,
            &draft,
            report.quality,
            report.contradictions.len(),
        );
        let exported_runtime_kv_blocks = draft.exported_kv_blocks.len();
        let drift_report = engine.drift_guard.evaluate(DriftInput {
            quality: report.quality,
            contradiction_count: report.contradictions.len(),
            metrics,
            route_budget,
            used_memories: used_memories.len(),
            exported_runtime_kv_blocks,
            stream_windows: stream_reports.len(),
        });
        draft.trace.push(ReasoningStep::new(
            "homeostatic_gate",
            homeostatic_gate.trace_line(),
            0.9,
        ));
        let admit_memory = report.store_as_memory
            && drift_report.allow_memory_write
            && homeostatic_gate.memory_admission_allowed;
        let admit_runtime_kv =
            admit_memory && drift_report.allow_runtime_kv_write && report.revision_passes == 0;

        let stored_memory_id = if admit_memory {
            let memory_text = format!(
                "prompt:{}\nanswer:{}\nlesson:{}",
                request.prompt.as_str(),
                report.revised_answer,
                report.lesson
            );
            let memory_embedding = engine.embed_for_backend(backend, &memory_text);
            embedding_diagnostics.record_memory_write(memory_embedding.diagnostics);
            Some(store_request_memory(
                &mut engine.cache,
                tenant_scope,
                TenantResourceLane::KvMemory,
                summarize_key(&request.prompt, &report.lesson),
                memory_embedding.vector,
                report.quality,
            ))
        } else {
            None
        };

        let stored_gist_memory_ids = if admit_memory {
            let mut ids = gist_records
                .iter()
                .filter(|gist| gist.importance >= 0.54)
                .map(|gist| {
                    let memory_text = gist.hint();
                    let gist_embedding = engine.embed_for_backend(backend, &memory_text);
                    embedding_diagnostics.record_gist_write(gist_embedding.diagnostics);
                    store_request_memory(
                        &mut engine.cache,
                        tenant_scope,
                        TenantResourceLane::KvMemory,
                        format_gist_key(&request.prompt, gist),
                        gist_embedding.vector,
                        (report.quality * gist.importance).clamp(0.0, 1.0),
                    )
                })
                .collect::<Vec<_>>();
            ids.sort_unstable();
            ids.dedup();
            ids
        } else {
            Vec::new()
        };
        let stored_runtime_kv_memory_ids = if admit_runtime_kv {
            let mut ids = draft
                .exported_kv_blocks
                .iter()
                .filter(|block| !block.is_empty())
                .map(|block| {
                    store_request_memory(
                        &mut engine.cache,
                        tenant_scope,
                        TenantResourceLane::RuntimeKv,
                        format_runtime_kv_key(&request.prompt, block),
                        block.vector(),
                        (report.quality * 0.86).clamp(0.05, 1.0),
                    )
                })
                .collect::<Vec<_>>();
            ids.sort_unstable();
            ids.dedup();
            ids
        } else {
            Vec::new()
        };

        let runtime_kv_segment_yield = runtime_diagnostics.runtime_kv_segment_yield();
        let runtime_kv_budget_pressure = runtime_kv_budget_pressure(
            exported_runtime_kv_blocks,
            runtime_diagnostics.budget_limited_runtime_kv_imports_skipped,
        );
        let mut memory_feedback = MemoryFeedbackReport::default();
        for memory in &used_memories {
            if let Some(amount) =
                used_memory_runtime_kv_segment_penalty_amount(&memory.key, runtime_kv_segment_yield)
            {
                let update = engine.cache.penalize(memory.id, amount);
                memory_feedback.record_penalty(amount, update);
            } else if admit_memory && !drift_report.penalize_used_memory {
                let amount = used_memory_reinforcement_amount(
                    &memory.key,
                    &report,
                    runtime_kv_segment_yield,
                );
                let update = engine.cache.reinforce(memory.id, amount);
                memory_feedback.record_reinforcement(amount, update);
            } else {
                let amount = used_memory_penalty_amount(&report, &drift_report, metrics);
                let update = engine.cache.penalize(memory.id, amount);
                memory_feedback.record_penalty(amount, update);
            }
        }

        let baseline_router_threshold = adaptive_before_inference
            .router
            .profile_thresholds
            .get(request.profile);
        let baseline_hierarchy_weights = adaptive_before_inference
            .hierarchy
            .profile_weights
            .get(request.profile);
        engine.router.observe_with_profile(request.profile, metrics);
        engine.hierarchy.observe(request.profile, metrics);
        if drift_report.rollback_adaptive {
            let rollback_router_threshold_delta =
                (engine.router.threshold_for(request.profile) - baseline_router_threshold).abs();
            let rollback_hierarchy_weight_delta = hierarchy_weight_delta(
                baseline_hierarchy_weights,
                engine
                    .hierarchy
                    .state()
                    .profile_weights
                    .get(request.profile),
            );
            engine.restore_adaptive_state(adaptive_before_inference);
            engine.evolution_ledger.record_drift_rollback(
                rollback_router_threshold_delta,
                rollback_hierarchy_weight_delta,
            );
        }
        let mut process_reward = engine.process_rewarder.score(ProcessRewardInput {
            profile: request.profile,
            route_budget,
            hierarchy: execution_hierarchy,
            metrics,
            quality: report.quality,
            contradiction_count: report.contradictions.len(),
            reflection_issue_count: report.issues.len(),
            critical_reflection_issue_count: report.critical_issue_count(),
            revision_action_count: report.revision_actions.len(),
            used_memories: used_memories.len(),
            used_experiences: used_experiences.len(),
            tier_counts: tier_plan.counts(),
            infini_counts: infini_memory_plan.counts(),
            recursive_schedule: recursive_schedule.clone(),
            recursive_runtime_calls,
            stream_windows: stream_reports.len(),
            stored_memory: stored_memory_id.is_some(),
            stored_gist_memories: stored_gist_memory_ids.len(),
            stored_runtime_kv_memories: stored_runtime_kv_memory_ids.len(),
            imported_runtime_kv_blocks: runtime_diagnostics.imported_kv_blocks,
            weak_runtime_kv_imports_skipped: runtime_diagnostics.weak_runtime_kv_imports_skipped,
            budget_limited_runtime_kv_imports_skipped: runtime_diagnostics
                .budget_limited_runtime_kv_imports_skipped,
            runtime_kv_segments_included: runtime_diagnostics.runtime_kv_segments_included,
            runtime_kv_segments_skipped: runtime_diagnostics.runtime_kv_segments_skipped,
            runtime_kv_segments_rejected: runtime_diagnostics.runtime_kv_segments_rejected,
            gist_records: gist_records.len(),
            toolsmith_plan: toolsmith_plan.clone(),
            agent_team_plan: agent_team_plan.clone(),
        });
        let mut online_reward_feedbacks = 0;
        let mut online_reward_reinforcements = 0;
        let mut online_reward_penalties = 0;
        let mut online_reward_strength = 0.0;
        let mut online_reward_reinforcement_strength = 0.0;
        let mut online_reward_penalty_strength = 0.0;
        if let Some(reward_metrics) =
            process_reward_feedback_metrics(&process_reward, metrics, &report, &drift_report)
        {
            engine
                .router
                .observe_with_profile(request.profile, reward_metrics);
            engine.hierarchy.observe(request.profile, reward_metrics);
            online_reward_feedbacks = 1;
            online_reward_strength = process_reward_feedback_strength(&process_reward);
            match process_reward.action {
                RewardAction::Reinforce => {
                    online_reward_reinforcements = 1;
                    online_reward_reinforcement_strength = online_reward_strength;
                }
                RewardAction::Penalize => {
                    online_reward_penalties = 1;
                    online_reward_penalty_strength = online_reward_strength;
                }
                RewardAction::Hold => {}
            }
            let feedback_note = process_reward_feedback_note(&process_reward, reward_metrics);
            process_reward.notes.push(feedback_note);
        }
        let runtime_kv_stored_count = stored_runtime_kv_memory_ids.len();
        let runtime_kv_hold =
            exported_runtime_kv_blocks.saturating_sub(runtime_kv_stored_count) > 0;
        let best_adapter_observation = runtime_adapter_observations.first();
        let runtime_selected_adapter = runtime_diagnostics
            .selected_adapter
            .as_deref()
            .and_then(RuntimeAdapterHint::canonical_name);
        let runtime_adapter_selection_mismatch = match (
            best_adapter_observation.map(|observation| observation.adapter.as_str()),
            runtime_selected_adapter,
        ) {
            (Some(best_adapter), Some(selected_adapter)) => best_adapter != selected_adapter,
            _ => false,
        };
        let runtime_adapter_current_signal = runtime_selected_adapter.is_some();
        let mut memory_admission = MemoryAdmissionPreview::from_feedback(MemoryAdmissionInput {
            prompt: &request.prompt,
            profile: request.profile,
            report: &report,
            process_reward: &process_reward,
            drift_report: &drift_report,
            stored_memory: stored_memory_id.is_some(),
            gist_records: gist_records.len(),
            stored_gist_memories: stored_gist_memory_ids.len(),
            imported_runtime_kv_blocks: runtime_diagnostics.imported_kv_blocks,
            exported_runtime_kv_blocks,
            stored_runtime_kv_memories: stored_runtime_kv_memory_ids.len(),
            weak_runtime_kv_imports_skipped: runtime_diagnostics.weak_runtime_kv_imports_skipped,
            runtime_kv_hold,
            runtime_kv_influence: runtime_diagnostics.kv_influence,
            budget_limited_runtime_kv_imports_skipped: runtime_diagnostics
                .budget_limited_runtime_kv_imports_skipped,
            runtime_kv_segments_included: runtime_diagnostics.runtime_kv_segments_included,
            runtime_kv_segments_skipped: runtime_diagnostics.runtime_kv_segments_skipped,
            runtime_kv_segments_rejected: runtime_diagnostics.runtime_kv_segments_rejected,
            used_memories: used_memories.len(),
            memory_feedback_updates: memory_feedback.total_updates(),
            runtime_adapter_observations: runtime_adapter_observations.len(),
            runtime_adapter_current_signal,
            runtime_adapter_selection_mismatch,
            runtime_adapter_best_score: best_adapter_observation
                .map(|observation| observation.score),
            runtime_adapter_best_reward: best_adapter_observation
                .map(|observation| observation.reward),
            runtime_adapter_best_quality: best_adapter_observation
                .map(|observation| observation.quality),
            toolsmith_blueprints: toolsmith_plan.blueprint_count(),
            toolsmith_ready: toolsmith_plan.ready_count(),
            toolsmith_held: toolsmith_plan.held_count(),
            toolsmith_rejected: toolsmith_plan.rejected_count(),
            toolsmith_gate_passed: toolsmith_plan.passed_rust_gate(),
            trace_segment_source_scope: None,
            trace_segment_target_scope: None,
            trace_segment_movement_review: None,
        });
        let genome_input = GenomeExpressionInput {
            profile: request.profile,
            quality: report.quality,
            process_reward: process_reward.total,
            contradiction_count: report.contradictions.len(),
            critical_reflection_issue_count: report.critical_issue_count(),
            revision_action_count: report.revision_actions.len(),
            used_memories: used_memories.len(),
            memory_feedback_updates: memory_feedback.total_updates(),
            route_attention_fraction: route_budget.attention_fraction,
            agent_team_collision_free: agent_team_plan.collision_free(),
            toolsmith_gate_passed: toolsmith_plan.passed_rust_gate(),
            drift_memory_write_allowed: drift_report.allow_memory_write,
            genome_mutation_allowed: homeostatic_gate.genome_mutation_allowed,
            drift_rollback: drift_report.rollback_adaptive,
            runtime_kv_hold,
        };
        let genome = active_genome.clone().with_feedback_health(&genome_input);
        let reasoning_genome_chain = DnaGeneChain::preview_from_genome(
            &genome,
            genome_scope.lineage_tenant_scope(),
            genome_scope.session_id.clone(),
            DnaGeneSourceEvidence::new(
                DnaGeneEvidenceKind::Reflection,
                genome_scope.scope_digest(),
                "post-generation reflection genome expression",
            )
            .with_privacy_gate(),
        );
        let reasoning_genome = genome.express(genome_input);
        let reasoning_genome_splice = reasoning_genome_splice_preview(
            request.profile,
            &recursive_schedule,
            &used_memories,
            &gist_records,
            exported_runtime_kv_blocks,
            report.quality,
            drift_report.rollback_adaptive,
            drift_report.penalize_used_memory,
            drift_report.allow_runtime_kv_write,
            runtime_kv_hold,
            reasoning_genome.stable_anchor_id.clone(),
            &genome_scope,
        );
        memory_admission = memory_admission.with_gene_segment_kv_records(
            gene_segment_kv_admission_records_from_splice(
                &reasoning_genome_splice,
                &genome_scope.session_id,
            ),
        );
        memory_admission.fusion_plan = reinforced_kv_fusion_plan_from_runtime_evidence(
            &memory_admission,
            &reasoning_genome_splice,
            process_reward.total,
        );
        let (adaptive_route_plan, compute_budget_schedule) =
            adaptive_route_plan_from_runtime_evidence(
                request.profile,
                route_budget.threshold,
                routing_context,
                ComputeBudgetContext::from_task_plan(
                    &task_hierarchy_plan,
                    recursive_schedule.prompt_tokens,
                )
                .with_max_tokens(request.max_tokens)
                .with_runtime_kv_budget_pressure(runtime_kv_budget_pressure),
                &reasoning_genome,
                &reasoning_genome_splice,
                process_reward.total,
                runtime_kv_segment_yield,
                runtime_kv_budget_pressure,
            );
        let authorization = genome_authorization;
        let (validated_plans, gene_purpose_reviews) =
            validated_genome_plans(&genome, &reasoning_genome, &genome_scope, authorization);
        let operator_decision = if authorization.is_some_and(GenomeEvolutionAuthorization::is_valid)
        {
            GeneScissorsOperatorDecision::Approved
        } else {
            GeneScissorsOperatorDecision::Pending
        };
        let mut gene_scissors_journal = GeneScissorsTransactionJournal::from_mutation_plans(
            request.profile,
            reasoning_genome.stable_anchor_id.clone(),
            &validated_plans,
        );
        for transaction in &mut gene_scissors_journal.transactions {
            transaction.operator_decision = operator_decision;
        }
        let validation = authorization
            .filter(|authorization| authorization.is_valid())
            .map(|authorization| {
                authorization
                    .validation()
                    .clone()
                    .with_artifact_digest(authorization.approval_ref())
            })
            .unwrap_or_else(DnaEvolutionValidationEvidence::default);
        let dna_evolution_controller = DnaEvolutionController::default().preview_plans(
            request.profile,
            active_genome.id.as_str(),
            reasoning_genome.stable_anchor_id.as_str(),
            &validated_plans,
            &validation,
            operator_decision,
            Some(&gene_scissors_journal),
        );
        let dna_writer_gate = UnifiedWriterGate::new()
            .with_policy(UnifiedWriterGatePolicy {
                durable_writes_enabled: authorization
                    .is_some_and(GenomeEvolutionAuthorization::is_valid),
                ..UnifiedWriterGatePolicy::default()
            })
            .evaluate([
                UnifiedWriterGateCandidate::genome_transaction_journal(&gene_scissors_journal),
                UnifiedWriterGateCandidate::dna_evolution_controller_report(
                    &dna_evolution_controller,
                ),
            ]);
        let dna_apply_plan = dna_evolution_controller.explicit_apply_plan(&dna_writer_gate);
        let dna_apply_receipt = if let Some(authorization) = authorization {
            if !authorization.is_valid() {
                crate::adaptive_state::GenomeEvolutionApplyReceipt::held(
                    request.profile,
                    genome_generation_before,
                    active_genome.id.clone(),
                    "approval_ref_missing",
                )
            } else if validated_plans.is_empty() {
                crate::adaptive_state::GenomeEvolutionApplyReceipt::held(
                    request.profile,
                    genome_generation_before,
                    active_genome.id.clone(),
                    "no_mutation_candidate",
                )
            } else if dna_writer_gate.decision != UnifiedWriterGateDecision::ReadyForExplicitApply {
                crate::adaptive_state::GenomeEvolutionApplyReceipt::held(
                    request.profile,
                    genome_generation_before,
                    active_genome.id.clone(),
                    "writer_gate_not_ready",
                )
            } else if dna_apply_plan.decision != DnaEvolutionApplyDecision::ReadyForExplicitApply {
                crate::adaptive_state::GenomeEvolutionApplyReceipt::held(
                    request.profile,
                    genome_generation_before,
                    active_genome.id.clone(),
                    "apply_plan_not_ready",
                )
            } else {
                engine.genome_runtime_state.apply_with_lineage(
                    request.profile,
                    &genome,
                    &validated_plans,
                    &gene_scissors_journal.to_journal_lines(),
                    authorization.approval_ref(),
                    &genome_scope.lineage_tenant_scope(),
                    &genome_scope.session_id,
                )
            }
        } else {
            crate::adaptive_state::GenomeEvolutionApplyReceipt::held(
                request.profile,
                genome_generation_before,
                active_genome.id.clone(),
                "explicit_authorization_missing",
            )
        };

        let router_threshold_after = engine.router.threshold();
        let live_router_threshold_delta = if drift_report.rollback_adaptive {
            0.0
        } else {
            (engine.router.threshold_for(request.profile) - baseline_router_threshold).abs()
        };
        let live_hierarchy_weight_delta = if drift_report.rollback_adaptive {
            0.0
        } else {
            hierarchy_weight_delta(
                baseline_hierarchy_weights,
                engine
                    .hierarchy
                    .state()
                    .profile_weights
                    .get(request.profile),
            )
        };
        if let Some(note) = runtime_error_note_from_trace(&draft.trace) {
            process_reward.notes.push(note);
        }
        let mut experience_process_reward = process_reward.clone();
        if let Some(note) = memory_feedback_note(&memory_feedback) {
            experience_process_reward.notes.push(note);
        }
        let live_evolution = LiveInferenceEvolution {
            router_threshold_delta: live_router_threshold_delta,
            hierarchy_weight_delta: live_hierarchy_weight_delta,
            online_reward_feedbacks,
            online_reward_reinforcements,
            online_reward_penalties,
            online_reward_strength,
            online_reward_reinforcement_strength,
            online_reward_penalty_strength,
            memory_reinforcements: memory_feedback.reinforced,
            memory_penalties: memory_feedback.penalized,
            stored_memory: stored_memory_id.is_some(),
            stored_gist_memories: stored_gist_memory_ids.len(),
            stored_runtime_kv_memories: stored_runtime_kv_memory_ids.len(),
            reflection_issues: report.issues.len(),
            critical_reflection_issues: report.critical_issue_count(),
            revision_actions: report.revision_actions.len(),
        };
        if defer_auto_replay && !discard_post_generation_state {
            auto_replay_report = engine.maybe_auto_replay(&request_scope);
        }
        let experience_id = engine.experience.record(ExperienceInput {
            prompt: request.prompt.clone(),
            profile: request.profile,
            lesson: report.lesson.clone(),
            quality: report.quality,
            contradictions: report.contradictions.clone(),
            reflection_issues: report.issues.clone(),
            revision_actions: report.revision_actions.clone(),
            stored_memory_id,
            router_threshold_after,
            stream_windows: stream_reports.len(),
            route_budget,
            hierarchy: execution_hierarchy,
            used_memory_ids: used_memories.iter().map(|memory| memory.id).collect(),
            gist_records: gist_records.clone(),
            gist_memory_ids: stored_gist_memory_ids.clone(),
            stored_runtime_kv_memory_ids: stored_runtime_kv_memory_ids.clone(),
            runtime_diagnostics: runtime_diagnostics.clone(),
            runtime_token_metrics: runtime_token_metrics.into(),
            process_reward: experience_process_reward,
            live_evolution,
        });
        engine
            .evolution_ledger
            .record_live_inference(live_evolution);
        let protected_memory_ids = protected_memory_ids(
            &used_memories,
            stored_memory_id,
            &stored_gist_memory_ids,
            &stored_runtime_kv_memory_ids,
        );
        let retention_protected_memory_ids = retention_protected_memory_ids(
            &used_memories,
            stored_memory_id,
            &stored_gist_memory_ids,
            &stored_runtime_kv_memory_ids,
        );
        let retention_report = engine.cache.apply_retention_with_protected(
            engine.memory_retention_policy,
            &retention_protected_memory_ids,
        );
        let memory_compaction_report = engine.cache.compact_similar_with_protected(
            engine.memory_compaction_policy.clone(),
            &protected_memory_ids,
        );
        if !drift_report.rollback_adaptive {
            let scoped_cache_entries = tenant_scope.map(|scope| engine.cache.entries_scoped(scope));
            let cache_entries = scoped_cache_entries
                .as_deref()
                .unwrap_or_else(|| engine.cache.entries());
            engine.last_tier_plan = engine.tiered_cache.plan(cache_entries, &used_memories);
        }
        let genome_evolution_preview = GenomeEvolutionPreview::new(
            request.profile,
            genome_generation_before,
            &reasoning_frame,
            genome.clone(),
            validated_plans.clone(),
            report.quality,
            process_reward.total,
            report.critical_issue_count(),
            report.contradictions.len(),
            &report.revised_answer,
            dna_evolution_controller.transaction_replay_passed,
            authorization.is_none()
                && reasoning_genome.read_only
                && !reasoning_genome.write_allowed
                && !reasoning_genome.applied
                && !dna_apply_receipt.applied,
        );

        InferenceOutcome {
            raw_answer: draft.answer.clone(),
            answer: report.revised_answer.clone(),
            report,
            auto_replay_report,
            metrics,
            runtime_token_metrics,
            embedding_diagnostics,
            runtime_diagnostics,
            runtime_adapter_observations,
            recursive_runtime_calls,
            homeostatic_gate,
            route_budget,
            adaptive_route_plan,
            compute_budget_schedule,
            task_hierarchy_plan,
            hierarchy: execution_hierarchy,
            tier_plan,
            tier_migrations,
            infini_memory_plan,
            recursive_schedule,
            hardware_plan,
            transformer_plan,
            toolsmith_plan,
            agent_team_plan,
            stream_reports,
            used_memories,
            memory_feedback,
            used_experiences,
            gist_records,
            stored_memory_id,
            stored_gist_memory_ids,
            exported_runtime_kv_blocks,
            stored_runtime_kv_memory_ids,
            memory_admission,
            drift_report,
            process_reward,
            genome_generation_before,
            genome_strategy,
            strategy_genome,
            pre_reasoning_genome,
            pre_reasoning_genome_chain,
            pre_reasoning_genome_splice,
            reasoning_frame,
            reasoning_frame_valid,
            task_gene_cascade,
            task_gene_review,
            task_skill_gene,
            reasoning_genome,
            reasoning_genome_chain,
            reasoning_genome_splice,
            gene_purpose_reviews,
            gene_scissors_journal,
            dna_evolution_controller,
            dna_writer_gate,
            dna_apply_plan,
            dna_apply_receipt,
            genome_evolution_preview,
            memory_retention_policy: engine.memory_retention_policy,
            memory_compaction_policy: engine.memory_compaction_policy.clone(),
            retention_report,
            memory_compaction_report,
            experience_id,
            router_threshold_after,
            live_evolution,
            evolution_ledger: engine.evolution_ledger,
        }
    }

    fn embed_for_backend<B: InferenceBackend>(&self, backend: &mut B, text: &str) -> EmbeddingCall {
        if let Some(vector) = backend.embed_text(text).filter(|vector| !vector.is_empty()) {
            return EmbeddingCall {
                diagnostics: EmbeddingCallDiagnostics {
                    source: EmbeddingSource::Runtime,
                    dimensions: vector.len(),
                },
                vector,
            };
        }

        let vector = self.embedder.embed(text);
        EmbeddingCall {
            diagnostics: EmbeddingCallDiagnostics {
                source: EmbeddingSource::Fallback,
                dimensions: vector.len(),
            },
            vector,
        }
    }

    fn scheduler_for_backend_window(
        &self,
        native_window_tokens: Option<usize>,
    ) -> RecursiveScheduler {
        let Some(native_window_tokens) = native_window_tokens.filter(|tokens| *tokens > 0) else {
            return self.recursive_scheduler.clone();
        };

        if native_window_tokens == self.recursive_scheduler.native_window_tokens() {
            return self.recursive_scheduler.clone();
        }

        RecursiveScheduler::new(
            native_window_tokens,
            self.recursive_scheduler
                .chunk_tokens()
                .min(native_window_tokens),
            self.recursive_scheduler.overlap_tokens(),
            self.recursive_scheduler.merge_fan_in(),
        )
    }
}

fn pressure_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn genome_fitness(genome: &ReasoningGenome) -> f32 {
    if genome.genes.is_empty() {
        return 0.0;
    }
    (genome
        .genes
        .iter()
        .map(|gene| gene.fitness.clamp(0.0, 1.0))
        .sum::<f32>()
        / genome.genes.len() as f32)
        .clamp(0.0, 1.0)
}

#[allow(clippy::too_many_arguments)]
fn dna_control_prompt(
    prompt: &str,
    genome: &ReasoningGenome,
    generation: u64,
    strategy: ReasoningGenomeStrategy,
    strategy_expression: &GenomeExpression,
    frame: &ReasoningFrame,
    frame_valid: bool,
    task_gene: &TaskGeneAdmissionReview,
    task_skill: &TaskSkillGeneCandidate,
    compute: &ComputeBudgetSchedule,
) -> String {
    let active = genome
        .genes
        .iter()
        .filter(|gene| {
            !matches!(
                gene.derived_status(),
                crate::reasoning_genome::ReasoningGeneStatus::Malignant
                    | crate::reasoning_genome::ReasoningGeneStatus::Quarantined
                    | crate::reasoning_genome::ReasoningGeneStatus::Regenerating
            )
        })
        .map(|gene| format!("{}:{}", gene.kind.as_str(), dna_label(&gene.label)))
        .collect::<Vec<_>>()
        .join("|");
    format!(
        "[noiron-dna genome={} generation={} strategy={} strategy_genes={} frame={} frame_valid={} vm_opcodes={} routing_bias={} memory_policy={} active={} task_gene={} task_skill={} threshold={:.3} compute={} saved_tokens={}]\n{}",
        genome.id,
        generation,
        strategy.as_str(),
        strategy_expression.active_gene_count(),
        frame.frame_id,
        frame_valid,
        frame.executed_opcodes.len(),
        frame.routing_bias_evidence_value(),
        frame.memory_policy_evidence_value(),
        active,
        task_gene.decision.as_str(),
        task_skill.decision.as_str(),
        compute.threshold_after,
        compute.compute_budget.as_str(),
        compute.saved_tokens,
        prompt
    )
}

fn bounded_milli(value: f32) -> u16 {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as u16
    } else {
        0
    }
}

fn dna_label(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_')
        })
        .take(48)
        .map(|character| if character == ' ' { '_' } else { character })
        .collect()
}

fn validated_genome_plans(
    genome: &ReasoningGenome,
    expression: &GenomeExpression,
    scope: &TenantScope,
    authorization: Option<&GenomeEvolutionAuthorization>,
) -> (Vec<MutationPlan>, Vec<GenePurposeRelabelProposal>) {
    if authorization.is_some_and(GenomeEvolutionAuthorization::rollback_requested) {
        let Some(target) = genome.genes.first() else {
            return (Vec::new(), Vec::new());
        };
        let mut plan = MutationPlan::preview(
            format!("mutation:{}:rollback", target.id),
            GeneScissorsIntent::Rollback,
            target.id.clone(),
            "explicitly authorized genome rollback",
            "restore the previous persisted genome snapshot",
            genome.stable_anchor_id.clone(),
        );
        if authorization.is_some_and(GenomeEvolutionAuthorization::is_valid) {
            plan.validation_status = GeneValidationStatus::Passed;
        }
        return (vec![plan], Vec::new());
    }

    let mut plans = expression.mutation_plans.clone();
    let mut reviews = Vec::new();
    for plan in &mut plans {
        if matches!(
            plan.intent,
            GeneScissorsIntent::Relabel | GeneScissorsIntent::Repair
        ) && let Some(gene) = genome
            .genes
            .iter()
            .find(|gene| gene.id == plan.target_gene_id)
        {
            let current = GenePurposeRecord::from_reasoning_gene(
                genome.profile,
                scope.lineage_tenant_scope(),
                GenePurposeEvidenceClass::Reflection,
                genome.stable_anchor_id.clone(),
                gene,
            );
            let evidence = GenePurposeRelabelEvidence::new(
                GenePurposeEvidenceClass::Reflection,
                crate::privacy_redaction::stable_redaction_digest([
                    "genome-purpose-feedback",
                    scope.scope_digest().as_str(),
                    plan.id.as_str(),
                ]),
                "bounded reflection feedback",
                plan.proposed_label.as_deref().unwrap_or(&gene.label),
                plan.proposed_purpose.as_deref().unwrap_or(&gene.purpose),
                genome.stable_anchor_id.clone(),
            )
            .with_tags(plan.proposed_tags.iter());
            let review = GenePurposeRelabelValidator::default().validate(&current, &evidence);
            if review.decision == GenePurposeRelabelDecision::Quarantined {
                plan.validation_status = GeneValidationStatus::Failed;
            }
            reviews.push(review);
        }
        if authorization.is_some_and(GenomeEvolutionAuthorization::is_valid)
            && plan.validation_status != GeneValidationStatus::Failed
        {
            plan.validation_status = GeneValidationStatus::Passed;
        }
    }
    (plans, reviews)
}

fn lookup_request_memories(
    cache: &crate::kv_cache::KvFusionCache,
    tenant_scope: Option<&TenantScope>,
    query: &[f32],
    limit: usize,
) -> Vec<MemoryMatch> {
    match tenant_scope {
        Some(scope) => cache.lookup_scoped(scope, query, limit),
        None => cache.lookup(query, limit),
    }
}

fn lookup_request_experiences(
    experience: &crate::experience::ExperienceStore,
    scoped_cache_entries: Option<&[MemoryEntry]>,
    prompt: &str,
    profile: crate::hierarchy::TaskProfile,
    limit: usize,
) -> Vec<crate::experience::ExperienceMatch> {
    match scoped_cache_entries {
        Some(entries) => {
            let visible_memory_ids = entries.iter().map(|entry| entry.id).collect::<Vec<_>>();
            experience
                .retrieval_report_matching(prompt, profile, limit, |record| {
                    experience_record_has_visible_memory(record, &visible_memory_ids)
                })
                .matches
        }
        None => experience.retrieve_lessons(prompt, profile, limit),
    }
}

fn experience_record_has_visible_memory(
    record: &ExperienceRecord,
    visible_memory_ids: &[u64],
) -> bool {
    record
        .stored_memory_id
        .is_some_and(|id| visible_memory_ids.contains(&id))
        || record
            .used_memory_ids
            .iter()
            .chain(record.gist_memory_ids.iter())
            .chain(record.stored_runtime_kv_memory_ids.iter())
            .any(|id| visible_memory_ids.contains(id))
}

fn store_request_memory(
    cache: &mut crate::kv_cache::KvFusionCache,
    tenant_scope: Option<&TenantScope>,
    lane: TenantResourceLane,
    key: String,
    vector: Vec<f32>,
    usefulness: f32,
) -> u64 {
    match tenant_scope {
        Some(scope) => cache.store_scoped_or_fuse(scope, lane, key, vector, usefulness),
        None => cache.store_or_fuse(key, vector, usefulness),
    }
}

fn reinforced_kv_fusion_plan_from_runtime_evidence(
    admission: &MemoryAdmissionPreview,
    splice: &DnaSplicePreview,
    process_reward: f32,
) -> ReinforcedKvFusionPlan {
    let mut candidates = admission
        .candidates
        .iter()
        .map(fusion_candidate_from_admission)
        .collect::<Vec<_>>();

    if admission.candidates.iter().any(|candidate| {
        candidate.kind == crate::memory_admission::MemoryAdmissionKind::GeneSegmentKvEvidence
    }) {
        return ReinforcedKvFusionPlan::from_candidates(
            ReinforcedKvFusionPolicy::default(),
            candidates,
        );
    }

    for (index, classified) in splice.segments.iter().enumerate() {
        let segment = &classified.segment;
        let source = fusion_source_from_gene_source(segment.source);
        let reinforcement = fusion_reinforcement_from_disposition(
            classified.disposition,
            process_reward,
            segment.fitness,
        );
        let contradictory = classified.disposition == GeneSegmentDisposition::Quarantined
            || classified
                .reasons
                .iter()
                .any(|reason| reason.contains("contradiction"));
        let required_anchor =
            segment.source == GeneSegmentSource::Prompt && segment.start_token == 0;
        candidates.push(
            ReinforcedKvFusionCandidate::new(
                format!("splice:{}:{index}", source.as_str()),
                source,
                segment.token_count().max(1),
            )
            .with_scores(
                segment_trust_score(segment),
                segment_recency(segment.kv_residency, segment.age),
                segment.fitness,
                fusion_task_relevance_from_disposition(classified.disposition, segment.source),
                reinforcement,
            )
            .with_privacy(fusion_privacy_from_segment(segment))
            .with_rollback_anchor(splice.stable_anchor_id.clone())
            .with_source_hash(if segment.source_hash.is_empty() {
                format!("segment:{}:{index}", source.as_str())
            } else {
                segment.source_hash.clone()
            })
            .with_contradictory(contradictory)
            .with_required_anchor(required_anchor),
        );
    }

    ReinforcedKvFusionPlan::from_candidates(ReinforcedKvFusionPolicy::default(), candidates)
}

fn gene_segment_kv_admission_records_from_splice(
    splice: &DnaSplicePreview,
    session_scope: &str,
) -> Vec<GeneSegmentKvAdmissionRecord> {
    splice
        .segments
        .iter()
        .map(|classified| {
            let segment = &classified.segment;
            GeneSegmentKvAdmissionRecord::new(
                &segment.id,
                segment.profile,
                segment.source.as_str(),
                &segment.source_hash,
                &segment.tenant_scope,
                session_scope,
                &splice.stable_anchor_id,
                segment.token_count(),
                segment.fitness,
                segment.schema_valid,
                segment.kv_shape_valid,
            )
            .with_quarantined(classified.disposition == GeneSegmentDisposition::Quarantined)
        })
        .collect()
}

fn fusion_source_from_gene_source(source: GeneSegmentSource) -> ReinforcedKvFusionSource {
    match source {
        GeneSegmentSource::Prompt | GeneSegmentSource::GenomeLedger => {
            ReinforcedKvFusionSource::GenomeSegment
        }
        GeneSegmentSource::SemanticMemory => ReinforcedKvFusionSource::SemanticMemory,
        GeneSegmentSource::GistMemory => ReinforcedKvFusionSource::GistMemory,
        GeneSegmentSource::RuntimeKv => ReinforcedKvFusionSource::RuntimeKv,
        GeneSegmentSource::ToolOutput => ReinforcedKvFusionSource::ColdEvidence,
    }
}

fn fusion_reinforcement_from_disposition(
    disposition: GeneSegmentDisposition,
    process_reward: f32,
    fitness: f32,
) -> f32 {
    let reward = process_reward.clamp(0.0, 1.0);
    match disposition {
        GeneSegmentDisposition::Retained => (reward * 0.60 + fitness * 0.40).clamp(0.0, 1.0),
        GeneSegmentDisposition::RepairCandidate => 0.05,
        GeneSegmentDisposition::Skipped => -0.25,
        GeneSegmentDisposition::Quarantined => -0.85,
    }
}

fn fusion_task_relevance_from_disposition(
    disposition: GeneSegmentDisposition,
    source: GeneSegmentSource,
) -> f32 {
    let source_relevance: f32 = match source {
        GeneSegmentSource::Prompt => 0.96,
        GeneSegmentSource::SemanticMemory => 0.82,
        GeneSegmentSource::GistMemory => 0.78,
        GeneSegmentSource::RuntimeKv => 0.84,
        GeneSegmentSource::GenomeLedger => 0.86,
        GeneSegmentSource::ToolOutput => 0.62,
    };
    let disposition_bonus: f32 = match disposition {
        GeneSegmentDisposition::Retained => 0.08,
        GeneSegmentDisposition::RepairCandidate => 0.02,
        GeneSegmentDisposition::Skipped | GeneSegmentDisposition::Quarantined => 0.0,
    };
    (source_relevance + disposition_bonus).clamp(0.0, 1.0)
}

fn fusion_privacy_from_segment(segment: &GeneSegment) -> MemoryPrivacyClassification {
    if segment.privacy_risk >= 0.50 {
        MemoryPrivacyClassification::SensitiveBlocked
    } else if segment.source == GeneSegmentSource::ToolOutput {
        MemoryPrivacyClassification::PublicSafe
    } else {
        MemoryPrivacyClassification::DigestOnly
    }
}

fn adaptive_route_plan_from_runtime_evidence(
    profile: crate::hierarchy::TaskProfile,
    threshold: f32,
    routing_context: RoutingContext,
    compute_budget: ComputeBudgetContext,
    reasoning_genome: &GenomeExpression,
    splice: &DnaSplicePreview,
    process_reward: f32,
    runtime_kv_segment_yield: Option<f32>,
    runtime_kv_budget_pressure: f32,
) -> (AdaptiveRoutingPlan, ComputeBudgetSchedule) {
    let mut candidates = Vec::new();

    for (index, classified) in splice.segments.iter().enumerate() {
        let segment = &classified.segment;
        let source = adaptive_route_source_from_gene_source(segment.source);
        let estimated_tokens = segment.token_count().max(1);
        let trust = segment_trust_score(segment);
        let components = route_components_with_runtime_kv_feedback(
            segment.source,
            AdaptiveRouteScoreComponents::new(
                segment_task_intent(profile, segment.source, classified.disposition),
                profile_language_mode(profile),
                profile_code_mode(profile),
                segment.fitness,
                segment_recency(segment.kv_residency, segment.age),
                trust,
                segment_compute_cost(estimated_tokens, source),
                (process_reward + segment.fitness * 0.5).clamp(0.0, 1.0),
            ),
            runtime_kv_segment_yield,
            runtime_kv_budget_pressure,
        );
        let anchor_required =
            segment.source == GeneSegmentSource::Prompt && segment.start_token == 0;
        candidates.push(
            AdaptiveRouteCandidate::new(
                format!("segment:{}:{index}", source.as_str()),
                source,
                estimated_tokens,
                components,
            )
            .with_anchor_required(anchor_required),
        );
    }

    for (index, record) in reasoning_genome.lifecycle_records.iter().enumerate() {
        let components = AdaptiveRouteScoreComponents::new(
            0.72,
            profile_language_mode(profile),
            profile_code_mode(profile),
            record.fitness_score,
            (1.0 - record.decay_score).clamp(0.0, 1.0),
            (1.0 - record.drift_score).clamp(0.0, 1.0),
            0.05,
            process_reward,
        );
        candidates.push(
            AdaptiveRouteCandidate::new(
                format!("gene:record:{index}"),
                AdaptiveRouteSource::ReasoningGenome,
                1,
                components,
            )
            .with_anchor_required(record.action.as_str() == "keep"),
        );
    }

    let budgeted = AdaptiveRoutingPlanner::new().plan_with_compute_budget(
        profile,
        threshold,
        routing_context,
        compute_budget,
        candidates,
    );
    (budgeted.routing_plan, budgeted.schedule)
}

fn adaptive_route_source_from_gene_source(source: GeneSegmentSource) -> AdaptiveRouteSource {
    match source {
        GeneSegmentSource::Prompt => AdaptiveRouteSource::PromptChunk,
        GeneSegmentSource::SemanticMemory => AdaptiveRouteSource::SemanticMemory,
        GeneSegmentSource::GistMemory => AdaptiveRouteSource::GistMemory,
        GeneSegmentSource::RuntimeKv => AdaptiveRouteSource::RuntimeKv,
        GeneSegmentSource::GenomeLedger => AdaptiveRouteSource::ReasoningGenome,
        GeneSegmentSource::ToolOutput => AdaptiveRouteSource::ToolOutput,
    }
}

fn segment_task_intent(
    profile: crate::hierarchy::TaskProfile,
    source: GeneSegmentSource,
    disposition: crate::reasoning_genome::GeneSegmentDisposition,
) -> f32 {
    let source_score: f32 = match source {
        GeneSegmentSource::Prompt => 0.92,
        GeneSegmentSource::SemanticMemory => 0.78,
        GeneSegmentSource::GistMemory => 0.74,
        GeneSegmentSource::RuntimeKv => 0.66,
        GeneSegmentSource::GenomeLedger => 0.82,
        GeneSegmentSource::ToolOutput => 0.70,
    };
    let profile_bonus: f32 = match profile {
        crate::hierarchy::TaskProfile::Coding => 0.05,
        crate::hierarchy::TaskProfile::Writing => 0.04,
        crate::hierarchy::TaskProfile::LongDocument => 0.08,
        crate::hierarchy::TaskProfile::General => 0.0,
    };
    let disposition_bonus: f32 = match disposition {
        crate::reasoning_genome::GeneSegmentDisposition::Retained => 0.06,
        crate::reasoning_genome::GeneSegmentDisposition::RepairCandidate => 0.02,
        crate::reasoning_genome::GeneSegmentDisposition::Skipped
        | crate::reasoning_genome::GeneSegmentDisposition::Quarantined => 0.0,
    };
    (source_score + profile_bonus + disposition_bonus).clamp(0.0, 1.0)
}

fn profile_language_mode(profile: crate::hierarchy::TaskProfile) -> f32 {
    match profile {
        crate::hierarchy::TaskProfile::Writing | crate::hierarchy::TaskProfile::LongDocument => {
            0.88
        }
        crate::hierarchy::TaskProfile::Coding => 0.54,
        crate::hierarchy::TaskProfile::General => 0.62,
    }
}

fn profile_code_mode(profile: crate::hierarchy::TaskProfile) -> f32 {
    match profile {
        crate::hierarchy::TaskProfile::Coding => 0.92,
        crate::hierarchy::TaskProfile::LongDocument => 0.36,
        crate::hierarchy::TaskProfile::General | crate::hierarchy::TaskProfile::Writing => 0.22,
    }
}

fn segment_recency(kv_residency: GeneKvResidency, age: u32) -> f32 {
    let residency = match kv_residency {
        GeneKvResidency::Sink => 0.92,
        GeneKvResidency::HotRecent => 0.86,
        GeneKvResidency::PackedSynopsis => 0.70,
        GeneKvResidency::ColdEvidence => 0.46,
        GeneKvResidency::None => 0.28,
    };
    let age_discount = (age.min(12) as f32 / 12.0) * 0.30;
    (residency - age_discount).clamp(0.0, 1.0)
}

fn segment_trust_score(segment: &GeneSegment) -> f32 {
    let schema = if segment.schema_valid { 0.28 } else { 0.0 };
    let kv_shape = if segment.kv_shape_valid { 0.22 } else { 0.0 };
    let drift = (1.0 - segment.drift_score).clamp(0.0, 1.0) * 0.30;
    let privacy = (1.0 - segment.privacy_risk).clamp(0.0, 1.0) * 0.20;
    (schema + kv_shape + drift + privacy).clamp(0.0, 1.0)
}

fn segment_compute_cost(estimated_tokens: usize, source: AdaptiveRouteSource) -> f32 {
    let token_cost = (estimated_tokens as f32 / 512.0).min(1.0);
    let source_cost: f32 = match source {
        AdaptiveRouteSource::PromptChunk => 0.18,
        AdaptiveRouteSource::SemanticMemory => 0.32,
        AdaptiveRouteSource::GistMemory => 0.20,
        AdaptiveRouteSource::RuntimeKv => 0.62,
        AdaptiveRouteSource::ReasoningGenome => 0.12,
        AdaptiveRouteSource::ToolOutput => 0.40,
    };
    (token_cost * 0.70 + source_cost * 0.30).clamp(0.0, 1.0)
}

fn route_components_with_runtime_kv_feedback(
    source: GeneSegmentSource,
    components: AdaptiveRouteScoreComponents,
    runtime_kv_segment_yield: Option<f32>,
    runtime_kv_budget_pressure: f32,
) -> AdaptiveRouteScoreComponents {
    if source != GeneSegmentSource::RuntimeKv {
        return components;
    }

    let segment_yield = runtime_kv_segment_yield
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 1.0))
        .unwrap_or(1.0);
    let budget_pressure = if runtime_kv_budget_pressure.is_finite() {
        runtime_kv_budget_pressure.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let waste = 1.0 - segment_yield;
    let usefulness = 0.20 + segment_yield * 0.80;
    let recency_factor = 0.35 + segment_yield * 0.65;
    let reward_factor = 0.15 + segment_yield * 0.85;
    let budget_usefulness = 1.0 - budget_pressure * 0.18;
    let budget_reward_factor = 1.0 - budget_pressure * 0.20;

    AdaptiveRouteScoreComponents::new(
        components.task_intent * usefulness * budget_usefulness,
        components.language_mode,
        components.code_mode,
        components.memory_fitness * usefulness * budget_usefulness,
        components.recency * recency_factor,
        components.trust * usefulness * budget_usefulness,
        (components.compute_cost + waste * 0.30 + budget_pressure * 0.20).clamp(0.0, 1.0),
        components.reward_history * reward_factor * budget_reward_factor,
    )
}

fn runtime_kv_budget_pressure(
    exported_runtime_kv_blocks: usize,
    budget_limited_runtime_kv_imports_skipped: usize,
) -> f32 {
    let total =
        exported_runtime_kv_blocks.saturating_add(budget_limited_runtime_kv_imports_skipped);
    if total == 0 {
        return 0.0;
    }

    (budget_limited_runtime_kv_imports_skipped as f32 / total as f32).clamp(0.0, 1.0)
}

fn reasoning_genome_splice_preview(
    profile: crate::hierarchy::TaskProfile,
    recursive_schedule: &RecursiveSchedule,
    used_memories: &[MemoryMatch],
    gist_records: &[GistRecord],
    exported_runtime_kv_blocks: usize,
    quality: f32,
    drift_rollback: bool,
    penalize_used_memory: bool,
    allow_runtime_kv_write: bool,
    runtime_kv_hold: bool,
    stable_anchor_id: String,
    tenant_scope: &TenantScope,
) -> DnaSplicePreview {
    let mut segments = Vec::new();
    let tenant_lineage = tenant_scope.lineage_tenant_scope();
    let prompt_source_hash = format!(
        "prompt:{}:tokens={}",
        profile_slug(profile),
        recursive_schedule.prompt_tokens
    );

    for chunk in &recursive_schedule.chunks {
        let drift_score = if drift_rollback { 0.72 } else { 0.04 };
        let kv_residency = if chunk.index == 0 {
            GeneKvResidency::Sink
        } else {
            GeneKvResidency::HotRecent
        };
        segments.push(
            GeneSegment::new(
                format!("segment:prompt:{}", chunk.index),
                profile,
                GeneSegmentSource::Prompt,
                chunk.start_token,
                chunk.end_token,
            )
            .with_source_hash(prompt_source_hash.clone())
            .with_metadata(
                format!("prompt chunk {}", chunk.index),
                "bounded prompt context for splice preview",
                format!("estimated_tokens={}", chunk.estimated_tokens),
            )
            .with_kv_residency(kv_residency)
            .with_health(quality, drift_score, 0.0),
        );
    }

    for memory in used_memories {
        let drift_score = if penalize_used_memory { 0.42 } else { 0.05 };
        segments.push(
            GeneSegment::new(
                format!("segment:memory:{}", memory.id),
                profile,
                GeneSegmentSource::SemanticMemory,
                0,
                1,
            )
            .with_source_hash(format!("memory:{}", memory.id))
            .with_metadata(
                format!("memory {}", memory.id),
                "retrieved semantic memory evidence",
                format!("similarity={:.3}", memory.similarity),
            )
            .with_kv_residency(GeneKvResidency::ColdEvidence)
            .with_health(memory.strength, drift_score, 0.0),
        );
    }

    for (index, gist) in gist_records.iter().enumerate() {
        segments.push(
            GeneSegment::new(
                format!("segment:gist:{index}"),
                profile,
                GeneSegmentSource::GistMemory,
                0,
                gist.source_tokens.max(1),
            )
            .with_source_hash(format!(
                "gist:{index}:{}:{}",
                gist.level.as_str(),
                gist.source_tokens
            ))
            .with_metadata(
                format!("{} gist", gist.level.as_str()),
                "candidate gist memory evidence",
                format!("importance={:.3}", gist.importance),
            )
            .with_kv_residency(GeneKvResidency::PackedSynopsis)
            .with_health(gist.importance, 0.04, 0.0),
        );
    }

    if exported_runtime_kv_blocks > 0 {
        let drift_score = if runtime_kv_hold || !allow_runtime_kv_write {
            0.72
        } else {
            0.05
        };
        let privacy_risk = if !allow_runtime_kv_write { 0.24 } else { 0.0 };
        segments.push(
            GeneSegment::new(
                "segment:runtime-kv",
                profile,
                GeneSegmentSource::RuntimeKv,
                0,
                exported_runtime_kv_blocks,
            )
            .with_source_hash(format!("runtime_kv:exported={exported_runtime_kv_blocks}"))
            .with_metadata(
                "runtime KV export",
                "runtime-generated KV evidence awaiting admission gates",
                format!("exported_blocks={exported_runtime_kv_blocks}"),
            )
            .with_kv_residency(if runtime_kv_hold {
                GeneKvResidency::ColdEvidence
            } else {
                GeneKvResidency::HotRecent
            })
            .with_health((quality * 0.86).clamp(0.0, 1.0), drift_score, privacy_risk),
        );
    }

    let mut preview = DnaSplicer::default().preview(profile, stable_anchor_id, segments);
    for classified in &mut preview.segments {
        classified.segment.tenant_scope.clone_from(&tenant_lineage);
    }
    preview
}

struct RustAnswerValidation {
    passed: bool,
    diagnostic: String,
}

fn validate_rust_answer(answer: &str) -> Option<RustAnswerValidation> {
    let code = extract_rust_code(answer)?;
    let validator = crate::rust_validation::RustSnippetValidator::new(
        std::env::temp_dir().join("rust-norion-runtime-rust-check"),
    );
    let check =
        crate::rust_validation::RustSnippetCheck::new(code).with_case_name("inference-answer");
    let validation = match validator.check(&check) {
        Ok(report) => {
            let diagnostic = if report.stderr.trim().is_empty() {
                report.stdout.clone()
            } else {
                report.stderr.clone()
            };
            if let Some(case_dir) = report.source_path.parent() {
                let _ = std::fs::remove_dir_all(case_dir);
            }
            RustAnswerValidation {
                passed: report.passed,
                diagnostic,
            }
        }
        Err(error) => RustAnswerValidation {
            passed: false,
            diagnostic: format!("rust validation unavailable: {error}"),
        },
    };
    Some(validation)
}

fn extract_rust_code(answer: &str) -> Option<String> {
    let fenced = answer
        .split("```")
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .filter_map(|(_, block)| {
            let block = block.trim();
            let code = block
                .strip_prefix("rust")
                .or_else(|| block.strip_prefix("rs"))
                .unwrap_or(block)
                .trim();
            looks_like_rust(code).then(|| code.to_owned())
        })
        .next();
    if fenced.is_some() {
        return fenced;
    }

    let lines = answer.lines().collect::<Vec<_>>();
    let start = lines.iter().position(|line| looks_like_rust(line.trim()))?;
    let mut code = String::new();
    let mut brace_depth = 0isize;
    let mut opened = false;
    for line in lines.into_iter().skip(start) {
        let trimmed = line.trim();
        if opened && brace_depth == 0 && !trimmed.is_empty() {
            break;
        }
        code.push_str(line);
        code.push('\n');
        for character in line.chars() {
            match character {
                '{' => {
                    brace_depth += 1;
                    opened = true;
                }
                '}' => brace_depth -= 1,
                _ => {}
            }
        }
        if opened && brace_depth == 0 {
            break;
        }
    }
    (!code.trim().is_empty()).then(|| code.trim().to_owned())
}

fn looks_like_rust(value: &str) -> bool {
    [
        "fn ", "pub fn ", "struct ", "enum ", "impl ", "trait ", "mod ",
    ]
    .iter()
    .any(|marker| value.starts_with(marker) || value.contains(&format!("\n{marker}")))
}

fn apply_memory_grounding_gate(
    prompt: &str,
    used_memories: &[MemoryMatch],
    report: &mut ReflectionReport,
) {
    if used_memories.is_empty() || !looks_like_fact_recall(prompt) {
        return;
    }

    let prompt_numbers = numeric_anchors(prompt);
    let Some(memory) = used_memories.iter().max_by(|left, right| {
        left.similarity
            .partial_cmp(&right.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) else {
        return;
    };
    if memory.similarity < 0.35 {
        return;
    }
    let source_prompt = memory
        .key
        .split_once(" :: ")
        .map_or(memory.key.as_str(), |item| item.0);
    let evidence_numbers = numeric_anchors(source_prompt);
    let required = evidence_numbers
        .difference(&prompt_numbers)
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if required.is_empty() {
        return;
    }

    let answer_numbers = numeric_anchors(&report.revised_answer);
    let missing = required
        .difference(&answer_numbers)
        .cloned()
        .collect::<Vec<_>>();
    let allowed = prompt_numbers
        .union(&evidence_numbers)
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let novel = answer_numbers
        .difference(&allowed)
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() && novel.is_empty() {
        return;
    }

    apply_critical_validation_issue(
        report,
        "memory_grounding_contradiction",
        "reject_ungrounded_memory_recall",
        format!(
            "retrieved fact anchors missing={} novel={}",
            missing.join("|"),
            novel.join("|")
        ),
    );
    if let Some(grounded_answer) = used_memories
        .iter()
        .filter_map(|memory| {
            let source = memory
                .key
                .split_once(" :: ")
                .map_or(memory.key.as_str(), |item| item.0);
            grounded_fact_answer(source, prompt)
        })
        .max_by_key(String::len)
    {
        report.revised_answer = grounded_answer;
    }
}

fn looks_like_fact_recall(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    [
        "是什么",
        "多少",
        "门槛",
        "复述",
        "回忆",
        "what is",
        "recall",
        "remember",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn numeric_anchors(value: &str) -> std::collections::BTreeSet<String> {
    value
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '.' | '%' | '-' | '_'))
        })
        .map(|token| token.trim_matches(|character: char| matches!(character, '-' | '_' | '.')))
        .filter(|token| token.chars().any(|character| character.is_ascii_digit()))
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn grounded_fact_answer(source_prompt: &str, prompt: &str) -> Option<String> {
    if prompt.contains("门槛") {
        let fact = source_prompt
            .split_once("门槛是")?
            .1
            .split("::")
            .next()?
            .split("然后")
            .next()?;
        let fact = fact
            .trim_matches('_')
            .split('_')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        return (!fact.is_empty()).then_some(fact);
    }
    None
}

fn apply_critical_validation_issue(
    report: &mut ReflectionReport,
    code: &str,
    action: &str,
    detail: String,
) {
    if !report.issues.iter().any(|issue| issue.code == code) {
        report.issues.push(ReflectionIssue::new(
            code,
            ReflectionSeverity::Critical,
            detail,
        ));
    }
    if !report.contradictions.iter().any(|item| item == code) {
        report.contradictions.push(code.to_owned());
    }
    if !report.revision_actions.iter().any(|item| item == action) {
        report.revision_actions.push(action.to_owned());
    }
    report.quality = report.quality.min(0.20);
    report.store_as_memory = false;
    report.lesson = format!(
        "rejected_by_validation code={code} action={action} quality={:.3}",
        report.quality
    );
    report.revised_answer = format!(
        "{}\n\nValidation failed: {code}. This answer was not stored or reinforced.",
        report.revised_answer.trim()
    );
}

fn apply_unverified_behavior_gate(report: &mut ReflectionReport) {
    const CODE: &str = "generated_code_behavior_unverified";
    const ACTION: &str = "require_executable_behavior_validation";
    if !report.issues.iter().any(|issue| issue.code == CODE) {
        report.issues.push(ReflectionIssue::new(
            CODE,
            ReflectionSeverity::Critical,
            "generated code has no executable behavior validation evidence",
        ));
    }
    if !report.contradictions.iter().any(|item| item == CODE) {
        report.contradictions.push(CODE.to_owned());
    }
    if !report.revision_actions.iter().any(|item| item == ACTION) {
        report.revision_actions.push(ACTION.to_owned());
    }
    report.quality = report.quality.min(0.20);
    report.store_as_memory = false;
    report.lesson = format!(
        "behavior_unverified code={CODE} action={ACTION} quality={:.3}",
        report.quality
    );
}

fn compact_diagnostic(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

fn profile_slug(profile: crate::hierarchy::TaskProfile) -> &'static str {
    match profile {
        crate::hierarchy::TaskProfile::General => "general",
        crate::hierarchy::TaskProfile::Coding => "coding",
        crate::hierarchy::TaskProfile::Writing => "writing",
        crate::hierarchy::TaskProfile::LongDocument => "long_document",
    }
}
