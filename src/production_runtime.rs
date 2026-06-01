use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::agent_team::AgentTeamPlan;
use crate::hardware::{
    HardwareAllocator, HardwarePlan, HardwareSnapshot, RuntimeAdapterHint,
    RuntimeManifestDeviceGateReport,
};
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::{ReasoningStep, RuntimeDiagnostics};
use crate::router::RouteBudget;
use crate::runtime::{
    ModelRuntime, RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeResponse,
    RuntimeToken, RuntimeTokenId,
};
use crate::runtime_manifest::{RuntimeManifest, TransformerRuntimeArchitecture};
use crate::toolsmith::ToolsmithPlan;
use crate::transformer::{
    AttentionKind, TransformerLayerPlan, TransformerPlanCounts, TransformerPlanner,
    TransformerRefactorPlan,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAssetSummary {
    pub weights_path: PathBuf,
    pub weights_bytes: u64,
    pub tokenizer_path: PathBuf,
    pub tokenizer_bytes: u64,
    pub config_path: Option<PathBuf>,
    pub config_bytes: Option<u64>,
}

impl RuntimeAssetSummary {
    fn from_manifest(manifest: &RuntimeManifest) -> Result<Self, RuntimeError> {
        let weights_path = manifest.assets.weights.clone().ok_or_else(|| {
            RuntimeError::new("weights asset path is required for production runtimes")
        })?;
        let tokenizer_path = manifest.assets.tokenizer.clone().ok_or_else(|| {
            RuntimeError::new("tokenizer asset path is required for production runtimes")
        })?;
        let config_path = manifest.assets.config.clone();

        Ok(Self {
            weights_bytes: asset_len("weights", &weights_path)?,
            tokenizer_bytes: asset_len("tokenizer", &tokenizer_path)?,
            config_bytes: config_path
                .as_deref()
                .map(|path| asset_len("config", path))
                .transpose()?,
            weights_path,
            tokenizer_path,
            config_path,
        })
    }

    pub fn summary_line(&self) -> String {
        format!(
            "weights={} weights_bytes={} tokenizer={} tokenizer_bytes={} config={} config_bytes={}",
            self.weights_path.display(),
            self.weights_bytes,
            self.tokenizer_path.display(),
            self.tokenizer_bytes,
            self.config_path
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.config_bytes
                .map(|bytes| bytes.to_string())
                .unwrap_or_else(|| "none".to_owned())
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProductionTransformerRuntime {
    manifest: RuntimeManifest,
    device_gate: RuntimeManifestDeviceGateReport,
    assets: RuntimeAssetSummary,
    imported_kv_blocks: Vec<RuntimeKvBlock>,
    exported_kv_blocks: Vec<RuntimeKvBlock>,
    kernel: Option<Arc<dyn ProductionForwardKernel>>,
}

pub trait ProductionForwardKernel: std::fmt::Debug + Send + Sync {
    fn generate(
        &self,
        context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionKernelConformanceGate {
    pub require_tokens: bool,
    pub require_trace: bool,
    pub require_forward_energy: bool,
    pub require_kv_influence: bool,
    pub require_kv_export_when_enabled: bool,
}

impl Default for ProductionKernelConformanceGate {
    fn default() -> Self {
        Self {
            require_tokens: true,
            require_trace: true,
            require_forward_energy: true,
            require_kv_influence: true,
            require_kv_export_when_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProductionKernelConformanceReport {
    pub passed: bool,
    pub model_id: String,
    pub selected_adapter: String,
    pub kernel_connected: bool,
    pub token_count: usize,
    pub trace_steps: usize,
    pub imported_kv_blocks: usize,
    pub exported_kv_blocks: usize,
    pub forward_energy: Option<f32>,
    pub kv_influence: Option<f32>,
    pub failures: Vec<String>,
}

impl ProductionKernelConformanceReport {
    fn new(model_id: &str, selected_adapter: &str, kernel_connected: bool) -> Self {
        Self {
            passed: false,
            model_id: model_id.to_owned(),
            selected_adapter: selected_adapter.to_owned(),
            kernel_connected,
            token_count: 0,
            trace_steps: 0,
            imported_kv_blocks: 0,
            exported_kv_blocks: 0,
            forward_energy: None,
            kv_influence: None,
            failures: Vec::new(),
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "production_kernel_conformance: passed={} model_id={} adapter={} kernel_connected={} tokens={} trace_steps={} imported_kv={} exported_kv={} forward_energy={} kv_influence={} failures={}",
            self.passed,
            self.model_id,
            self.selected_adapter,
            self.kernel_connected,
            self.token_count,
            self.trace_steps,
            self.imported_kv_blocks,
            self.exported_kv_blocks,
            option_f32_display(self.forward_energy),
            option_f32_display(self.kv_influence),
            self.failures.len()
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProductionKernelContext<'a> {
    pub manifest: &'a RuntimeManifest,
    pub device_gate: &'a RuntimeManifestDeviceGateReport,
    pub assets: &'a RuntimeAssetSummary,
    pub imported_kv_blocks: &'a [RuntimeKvBlock],
    pub request: &'a RuntimeRequest,
}

#[derive(Debug, Clone)]
pub struct ProductionKernelOutput {
    pub answer: String,
    pub tokens: Vec<RuntimeToken>,
    pub trace: Vec<ReasoningStep>,
    pub diagnostics: RuntimeDiagnostics,
    pub exported_kv_blocks: Vec<RuntimeKvBlock>,
}

impl ProductionKernelOutput {
    pub fn new(answer: impl Into<String>) -> Self {
        Self {
            answer: answer.into(),
            tokens: Vec::new(),
            trace: Vec::new(),
            diagnostics: RuntimeDiagnostics::default(),
            exported_kv_blocks: Vec::new(),
        }
    }

    pub fn with_tokens(mut self, tokens: Vec<RuntimeToken>) -> Self {
        self.tokens = tokens;
        self
    }

    pub fn with_trace(mut self, trace: Vec<ReasoningStep>) -> Self {
        self.trace = trace;
        self
    }

    pub fn with_diagnostics(mut self, diagnostics: RuntimeDiagnostics) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn with_exported_kv_blocks(mut self, blocks: Vec<RuntimeKvBlock>) -> Self {
        self.exported_kv_blocks = blocks;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceProductionForwardKernel;

impl ReferenceProductionForwardKernel {
    pub fn new() -> Self {
        Self
    }
}

impl ProductionForwardKernel for ReferenceProductionForwardKernel {
    fn generate(
        &self,
        context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        let token_ids = production_token_ids(&context.request.prompt);
        let forward = run_reference_forward(&token_ids, context);
        let counts = count_reference_layers(&forward.layer_summaries);
        let exported_kv_blocks = export_reference_kv(&forward, context);
        let answer = reference_answer(context, token_ids.len(), &forward, counts);
        let tokens = answer
            .split_whitespace()
            .map(|text| {
                let entropy = estimated_entropy(text, forward.energy, forward.kv_influence);
                RuntimeToken {
                    text: text.to_owned(),
                    logprob: Some(-entropy),
                    entropy: Some(entropy),
                }
            })
            .collect::<Vec<_>>();
        let trace = vec![
            ReasoningStep::new(
                "reference_production_kernel",
                format!(
                    "executed manifest-backed Rust reference kernel for model_id={} adapter={}",
                    context.manifest.metadata.model_id,
                    context.device_gate.runtime_adapter_name()
                ),
                0.84,
            ),
            ReasoningStep::new(
                "reference_transformer_forward",
                format!(
                    "ran {} layers with hidden size {} energy {:.3} kv_influence {:.3}",
                    forward.layer_summaries.len(),
                    context.manifest.architecture.hidden_size,
                    forward.energy,
                    forward.kv_influence
                ),
                0.82,
            ),
            ReasoningStep::new(
                "reference_kv_exchange",
                format!(
                    "received {} imported KV blocks and prepared {} exported KV blocks",
                    context.imported_kv_blocks.len(),
                    exported_kv_blocks.len()
                ),
                0.80,
            ),
        ];
        let diagnostics = RuntimeDiagnostics {
            model_id: Some(context.manifest.metadata.model_id.clone()),
            selected_adapter: context
                .device_gate
                .runtime_adapter
                .map(|adapter| adapter.as_str().to_owned()),
            layer_count: forward.layer_summaries.len(),
            hidden_size: context.manifest.architecture.hidden_size,
            local_window_tokens: context.manifest.architecture.local_window_tokens,
            forward_energy: Some(forward.energy),
            kv_influence: Some(forward.kv_influence),
            imported_kv_blocks: context.imported_kv_blocks.len(),
            exported_kv_blocks: exported_kv_blocks.len(),
        };

        Ok(ProductionKernelOutput::new(answer)
            .with_tokens(tokens)
            .with_trace(trace)
            .with_diagnostics(diagnostics)
            .with_exported_kv_blocks(exported_kv_blocks))
    }
}

#[derive(Debug, Clone)]
pub struct ModelRuntimeForwardKernel<R> {
    runtime: Arc<Mutex<R>>,
}

impl<R> ModelRuntimeForwardKernel<R> {
    pub fn new(runtime: R) -> Self {
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
        }
    }

    pub fn with_shared_runtime(runtime: Arc<Mutex<R>>) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> Arc<Mutex<R>> {
        Arc::clone(&self.runtime)
    }
}

impl<R> ProductionForwardKernel for ModelRuntimeForwardKernel<R>
where
    R: ModelRuntime + std::fmt::Debug + Send + 'static,
{
    fn generate(
        &self,
        context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::new("model runtime forward kernel lock is poisoned"))?;
        let imported = runtime.import_kv(context.imported_kv_blocks)?;
        let mut request = context.request.clone();
        request.runtime_metadata = context.manifest.runtime_metadata();
        request.runtime_architecture = context.manifest.architecture;

        let response = runtime.generate(request)?;
        let exported_kv_blocks = runtime.export_kv()?;
        let mut diagnostics = response.diagnostics;
        diagnostics.imported_kv_blocks = imported;
        diagnostics.exported_kv_blocks = exported_kv_blocks.len();

        Ok(ProductionKernelOutput::new(response.answer)
            .with_tokens(response.tokens)
            .with_trace(response.trace)
            .with_diagnostics(diagnostics)
            .with_exported_kv_blocks(exported_kv_blocks))
    }
}

impl ProductionTransformerRuntime {
    pub fn from_manifest_for_plan(
        manifest: RuntimeManifest,
        plan: &HardwarePlan,
    ) -> Result<Self, RuntimeError> {
        let validation = manifest.validate_for_production();
        if !validation.passed() {
            return Err(RuntimeError::new(format!(
                "production runtime manifest rejected for model_id={}: {}",
                manifest.metadata.model_id,
                validation.errors.join("; ")
            )));
        }

        let device_gate = RuntimeManifestDeviceGateReport::evaluate(&manifest, plan);
        if !device_gate.passed() {
            return Err(RuntimeError::new(format!(
                "production runtime device gate rejected model_id={} device={} adapter={}: {}",
                manifest.metadata.model_id,
                device_gate.device.as_str(),
                device_gate.runtime_adapter_name(),
                device_gate.failures.join("; ")
            )));
        }

        let assets = RuntimeAssetSummary::from_manifest(&manifest)?;

        Ok(Self {
            manifest,
            device_gate,
            assets,
            imported_kv_blocks: Vec::new(),
            exported_kv_blocks: Vec::new(),
            kernel: None,
        })
    }

    pub fn manifest(&self) -> &RuntimeManifest {
        &self.manifest
    }

    pub fn device_gate(&self) -> &RuntimeManifestDeviceGateReport {
        &self.device_gate
    }

    pub fn assets(&self) -> &RuntimeAssetSummary {
        &self.assets
    }

    pub fn selected_adapter(&self) -> Option<RuntimeAdapterHint> {
        self.device_gate.runtime_adapter
    }

    pub fn runtime_device_contract(&self) -> &str {
        &self.device_gate.runtime_device_contract
    }

    pub fn imported_kv_blocks(&self) -> &[RuntimeKvBlock] {
        &self.imported_kv_blocks
    }

    pub fn exported_kv_blocks(&self) -> &[RuntimeKvBlock] {
        &self.exported_kv_blocks
    }

    pub fn with_kernel<K>(mut self, kernel: K) -> Self
    where
        K: ProductionForwardKernel + 'static,
    {
        self.kernel = Some(Arc::new(kernel));
        self
    }

    pub fn with_shared_kernel(mut self, kernel: Arc<dyn ProductionForwardKernel>) -> Self {
        self.kernel = Some(kernel);
        self
    }

    pub fn kernel_connected(&self) -> bool {
        self.kernel.is_some()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "production_runtime: model_id={} device={} adapter={} weights_bytes={} tokenizer_bytes={} kernel={}",
            self.manifest.metadata.model_id,
            self.device_gate.device.as_str(),
            self.device_gate.runtime_adapter_name(),
            self.assets.weights_bytes,
            self.assets.tokenizer_bytes,
            if self.kernel_connected() {
                "connected"
            } else {
                "not-connected"
            }
        )
    }

    pub fn conformance_report(
        &self,
        gate: ProductionKernelConformanceGate,
    ) -> ProductionKernelConformanceReport {
        let mut report = ProductionKernelConformanceReport::new(
            &self.manifest.metadata.model_id,
            self.device_gate.runtime_adapter_name(),
            self.kernel_connected(),
        );

        if !self.kernel_connected() {
            report
                .failures
                .push("production forward kernel is not connected".to_owned());
            return report;
        }

        let mut runtime = self.clone();
        let import_blocks = conformance_import_blocks(&self.manifest);
        match runtime.import_kv(&import_blocks) {
            Ok(imported) => {
                report.imported_kv_blocks = imported;
                if self.manifest.kv_policy.import_enabled && imported == 0 {
                    report.failures.push(
                        "runtime KV import is enabled but conformance import admitted no blocks"
                            .to_owned(),
                    );
                }
            }
            Err(error) => {
                report
                    .failures
                    .push(format!("conformance KV import failed: {}", error.message()));
                return report;
            }
        }

        let request = conformance_request(&self.manifest, &self.device_gate);
        let response = match runtime.generate(request) {
            Ok(response) => response,
            Err(error) => {
                report.failures.push(format!(
                    "conformance generation failed: {}",
                    error.message()
                ));
                return report;
            }
        };

        report.token_count = response.tokens.len();
        report.trace_steps = response.trace.len();
        report.forward_energy = response.diagnostics.forward_energy;
        report.kv_influence = response.diagnostics.kv_influence;
        report.exported_kv_blocks = runtime.exported_kv_blocks().len();
        report.imported_kv_blocks = response.diagnostics.imported_kv_blocks;

        evaluate_conformance_response(&self.manifest, gate, &response, &mut report);

        match runtime.export_kv() {
            Ok(exported) => {
                if exported.len() != report.exported_kv_blocks {
                    report.failures.push(format!(
                        "export_kv returned {} blocks but diagnostics/runtime recorded {}",
                        exported.len(),
                        report.exported_kv_blocks
                    ));
                }
            }
            Err(error) => {
                report
                    .failures
                    .push(format!("conformance KV export failed: {}", error.message()));
            }
        }

        report.passed = report.failures.is_empty();
        report
    }
}

impl ModelRuntime for ProductionTransformerRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        self.manifest.runtime_metadata()
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        self.manifest.architecture
    }

    fn tokenize(&self, prompt: &str) -> Result<Vec<RuntimeTokenId>, RuntimeError> {
        Ok(production_tokenize(prompt)
            .into_iter()
            .map(|text| RuntimeTokenId::new((stable_hash(&text) % 1_000_000) as u32, text))
            .collect())
    }

    fn embed(&self, tokens: &[RuntimeTokenId]) -> Result<RuntimeEmbedding, RuntimeError> {
        Ok(RuntimeEmbedding::new(embed_tokens(
            tokens,
            self.manifest.metadata.embedding_dimensions,
        )))
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        if !self.manifest.kv_policy.import_enabled {
            self.imported_kv_blocks.clear();
            return Ok(0);
        }

        let max_blocks = self
            .manifest
            .kv_policy
            .max_import_blocks
            .min(self.device_gate.kv_prefetch_blocks);
        self.imported_kv_blocks.clear();
        self.imported_kv_blocks = validate_imported_kv_blocks(blocks, max_blocks, &self.manifest)?;

        Ok(self.imported_kv_blocks.len())
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        if !self.manifest.kv_policy.export_enabled {
            self.exported_kv_blocks.clear();
            return Ok(Vec::new());
        }

        Ok(self
            .exported_kv_blocks
            .iter()
            .take(self.manifest.kv_policy.max_export_blocks)
            .cloned()
            .collect())
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        if let Some(kernel) = &self.kernel {
            self.exported_kv_blocks.clear();
            let output = kernel.generate(ProductionKernelContext {
                manifest: &self.manifest,
                device_gate: &self.device_gate,
                assets: &self.assets,
                imported_kv_blocks: &self.imported_kv_blocks,
                request: &request,
            })?;
            self.exported_kv_blocks =
                validate_exported_kv_blocks(output.exported_kv_blocks, &self.manifest, &request)?;

            let mut response =
                RuntimeResponse::new(output.answer).with_diagnostics(normalize_kernel_diagnostics(
                    output.diagnostics,
                    &self.manifest,
                    &self.device_gate,
                    self.imported_kv_blocks.len(),
                    self.exported_kv_blocks.len(),
                ));
            response.tokens = output.tokens;
            response.trace = output.trace;

            return Ok(response);
        }

        Err(RuntimeError::new(format!(
            "production Transformer kernel is not connected for model_id={} adapter={} device={}; manifest assets and device contract passed, but a self-developed forward kernel must implement generate for prompt '{}'",
            self.manifest.metadata.model_id,
            self.device_gate.runtime_adapter_name(),
            self.device_gate.device.as_str(),
            compact(&request.prompt, 96)
        )))
    }
}

fn validate_imported_kv_blocks(
    blocks: &[RuntimeKvBlock],
    max_blocks: usize,
    manifest: &RuntimeManifest,
) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
    let mut accepted = Vec::new();
    for (index, block) in blocks.iter().take(max_blocks).enumerate() {
        validate_kv_block(
            index,
            block,
            manifest,
            manifest.metadata.native_context_window,
            "imported",
            "control plane supplied invalid imported KV block",
        )?;
        accepted.push(block.clone());
    }

    Ok(accepted)
}

fn validate_exported_kv_blocks(
    blocks: Vec<RuntimeKvBlock>,
    manifest: &RuntimeManifest,
    request: &RuntimeRequest,
) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
    if !manifest.kv_policy.export_enabled {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        return Err(RuntimeError::new(format!(
            "production kernel exported {} KV blocks but runtime KV export is disabled for model_id={}",
            blocks.len(),
            manifest.metadata.model_id
        )));
    }

    let token_upper_bound = manifest
        .metadata
        .native_context_window
        .max(request.runtime_metadata.native_context_window)
        .max(request.recursive_schedule.prompt_tokens)
        .saturating_add(request.max_tokens.max(1));
    let mut accepted = Vec::new();
    for (index, block) in blocks
        .into_iter()
        .take(manifest.kv_policy.max_export_blocks)
        .enumerate()
    {
        validate_kv_block(
            index,
            &block,
            manifest,
            token_upper_bound,
            "exported",
            "production kernel returned invalid exported KV block",
        )?;
        accepted.push(block);
    }

    Ok(accepted)
}

fn validate_kv_block(
    index: usize,
    block: &RuntimeKvBlock,
    manifest: &RuntimeManifest,
    token_upper_bound: usize,
    direction: &str,
    prefix: &str,
) -> Result<(), RuntimeError> {
    let architecture = manifest.architecture;
    let token_span = block.token_end.saturating_sub(block.token_start).max(1);
    let per_token_vector_bound = architecture
        .hidden_size
        .max(manifest.metadata.embedding_dimensions)
        .max(1);
    let vector_bound = per_token_vector_bound.saturating_mul(token_span);

    let error = |reason: String| {
        RuntimeError::new(format!(
            "{prefix} {index} for model_id={}: {reason}",
            manifest.metadata.model_id
        ))
    };

    if block.layer >= architecture.layer_count {
        return Err(error(format!(
            "layer {} exceeds manifest layer_count {}",
            block.layer, architecture.layer_count
        )));
    }
    if block.head >= architecture.kv_heads {
        return Err(error(format!(
            "head {} exceeds manifest kv_heads {}",
            block.head, architecture.kv_heads
        )));
    }
    if block.token_start >= block.token_end {
        return Err(error(format!(
            "token range {}..{} is empty or reversed",
            block.token_start, block.token_end
        )));
    }
    if block.token_end > token_upper_bound {
        return Err(error(format!(
            "token_end {} exceeds KV token bound {}",
            block.token_end, token_upper_bound
        )));
    }
    if block.key.is_empty() || block.value.is_empty() {
        return Err(error(
            "key and value vectors must both be non-empty".to_owned(),
        ));
    }
    if block.key.len() != block.value.len() {
        return Err(error(format!(
            "key/value dimensions differ: key={} value={}",
            block.key.len(),
            block.value.len()
        )));
    }
    if block.key.len() > vector_bound {
        return Err(error(format!(
            "key/value dimensions {} exceed per-block bound {}",
            block.key.len(),
            vector_bound
        )));
    }
    if !block.key.iter().all(|value| value.is_finite()) {
        return Err(error(format!("{direction} key contains non-finite value")));
    }
    if !block.value.iter().all(|value| value.is_finite()) {
        return Err(error(format!(
            "{direction} value contains non-finite value"
        )));
    }

    Ok(())
}

fn normalize_kernel_diagnostics(
    mut diagnostics: RuntimeDiagnostics,
    manifest: &RuntimeManifest,
    device_gate: &RuntimeManifestDeviceGateReport,
    imported_kv_blocks: usize,
    exported_kv_blocks: usize,
) -> RuntimeDiagnostics {
    if diagnostics.model_id.is_none() {
        diagnostics.model_id = Some(manifest.metadata.model_id.clone());
    }
    if diagnostics.selected_adapter.is_none() {
        diagnostics.selected_adapter = device_gate
            .runtime_adapter
            .map(|adapter| adapter.as_str().to_owned());
    }
    if diagnostics.layer_count == 0 {
        diagnostics.layer_count = manifest.architecture.layer_count;
    }
    if diagnostics.hidden_size == 0 {
        diagnostics.hidden_size = manifest.architecture.hidden_size;
    }
    if diagnostics.local_window_tokens == 0 {
        diagnostics.local_window_tokens = manifest.architecture.local_window_tokens;
    }
    diagnostics.imported_kv_blocks = imported_kv_blocks;
    diagnostics.exported_kv_blocks = exported_kv_blocks;
    diagnostics
}

fn conformance_import_blocks(manifest: &RuntimeManifest) -> Vec<RuntimeKvBlock> {
    if !manifest.kv_policy.import_enabled || manifest.kv_policy.max_import_blocks == 0 {
        return Vec::new();
    }

    let dims = manifest
        .architecture
        .hidden_size
        .max(manifest.metadata.embedding_dimensions)
        .clamp(1, 16);
    vec![RuntimeKvBlock::new(
        0,
        0,
        0,
        1,
        deterministic_vector("conformance-key", dims),
        deterministic_vector("conformance-value", dims),
    )]
}

fn conformance_request(
    manifest: &RuntimeManifest,
    device_gate: &RuntimeManifestDeviceGateReport,
) -> RuntimeRequest {
    let prompt = format!(
        "Run production kernel conformance for {} with KV import and export diagnostics.",
        manifest.metadata.model_id
    );
    let prompt_tokens = prompt.split_whitespace().count().max(1);
    let hardware_plan = HardwareAllocator::new().plan(
        HardwareSnapshot::new(device_gate.device, 0.35, 0.30, 0.45, 0.20),
        TaskProfile::Coding,
        prompt_tokens,
        HierarchyWeights::default(),
    );

    RuntimeRequest {
        prompt,
        profile: TaskProfile::Coding,
        runtime_metadata: manifest.runtime_metadata(),
        runtime_architecture: manifest.architecture,
        memory_hints: Vec::new(),
        infini_memory_hints: Vec::new(),
        experience_hints: Vec::new(),
        runtime_adapter_observations: Vec::new(),
        toolsmith_plan: ToolsmithPlan::default(),
        agent_team_plan: AgentTeamPlan::default(),
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 2,
            fast_tokens: 1,
            attention_fraction: 2.0 / 3.0,
        },
        hierarchy: HierarchyWeights::default(),
        transformer_plan: TransformerRefactorPlan::default(),
        recursive_schedule: crate::recursive_scheduler::RecursiveSchedule::default(),
        hardware_plan,
        max_tokens: 32,
    }
}

fn evaluate_conformance_response(
    manifest: &RuntimeManifest,
    gate: ProductionKernelConformanceGate,
    response: &RuntimeResponse,
    report: &mut ProductionKernelConformanceReport,
) {
    if response.answer.trim().is_empty() {
        report
            .failures
            .push("kernel returned an empty answer".to_owned());
    }

    if gate.require_tokens && response.tokens.is_empty() {
        report
            .failures
            .push("kernel did not return runtime token uncertainty records".to_owned());
    }
    if response
        .tokens
        .iter()
        .any(|token| token.text.trim().is_empty())
    {
        report
            .failures
            .push("kernel returned a runtime token with empty text".to_owned());
    }
    if response.tokens.iter().any(|token| {
        token.entropy.is_some_and(|value| !value.is_finite())
            || token.logprob.is_some_and(|value| !value.is_finite())
    }) {
        report
            .failures
            .push("kernel returned non-finite runtime token uncertainty".to_owned());
    }

    if gate.require_trace && response.trace.is_empty() {
        report
            .failures
            .push("kernel did not return reasoning trace steps".to_owned());
    }
    if response
        .trace
        .iter()
        .any(|step| step.label.trim().is_empty() || step.confidence.is_nan())
    {
        report
            .failures
            .push("kernel returned malformed reasoning trace steps".to_owned());
    }

    let diagnostics = &response.diagnostics;
    if diagnostics.model_id.as_deref() != Some(manifest.metadata.model_id.as_str()) {
        report.failures.push(format!(
            "diagnostics model_id {:?} does not match manifest model_id {}",
            diagnostics.model_id, manifest.metadata.model_id
        ));
    }
    if diagnostics.layer_count != manifest.architecture.layer_count {
        report.failures.push(format!(
            "diagnostics layer_count {} does not match manifest layer_count {}",
            diagnostics.layer_count, manifest.architecture.layer_count
        ));
    }
    if diagnostics.hidden_size != manifest.architecture.hidden_size {
        report.failures.push(format!(
            "diagnostics hidden_size {} does not match manifest hidden_size {}",
            diagnostics.hidden_size, manifest.architecture.hidden_size
        ));
    }
    if diagnostics.local_window_tokens != manifest.architecture.local_window_tokens {
        report.failures.push(format!(
            "diagnostics local_window_tokens {} does not match manifest local_window_tokens {}",
            diagnostics.local_window_tokens, manifest.architecture.local_window_tokens
        ));
    }
    if gate.require_forward_energy
        && diagnostics
            .forward_energy
            .filter(|value| value.is_finite() && *value > 0.0)
            .is_none()
    {
        report
            .failures
            .push("kernel did not report positive finite forward_energy".to_owned());
    }
    if gate.require_kv_influence
        && diagnostics
            .kv_influence
            .filter(|value| value.is_finite() && *value >= 0.0)
            .is_none()
    {
        report
            .failures
            .push("kernel did not report finite non-negative kv_influence".to_owned());
    }
    if gate.require_kv_export_when_enabled
        && manifest.kv_policy.export_enabled
        && diagnostics.exported_kv_blocks == 0
    {
        report
            .failures
            .push("runtime KV export is enabled but kernel exported no KV blocks".to_owned());
    }
    if diagnostics.exported_kv_blocks != report.exported_kv_blocks {
        report.failures.push(format!(
            "diagnostics exported_kv_blocks {} does not match runtime exported KV {}",
            diagnostics.exported_kv_blocks, report.exported_kv_blocks
        ));
    }
}

fn deterministic_vector(seed: &str, dims: usize) -> Vec<f32> {
    let dims = dims.max(1);
    let mut vector = (0..dims)
        .map(|index| {
            let hash = stable_hash(&format!("{seed}:{index}"));
            ((hash % 997) as f32 / 997.0) - 0.5
        })
        .collect::<Vec<_>>();
    normalize(&mut vector);
    vector
}

fn option_f32_display(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "none".to_owned())
}

#[derive(Debug, Clone)]
struct ReferenceForwardState {
    vector: Vec<f32>,
    layer_summaries: Vec<ReferenceLayerSummary>,
    energy: f32,
    kv_influence: f32,
}

#[derive(Debug, Clone)]
struct ReferenceLayerSummary {
    layer_index: usize,
    attention: AttentionKind,
    window_size: usize,
    compute_fraction: f32,
    activation: f32,
}

fn production_token_ids(prompt: &str) -> Vec<RuntimeTokenId> {
    production_tokenize(prompt)
        .into_iter()
        .map(|text| RuntimeTokenId::new((stable_hash(&text) % 1_000_000) as u32, text))
        .collect()
}

fn run_reference_forward(
    token_ids: &[RuntimeTokenId],
    context: ProductionKernelContext<'_>,
) -> ReferenceForwardState {
    let mut vector = embed_tokens(
        token_ids,
        context.manifest.metadata.embedding_dimensions.max(1),
    );
    if vector.is_empty() {
        vector.push(0.0);
    }

    let layers = reference_layers(context);
    let mut layer_summaries = Vec::with_capacity(layers.len());
    let mut kv_influence = 0.0;

    for layer in &layers {
        kv_influence += apply_reference_imported_kv(&mut vector, context.imported_kv_blocks, layer);
        apply_reference_layer(
            &mut vector,
            layer,
            context.request.route_budget.attention_fraction,
            context.assets.weights_bytes,
        );
        layer_summaries.push(ReferenceLayerSummary {
            layer_index: layer.layer_index,
            attention: layer.attention,
            window_size: layer.window_size,
            compute_fraction: layer.compute_fraction,
            activation: mean_abs(&vector),
        });
    }

    if layer_summaries.is_empty() {
        normalize(&mut vector);
    }

    ReferenceForwardState {
        energy: mean_abs(&vector),
        vector,
        layer_summaries,
        kv_influence,
    }
}

fn reference_layers(context: ProductionKernelContext<'_>) -> Vec<TransformerLayerPlan> {
    let architecture = context.manifest.architecture;
    let layer_count = architecture.layer_count.max(1);
    let local_window = architecture.local_window_tokens.max(16);
    let native_window = context
        .manifest
        .metadata
        .native_context_window
        .max(local_window)
        .max(16);

    let mut plan = if context.request.transformer_plan.layers.len() == layer_count {
        context.request.transformer_plan.clone()
    } else {
        TransformerPlanner::new(layer_count, local_window).plan(
            context.request.profile,
            context.request.hierarchy,
            context.request.route_budget,
        )
    };

    for (index, layer) in plan.layers.iter_mut().enumerate() {
        layer.layer_index = index;
        layer.window_size = match layer.attention {
            AttentionKind::Global => layer.window_size.clamp(local_window, native_window),
            AttentionKind::LocalWindow | AttentionKind::ConvolutionalFusion => {
                layer.window_size.clamp(16, local_window)
            }
        };
    }

    plan.layers
}

fn apply_reference_imported_kv(
    vector: &mut [f32],
    imported_kv_blocks: &[RuntimeKvBlock],
    layer: &TransformerLayerPlan,
) -> f32 {
    if vector.is_empty() || imported_kv_blocks.is_empty() {
        return 0.0;
    }

    let mut applied = 0.0;
    for block in imported_kv_blocks
        .iter()
        .filter(|block| block.layer == layer.layer_index)
        .take(8)
    {
        let scale = match layer.attention {
            AttentionKind::Global => 0.034,
            AttentionKind::LocalWindow => 0.021,
            AttentionKind::ConvolutionalFusion => 0.014,
        } * layer.compute_fraction.clamp(0.1, 1.0);
        for (offset, value) in block.vector().iter().enumerate() {
            let index = (offset + block.head) % vector.len();
            vector[index] += value * scale;
            applied += value.abs() * scale;
        }
    }

    applied
}

fn apply_reference_layer(
    vector: &mut [f32],
    layer: &TransformerLayerPlan,
    attention_fraction: f32,
    weights_bytes: u64,
) {
    if vector.is_empty() {
        return;
    }

    let asset_phase = ((weights_bytes as usize % 97) + layer.layer_index + 1) as f32;
    match layer.attention {
        AttentionKind::Global => {
            let mean = vector.iter().sum::<f32>() / vector.len() as f32;
            let gain =
                0.042 + layer.compute_fraction * 0.052 + attention_fraction.clamp(0.0, 1.0) * 0.018;
            for (index, value) in vector.iter_mut().enumerate() {
                let positional = ((index + 1) as f32 * asset_phase).sin() * 0.0025;
                *value = *value * (1.0 - gain) + mean * gain + positional;
            }
        }
        AttentionKind::LocalWindow => {
            let previous = vector.to_vec();
            let radius =
                ((layer.window_size / 64).max(1)).min(previous.len().saturating_sub(1).max(1));
            let gain = 0.096 + layer.compute_fraction * 0.075;
            for index in 0..vector.len() {
                let left = previous[index.saturating_sub(radius)];
                let right = previous[(index + radius).min(previous.len() - 1)];
                let local = (left + previous[index] + right) / 3.0;
                vector[index] = previous[index] * (1.0 - gain) + local * gain;
            }
        }
        AttentionKind::ConvolutionalFusion => {
            let previous = vector.to_vec();
            let gain = 0.118 + layer.compute_fraction * 0.118;
            for index in 0..vector.len() {
                let prev = previous[index.saturating_sub(1)];
                let center = previous[index];
                let next = previous[(index + 1).min(previous.len() - 1)];
                let fused = prev * 0.25 + center * 0.50 + next * 0.25;
                let phase = ((index + 1) as f32 + asset_phase).cos() * 0.0018;
                vector[index] = center * (1.0 - gain) + fused * gain + phase;
            }
        }
    }
    normalize(vector);
}

fn count_reference_layers(layers: &[ReferenceLayerSummary]) -> TransformerPlanCounts {
    let mut counts = TransformerPlanCounts::default();
    for layer in layers {
        match layer.attention {
            AttentionKind::Global => counts.global += 1,
            AttentionKind::LocalWindow => counts.local += 1,
            AttentionKind::ConvolutionalFusion => counts.convolution += 1,
        }
    }
    counts
}

fn export_reference_kv(
    forward: &ReferenceForwardState,
    context: ProductionKernelContext<'_>,
) -> Vec<RuntimeKvBlock> {
    if forward.vector.is_empty() || !context.manifest.kv_policy.export_enabled {
        return Vec::new();
    }

    let architecture = context.manifest.architecture;
    let max_blocks = context.manifest.kv_policy.max_export_blocks.max(1);
    let block_count = forward
        .layer_summaries
        .len()
        .clamp(1, 4)
        .min(context.request.recursive_schedule.chunk_count().max(1) + 2)
        .min(max_blocks);
    let midpoint = (forward.vector.len() / 2).max(1);
    let key = forward
        .vector
        .iter()
        .copied()
        .take(midpoint)
        .collect::<Vec<_>>();
    let value = forward
        .vector
        .iter()
        .copied()
        .skip(midpoint)
        .take(midpoint)
        .collect::<Vec<_>>();
    let value = if value.is_empty() { key.clone() } else { value };

    (0..block_count)
        .map(|index| {
            let summary = forward.layer_summaries.get(index);
            let layer = summary.map(|summary| summary.layer_index).unwrap_or(index)
                % architecture.layer_count.max(1);
            let head = summary
                .map(|summary| {
                    let attention_offset = match summary.attention {
                        AttentionKind::Global => 0,
                        AttentionKind::LocalWindow => 1,
                        AttentionKind::ConvolutionalFusion => 2,
                    };
                    attention_offset + summary.window_size + index
                })
                .unwrap_or(index)
                % architecture.kv_heads.max(1);
            let compute_scale = summary
                .map(|summary| summary.compute_fraction + summary.activation)
                .unwrap_or(1.0)
                .clamp(0.25, 1.50);
            RuntimeKvBlock::new(
                layer,
                head,
                index,
                index + 1,
                scaled(&key, compute_scale + index as f32 * 0.02),
                scaled(&value, compute_scale - index as f32 * 0.015),
            )
        })
        .collect()
}

fn reference_answer(
    context: ProductionKernelContext<'_>,
    prompt_tokens: usize,
    forward: &ReferenceForwardState,
    counts: TransformerPlanCounts,
) -> String {
    format!(
        "Reference production Transformer kernel result for '{}'. The self-developed production boundary loaded manifest {}, tokenizer {}, local weights bytes {}, and selected adapter {} on device {}. It processed {} prompt tokens, {} imported KV blocks, and executed {} deterministic Rust Transformer-style layers with {:.3} state energy and {:.3} KV influence: {} global, {} local-window, and {} convolutional-fusion layers. Noiron keeps this reference kernel replaceable while the control plane exercises production manifest gates, device contracts, runtime KV exchange, reflection, process rewards, and durable local memory.",
        compact(&context.request.prompt, 96),
        context.manifest.metadata.model_id,
        context.manifest.metadata.tokenizer,
        context.assets.weights_bytes,
        context.device_gate.runtime_adapter_name(),
        context.device_gate.device.as_str(),
        prompt_tokens,
        context.imported_kv_blocks.len(),
        forward.layer_summaries.len(),
        forward.energy,
        forward.kv_influence,
        counts.global,
        counts.local,
        counts.convolution,
    )
}

fn mean_abs(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(|value| value.abs()).sum::<f32>() / values.len() as f32
}

fn estimated_entropy(token: &str, energy: f32, kv_influence: f32) -> f32 {
    let unique_chars = token
        .chars()
        .collect::<std::collections::HashSet<_>>()
        .len();
    (0.10 + unique_chars as f32 / 36.0 + energy * 0.08 + kv_influence.min(1.0) * 0.03)
        .clamp(0.05, 1.35)
}

fn scaled(values: &[f32], scale: f32) -> Vec<f32> {
    values.iter().map(|value| value * scale).collect()
}

fn asset_len(label: &str, path: &Path) -> Result<u64, RuntimeError> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| {
            RuntimeError::new(format!(
                "failed to read {label} asset metadata at {}: {error}",
                path.display()
            ))
        })
}

fn production_tokenize(text: &str) -> Vec<String> {
    let tokens = text
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| ch.is_ascii_punctuation())
                .to_owned()
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if !tokens.is_empty() {
        return tokens;
    }

    text.chars()
        .filter(|ch| !ch.is_whitespace())
        .map(|ch| ch.to_string())
        .collect()
}

fn embed_tokens(tokens: &[RuntimeTokenId], dimensions: usize) -> Vec<f32> {
    let dimensions = dimensions.max(1);
    let mut vector = vec![0.0; dimensions];

    for (position, token) in tokens.iter().enumerate() {
        let hash = stable_hash(&format!("{}:{}", token.id, token.text));
        for offset in 0..4 {
            let index = ((hash >> (offset * 11)) as usize) % dimensions;
            vector[index] += 1.0 / (position as f32 + offset as f32 + 1.0);
        }
    }

    normalize(&mut vector);
    vector
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector {
            *value /= norm;
        }
    }
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{DeviceClass, HardwareAllocator, HardwareSnapshot, RuntimeAdapterHint};
    use crate::hierarchy::HierarchyWeights;
    use crate::local_runtime::LocalTransformerRuntime;
    use crate::runtime::ModelRuntime;
    use crate::runtime_manifest::{
        RuntimeAssetPaths, RuntimeKvPolicy, TransformerRuntimeArchitecture,
    };
    use std::fs::{self, File};
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn production_runtime_accepts_manifest_assets_and_device_contract() {
        let (asset_dir, weights, tokenizer, config) = create_assets("production-runtime-assets");
        let manifest = production_manifest(&weights, &tokenizer)
            .with_assets(
                RuntimeAssetPaths::new()
                    .with_weights(&weights)
                    .with_tokenizer(&tokenizer)
                    .with_config(&config),
            )
            .with_adapter_hints(vec![
                RuntimeAdapterHint::PortableRust,
                RuntimeAdapterHint::CpuSimd,
            ]);
        let plan = cpu_plan();

        let runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &plan).unwrap();

        assert_eq!(runtime.metadata().model_id, "noiron-production-transformer");
        assert_eq!(runtime.architecture().layer_count, 6);
        assert!(runtime.device_gate().passed());
        assert_eq!(runtime.assets().weights_bytes, 7);
        assert_eq!(runtime.assets().tokenizer_bytes, 9);
        assert_eq!(runtime.assets().config_bytes, Some(6));
        assert!(runtime.runtime_device_contract().contains("device=cpu"));
        assert!(runtime.selected_adapter().is_some());
        assert!(runtime.summary_line().contains("kernel=not-connected"));
        assert!(!runtime.kernel_connected());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_exposes_bootstrap_tokens_and_embeddings() {
        let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-embed");
        let manifest = production_manifest(&weights, &tokenizer);
        let runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap();

        let tokens = runtime.tokenize("Rust Noiron runtime").unwrap();
        let embedding = runtime.embed(&tokens).unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(embedding.dimensions, 64);
        assert!(embedding.values.iter().any(|value| *value > 0.0));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_rejects_missing_assets() {
        let manifest = RuntimeManifest::self_developed("missing-production", "tokenizer", 4096, 64)
            .with_architecture(TransformerRuntimeArchitecture::new(6, 64, 4, 2, 1024));

        let error = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
            .unwrap_err();

        assert!(error.message().contains("weights asset path is required"));
        assert!(error.message().contains("tokenizer asset path is required"));
    }

    #[test]
    fn production_runtime_rejects_device_adapter_mismatch() {
        let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-mismatch");
        let manifest = production_manifest(&weights, &tokenizer)
            .with_adapter_hints(vec![RuntimeAdapterHint::Cuda]);

        let error = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
            .unwrap_err();

        assert!(error.message().contains("device gate rejected"));
        assert!(error.message().contains("no adapter intersection"));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_imports_kv_with_manifest_and_device_limits() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-import-kv");
        let manifest = production_manifest(&weights, &tokenizer).with_kv_policy(RuntimeKvPolicy {
            import_enabled: true,
            export_enabled: true,
            max_import_blocks: 1,
            max_export_blocks: 2,
        });
        let mut plan = cpu_plan();
        plan.execution.kv_prefetch_blocks = 1;
        let mut runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &plan).unwrap();
        let blocks = vec![
            RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2]),
            RuntimeKvBlock::new(0, 1, 1, 2, vec![0.3], vec![0.4]),
        ];

        let imported = runtime.import_kv(&blocks).unwrap();
        let exported = runtime.export_kv().unwrap();

        assert_eq!(imported, 1);
        assert_eq!(runtime.imported_kv_blocks().len(), 1);
        assert!(exported.is_empty());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_rejects_invalid_imported_kv() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-invalid-import-kv");
        let manifest = production_manifest(&weights, &tokenizer);
        let mut runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap();

        runtime
            .import_kv(&[RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2])])
            .unwrap();
        assert_eq!(runtime.imported_kv_blocks().len(), 1);

        let cases = vec![
            (
                "layer",
                RuntimeKvBlock::new(6, 0, 0, 1, vec![0.1], vec![0.2]),
                "layer 6 exceeds manifest layer_count 6",
            ),
            (
                "head",
                RuntimeKvBlock::new(0, 2, 0, 1, vec![0.1], vec![0.2]),
                "head 2 exceeds manifest kv_heads 2",
            ),
            (
                "range",
                RuntimeKvBlock::new(0, 0, 2, 2, vec![0.1], vec![0.2]),
                "token range 2..2 is empty or reversed",
            ),
            (
                "dimension",
                RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2, 0.3]),
                "key/value dimensions differ",
            ),
            (
                "finite",
                RuntimeKvBlock::new(0, 0, 0, 1, vec![f32::NAN], vec![0.2]),
                "imported key contains non-finite value",
            ),
        ];

        for (label, block, expected) in cases {
            let mut invalid_runtime = runtime.clone();

            let error = invalid_runtime.import_kv(&[block]).unwrap_err();

            assert!(
                error.message().contains("invalid imported KV block 0"),
                "{label}: {}",
                error.message()
            );
            assert!(
                error.message().contains(expected),
                "{label}: {}",
                error.message()
            );
            assert!(invalid_runtime.imported_kv_blocks().is_empty(), "{label}");
        }

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_generate_errors_until_forward_kernel_is_connected() {
        let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-generate");
        let manifest = production_manifest(&weights, &tokenizer);
        let mut runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap();

        let error = runtime.generate(sample_request()).unwrap_err();

        assert!(error.message().contains("kernel is not connected"));
        assert!(error.message().contains("noiron-production-transformer"));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_can_generate_through_attached_forward_kernel() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-attached-kernel");
        let manifest = production_manifest(&weights, &tokenizer);
        let mut runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
                .unwrap()
                .with_kernel(MockForwardKernel);
        let blocks = vec![RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2])];
        runtime.import_kv(&blocks).unwrap();

        let response = runtime.generate(sample_request()).unwrap();
        let exported = runtime.export_kv().unwrap();

        assert!(runtime.kernel_connected());
        assert!(runtime.summary_line().contains("kernel=connected"));
        assert!(response.answer.contains("kernel answer"));
        assert_eq!(response.tokens.len(), 1);
        assert_eq!(response.trace[0].label, "production_kernel");
        assert_eq!(
            response.diagnostics.model_id.as_deref(),
            Some("noiron-production-transformer")
        );
        assert_eq!(
            response.diagnostics.selected_adapter.as_deref(),
            Some("portable-rust")
        );
        assert_eq!(response.diagnostics.imported_kv_blocks, 1);
        assert_eq!(response.diagnostics.exported_kv_blocks, 1);
        assert_eq!(exported.len(), 1);
        assert_eq!(runtime.exported_kv_blocks().len(), 1);

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn reference_production_kernel_generates_diagnostics_and_kv() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-reference-kernel");
        let manifest = production_manifest(&weights, &tokenizer);
        let mut runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
                .unwrap()
                .with_kernel(ReferenceProductionForwardKernel::new());
        runtime
            .import_kv(&[RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2])])
            .unwrap();

        let response = runtime.generate(sample_request()).unwrap();
        let exported = runtime.export_kv().unwrap();

        assert!(
            response
                .answer
                .contains("Reference production Transformer kernel result")
        );
        assert!(!response.tokens.is_empty());
        assert!(
            response
                .trace
                .iter()
                .any(|step| step.label == "reference_production_kernel")
        );
        assert_eq!(
            response.diagnostics.model_id.as_deref(),
            Some("noiron-production-transformer")
        );
        assert_eq!(
            response.diagnostics.selected_adapter.as_deref(),
            Some("portable-rust")
        );
        assert_eq!(response.diagnostics.layer_count, 6);
        assert!(response.diagnostics.forward_energy.unwrap() > 0.0);
        assert!(response.diagnostics.kv_influence.unwrap() > 0.0);
        assert_eq!(response.diagnostics.imported_kv_blocks, 1);
        assert!(!exported.is_empty());
        assert!(exported.iter().all(|block| block.layer < 6));
        assert!(exported.iter().all(|block| block.head < 2));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn reference_production_kernel_passes_conformance_gate() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-conformance-reference");
        let manifest = production_manifest(&weights, &tokenizer);
        let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
            .unwrap()
            .with_kernel(ReferenceProductionForwardKernel::new());

        let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

        assert!(report.passed, "{report:?}");
        assert!(report.kernel_connected);
        assert!(report.token_count > 0);
        assert!(report.trace_steps > 0);
        assert!(report.imported_kv_blocks > 0);
        assert!(report.exported_kv_blocks > 0);
        assert!(report.forward_energy.unwrap() > 0.0);
        assert!(report.kv_influence.unwrap() >= 0.0);
        assert!(report.summary_line().contains("passed=true"));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn model_runtime_forward_kernel_wraps_local_runtime_for_production_boundary() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-local-kernel");
        let manifest = production_manifest(&weights, &tokenizer);
        let local_runtime = LocalTransformerRuntime::with_manifest(manifest.clone());
        let mut runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
                .unwrap()
                .with_kernel(ModelRuntimeForwardKernel::new(local_runtime));
        runtime
            .import_kv(&[RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2])])
            .unwrap();

        let response = runtime.generate(sample_request()).unwrap();
        let exported = runtime.export_kv().unwrap();

        assert!(response.answer.contains("Local Transformer runtime result"));
        assert_eq!(
            response.diagnostics.model_id.as_deref(),
            Some("noiron-production-transformer")
        );
        assert_eq!(
            response.diagnostics.selected_adapter.as_deref(),
            Some("portable-rust")
        );
        assert_eq!(response.diagnostics.layer_count, 6);
        assert_eq!(response.diagnostics.hidden_size, 64);
        assert_eq!(response.diagnostics.imported_kv_blocks, 1);
        assert!(!response.tokens.is_empty());
        assert!(!response.trace.is_empty());
        assert!(!exported.is_empty());
        assert!(exported.iter().all(|block| block.layer < 6));
        assert!(exported.iter().all(|block| block.head < 2));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn local_model_runtime_kernel_passes_conformance_gate() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-local-kernel-conformance");
        let manifest = production_manifest(&weights, &tokenizer);
        let local_runtime = LocalTransformerRuntime::with_manifest(manifest.clone());
        let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
            .unwrap()
            .with_kernel(ModelRuntimeForwardKernel::new(local_runtime));

        let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

        assert!(report.passed, "{report:?}");
        assert_eq!(report.model_id, "noiron-production-transformer");
        assert_eq!(report.selected_adapter, "portable-rust");
        assert!(report.imported_kv_blocks > 0);
        assert!(report.exported_kv_blocks > 0);
        assert!(report.forward_energy.unwrap() > 0.0);

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_kernel_conformance_gate_fails_when_kernel_is_missing() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-conformance-missing");
        let manifest = production_manifest(&weights, &tokenizer);
        let runtime =
            ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap();

        let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

        assert!(!report.passed);
        assert!(!report.kernel_connected);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("production forward kernel is not connected"))
        );

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_kernel_conformance_gate_fails_malformed_kernel_output() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-conformance-malformed");
        let manifest = production_manifest(&weights, &tokenizer);
        let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
            .unwrap()
            .with_kernel(MalformedForwardKernel);

        let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

        assert!(!report.passed);
        assert_eq!(report.token_count, 0);
        assert_eq!(report.trace_steps, 0);
        assert!(report.failures.iter().any(|failure| {
            failure.contains("kernel did not return runtime token uncertainty records")
        }));
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("kernel did not return reasoning trace steps"))
        );
        assert!(report.failures.iter().any(|failure| {
            failure.contains("kernel did not report positive finite forward_energy")
        }));
        assert!(report.failures.iter().any(|failure| {
            failure.contains("kernel did not report finite non-negative kv_influence")
        }));
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("kernel exported no KV blocks"))
        );

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_rejects_invalid_kernel_exported_kv() {
        let (asset_dir, weights, tokenizer, _config) =
            create_assets("production-runtime-invalid-kernel-kv");
        let manifest = production_manifest(&weights, &tokenizer);
        let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
            .unwrap()
            .with_kernel(MockForwardKernel);
        let mut runtime = runtime;

        runtime.generate(sample_request()).unwrap();
        assert_eq!(runtime.exported_kv_blocks().len(), 1);

        let cases = vec![
            (
                "layer",
                RuntimeKvBlock::new(6, 0, 0, 1, vec![0.1], vec![0.2]),
                "layer 6 exceeds manifest layer_count 6",
            ),
            (
                "head",
                RuntimeKvBlock::new(0, 2, 0, 1, vec![0.1], vec![0.2]),
                "head 2 exceeds manifest kv_heads 2",
            ),
            (
                "range",
                RuntimeKvBlock::new(0, 0, 2, 2, vec![0.1], vec![0.2]),
                "token range 2..2 is empty or reversed",
            ),
            (
                "dimension",
                RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2, 0.3]),
                "key/value dimensions differ",
            ),
            (
                "finite",
                RuntimeKvBlock::new(0, 0, 0, 1, vec![f32::NAN], vec![0.2]),
                "key contains non-finite value",
            ),
        ];

        for (label, block, expected) in cases {
            let mut invalid_runtime = runtime.clone().with_kernel(InvalidExportKernel { block });

            let error = invalid_runtime.generate(sample_request()).unwrap_err();

            assert!(
                error.message().contains("invalid exported KV block 0"),
                "{label}: {}",
                error.message()
            );
            assert!(
                error.message().contains(expected),
                "{label}: {}",
                error.message()
            );
            assert!(invalid_runtime.exported_kv_blocks().is_empty(), "{label}");
            assert!(invalid_runtime.export_kv().unwrap().is_empty(), "{label}");
        }

        fs::remove_dir_all(asset_dir).unwrap();
    }

    fn production_manifest(weights: &Path, tokenizer: &Path) -> RuntimeManifest {
        RuntimeManifest::self_developed(
            "noiron-production-transformer",
            "noiron-production-tokenizer",
            4096,
            64,
        )
        .with_architecture(TransformerRuntimeArchitecture::new(6, 64, 4, 2, 1024))
        .with_supported_devices(vec![DeviceClass::CpuOnly])
        .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust])
        .with_assets(
            RuntimeAssetPaths::new()
                .with_weights(weights)
                .with_tokenizer(tokenizer),
        )
    }

    fn cpu_plan() -> HardwarePlan {
        HardwareAllocator::new().plan(
            HardwareSnapshot::new(DeviceClass::CpuOnly, 0.20, 0.10, 0.30, 0.10),
            crate::hierarchy::TaskProfile::General,
            512,
            HierarchyWeights::default(),
        )
    }

    fn sample_request() -> RuntimeRequest {
        RuntimeRequest {
            prompt: "connect production runtime".to_owned(),
            profile: crate::hierarchy::TaskProfile::General,
            runtime_metadata: RuntimeMetadata::new(
                "noiron-production-transformer",
                "noiron-production-tokenizer",
                4096,
                64,
            ),
            runtime_architecture: TransformerRuntimeArchitecture::new(6, 64, 4, 2, 1024),
            memory_hints: Vec::new(),
            infini_memory_hints: Vec::new(),
            experience_hints: Vec::new(),
            runtime_adapter_observations: Vec::new(),
            toolsmith_plan: crate::toolsmith::ToolsmithPlan::default(),
            agent_team_plan: crate::agent_team::AgentTeamPlan::default(),
            route_budget: crate::router::RouteBudget {
                threshold: 0.5,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::default(),
            transformer_plan: crate::transformer::TransformerRefactorPlan::default(),
            recursive_schedule: crate::recursive_scheduler::RecursiveSchedule::default(),
            hardware_plan: cpu_plan(),
            max_tokens: 32,
        }
    }

    fn create_assets(name: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let asset_dir = temp_asset_dir(name);
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        let config = asset_dir.join("config.noiron");
        write_asset(&weights, b"weights");
        write_asset(&tokenizer, b"tokenizer");
        write_asset(&config, b"config");
        (asset_dir, weights, tokenizer, config)
    }

    fn write_asset(path: &Path, bytes: &[u8]) {
        let mut file = File::create(path).unwrap();
        file.write_all(bytes).unwrap();
    }

    fn temp_asset_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{unique}"))
    }

    #[derive(Debug, Clone)]
    struct MockForwardKernel;

    impl ProductionForwardKernel for MockForwardKernel {
        fn generate(
            &self,
            context: ProductionKernelContext<'_>,
        ) -> Result<ProductionKernelOutput, RuntimeError> {
            Ok(ProductionKernelOutput::new(format!(
                "kernel answer for {} with {} imported KV blocks",
                context.manifest.metadata.model_id,
                context.imported_kv_blocks.len()
            ))
            .with_tokens(vec![RuntimeToken {
                text: "kernel".to_owned(),
                logprob: Some(-0.2),
                entropy: Some(0.3),
            }])
            .with_trace(vec![ReasoningStep::new(
                "production_kernel",
                context.device_gate.runtime_device_contract.clone(),
                0.88,
            )])
            .with_diagnostics(RuntimeDiagnostics {
                forward_energy: Some(0.42),
                kv_influence: Some(0.25),
                ..RuntimeDiagnostics::default()
            })
            .with_exported_kv_blocks(vec![RuntimeKvBlock::new(1, 0, 0, 1, vec![0.3], vec![0.4])]))
        }
    }

    #[derive(Debug, Clone)]
    struct InvalidExportKernel {
        block: RuntimeKvBlock,
    }

    impl ProductionForwardKernel for InvalidExportKernel {
        fn generate(
            &self,
            _context: ProductionKernelContext<'_>,
        ) -> Result<ProductionKernelOutput, RuntimeError> {
            Ok(ProductionKernelOutput::new("invalid kernel export")
                .with_exported_kv_blocks(vec![self.block.clone()]))
        }
    }

    #[derive(Debug, Clone)]
    struct MalformedForwardKernel;

    impl ProductionForwardKernel for MalformedForwardKernel {
        fn generate(
            &self,
            _context: ProductionKernelContext<'_>,
        ) -> Result<ProductionKernelOutput, RuntimeError> {
            Ok(ProductionKernelOutput::new("malformed kernel output"))
        }
    }
}
