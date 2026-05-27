use std::io;
use std::path::Path;

use crate::experience::{ExperienceInput, ExperienceStore};
use crate::hierarchy::{HierarchyController, HierarchyWeights, TaskProfile};
use crate::kv_cache::{KvFusionCache, MemoryMatch};
use crate::reflection::{InferenceDraft, ReasoningStep, ReflectionReport, Reflector};
use crate::router::{GenerationMetrics, NoironRouter, RouteBudget};
use crate::tiered_cache::{TieredCachePlan, TieredCacheScheduler};
use crate::token_stream::{TokenStreamMonitor, TokenWindowReport};

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
}

pub trait InferenceBackend {
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
    pub stream_reports: Vec<TokenWindowReport>,
    pub used_memories: Vec<MemoryMatch>,
    pub stored_memory_id: Option<u64>,
    pub experience_id: u64,
    pub router_threshold_after: f32,
}

#[derive(Debug, Clone)]
pub struct NoironEngine {
    pub router: NoironRouter,
    pub cache: KvFusionCache,
    pub hierarchy: HierarchyController,
    pub tiered_cache: TieredCacheScheduler,
    pub stream_monitor: TokenStreamMonitor,
    pub experience: ExperienceStore,
    pub reflector: Reflector,
    embedder: TextEmbedder,
}

impl Default for NoironEngine {
    fn default() -> Self {
        Self {
            router: NoironRouter::new(),
            cache: KvFusionCache::new(),
            hierarchy: HierarchyController::new(),
            tiered_cache: TieredCacheScheduler::new(),
            stream_monitor: TokenStreamMonitor::default(),
            experience: ExperienceStore::new(),
            reflector: Reflector::new(),
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
        Ok(Self::with_cache(KvFusionCache::load_from_disk(path)?))
    }

    pub fn load_state(
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let mut engine = Self::load_memory(memory_path)?;
        engine.experience = ExperienceStore::load_from_disk_kv(experience_path)?;
        Ok(engine)
    }

    pub fn save_memory(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.cache.save_to_disk(path)
    }

    pub fn save_experience(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.experience.save_to_disk_kv(path)
    }

    pub fn infer<B: InferenceBackend>(
        &mut self,
        request: InferenceRequest,
        backend: &mut B,
    ) -> InferenceOutcome {
        let query_vector = self.embedder.embed(&request.prompt);
        let used_memories = self.cache.lookup(&query_vector, 4);
        let tier_plan = self.tiered_cache.plan(self.cache.entries(), &used_memories);
        let route_budget = self.router.budget_for_prompt(&request.prompt);
        let hierarchy = self.hierarchy.adapt_to_profile(request.profile);

        let draft = backend.generate(GenerationContext {
            prompt: &request.prompt,
            profile: request.profile,
            memories: &used_memories,
            route_budget,
            hierarchy,
            tier_plan: &tier_plan,
        });
        let report = self.reflector.reflect(&request.prompt, &draft);
        let metrics = metrics_from_report(&draft, &report, route_budget);
        let stream_reports = self.stream_monitor.observe_generated(
            &mut self.router,
            &draft.answer,
            report.quality,
            report.contradictions.len(),
        );

        let stored_memory_id = if report.store_as_memory {
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

        for memory in &used_memories {
            if report.store_as_memory {
                self.cache.reinforce(memory.id, report.quality);
            } else {
                self.cache.penalize(memory.id, 1.0 - report.quality);
            }
        }

        self.router.observe(metrics);
        let hierarchy = self.hierarchy.observe(request.profile, metrics);
        let router_threshold_after = self.router.threshold();
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
        });

        InferenceOutcome {
            answer: report.revised_answer.clone(),
            report,
            metrics,
            route_budget,
            hierarchy,
            tier_plan,
            stream_reports,
            used_memories,
            stored_memory_id,
            experience_id,
            router_threshold_after,
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

        let answer = format!(
            "Prototype inference result: keep Noiron as a control layer around the model backend. \
             Use entropy routing for attention decisions, reinforced KV fusion for local memory, \
             task-aware hierarchy weights for compute allocation, and reflection to score each draft \
             before storing it. Profile hint: {profile_hint}. Prompt anchor: {}. Memory hints: {memory_summary}. \
             Route budget: {:.0}% attention, {} fast tokens, {} attention tokens. \
             Tier plan: {} hot GPU, {} warm RAM, {} cold disk memories.",
            compact(&context.prompt, 120),
            context.route_budget.attention_fraction * 100.0,
            context.route_budget.fast_tokens,
            context.route_budget.attention_tokens,
            tier_counts.hot_gpu,
            tier_counts.warm_ram,
            tier_counts.cold_disk
        );

        InferenceDraft::new(
            answer,
            vec![
                ReasoningStep::new(
                    "route",
                    "estimated token entropy and selected attention budget",
                    0.82,
                ),
                ReasoningStep::new("memory", "looked up similar reinforced KV memories", 0.78),
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
        assert!(outcome.answer.contains("Tier plan"));
    }
}
