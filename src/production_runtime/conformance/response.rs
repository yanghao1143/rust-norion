use crate::runtime::RuntimeResponse;
use crate::runtime_manifest::RuntimeManifest;

use super::contract::ProductionKernelConformanceGate;
use super::report::ProductionKernelConformanceReport;

pub(super) fn evaluate_conformance_response(
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
    if diagnostics.exported_kv_blocks != report.exported_kv_blocks {
        report.failures.push(format!(
            "diagnostics exported_kv_blocks {} does not match runtime exported KV {}",
            diagnostics.exported_kv_blocks, report.exported_kv_blocks
        ));
    }
}
