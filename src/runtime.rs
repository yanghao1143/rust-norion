use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::engine::{GenerationContext, InferenceBackend};
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::reflection::{InferenceDraft, ReasoningStep};
use crate::router::RouteBudget;

#[derive(Debug, Clone)]
pub struct RuntimeRequest {
    pub prompt: String,
    pub profile: TaskProfile,
    pub memory_hints: Vec<String>,
    pub experience_hints: Vec<String>,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub max_tokens: usize,
}

impl RuntimeRequest {
    pub fn from_context(context: &GenerationContext<'_>, max_tokens: usize) -> Self {
        Self {
            prompt: context.prompt.to_owned(),
            profile: context.profile,
            memory_hints: context
                .memories
                .iter()
                .map(|memory| {
                    format!(
                        "{} similarity={:.3} strength={:.3}",
                        memory.key, memory.similarity, memory.strength
                    )
                })
                .collect(),
            experience_hints: context
                .experiences
                .iter()
                .map(|experience| {
                    format!(
                        "{} score={:.3} quality={:.3}",
                        experience.lesson, experience.score, experience.quality
                    )
                })
                .collect(),
            route_budget: context.route_budget,
            hierarchy: context.hierarchy,
            max_tokens,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeToken {
    pub text: String,
    pub logprob: Option<f32>,
    pub entropy: Option<f32>,
}

impl RuntimeToken {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            logprob: None,
            entropy: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResponse {
    pub answer: String,
    pub tokens: Vec<RuntimeToken>,
    pub trace: Vec<ReasoningStep>,
}

impl RuntimeResponse {
    pub fn new(answer: impl Into<String>) -> Self {
        Self {
            answer: answer.into(),
            tokens: Vec::new(),
            trace: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for RuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RuntimeError {}

pub trait ModelRuntime {
    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError>;
}

#[derive(Debug, Clone)]
pub struct RuntimeBackend<R> {
    runtime: R,
    max_tokens: usize,
    last_error: Option<RuntimeError>,
}

impl<R> RuntimeBackend<R> {
    pub fn new(runtime: R) -> Self {
        Self {
            runtime,
            max_tokens: 512,
            last_error: None,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens.max(1);
        self
    }

    pub fn runtime(&self) -> &R {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut R {
        &mut self.runtime
    }

    pub fn last_error(&self) -> Option<&RuntimeError> {
        self.last_error.as_ref()
    }
}

impl<R: ModelRuntime> InferenceBackend for RuntimeBackend<R> {
    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        let request = RuntimeRequest::from_context(&context, self.max_tokens);

        match self.runtime.generate(request) {
            Ok(response) => {
                self.last_error = None;
                let trace = if response.trace.is_empty() {
                    trace_from_tokens(&response.tokens)
                } else {
                    response.trace
                };
                InferenceDraft::new(response.answer, trace)
            }
            Err(error) => {
                self.last_error = Some(error.clone());
                InferenceDraft::new(
                    format!("Runtime backend error: {}", error.message()),
                    vec![ReasoningStep::new("runtime_error", error.message(), 0.0)],
                )
            }
        }
    }
}

fn trace_from_tokens(tokens: &[RuntimeToken]) -> Vec<ReasoningStep> {
    if tokens.is_empty() {
        return vec![ReasoningStep::new(
            "runtime",
            "generated without token trace",
            0.55,
        )];
    }

    let entropy_count = tokens
        .iter()
        .filter(|token| token.entropy.is_some())
        .count();
    let average_entropy =
        tokens.iter().filter_map(|token| token.entropy).sum::<f32>() / entropy_count.max(1) as f32;
    let confidence = (1.0 - average_entropy / 4.0).clamp(0.2, 0.95);

    vec![ReasoningStep::new(
        "runtime",
        format!("generated {} tokens", tokens.len()),
        confidence,
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience::ExperienceMatch;
    use crate::kv_cache::MemoryMatch;
    use crate::tiered_cache::TieredCachePlan;

    #[derive(Debug, Default, Clone)]
    struct MockRuntime {
        seen: Option<RuntimeRequest>,
    }

    impl ModelRuntime for MockRuntime {
        fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
            self.seen = Some(request.clone());
            let mut response = RuntimeResponse::new(format!(
                "runtime saw {} memories and {} experiences",
                request.memory_hints.len(),
                request.experience_hints.len()
            ));
            response.tokens = vec![RuntimeToken {
                text: "runtime".to_owned(),
                logprob: Some(-0.1),
                entropy: Some(0.2),
            }];
            Ok(response)
        }
    }

    #[test]
    fn runtime_backend_maps_context_to_request() {
        let memories = vec![MemoryMatch {
            id: 1,
            key: "kv memory".to_owned(),
            similarity: 0.8,
            strength: 1.2,
        }];
        let experiences = vec![ExperienceMatch {
            id: 1,
            prompt: "prompt".to_owned(),
            lesson: "lesson".to_owned(),
            quality: 0.9,
            score: 0.88,
        }];
        let tier_plan = TieredCachePlan::default();
        let context = GenerationContext {
            prompt: "build runtime",
            profile: TaskProfile::Coding,
            memories: &memories,
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            tier_plan: &tier_plan,
            experiences: &experiences,
        };
        let mut backend = RuntimeBackend::new(MockRuntime::default()).with_max_tokens(128);

        let draft = backend.generate(context);
        let seen = backend.runtime().seen.as_ref().unwrap();

        assert!(draft.answer.contains("1 memories and 1 experiences"));
        assert_eq!(seen.max_tokens, 128);
        assert_eq!(seen.memory_hints.len(), 1);
        assert_eq!(seen.experience_hints.len(), 1);
    }

    #[derive(Debug, Default, Clone)]
    struct FailingRuntime;

    impl ModelRuntime for FailingRuntime {
        fn generate(&mut self, _request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
            Err(RuntimeError::new("model file missing"))
        }
    }

    #[test]
    fn runtime_errors_become_low_confidence_drafts() {
        let tier_plan = TieredCachePlan::default();
        let context = GenerationContext {
            prompt: "build runtime",
            profile: TaskProfile::Coding,
            memories: &[],
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            tier_plan: &tier_plan,
            experiences: &[],
        };
        let mut backend = RuntimeBackend::new(FailingRuntime);

        let draft = backend.generate(context);

        assert!(draft.answer.contains("model file missing"));
        assert_eq!(draft.trace[0].confidence, 0.0);
        assert_eq!(
            backend.last_error().unwrap().message(),
            "model file missing"
        );
    }
}
