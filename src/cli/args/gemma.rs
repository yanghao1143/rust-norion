use std::path::PathBuf;

use rust_norion::{
    GEMMA4_12B_DEFAULT_LOCAL_ISQ, GEMMA4_12B_DEFAULT_PAGED_ATTN, GEMMA4_12B_DEFAULT_RUNTIME_WINDOW,
    GemmaRuntimeConfig, GemmaRuntimeQuantizationMode, RuntimeMetadata, TaskProfile,
    ensure_gemma4_12b_runtime_defaults, infer_hf_cache_from_local_snapshot,
};

use crate::gemma_business::paths::gemma_smoke_run_dir;
use crate::gemma_business::{
    GEMMA_BUSINESS_CYCLE_SMOKE_DIR, GEMMA_BUSINESS_SMOKE_DIR, GEMMA_BUSINESS_SMOKE_PROMPT,
    GEMMA_MODEL_SERVICE_SMOKE_DIR, GEMMA_SMOKE_DEFAULT_KEEP_RUNS,
    GEMMA4_12B_SMOKE_COLD_START_TIMEOUT_MS,
};

use super::values::parse_usize;

const DEFAULT_NOIRON_PROMPT: &str = "Design a Rust Noiron prototype with adaptive routing, KV fusion, hierarchy control, and reflection.";

pub(crate) const DEFAULT_SMOKE_KEEP_RUNS: usize = GEMMA_SMOKE_DEFAULT_KEEP_RUNS;

#[derive(Clone, Copy)]
pub(crate) enum GemmaSmokeKind {
    Business,
    BusinessCycle,
    ModelService,
}

impl GemmaSmokeKind {
    pub(crate) fn from_flags(
        business_smoke: bool,
        business_cycle_smoke: bool,
        model_service_smoke: bool,
    ) -> Option<Self> {
        if model_service_smoke {
            Some(Self::ModelService)
        } else if business_cycle_smoke {
            Some(Self::BusinessCycle)
        } else if business_smoke {
            Some(Self::Business)
        } else {
            None
        }
    }

    fn run_dir(self) -> PathBuf {
        gemma_smoke_run_dir(match self {
            Self::Business => GEMMA_BUSINESS_SMOKE_DIR,
            Self::BusinessCycle => GEMMA_BUSINESS_CYCLE_SMOKE_DIR,
            Self::ModelService => GEMMA_MODEL_SERVICE_SMOKE_DIR,
        })
    }
}

pub(crate) struct GemmaRuntimeParseOptions {
    pub(crate) enabled: bool,
    pub(crate) program: PathBuf,
    pub(crate) server: Option<String>,
    pub(crate) quantization: String,
    pub(crate) quantization_mode: GemmaRuntimeQuantizationMode,
    pub(crate) token_source: Option<String>,
    pub(crate) paged_attn: Option<String>,
    pub(crate) thinking: Option<String>,
    pub(crate) hf_cache: Option<PathBuf>,
}

impl Default for GemmaRuntimeParseOptions {
    fn default() -> Self {
        let runtime = GemmaRuntimeConfig::default();
        Self {
            enabled: false,
            program: runtime.program,
            server: None,
            quantization: runtime.quantization,
            quantization_mode: runtime.quantization_mode,
            token_source: runtime.token_source,
            paged_attn: runtime.paged_attn,
            thinking: runtime.thinking,
            hf_cache: runtime.hf_cache,
        }
    }
}

impl GemmaRuntimeParseOptions {
    pub(crate) fn ensure_12b_defaults(
        &mut self,
        runtime_metadata: &mut RuntimeMetadata,
        runtime_layer_count: &mut Option<usize>,
        runtime_hidden_size: &mut Option<usize>,
        runtime_attention_heads: &mut Option<usize>,
        runtime_kv_heads: &mut Option<usize>,
        runtime_local_window_tokens: &mut Option<usize>,
    ) {
        ensure_gemma4_12b_runtime_defaults(
            &mut self.enabled,
            runtime_metadata,
            runtime_layer_count,
            runtime_hidden_size,
            runtime_attention_heads,
            runtime_kv_heads,
            runtime_local_window_tokens,
        );
    }

    pub(crate) fn apply_local_snapshot(
        &mut self,
        model_id: &str,
        runtime_metadata: &mut RuntimeMetadata,
    ) {
        runtime_metadata.model_id = model_id.to_owned();
        self.quantization = GEMMA4_12B_DEFAULT_LOCAL_ISQ.to_owned();
        self.quantization_mode = GemmaRuntimeQuantizationMode::Isq;
        self.token_source = Some("none".to_owned());
        self.paged_attn = Some(GEMMA4_12B_DEFAULT_PAGED_ATTN.to_owned());
    }

    pub(crate) fn set_quantization(&mut self, quantization: String) {
        self.quantization = quantization;
        self.quantization_mode = GemmaRuntimeQuantizationMode::Quant;
    }

    pub(crate) fn set_isq(&mut self, isq: String) {
        self.quantization = isq;
        self.quantization_mode = GemmaRuntimeQuantizationMode::Isq;
    }

    fn infer_smoke_hf_cache(&mut self, runtime_metadata: &RuntimeMetadata) {
        if self.hf_cache.is_none() {
            self.hf_cache = infer_hf_cache_from_local_snapshot(&runtime_metadata.model_id);
        }
    }
}

pub(crate) struct GemmaFlagParse<'a> {
    pub(crate) business_smoke: &'a mut bool,
    pub(crate) business_cycle_smoke: &'a mut bool,
    pub(crate) business_regression_gate_path: &'a mut Option<PathBuf>,
    pub(crate) business_cycle_smoke_report_gate_path: &'a mut Option<PathBuf>,
    pub(crate) model_service_smoke: &'a mut bool,
    pub(crate) smoke_check_only: &'a mut bool,
    pub(crate) smoke_keep_runs: &'a mut usize,
    pub(crate) runtime: &'a mut GemmaRuntimeParseOptions,
    pub(crate) runtime_metadata: &'a mut RuntimeMetadata,
    pub(crate) runtime_layer_count: &'a mut Option<usize>,
    pub(crate) runtime_hidden_size: &'a mut Option<usize>,
    pub(crate) runtime_attention_heads: &'a mut Option<usize>,
    pub(crate) runtime_kv_heads: &'a mut Option<usize>,
    pub(crate) runtime_local_window_tokens: &'a mut Option<usize>,
}

impl GemmaFlagParse<'_> {
    pub(crate) fn parse(&mut self, raw: &[String], index: usize) -> Option<usize> {
        let flag = raw.get(index)?.as_str();
        match flag {
            "--gemma-12b-runtime" | "--gemma4-12b-runtime" => {
                self.ensure_12b_defaults();
                Some(1)
            }
            "--gemma-business-smoke" => {
                *self.business_smoke = true;
                self.ensure_12b_defaults();
                Some(1)
            }
            "--gemma-business-cycle-smoke" | "--gemma-cycle-smoke" => {
                *self.business_cycle_smoke = true;
                self.ensure_12b_defaults();
                Some(1)
            }
            "--gemma-business-cycle-smoke-report-gate"
            | "--gemma-cycle-smoke-report-gate"
            | "--gemma-business-cycle-report-gate" => {
                let path = raw.get(index + 1)?;
                *self.business_cycle_smoke_report_gate_path = Some(PathBuf::from(path));
                Some(2)
            }
            "--gemma-business-regression-gate" | "--business-regression-gate" => {
                let path = raw.get(index + 1)?;
                *self.business_regression_gate_path = Some(PathBuf::from(path));
                Some(2)
            }
            "--gemma-model-service-smoke" | "--gemma-service-smoke" => {
                *self.model_service_smoke = true;
                self.ensure_12b_defaults();
                Some(1)
            }
            "--gemma-smoke-check-only" | "--gemma-check-only" => {
                *self.smoke_check_only = true;
                Some(1)
            }
            "--gemma-smoke-keep-runs" | "--gemma-smoke-retain-runs" => {
                let keep_runs = raw.get(index + 1)?;
                *self.smoke_keep_runs = parse_usize(keep_runs, DEFAULT_SMOKE_KEEP_RUNS);
                Some(2)
            }
            "--gemma-runtime-command" => {
                let program = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime.program = PathBuf::from(program);
                Some(2)
            }
            "--gemma-runtime-server" | "--gemma-runtime-url" => {
                let server = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime.server = Some(server.clone());
                Some(2)
            }
            "--gemma-local-snapshot" => {
                let snapshot = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime
                    .apply_local_snapshot(snapshot, self.runtime_metadata);
                Some(2)
            }
            "--gemma-model-id" | "--gemma-runtime-model-id" => {
                let model_id = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime_metadata.model_id = model_id.clone();
                Some(2)
            }
            "--gemma-quant" | "--gemma-runtime-quant" => {
                let quantization = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime.set_quantization(quantization.clone());
                Some(2)
            }
            "--gemma-isq" | "--gemma-runtime-isq" => {
                let isq = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime.set_isq(isq.clone());
                Some(2)
            }
            "--gemma-runtime-window" => {
                let window = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime_metadata.native_context_window =
                    parse_usize(window, default_runtime_window_tokens());
                Some(2)
            }
            "--gemma-token-source" => {
                let token_source = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime.token_source = Some(token_source.clone());
                Some(2)
            }
            "--gemma-paged-attn" => {
                let paged_attn = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime.paged_attn = Some(paged_attn.clone());
                Some(2)
            }
            "--gemma-thinking" => {
                let thinking = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime.thinking = Some(thinking.clone());
                Some(2)
            }
            "--gemma-hf-cache" => {
                let hf_cache = raw.get(index + 1)?;
                self.ensure_12b_defaults();
                self.runtime.hf_cache = Some(PathBuf::from(hf_cache));
                Some(2)
            }
            _ => None,
        }
    }

    fn ensure_12b_defaults(&mut self) {
        self.runtime.ensure_12b_defaults(
            self.runtime_metadata,
            self.runtime_layer_count,
            self.runtime_hidden_size,
            self.runtime_attention_heads,
            self.runtime_kv_heads,
            self.runtime_local_window_tokens,
        );
    }
}

pub(crate) struct GemmaSmokeDefaults<'a> {
    pub(crate) kind: GemmaSmokeKind,
    pub(crate) memory_path: &'a mut PathBuf,
    pub(crate) experience_path: &'a mut PathBuf,
    pub(crate) adaptive_path: &'a mut PathBuf,
    pub(crate) trace_path: &'a mut Option<PathBuf>,
    pub(crate) trace_schema_gate_path: &'a mut Option<PathBuf>,
    pub(crate) memory_path_set: bool,
    pub(crate) experience_path_set: bool,
    pub(crate) adaptive_path_set: bool,
    pub(crate) trace_path_set: bool,
    pub(crate) trace_schema_gate_path_set: bool,
    pub(crate) gemma_runtime: &'a mut GemmaRuntimeParseOptions,
    pub(crate) runtime_metadata: &'a RuntimeMetadata,
    pub(crate) profile: &'a mut Option<TaskProfile>,
    pub(crate) runtime_timeout_ms: &'a mut Option<u64>,
    pub(crate) runtime_args: &'a mut Vec<String>,
}

impl GemmaSmokeDefaults<'_> {
    pub(crate) fn apply(self) {
        let smoke_dir = self.kind.run_dir();
        if !self.memory_path_set {
            *self.memory_path = smoke_dir.join("memory.ndkv");
        }
        if !self.experience_path_set {
            *self.experience_path = smoke_dir.join("experience.ndkv");
        }
        if !self.adaptive_path_set {
            *self.adaptive_path = smoke_dir.join("adaptive.ndkv");
        }
        if !self.trace_path_set {
            let schema_gate_path = self.trace_schema_gate_path.as_ref().cloned();
            *self.trace_path = schema_gate_path.or_else(|| Some(smoke_dir.join("trace.jsonl")));
        }
        if !self.trace_schema_gate_path_set {
            *self.trace_schema_gate_path = self.trace_path.clone();
        }

        self.gemma_runtime
            .infer_smoke_hf_cache(self.runtime_metadata);

        if self.profile.is_none() {
            *self.profile = Some(TaskProfile::Coding);
        }
        if self.runtime_timeout_ms.is_none() {
            *self.runtime_timeout_ms = Some(GEMMA4_12B_SMOKE_COLD_START_TIMEOUT_MS);
        }
        if !self.runtime_args.iter().any(|arg| arg == "--seed") {
            self.runtime_args.push("--seed".to_owned());
            self.runtime_args.push("7".to_owned());
        }
    }
}

pub(crate) fn default_runtime_window_tokens() -> usize {
    GEMMA4_12B_DEFAULT_RUNTIME_WINDOW
}

pub(crate) fn prompt_or_default(
    prompt_parts: &[String],
    gemma_smoke_kind: Option<GemmaSmokeKind>,
) -> String {
    if prompt_parts.is_empty() {
        if gemma_smoke_kind.is_some() {
            GEMMA_BUSINESS_SMOKE_PROMPT.to_owned()
        } else {
            DEFAULT_NOIRON_PROMPT.to_owned()
        }
    } else {
        prompt_parts.join(" ")
    }
}
