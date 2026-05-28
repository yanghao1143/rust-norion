use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::engine::{GenerationContext, InferenceBackend};
use crate::experience::ExperienceMatch;
use crate::hardware::{HardwarePlan, RuntimeAdapterHint};
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::kv_exchange::RuntimeKvBlock;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::reflection::{DraftToken, InferenceDraft, ReasoningStep, RuntimeDiagnostics};
use crate::router::RouteBudget;
use crate::runtime_manifest::{
    TransformerRuntimeArchitecture, default_transformer_runtime_architecture,
};
use crate::tiered_cache::MemoryTier;
use crate::transformer::{AttentionKind, TransformerRefactorPlan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMetadata {
    pub model_id: String,
    pub tokenizer: String,
    pub native_context_window: usize,
    pub embedding_dimensions: usize,
    pub supports_kv_import: bool,
    pub supports_kv_export: bool,
    pub max_kv_import_blocks: usize,
    pub max_kv_export_blocks: usize,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
}

impl RuntimeMetadata {
    pub fn new(
        model_id: impl Into<String>,
        tokenizer: impl Into<String>,
        native_context_window: usize,
        embedding_dimensions: usize,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            tokenizer: tokenizer.into(),
            native_context_window,
            embedding_dimensions,
            supports_kv_import: false,
            supports_kv_export: false,
            max_kv_import_blocks: 0,
            max_kv_export_blocks: 0,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 4,
        }
    }

    pub fn with_kv_exchange(mut self, import: bool, export: bool) -> Self {
        self.supports_kv_import = import;
        self.supports_kv_export = export;
        self.max_kv_import_blocks = if import {
            self.max_kv_import_blocks.max(8)
        } else {
            0
        };
        self.max_kv_export_blocks = if export {
            self.max_kv_export_blocks.max(4)
        } else {
            0
        };
        self
    }

    pub fn with_kv_limits(mut self, max_import_blocks: usize, max_export_blocks: usize) -> Self {
        self.max_kv_import_blocks = if self.supports_kv_import {
            max_import_blocks.max(1)
        } else {
            0
        };
        self.max_kv_export_blocks = if self.supports_kv_export {
            max_export_blocks.max(1)
        } else {
            0
        };
        self
    }

    pub fn with_kv_precision(mut self, hot_bits: u8, cold_bits: u8) -> Self {
        self.hot_kv_precision_bits = if matches!(hot_bits, 4 | 8) {
            hot_bits
        } else {
            8
        };
        self.cold_kv_precision_bits = if matches!(cold_bits, 4 | 8) {
            cold_bits
        } else {
            4
        };
        self
    }

    pub fn summary(&self) -> String {
        format!(
            "model_id={} tokenizer={} native_context_window={} embedding_dimensions={} kv_import={} kv_export={} max_kv_import_blocks={} max_kv_export_blocks={} kv_bits={}/{}",
            self.model_id,
            self.tokenizer,
            self.native_context_window,
            self.embedding_dimensions,
            self.supports_kv_import,
            self.supports_kv_export,
            self.max_kv_import_blocks,
            self.max_kv_export_blocks,
            self.hot_kv_precision_bits,
            self.cold_kv_precision_bits
        )
    }
}

impl Default for RuntimeMetadata {
    fn default() -> Self {
        Self {
            model_id: "unknown-self-developed-runtime".to_owned(),
            tokenizer: "unknown".to_owned(),
            native_context_window: 0,
            embedding_dimensions: 0,
            supports_kv_import: false,
            supports_kv_export: false,
            max_kv_import_blocks: 0,
            max_kv_export_blocks: 0,
            hot_kv_precision_bits: 8,
            cold_kv_precision_bits: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTokenId {
    pub id: u32,
    pub text: String,
}

impl RuntimeTokenId {
    pub fn new(id: u32, text: impl Into<String>) -> Self {
        Self {
            id,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeEmbedding {
    pub dimensions: usize,
    pub values: Vec<f32>,
}

impl RuntimeEmbedding {
    pub fn new(values: Vec<f32>) -> Self {
        Self {
            dimensions: values.len(),
            values,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeRequest {
    pub prompt: String,
    pub profile: TaskProfile,
    pub runtime_metadata: RuntimeMetadata,
    pub runtime_architecture: TransformerRuntimeArchitecture,
    pub memory_hints: Vec<String>,
    pub infini_memory_hints: Vec<String>,
    pub experience_hints: Vec<String>,
    pub runtime_adapter_observations: Vec<RuntimeAdapterObservation>,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub transformer_plan: TransformerRefactorPlan,
    pub recursive_schedule: RecursiveSchedule,
    pub hardware_plan: HardwarePlan,
    pub max_tokens: usize,
}

impl RuntimeRequest {
    pub fn from_context(
        context: &GenerationContext<'_>,
        max_tokens: usize,
        runtime_metadata: RuntimeMetadata,
        runtime_architecture: TransformerRuntimeArchitecture,
    ) -> Self {
        let runtime_adapter_observations = RuntimeAdapterObservation::from_experiences(
            context.experiences,
            &runtime_metadata.model_id,
        );

        Self {
            prompt: context.prompt.to_owned(),
            profile: context.profile,
            runtime_metadata,
            runtime_architecture,
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
            infini_memory_hints: context
                .infini_memory_plan
                .local_window()
                .iter()
                .chain(context.infini_memory_plan.global_memory())
                .map(|memory| {
                    format!(
                        "{:?}:{} score={:.3} tokens={} reason={}",
                        memory.scope,
                        memory.key,
                        memory.score,
                        memory.estimated_tokens,
                        memory.reason
                    )
                })
                .collect(),
            experience_hints: context
                .experiences
                .iter()
                .map(|experience| {
                    let gist_hints = if experience.gist_hints.is_empty() {
                        "none".to_owned()
                    } else {
                        experience.gist_hints.join(" | ")
                    };
                    format!(
                        "{} score={:.3} quality={:.3} reward={:.3}/{} gist_hints={}",
                        experience.lesson,
                        experience.score,
                        experience.quality,
                        experience.process_reward,
                        experience.reward_action.as_str(),
                        gist_hints
                    )
                })
                .collect(),
            runtime_adapter_observations,
            route_budget: context.route_budget,
            hierarchy: context.hierarchy,
            transformer_plan: context.transformer_plan.clone(),
            recursive_schedule: context.recursive_schedule.clone(),
            hardware_plan: context.hardware_plan.clone(),
            max_tokens,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeAdapterObservation {
    pub adapter: RuntimeAdapterHint,
    pub score: f32,
    pub reward: f32,
    pub quality: f32,
    pub forward_energy: Option<f32>,
    pub kv_influence: Option<f32>,
    pub experience_id: u64,
}

impl RuntimeAdapterObservation {
    pub fn new(
        adapter: RuntimeAdapterHint,
        score: f32,
        reward: f32,
        quality: f32,
        forward_energy: Option<f32>,
        kv_influence: Option<f32>,
        experience_id: u64,
    ) -> Self {
        Self {
            adapter,
            score: score.clamp(0.0, 1.0),
            reward: reward.clamp(0.0, 1.0),
            quality: quality.clamp(0.0, 1.0),
            forward_energy: forward_energy.filter(|value| value.is_finite()),
            kv_influence: kv_influence.filter(|value| value.is_finite()),
            experience_id,
        }
    }

    pub fn from_experiences(experiences: &[ExperienceMatch], runtime_model_id: &str) -> Vec<Self> {
        let mut observations = experiences
            .iter()
            .filter(|experience| {
                runtime_model_id.is_empty()
                    || experience
                        .runtime_model_id
                        .as_deref()
                        .map(|model_id| model_id == runtime_model_id)
                        .unwrap_or(true)
            })
            .filter_map(|experience| {
                let adapter =
                    parse_runtime_adapter_hint(experience.runtime_selected_adapter.as_deref()?)?;
                let base = experience.score * 0.38
                    + experience.process_reward * 0.34
                    + experience.quality * 0.22;
                let kv_bonus = experience
                    .runtime_kv_influence
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0)
                    * 0.06;
                let energy_penalty = experience
                    .runtime_forward_energy
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0)
                    * 0.04;
                Some(Self::new(
                    adapter,
                    base + kv_bonus - energy_penalty,
                    experience.process_reward,
                    experience.quality,
                    experience.runtime_forward_energy,
                    experience.runtime_kv_influence,
                    experience.id,
                ))
            })
            .collect::<Vec<_>>();

        observations.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.experience_id.cmp(&right.experience_id))
        });
        observations.truncate(6);
        observations
    }

    pub fn summary(&self) -> String {
        format!(
            "adapter={} score={:.3} reward={:.3} quality={:.3} forward_energy={} kv_influence={} experience={}",
            self.adapter.as_str(),
            self.score,
            self.reward,
            self.quality,
            option_f32_display(self.forward_energy),
            option_f32_display(self.kv_influence),
            self.experience_id
        )
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
    pub diagnostics: RuntimeDiagnostics,
}

impl RuntimeResponse {
    pub fn new(answer: impl Into<String>) -> Self {
        Self {
            answer: answer.into(),
            tokens: Vec::new(),
            trace: Vec::new(),
            diagnostics: RuntimeDiagnostics::default(),
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: RuntimeDiagnostics) -> Self {
        self.diagnostics = diagnostics;
        self
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
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata::default()
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        let metadata = self.metadata();
        default_transformer_runtime_architecture(
            metadata.native_context_window,
            metadata.embedding_dimensions,
        )
    }

    fn tokenize(&self, prompt: &str) -> Result<Vec<RuntimeTokenId>, RuntimeError> {
        Ok(prompt
            .split_whitespace()
            .enumerate()
            .map(|(index, text)| RuntimeTokenId::new(index as u32, text))
            .collect())
    }

    fn embed(&self, _tokens: &[RuntimeTokenId]) -> Result<RuntimeEmbedding, RuntimeError> {
        Ok(RuntimeEmbedding::empty())
    }

    fn embed_text(&self, text: &str) -> Result<RuntimeEmbedding, RuntimeError> {
        let tokens = self.tokenize(text)?;
        self.embed(&tokens)
    }

    fn import_kv(&mut self, _blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        Ok(0)
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(Vec::new())
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPromptMode {
    Stdin,
    Args,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandWireFormat {
    Text,
    Json,
}

impl CommandWireFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandRuntime {
    program: PathBuf,
    args: Vec<String>,
    prompt_mode: CommandPromptMode,
    wire_format: CommandWireFormat,
    metadata: RuntimeMetadata,
    architecture: Option<TransformerRuntimeArchitecture>,
}

impl CommandRuntime {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            prompt_mode: CommandPromptMode::Stdin,
            wire_format: CommandWireFormat::Text,
            metadata: RuntimeMetadata::default(),
            architecture: None,
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

    pub fn wire_format(mut self, wire_format: CommandWireFormat) -> Self {
        self.wire_format = wire_format;
        self
    }

    pub fn with_metadata(mut self, metadata: RuntimeMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_architecture(mut self, architecture: TransformerRuntimeArchitecture) -> Self {
        self.architecture = Some(architecture);
        self
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn command_args(&self) -> &[String] {
        &self.args
    }

    fn expanded_args(&self, request: &RuntimeRequest, payload: &str) -> Vec<String> {
        self.args
            .iter()
            .map(|arg| {
                arg.replace("{prompt}", payload)
                    .replace("{runtime_payload}", payload)
                    .replace("{wire_format}", self.wire_format.as_str())
                    .replace("{max_tokens}", &request.max_tokens.to_string())
                    .replace("{memory_hints}", &request.memory_hints.join("\n"))
                    .replace(
                        "{infini_memory_hints}",
                        &request.infini_memory_hints.join("\n"),
                    )
                    .replace("{experience_hints}", &request.experience_hints.join("\n"))
                    .replace(
                        "{runtime_adapter_observations}",
                        &request
                            .runtime_adapter_observations
                            .iter()
                            .map(RuntimeAdapterObservation::summary)
                            .collect::<Vec<_>>()
                            .join("\n"),
                    )
                    .replace(
                        "{recursive_schedule}",
                        &request.recursive_schedule.summary(),
                    )
                    .replace("{runtime_metadata}", &request.runtime_metadata.summary())
                    .replace(
                        "{runtime_architecture}",
                        &request.runtime_architecture.summary(),
                    )
            })
            .collect()
    }
}

impl ModelRuntime for CommandRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        self.metadata.clone()
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        self.architecture.unwrap_or_else(|| {
            default_transformer_runtime_architecture(
                self.metadata.native_context_window,
                self.metadata.embedding_dimensions,
            )
        })
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        let payload = format_runtime_payload(&request, self.wire_format);
        let mut command = Command::new(&self.program);
        command.args(self.expanded_args(&request, &payload));

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
            stdin.write_all(payload.as_bytes()).map_err(|error| {
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

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let mut response = match self.wire_format {
            CommandWireFormat::Text => RuntimeResponse::new(stdout),
            CommandWireFormat::Json => parse_runtime_response_json(&stdout)?,
        };
        response.trace.push(ReasoningStep::new(
            "command_runtime",
            format!("executed {}", self.program.display()),
            0.72,
        ));
        Ok(response)
    }
}

fn format_runtime_payload(request: &RuntimeRequest, wire_format: CommandWireFormat) -> String {
    match wire_format {
        CommandWireFormat::Text => format_runtime_prompt(request),
        CommandWireFormat::Json => runtime_request_json(request),
    }
}

fn format_runtime_prompt(request: &RuntimeRequest) -> String {
    let transformer_counts = request.transformer_plan.counts();
    format!(
        "Noiron runtime request\n\
         runtime: {}\n\
         runtime_architecture: {}\n\
         profile: {:?}\n\
         max_tokens: {}\n\
         route: threshold={:.3} attention_fraction={:.3} attention_tokens={} fast_tokens={}\n\
         hierarchy: global={:.3} local={:.3} convolution={:.3}\n\
         transformer: template={} global_layers={} local_layers={} convolution_layers={}\n\
         recursive: {}\n\
         hardware: {}\n\
         memory_hints:\n{}\n\
         infini_memory_hints:\n{}\n\
         experience_hints:\n{}\n\
         runtime_adapter_observations:\n{}\n\
         prompt:\n{}",
        request.runtime_metadata.summary(),
        request.runtime_architecture.summary(),
        request.profile,
        request.max_tokens,
        request.route_budget.threshold,
        request.route_budget.attention_fraction,
        request.route_budget.attention_tokens,
        request.route_budget.fast_tokens,
        request.hierarchy.global,
        request.hierarchy.local,
        request.hierarchy.convolution,
        request.transformer_plan.template_name(),
        transformer_counts.global,
        transformer_counts.local,
        transformer_counts.convolution,
        request.recursive_schedule.summary(),
        request.hardware_plan.summary(),
        bullet_list(&request.memory_hints),
        bullet_list(&request.infini_memory_hints),
        bullet_list(&request.experience_hints),
        bullet_runtime_adapter_observations(&request.runtime_adapter_observations),
        request.prompt
    )
}

pub fn runtime_request_json(request: &RuntimeRequest) -> String {
    let transformer_counts = request.transformer_plan.counts();
    let transformer_layers = request
        .transformer_plan
        .layers
        .iter()
        .map(|layer| {
            format!(
                "{{\"layer_index\":{},\"attention\":{},\"compute_fraction\":{:.6},\"window_size\":{}}}",
                layer.layer_index,
                json_string(attention_kind_str(layer.attention)),
                layer.compute_fraction,
                layer.window_size
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let recursive_chunks = request
        .recursive_schedule
        .chunks
        .iter()
        .map(|chunk| {
            format!(
                "{{\"index\":{},\"start_token\":{},\"end_token\":{},\"estimated_tokens\":{},\"overlap_left\":{},\"overlap_right\":{}}}",
                chunk.index,
                chunk.start_token,
                chunk.end_token,
                chunk.estimated_tokens,
                chunk.overlap_left,
                chunk.overlap_right
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let recursive_merge_rounds = request
        .recursive_schedule
        .merge_rounds
        .iter()
        .map(|round| {
            format!(
                "{{\"round\":{},\"input_units\":{},\"output_units\":{}}}",
                round.round, round.input_units, round.output_units
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let recursive_execution_waves = request
        .recursive_schedule
        .execution_waves
        .iter()
        .map(|wave| {
            format!(
                "{{\"wave\":{},\"start_chunk\":{},\"end_chunk\":{},\"chunk_count\":{}}}",
                wave.wave, wave.start_chunk, wave.end_chunk, wave.chunk_count
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let adapter_hints = request
        .hardware_plan
        .execution
        .adapter_hints
        .iter()
        .map(|adapter| adapter.as_str())
        .collect::<Vec<_>>();
    let runtime_adapter_observations = request
        .runtime_adapter_observations
        .iter()
        .map(runtime_adapter_observation_json)
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "{{\
         \"schema\":{},\
         \"prompt\":{},\
         \"profile\":{},\
         \"max_tokens\":{},\
         \"runtime\":{{\
         \"model_id\":{},\
         \"tokenizer\":{},\
         \"native_context_window\":{},\
         \"embedding_dimensions\":{},\
         \"supports_kv_import\":{},\
         \"supports_kv_export\":{},\
         \"max_kv_import_blocks\":{},\
         \"max_kv_export_blocks\":{},\
         \"hot_kv_precision_bits\":{},\
         \"cold_kv_precision_bits\":{}\
         }},\
         \"runtime_architecture\":{{\
         \"layer_count\":{},\
         \"hidden_size\":{},\
         \"attention_heads\":{},\
         \"kv_heads\":{},\
         \"local_window_tokens\":{}\
         }},\
         \"route\":{{\
         \"threshold\":{:.6},\
         \"attention_fraction\":{:.6},\
         \"attention_tokens\":{},\
         \"fast_tokens\":{}\
         }},\
         \"hierarchy\":{{\
         \"global\":{:.6},\
         \"local\":{:.6},\
         \"convolution\":{:.6}\
         }},\
         \"transformer\":{{\
         \"template\":{},\
         \"global_layers\":{},\
         \"local_layers\":{},\
         \"convolution_layers\":{},\
         \"layers\":[{}]\
         }},\
         \"recursive\":{{\
         \"required\":{},\
         \"prompt_tokens\":{},\
         \"native_window\":{},\
         \"chunk_tokens\":{},\
         \"overlap_tokens\":{},\
         \"merge_fan_in\":{},\
         \"max_parallel_chunks\":{},\
         \"chunks\":[{}],\
         \"merge_rounds\":[{}],\
         \"execution_waves\":[{}]\
         }},\
         \"hardware\":{{\
         \"device\":{},\
         \"tier\":{},\
         \"pressure\":{:.6},\
         \"latency_budget_ms\":{},\
         \"local_kv_token_budget\":{},\
         \"global_kv_token_budget\":{},\
         \"execution\":{{\
         \"primary_lane\":{},\
         \"fallback_lane\":{},\
         \"memory_mode\":{},\
         \"adapter_hints\":{},\
         \"max_parallel_chunks\":{},\
         \"kv_prefetch_blocks\":{},\
         \"hot_kv_precision_bits\":{},\
         \"cold_kv_precision_bits\":{},\
         \"allow_disk_spill\":{}\
         }},\
         \"notes\":{}\
         }},\
         \"memory_hints\":{},\
         \"infini_memory_hints\":{},\
         \"experience_hints\":{},\
         \"runtime_adapter_observations\":[{}]\
         }}",
        json_string("rust-norion-runtime-request-v1"),
        json_string(&request.prompt),
        json_string(task_profile_str(request.profile)),
        request.max_tokens,
        json_string(&request.runtime_metadata.model_id),
        json_string(&request.runtime_metadata.tokenizer),
        request.runtime_metadata.native_context_window,
        request.runtime_metadata.embedding_dimensions,
        request.runtime_metadata.supports_kv_import,
        request.runtime_metadata.supports_kv_export,
        request.runtime_metadata.max_kv_import_blocks,
        request.runtime_metadata.max_kv_export_blocks,
        request.runtime_metadata.hot_kv_precision_bits,
        request.runtime_metadata.cold_kv_precision_bits,
        request.runtime_architecture.layer_count,
        request.runtime_architecture.hidden_size,
        request.runtime_architecture.attention_heads,
        request.runtime_architecture.kv_heads,
        request.runtime_architecture.local_window_tokens,
        request.route_budget.threshold,
        request.route_budget.attention_fraction,
        request.route_budget.attention_tokens,
        request.route_budget.fast_tokens,
        request.hierarchy.global,
        request.hierarchy.local,
        request.hierarchy.convolution,
        json_string(request.transformer_plan.template_name()),
        transformer_counts.global,
        transformer_counts.local,
        transformer_counts.convolution,
        transformer_layers,
        request.recursive_schedule.requires_recursion,
        request.recursive_schedule.prompt_tokens,
        request.recursive_schedule.native_window_tokens,
        request.recursive_schedule.chunk_tokens,
        request.recursive_schedule.overlap_tokens,
        request.recursive_schedule.merge_fan_in,
        request.recursive_schedule.max_parallel_chunks,
        recursive_chunks,
        recursive_merge_rounds,
        recursive_execution_waves,
        json_string(request.hardware_plan.device.as_str()),
        json_string(request.hardware_plan.tier.as_str()),
        request.hardware_plan.pressure,
        option_u64_json(request.hardware_plan.latency_budget_ms),
        request.hardware_plan.local_kv_token_budget,
        request.hardware_plan.global_kv_token_budget,
        json_string(request.hardware_plan.execution.primary_lane.as_str()),
        json_string(request.hardware_plan.execution.fallback_lane.as_str()),
        json_string(request.hardware_plan.execution.memory_mode.as_str()),
        json_str_array(adapter_hints),
        request.hardware_plan.execution.max_parallel_chunks,
        request.hardware_plan.execution.kv_prefetch_blocks,
        request.hardware_plan.execution.hot_kv_precision_bits,
        request.hardware_plan.execution.cold_kv_precision_bits,
        request.hardware_plan.execution.allow_disk_spill,
        json_str_array(request.hardware_plan.notes.iter().map(String::as_str)),
        json_str_array(request.memory_hints.iter().map(String::as_str)),
        json_str_array(request.infini_memory_hints.iter().map(String::as_str)),
        json_str_array(request.experience_hints.iter().map(String::as_str)),
        runtime_adapter_observations
    )
}

pub fn parse_runtime_response_json(payload: &str) -> Result<RuntimeResponse, RuntimeError> {
    let schema = extract_json_string_field(payload, "schema")
        .ok_or_else(|| RuntimeError::new("runtime response JSON must include a schema string"))?;
    if schema != "rust-norion-runtime-response-v1" {
        return Err(RuntimeError::new(
            "runtime response schema must be rust-norion-runtime-response-v1",
        ));
    }
    let answer = extract_json_string_field(payload, "answer")
        .ok_or_else(|| RuntimeError::new("runtime response JSON must include an answer string"))?;
    if answer.trim().is_empty() {
        return Err(RuntimeError::new(
            "runtime response JSON must include a non-empty answer",
        ));
    }

    let mut response = RuntimeResponse::new(answer);
    response.tokens = extract_json_array_field(payload, "tokens")
        .map(split_json_objects)
        .unwrap_or_default()
        .iter()
        .map(|token| RuntimeToken {
            text: extract_json_string_field(token, "text").unwrap_or_default(),
            logprob: extract_json_number_field(token, "logprob"),
            entropy: extract_json_number_field(token, "entropy"),
        })
        .filter(|token| !token.text.is_empty())
        .collect();
    response.trace = extract_json_array_field(payload, "trace")
        .map(split_json_objects)
        .unwrap_or_default()
        .iter()
        .map(|step| {
            ReasoningStep::new(
                extract_json_string_field(step, "label").unwrap_or_else(|| "runtime".to_owned()),
                extract_json_string_field(step, "content").unwrap_or_default(),
                extract_json_number_field(step, "confidence").unwrap_or(0.5),
            )
        })
        .collect();
    response.diagnostics = extract_json_object_field(payload, "diagnostics")
        .map(parse_runtime_diagnostics)
        .unwrap_or_default();
    Ok(response)
}

fn parse_runtime_diagnostics(payload: &str) -> RuntimeDiagnostics {
    RuntimeDiagnostics {
        model_id: extract_json_string_field(payload, "model_id"),
        selected_adapter: extract_json_string_field(payload, "selected_adapter"),
        layer_count: extract_json_usize_field(payload, "layer_count").unwrap_or(0),
        hidden_size: extract_json_usize_field(payload, "hidden_size").unwrap_or(0),
        local_window_tokens: extract_json_usize_field(payload, "local_window_tokens").unwrap_or(0),
        forward_energy: extract_json_finite_number_field(payload, "forward_energy"),
        kv_influence: extract_json_finite_number_field(payload, "kv_influence"),
        imported_kv_blocks: extract_json_usize_field(payload, "imported_kv_blocks").unwrap_or(0),
        exported_kv_blocks: extract_json_usize_field(payload, "exported_kv_blocks").unwrap_or(0),
    }
}

fn task_profile_str(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn attention_kind_str(attention: AttentionKind) -> &'static str {
    match attention {
        AttentionKind::Global => "global",
        AttentionKind::LocalWindow => "local_window",
        AttentionKind::ConvolutionalFusion => "convolutional_fusion",
    }
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_f32_json(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "null".to_owned())
}

fn option_f32_display(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "none".to_owned())
}

fn runtime_adapter_observation_json(observation: &RuntimeAdapterObservation) -> String {
    format!(
        "{{\"adapter\":{},\"score\":{:.6},\"reward\":{:.6},\"quality\":{:.6},\"forward_energy\":{},\"kv_influence\":{},\"experience_id\":{}}}",
        json_string(observation.adapter.as_str()),
        observation.score,
        observation.reward,
        observation.quality,
        option_f32_json(observation.forward_energy),
        option_f32_json(observation.kv_influence),
        observation.experience_id
    )
}

fn json_str_array<'a, I>(items: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    let values = items
        .into_iter()
        .map(json_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn extract_json_string_field(source: &str, field: &str) -> Option<String> {
    extract_json_field(source, field).and_then(parse_json_string)
}

fn extract_json_number_field(source: &str, field: &str) -> Option<f32> {
    extract_json_field(source, field).and_then(|value| value.trim().parse::<f32>().ok())
}

fn extract_json_finite_number_field(source: &str, field: &str) -> Option<f32> {
    extract_json_number_field(source, field).filter(|value| value.is_finite())
}

fn extract_json_usize_field(source: &str, field: &str) -> Option<usize> {
    extract_json_field(source, field).and_then(|value| value.trim().parse::<usize>().ok())
}

fn extract_json_array_field<'a>(source: &'a str, field: &str) -> Option<&'a str> {
    extract_json_field(source, field).filter(|value| value.trim_start().starts_with('['))
}

fn extract_json_object_field<'a>(source: &'a str, field: &str) -> Option<&'a str> {
    extract_json_field(source, field).filter(|value| value.trim_start().starts_with('{'))
}

fn extract_json_field<'a>(source: &'a str, field: &str) -> Option<&'a str> {
    let needle = json_string(field);
    let key_start = source.find(&needle)?;
    let after_key = key_start + needle.len();
    let colon_offset = source[after_key..].find(':')?;
    let mut value_start = after_key + colon_offset + 1;
    while source[value_start..]
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
    {
        value_start += source[value_start..].chars().next()?.len_utf8();
    }
    let value_end = json_value_end(source, value_start)?;
    Some(&source[value_start..value_end])
}

fn split_json_objects(array_value: &str) -> Vec<&str> {
    let trimmed = array_value.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Vec::new();
    }

    let inner_start = 1;
    let inner_end = trimmed.len().saturating_sub(1);
    let inner = &trimmed[inner_start..inner_end];
    let mut objects = Vec::new();
    let mut index = 0;

    while index < inner.len() {
        while index < inner.len() {
            let Some(ch) = inner[index..].chars().next() else {
                break;
            };
            if ch == ',' || ch.is_whitespace() {
                index += ch.len_utf8();
            } else {
                break;
            }
        }
        if index >= inner.len() {
            break;
        }
        if !inner[index..].starts_with('{') {
            break;
        }
        let Some(end) = json_value_end(inner, index) else {
            break;
        };
        objects.push(&inner[index..end]);
        index = end;
    }

    objects
}

fn json_value_end(source: &str, start: usize) -> Option<usize> {
    let first = source[start..].chars().next()?;
    match first {
        '"' => scan_json_string_end(source, start),
        '[' => scan_json_compound_end(source, start, '[', ']'),
        '{' => scan_json_compound_end(source, start, '{', '}'),
        _ => {
            let mut end = start;
            while end < source.len() {
                let ch = source[end..].chars().next()?;
                if ch == ',' || ch == '}' || ch == ']' || ch.is_whitespace() {
                    break;
                }
                end += ch.len_utf8();
            }
            Some(end)
        }
    }
}

fn scan_json_string_end(source: &str, start: usize) -> Option<usize> {
    let mut escaped = false;
    let mut index = start + 1;
    while index < source.len() {
        let ch = source[index..].chars().next()?;
        index += ch.len_utf8();
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(index);
        }
    }
    None
}

fn scan_json_compound_end(source: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0_usize;
    let mut index = start;
    while index < source.len() {
        let ch = source[index..].chars().next()?;
        if ch == '"' {
            index = scan_json_string_end(source, index)?;
            continue;
        }
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index + ch.len_utf8());
            }
        }
        index += ch.len_utf8();
    }
    None
}

fn parse_json_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('"') || !trimmed.ends_with('"') {
        return None;
    }

    let mut out = String::new();
    let mut chars = trimmed[1..trimmed.len().saturating_sub(1)].chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next()? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'b' => out.push('\u{0008}'),
            'f' => out.push('\u{000c}'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'u' => {
                let code = (0..4).filter_map(|_| chars.next()).collect::<String>();
                let value = u32::from_str_radix(&code, 16).ok()?;
                out.push(char::from_u32(value)?);
            }
            other => out.push(other),
        }
    }
    Some(out)
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

fn bullet_runtime_adapter_observations(items: &[RuntimeAdapterObservation]) -> String {
    if items.is_empty() {
        return "- none".to_owned();
    }

    items
        .iter()
        .map(|item| format!("- {}", item.summary()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_runtime_adapter_hint(value: &str) -> Option<RuntimeAdapterHint> {
    match value {
        "portable-rust" => Some(RuntimeAdapterHint::PortableRust),
        "cpu-simd" => Some(RuntimeAdapterHint::CpuSimd),
        "wgpu" => Some(RuntimeAdapterHint::Wgpu),
        "webgpu" => Some(RuntimeAdapterHint::WebGpu),
        "vulkan" => Some(RuntimeAdapterHint::Vulkan),
        "metal" => Some(RuntimeAdapterHint::Metal),
        "cuda" => Some(RuntimeAdapterHint::Cuda),
        "rocm" => Some(RuntimeAdapterHint::Rocm),
        "oneapi" => Some(RuntimeAdapterHint::OneApi),
        "directml" => Some(RuntimeAdapterHint::DirectMl),
        "coreml" => Some(RuntimeAdapterHint::CoreMl),
        "nnapi" => Some(RuntimeAdapterHint::Nnapi),
        "qnn" => Some(RuntimeAdapterHint::Qnn),
        "openvino" => Some(RuntimeAdapterHint::OpenVino),
        "cann" => Some(RuntimeAdapterHint::Cann),
        "mlu" => Some(RuntimeAdapterHint::Mlu),
        "rknn" => Some(RuntimeAdapterHint::Rknn),
        "multi-device" => Some(RuntimeAdapterHint::MultiDevice),
        "custom-accelerator" => Some(RuntimeAdapterHint::CustomAccelerator),
        _ => None,
    }
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
    fn runtime_native_context_window(&self) -> Option<usize> {
        let window = self.runtime.metadata().native_context_window;
        (window > 0).then_some(window)
    }

    fn embed_text(&mut self, text: &str) -> Option<Vec<f32>> {
        match self.runtime.embed_text(text) {
            Ok(embedding) if !embedding.values.is_empty() => Some(embedding.values),
            Ok(_) => None,
            Err(error) => {
                self.last_error = Some(error);
                None
            }
        }
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        let runtime_metadata = self.runtime.metadata();
        let runtime_architecture = self.runtime.architecture();
        let import_blocks = runtime_kv_blocks_from_context(&context, &runtime_metadata);
        let imported_kv_blocks = if runtime_metadata.supports_kv_import && !import_blocks.is_empty()
        {
            match self.runtime.import_kv(&import_blocks) {
                Ok(count) => count,
                Err(error) => {
                    self.last_error = Some(error.clone());
                    return InferenceDraft::new(
                        format!("Runtime backend error: {}", error.message()),
                        vec![ReasoningStep::new(
                            "runtime_kv_import_error",
                            error.message(),
                            0.0,
                        )],
                    );
                }
            }
        } else {
            0
        };
        let request = RuntimeRequest::from_context(
            &context,
            self.max_tokens,
            runtime_metadata.clone(),
            runtime_architecture,
        );

        match self.runtime.generate(request) {
            Ok(response) => {
                self.last_error = None;
                let trace = if response.trace.is_empty() {
                    trace_from_tokens(&response.tokens)
                } else {
                    response.trace
                };
                let tokens = response
                    .tokens
                    .into_iter()
                    .map(|token| DraftToken {
                        text: token.text,
                        logprob: token.logprob,
                        entropy: token.entropy,
                    })
                    .collect();
                let mut trace = trace;
                if imported_kv_blocks > 0 {
                    trace.push(ReasoningStep::new(
                        "runtime_kv_import",
                        format!("imported {imported_kv_blocks} KV blocks"),
                        0.78,
                    ));
                }
                let exported_kv_blocks = if runtime_metadata.supports_kv_export {
                    match self.runtime.export_kv() {
                        Ok(blocks) if !blocks.is_empty() => {
                            trace.push(ReasoningStep::new(
                                "runtime_kv_export",
                                format!("exported {} KV blocks", blocks.len()),
                                0.74,
                            ));
                            blocks
                        }
                        Ok(_) => Vec::new(),
                        Err(error) => {
                            trace.push(ReasoningStep::new(
                                "runtime_kv_export_error",
                                error.message(),
                                0.22,
                            ));
                            Vec::new()
                        }
                    }
                } else {
                    Vec::new()
                };
                let mut diagnostics = response.diagnostics;
                diagnostics.imported_kv_blocks = imported_kv_blocks;
                diagnostics.exported_kv_blocks = exported_kv_blocks.len();
                InferenceDraft::new(response.answer, trace)
                    .with_tokens(tokens)
                    .with_exported_kv_blocks(exported_kv_blocks)
                    .with_runtime_diagnostics(diagnostics)
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

fn runtime_kv_blocks_from_context(
    context: &GenerationContext<'_>,
    metadata: &RuntimeMetadata,
) -> Vec<RuntimeKvBlock> {
    if !metadata.supports_kv_import {
        return Vec::new();
    }

    let dimensions = if metadata.embedding_dimensions > 0 {
        Some(metadata.embedding_dimensions)
    } else {
        None
    };
    let manifest_limit = if metadata.max_kv_import_blocks > 0 {
        metadata.max_kv_import_blocks
    } else {
        context.hardware_plan.execution.kv_prefetch_blocks
    };
    let prefetch_limit = context
        .hardware_plan
        .execution
        .kv_prefetch_blocks
        .min(manifest_limit)
        .max(1);

    context
        .memories
        .iter()
        .filter(|memory| !memory.vector.is_empty())
        .filter(|memory| {
            context
                .tier_plan
                .placement_for(memory.id)
                .map(|placement| placement.tier != MemoryTier::ColdDisk)
                .unwrap_or(true)
        })
        .take(prefetch_limit)
        .enumerate()
        .map(|(index, memory)| {
            let key = fit_runtime_vector(&memory.vector, dimensions);
            let weighted = memory
                .vector
                .iter()
                .map(|value| value * memory.strength)
                .collect::<Vec<_>>();
            let value = fit_runtime_vector(&weighted, dimensions);

            RuntimeKvBlock::new(0, index, index, index + 1, key, value)
        })
        .collect()
}

fn fit_runtime_vector(vector: &[f32], dimensions: Option<usize>) -> Vec<f32> {
    let Some(dimensions) = dimensions else {
        return vector.to_vec();
    };
    let mut out = vector.iter().copied().take(dimensions).collect::<Vec<_>>();
    out.resize(dimensions, 0.0);
    out
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
    use crate::infini_memory::{InfiniMemoryItem, InfiniMemoryPlan, InfiniMemoryScope};
    use crate::kv_cache::MemoryMatch;
    use crate::tiered_cache::TieredCachePlan;
    use crate::transformer::TransformerRefactorPlan;

    #[derive(Debug, Default, Clone)]
    struct MockRuntime {
        seen: Option<RuntimeRequest>,
    }

    impl ModelRuntime for MockRuntime {
        fn metadata(&self) -> RuntimeMetadata {
            RuntimeMetadata::new("mock-self-transformer", "mock-bpe", 32_768, 128)
                .with_kv_exchange(true, true)
        }

        fn architecture(&self) -> TransformerRuntimeArchitecture {
            TransformerRuntimeArchitecture::new(18, 128, 8, 4, 4096)
        }

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
            vector: vec![0.1, 0.2, 0.3],
        }];
        let experiences = vec![ExperienceMatch {
            id: 1,
            prompt: "prompt".to_owned(),
            lesson: "lesson".to_owned(),
            quality: 0.9,
            score: 0.88,
            gist_hints: vec!["document:gist importance=0.900".to_owned()],
            reflection_issue_codes: Vec::new(),
            revision_actions: Vec::new(),
            process_reward: 0.81,
            reward_action: crate::process_reward::RewardAction::Reinforce,
            runtime_model_id: Some("mock-self-transformer".to_owned()),
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_forward_energy: Some(0.2),
            runtime_kv_influence: Some(0.1),
            recursive_runtime_calls: None,
        }];
        let tier_plan = TieredCachePlan::default();
        let infini_memory_plan = InfiniMemoryPlan::new(
            vec![InfiniMemoryItem {
                id: 1,
                key: "local kv memory".to_owned(),
                scope: InfiniMemoryScope::LocalWindow,
                score: 0.91,
                estimated_tokens: 3,
                reason: "test local".to_owned(),
            }],
            Vec::new(),
            Vec::new(),
        );
        let transformer_plan = TransformerRefactorPlan::default();
        let recursive_schedule = RecursiveSchedule::default();
        let hardware_plan = HardwarePlan::default();
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
            infini_memory_plan: &infini_memory_plan,
            recursive_schedule: &recursive_schedule,
            hardware_plan: &hardware_plan,
            experiences: &experiences,
            transformer_plan: &transformer_plan,
        };
        let mut backend = RuntimeBackend::new(MockRuntime::default()).with_max_tokens(128);

        let draft = backend.generate(context);
        let seen = backend.runtime().seen.as_ref().unwrap();

        assert!(draft.answer.contains("1 memories and 1 experiences"));
        assert_eq!(seen.max_tokens, 128);
        assert_eq!(seen.runtime_metadata.model_id, "mock-self-transformer");
        assert_eq!(seen.runtime_metadata.native_context_window, 32_768);
        assert_eq!(seen.runtime_architecture.layer_count, 18);
        assert_eq!(seen.runtime_architecture.hidden_size, 128);
        assert_eq!(seen.runtime_architecture.attention_heads, 8);
        assert_eq!(seen.runtime_architecture.kv_heads, 4);
        assert_eq!(seen.runtime_architecture.local_window_tokens, 4096);
        assert!(seen.runtime_metadata.supports_kv_import);
        assert!(seen.runtime_metadata.supports_kv_export);
        assert_eq!(seen.memory_hints.len(), 1);
        assert_eq!(seen.infini_memory_hints.len(), 1);
        assert_eq!(seen.experience_hints.len(), 1);
        assert_eq!(seen.runtime_adapter_observations.len(), 1);
        assert_eq!(
            seen.runtime_adapter_observations[0].adapter,
            RuntimeAdapterHint::PortableRust
        );
        assert!(seen.runtime_adapter_observations[0].score > 0.70);
        assert!(!seen.recursive_schedule.requires_recursion);
        assert!(seen.hardware_plan.local_kv_token_budget > 0);
        assert!(seen.transformer_plan.is_empty());
    }

    #[derive(Debug, Default, Clone)]
    struct SelfDevelopedRuntime {
        imported_blocks: usize,
    }

    impl ModelRuntime for SelfDevelopedRuntime {
        fn metadata(&self) -> RuntimeMetadata {
            RuntimeMetadata::new("noiron-dev-transformer", "noiron-wordpiece", 65_536, 256)
                .with_kv_exchange(true, true)
        }

        fn architecture(&self) -> TransformerRuntimeArchitecture {
            TransformerRuntimeArchitecture::new(24, 256, 8, 4, 8192)
        }

        fn tokenize(&self, prompt: &str) -> Result<Vec<RuntimeTokenId>, RuntimeError> {
            Ok(prompt
                .split_whitespace()
                .enumerate()
                .map(|(index, text)| RuntimeTokenId::new(10_000 + index as u32, text))
                .collect())
        }

        fn embed(&self, tokens: &[RuntimeTokenId]) -> Result<RuntimeEmbedding, RuntimeError> {
            Ok(RuntimeEmbedding::new(vec![tokens.len() as f32, 1.0, 0.5]))
        }

        fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
            self.imported_blocks += blocks.len();
            Ok(blocks.len())
        }

        fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
            Ok(vec![RuntimeKvBlock::new(
                1,
                2,
                0,
                4,
                vec![0.1, 0.2],
                vec![0.3, 0.4],
            )])
        }

        fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
            Ok(RuntimeResponse::new(format!(
                "generated with {}",
                request.runtime_metadata.model_id
            )))
        }
    }

    #[derive(Debug, Default, Clone)]
    struct ManifestBoundRuntime {
        imported_blocks: usize,
    }

    impl ModelRuntime for ManifestBoundRuntime {
        fn metadata(&self) -> RuntimeMetadata {
            RuntimeMetadata::new(
                "manifest-bound-transformer",
                "noiron-wordpiece",
                65_536,
                256,
            )
            .with_kv_exchange(true, true)
            .with_kv_limits(1, 2)
            .with_kv_precision(4, 4)
        }

        fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
            self.imported_blocks += blocks.len();
            Ok(blocks.len())
        }

        fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
            Ok(RuntimeResponse::new(format!(
                "generated with max import {}",
                request.runtime_metadata.max_kv_import_blocks
            )))
        }
    }

    #[test]
    fn self_developed_runtime_abi_exposes_tokens_embeddings_and_kv_exchange() {
        let mut runtime = SelfDevelopedRuntime::default();

        let metadata = runtime.metadata();
        let architecture = runtime.architecture();
        let tokens = runtime.tokenize("alpha beta").unwrap();
        let embedding = runtime.embed(&tokens).unwrap();
        let text_embedding = runtime.embed_text("alpha beta gamma").unwrap();
        let imported = runtime
            .import_kv(&[RuntimeKvBlock::new(
                0,
                1,
                0,
                2,
                vec![0.1, 0.2],
                vec![0.3, 0.4],
            )])
            .unwrap();
        let exported = runtime.export_kv().unwrap();

        assert_eq!(metadata.model_id, "noiron-dev-transformer");
        assert_eq!(metadata.tokenizer, "noiron-wordpiece");
        assert_eq!(architecture.layer_count, 24);
        assert_eq!(architecture.hidden_size, 256);
        assert_eq!(architecture.attention_heads, 8);
        assert_eq!(architecture.kv_heads, 4);
        assert_eq!(architecture.local_window_tokens, 8192);
        assert_eq!(tokens[0], RuntimeTokenId::new(10_000, "alpha"));
        assert_eq!(embedding.dimensions, 3);
        assert_eq!(text_embedding.values, vec![3.0, 1.0, 0.5]);
        assert_eq!(imported, 1);
        assert_eq!(runtime.imported_blocks, 1);
        assert_eq!(exported[0].layer, 1);
        assert_eq!(exported[0].head, 2);
    }

    #[test]
    fn runtime_backend_exposes_model_side_embeddings() {
        let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

        let embedding = backend.embed_text("alpha beta").unwrap();

        assert_eq!(embedding, vec![2.0, 1.0, 0.5]);
    }

    #[test]
    fn runtime_backend_imports_memory_kv_and_returns_exported_blocks() {
        let memories = vec![MemoryMatch {
            id: 7,
            key: "hot runtime memory".to_owned(),
            similarity: 0.91,
            strength: 1.25,
            vector: vec![0.1, 0.2, 0.3],
        }];
        let tier_plan = TieredCachePlan::default();
        let infini_memory_plan = InfiniMemoryPlan::default();
        let transformer_plan = TransformerRefactorPlan::default();
        let recursive_schedule = RecursiveSchedule::default();
        let hardware_plan = HardwarePlan::default();
        let context = GenerationContext {
            prompt: "use runtime kv",
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
            infini_memory_plan: &infini_memory_plan,
            recursive_schedule: &recursive_schedule,
            hardware_plan: &hardware_plan,
            experiences: &[],
            transformer_plan: &transformer_plan,
        };
        let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

        let draft = backend.generate(context);

        assert_eq!(backend.runtime().imported_blocks, 1);
        assert_eq!(draft.exported_kv_blocks.len(), 1);
        assert!(
            draft
                .trace
                .iter()
                .any(|step| step.label == "runtime_kv_import")
        );
        assert!(
            draft
                .trace
                .iter()
                .any(|step| step.label == "runtime_kv_export")
        );
    }

    #[test]
    fn runtime_kv_import_respects_device_prefetch_budget() {
        let memories = vec![
            MemoryMatch {
                id: 7,
                key: "hot runtime memory one".to_owned(),
                similarity: 0.95,
                strength: 1.25,
                vector: vec![0.1, 0.2, 0.3],
            },
            MemoryMatch {
                id: 8,
                key: "hot runtime memory two".to_owned(),
                similarity: 0.90,
                strength: 1.10,
                vector: vec![0.4, 0.5, 0.6],
            },
        ];
        let tier_plan = TieredCachePlan::default();
        let infini_memory_plan = InfiniMemoryPlan::default();
        let transformer_plan = TransformerRefactorPlan::default();
        let recursive_schedule = RecursiveSchedule::default();
        let mut hardware_plan = HardwarePlan::default();
        hardware_plan.execution.kv_prefetch_blocks = 1;
        let context = GenerationContext {
            prompt: "limit runtime kv",
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
            infini_memory_plan: &infini_memory_plan,
            recursive_schedule: &recursive_schedule,
            hardware_plan: &hardware_plan,
            experiences: &[],
            transformer_plan: &transformer_plan,
        };
        let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

        let draft = backend.generate(context);

        assert_eq!(backend.runtime().imported_blocks, 1);
        assert_eq!(draft.exported_kv_blocks.len(), 1);
    }

    #[test]
    fn runtime_kv_import_respects_manifest_import_limit() {
        let memories = vec![
            MemoryMatch {
                id: 7,
                key: "hot runtime memory one".to_owned(),
                similarity: 0.95,
                strength: 1.25,
                vector: vec![0.1, 0.2, 0.3],
            },
            MemoryMatch {
                id: 8,
                key: "hot runtime memory two".to_owned(),
                similarity: 0.90,
                strength: 1.10,
                vector: vec![0.4, 0.5, 0.6],
            },
        ];
        let tier_plan = TieredCachePlan::default();
        let infini_memory_plan = InfiniMemoryPlan::default();
        let transformer_plan = TransformerRefactorPlan::default();
        let recursive_schedule = RecursiveSchedule::default();
        let mut hardware_plan = HardwarePlan::default();
        hardware_plan.execution.kv_prefetch_blocks = 4;
        let context = GenerationContext {
            prompt: "limit runtime kv with manifest",
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
            infini_memory_plan: &infini_memory_plan,
            recursive_schedule: &recursive_schedule,
            hardware_plan: &hardware_plan,
            experiences: &[],
            transformer_plan: &transformer_plan,
        };
        let mut backend = RuntimeBackend::new(ManifestBoundRuntime::default());

        let draft = backend.generate(context);

        assert_eq!(backend.runtime().imported_blocks, 1);
        assert!(draft.answer.contains("max import 1"));
    }

    #[test]
    fn default_runtime_abi_keeps_command_runtime_compatible() {
        let runtime = CommandRuntime::new("runner");

        let metadata = runtime.metadata();
        let tokens = runtime.tokenize("fallback tokenize").unwrap();
        let embedding = runtime.embed(&tokens).unwrap();

        assert_eq!(metadata, RuntimeMetadata::default());
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "fallback");
        assert_eq!(embedding.dimensions, 0);
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
        let infini_memory_plan = InfiniMemoryPlan::default();
        let transformer_plan = TransformerRefactorPlan::default();
        let recursive_schedule = RecursiveSchedule::default();
        let hardware_plan = HardwarePlan::default();
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
            infini_memory_plan: &infini_memory_plan,
            recursive_schedule: &recursive_schedule,
            hardware_plan: &hardware_plan,
            experiences: &[],
            transformer_plan: &transformer_plan,
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
        let metadata = RuntimeMetadata::new("command-model", "command-tokenizer", 16_384, 384)
            .with_kv_exchange(true, false);
        let runtime = CommandRuntime::new("runner")
            .with_metadata(metadata)
            .arg("--prompt")
            .arg("{prompt}")
            .arg("--max")
            .arg("{max_tokens}")
            .arg("--runtime")
            .arg("{runtime_metadata}")
            .arg("--architecture")
            .arg("{runtime_architecture}")
            .prompt_mode(CommandPromptMode::Args);
        let request = sample_request();
        let prompt = format_runtime_prompt(&request);
        let args = runtime.expanded_args(&request, &prompt);

        assert!(prompt.contains("runtime:"));
        assert!(prompt.contains("runtime_architecture:"));
        assert!(prompt.contains("model_id=sample-transformer"));
        assert!(prompt.contains("native_context_window=8192"));
        assert!(prompt.contains("layers=16"));
        assert!(prompt.contains("attention_heads=8"));
        assert!(prompt.contains("local_window=2048"));
        assert!(prompt.contains("max_kv_import_blocks=8"));
        assert!(prompt.contains("kv_bits=8/4"));
        assert!(prompt.contains("memory_hints"));
        assert!(prompt.contains("infini_memory_hints"));
        assert!(prompt.contains("experience_hints"));
        assert!(prompt.contains("recursive:"));
        assert!(prompt.contains("hardware:"));
        assert!(prompt.contains("transformer: template=none"));
        assert!(args[1].contains("Noiron runtime request"));
        assert_eq!(args[3], "64");
        assert!(args[5].contains("model_id=sample-transformer"));
        assert!(args[7].contains("layers=16"));
    }

    #[test]
    fn runtime_request_json_includes_control_plane_sections() {
        let request = sample_request();

        let payload = runtime_request_json(&request);

        assert_eq!(
            extract_json_string_field(&payload, "schema").unwrap(),
            "rust-norion-runtime-request-v1"
        );
        assert_eq!(
            extract_json_string_field(&payload, "profile").unwrap(),
            "coding"
        );
        assert_eq!(
            extract_json_string_field(&payload, "model_id").unwrap(),
            "sample-transformer"
        );
        assert_eq!(
            extract_json_number_field(&payload, "max_kv_import_blocks").unwrap(),
            8.0
        );
        assert_eq!(
            extract_json_number_field(&payload, "max_kv_export_blocks").unwrap(),
            4.0
        );
        assert_eq!(
            extract_json_number_field(&payload, "hot_kv_precision_bits").unwrap(),
            8.0
        );
        assert_eq!(
            extract_json_number_field(&payload, "cold_kv_precision_bits").unwrap(),
            4.0
        );
        assert_eq!(
            extract_json_number_field(&payload, "layer_count").unwrap(),
            16.0
        );
        assert_eq!(
            extract_json_number_field(&payload, "hidden_size").unwrap(),
            64.0
        );
        assert_eq!(
            extract_json_number_field(&payload, "attention_heads").unwrap(),
            8.0
        );
        assert_eq!(
            extract_json_number_field(&payload, "kv_heads").unwrap(),
            4.0
        );
        assert_eq!(
            extract_json_number_field(&payload, "local_window_tokens").unwrap(),
            2048.0
        );
        assert_eq!(
            extract_json_number_field(&payload, "attention_tokens").unwrap(),
            2.0
        );
        assert_eq!(
            extract_json_string_field(&payload, "primary_lane").unwrap(),
            "cpu-vector"
        );
        assert!(payload.contains("\"execution_waves\""));
        assert!(payload.contains("\"max_parallel_chunks\""));
        assert!(payload.contains("\"template\":\"none\""));
        assert!(payload.contains("\"memory_hints\":[\"memory hint\"]"));
        assert_eq!(extract_json_array_field(&payload, "layers").unwrap(), "[]");
        assert!(payload.contains("\"runtime_adapter_observations\":["));
        assert!(payload.contains("\"adapter\":\"cpu-simd\""));
        assert!(payload.contains("\"experience_id\":9"));
    }

    #[test]
    fn runtime_response_json_parses_tokens_and_trace() {
        let payload = r#"{
            "schema": "rust-norion-runtime-response-v1",
            "answer": "structured runtime answer",
            "tokens": [
                {"text": "structured", "logprob": -0.2, "entropy": 0.3},
                {"text": "answer", "entropy": 0.4}
            ],
            "trace": [
                {"label": "runtime", "content": "generated with JSON ABI", "confidence": 0.91}
            ],
            "diagnostics": {
                "model_id": "json-self-runtime",
                "selected_adapter": "portable-rust",
                "layer_count": 24,
                "hidden_size": 256,
                "local_window_tokens": 4096,
                "forward_energy": 0.42,
                "kv_influence": 0.18,
                "imported_kv_blocks": 2,
                "exported_kv_blocks": 3
            }
        }"#;

        let response = parse_runtime_response_json(payload).unwrap();

        assert_eq!(response.answer, "structured runtime answer");
        assert_eq!(response.tokens.len(), 2);
        assert_eq!(response.tokens[0].logprob, Some(-0.2));
        assert_eq!(response.tokens[1].entropy, Some(0.4));
        assert_eq!(response.trace[0].label, "runtime");
        assert!((response.trace[0].confidence - 0.91).abs() < 0.0001);
        assert_eq!(
            response.diagnostics.model_id.as_deref(),
            Some("json-self-runtime")
        );
        assert_eq!(
            response.diagnostics.selected_adapter.as_deref(),
            Some("portable-rust")
        );
        assert_eq!(response.diagnostics.layer_count, 24);
        assert_eq!(response.diagnostics.hidden_size, 256);
        assert_eq!(response.diagnostics.local_window_tokens, 4096);
        assert_eq!(response.diagnostics.forward_energy, Some(0.42));
        assert_eq!(response.diagnostics.kv_influence, Some(0.18));
        assert_eq!(response.diagnostics.imported_kv_blocks, 2);
        assert_eq!(response.diagnostics.exported_kv_blocks, 3);
    }

    #[test]
    fn command_runtime_can_expand_json_wire_payload() {
        let runtime = CommandRuntime::new("runner")
            .wire_format(CommandWireFormat::Json)
            .arg("--wire")
            .arg("{wire_format}")
            .arg("--payload")
            .arg("{runtime_payload}")
            .prompt_mode(CommandPromptMode::Args);
        let request = sample_request();
        let payload = format_runtime_payload(&request, CommandWireFormat::Json);
        let args = runtime.expanded_args(&request, &payload);

        assert_eq!(args[1], "json");
        assert!(args[3].contains("\"schema\":\"rust-norion-runtime-request-v1\""));
        assert!(args[3].contains("\"hardware\""));
        assert!(args[3].contains("\"runtime_architecture\""));
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
            runtime_metadata: RuntimeMetadata::new(
                "sample-transformer",
                "sample-tokenizer",
                8192,
                64,
            )
            .with_kv_exchange(true, true),
            runtime_architecture: TransformerRuntimeArchitecture::new(16, 64, 8, 4, 2048),
            memory_hints: vec!["memory hint".to_owned()],
            infini_memory_hints: vec!["LocalWindow:memory hint score=0.900".to_owned()],
            experience_hints: vec!["experience hint".to_owned()],
            runtime_adapter_observations: vec![RuntimeAdapterObservation::new(
                RuntimeAdapterHint::CpuSimd,
                0.82,
                0.80,
                0.86,
                Some(0.20),
                Some(0.30),
                9,
            )],
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 2,
                fast_tokens: 1,
                attention_fraction: 0.66,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            transformer_plan: TransformerRefactorPlan::default(),
            recursive_schedule: RecursiveSchedule::default(),
            hardware_plan: HardwarePlan::default(),
            max_tokens: 64,
        }
    }
}
