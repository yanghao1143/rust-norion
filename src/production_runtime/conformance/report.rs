use crate::hardware::RuntimeManifestDeviceGateReport;
use crate::runtime_manifest::RuntimeManifest;

use super::util::option_f32_display;

#[derive(Debug, Clone, PartialEq)]
pub struct ProductionKernelConformanceReport {
    pub passed: bool,
    pub model_id: String,
    pub selected_adapter: String,
    pub kernel_connected: bool,
    pub manifest_hot_kv_bits: u8,
    pub manifest_cold_kv_bits: u8,
    pub device_hot_kv_bits: u8,
    pub device_cold_kv_bits: u8,
    pub request_runtime_hot_kv_bits: u8,
    pub request_runtime_cold_kv_bits: u8,
    pub request_device_hot_kv_bits: u8,
    pub request_device_cold_kv_bits: u8,
    pub token_count: usize,
    pub uncertainty_token_count: usize,
    pub average_entropy: Option<f32>,
    pub average_neg_logprob: Option<f32>,
    pub uncertainty_perplexity: Option<f32>,
    pub trace_steps: usize,
    pub imported_kv_blocks: usize,
    pub exported_kv_blocks: usize,
    pub runtime_kv_segments_included: usize,
    pub runtime_kv_segments_skipped: usize,
    pub runtime_kv_segments_rejected: usize,
    pub forward_energy: Option<f32>,
    pub kv_influence: Option<f32>,
    pub global_layers: usize,
    pub local_window_layers: usize,
    pub convolutional_fusion_layers: usize,
    pub failures: Vec<String>,
}

impl ProductionKernelConformanceReport {
    pub(super) fn new(
        manifest: &RuntimeManifest,
        device_gate: &RuntimeManifestDeviceGateReport,
        kernel_connected: bool,
    ) -> Self {
        Self {
            passed: false,
            model_id: manifest.metadata.model_id.clone(),
            selected_adapter: device_gate.runtime_adapter_name().to_owned(),
            kernel_connected,
            manifest_hot_kv_bits: manifest.quantization.hot_kv.width(),
            manifest_cold_kv_bits: manifest.quantization.cold_kv.width(),
            device_hot_kv_bits: device_gate.hot_kv_precision_bits,
            device_cold_kv_bits: device_gate.cold_kv_precision_bits,
            request_runtime_hot_kv_bits: 0,
            request_runtime_cold_kv_bits: 0,
            request_device_hot_kv_bits: 0,
            request_device_cold_kv_bits: 0,
            token_count: 0,
            uncertainty_token_count: 0,
            average_entropy: None,
            average_neg_logprob: None,
            uncertainty_perplexity: None,
            trace_steps: 0,
            imported_kv_blocks: 0,
            exported_kv_blocks: 0,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            forward_energy: None,
            kv_influence: None,
            global_layers: 0,
            local_window_layers: 0,
            convolutional_fusion_layers: 0,
            failures: Vec::new(),
        }
    }

    pub fn failed(
        model_id: impl Into<String>,
        selected_adapter: impl Into<String>,
        kernel_connected: bool,
        failure: impl Into<String>,
    ) -> Self {
        let mut report = Self {
            passed: false,
            model_id: model_id.into(),
            selected_adapter: selected_adapter.into(),
            kernel_connected,
            manifest_hot_kv_bits: 0,
            manifest_cold_kv_bits: 0,
            device_hot_kv_bits: 0,
            device_cold_kv_bits: 0,
            request_runtime_hot_kv_bits: 0,
            request_runtime_cold_kv_bits: 0,
            request_device_hot_kv_bits: 0,
            request_device_cold_kv_bits: 0,
            token_count: 0,
            uncertainty_token_count: 0,
            average_entropy: None,
            average_neg_logprob: None,
            uncertainty_perplexity: None,
            trace_steps: 0,
            imported_kv_blocks: 0,
            exported_kv_blocks: 0,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            forward_energy: None,
            kv_influence: None,
            global_layers: 0,
            local_window_layers: 0,
            convolutional_fusion_layers: 0,
            failures: Vec::new(),
        };
        report.failures.push(failure.into());
        report
    }

    pub fn summary_line(&self) -> String {
        format!(
            "production_kernel_conformance: passed={} model_id={} adapter={} kernel_connected={} manifest_kv_bits={}/{} device_kv_bits={}/{} request_runtime_kv_bits={}/{} request_device_kv_bits={}/{} tokens={} uncertainty_tokens={} average_entropy={} average_neg_logprob={} uncertainty_perplexity={} trace_steps={} imported_kv={} exported_kv={} runtime_kv_segments_included={} runtime_kv_segments_skipped={} runtime_kv_segments_rejected={} forward_energy={} kv_influence={} global_layers={} local_window_layers={} convolutional_fusion_layers={} failures={}",
            self.passed,
            self.model_id,
            self.selected_adapter,
            self.kernel_connected,
            self.manifest_hot_kv_bits,
            self.manifest_cold_kv_bits,
            self.device_hot_kv_bits,
            self.device_cold_kv_bits,
            self.request_runtime_hot_kv_bits,
            self.request_runtime_cold_kv_bits,
            self.request_device_hot_kv_bits,
            self.request_device_cold_kv_bits,
            self.token_count,
            self.uncertainty_token_count,
            option_f32_display(self.average_entropy),
            option_f32_display(self.average_neg_logprob),
            option_f32_display(self.uncertainty_perplexity),
            self.trace_steps,
            self.imported_kv_blocks,
            self.exported_kv_blocks,
            self.runtime_kv_segments_included,
            self.runtime_kv_segments_skipped,
            self.runtime_kv_segments_rejected,
            option_f32_display(self.forward_energy),
            option_f32_display(self.kv_influence),
            self.global_layers,
            self.local_window_layers,
            self.convolutional_fusion_layers,
            self.failures.len()
        )
    }

    pub fn runtime_kv_segment_count(&self) -> usize {
        self.runtime_kv_segments_included
            .saturating_add(self.runtime_kv_segments_skipped)
            .saturating_add(self.runtime_kv_segments_rejected)
    }
}
