use crate::hardware::HardwarePlan;
use crate::hierarchy::TaskProfile;
use crate::kv_exchange::{RuntimeKvBlock, RuntimeKvBlockValidationError};
use crate::reflection::{ReasoningStep, RuntimeDiagnostics};
use crate::runtime_manifest::TransformerRuntimeArchitecture;
use crate::tenant_scope::{
    TenantAccessKind, TenantIsolationGate, TenantResourceLane, TenantScope, TenantScopedKey,
};

use super::types::{
    ModelRuntime, RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeResponse,
    RuntimeToken, RuntimeTokenId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkedKvCacheMode {
    NoCache,
    ChunkedCache,
    GenomeFiltered,
}

impl ChunkedKvCacheMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoCache => "no_cache",
            Self::ChunkedCache => "chunked_cache",
            Self::GenomeFiltered => "genome_filtered",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkedKvHookDecision {
    Include,
    Skip,
    Reject,
}

impl ChunkedKvHookDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Skip => "skip",
            Self::Reject => "reject",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkedKvSegment {
    pub segment_id: String,
    pub cache_ref: TenantScopedKey,
    pub token_start: usize,
    pub token_end: usize,
    pub attention_threshold: f32,
    pub genome_gate_passed: bool,
    pub kv_blocks: Vec<RuntimeKvBlock>,
}

impl ChunkedKvSegment {
    pub fn new(
        segment_id: impl AsRef<str>,
        cache_ref: TenantScopedKey,
        token_start: usize,
        token_end: usize,
    ) -> Self {
        Self {
            segment_id: sanitize_id(segment_id.as_ref(), "segment"),
            cache_ref,
            token_start,
            token_end: token_end.max(token_start.saturating_add(1)),
            attention_threshold: 0.50,
            genome_gate_passed: true,
            kv_blocks: Vec::new(),
        }
    }

    pub fn with_attention_threshold(mut self, attention_threshold: f32) -> Self {
        self.attention_threshold = clamp_unit(attention_threshold);
        self
    }

    pub fn with_genome_gate(mut self, passed: bool) -> Self {
        self.genome_gate_passed = passed;
        self
    }

    pub fn with_kv_blocks(mut self, kv_blocks: Vec<RuntimeKvBlock>) -> Self {
        self.kv_blocks = kv_blocks;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkedKvHookRecord {
    pub trace_id: String,
    pub segment_id: String,
    pub cache_ref_digest: String,
    pub token_start: usize,
    pub token_end: usize,
    pub decision: ChunkedKvHookDecision,
    pub attention_threshold: f32,
    pub kv_blocks: usize,
    pub reason: String,
    pub tenant_isolation_allowed: bool,
    pub genome_gate_passed: bool,
    pub redacted: bool,
}

impl ChunkedKvHookRecord {
    pub fn summary_line(&self) -> String {
        format!(
            "chunked_kv_hook trace_id={} segment={} cache_ref={} token_start={} token_end={} decision={} attention_threshold={:.3} kv_blocks={} tenant_allowed={} genome_gate={} reason={} redacted={}",
            self.trace_id,
            self.segment_id,
            self.cache_ref_digest,
            self.token_start,
            self.token_end,
            self.decision.as_str(),
            self.attention_threshold,
            self.kv_blocks,
            self.tenant_isolation_allowed,
            self.genome_gate_passed,
            self.reason,
            self.redacted
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustNativeAdapterRequest {
    pub prompt: String,
    pub profile: TaskProfile,
    pub trace_id: String,
    pub tenant_scope: TenantScope,
    pub device_execution: RustNativeAdapterDeviceExecution,
    pub runtime_metadata: RuntimeMetadata,
    pub runtime_architecture: TransformerRuntimeArchitecture,
    pub max_tokens: usize,
    pub cache_mode: ChunkedKvCacheMode,
    pub max_attention_threshold: f32,
    pub segments: Vec<ChunkedKvSegment>,
    pub gate_summaries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustNativeAdapterDeviceExecution {
    pub device_profile: String,
    pub primary_lane: String,
    pub fallback_lane: String,
    pub memory_mode: String,
    pub hot_kv_precision_bits: u8,
    pub cold_kv_precision_bits: u8,
}

impl RustNativeAdapterDeviceExecution {
    pub fn from_hardware_plan(plan: &HardwarePlan) -> Self {
        Self {
            device_profile: plan.device.as_str().to_owned(),
            primary_lane: plan.execution.primary_lane.as_str().to_owned(),
            fallback_lane: plan.execution.fallback_lane.as_str().to_owned(),
            memory_mode: plan.execution.memory_mode.as_str().to_owned(),
            hot_kv_precision_bits: plan.execution.hot_kv_precision_bits,
            cold_kv_precision_bits: plan.execution.cold_kv_precision_bits,
        }
    }
}

impl Default for RustNativeAdapterDeviceExecution {
    fn default() -> Self {
        Self::from_hardware_plan(&HardwarePlan::default())
    }
}

impl RustNativeAdapterRequest {
    pub fn new(
        prompt: impl Into<String>,
        profile: TaskProfile,
        trace_id: impl AsRef<str>,
        tenant_scope: TenantScope,
    ) -> Self {
        let runtime_metadata =
            RuntimeMetadata::new("rust-native-mock", "noiron-wordpiece", 4096, 8)
                .with_kv_exchange(true, true)
                .with_kv_limits(8, 4);
        let runtime_architecture = TransformerRuntimeArchitecture::new(4, 8, 4, 2, 1024);
        Self {
            prompt: prompt.into(),
            profile,
            trace_id: sanitize_id(trace_id.as_ref(), "trace"),
            tenant_scope,
            device_execution: RustNativeAdapterDeviceExecution::default(),
            runtime_metadata,
            runtime_architecture,
            max_tokens: 128,
            cache_mode: ChunkedKvCacheMode::ChunkedCache,
            max_attention_threshold: 0.72,
            segments: Vec::new(),
            gate_summaries: Vec::new(),
        }
    }

    pub fn from_runtime_request(
        request: RuntimeRequest,
        trace_id: impl AsRef<str>,
        tenant_scope: TenantScope,
        cache_mode: ChunkedKvCacheMode,
    ) -> Self {
        let RuntimeRequest {
            prompt,
            profile,
            runtime_metadata,
            runtime_architecture,
            hardware_plan,
            max_tokens,
            ..
        } = request;
        let device_execution = RustNativeAdapterDeviceExecution::from_hardware_plan(&hardware_plan);
        Self {
            prompt,
            profile,
            trace_id: sanitize_id(trace_id.as_ref(), "trace"),
            tenant_scope,
            device_execution,
            runtime_metadata,
            runtime_architecture,
            max_tokens: max_tokens.max(1),
            cache_mode,
            max_attention_threshold: 0.72,
            segments: Vec::new(),
            gate_summaries: Vec::new(),
        }
    }

    pub fn with_segments(mut self, segments: Vec<ChunkedKvSegment>) -> Self {
        self.segments = segments;
        self
    }

    pub fn with_cache_mode(mut self, cache_mode: ChunkedKvCacheMode) -> Self {
        self.cache_mode = cache_mode;
        self
    }

    pub fn with_gate_summary(mut self, summary: impl Into<String>) -> Self {
        self.gate_summaries.push(sanitize_summary(&summary.into()));
        self
    }

    pub fn with_max_attention_threshold(mut self, max_attention_threshold: f32) -> Self {
        self.max_attention_threshold = clamp_unit(max_attention_threshold);
        self
    }
}

#[derive(Debug, Clone)]
pub struct RustNativeAdapterReport {
    pub trace_id: String,
    pub cache_mode: ChunkedKvCacheMode,
    pub response: RuntimeResponse,
    pub hook_records: Vec<ChunkedKvHookRecord>,
    pub stream_tokens: Vec<RuntimeToken>,
    pub imported_kv_blocks: usize,
    pub exported_kv_blocks: usize,
    pub gate_summary_digest: String,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl RustNativeAdapterReport {
    pub fn included_segments(&self) -> usize {
        chunked_kv_hook_decision_counts(&self.hook_records).0
    }

    pub fn skipped_segments(&self) -> usize {
        chunked_kv_hook_decision_counts(&self.hook_records).1
    }

    pub fn rejected_segments(&self) -> usize {
        chunked_kv_hook_decision_counts(&self.hook_records).2
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn hook_summaries(&self) -> Vec<String> {
        self.hook_records
            .iter()
            .map(ChunkedKvHookRecord::summary_line)
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "rust_native_adapter trace_id={} mode={} included={} skipped={} rejected={} imported_kv_blocks={} exported_kv_blocks={} stream_tokens={} gate_summary={} read_only={} write_allowed={} applied={}",
            self.trace_id,
            self.cache_mode.as_str(),
            self.included_segments(),
            self.skipped_segments(),
            self.rejected_segments(),
            self.imported_kv_blocks,
            self.exported_kv_blocks,
            self.stream_tokens.len(),
            self.gate_summary_digest,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }

    pub fn openai_stream_events(&self) -> Vec<RustNativeAdapterStreamEvent> {
        let mut events = Vec::with_capacity(self.stream_tokens.len() + 3);
        events.push(RustNativeAdapterStreamEvent::new(
            "response.created",
            format!(
                "{{\"id\":{},\"object\":\"chat.completion.chunk\",\"model\":{},\"cache_mode\":{},\"gate_summary\":{},\"read_only\":{},\"write_allowed\":{},\"applied\":{}}}",
                json_string(&self.trace_id),
                json_string(
                    self.response
                        .diagnostics
                        .model_id
                        .as_deref()
                        .unwrap_or("rust-native")
                ),
                json_string(self.cache_mode.as_str()),
                json_string(&self.gate_summary_digest),
                self.read_only,
                self.write_allowed,
                self.applied
            ),
        ));
        for (index, token) in self.stream_tokens.iter().enumerate() {
            events.push(RustNativeAdapterStreamEvent::new(
                "response.output_text.delta",
                format!(
                    "{{\"id\":{},\"index\":{},\"delta\":{}}}",
                    json_string(&self.trace_id),
                    index,
                    json_string(&token.text)
                ),
            ));
        }
        events.push(RustNativeAdapterStreamEvent::new(
            "response.completed",
            format!(
                "{{\"id\":{},\"imported_kv_blocks\":{},\"exported_kv_blocks\":{},\"included_segments\":{},\"skipped_segments\":{},\"rejected_segments\":{},\"gate_summary\":{},\"read_only\":{},\"write_allowed\":{},\"applied\":{}}}",
                json_string(&self.trace_id),
                self.imported_kv_blocks,
                self.exported_kv_blocks,
                self.included_segments(),
                self.skipped_segments(),
                self.rejected_segments(),
                json_string(&self.gate_summary_digest),
                self.read_only,
                self.write_allowed,
                self.applied
            ),
        ));
        events.push(RustNativeAdapterStreamEvent::new("done", "[DONE]"));
        events
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustNativeAdapterStreamEvent {
    pub event: String,
    pub data: String,
}

impl RustNativeAdapterStreamEvent {
    pub fn new(event: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            event: event.into(),
            data: data.into(),
        }
    }

    pub fn sse_line(&self) -> String {
        format!("event: {}\ndata: {}\n", self.event, self.data)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustNativeAdapterModeComparison {
    pub cache_mode: ChunkedKvCacheMode,
    pub included_segments: usize,
    pub skipped_segments: usize,
    pub rejected_segments: usize,
    pub imported_kv_blocks: usize,
    pub exported_kv_blocks: usize,
    pub stream_tokens: usize,
    pub gate_summary_digest: String,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl RustNativeAdapterModeComparison {
    pub fn from_report(report: &RustNativeAdapterReport) -> Self {
        Self {
            cache_mode: report.cache_mode,
            included_segments: report.included_segments(),
            skipped_segments: report.skipped_segments(),
            rejected_segments: report.rejected_segments(),
            imported_kv_blocks: report.imported_kv_blocks,
            exported_kv_blocks: report.exported_kv_blocks,
            stream_tokens: report.stream_tokens.len(),
            gate_summary_digest: report.gate_summary_digest.clone(),
            read_only: report.read_only,
            write_allowed: report.write_allowed,
            applied: report.applied,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "mode={} included={} skipped={} rejected={} imported_kv_blocks={} exported_kv_blocks={} stream_tokens={} gate_summary={} read_only={} write_allowed={} applied={}",
            self.cache_mode.as_str(),
            self.included_segments,
            self.skipped_segments,
            self.rejected_segments,
            self.imported_kv_blocks,
            self.exported_kv_blocks,
            self.stream_tokens,
            self.gate_summary_digest,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustNativeAdapterComparisonReport {
    pub trace_id: String,
    pub modes: Vec<RustNativeAdapterModeComparison>,
}

impl RustNativeAdapterComparisonReport {
    pub fn mode(&self, mode: ChunkedKvCacheMode) -> Option<&RustNativeAdapterModeComparison> {
        self.modes
            .iter()
            .find(|comparison| comparison.cache_mode == mode)
    }

    pub fn has_required_cache_modes(&self) -> bool {
        [
            ChunkedKvCacheMode::NoCache,
            ChunkedKvCacheMode::ChunkedCache,
            ChunkedKvCacheMode::GenomeFiltered,
        ]
        .into_iter()
        .all(|mode| self.mode(mode).is_some())
    }

    pub fn imported_delta_vs_no_cache(&self, mode: ChunkedKvCacheMode) -> Option<isize> {
        let no_cache = self.mode(ChunkedKvCacheMode::NoCache)?;
        let candidate = self.mode(mode)?;
        Some(candidate.imported_kv_blocks as isize - no_cache.imported_kv_blocks as isize)
    }

    pub fn summary_line(&self) -> String {
        let modes = self
            .modes
            .iter()
            .map(RustNativeAdapterModeComparison::summary_line)
            .collect::<Vec<_>>()
            .join(" | ");
        format!(
            "rust_native_adapter_benchmark trace_id={} cache_modes={} has_required_cache_modes={} {}",
            self.trace_id,
            self.modes.len(),
            self.has_required_cache_modes(),
            modes
        )
    }
}

pub trait RustNativeInferenceAdapter {
    fn metadata(&self) -> RuntimeMetadata;

    fn architecture(&self) -> TransformerRuntimeArchitecture;

    fn embed_text(&self, text: &str) -> Result<RuntimeEmbedding, RuntimeError>;

    fn import_cache_blocks(
        &mut self,
        cache_mode: ChunkedKvCacheMode,
        blocks: &[RuntimeKvBlock],
    ) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        if cache_mode == ChunkedKvCacheMode::NoCache {
            Ok(Vec::new())
        } else {
            Ok(blocks.to_vec())
        }
    }

    fn export_cache_blocks(
        &mut self,
        report: &RustNativeAdapterReport,
    ) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(report.response.exported_kv_blocks.clone())
    }

    fn generate(
        &mut self,
        request: RustNativeAdapterRequest,
    ) -> Result<RustNativeAdapterReport, RuntimeError> {
        let mut ignore_token: fn(&RuntimeToken) -> Result<(), RuntimeError> = ignore_runtime_token;
        self.generate_stream(request, &mut ignore_token)
    }

    fn generate_stream(
        &mut self,
        request: RustNativeAdapterRequest,
        on_token: &mut dyn FnMut(&RuntimeToken) -> Result<(), RuntimeError>,
    ) -> Result<RustNativeAdapterReport, RuntimeError>;

    fn compare_cache_modes(
        &mut self,
        request: RustNativeAdapterRequest,
        modes: &[ChunkedKvCacheMode],
    ) -> Result<RustNativeAdapterComparisonReport, RuntimeError> {
        let trace_id = request.trace_id.clone();
        let modes = if modes.is_empty() {
            vec![
                ChunkedKvCacheMode::NoCache,
                ChunkedKvCacheMode::ChunkedCache,
                ChunkedKvCacheMode::GenomeFiltered,
            ]
        } else {
            modes.to_vec()
        };
        let mut comparisons = Vec::with_capacity(modes.len());
        for mode in modes {
            let report = self.generate(request.clone().with_cache_mode(mode))?;
            comparisons.push(RustNativeAdapterModeComparison::from_report(&report));
        }
        Ok(RustNativeAdapterComparisonReport {
            trace_id,
            modes: comparisons,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MockRustNativeAdapter {
    metadata: RuntimeMetadata,
    architecture: TransformerRuntimeArchitecture,
}

impl Default for MockRustNativeAdapter {
    fn default() -> Self {
        Self {
            metadata: RuntimeMetadata::new("rust-native-mock", "noiron-wordpiece", 4096, 8)
                .with_kv_exchange(true, true)
                .with_kv_limits(8, 4)
                .with_kv_precision(8, 4),
            architecture: TransformerRuntimeArchitecture::new(4, 8, 4, 2, 1024),
        }
    }
}

impl MockRustNativeAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct RustNativeModelRuntime<A> {
    adapter: A,
    tenant_scope: TenantScope,
    cache_mode: ChunkedKvCacheMode,
    pending_imported_kv_blocks: Vec<RuntimeKvBlock>,
    last_exported_kv_blocks: Vec<RuntimeKvBlock>,
    last_report: Option<RustNativeAdapterReport>,
}

impl<A: RustNativeInferenceAdapter> RustNativeModelRuntime<A> {
    pub fn new(adapter: A) -> Self {
        Self {
            adapter,
            tenant_scope: TenantScope::local_single_user(),
            cache_mode: ChunkedKvCacheMode::ChunkedCache,
            pending_imported_kv_blocks: Vec::new(),
            last_exported_kv_blocks: Vec::new(),
            last_report: None,
        }
    }

    pub fn with_tenant_scope(mut self, tenant_scope: TenantScope) -> Self {
        self.tenant_scope = tenant_scope;
        self
    }

    pub fn with_cache_mode(mut self, cache_mode: ChunkedKvCacheMode) -> Self {
        self.cache_mode = cache_mode;
        self
    }

    pub fn adapter(&self) -> &A {
        &self.adapter
    }

    pub fn adapter_mut(&mut self) -> &mut A {
        &mut self.adapter
    }

    pub fn last_report(&self) -> Option<&RustNativeAdapterReport> {
        self.last_report.as_ref()
    }

    pub fn into_inner(self) -> A {
        self.adapter
    }
}

impl<A: RustNativeInferenceAdapter> ModelRuntime for RustNativeModelRuntime<A> {
    fn metadata(&self) -> RuntimeMetadata {
        self.adapter.metadata()
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        self.adapter.architecture()
    }

    fn embed(&self, tokens: &[RuntimeTokenId]) -> Result<RuntimeEmbedding, RuntimeError> {
        let text = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        self.adapter.embed_text(&text)
    }

    fn embed_text(&self, text: &str) -> Result<RuntimeEmbedding, RuntimeError> {
        self.adapter.embed_text(text)
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        self.pending_imported_kv_blocks =
            self.adapter.import_cache_blocks(self.cache_mode, blocks)?;
        Ok(self.pending_imported_kv_blocks.len())
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(self.last_exported_kv_blocks.clone())
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        let mut ignore_token: fn(&RuntimeToken) -> Result<(), RuntimeError> = ignore_runtime_token;
        self.generate_stream(request, &mut ignore_token)
    }

    fn generate_stream(
        &mut self,
        request: RuntimeRequest,
        on_token: &mut dyn FnMut(&RuntimeToken) -> Result<(), RuntimeError>,
    ) -> Result<RuntimeResponse, RuntimeError> {
        let trace_id = runtime_request_trace_id(&request);
        let route_threshold = clamp_unit(request.route_budget.threshold);
        let imported_kv_blocks = if request.imported_kv_blocks.is_empty() {
            self.pending_imported_kv_blocks.clone()
        } else {
            request.imported_kv_blocks.clone()
        };
        let segments = imported_kv_segments(&self.tenant_scope, &imported_kv_blocks);
        let request = RustNativeAdapterRequest::from_runtime_request(
            request,
            trace_id,
            self.tenant_scope.clone(),
            self.cache_mode,
        )
        .with_segments(segments)
        .with_max_attention_threshold(route_threshold)
        .with_gate_summary(format!(
            "runtime_bridge_imports={}",
            imported_kv_blocks.len()
        ))
        .with_gate_summary("writer_gate=preview_only");

        let report = self.adapter.generate_stream(request, on_token)?;
        let response = report.response.clone();
        self.last_exported_kv_blocks = self.adapter.export_cache_blocks(&report)?;
        self.pending_imported_kv_blocks.clear();
        self.last_report = Some(report);
        Ok(response)
    }
}

impl RustNativeInferenceAdapter for MockRustNativeAdapter {
    fn metadata(&self) -> RuntimeMetadata {
        self.metadata.clone()
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        self.architecture
    }

    fn embed_text(&self, text: &str) -> Result<RuntimeEmbedding, RuntimeError> {
        let mut values = text
            .split_whitespace()
            .take(self.metadata.embedding_dimensions.max(1))
            .enumerate()
            .map(|(index, token)| ((token.len() + index + 1) as f32 / 32.0).clamp(0.0, 1.0))
            .collect::<Vec<_>>();
        values.resize(self.metadata.embedding_dimensions, 0.0);
        Ok(RuntimeEmbedding::new(values))
    }

    fn generate_stream(
        &mut self,
        request: RustNativeAdapterRequest,
        on_token: &mut dyn FnMut(&RuntimeToken) -> Result<(), RuntimeError>,
    ) -> Result<RustNativeAdapterReport, RuntimeError> {
        let hook_records = evaluate_chunked_kv_hooks(&request, &self.metadata, self.architecture);
        let (included_segments, skipped_segments, rejected_segments) =
            chunked_kv_hook_decision_counts(&hook_records);
        let imported_kv_blocks = hook_records
            .iter()
            .filter(|record| record.decision == ChunkedKvHookDecision::Include)
            .map(|record| record.kv_blocks)
            .sum::<usize>();
        let exported_kv_blocks = if request.cache_mode == ChunkedKvCacheMode::NoCache
            || !self.metadata.supports_kv_export
        {
            0
        } else {
            imported_kv_blocks.min(self.metadata.max_kv_export_blocks)
        };
        let answer = format!(
            "rust-native:{}:{}:{}",
            request.trace_id,
            request.cache_mode.as_str(),
            imported_kv_blocks
        );
        let stream_tokens = answer.split(':').map(RuntimeToken::new).collect::<Vec<_>>();
        for token in &stream_tokens {
            on_token(token)?;
        }

        let gate_summary_digest = gate_summary_digest(&request.gate_summaries);
        let diagnostics = RuntimeDiagnostics {
            model_id: Some(self.metadata.model_id.clone()),
            selected_adapter: Some("portable-rust".to_owned()),
            adapter_cache_mode: Some(request.cache_mode.as_str().to_owned()),
            adapter_stream_trace_id: Some(request.trace_id.clone()),
            adapter_stream_gate_summary_digest: Some(gate_summary_digest.clone()),
            adapter_stream_read_only: Some(true),
            adapter_stream_write_allowed: Some(false),
            adapter_stream_applied: Some(false),
            device_profile: Some(request.device_execution.device_profile.clone()),
            primary_lane: Some(request.device_execution.primary_lane.clone()),
            fallback_lane: Some(request.device_execution.fallback_lane.clone()),
            memory_mode: Some(request.device_execution.memory_mode.clone()),
            device_execution_source: Some(
                RuntimeDiagnostics::runtime_reported_device_execution_source().to_owned(),
            ),
            layer_count: self.architecture.layer_count,
            global_layers: 1,
            local_window_layers: 1,
            convolutional_fusion_layers: 1,
            hidden_size: self.architecture.hidden_size,
            local_window_tokens: self.architecture.local_window_tokens,
            forward_energy: Some(0.31 + imported_kv_blocks as f32 * 0.01),
            kv_influence: (imported_kv_blocks > 0)
                .then(|| (imported_kv_blocks as f32 / 8.0).clamp(0.0, 1.0)),
            imported_kv_blocks,
            exported_kv_blocks,
            runtime_kv_segments_included: included_segments,
            runtime_kv_segments_skipped: skipped_segments,
            runtime_kv_segments_rejected: rejected_segments,
            hot_kv_precision_bits: Some(request.device_execution.hot_kv_precision_bits),
            cold_kv_precision_bits: Some(request.device_execution.cold_kv_precision_bits),
            ..RuntimeDiagnostics::default()
        };
        let exported_blocks = (0..exported_kv_blocks)
            .map(|index| {
                let key = vec![0.10 + index as f32; self.metadata.embedding_dimensions.max(1)];
                let value = vec![0.20 + index as f32; self.metadata.embedding_dimensions.max(1)];
                RuntimeKvBlock::new(
                    index % self.architecture.layer_count.max(1),
                    index % self.architecture.kv_heads.max(1),
                    index,
                    index + 1,
                    key,
                    value,
                )
            })
            .collect::<Vec<_>>();
        let mut response = RuntimeResponse::new(answer.clone()).with_diagnostics(diagnostics);
        response.tokens = stream_tokens.clone();
        response.exported_kv_blocks = exported_blocks;
        response.trace = vec![
            ReasoningStep::new(
                "rust_native_adapter_trace",
                format!(
                    "trace_id={} mode={} gate_summary={}",
                    request.trace_id,
                    request.cache_mode.as_str(),
                    gate_summary_digest
                ),
                0.88,
            ),
            ReasoningStep::new(
                "rust_native_chunked_kv",
                format!(
                    "included={} skipped={} rejected={}",
                    included_segments, skipped_segments, rejected_segments
                ),
                0.82,
            ),
        ];

        Ok(RustNativeAdapterReport {
            trace_id: request.trace_id,
            cache_mode: request.cache_mode,
            response,
            hook_records,
            stream_tokens,
            imported_kv_blocks,
            exported_kv_blocks,
            gate_summary_digest,
            read_only: true,
            write_allowed: false,
            applied: false,
        })
    }
}

fn chunked_kv_hook_decision_counts(records: &[ChunkedKvHookRecord]) -> (usize, usize, usize) {
    let mut included = 0;
    let mut skipped = 0;
    let mut rejected = 0;
    for record in records {
        match record.decision {
            ChunkedKvHookDecision::Include => included += 1,
            ChunkedKvHookDecision::Skip => skipped += 1,
            ChunkedKvHookDecision::Reject => rejected += 1,
        }
    }
    (included, skipped, rejected)
}

fn evaluate_chunked_kv_hooks(
    request: &RustNativeAdapterRequest,
    metadata: &RuntimeMetadata,
    architecture: TransformerRuntimeArchitecture,
) -> Vec<ChunkedKvHookRecord> {
    let gate = TenantIsolationGate::new();
    request
        .segments
        .iter()
        .map(|segment| {
            let isolation = gate.check_key_access(
                &request.tenant_scope,
                &segment.cache_ref,
                TenantAccessKind::Read,
            );
            let (decision, reason) = if request.cache_mode == ChunkedKvCacheMode::NoCache {
                (ChunkedKvHookDecision::Skip, "cache_mode_no_cache")
            } else if !isolation.allowed {
                (ChunkedKvHookDecision::Reject, "tenant_scope_rejected")
            } else if request.cache_mode == ChunkedKvCacheMode::GenomeFiltered
                && !segment.genome_gate_passed
            {
                (ChunkedKvHookDecision::Skip, "genome_gate_filtered")
            } else if segment.attention_threshold > request.max_attention_threshold {
                (
                    ChunkedKvHookDecision::Skip,
                    "attention_threshold_above_budget",
                )
            } else if let Some(error) = first_kv_shape_error(segment, metadata, architecture) {
                (ChunkedKvHookDecision::Reject, error)
            } else {
                (ChunkedKvHookDecision::Include, "chunked_kv_included")
            };
            ChunkedKvHookRecord {
                trace_id: request.trace_id.clone(),
                segment_id: segment.segment_id.clone(),
                cache_ref_digest: segment.cache_ref.key_digest(),
                token_start: segment.token_start,
                token_end: segment.token_end,
                decision,
                attention_threshold: segment.attention_threshold,
                kv_blocks: if decision == ChunkedKvHookDecision::Include {
                    segment.kv_blocks.len()
                } else {
                    0
                },
                reason: reason.to_owned(),
                tenant_isolation_allowed: isolation.allowed,
                genome_gate_passed: segment.genome_gate_passed,
                redacted: true,
            }
        })
        .collect()
}

fn first_kv_shape_error(
    segment: &ChunkedKvSegment,
    metadata: &RuntimeMetadata,
    architecture: TransformerRuntimeArchitecture,
) -> Option<&'static str> {
    let dimensions = (metadata.embedding_dimensions > 0).then_some(metadata.embedding_dimensions);
    for block in &segment.kv_blocks {
        if let Err(error) =
            block.validate_shape(architecture.layer_count, architecture.kv_heads, dimensions)
        {
            return Some(kv_error_reason(&error));
        }
    }
    None
}

fn kv_error_reason(error: &RuntimeKvBlockValidationError) -> &'static str {
    match error {
        RuntimeKvBlockValidationError::EmptyTokenRange => "kv_empty_token_range",
        RuntimeKvBlockValidationError::LayerOutOfRange { .. } => "kv_layer_out_of_range",
        RuntimeKvBlockValidationError::HeadOutOfRange { .. } => "kv_head_out_of_range",
        RuntimeKvBlockValidationError::EmptyKey => "kv_empty_key",
        RuntimeKvBlockValidationError::EmptyValue => "kv_empty_value",
        RuntimeKvBlockValidationError::NonFiniteKey => "kv_non_finite_key",
        RuntimeKvBlockValidationError::NonFiniteValue => "kv_non_finite_value",
        RuntimeKvBlockValidationError::KeyDimensions { .. } => "kv_key_dimensions",
        RuntimeKvBlockValidationError::ValueDimensions { .. } => "kv_value_dimensions",
    }
}

fn gate_summary_digest(summaries: &[String]) -> String {
    stable_digest(&summaries.join("|"))
}

fn runtime_request_trace_id(request: &RuntimeRequest) -> String {
    format!(
        "runtime-{}",
        stable_digest(&format!(
            "{}:{}:{}:{}",
            task_profile_label(request.profile),
            request.max_tokens,
            request.imported_kv_blocks.len(),
            request.prompt
        ))
        .trim_start_matches("fnv64:")
    )
}

fn task_profile_label(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long",
    }
}

fn imported_kv_segments(
    tenant_scope: &TenantScope,
    imported_kv_blocks: &[RuntimeKvBlock],
) -> Vec<ChunkedKvSegment> {
    imported_kv_blocks
        .iter()
        .enumerate()
        .map(|(index, block)| {
            ChunkedKvSegment::new(
                format!("runtime-import-{index}"),
                tenant_scope.scoped_key(TenantResourceLane::RuntimeKv, format!("import:{index}")),
                block.token_start,
                block.token_end,
            )
            .with_attention_threshold(adaptive_kv_attention_threshold(block))
            .with_genome_gate(true)
            .with_kv_blocks(vec![block.clone()])
        })
        .collect()
}

fn adaptive_kv_attention_threshold(block: &RuntimeKvBlock) -> f32 {
    let key_energy = mean_abs(&block.key);
    let value_energy = mean_abs(&block.value);
    if key_energy <= f32::EPSILON {
        return 0.90;
    }

    let influence = (value_energy / key_energy).clamp(0.0, 1.0);
    (0.20 + (1.0 - influence) * 0.55).clamp(0.15, 0.90)
}

fn mean_abs(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(|value| value.abs()).sum::<f32>() / values.len() as f32
}

fn sanitize_id(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_' | '.'))
        .take(96)
        .collect::<String>();
    if sanitized.is_empty() {
        fallback.to_owned()
    } else {
        sanitized
    }
}

fn sanitize_summary(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_' | '=' | '.') {
                ch
            } else {
                '_'
            }
        })
        .take(96)
        .collect()
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn stable_digest(value: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("fnv64:{hash:016x}")
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control.is_control() => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn ignore_runtime_token(_token: &RuntimeToken) -> Result<(), RuntimeError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tenant_scope::TenantResourceLane;

    fn scope() -> TenantScope {
        TenantScope::new("tenant-a", "workspace", "session")
    }

    fn segment(
        scope: &TenantScope,
        id: &str,
        attention_threshold: f32,
        genome_gate: bool,
    ) -> ChunkedKvSegment {
        ChunkedKvSegment::new(
            id,
            scope.scoped_key(TenantResourceLane::RuntimeKv, format!("cache:{id}")),
            0,
            8,
        )
        .with_attention_threshold(attention_threshold)
        .with_genome_gate(genome_gate)
        .with_kv_blocks(vec![RuntimeKvBlock::new(
            0,
            0,
            0,
            1,
            vec![0.1; 8],
            vec![0.2; 8],
        )])
    }

    #[test]
    fn mock_native_adapter_imports_chunked_kv_and_streams_trace_tokens() {
        let scope = scope();
        let request = RustNativeAdapterRequest::new(
            "use chunked kv",
            TaskProfile::Coding,
            "trace-38",
            scope.clone(),
        )
        .with_segments(vec![segment(&scope, "seg-a", 0.40, true)])
        .with_gate_summary("memory_gate=preview_only")
        .with_gate_summary("genome_gate=passed");
        let mut adapter = MockRustNativeAdapter::new();
        let mut streamed = Vec::new();

        let report = adapter
            .generate_stream(request, &mut |token| {
                streamed.push(token.text.clone());
                Ok(())
            })
            .unwrap();

        assert_eq!(report.included_segments(), 1);
        assert_eq!(report.imported_kv_blocks, 1);
        assert_eq!(report.exported_kv_blocks, 1);
        assert_eq!(report.hook_records[0].token_start, 0);
        assert_eq!(report.hook_records[0].token_end, 8);
        assert_eq!(
            streamed,
            vec!["rust-native", "trace-38", "chunked_cache", "1"]
        );
        assert!(report.response.trace.iter().any(|step| {
            step.label == "rust_native_adapter_trace" && step.content.contains("trace_id=trace-38")
        }));
        assert!(report.summary_line().contains("gate_summary=fnv64:"));
        assert!(
            report
                .hook_summaries()
                .iter()
                .any(|summary| summary.contains("token_start=0 token_end=8"))
        );
        assert!(report.is_preview_only());
    }

    #[test]
    fn adapter_trait_exposes_cache_import_export_operations() {
        let scope = scope();
        let blocks = vec![RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1; 8], vec![0.2; 8])];
        let mut adapter = MockRustNativeAdapter::new();

        assert_eq!(
            adapter
                .import_cache_blocks(ChunkedKvCacheMode::ChunkedCache, &blocks)
                .unwrap(),
            blocks
        );
        assert!(
            adapter
                .import_cache_blocks(ChunkedKvCacheMode::NoCache, &blocks)
                .unwrap()
                .is_empty()
        );

        let request = RustNativeAdapterRequest::new(
            "export cache",
            TaskProfile::Coding,
            "trace-cache-op",
            scope.clone(),
        )
        .with_segments(vec![segment(&scope, "seg-cache-op", 0.20, true)]);
        let report = adapter.generate(request).unwrap();

        assert_eq!(
            adapter.export_cache_blocks(&report).unwrap(),
            report.response.exported_kv_blocks
        );
    }

    #[test]
    fn genome_filtered_mode_skips_segments_that_fail_gene_gate() {
        let scope = scope();
        let request = RustNativeAdapterRequest::new(
            "filter genome",
            TaskProfile::Coding,
            "trace-genome",
            scope.clone(),
        )
        .with_cache_mode(ChunkedKvCacheMode::GenomeFiltered)
        .with_segments(vec![
            segment(&scope, "seg-pass", 0.30, true),
            segment(&scope, "seg-filter", 0.30, false),
        ]);
        let mut adapter = MockRustNativeAdapter::new();

        let report = adapter.generate(request).unwrap();

        assert_eq!(report.included_segments(), 1);
        assert_eq!(report.skipped_segments(), 1);
        assert!(report.hook_summaries().iter().any(|summary| {
            summary.contains("segment=seg-filter")
                && summary.contains("decision=skip")
                && summary.contains("reason=genome_gate_filtered")
        }));
        assert_eq!(report.imported_kv_blocks, 1);
    }

    #[test]
    fn cross_tenant_cache_refs_are_rejected_before_import() {
        let actor = scope();
        let foreign = TenantScope::new("tenant-b", "workspace", "session");
        let request = RustNativeAdapterRequest::new(
            "reject tenant",
            TaskProfile::Coding,
            "trace-tenant",
            actor,
        )
        .with_segments(vec![segment(&foreign, "foreign", 0.20, true)]);
        let mut adapter = MockRustNativeAdapter::new();

        let report = adapter.generate(request).unwrap();

        assert_eq!(report.included_segments(), 0);
        assert_eq!(report.rejected_segments(), 1);
        assert_eq!(report.imported_kv_blocks, 0);
        assert!(report.hook_summaries().iter().all(|summary| {
            summary.contains("cache_ref=fnv64:")
                && !summary.contains("tenant-b")
                && !summary.contains("reject tenant")
        }));
        assert!(
            report
                .hook_summaries()
                .iter()
                .any(|summary| summary.contains("reason=tenant_scope_rejected"))
        );
    }

    #[test]
    fn no_cache_mode_compares_without_import_or_export() {
        let scope = scope();
        let request = RustNativeAdapterRequest::new(
            "no cache",
            TaskProfile::Coding,
            "trace-nocache",
            scope.clone(),
        )
        .with_cache_mode(ChunkedKvCacheMode::NoCache)
        .with_segments(vec![segment(&scope, "seg-a", 0.20, true)]);
        let mut adapter = MockRustNativeAdapter::new();

        let report = adapter.generate(request).unwrap();

        assert_eq!(report.included_segments(), 0);
        assert_eq!(report.skipped_segments(), 1);
        assert_eq!(report.imported_kv_blocks, 0);
        assert_eq!(report.exported_kv_blocks, 0);
        assert!(
            report
                .hook_summaries()
                .iter()
                .any(|summary| summary.contains("reason=cache_mode_no_cache"))
        );
    }

    #[test]
    fn cache_mode_comparison_reports_no_cache_chunked_and_genome_filtered_paths() {
        let scope = scope();
        let request = RustNativeAdapterRequest::new(
            "compare cache modes",
            TaskProfile::Coding,
            "trace-compare",
            scope.clone(),
        )
        .with_segments(vec![
            segment(&scope, "seg-pass", 0.20, true),
            segment(&scope, "seg-filter", 0.20, false),
        ])
        .with_gate_summary("writer_gate=preview_only");
        let mut adapter = MockRustNativeAdapter::new();

        let report = adapter.compare_cache_modes(request, &[]).unwrap();

        assert!(report.has_required_cache_modes());
        for mode in &report.modes {
            assert!(mode.read_only);
            assert!(!mode.write_allowed);
            assert!(!mode.applied);
        }
        assert_eq!(
            report
                .mode(ChunkedKvCacheMode::NoCache)
                .unwrap()
                .imported_kv_blocks,
            0
        );
        assert_eq!(
            report
                .mode(ChunkedKvCacheMode::ChunkedCache)
                .unwrap()
                .imported_kv_blocks,
            2
        );
        assert_eq!(
            report
                .mode(ChunkedKvCacheMode::GenomeFiltered)
                .unwrap()
                .imported_kv_blocks,
            1
        );
        assert_eq!(
            report.imported_delta_vs_no_cache(ChunkedKvCacheMode::ChunkedCache),
            Some(2)
        );
        assert_eq!(
            report.imported_delta_vs_no_cache(ChunkedKvCacheMode::GenomeFiltered),
            Some(1)
        );
        assert!(report.summary_line().contains("cache_modes=3"));
        assert!(report.summary_line().contains("mode=no_cache"));
        assert!(report.summary_line().contains("mode=chunked_cache"));
        assert!(report.summary_line().contains("mode=genome_filtered"));
        assert!(report.summary_line().contains("read_only=true"));
        assert!(report.summary_line().contains("write_allowed=false"));
        assert!(report.summary_line().contains("applied=false"));
        assert!(
            report
                .mode(ChunkedKvCacheMode::GenomeFiltered)
                .unwrap()
                .skipped_segments
                > 0
        );
    }

    #[test]
    fn stream_events_preserve_trace_and_gate_summary_without_prompt_or_cache_leak() {
        let scope = scope();
        let request = RustNativeAdapterRequest::new(
            "secret prompt should not appear",
            TaskProfile::Coding,
            "trace-openai",
            scope.clone(),
        )
        .with_segments(vec![segment(&scope, "seg-a", 0.20, true)])
        .with_gate_summary("writer_gate=preview_only");
        let mut adapter = MockRustNativeAdapter::new();

        let report = adapter.generate(request).unwrap();
        let events = report.openai_stream_events();

        assert_eq!(events.first().unwrap().event, "response.created");
        assert!(events.first().unwrap().data.contains("\"read_only\":true"));
        assert!(
            events
                .first()
                .unwrap()
                .data
                .contains("\"write_allowed\":false")
        );
        assert!(events.first().unwrap().data.contains("\"applied\":false"));
        assert_eq!(events.last().unwrap().event, "done");
        assert_eq!(events.last().unwrap().data, "[DONE]");
        assert!(events.iter().any(|event| {
            event.event == "response.output_text.delta" && event.data.contains("trace-openai")
        }));
        assert!(events.iter().any(|event| {
            event.event == "response.completed"
                && event.data.contains("\"gate_summary\":\"fnv64:")
                && event.data.contains("\"imported_kv_blocks\":1")
                && event.data.contains("\"read_only\":true")
                && event.data.contains("\"write_allowed\":false")
                && event.data.contains("\"applied\":false")
        }));
        let joined = events
            .iter()
            .map(RustNativeAdapterStreamEvent::sse_line)
            .collect::<Vec<_>>()
            .join("");
        assert!(joined.contains("event: response.created"));
        assert!(joined.contains("data: [DONE]"));
        assert!(!joined.contains("secret prompt should not appear"));
        assert!(!joined.contains("cache:seg-a"));
        assert!(!joined.contains("tenant-a"));
    }
}
