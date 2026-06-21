use crate::adaptive_state::LiveInferenceEvolution;
use crate::agent_team::AgentTeamInput;
use crate::drift::DriftInput;
use crate::experience::ExperienceInput;
use crate::gist_memory::GistRecord;
use crate::kv_cache::MemoryMatch;
use crate::memory_admission::{MemoryAdmissionInput, MemoryAdmissionPreview};
use crate::process_reward::{ProcessRewardInput, RewardAction};
use crate::reasoning_genome::{
    DnaSplicePreview, DnaSplicer, GeneKvResidency, GeneSegment, GeneSegmentSource,
    GenomeExpressionInput, ReasoningGenome,
};
use crate::recursive_scheduler::{RecursiveSchedule, RecursiveScheduler};
use crate::reflection::DraftToken;
use crate::router::RoutingContext;
use crate::runtime::RuntimeAdapterObservation;
use crate::toolsmith::ToolsmithInput;

use super::NoironEngine;
use super::memory_keys::{
    format_gist_key, format_runtime_kv_key, protected_memory_ids, summarize_key,
};
use super::metrics::{hierarchy_weight_delta, metrics_from_report, runtime_error_note_from_trace};
use super::recursive::{generate_with_recursive_schedule, generate_with_recursive_schedule_stream};
use super::replay_feedback::*;
use super::types::{
    EmbeddingCall, EmbeddingCallDiagnostics, EmbeddingDiagnostics, EmbeddingSource,
    GenerationContext, InferenceBackend, InferenceOutcome, InferenceRequest, MemoryFeedbackReport,
    RuntimeTokenMetrics,
};

impl NoironEngine {
    pub fn infer<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
    ) -> InferenceOutcome {
        self.infer_with_stream_observer(request, backend, None)
    }

    pub fn infer_stream<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
        on_token: &mut dyn FnMut(&DraftToken),
    ) -> InferenceOutcome {
        self.infer_with_stream_observer(request, backend, Some(on_token))
    }

    fn infer_with_stream_observer<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
        mut on_token: Option<&mut dyn FnMut(&DraftToken)>,
    ) -> InferenceOutcome {
        backend.configure_generation(request.max_tokens);
        let auto_replay_report = self.maybe_auto_replay();
        let adaptive_before_inference = self.adaptive_state();
        let query_embedding = self.embed_for_backend(backend, &request.prompt);
        let mut embedding_diagnostics =
            EmbeddingDiagnostics::from_query(query_embedding.diagnostics);
        let used_memories = self.cache.lookup(&query_embedding.vector, 4);
        let used_experiences =
            self.experience
                .retrieve_lessons(&request.prompt, request.profile, 3);
        let recursive_scheduler =
            self.scheduler_for_backend_window(backend.runtime_native_context_window());
        let recursive_schedule = recursive_scheduler.plan(&request.prompt);
        let base_hierarchy = self.hierarchy.adapt_to_profile(request.profile);
        let hardware_plan = self.hardware_allocator.plan(
            self.hardware_snapshot,
            request.profile,
            recursive_schedule.prompt_tokens,
            base_hierarchy,
        );
        let recursive_schedule =
            recursive_schedule.with_parallel_budget(hardware_plan.execution.max_parallel_chunks);
        let tier_plan = self.tiered_cache.plan(self.cache.entries(), &used_memories);
        let tier_migrations = tier_plan.migrations_from(&self.last_tier_plan);
        let infini_memory_planner = self.infini_memory_planner.clone().with_token_budgets(
            hardware_plan.local_kv_token_budget,
            hardware_plan.global_kv_token_budget,
        );
        let infini_memory_plan = infini_memory_planner.plan(self.cache.entries(), &used_memories);
        let routing_context = RoutingContext {
            profile: request.profile,
            context_tokens: recursive_schedule.prompt_tokens,
            cache_hit_rate: used_memories.len() as f32 / 4.0,
            latency_budget_ms: hardware_plan.latency_budget_ms,
            hardware_pressure: hardware_plan.pressure,
            compute_headroom: hardware_plan.compute_headroom(),
            hierarchy: hardware_plan.hierarchy,
        };
        let route_budget = self
            .router
            .budget_for_prompt_with_context(&request.prompt, routing_context);
        let hierarchy = hardware_plan.hierarchy;
        let transformer_plan =
            self.transformer_planner
                .plan(request.profile, hierarchy, route_budget);
        let toolsmith_plan = self.toolsmith_planner.plan(ToolsmithInput {
            prompt: &request.prompt,
            profile: request.profile,
            memories: &used_memories,
            experiences: &used_experiences,
            hardware_plan: &hardware_plan,
        });
        let agent_team_plan = self.agent_team_planner.plan(AgentTeamInput {
            prompt: &request.prompt,
            profile: request.profile,
            memories: &used_memories,
            experiences: &used_experiences,
            hardware_plan: &hardware_plan,
            route_budget,
            recursive_schedule: &recursive_schedule,
            toolsmith_plan: &toolsmith_plan,
        });

        let generation_context = GenerationContext {
            prompt: &request.prompt,
            profile: request.profile,
            memories: &used_memories,
            route_budget,
            hierarchy,
            tier_plan: &tier_plan,
            infini_memory_plan: &infini_memory_plan,
            recursive_schedule: &recursive_schedule,
            hardware_plan: &hardware_plan,
            experiences: &used_experiences,
            toolsmith_plan: &toolsmith_plan,
            agent_team_plan: &agent_team_plan,
            transformer_plan: &transformer_plan,
        };
        let (draft, recursive_runtime_calls) = if let Some(on_token) = on_token.as_mut() {
            generate_with_recursive_schedule_stream(backend, generation_context, *on_token)
        } else {
            generate_with_recursive_schedule(backend, generation_context)
        };
        let report = self.reflector.reflect(&request.prompt, &draft);
        let runtime_token_metrics = RuntimeTokenMetrics::from_draft(&draft);
        let runtime_diagnostics = draft.runtime_diagnostics.clone();
        let runtime_adapter_observations = RuntimeAdapterObservation::from_experiences_for_hardware(
            &used_experiences,
            runtime_diagnostics.model_id.as_deref().unwrap_or_default(),
            &hardware_plan,
        );
        let metrics = metrics_from_report(&draft, &report, route_budget, runtime_token_metrics);
        let gist_records =
            self.gist_generator
                .generate(&request.prompt, &report.revised_answer, report.quality);
        let stream_reports = self.stream_monitor.observe_draft_with_profile(
            &mut self.router,
            request.profile,
            &draft,
            report.quality,
            report.contradictions.len(),
        );
        let exported_runtime_kv_blocks = draft.exported_kv_blocks.len();
        let drift_report = self.drift_guard.evaluate(DriftInput {
            quality: report.quality,
            contradiction_count: report.contradictions.len(),
            metrics,
            route_budget,
            used_memories: used_memories.len(),
            exported_runtime_kv_blocks,
            stream_windows: stream_reports.len(),
        });
        let admit_memory = report.store_as_memory && drift_report.allow_memory_write;
        let admit_runtime_kv =
            admit_memory && drift_report.allow_runtime_kv_write && report.revision_passes == 0;

        let stored_memory_id = if admit_memory {
            let memory_text = format!(
                "prompt:{}\nanswer:{}\nlesson:{}",
                request.prompt.as_str(),
                report.revised_answer,
                report.lesson
            );
            let memory_embedding = self.embed_for_backend(backend, &memory_text);
            embedding_diagnostics.record_memory_write(memory_embedding.diagnostics);
            Some(self.cache.store_or_fuse(
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
                    let gist_embedding = self.embed_for_backend(backend, &memory_text);
                    embedding_diagnostics.record_gist_write(gist_embedding.diagnostics);
                    self.cache.store_or_fuse(
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
                    self.cache.store_or_fuse(
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

        let mut memory_feedback = MemoryFeedbackReport::default();
        for memory in &used_memories {
            if admit_memory && !drift_report.penalize_used_memory {
                let amount = used_memory_reinforcement_amount(&report);
                let update = self.cache.reinforce(memory.id, amount);
                memory_feedback.record_reinforcement(amount, update);
            } else {
                let amount = used_memory_penalty_amount(&report, &drift_report, metrics);
                let update = self.cache.penalize(memory.id, amount);
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
        self.router.observe_with_profile(request.profile, metrics);
        let mut hierarchy = self.hierarchy.observe(request.profile, metrics);
        if drift_report.rollback_adaptive {
            let rollback_router_threshold_delta =
                (self.router.threshold_for(request.profile) - baseline_router_threshold).abs();
            let rollback_hierarchy_weight_delta = hierarchy_weight_delta(
                baseline_hierarchy_weights,
                self.hierarchy.state().profile_weights.get(request.profile),
            );
            self.restore_adaptive_state(adaptive_before_inference);
            self.evolution_ledger.record_drift_rollback(
                rollback_router_threshold_delta,
                rollback_hierarchy_weight_delta,
            );
            hierarchy = self.hierarchy.current();
        }
        let mut process_reward = self.process_rewarder.score(ProcessRewardInput {
            profile: request.profile,
            route_budget,
            hierarchy,
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
            self.router
                .observe_with_profile(request.profile, reward_metrics);
            hierarchy = self.hierarchy.observe(request.profile, reward_metrics);
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
        let runtime_adapter_selection_mismatch = match (
            best_adapter_observation.map(|observation| observation.adapter.as_str()),
            runtime_diagnostics.selected_adapter.as_deref(),
        ) {
            (Some(best_adapter), Some(selected_adapter)) => best_adapter != selected_adapter,
            _ => false,
        };
        let memory_admission = MemoryAdmissionPreview::from_feedback(MemoryAdmissionInput {
            prompt: &request.prompt,
            profile: request.profile,
            report: &report,
            process_reward: &process_reward,
            drift_report: &drift_report,
            stored_memory: stored_memory_id.is_some(),
            gist_records: gist_records.len(),
            stored_gist_memories: stored_gist_memory_ids.len(),
            exported_runtime_kv_blocks,
            stored_runtime_kv_memories: stored_runtime_kv_memory_ids.len(),
            runtime_kv_hold,
            used_memories: used_memories.len(),
            memory_feedback_updates: memory_feedback.total_updates(),
            runtime_adapter_observations: runtime_adapter_observations.len(),
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
            drift_rollback: drift_report.rollback_adaptive,
            runtime_kv_hold,
        };
        let reasoning_genome = ReasoningGenome::default_for_profile(request.profile)
            .with_feedback_health(&genome_input)
            .express(genome_input);
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
        );

        let router_threshold_after = self.router.threshold();
        let live_router_threshold_delta = if drift_report.rollback_adaptive {
            0.0
        } else {
            (self.router.threshold_for(request.profile) - baseline_router_threshold).abs()
        };
        let live_hierarchy_weight_delta = if drift_report.rollback_adaptive {
            0.0
        } else {
            hierarchy_weight_delta(
                baseline_hierarchy_weights,
                self.hierarchy.state().profile_weights.get(request.profile),
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
        let experience_id = self.experience.record(ExperienceInput {
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
            hierarchy,
            used_memory_ids: used_memories.iter().map(|memory| memory.id).collect(),
            gist_records: gist_records.clone(),
            gist_memory_ids: stored_gist_memory_ids.clone(),
            stored_runtime_kv_memory_ids: stored_runtime_kv_memory_ids.clone(),
            runtime_diagnostics: runtime_diagnostics.clone(),
            runtime_token_metrics: runtime_token_metrics.into(),
            process_reward: experience_process_reward,
            live_evolution,
        });
        self.evolution_ledger.record_live_inference(live_evolution);
        let retention_report = self.cache.apply_retention(self.memory_retention_policy);
        let protected_memory_ids = protected_memory_ids(
            &used_memories,
            stored_memory_id,
            &stored_gist_memory_ids,
            &stored_runtime_kv_memory_ids,
        );
        let memory_compaction_report = self.cache.compact_similar_with_protected(
            self.memory_compaction_policy.clone(),
            &protected_memory_ids,
        );
        if !drift_report.rollback_adaptive {
            self.last_tier_plan = self.tiered_cache.plan(self.cache.entries(), &used_memories);
        }

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
            route_budget,
            hierarchy,
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
            reasoning_genome,
            reasoning_genome_splice,
            memory_retention_policy: self.memory_retention_policy,
            memory_compaction_policy: self.memory_compaction_policy.clone(),
            retention_report,
            memory_compaction_report,
            experience_id,
            router_threshold_after,
            live_evolution,
            evolution_ledger: self.evolution_ledger,
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
) -> DnaSplicePreview {
    let mut segments = Vec::new();
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

    DnaSplicer::default().preview(profile, stable_anchor_id, segments)
}

fn profile_slug(profile: crate::hierarchy::TaskProfile) -> &'static str {
    match profile {
        crate::hierarchy::TaskProfile::General => "general",
        crate::hierarchy::TaskProfile::Coding => "coding",
        crate::hierarchy::TaskProfile::Writing => "writing",
        crate::hierarchy::TaskProfile::LongDocument => "long_document",
    }
}
