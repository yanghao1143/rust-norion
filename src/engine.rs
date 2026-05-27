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
use crate::kv_cache::{KvFusionCache, MemoryMatch, MemoryRetentionPolicy, RetentionReport};
use crate::kv_exchange::RuntimeKvBlock;
use crate::process_reward::{
    ProcessRewardInput, ProcessRewardReport, ProcessRewarder, RewardAction,
};
use crate::recursive_scheduler::{RecursiveSchedule, RecursiveScheduler};
use crate::reflection::{InferenceDraft, ReasoningStep, ReflectionReport, Reflector};
use crate::router::{GenerationMetrics, NoironRouter, RouteBudget, RoutingContext};
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

pub trait InferenceBackend {
    fn runtime_native_context_window(&self) -> Option<usize> {
        None
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft;
}

#[derive(Debug, Clone)]
pub struct InferenceOutcome {
    pub answer: String,
    pub report: ReflectionReport,
    pub metrics: GenerationMetrics,
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
    pub retention_report: RetentionReport,
    pub experience_id: u64,
    pub router_threshold_after: f32,
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
        }
    }

    pub fn restore_adaptive_state(&mut self, state: AdaptiveState) {
        self.router.restore_state(state.router);
        self.hierarchy.restore_state(state.hierarchy);
        self.last_tier_plan = state.tier_plan;
    }

    pub fn save_adaptive_state(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.adaptive_state().save_to_disk_kv(path)
    }

    pub fn set_hardware_snapshot(&mut self, snapshot: HardwareSnapshot) {
        self.hardware_snapshot = snapshot;
    }

    pub fn replay_experience(&mut self, limit: usize) -> ExperienceReplayReport {
        let plan = self
            .experience_replay_planner
            .plan(self.experience.records(), limit);
        let mut report = ExperienceReplayReport::from_plan(&plan);

        for item in plan.items {
            let metrics = replay_metrics(&item);
            self.router.observe(metrics);
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
                "experience:{}:{} reward={:.3} lesson={}",
                item.experience_id,
                item.action.as_str(),
                item.reward,
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
        let adaptive_before_inference = self.adaptive_state();
        let query_vector = self.embedder.embed(&request.prompt);
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
        };
        let route_budget = self
            .router
            .budget_for_prompt_with_context(&request.prompt, routing_context);
        let hierarchy = hardware_plan.hierarchy;
        let transformer_plan =
            self.transformer_planner
                .plan(request.profile, hierarchy, route_budget);

        let draft = backend.generate(GenerationContext {
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
        });
        let report = self.reflector.reflect(&request.prompt, &draft);
        let metrics = metrics_from_report(&draft, &report, route_budget);
        let gist_records =
            self.gist_generator
                .generate(&request.prompt, &report.revised_answer, report.quality);
        let stream_reports = self.stream_monitor.observe_draft(
            &mut self.router,
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
        let admit_runtime_kv = admit_memory && drift_report.allow_runtime_kv_write;

        let stored_memory_id = if admit_memory {
            let memory_text = format!(
                "prompt:{}\nanswer:{}\nlesson:{}",
                request.prompt.as_str(),
                report.revised_answer,
                report.lesson
            );
            let memory_vector = self.embedder.embed(&memory_text);
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
                        self.embedder.embed(&memory_text),
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

        self.router.observe(metrics);
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
            stored_memory_id,
            router_threshold_after,
            stream_windows: stream_reports.len(),
            hierarchy,
            gist_records: gist_records.clone(),
            gist_memory_ids: stored_gist_memory_ids.clone(),
            process_reward: process_reward.clone(),
        });
        let retention_report = self.cache.apply_retention(MemoryRetentionPolicy::default());
        if !drift_report.rollback_adaptive {
            self.last_tier_plan = self.tiered_cache.plan(self.cache.entries(), &used_memories);
        }

        InferenceOutcome {
            answer: report.revised_answer.clone(),
            report,
            metrics,
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
            retention_report,
            experience_id,
            router_threshold_after,
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
             Recursive schedule: required={}, {} chunks, {} merge rounds, {} prompt tokens, native window {}. \
             Hardware plan: {}. \
             Transformer plan: {} global, {} local, {} convolution layers.",
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
            recursive_schedule.prompt_tokens,
            recursive_schedule.native_window_tokens,
            hardware_plan.summary(),
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
) -> GenerationMetrics {
    let token_count = approximate_token_count(&draft.answer);
    let route_pressure = (1.0 - route_budget.attention_fraction).max(0.0) * 2.5;
    let perplexity = 4.0
        + (1.0 - report.quality) * 24.0
        + route_pressure
        + report.contradictions.len() as f32 * 3.5;

    GenerationMetrics {
        perplexity,
        semantic_consistency: report.quality,
        contradiction_count: report.contradictions.len(),
        token_count,
    }
}

fn replay_metrics(item: &ExperienceReplayItem) -> GenerationMetrics {
    match item.action {
        RewardAction::Reinforce => GenerationMetrics {
            perplexity: (6.0 + (1.0 - item.reward) * 8.0 + item.stream_windows as f32 * 0.03)
                .clamp(3.0, 18.0),
            semantic_consistency: item.quality.max(item.reward).clamp(0.0, 1.0),
            contradiction_count: item.contradiction_count,
            token_count: 64,
        },
        RewardAction::Penalize => GenerationMetrics {
            perplexity: (18.0 + (1.0 - item.reward) * 18.0 + item.stream_windows as f32 * 0.05)
                .clamp(12.0, 48.0),
            semantic_consistency: item.quality.min(item.reward).clamp(0.0, 1.0),
            contradiction_count: item.contradiction_count.max(1),
            token_count: 64,
        },
        RewardAction::Hold => GenerationMetrics {
            perplexity: 10.0,
            semantic_consistency: item.quality.clamp(0.0, 1.0),
            contradiction_count: item.contradiction_count,
            token_count: 64,
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
    use crate::hardware::DeviceClass;
    use crate::process_reward::ProcessRewardComponents;
    use crate::reflection::DraftToken;
    use crate::tiered_cache::TierMigrationAction;

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

    #[test]
    fn inference_exposes_tiered_cache_plan() {
        let mut cache = KvFusionCache::new();
        cache.store_or_fuse("Rust Noiron tiered memory", vec![1.0, 0.0, 0.0], 1.0);
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
        assert!(outcome.answer.contains("Recursive schedule"));
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
        assert!(outcome.report.store_as_memory);
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
            stored_memory_id: Some(memory_id),
            router_threshold_after: 0.55,
            stream_windows: 2,
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
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
            stored_memory_id: None,
            router_threshold_after: 0.55,
            stream_windows: 2,
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
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
        let state = engine.adaptive_state();

        let mut restored = NoironEngine::new();
        restored.restore_adaptive_state(state);

        assert_eq!(restored.router.observations(), engine.router.observations());
        assert!((restored.router.threshold() - engine.router.threshold()).abs() < 0.0001);
        assert!(
            (restored.hierarchy.current().local - engine.hierarchy.current().local).abs() < 0.0001
        );
    }
}
