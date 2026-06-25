use crate::runtime::ModelRuntime;

use super::super::ProductionTransformerRuntime;
use super::contract::ProductionKernelConformanceGate;
use super::report::ProductionKernelConformanceReport;
use super::request::{
    conformance_import_blocks, conformance_request, evaluate_conformance_request_contract,
};
use super::response::evaluate_conformance_response;
use super::token::ProductionTokenUncertainty;

impl ProductionTransformerRuntime {
    pub fn conformance_report(
        &self,
        gate: ProductionKernelConformanceGate,
    ) -> ProductionKernelConformanceReport {
        let mut report = ProductionKernelConformanceReport::new(
            &self.manifest,
            &self.device_gate,
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
        report.request_runtime_hot_kv_bits = request.runtime_metadata.hot_kv_precision_bits;
        report.request_runtime_cold_kv_bits = request.runtime_metadata.cold_kv_precision_bits;
        report.request_device_hot_kv_bits = request.hardware_plan.execution.hot_kv_precision_bits;
        report.request_device_cold_kv_bits = request.hardware_plan.execution.cold_kv_precision_bits;
        evaluate_conformance_request_contract(
            &self.manifest,
            &self.device_gate,
            &request,
            &mut report,
        );
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
        let token_uncertainty = ProductionTokenUncertainty::from_tokens(&response.tokens);
        report.uncertainty_token_count = token_uncertainty.uncertainty_token_count;
        report.average_entropy = token_uncertainty.average_entropy;
        report.average_neg_logprob = token_uncertainty.average_neg_logprob;
        report.uncertainty_perplexity = token_uncertainty.uncertainty_perplexity;
        report.trace_steps = response.trace.len();
        report.forward_energy = response.diagnostics.forward_energy;
        report.kv_influence = response.diagnostics.kv_influence;
        report.runtime_kv_segments_included = response.diagnostics.runtime_kv_segments_included;
        report.runtime_kv_segments_skipped = response.diagnostics.runtime_kv_segments_skipped;
        report.runtime_kv_segments_rejected = response.diagnostics.runtime_kv_segments_rejected;
        report.global_layers = response.diagnostics.global_layers;
        report.local_window_layers = response.diagnostics.local_window_layers;
        report.convolutional_fusion_layers = response.diagnostics.convolutional_fusion_layers;
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
