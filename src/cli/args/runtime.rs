use std::path::PathBuf;

use rust_norion::{
    CommandPromptMode, CommandRuntime, CommandWireFormat, DeviceClass, GemmaRuntimeConfig,
    GemmaRuntimeQuantizationMode, HardwareAllocator, HardwarePlan, HardwareSnapshot,
    HierarchyWeights, LocalTransformerRuntime, MistralRsHttpRuntime, ModelRuntimeForwardKernel,
    RecursiveScheduler, ReferenceProductionForwardKernel, RuntimeAssetPaths, RuntimeError,
    RuntimeManifest, RuntimeMetadata, TaskProfile, TransformerRuntimeArchitecture,
};

use super::Args;
use super::values::{parse_u64, parse_usize};

pub(crate) struct RuntimeFlagParse<'a> {
    pub(crate) local_runtime: &'a mut bool,
    pub(crate) production_runtime: &'a mut bool,
    pub(crate) production_reference_kernel: &'a mut bool,
    pub(crate) production_local_kernel: &'a mut bool,
    pub(crate) production_kernel_conformance_gate: &'a mut bool,
    pub(crate) production_kernel_conformance_all_devices_gate: &'a mut bool,
    pub(crate) runtime_manifest_gate: &'a mut bool,
    pub(crate) runtime_manifest_all_devices_gate: &'a mut bool,
    pub(crate) runtime_weights_path: &'a mut Option<PathBuf>,
    pub(crate) runtime_tokenizer_path: &'a mut Option<PathBuf>,
    pub(crate) runtime_config_path: &'a mut Option<PathBuf>,
    pub(crate) runtime_layer_count: &'a mut Option<usize>,
    pub(crate) runtime_hidden_size: &'a mut Option<usize>,
    pub(crate) runtime_attention_heads: &'a mut Option<usize>,
    pub(crate) runtime_kv_heads: &'a mut Option<usize>,
    pub(crate) runtime_local_window_tokens: &'a mut Option<usize>,
    pub(crate) runtime_command: &'a mut Option<PathBuf>,
    pub(crate) runtime_args: &'a mut Vec<String>,
    pub(crate) runtime_timeout_ms: &'a mut Option<u64>,
    pub(crate) runtime_stream_idle_timeout_ms: &'a mut Option<u64>,
    pub(crate) runtime_prompt_mode: &'a mut CommandPromptMode,
    pub(crate) runtime_wire_format: &'a mut CommandWireFormat,
    pub(crate) runtime_metadata: &'a mut RuntimeMetadata,
}

impl RuntimeFlagParse<'_> {
    pub(crate) fn parse(&mut self, raw: &[String], index: usize) -> Option<usize> {
        let flag = raw.get(index)?.as_str();
        match flag {
            "--local-runtime" => {
                *self.local_runtime = true;
                self.enable_runtime_kv_exchange();
                Some(1)
            }
            "--production-runtime" => {
                *self.production_runtime = true;
                self.enable_runtime_kv_exchange();
                Some(1)
            }
            "--production-reference-kernel" => {
                *self.production_runtime = true;
                *self.production_reference_kernel = true;
                self.enable_runtime_kv_exchange();
                Some(1)
            }
            "--production-local-kernel" => {
                *self.production_runtime = true;
                *self.production_local_kernel = true;
                self.enable_runtime_kv_exchange();
                Some(1)
            }
            "--production-kernel-conformance-gate" => {
                *self.production_runtime = true;
                *self.production_kernel_conformance_gate = true;
                self.enable_runtime_kv_exchange();
                Some(1)
            }
            "--production-kernel-conformance-all-devices-gate" => {
                *self.production_runtime = true;
                *self.production_kernel_conformance_gate = true;
                *self.production_kernel_conformance_all_devices_gate = true;
                self.enable_runtime_kv_exchange();
                Some(1)
            }
            "--runtime-manifest-gate" => {
                *self.runtime_manifest_gate = true;
                Some(1)
            }
            "--runtime-manifest-all-devices-gate" => {
                *self.runtime_manifest_gate = true;
                *self.runtime_manifest_all_devices_gate = true;
                Some(1)
            }
            "--runtime-weights" => {
                let path = raw.get(index + 1)?;
                *self.runtime_weights_path = Some(PathBuf::from(path));
                Some(2)
            }
            "--runtime-tokenizer-path" => {
                let path = raw.get(index + 1)?;
                *self.runtime_tokenizer_path = Some(PathBuf::from(path));
                Some(2)
            }
            "--runtime-config" => {
                let path = raw.get(index + 1)?;
                *self.runtime_config_path = Some(PathBuf::from(path));
                Some(2)
            }
            "--runtime-layers" => {
                let layers = raw.get(index + 1)?;
                *self.runtime_layer_count = Some(parse_usize(layers, 0));
                Some(2)
            }
            "--runtime-hidden-size" => {
                let hidden_size = raw.get(index + 1)?;
                *self.runtime_hidden_size = Some(parse_usize(hidden_size, 0));
                Some(2)
            }
            "--runtime-attention-heads" => {
                let attention_heads = raw.get(index + 1)?;
                *self.runtime_attention_heads = Some(parse_usize(attention_heads, 0));
                Some(2)
            }
            "--runtime-kv-heads" => {
                let kv_heads = raw.get(index + 1)?;
                *self.runtime_kv_heads = Some(parse_usize(kv_heads, 0));
                Some(2)
            }
            "--runtime-local-window" => {
                let local_window = raw.get(index + 1)?;
                *self.runtime_local_window_tokens = Some(parse_usize(local_window, 0));
                Some(2)
            }
            "--runtime-command" => {
                let path = raw.get(index + 1)?;
                *self.runtime_command = Some(PathBuf::from(path));
                Some(2)
            }
            "--runtime-arg" => {
                let arg = raw.get(index + 1)?;
                self.runtime_args.push(arg.clone());
                Some(2)
            }
            "--runtime-timeout-ms" => {
                let timeout = raw.get(index + 1)?;
                *self.runtime_timeout_ms = Some(parse_u64(timeout, 1).max(1));
                Some(2)
            }
            "--runtime-stream-idle-timeout-ms" => {
                let timeout = raw.get(index + 1)?;
                *self.runtime_stream_idle_timeout_ms = Some(parse_u64(timeout, 1).max(1));
                Some(2)
            }
            "--runtime-prompt-mode" => {
                let mode = raw.get(index + 1)?;
                *self.runtime_prompt_mode = match mode.as_str() {
                    "args" => CommandPromptMode::Args,
                    _ => CommandPromptMode::Stdin,
                };
                Some(2)
            }
            "--runtime-wire-format" => {
                let wire_format = raw.get(index + 1)?;
                *self.runtime_wire_format = match wire_format.as_str() {
                    "json" => CommandWireFormat::Json,
                    _ => CommandWireFormat::Text,
                };
                Some(2)
            }
            "--runtime-json" => {
                *self.runtime_wire_format = CommandWireFormat::Json;
                Some(1)
            }
            "--runtime-model-id" => {
                let model_id = raw.get(index + 1)?;
                self.runtime_metadata.model_id = model_id.clone();
                Some(2)
            }
            "--runtime-tokenizer" => {
                let tokenizer = raw.get(index + 1)?;
                self.runtime_metadata.tokenizer = tokenizer.clone();
                Some(2)
            }
            "--runtime-native-window" => {
                let native_window = raw.get(index + 1)?;
                self.runtime_metadata.native_context_window =
                    parse_usize(native_window, self.runtime_metadata.native_context_window);
                Some(2)
            }
            "--runtime-embedding-dims" => {
                let dimensions = raw.get(index + 1)?;
                self.runtime_metadata.embedding_dimensions =
                    parse_usize(dimensions, self.runtime_metadata.embedding_dimensions);
                Some(2)
            }
            "--runtime-kv-import" => {
                self.runtime_metadata.supports_kv_import = true;
                Some(1)
            }
            "--runtime-kv-export" => {
                self.runtime_metadata.supports_kv_export = true;
                Some(1)
            }
            "--runtime-kv-exchange" => {
                self.enable_runtime_kv_exchange();
                Some(1)
            }
            _ => None,
        }
    }

    fn enable_runtime_kv_exchange(&mut self) {
        self.runtime_metadata.supports_kv_import = true;
        self.runtime_metadata.supports_kv_export = true;
    }
}

impl Args {
    pub(crate) fn runtime_manifest(&self) -> RuntimeManifest {
        let mut assets = RuntimeAssetPaths::new();
        if let Some(path) = &self.runtime_weights_path {
            assets = assets.with_weights(path.clone());
        }
        if let Some(path) = &self.runtime_tokenizer_path {
            assets = assets.with_tokenizer(path.clone());
        }
        if let Some(path) = &self.runtime_config_path {
            assets = assets.with_config(path.clone());
        }

        let manifest = RuntimeManifest::from_metadata(self.runtime_metadata.clone());
        let defaults = manifest.architecture;
        let architecture = TransformerRuntimeArchitecture::new(
            self.runtime_layer_count.unwrap_or(defaults.layer_count),
            self.runtime_hidden_size.unwrap_or(defaults.hidden_size),
            self.runtime_attention_heads
                .unwrap_or(defaults.attention_heads),
            self.runtime_kv_heads.unwrap_or(defaults.kv_heads),
            self.runtime_local_window_tokens
                .unwrap_or(defaults.local_window_tokens),
        );

        manifest.with_assets(assets).with_architecture(architecture)
    }

    pub(crate) fn runtime_manifest_device_plan(&self) -> HardwarePlan {
        self.runtime_manifest_device_plan_for(self.device, self.profile, &self.prompt)
    }

    pub(crate) fn runtime_manifest_device_plan_for(
        &self,
        device: DeviceClass,
        profile: TaskProfile,
        prompt: &str,
    ) -> HardwarePlan {
        let snapshot = HardwareSnapshot::new(
            device,
            self.cpu_load,
            self.gpu_load,
            self.ram_load,
            self.disk_load,
        );
        let prompt_tokens = RecursiveScheduler::new(
            self.native_window_tokens,
            self.chunk_tokens,
            self.chunk_overlap_tokens,
            self.merge_fan_in,
        )
        .plan(prompt)
        .prompt_tokens;

        HardwareAllocator::new().plan(
            snapshot,
            profile,
            prompt_tokens,
            HierarchyWeights::default(),
        )
    }

    pub(crate) fn production_runtime(
        &self,
    ) -> std::io::Result<rust_norion::ProductionTransformerRuntime> {
        self.production_runtime_for_case(self.device, self.profile, &self.prompt)
    }

    pub(crate) fn production_runtime_for_case(
        &self,
        device: DeviceClass,
        profile: TaskProfile,
        prompt: &str,
    ) -> std::io::Result<rust_norion::ProductionTransformerRuntime> {
        let runtime = rust_norion::ProductionTransformerRuntime::from_manifest_for_plan(
            self.runtime_manifest(),
            &self.runtime_manifest_device_plan_for(device, profile, prompt),
        )
        .map_err(runtime_error_to_io)?;

        if self.production_local_kernel {
            let local = LocalTransformerRuntime::with_manifest(self.runtime_manifest());
            Ok(runtime.with_kernel(ModelRuntimeForwardKernel::new(local)))
        } else if self.production_reference_kernel {
            Ok(runtime.with_kernel(ReferenceProductionForwardKernel::new()))
        } else if let Some(command_runtime) = self.command_runtime() {
            Ok(runtime.with_kernel(ModelRuntimeForwardKernel::new(command_runtime)))
        } else {
            Ok(runtime)
        }
    }

    pub(crate) fn command_runtime(&self) -> Option<CommandRuntime> {
        if self.gemma_12b_runtime {
            if self.gemma_runtime_server.is_some() {
                return None;
            }
            return Some(
                self.apply_command_runtime_options(self.gemma_runtime_config().command_runtime()),
            );
        }

        let manifest = self.runtime_manifest();
        self.runtime_command.clone().map(|runtime_command| {
            let activation_payload = self.command_runtime_activation_payload(&runtime_command);
            self.apply_command_runtime_options(CommandRuntime::new(runtime_command))
                .args(self.runtime_args.clone())
                .prompt_mode(self.runtime_prompt_mode)
                .wire_format(self.runtime_wire_format)
                .with_metadata(manifest.runtime_metadata())
                .with_architecture(manifest.architecture)
                .with_development_pollution_activation_gate(
                    "runtime_command",
                    "process_start",
                    activation_payload,
                )
        })
    }

    fn command_runtime_activation_payload(&self, program: &PathBuf) -> String {
        let mut parts = vec![
            format!("program={}", program.display()),
            format!("model_id={}", self.runtime_metadata.model_id),
            format!("tokenizer={}", self.runtime_metadata.tokenizer),
        ];
        if !self.runtime_args.is_empty() {
            parts.push(format!("args={}", self.runtime_args.join(" ")));
        }
        if let Some(path) = &self.runtime_weights_path {
            parts.push(format!("weights={}", path.display()));
        }
        if let Some(path) = &self.runtime_tokenizer_path {
            parts.push(format!("tokenizer_path={}", path.display()));
        }
        if let Some(path) = &self.runtime_config_path {
            parts.push(format!("config={}", path.display()));
        }
        parts.join(" ")
    }

    pub(crate) fn apply_command_runtime_options(&self, runtime: CommandRuntime) -> CommandRuntime {
        if let Some(timeout_ms) = self.runtime_timeout_ms {
            runtime.with_timeout_ms(timeout_ms)
        } else {
            runtime
        }
    }

    pub(crate) fn gemma_http_runtime(&self) -> std::io::Result<Option<MistralRsHttpRuntime>> {
        let Some(server) = &self.gemma_runtime_server else {
            return Ok(None);
        };
        let runtime = self
            .gemma_runtime_config()
            .http_runtime(server)
            .map_err(runtime_error_to_io)?;
        let runtime = if let Some(timeout_ms) = self.runtime_timeout_ms {
            runtime.with_timeout_ms(timeout_ms)
        } else {
            runtime
        };
        let runtime = if let Some(timeout_ms) = self.runtime_stream_idle_timeout_ms {
            runtime.with_stream_idle_timeout_ms(timeout_ms)
        } else {
            runtime
        };
        Ok(Some(runtime))
    }

    pub(crate) fn gemma_runtime_config(&self) -> GemmaRuntimeConfig {
        let config = match self.gemma_runtime_quantization_mode {
            GemmaRuntimeQuantizationMode::Quant => {
                GemmaRuntimeConfig::new().with_quantization(self.gemma_runtime_quantization.clone())
            }
            GemmaRuntimeQuantizationMode::Isq => {
                GemmaRuntimeConfig::new().with_isq(self.gemma_runtime_quantization.clone())
            }
        };
        let config = config
            .with_program(self.gemma_runtime_program.clone())
            .with_model_id(self.runtime_metadata.model_id.clone())
            .with_runtime_window_tokens(self.runtime_metadata.native_context_window.max(1))
            .with_passthrough_args(self.runtime_args.clone());
        let config = if let Some(token_source) = &self.gemma_runtime_token_source {
            config.with_token_source(token_source.clone())
        } else {
            config.without_token_source()
        };
        let config = if let Some(paged_attn) = &self.gemma_runtime_paged_attn {
            config.with_paged_attn(paged_attn.clone())
        } else {
            config
        };
        let config = if let Some(thinking) = &self.gemma_runtime_thinking {
            config.with_thinking(thinking.clone())
        } else {
            config.without_thinking()
        };
        if let Some(hf_cache) = &self.gemma_runtime_hf_cache {
            config.with_hf_cache(hf_cache.clone())
        } else {
            config
        }
    }
}

fn runtime_error_to_io(error: RuntimeError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, error.message().to_owned())
}
