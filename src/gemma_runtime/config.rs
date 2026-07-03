use std::path::PathBuf;

use crate::runtime::{
    CommandPromptMode, CommandRuntime, CommandTextOutputFilter, CommandWireFormat,
    MistralRsHttpRuntime, RuntimeError, RuntimeMetadata,
};
use crate::runtime_manifest::{RuntimeManifest, TransformerRuntimeArchitecture};

use super::fit::GemmaRuntimeFitSummary;
use super::quantization::GemmaRuntimeQuantizationMode;
use super::spec::{
    GEMMA4_12B_ATTENTION_HEADS, GEMMA4_12B_DEFAULT_QUANT, GEMMA4_12B_DEFAULT_RUNTIME_WINDOW,
    GEMMA4_12B_DEFAULT_THINKING, GEMMA4_12B_HIDDEN_SIZE, GEMMA4_12B_KV_HEADS,
    GEMMA4_12B_LAYER_COUNT, GEMMA4_12B_LOCAL_WINDOW_TOKENS, GEMMA4_12B_MODEL_ID,
    GEMMA4_12B_NATIVE_CONTEXT_WINDOW, GEMMA4_12B_TOKENIZER,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemmaRuntimeConfig {
    pub program: PathBuf,
    pub model_id: String,
    pub quantization: String,
    pub quantization_mode: GemmaRuntimeQuantizationMode,
    pub runtime_window_tokens: usize,
    pub token_source: Option<String>,
    pub paged_attn: Option<String>,
    pub thinking: Option<String>,
    pub hf_cache: Option<PathBuf>,
    pub passthrough_args: Vec<String>,
}

impl Default for GemmaRuntimeConfig {
    fn default() -> Self {
        Self {
            program: PathBuf::from("mistralrs"),
            model_id: GEMMA4_12B_MODEL_ID.to_owned(),
            quantization: GEMMA4_12B_DEFAULT_QUANT.to_owned(),
            quantization_mode: GemmaRuntimeQuantizationMode::Quant,
            runtime_window_tokens: GEMMA4_12B_DEFAULT_RUNTIME_WINDOW,
            token_source: None,
            paged_attn: None,
            thinking: Some(GEMMA4_12B_DEFAULT_THINKING.to_owned()),
            hf_cache: None,
            passthrough_args: Vec::new(),
        }
    }
}

impl GemmaRuntimeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_program(mut self, program: impl Into<PathBuf>) -> Self {
        self.program = program.into();
        self
    }

    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = model_id.into();
        self
    }

    pub fn with_quantization(mut self, quantization: impl Into<String>) -> Self {
        self.quantization = quantization.into();
        self.quantization_mode = GemmaRuntimeQuantizationMode::Quant;
        self
    }

    pub fn with_isq(mut self, quantization: impl Into<String>) -> Self {
        self.quantization = quantization.into();
        self.quantization_mode = GemmaRuntimeQuantizationMode::Isq;
        self
    }

    pub fn with_runtime_window_tokens(mut self, runtime_window_tokens: usize) -> Self {
        self.runtime_window_tokens = runtime_window_tokens.max(1);
        self
    }

    pub fn with_token_source(mut self, token_source: impl Into<String>) -> Self {
        self.token_source = Some(token_source.into());
        self
    }

    pub fn without_token_source(mut self) -> Self {
        self.token_source = None;
        self
    }

    pub fn with_paged_attn(mut self, paged_attn: impl Into<String>) -> Self {
        self.paged_attn = Some(paged_attn.into());
        self
    }

    pub fn with_thinking(mut self, thinking: impl Into<String>) -> Self {
        self.thinking = Some(thinking.into());
        self
    }

    pub fn without_thinking(mut self) -> Self {
        self.thinking = None;
        self
    }

    pub fn with_hf_cache(mut self, hf_cache: impl Into<PathBuf>) -> Self {
        self.hf_cache = Some(hf_cache.into());
        self
    }

    pub fn with_passthrough_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.passthrough_args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata::new(
            self.model_id.clone(),
            GEMMA4_12B_TOKENIZER,
            self.runtime_window_tokens
                .min(GEMMA4_12B_NATIVE_CONTEXT_WINDOW),
            GEMMA4_12B_HIDDEN_SIZE,
        )
        .with_kv_precision(8, 4)
    }

    pub fn architecture(&self) -> TransformerRuntimeArchitecture {
        TransformerRuntimeArchitecture::new(
            GEMMA4_12B_LAYER_COUNT,
            GEMMA4_12B_HIDDEN_SIZE,
            GEMMA4_12B_ATTENTION_HEADS,
            GEMMA4_12B_KV_HEADS,
            GEMMA4_12B_LOCAL_WINDOW_TOKENS.min(self.runtime_window_tokens),
        )
    }

    pub fn manifest(&self) -> RuntimeManifest {
        RuntimeManifest::from_metadata(self.metadata()).with_architecture(self.architecture())
    }

    pub fn command_args(&self) -> Vec<String> {
        let mut args = vec!["run".to_owned()];
        if let Some(thinking) = &self.thinking {
            args.push("--thinking".to_owned());
            args.push(thinking.clone());
        }
        args.push("-i".to_owned());
        args.push("{user_prompt}".to_owned());
        args.push("auto".to_owned());
        if let Some(token_source) = &self.token_source {
            args.push("--token-source".to_owned());
            args.push(token_source.clone());
        }
        args.push(self.quantization_mode.flag().to_owned());
        args.push(self.quantization.clone());
        args.push("-m".to_owned());
        args.push(self.model_id.clone());
        if let Some(hf_cache) = &self.hf_cache {
            args.push("--hf-cache".to_owned());
            args.push(hf_cache.display().to_string());
        }
        args.push("--max-seq-len".to_owned());
        args.push(
            self.runtime_window_tokens
                .min(GEMMA4_12B_NATIVE_CONTEXT_WINDOW)
                .to_string(),
        );
        if let Some(paged_attn) = &self.paged_attn {
            args.push("--paged-attn".to_owned());
            args.push(paged_attn.clone());
        }
        args.extend(self.passthrough_args.clone());
        args
    }

    pub fn serve_args(&self, host: &str, port: u16) -> Vec<String> {
        let mut args = vec![
            "serve".to_owned(),
            "--host".to_owned(),
            host.to_owned(),
            "--port".to_owned(),
            port.to_string(),
            "--no-ui".to_owned(),
            "auto".to_owned(),
        ];
        if let Some(token_source) = &self.token_source {
            args.push("--token-source".to_owned());
            args.push(token_source.clone());
        }
        args.push(self.quantization_mode.flag().to_owned());
        args.push(self.quantization.clone());
        args.push("-m".to_owned());
        args.push(self.model_id.clone());
        if let Some(hf_cache) = &self.hf_cache {
            args.push("--hf-cache".to_owned());
            args.push(hf_cache.display().to_string());
        }
        args.push("--max-seq-len".to_owned());
        args.push(
            self.runtime_window_tokens
                .min(GEMMA4_12B_NATIVE_CONTEXT_WINDOW)
                .to_string(),
        );
        if let Some(paged_attn) = &self.paged_attn {
            args.push("--paged-attn".to_owned());
            args.push(paged_attn.clone());
        }
        args.extend(self.passthrough_args.clone());
        args
    }

    pub fn command_runtime(&self) -> CommandRuntime {
        let args = self.command_args();
        let activation_payload = self.command_runtime_activation_payload(&args);
        CommandRuntime::new(self.program.clone())
            .args(args)
            .prompt_mode(CommandPromptMode::Args)
            .wire_format(CommandWireFormat::Text)
            .text_output_filter(CommandTextOutputFilter::MistralRsCli)
            .with_metadata(self.manifest().runtime_metadata())
            .with_architecture(self.architecture())
            .with_development_pollution_activation_gate(
                "gemma_command_runtime",
                "model_weight_load",
                activation_payload,
            )
    }

    fn command_runtime_activation_payload(&self, args: &[String]) -> String {
        let mut parts = vec![
            format!("program={}", self.program.display()),
            format!("model_id={}", self.model_id),
            format!("quantization={}", self.quantization),
            format!("quantization_mode={}", self.quantization_mode.flag()),
        ];
        if let Some(token_source) = &self.token_source {
            parts.push(format!("token_source={token_source}"));
        }
        if let Some(hf_cache) = &self.hf_cache {
            parts.push(format!("hf_cache={}", hf_cache.display()));
        }
        if !self.passthrough_args.is_empty() {
            parts.push(format!(
                "passthrough_args={}",
                self.passthrough_args.join(" ")
            ));
        }
        parts.push(format!("args={}", args.join(" ")));
        parts.join(" ")
    }

    pub fn http_runtime(
        &self,
        base_url: impl AsRef<str>,
    ) -> Result<MistralRsHttpRuntime, RuntimeError> {
        Ok(MistralRsHttpRuntime::new(base_url)?
            .with_metadata(self.manifest().runtime_metadata())
            .with_architecture(self.architecture()))
    }

    pub fn fit_summary(&self, total_vram_mib: usize) -> GemmaRuntimeFitSummary {
        GemmaRuntimeFitSummary::for_vram(total_vram_mib)
    }
}
