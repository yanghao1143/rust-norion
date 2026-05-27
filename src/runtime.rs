use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPromptMode {
    Stdin,
    Args,
}

#[derive(Debug, Clone)]
pub struct CommandRuntime {
    program: PathBuf,
    args: Vec<String>,
    prompt_mode: CommandPromptMode,
}

impl CommandRuntime {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            prompt_mode: CommandPromptMode::Stdin,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn prompt_mode(mut self, prompt_mode: CommandPromptMode) -> Self {
        self.prompt_mode = prompt_mode;
        self
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn command_args(&self) -> &[String] {
        &self.args
    }

    fn expanded_args(&self, request: &RuntimeRequest, prompt: &str) -> Vec<String> {
        self.args
            .iter()
            .map(|arg| {
                arg.replace("{prompt}", prompt)
                    .replace("{max_tokens}", &request.max_tokens.to_string())
                    .replace("{memory_hints}", &request.memory_hints.join("\n"))
                    .replace("{experience_hints}", &request.experience_hints.join("\n"))
            })
            .collect()
    }
}

impl ModelRuntime for CommandRuntime {
    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        let prompt = format_runtime_prompt(&request);
        let mut command = Command::new(&self.program);
        command.args(self.expanded_args(&request, &prompt));

        if self.prompt_mode == CommandPromptMode::Stdin {
            command.stdin(Stdio::piped());
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|error| {
            RuntimeError::new(format!(
                "failed to spawn runtime command {}: {error}",
                self.program.display()
            ))
        })?;

        if self.prompt_mode == CommandPromptMode::Stdin {
            let Some(mut stdin) = child.stdin.take() else {
                return Err(RuntimeError::new("runtime command did not expose stdin"));
            };
            stdin.write_all(prompt.as_bytes()).map_err(|error| {
                RuntimeError::new(format!("failed to write runtime prompt to stdin: {error}"))
            })?;
        }

        let output = child.wait_with_output().map_err(|error| {
            RuntimeError::new(format!("failed to wait for runtime command: {error}"))
        })?;
        if !output.status.success() {
            return Err(RuntimeError::new(format!(
                "runtime command exited with status {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let answer = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let mut response = RuntimeResponse::new(answer);
        response.trace = vec![ReasoningStep::new(
            "command_runtime",
            format!("executed {}", self.program.display()),
            0.72,
        )];
        Ok(response)
    }
}

fn format_runtime_prompt(request: &RuntimeRequest) -> String {
    format!(
        "Noiron runtime request\n\
         profile: {:?}\n\
         max_tokens: {}\n\
         route: threshold={:.3} attention_fraction={:.3} attention_tokens={} fast_tokens={}\n\
         hierarchy: global={:.3} local={:.3} convolution={:.3}\n\
         memory_hints:\n{}\n\
         experience_hints:\n{}\n\
         prompt:\n{}",
        request.profile,
        request.max_tokens,
        request.route_budget.threshold,
        request.route_budget.attention_fraction,
        request.route_budget.attention_tokens,
        request.route_budget.fast_tokens,
        request.hierarchy.global,
        request.hierarchy.local,
        request.hierarchy.convolution,
        bullet_list(&request.memory_hints),
        bullet_list(&request.experience_hints),
        request.prompt
    )
}

fn bullet_list(items: &[String]) -> String {
    if items.is_empty() {
        return "- none".to_owned();
    }

    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
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

    #[test]
    fn command_runtime_formats_prompt_and_expands_placeholders() {
        let runtime = CommandRuntime::new("runner")
            .arg("--prompt")
            .arg("{prompt}")
            .arg("--max")
            .arg("{max_tokens}")
            .prompt_mode(CommandPromptMode::Args);
        let request = sample_request();
        let prompt = format_runtime_prompt(&request);
        let args = runtime.expanded_args(&request, &prompt);

        assert!(prompt.contains("memory_hints"));
        assert!(prompt.contains("experience_hints"));
        assert!(args[1].contains("Noiron runtime request"));
        assert_eq!(args[3], "64");
    }

    #[test]
    fn command_runtime_reports_spawn_errors() {
        let mut runtime = CommandRuntime::new("__rust_norion_missing_command__")
            .prompt_mode(CommandPromptMode::Args);

        let error = runtime.generate(sample_request()).unwrap_err();

        assert!(error.message().contains("failed to spawn runtime command"));
    }

    fn sample_request() -> RuntimeRequest {
        RuntimeRequest {
            prompt: "build a command runtime".to_owned(),
            profile: TaskProfile::Coding,
            memory_hints: vec!["memory hint".to_owned()],
            experience_hints: vec!["experience hint".to_owned()],
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 2,
                fast_tokens: 1,
                attention_fraction: 0.66,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            max_tokens: 64,
        }
    }
}
