use crate::reflection::RuntimeDiagnostics;
use crate::runtime::RuntimeResponse;
use crate::runtime_manifest::RuntimeManifest;

use super::contract::ProductionKernelConformanceGate;
use super::report::ProductionKernelConformanceReport;

pub(super) fn evaluate_conformance_response(
    manifest: &RuntimeManifest,
    gate: ProductionKernelConformanceGate,
    kernel_reported_runtime_kv_segment_signal: bool,
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
    if gate.require_tokens && !response.tokens.is_empty() && report.uncertainty_token_count == 0 {
        report.failures.push(
            "kernel did not return any runtime token entropy/logprob uncertainty signal".to_owned(),
        );
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
    if diagnostics.layer_mode_count() != diagnostics.layer_count {
        report.failures.push(format!(
            "diagnostics layer mode count {} does not match layer_count {}",
            diagnostics.layer_mode_count(),
            diagnostics.layer_count
        ));
    }
    if gate.require_layer_mode_coverage && !diagnostics.has_all_layer_modes() {
        report.failures.push(format!(
            "kernel did not cover all Transformer layer modes: global={} local_window={} convolutional_fusion={}",
            diagnostics.global_layers,
            diagnostics.local_window_layers,
            diagnostics.convolutional_fusion_layers
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
    if gate.require_runtime_kv_segment_signal
        && manifest.kv_policy.import_enabled
        && diagnostics.imported_kv_blocks > 0
        && !kernel_reported_runtime_kv_segment_signal
    {
        report.failures.push(
            "runtime KV import is enabled but kernel reported no KV segment signal".to_owned(),
        );
    }
    let has_any_adapter_stream_write_gate_field = diagnostics.adapter_stream_read_only.is_some()
        || diagnostics.adapter_stream_write_allowed.is_some()
        || diagnostics.adapter_stream_applied.is_some();
    let has_complete_adapter_stream_write_gate = diagnostics.has_adapter_stream_write_gate_signal();
    let has_adapter_stream_evidence = diagnostics.has_adapter_stream_trace_signal()
        || diagnostics.has_adapter_stream_gate_summary_signal();
    if gate.require_adapter_stream_preview_only
        && diagnostics
            .adapter_stream_gate_summary_digest
            .as_deref()
            .is_some_and(|value| {
                RuntimeDiagnostics::normalize_adapter_stream_gate_summary_digest(value).is_none()
            })
    {
        report
            .failures
            .push("kernel reported malformed adapter stream gate summary digest".to_owned());
    }
    if gate.require_adapter_stream_preview_only
        && has_any_adapter_stream_write_gate_field
        && !has_complete_adapter_stream_write_gate
    {
        report
            .failures
            .push("kernel reported partial adapter stream write gate state".to_owned());
    }
    if gate.require_adapter_stream_preview_only
        && has_adapter_stream_evidence
        && !has_any_adapter_stream_write_gate_field
    {
        report
            .failures
            .push("kernel reported adapter stream without write gate state".to_owned());
    }
    if gate.require_adapter_stream_preview_only
        && diagnostics.adapter_stream_preview_only() == Some(false)
    {
        report
            .failures
            .push("kernel adapter stream was not preview-only during conformance".to_owned());
    }
    if diagnostics.exported_kv_blocks != report.exported_kv_blocks {
        report.failures.push(format!(
            "diagnostics exported_kv_blocks {} does not match runtime exported KV {}",
            diagnostics.exported_kv_blocks, report.exported_kv_blocks
        ));
    }
    if diagnostics.weak_runtime_kv_imports_skipped > 0 {
        report.failures.push(format!(
            "kernel skipped {} weak runtime KV imports during conformance",
            diagnostics.weak_runtime_kv_imports_skipped
        ));
    }
    if diagnostics.budget_limited_runtime_kv_imports_skipped > 0 {
        report.failures.push(format!(
            "kernel skipped {} runtime KV imports due to budget during conformance",
            diagnostics.budget_limited_runtime_kv_imports_skipped
        ));
    }
}
