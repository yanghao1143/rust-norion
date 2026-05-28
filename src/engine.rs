use std::io;
use std::path::Path;

use crate::adaptive_state::AdaptiveState;
use crate::drift::{DriftGuard, DriftInput, DriftReport};
use crate::experience::{ExperienceInput, ExperienceMatch, ExperienceStore};
use crate::experience_replay::{
    ExperienceReplayItem, ExperienceReplayPlanner, ExperienceReplayReport,
};
use crate::gist_memory::{GistGenerator, GistRecord};
use crate::hardware::{HardwareAllocator, HardwarePlan, HardwareSnapshot};
use crate::hierarchy::{HierarchyController, HierarchyWeights, TaskProfile};
use crate::infini_memory::{InfiniMemoryPlan, InfiniMemoryPlanner};
use crate::kv_cache::{
    KvFusionCache, MemoryCompactionPolicy, MemoryCompactionReport, MemoryMatch,
    MemoryRetentionPolicy, RetentionReport,
};
use crate::kv_exchange::RuntimeKvBlock;
use crate::process_reward::{
    ProcessRewardInput, ProcessRewardReport, ProcessRewarder, RewardAction,
};
use crate::recursive_scheduler::{RecursiveChunk, RecursiveSchedule, RecursiveScheduler};
use crate::reflection::{
    InferenceDraft, ReasoningStep, ReflectionReport, Reflector, RuntimeDiagnostics,
};
use crate::router::{GenerationMetrics, NoironRouter, RouteBudget, RoutingContext};
use crate::runtime::RuntimeAdapterObservation;
use crate::tiered_cache::{TierMigration, TieredCachePlan, TieredCacheScheduler};
use crate::token_stream::{TokenStreamMonitor, TokenWindowReport};
use crate::transformer::{TransformerPlanner, TransformerRefactorPlan};

#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub prompt: String,
    pub profile: TaskProfile,
}

impl InferenceRequest {
    pub fn new(prompt: impl Into<String>, profile: TaskProfile) -> Self {
        Self {
            prompt: prompt.into(),
            profile,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenerationContext<'a> {
    pub prompt: &'a str,
    pub profile: TaskProfile,
    pub memories: &'a [MemoryMatch],
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub tier_plan: &'a TieredCachePlan,
    pub infini_memory_plan: &'a InfiniMemoryPlan,
    pub recursive_schedule: &'a RecursiveSchedule,
    pub hardware_plan: &'a HardwarePlan,
    pub experiences: &'a [ExperienceMatch],
    pub transformer_plan: &'a TransformerRefactorPlan,
}

impl<'a> GenerationContext<'a> {
    fn with_prompt<'b>(&'b self, prompt: &'b str) -> GenerationContext<'b>
    where
        'a: 'b,
    {
        GenerationContext {
            prompt,
            profile: self.profile,
            memories: self.memories,
            route_budget: self.route_budget,
            hierarchy: self.hierarchy,
            tier_plan: self.tier_plan,
            infini_memory_plan: self.infini_memory_plan,
            recursive_schedule: self.recursive_schedule,
            hardware_plan: self.hardware_plan,
            experiences: self.experiences,
            transformer_plan: self.transformer_plan,
        }
    }
}

pub trait InferenceBackend {
    fn runtime_native_context_window(&self) -> Option<usize> {
        None
    }

    fn embed_text(&mut self, _text: &str) -> Option<Vec<f32>> {
        None
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft;
}

#[derive(Debug, Clone)]
pub struct InferenceOutcome {
    pub answer: String,
    pub report: ReflectionReport,
    pub auto_replay_report: Option<ExperienceReplayReport>,
    pub metrics: GenerationMetrics,
    pub runtime_token_metrics: RuntimeTokenMetrics,
    pub runtime_diagnostics: RuntimeDiagnostics,
    pub runtime_adapter_observations: Vec<RuntimeAdapterObservation>,
    pub recursive_runtime_calls: usize,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub tier_plan: TieredCachePlan,
    pub tier_migrations: Vec<TierMigration>,
    pub infini_memory_plan: InfiniMemoryPlan,
    pub recursive_schedule: RecursiveSchedule,
    pub hardware_plan: HardwarePlan,
    pub transformer_plan: TransformerRefactorPlan,
    pub stream_reports: Vec<TokenWindowReport>,
    pub used_memories: Vec<MemoryMatch>,
    pub used_experiences: Vec<ExperienceMatch>,
    pub gist_records: Vec<GistRecord>,
    pub stored_memory_id: Option<u64>,
    pub stored_gist_memory_ids: Vec<u64>,
    pub exported_runtime_kv_blocks: usize,
    pub stored_runtime_kv_memory_ids: Vec<u64>,
    pub drift_report: DriftReport,
    pub process_reward: ProcessRewardReport,
    pub memory_retention_policy: MemoryRetentionPolicy,
    pub memory_compaction_policy: MemoryCompactionPolicy,
    pub retention_report: RetentionReport,
    pub memory_compaction_report: MemoryCompactionReport,
    pub experience_id: u64,
    pub router_threshold_after: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeTokenMetrics {
    pub token_count: usize,
    pub entropy_count: usize,
    pub logprob_count: usize,
    pub average_entropy: Option<f32>,
    pub average_neg_logprob: Option<f32>,
    pub uncertainty_perplexity: Option<f32>,
}

impl RuntimeTokenMetrics {
    pub fn from_draft(draft: &InferenceDraft) -> Self {
        let mut entropy_total = 0.0;
        let mut entropy_count = 0;
        let mut neg_logprob_total = 0.0;
        let mut logprob_count = 0;
        let mut loss_total = 0.0;
        let mut loss_count = 0;

        for token in &draft.tokens {
            let entropy = token.entropy.and_then(bounded_entropy);
            let neg_logprob = token.logprob.and_then(bounded_neg_logprob);

            if let Some(entropy) = entropy {
                entropy_total += entropy;
                entropy_count += 1;
            }
            if let Some(neg_logprob) = neg_logprob {
                neg_logprob_total += neg_logprob;
                logprob_count += 1;
            }

            match (entropy, neg_logprob) {
                (Some(entropy), Some(neg_logprob)) => {
                    loss_total += 2.0 + entropy * 4.0 + neg_logprob;
                    loss_count += 1;
                }
                (Some(entropy), None) => {
                    loss_total += 2.0 + entropy * 4.0;
                    loss_count += 1;
                }
                (None, Some(neg_logprob)) => {
                    loss_total += 2.0 + neg_logprob;
                    loss_count += 1;
                }
                (None, None) => {}
            }
        }

        Self {
            token_count: draft.tokens.len(),
            entropy_count,
            logprob_count,
            average_entropy: average(entropy_total, entropy_count),
            average_neg_logprob: average(neg_logprob_total, logprob_count),
            uncertainty_perplexity: average(loss_total, loss_count),
        }
    }

    pub fn has_uncertainty_signal(self) -> bool {
        self.uncertainty_perplexity.is_some()
    }
}

fn bounded_entropy(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 4.0))
}

fn bounded_neg_logprob(value: f32) -> Option<f32> {
    let value = -value;
    value.is_finite().then(|| value.max(0.0).min(12.0))
}

#[derive(Debug, Clone)]
pub struct NoironEngine {
    pub router: NoironRouter,
    pub cache: KvFusionCache,
    pub hierarchy: HierarchyController,
    pub tiered_cache: TieredCacheScheduler,
    pub infini_memory_planner: InfiniMemoryPlanner,
    pub hardware_allocator: HardwareAllocator,
    pub hardware_snapshot: HardwareSnapshot,
    pub recursive_scheduler: RecursiveScheduler,
    pub stream_monitor: TokenStreamMonitor,
    pub transformer_planner: TransformerPlanner,
    pub experience: ExperienceStore,
    pub experience_replay_planner: ExperienceReplayPlanner,
    pub gist_generator: GistGenerator,
    pub process_rewarder: ProcessRewarder,
    pub drift_guard: DriftGuard,
    pub reflector: Reflector,
    pub auto_replay_limit: usize,
    pub memory_retention_policy: MemoryRetentionPolicy,
    pub memory_compaction_policy: MemoryCompactionPolicy,
    last_tier_plan: TieredCachePlan,
    embedder: TextEmbedder,
}

impl Default for NoironEngine {
    fn default() -> Self {
        Self {
            router: NoironRouter::new(),
            cache: KvFusionCache::new(),
            hierarchy: HierarchyController::new(),
            tiered_cache: TieredCacheScheduler::new(),
            infini_memory_planner: InfiniMemoryPlanner::new(),
            hardware_allocator: HardwareAllocator::new(),
            hardware_snapshot: HardwareSnapshot::default(),
            recursive_scheduler: RecursiveScheduler::default(),
            stream_monitor: TokenStreamMonitor::default(),
            transformer_planner: TransformerPlanner::default(),
            experience: ExperienceStore::new(),
            experience_replay_planner: ExperienceReplayPlanner::new(),
            gist_generator: GistGenerator::new(),
            process_rewarder: ProcessRewarder::new(),
            drift_guard: DriftGuard::new(),
            reflector: Reflector::new(),
            auto_replay_limit: 2,
            memory_retention_policy: MemoryRetentionPolicy::default(),
            memory_compaction_policy: MemoryCompactionPolicy::default(),
            last_tier_plan: TieredCachePlan::default(),
            embedder: TextEmbedder::default(),
        }
    }
}

impl NoironEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cache(cache: KvFusionCache) -> Self {
        Self {
            cache,
            ..Self::default()
        }
    }

    pub fn load_memory(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self::with_cache(KvFusionCache::load_persistent(path)?))
    }

    pub fn load_state(
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let mut engine = Self::load_memory(memory_path)?;
        engine.experience = ExperienceStore::load_from_disk_kv(experience_path)?;
        Ok(engine)
    }

    pub fn load_full_state(
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
        adaptive_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let mut engine = Self::load_state(memory_path, experience_path)?;
        if let Some(state) = AdaptiveState::load_from_disk_kv(adaptive_path)? {
            engine.restore_adaptive_state(state);
        }
        Ok(engine)
    }

    pub fn save_memory(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.cache.save_persistent(path)
    }

    pub fn save_experience(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.experience.save_to_disk_kv(path)
    }

    pub fn adaptive_state(&self) -> AdaptiveState {
        AdaptiveState {
            router: self.router.state(),
            hierarchy: self.hierarchy.state(),
            tier_plan: self.last_tier_plan.clone(),
            memory_retention_policy: self.memory_retention_policy,
            memory_compaction_policy: self.memory_compaction_policy.clone(),
        }
    }

    pub fn restore_adaptive_state(&mut self, state: AdaptiveState) {
        self.router.restore_state(state.router);
        self.hierarchy.restore_state(state.hierarchy);
        self.last_tier_plan = state.tier_plan;
        self.memory_retention_policy = state.memory_retention_policy;
        self.memory_compaction_policy = state.memory_compaction_policy;
    }

    pub fn save_adaptive_state(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.adaptive_state().save_to_disk_kv(path)
    }

    pub fn save_full_state(
        &self,
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
        adaptive_path: impl AsRef<Path>,
    ) -> io::Result<()> {
        self.save_memory(memory_path)?;
        self.save_experience(experience_path)?;
        self.save_adaptive_state(adaptive_path)
    }

    pub fn set_hardware_snapshot(&mut self, snapshot: HardwareSnapshot) {
        self.hardware_snapshot = snapshot;
    }

    pub fn set_auto_replay_limit(&mut self, limit: usize) {
        self.auto_replay_limit = limit;
    }

    pub fn set_memory_retention_policy(&mut self, policy: MemoryRetentionPolicy) {
        self.memory_retention_policy = policy;
    }

    pub fn set_memory_compaction_policy(&mut self, policy: MemoryCompactionPolicy) {
        self.memory_compaction_policy = policy;
    }

    pub fn replay_experience(&mut self, limit: usize) -> ExperienceReplayReport {
        let plan = self
            .experience_replay_planner
            .plan(self.experience.records(), limit);
        let mut report = ExperienceReplayReport::from_plan(&plan);

        for item in plan.items {
            let metrics = replay_metrics(&item);
            self.router.observe_with_profile(item.profile, metrics);
            self.hierarchy.observe(item.profile, metrics);

            match item.action {
                RewardAction::Reinforce => {
                    for memory_id in &item.memory_ids {
                        self.cache.reinforce(*memory_id, item.reward);
                        report.touched_memories += 1;
                    }
                    report.reinforced += 1;
                }
                RewardAction::Penalize => {
                    let penalty = 1.0 - item.reward;
                    for memory_id in &item.memory_ids {
                        self.cache.penalize(*memory_id, penalty);
                        report.touched_memories += 1;
                    }
                    report.penalized += 1;
                }
                RewardAction::Hold => {}
            }

            report.applied += 1;
            report.notes.push(format!(
                "experience:{}:{} reward={:.3} reflection_issues={} critical={} actions={} lesson={}",
                item.experience_id,
                item.action.as_str(),
                item.reward,
                item.reflection_issue_count,
                item.critical_reflection_issue_count,
                item.revision_action_count,
                compact(&item.lesson, 64)
            ));
        }

        report
    }

    pub fn infer<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
    ) -> InferenceOutcome {
        let auto_replay_report = self.maybe_auto_replay();
        let adaptive_before_inference = self.adaptive_state();
        let query_vector = self.embed_for_backend(backend, &request.prompt);
        let used_memories = self.cache.lookup(&query_vector, 4);
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
        };
        let route_budget = self
            .router
            .budget_for_prompt_with_context(&request.prompt, routing_context);
        let hierarchy = hardware_plan.hierarchy;
        let transformer_plan =
            self.transformer_planner
                .plan(request.profile, hierarchy, route_budget);

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
            transformer_plan: &transformer_plan,
        };
        let (draft, recursive_runtime_calls) =
            generate_with_recursive_schedule(backend, generation_context);
        let report = self.reflector.reflect(&request.prompt, &draft);
        let runtime_token_metrics = RuntimeTokenMetrics::from_draft(&draft);
        let runtime_diagnostics = draft.runtime_diagnostics.clone();
        let runtime_adapter_observations = RuntimeAdapterObservation::from_experiences(
            &used_experiences,
            runtime_diagnostics.model_id.as_deref().unwrap_or_default(),
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
            let memory_vector = self.embed_for_backend(backend, &memory_text);
            Some(self.cache.store_or_fuse(
                summarize_key(&request.prompt, &report.lesson),
                memory_vector,
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
                    self.cache.store_or_fuse(
                        format_gist_key(&request.prompt, gist),
                        self.embed_for_backend(backend, &memory_text),
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

        for memory in &used_memories {
            if admit_memory && !drift_report.penalize_used_memory {
                self.cache.reinforce(memory.id, report.quality);
            } else {
                self.cache.penalize(memory.id, 1.0 - report.quality);
            }
        }

        self.router.observe_with_profile(request.profile, metrics);
        let mut hierarchy = self.hierarchy.observe(request.profile, metrics);
        if drift_report.rollback_adaptive {
            self.restore_adaptive_state(adaptive_before_inference);
            hierarchy = self.hierarchy.current();
        }
        let router_threshold_after = self.router.threshold();
        let process_reward = self.process_rewarder.score(ProcessRewardInput {
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
            stream_windows: stream_reports.len(),
            stored_memory: stored_memory_id.is_some(),
            stored_gist_memories: stored_gist_memory_ids.len(),
            stored_runtime_kv_memories: stored_runtime_kv_memory_ids.len(),
            gist_records: gist_records.len(),
        });
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
            process_reward: process_reward.clone(),
        });
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
            answer: report.revised_answer.clone(),
            report,
            auto_replay_report,
            metrics,
            runtime_token_metrics,
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
            stream_reports,
            used_memories,
            used_experiences,
            gist_records,
            stored_memory_id,
            stored_gist_memory_ids,
            exported_runtime_kv_blocks,
            stored_runtime_kv_memory_ids,
            drift_report,
            process_reward,
            memory_retention_policy: self.memory_retention_policy,
            memory_compaction_policy: self.memory_compaction_policy.clone(),
            retention_report,
            memory_compaction_report,
            experience_id,
            router_threshold_after,
        }
    }

    fn embed_for_backend<B: InferenceBackend>(&self, backend: &mut B, text: &str) -> Vec<f32> {
        backend
            .embed_text(text)
            .filter(|vector| !vector.is_empty())
            .unwrap_or_else(|| self.embedder.embed(text))
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

    fn maybe_auto_replay(&mut self) -> Option<ExperienceReplayReport> {
        if self.auto_replay_limit == 0 || self.experience.is_empty() {
            return None;
        }
        if self.hardware_snapshot.pressure() >= 0.72 {
            return None;
        }

        let report = self.replay_experience(self.auto_replay_limit);
        if report.applied == 0 {
            None
        } else {
            Some(report)
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeuristicBackend;

impl InferenceBackend for HeuristicBackend {
    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        let memory_summary = if context.memories.is_empty() {
            "no prior memory".to_owned()
        } else {
            context
                .memories
                .iter()
                .take(2)
                .map(|item| format!("{} ({:.2})", item.key, item.similarity))
                .collect::<Vec<_>>()
                .join("; ")
        };
        let profile_hint = match context.profile {
            TaskProfile::General => "balanced global/local/convolution routing",
            TaskProfile::Coding => "strong local-window attention for syntax and interfaces",
            TaskProfile::Writing => "strong global attention for long-range continuity",
            TaskProfile::LongDocument => "strong convolutional fusion for long context compression",
        };
        let tier_counts = context.tier_plan.counts();
        let infini_counts = context.infini_memory_plan.counts();
        let recursive_schedule = context.recursive_schedule;
        let hardware_plan = context.hardware_plan;
        let transformer_counts = context.transformer_plan.counts();
        let experience_summary = if context.experiences.is_empty() {
            "no prior experience".to_owned()
        } else {
            context
                .experiences
                .iter()
                .take(2)
                .map(|item| {
                    let gist_hint = if item.gist_hints.is_empty() {
                        "no gist".to_owned()
                    } else {
                        item.gist_hints.join(" | ")
                    };
                    format!(
                        "{} ({:.2}) reward={:.2}/{} gist: {}",
                        item.lesson,
                        item.score,
                        item.process_reward,
                        item.reward_action.as_str(),
                        gist_hint
                    )
                })
                .collect::<Vec<_>>()
                .join("; ")
        };

        let answer = format!(
            "Prototype inference result: keep Noiron as a control layer around the model backend. \
             Use multi-factor routing for projection, local-window attention, global attention, \
             and convolutional fusion decisions; reinforced KV fusion for local memory; task-aware \
             hierarchy weights for compute allocation; and reflection to score each draft before \
             storing it. Profile hint: {profile_hint}. Prompt anchor: {}. Memory hints: {memory_summary}. \
             Experience hints: {experience_summary}. \
             Route budget: {:.0}% attention, {} fast tokens, {} attention tokens. \
             Tier plan: {} hot GPU, {} warm RAM, {} cold disk memories. \
             Infini memory: {} local-window ({} tokens), {} global ({} tokens), {} sparse-skipped ({} tokens) memories. \
             Recursive schedule: required={}, {} chunks, {} merge rounds, {} execution waves, max parallel {}, {} prompt tokens, native window {}. \
             Hardware plan: {}. \
             Transformer plan: template {}, {} global, {} local, {} convolution layers.",
            compact(&context.prompt, 120),
            context.route_budget.attention_fraction * 100.0,
            context.route_budget.fast_tokens,
            context.route_budget.attention_tokens,
            tier_counts.hot_gpu,
            tier_counts.warm_ram,
            tier_counts.cold_disk,
            infini_counts.local_window,
            infini_counts.local_tokens,
            infini_counts.global_memory,
            infini_counts.global_tokens,
            infini_counts.skipped,
            infini_counts.skipped_tokens,
            recursive_schedule.requires_recursion,
            recursive_schedule.chunk_count(),
            recursive_schedule.merge_round_count(),
            recursive_schedule.execution_wave_count(),
            recursive_schedule.max_parallel_chunks,
            recursive_schedule.prompt_tokens,
            recursive_schedule.native_window_tokens,
            hardware_plan.summary(),
            context.transformer_plan.template_name(),
            transformer_counts.global,
            transformer_counts.local,
            transformer_counts.convolution
        );

        InferenceDraft::new(
            answer,
            vec![
                ReasoningStep::new(
                    "route",
                    "combined entropy, task profile, context, cache, and latency signals",
                    0.82,
                ),
                ReasoningStep::new("memory", "looked up similar reinforced KV memories", 0.78),
                ReasoningStep::new(
                    "recursive_schedule",
                    "planned single-pass or chunk/merge control for native-window limits",
                    0.77,
                ),
                ReasoningStep::new(
                    "reflection",
                    "draft will be scored before reinforcement",
                    0.84,
                ),
            ],
        )
    }
}

#[derive(Debug, Clone)]
struct TextEmbedder {
    dimensions: usize,
}

impl Default for TextEmbedder {
    fn default() -> Self {
        Self { dimensions: 64 }
    }
}

impl TextEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0; self.dimensions];

        for ch in text.chars().filter(|ch| !ch.is_whitespace()) {
            let index = hash_char(ch) % self.dimensions;
            vector[index] += char_weight(ch);
        }

        let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in &mut vector {
                *value /= norm;
            }
        }

        vector
    }
}

fn metrics_from_report(
    draft: &InferenceDraft,
    report: &ReflectionReport,
    route_budget: RouteBudget,
    runtime_token_metrics: RuntimeTokenMetrics,
) -> GenerationMetrics {
    let token_count =
        approximate_token_count(&report.revised_answer).max(approximate_token_count(&draft.answer));
    let route_pressure = (1.0 - route_budget.attention_fraction).max(0.0) * 2.5;
    let baseline_perplexity = 4.0
        + (1.0 - report.quality) * 24.0
        + route_pressure
        + report.contradictions.len() as f32 * 3.5;
    let perplexity = runtime_token_metrics
        .uncertainty_perplexity
        .map(|runtime_perplexity| baseline_perplexity * 0.55 + runtime_perplexity * 0.45)
        .unwrap_or(baseline_perplexity);

    GenerationMetrics {
        perplexity,
        semantic_consistency: report.quality,
        contradiction_count: report.contradictions.len(),
        token_count,
    }
}

fn average(total: f32, count: usize) -> Option<f32> {
    if count == 0 {
        None
    } else {
        Some(total / count as f32)
    }
}

fn replay_metrics(item: &ExperienceReplayItem) -> GenerationMetrics {
    let token_count = (item.route_budget.attention_tokens + item.route_budget.fast_tokens).max(1);
    match item.action {
        RewardAction::Reinforce => GenerationMetrics {
            perplexity: (6.0 + (1.0 - item.reward) * 8.0 + item.stream_windows as f32 * 0.03)
                .clamp(3.0, 18.0),
            semantic_consistency: item.quality.max(item.reward).clamp(0.0, 1.0),
            contradiction_count: item.contradiction_count,
            token_count,
        },
        RewardAction::Penalize => GenerationMetrics {
            perplexity: (18.0 + (1.0 - item.reward) * 18.0 + item.stream_windows as f32 * 0.05)
                .clamp(12.0, 48.0),
            semantic_consistency: item.quality.min(item.reward).clamp(0.0, 1.0),
            contradiction_count: item
                .contradiction_count
                .max(item.critical_reflection_issue_count)
                .max(1),
            token_count,
        },
        RewardAction::Hold => GenerationMetrics {
            perplexity: 10.0,
            semantic_consistency: item.quality.clamp(0.0, 1.0),
            contradiction_count: item
                .contradiction_count
                .max(item.critical_reflection_issue_count),
            token_count,
        },
    }
}

fn approximate_token_count(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    if word_count > 0 {
        word_count
    } else {
        text.chars().count().div_ceil(2)
    }
}

fn summarize_key(prompt: &str, lesson: &str) -> String {
    format!("{} :: {}", compact(prompt, 96), compact(lesson, 64))
}

fn format_gist_key(prompt: &str, gist: &GistRecord) -> String {
    format!(
        "gist:{}:{} :: {}",
        gist.level.as_str(),
        compact(prompt, 64),
        compact(&gist.title, 64)
    )
}

fn format_runtime_kv_key(prompt: &str, block: &RuntimeKvBlock) -> String {
    format!(
        "runtime_kv:l{}h{}:{}-{} :: {}",
        block.layer,
        block.head,
        block.token_start,
        block.token_end,
        compact(prompt, 64)
    )
}

fn protected_memory_ids(
    used_memories: &[MemoryMatch],
    stored_memory_id: Option<u64>,
    stored_gist_memory_ids: &[u64],
    stored_runtime_kv_memory_ids: &[u64],
) -> Vec<u64> {
    let mut ids = used_memories
        .iter()
        .map(|memory| memory.id)
        .collect::<Vec<_>>();
    if let Some(id) = stored_memory_id {
        ids.push(id);
    }
    ids.extend_from_slice(stored_gist_memory_ids);
    ids.extend_from_slice(stored_runtime_kv_memory_ids);
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn generate_with_recursive_schedule<B: InferenceBackend>(
    backend: &mut B,
    context: GenerationContext<'_>,
) -> (InferenceDraft, usize) {
    if !context.recursive_schedule.requires_recursion {
        return (backend.generate(context), 1);
    }

    let mut chunk_drafts = Vec::new();
    for chunk in &context.recursive_schedule.chunks {
        let prompt = recursive_chunk_prompt(context.prompt, chunk);
        chunk_drafts.push(backend.generate(context.with_prompt(&prompt)));
    }

    let mut runtime_calls = chunk_drafts.len();
    let mut merge_inputs = chunk_drafts
        .iter()
        .enumerate()
        .map(|(index, draft)| format!("chunk_{index}: {}", compact(&draft.answer, 600)))
        .collect::<Vec<_>>();
    let mut merge_drafts = Vec::new();

    for round in &context.recursive_schedule.merge_rounds {
        let groups = merge_inputs
            .chunks(context.recursive_schedule.merge_fan_in.max(2))
            .map(|items| items.join("\n"))
            .collect::<Vec<_>>();
        let mut next_inputs = Vec::new();

        for (group_index, group) in groups.iter().enumerate() {
            let prompt = recursive_merge_prompt(context.prompt, round.round, group_index, group);
            let draft = backend.generate(context.with_prompt(&prompt));
            next_inputs.push(format!(
                "merge_r{}_g{}: {}",
                round.round,
                group_index,
                compact(&draft.answer, 600)
            ));
            merge_drafts.push(draft);
            runtime_calls += 1;
        }

        merge_inputs = next_inputs;
    }

    (
        merge_recursive_drafts(context.prompt, chunk_drafts, merge_drafts),
        runtime_calls,
    )
}

fn recursive_chunk_prompt(prompt: &str, chunk: &RecursiveChunk) -> String {
    let chunk_text = prompt_chunk_text(prompt, chunk);
    format!(
        "Noiron recursive chunk {} covering estimated tokens {}..{} with left overlap {} and right overlap {}.\nOriginal prompt anchor: {}\nChunk text:\n{}\nTask: produce a concise, reusable chunk summary with key facts, constraints, and unresolved dependencies for later merge.",
        chunk.index,
        chunk.start_token,
        chunk.end_token,
        chunk.overlap_left,
        chunk.overlap_right,
        compact(prompt, 1_200),
        chunk_text
    )
}

fn prompt_chunk_text(prompt: &str, chunk: &RecursiveChunk) -> String {
    if prompt.chars().any(char::is_whitespace) {
        let words = prompt.split_whitespace().collect::<Vec<_>>();
        return words
            .get(chunk.start_token..chunk.end_token.min(words.len()))
            .unwrap_or(&[])
            .join(" ");
    }

    let divisor = if prompt.is_ascii() { 4 } else { 2 };
    let start = chunk.start_token.saturating_mul(divisor);
    let end = chunk.end_token.saturating_mul(divisor);
    let text = prompt
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect::<String>();
    if text.is_empty() {
        compact(prompt, 1_200)
    } else {
        text
    }
}

fn recursive_merge_prompt(prompt: &str, round: usize, group_index: usize, group: &str) -> String {
    format!(
        "Noiron recursive merge round {round} group {group_index}.\nOriginal prompt anchor: {}\nChunk or prior-merge summaries:\n{group}\nTask: merge these summaries into one coherent answer fragment, preserve conflicts, and keep reusable long-context memory cues.",
        compact(prompt, 1_200)
    )
}

fn merge_recursive_drafts(
    prompt: &str,
    chunk_drafts: Vec<InferenceDraft>,
    merge_drafts: Vec<InferenceDraft>,
) -> InferenceDraft {
    let final_answer = merge_drafts
        .last()
        .or_else(|| chunk_drafts.last())
        .map(|draft| draft.answer.clone())
        .unwrap_or_default();
    let answer = format!(
        "Recursive Noiron merged answer for '{}'. Final merge: {}",
        compact(prompt, 160),
        final_answer
    );
    let mut trace = vec![ReasoningStep::new(
        "recursive_runtime",
        format!(
            "executed {} chunk drafts and {} merge drafts",
            chunk_drafts.len(),
            merge_drafts.len()
        ),
        0.82,
    )];
    let mut tokens = Vec::new();
    let mut exported_kv_blocks = Vec::new();
    let mut diagnostics = Vec::new();

    for draft in chunk_drafts.iter().chain(merge_drafts.iter()) {
        trace.extend(draft.trace.clone());
        tokens.extend(draft.tokens.clone());
        exported_kv_blocks.extend(draft.exported_kv_blocks.clone());
        diagnostics.push(draft.runtime_diagnostics.clone());
    }

    InferenceDraft::new(answer, trace)
        .with_tokens(tokens)
        .with_exported_kv_blocks(exported_kv_blocks)
        .with_runtime_diagnostics(merge_runtime_diagnostics(&diagnostics))
}

fn merge_runtime_diagnostics(diagnostics: &[RuntimeDiagnostics]) -> RuntimeDiagnostics {
    let mut merged = RuntimeDiagnostics::default();
    let mut forward_energy_total = 0.0;
    let mut forward_energy_count = 0;
    let mut kv_influence_total = 0.0;
    let mut kv_influence_count = 0;

    for diagnostic in diagnostics {
        if merged.model_id.is_none() {
            merged.model_id = diagnostic.model_id.clone();
        }
        if merged.selected_adapter.is_none() {
            merged.selected_adapter = diagnostic.selected_adapter.clone();
        }
        merged.layer_count += diagnostic.layer_count;
        merged.hidden_size = merged.hidden_size.max(diagnostic.hidden_size);
        merged.local_window_tokens = merged
            .local_window_tokens
            .max(diagnostic.local_window_tokens);
        merged.imported_kv_blocks += diagnostic.imported_kv_blocks;
        merged.exported_kv_blocks += diagnostic.exported_kv_blocks;

        if let Some(value) = diagnostic.forward_energy.filter(|value| value.is_finite()) {
            forward_energy_total += value;
            forward_energy_count += 1;
        }
        if let Some(value) = diagnostic.kv_influence.filter(|value| value.is_finite()) {
            kv_influence_total += value;
            kv_influence_count += 1;
        }
    }

    merged.forward_energy = average(forward_energy_total, forward_energy_count);
    merged.kv_influence = average(kv_influence_total, kv_influence_count);
    merged
}

fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn hash_char(ch: char) -> usize {
    let mut buffer = [0_u8; 4];
    let mut hash = 0xcbf29ce484222325_u64;

    for byte in ch.encode_utf8(&mut buffer).as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    hash as usize
}

fn char_weight(ch: char) -> f32 {
    if ch.is_ascii_alphabetic() {
        1.0
    } else if ch.is_ascii_digit() {
        1.15
    } else if ch.is_ascii_punctuation() {
        0.35
    } else {
        1.25
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{DeviceClass, RuntimeAdapterHint};
    use crate::local_runtime::LocalTransformerRuntime;
    use crate::process_reward::ProcessRewardComponents;
    use crate::reflection::DraftToken;
    use crate::runtime::RuntimeBackend;
    use crate::tiered_cache::TierMigrationAction;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn inference_updates_router_and_memory() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("build a Rust Noiron routing cache", TaskProfile::Coding),
            &mut backend,
        );

        assert!(outcome.answer.contains("Noiron"));
        assert!(outcome.stored_memory_id.is_some());
        assert!(!outcome.stream_reports.is_empty());
        assert_eq!(
            engine.router.observations(),
            outcome.stream_reports.len() as u64 + 1
        );
        assert_eq!(engine.experience.len(), 1);
        assert_eq!(outcome.experience_id, 1);
        assert!(outcome.process_reward.total > 0.0);
        assert!(
            (engine.experience.records()[0].process_reward.total - outcome.process_reward.total)
                .abs()
                < 0.0001
        );
        assert!(!outcome.transformer_plan.is_empty());
        assert!(!engine.cache.is_empty());
    }

    #[derive(Debug, Clone)]
    struct ShortRepairBackend;

    impl InferenceBackend for ShortRepairBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                "Rust routes.",
                vec![ReasoningStep::new("draft", "short but grounded", 0.86)],
            )
            .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                0,
                0,
                0,
                1,
                vec![0.1, 0.2, 0.3],
                vec![0.3, 0.2, 0.1],
            )])
        }
    }

    #[test]
    fn reflection_repair_rechecks_answer_without_admitting_stale_runtime_kv() {
        let mut engine = NoironEngine::new();
        let mut backend = ShortRepairBackend;
        let outcome = engine.infer(
            InferenceRequest::new(
                "Explain Rust Noiron adaptive routing decisions",
                TaskProfile::Coding,
            ),
            &mut backend,
        );

        assert_eq!(outcome.report.revision_passes, 1);
        assert!(outcome.answer.contains("Reflection repair"));
        assert!(outcome.stored_memory_id.is_some());
        assert_eq!(outcome.exported_runtime_kv_blocks, 1);
        assert!(outcome.stored_runtime_kv_memory_ids.is_empty());
    }

    #[test]
    fn inference_auto_replays_prior_experience_before_next_run() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;

        let first = engine.infer(
            InferenceRequest::new("build a Rust Noiron replay loop", TaskProfile::Coding),
            &mut backend,
        );
        let second = engine.infer(
            InferenceRequest::new("build a Rust Noiron replay loop", TaskProfile::Coding),
            &mut backend,
        );

        assert!(first.auto_replay_report.is_none());
        let report = second.auto_replay_report.as_ref().unwrap();
        assert!(report.applied >= 1);
        assert!(report.reinforced >= 1 || report.penalized >= 1);
    }

    #[test]
    fn auto_replay_skips_when_hardware_pressure_is_high() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;

        engine.infer(
            InferenceRequest::new("build a Rust Noiron replay loop", TaskProfile::Coding),
            &mut backend,
        );
        engine.set_hardware_snapshot(HardwareSnapshot::new(
            DeviceClass::CpuOnly,
            0.98,
            0.90,
            0.96,
            0.80,
        ));
        let second = engine.infer(
            InferenceRequest::new("build a Rust Noiron replay loop", TaskProfile::Coding),
            &mut backend,
        );

        assert!(second.auto_replay_report.is_none());
    }

    #[test]
    fn inference_exposes_tiered_cache_plan() {
        let mut cache = KvFusionCache::new();
        let vector = TextEmbedder::default().embed("Rust Noiron tiered memory");
        cache.store_or_fuse("Rust Noiron tiered memory", vector, 1.0);
        let mut engine = NoironEngine::with_cache(cache);
        let mut backend = HeuristicBackend;

        let outcome = engine.infer(
            InferenceRequest::new("Rust Noiron tiered memory", TaskProfile::Coding),
            &mut backend,
        );

        assert_eq!(outcome.tier_plan.placements().len(), 1);
        assert_eq!(outcome.tier_migrations.len(), 1);
        assert_eq!(outcome.infini_memory_plan.counts().local_window, 1);
        assert!(outcome.answer.contains("Tier plan"));
        assert!(outcome.answer.contains("Infini memory"));
    }

    #[test]
    fn inference_exposes_recursive_schedule_for_long_prompt() {
        let mut engine = NoironEngine::new();
        engine.recursive_scheduler = RecursiveScheduler::new(8, 6, 2, 2);
        let prompt = (0..14)
            .map(|index| format!("chunk_token_{index}"))
            .collect::<Vec<_>>()
            .join(" ");
        let mut backend = HeuristicBackend;

        let outcome = engine.infer(
            InferenceRequest::new(prompt, TaskProfile::LongDocument),
            &mut backend,
        );

        assert!(outcome.recursive_schedule.requires_recursion);
        assert_eq!(outcome.recursive_schedule.chunk_count(), 3);
        assert_eq!(outcome.recursive_schedule.merge_round_count(), 2);
        assert_eq!(
            outcome.recursive_schedule.max_parallel_chunks,
            outcome.hardware_plan.execution.max_parallel_chunks
        );
        assert_eq!(outcome.recursive_schedule.execution_wave_count(), 2);
        assert_eq!(outcome.recursive_runtime_calls, 6);
        assert!(outcome.answer.contains("Recursive Noiron merged answer"));
        assert!(outcome.answer.contains("Recursive schedule"));
    }

    #[test]
    fn recursive_inference_calls_backend_for_chunks_and_merges() {
        struct CountingBackend {
            prompts: Vec<String>,
        }

        impl InferenceBackend for CountingBackend {
            fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
                self.prompts.push(context.prompt.to_owned());
                InferenceDraft::new(
                    format!("draft {}", self.prompts.len()),
                    vec![ReasoningStep::new("count", "counted recursive call", 0.9)],
                )
            }
        }

        let mut engine = NoironEngine::new();
        engine.recursive_scheduler = RecursiveScheduler::new(8, 6, 2, 2);
        let prompt = (0..14)
            .map(|index| format!("recursive_call_{index}"))
            .collect::<Vec<_>>()
            .join(" ");
        let mut backend = CountingBackend {
            prompts: Vec::new(),
        };

        let outcome = engine.infer(
            InferenceRequest::new(prompt, TaskProfile::LongDocument),
            &mut backend,
        );

        assert_eq!(outcome.recursive_schedule.chunk_count(), 3);
        assert_eq!(outcome.recursive_schedule.merge_round_count(), 2);
        assert_eq!(outcome.recursive_runtime_calls, 6);
        assert_eq!(backend.prompts.len(), outcome.recursive_runtime_calls);
        assert!(
            backend
                .prompts
                .iter()
                .filter(|prompt| prompt.contains("Noiron recursive chunk"))
                .count()
                >= 3
        );
        assert!(
            backend
                .prompts
                .iter()
                .filter(|prompt| prompt.contains("Noiron recursive merge round"))
                .count()
                >= 2
        );
    }

    #[test]
    fn hardware_parallel_budget_limits_recursive_execution_waves() {
        let mut engine = NoironEngine::new();
        engine.recursive_scheduler = RecursiveScheduler::new(8, 6, 2, 2);
        engine.set_hardware_snapshot(HardwareSnapshot::new(
            DeviceClass::Embedded,
            0.82,
            0.0,
            0.82,
            0.55,
        ));
        let prompt = (0..14)
            .map(|index| format!("edge_chunk_{index}"))
            .collect::<Vec<_>>()
            .join(" ");
        let mut backend = HeuristicBackend;

        let outcome = engine.infer(
            InferenceRequest::new(prompt, TaskProfile::LongDocument),
            &mut backend,
        );

        assert_eq!(outcome.hardware_plan.execution.max_parallel_chunks, 1);
        assert_eq!(outcome.recursive_schedule.max_parallel_chunks, 1);
        assert_eq!(
            outcome.recursive_schedule.execution_wave_count(),
            outcome.recursive_schedule.chunk_count()
        );
    }

    #[test]
    fn inference_uses_backend_native_window_for_recursive_schedule() {
        struct SmallWindowBackend;

        impl InferenceBackend for SmallWindowBackend {
            fn runtime_native_context_window(&self) -> Option<usize> {
                Some(4)
            }

            fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
                InferenceDraft::new(
                    format!(
                        "native window {} chunks {}",
                        context.recursive_schedule.native_window_tokens,
                        context.recursive_schedule.chunk_count()
                    ),
                    vec![ReasoningStep::new("runtime", "used native window", 0.9)],
                )
            }
        }

        let mut engine = NoironEngine::new();
        let mut backend = SmallWindowBackend;

        let outcome = engine.infer(
            InferenceRequest::new("one two three four five six", TaskProfile::LongDocument),
            &mut backend,
        );

        assert!(outcome.recursive_schedule.requires_recursion);
        assert_eq!(outcome.recursive_schedule.native_window_tokens, 4);
        assert!(outcome.recursive_schedule.chunk_count() > 1);
        assert!(outcome.answer.contains("native window 4"));
    }

    #[test]
    fn inference_generates_gist_memory_for_high_quality_answer() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;

        let outcome = engine.infer(
            InferenceRequest::new(
                "Rust Noiron hierarchical gist memory for long context control",
                TaskProfile::LongDocument,
            ),
            &mut backend,
        );

        assert!(!outcome.gist_records.is_empty());
        assert!(!outcome.stored_gist_memory_ids.is_empty());
        assert_eq!(
            engine.experience.records()[0].gist_records.len(),
            outcome.gist_records.len()
        );
        assert_eq!(
            engine.experience.records()[0].gist_memory_ids,
            outcome.stored_gist_memory_ids
        );
    }

    #[test]
    fn inference_stores_high_quality_exported_runtime_kv() {
        struct ExportingBackend;

        impl InferenceBackend for ExportingBackend {
            fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
                InferenceDraft::new(
                    "Rust runtime KV export memory should be stored as useful Noiron local memory for future routing.",
                    vec![ReasoningStep::new("runtime", "exported reusable kv", 0.92)],
                )
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    2,
                    1,
                    0,
                    4,
                    vec![0.1, 0.2],
                    vec![0.3, 0.4],
                )])
            }
        }

        let mut engine = NoironEngine::new();
        let mut backend = ExportingBackend;

        let outcome = engine.infer(
            InferenceRequest::new("Rust runtime KV export memory", TaskProfile::Coding),
            &mut backend,
        );

        assert_eq!(outcome.exported_runtime_kv_blocks, 1);
        assert_eq!(outcome.stored_runtime_kv_memory_ids.len(), 1);
        assert!(
            engine
                .cache
                .entries()
                .iter()
                .any(|entry| entry.key.contains("runtime_kv:l2h1"))
        );
    }

    #[test]
    fn drift_guard_blocks_contradictory_runtime_kv_memory() {
        struct ContradictingBackend;

        impl InferenceBackend for ContradictingBackend {
            fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
                InferenceDraft::new(
                    "Rust Noiron drift guard is certain about this answer, but it is also uncertain in the same claim, so the self-evolving memory path should treat it as unsafe.",
                    vec![ReasoningStep::new("runtime", "contradictory draft", 0.92)],
                )
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    1,
                    0,
                    0,
                    2,
                    vec![0.2, 0.4],
                    vec![0.3, 0.5],
                )])
            }
        }

        let mut engine = NoironEngine::new();
        let mut backend = ContradictingBackend;

        let outcome = engine.infer(
            InferenceRequest::new("Rust Noiron drift guard", TaskProfile::Coding),
            &mut backend,
        );

        assert_eq!(outcome.exported_runtime_kv_blocks, 1);
        assert_eq!(
            outcome.drift_report.severity,
            crate::drift::DriftSeverity::Block
        );
        assert!(!outcome.report.store_as_memory);
        assert!(outcome.report.critical_issue_count() > 0);
        assert!(
            outcome
                .report
                .issue_codes()
                .iter()
                .any(|code| code == "conflicting_certainty_markers")
        );
        assert!(outcome.stored_memory_id.is_none());
        assert!(outcome.stored_runtime_kv_memory_ids.is_empty());
    }

    #[test]
    fn drift_guard_rolls_back_adaptive_state_for_bad_draft() {
        struct BadBackend;

        impl InferenceBackend for BadBackend {
            fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
                InferenceDraft::new("", vec![ReasoningStep::new("runtime", "empty", 0.0)])
            }
        }

        let mut engine = NoironEngine::new();
        let threshold_before = engine.router.threshold();
        let hierarchy_before = engine.hierarchy.current();
        let mut backend = BadBackend;

        let outcome = engine.infer(
            InferenceRequest::new("Rust Noiron rollback bad draft", TaskProfile::Coding),
            &mut backend,
        );

        assert_eq!(
            outcome.drift_report.severity,
            crate::drift::DriftSeverity::Rollback
        );
        assert!((outcome.router_threshold_after - threshold_before).abs() < 0.0001);
        assert!((engine.router.threshold() - threshold_before).abs() < 0.0001);
        assert!((engine.hierarchy.current().local - hierarchy_before.local).abs() < 0.0001);
        assert!(outcome.stored_memory_id.is_none());
    }

    #[test]
    fn inference_uses_hardware_pressure_for_latency_and_kv_budget() {
        let mut cache = KvFusionCache::new();
        cache.store_or_fuse("hardware constrained memory", vec![1.0, 0.0, 0.0], 1.0);
        let mut engine = NoironEngine::with_cache(cache);
        engine.set_hardware_snapshot(HardwareSnapshot::new(
            DeviceClass::CpuOnly,
            0.95,
            0.0,
            0.90,
            0.50,
        ));
        let mut backend = HeuristicBackend;

        let outcome = engine.infer(
            InferenceRequest::new("hardware constrained memory", TaskProfile::LongDocument),
            &mut backend,
        );

        assert!(outcome.hardware_plan.latency_budget_ms.is_some());
        assert!(outcome.hardware_plan.local_kv_token_budget < 512);
        assert!(outcome.hardware_plan.global_kv_token_budget < 4096);
        assert!(outcome.answer.contains("Hardware plan"));
    }

    #[test]
    fn hardware_pressure_flows_into_route_budget() {
        let prompt = (0..8)
            .map(|index| format!("ComputeA{index}B{index}C{index}D"))
            .collect::<Vec<_>>()
            .join(" ");
        let mut roomy_engine = NoironEngine::new();
        roomy_engine.set_hardware_snapshot(HardwareSnapshot::new(
            DeviceClass::Server,
            0.10,
            0.15,
            0.20,
            0.10,
        ));
        let mut constrained_engine = NoironEngine::new();
        constrained_engine.set_hardware_snapshot(HardwareSnapshot::new(
            DeviceClass::Embedded,
            0.95,
            0.0,
            0.92,
            0.70,
        ));
        let mut roomy_backend = HeuristicBackend;
        let mut constrained_backend = HeuristicBackend;

        let roomy = roomy_engine.infer(
            InferenceRequest::new(prompt.clone(), TaskProfile::Coding),
            &mut roomy_backend,
        );
        let constrained = constrained_engine.infer(
            InferenceRequest::new(prompt, TaskProfile::Coding),
            &mut constrained_backend,
        );

        assert!(
            roomy.hardware_plan.compute_headroom() > constrained.hardware_plan.compute_headroom()
        );
        assert!(
            roomy.route_budget.attention_fraction > constrained.route_budget.attention_fraction
        );
    }

    #[test]
    fn runtime_token_uncertainty_raises_generation_perplexity() {
        let low_entropy = InferenceDraft::new(
            "A stable local runtime answer with enough detail to pass reflection.",
            vec![],
        )
        .with_tokens(vec![
            DraftToken {
                text: "stable".to_owned(),
                logprob: Some(-0.05),
                entropy: Some(0.05),
            },
            DraftToken {
                text: "answer".to_owned(),
                logprob: Some(-0.08),
                entropy: Some(0.08),
            },
        ]);
        let high_entropy = InferenceDraft::new(
            "A stable local runtime answer with enough detail to pass reflection.",
            vec![],
        )
        .with_tokens(vec![
            DraftToken {
                text: "unstable".to_owned(),
                logprob: Some(-2.5),
                entropy: Some(0.95),
            },
            DraftToken {
                text: "answer".to_owned(),
                logprob: Some(-1.8),
                entropy: Some(0.85),
            },
        ]);
        let report = ReflectionReport {
            quality: 0.88,
            contradictions: Vec::new(),
            issues: Vec::new(),
            revision_actions: Vec::new(),
            revision_passes: 0,
            revised_answer: low_entropy.answer.clone(),
            store_as_memory: true,
            lesson: "runtime token metrics should affect Noiron feedback".to_owned(),
        };
        let budget = RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 3,
            attention_fraction: 0.25,
        };

        let low_token_metrics = RuntimeTokenMetrics::from_draft(&low_entropy);
        let high_token_metrics = RuntimeTokenMetrics::from_draft(&high_entropy);
        let low = metrics_from_report(&low_entropy, &report, budget, low_token_metrics);
        let high = metrics_from_report(&high_entropy, &report, budget, high_token_metrics);

        assert!(
            high_token_metrics.average_entropy.unwrap()
                > low_token_metrics.average_entropy.unwrap()
        );
        assert!(
            high_token_metrics.uncertainty_perplexity.unwrap()
                > low_token_metrics.uncertainty_perplexity.unwrap()
        );
        assert!(high.perplexity > low.perplexity + 2.0);
        assert_eq!(high.semantic_consistency, low.semantic_consistency);
    }

    #[test]
    fn runtime_token_metrics_ignore_non_finite_runtime_values() {
        let draft = InferenceDraft::new("runtime returned partial token metadata", vec![])
            .with_tokens(vec![
                DraftToken {
                    text: "bad-entropy".to_owned(),
                    logprob: Some(f32::NAN),
                    entropy: Some(f32::INFINITY),
                },
                DraftToken {
                    text: "valid".to_owned(),
                    logprob: Some(-0.5),
                    entropy: Some(0.25),
                },
            ]);

        let metrics = RuntimeTokenMetrics::from_draft(&draft);

        assert_eq!(metrics.token_count, 2);
        assert_eq!(metrics.entropy_count, 1);
        assert_eq!(metrics.logprob_count, 1);
        assert_eq!(metrics.average_entropy, Some(0.25));
        assert_eq!(metrics.average_neg_logprob, Some(0.5));
        assert_eq!(metrics.uncertainty_perplexity, Some(3.5));
    }

    #[test]
    fn inference_outcome_exposes_runtime_adapter_observations() {
        struct DiagnosedBackend;

        impl InferenceBackend for DiagnosedBackend {
            fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
                InferenceDraft::new(
                    "A stable adapter-aware runtime answer with useful control detail.",
                    vec![ReasoningStep::new(
                        "runtime",
                        "selected a historically useful adapter",
                        0.91,
                    )],
                )
                .with_runtime_diagnostics(RuntimeDiagnostics {
                    model_id: Some("self-transformer-test".to_owned()),
                    selected_adapter: Some(RuntimeAdapterHint::CpuSimd.as_str().to_owned()),
                    layer_count: 6,
                    hidden_size: 128,
                    local_window_tokens: 4096,
                    forward_energy: Some(0.20),
                    kv_influence: Some(0.46),
                    imported_kv_blocks: 1,
                    exported_kv_blocks: 1,
                })
            }
        }

        let mut engine = NoironEngine::new();
        engine.experience.record(ExperienceInput {
            prompt: "adapter observation history".to_owned(),
            profile: TaskProfile::Coding,
            lesson: "prefer cpu SIMD when prior self-developed runtime reward is strong".to_owned(),
            quality: 0.92,
            contradictions: Vec::new(),
            reflection_issues: Vec::new(),
            revision_actions: Vec::new(),
            stored_memory_id: None,
            router_threshold_after: 0.55,
            stream_windows: 1,
            route_budget: RouteBudget {
                threshold: 0.55,
                attention_tokens: 2,
                fast_tokens: 2,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: Vec::new(),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: RuntimeDiagnostics {
                model_id: Some("self-transformer-test".to_owned()),
                selected_adapter: Some(RuntimeAdapterHint::CpuSimd.as_str().to_owned()),
                layer_count: 6,
                hidden_size: 128,
                local_window_tokens: 4096,
                forward_energy: Some(0.18),
                kv_influence: Some(0.50),
                imported_kv_blocks: 1,
                exported_kv_blocks: 2,
            },
            process_reward: ProcessRewardReport {
                total: 0.90,
                action: RewardAction::Reinforce,
                components: ProcessRewardComponents::default(),
                notes: Vec::new(),
            },
        });
        let mut backend = DiagnosedBackend;

        let outcome = engine.infer(
            InferenceRequest::new("adapter observation history", TaskProfile::Coding),
            &mut backend,
        );

        assert_eq!(outcome.runtime_adapter_observations.len(), 1);
        assert_eq!(
            outcome.runtime_adapter_observations[0].adapter,
            RuntimeAdapterHint::CpuSimd
        );
        assert!(outcome.runtime_adapter_observations[0].score > 0.80);
    }

    #[test]
    fn replay_experience_reinforces_rewarded_memory() {
        let mut engine = NoironEngine::new();
        let memory_id = engine
            .cache
            .store_or_fuse("replay memory", vec![1.0, 0.0, 0.0], 0.8);
        engine.experience.record(ExperienceInput {
            prompt: "Rust replay router".to_owned(),
            profile: TaskProfile::Coding,
            lesson: "reinforce high reward control path".to_owned(),
            quality: 0.92,
            contradictions: Vec::new(),
            reflection_issues: Vec::new(),
            revision_actions: Vec::new(),
            stored_memory_id: Some(memory_id),
            router_threshold_after: 0.55,
            stream_windows: 2,
            route_budget: RouteBudget {
                threshold: 0.55,
                attention_tokens: 2,
                fast_tokens: 2,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: vec![memory_id],
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: RuntimeDiagnostics::default(),
            process_reward: ProcessRewardReport {
                total: 0.91,
                action: RewardAction::Reinforce,
                components: ProcessRewardComponents::default(),
                notes: Vec::new(),
            },
        });
        let before_hits = engine.cache.entries()[0].hits;

        let report = engine.replay_experience(4);

        assert_eq!(report.applied, 1);
        assert_eq!(report.reinforced, 1);
        assert!(engine.cache.entries()[0].hits > before_hits);
        assert!(engine.router.observations() > 0);
    }

    #[test]
    fn replay_experience_reinforces_used_memory_ids() {
        let mut engine = NoironEngine::new();
        let memory_id = engine
            .cache
            .store_or_fuse("used replay memory", vec![1.0, 0.0, 0.0], 0.8);
        engine.experience.record(ExperienceInput {
            prompt: "Rust replay used memory".to_owned(),
            profile: TaskProfile::Coding,
            lesson: "reinforce memories that helped a high reward answer".to_owned(),
            quality: 0.93,
            contradictions: Vec::new(),
            reflection_issues: Vec::new(),
            revision_actions: Vec::new(),
            stored_memory_id: None,
            router_threshold_after: 0.55,
            stream_windows: 2,
            route_budget: RouteBudget {
                threshold: 0.55,
                attention_tokens: 1,
                fast_tokens: 3,
                attention_fraction: 0.25,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: vec![memory_id],
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: RuntimeDiagnostics::default(),
            process_reward: ProcessRewardReport {
                total: 0.90,
                action: RewardAction::Reinforce,
                components: ProcessRewardComponents::default(),
                notes: Vec::new(),
            },
        });
        let before_hits = engine.cache.entries()[0].hits;

        let report = engine.replay_experience(4);

        assert_eq!(report.touched_memories, 1);
        assert!(engine.cache.entries()[0].hits > before_hits);
    }

    #[test]
    fn inference_tracks_tier_migrations_across_runs() {
        let mut cache = KvFusionCache::new();
        cache.store_or_fuse("Rust Noiron tiered memory", vec![1.0, 0.0, 0.0], 1.0);
        let mut engine = NoironEngine::with_cache(cache);
        let mut backend = HeuristicBackend;

        let first = engine.infer(
            InferenceRequest::new("Rust Noiron tiered memory", TaskProfile::Coding),
            &mut backend,
        );
        let second = engine.infer(
            InferenceRequest::new("Rust Noiron tiered memory", TaskProfile::Coding),
            &mut backend,
        );

        assert!(
            first
                .tier_migrations
                .iter()
                .any(|migration| migration.action == TierMigrationAction::New)
        );
        assert!(
            second
                .tier_migrations
                .iter()
                .any(|migration| migration.from.is_some())
        );
        assert!(
            second
                .tier_migrations
                .iter()
                .any(|migration| migration.action != TierMigrationAction::New)
        );
    }

    #[test]
    fn inference_uses_relevant_experience() {
        let mut engine = NoironEngine::new();
        engine.experience.record(ExperienceInput {
            prompt: "Rust router feedback".to_owned(),
            profile: TaskProfile::Coding,
            lesson: "reuse token-window feedback lessons".to_owned(),
            quality: 0.9,
            contradictions: Vec::new(),
            reflection_issues: Vec::new(),
            revision_actions: Vec::new(),
            stored_memory_id: None,
            router_threshold_after: 0.55,
            stream_windows: 2,
            route_budget: RouteBudget {
                threshold: 0.55,
                attention_tokens: 1,
                fast_tokens: 3,
                attention_fraction: 0.25,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: Vec::new(),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: RuntimeDiagnostics::default(),
            process_reward: ProcessRewardReport::default(),
        });
        let mut backend = HeuristicBackend;

        let outcome = engine.infer(
            InferenceRequest::new("Rust router feedback", TaskProfile::Coding),
            &mut backend,
        );

        assert_eq!(outcome.used_experiences.len(), 1);
        assert!(outcome.answer.contains("Experience hints"));
    }

    #[test]
    fn full_state_roundtrip_reuses_memory_experience_and_runtime_kv() {
        let memory_path = temp_path("full-state-memory", "ndkv");
        let experience_path = temp_path("full-state-experience", "ndkv");
        let adaptive_path = temp_path("full-state-adaptive", "ndkv");
        let prompt = "Rust Noiron persistent runtime KV memory";

        let mut engine = NoironEngine::new();
        engine.set_memory_retention_policy(MemoryRetentionPolicy {
            stale_after: 11,
            decay_rate: 0.12,
            remove_below_strength: 0.08,
            remove_after_failures: 7,
        });
        engine.set_memory_compaction_policy(MemoryCompactionPolicy {
            similarity_threshold: 0.91,
            max_candidates: 64,
            max_merges: 4,
        });
        let mut first_backend = RuntimeBackend::new(LocalTransformerRuntime::default());
        let first = engine.infer(
            InferenceRequest::new(prompt, TaskProfile::Coding),
            &mut first_backend,
        );
        assert!(first.stored_memory_id.is_some());
        assert!(!first.stored_runtime_kv_memory_ids.is_empty());

        engine
            .save_full_state(&memory_path, &experience_path, &adaptive_path)
            .unwrap();

        let mut restored =
            NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
        assert_eq!(restored.memory_retention_policy.stale_after, 11);
        assert!((restored.memory_retention_policy.decay_rate - 0.12).abs() < 0.0001);
        assert_eq!(restored.memory_compaction_policy.max_candidates, 64);
        assert_eq!(restored.memory_compaction_policy.max_merges, 4);
        let mut second_backend = RuntimeBackend::new(LocalTransformerRuntime::default());
        let second = restored.infer(
            InferenceRequest::new(prompt, TaskProfile::Coding),
            &mut second_backend,
        );

        assert!(!second.used_memories.is_empty());
        assert!(!second.used_experiences.is_empty());
        assert!(!second_backend.runtime().imported_kv_blocks().is_empty());
        assert!(second.answer.contains("imported"));

        cleanup(memory_path);
        cleanup(experience_path);
        cleanup(adaptive_path);
    }

    #[test]
    fn inference_stream_monitor_uses_backend_tokens() {
        struct TokenBackend;

        impl InferenceBackend for TokenBackend {
            fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
                InferenceDraft::new(
                    "easy hard",
                    vec![ReasoningStep::new("tokens", "runtime token metadata", 0.9)],
                )
                .with_tokens(vec![
                    DraftToken {
                        text: "easy".to_owned(),
                        logprob: Some(-0.1),
                        entropy: Some(0.1),
                    },
                    DraftToken {
                        text: "hard".to_owned(),
                        logprob: Some(-1.2),
                        entropy: Some(0.9),
                    },
                ])
            }
        }

        let mut engine = NoironEngine::new();
        engine.stream_monitor = TokenStreamMonitor::new(2);
        let mut backend = TokenBackend;

        let outcome = engine.infer(
            InferenceRequest::new("runtime token metadata", TaskProfile::Coding),
            &mut backend,
        );

        assert_eq!(outcome.stream_reports.len(), 1);
        assert_eq!(outcome.stream_reports[0].observations[0].entropy, 0.1);
        assert_eq!(outcome.stream_reports[0].observations[1].entropy, 0.9);
    }

    #[test]
    fn adaptive_state_restores_router_and_hierarchy() {
        let mut engine = NoironEngine::new();
        engine.router.observe(GenerationMetrics {
            perplexity: 4.0,
            semantic_consistency: 0.98,
            contradiction_count: 0,
            token_count: 8,
        });
        engine.hierarchy.adapt_to_profile(TaskProfile::Coding);
        engine.set_memory_retention_policy(MemoryRetentionPolicy {
            stale_after: 9,
            decay_rate: 0.18,
            remove_below_strength: 0.11,
            remove_after_failures: 6,
        });
        engine.set_memory_compaction_policy(MemoryCompactionPolicy {
            similarity_threshold: 0.89,
            max_candidates: 48,
            max_merges: 3,
        });
        let state = engine.adaptive_state();

        let mut restored = NoironEngine::new();
        restored.restore_adaptive_state(state);

        assert_eq!(restored.router.observations(), engine.router.observations());
        assert!((restored.router.threshold() - engine.router.threshold()).abs() < 0.0001);
        assert!(
            (restored.hierarchy.current().local - engine.hierarchy.current().local).abs() < 0.0001
        );
        assert_eq!(restored.memory_retention_policy.stale_after, 9);
        assert!((restored.memory_retention_policy.decay_rate - 0.18).abs() < 0.0001);
        assert_eq!(restored.memory_compaction_policy.max_candidates, 48);
        assert_eq!(restored.memory_compaction_policy.max_merges, 3);
    }

    fn temp_path(label: &str, extension: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-{label}-{}-{nanos}.{extension}",
            std::process::id()
        ))
    }

    fn cleanup(path: std::path::PathBuf) {
        let _ = fs::remove_file(path);
    }
}
