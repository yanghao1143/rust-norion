use crate::engine::{GenerationContext, InferenceBackend};
use crate::reflection::{DraftToken, InferenceDraft, ReasoningStep};

use super::contract::{
    populate_runtime_device_execution, populate_runtime_kv_precision,
    validate_runtime_response_contract,
};
use super::kv_import::runtime_kv_import_selection_from_context;
use super::kv_safety::{RuntimeKvSafetyReport, sanitize_runtime_kv_blocks};
use super::types::{ModelRuntime, RuntimeError, RuntimeRequest, RuntimeResponse, RuntimeToken};

#[derive(Debug, Clone)]
pub struct RuntimeBackend<R> {
    runtime: R,
    max_tokens: usize,
    generation_max_tokens: Option<usize>,
    runtime_endpoint_override: Option<String>,
    last_error: Option<RuntimeError>,
}

impl<R> RuntimeBackend<R> {
    pub fn new(runtime: R) -> Self {
        Self {
            runtime,
            max_tokens: 512,
            generation_max_tokens: None,
            runtime_endpoint_override: None,
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
    fn configure_generation(&mut self, max_tokens: Option<usize>) {
        self.generation_max_tokens = max_tokens.map(|value| value.max(1));
    }

    fn configure_runtime_endpoint_override(
        &mut self,
        base_url: Option<&str>,
    ) -> Result<bool, String> {
        let Some(base_url) = base_url.map(str::trim).filter(|value| !value.is_empty()) else {
            self.runtime_endpoint_override = None;
            return Ok(false);
        };
        if !self.runtime.supports_endpoint_override() {
            self.runtime_endpoint_override = None;
            return Ok(false);
        }
        match self.runtime.clone_for_endpoint_override(base_url) {
            Ok(Some(_)) => {
                self.runtime_endpoint_override = Some(base_url.to_owned());
                Ok(true)
            }
            Ok(None) => {
                self.runtime_endpoint_override = None;
                Ok(false)
            }
            Err(error) => {
                self.runtime_endpoint_override = None;
                Err(error.message().to_owned())
            }
        }
    }

    fn runtime_endpoint_override_active(&self) -> Option<&str> {
        self.runtime_endpoint_override.as_deref()
    }

    fn runtime_native_context_window(&self) -> Option<usize> {
        let window = self.runtime.metadata().native_context_window;
        (window > 0).then_some(window)
    }

    fn embed_text(&mut self, text: &str) -> Option<Vec<f32>> {
        let endpoint_override = self.runtime_endpoint_override.clone();
        let mut override_runtime = match self.override_runtime(endpoint_override.as_deref()) {
            Ok(runtime) => runtime,
            Err(error) => {
                self.last_error = Some(error);
                return None;
            }
        };
        let runtime = override_runtime.as_mut().unwrap_or(&mut self.runtime);
        match runtime.embed_text(text) {
            Ok(embedding) if !embedding.values.is_empty() => Some(embedding.values),
            Ok(_) => None,
            Err(error) => {
                self.last_error = Some(error);
                None
            }
        }
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        self.generate_with_runtime(context, None)
    }

    fn generate_stream(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken),
    ) -> InferenceDraft {
        let mut checked = |token: &DraftToken| {
            on_token(token);
            Ok(())
        };
        self.generate_with_runtime(context, Some(&mut checked))
    }

    fn generate_stream_checked(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
    ) -> InferenceDraft {
        self.generate_with_runtime(context, Some(on_token))
    }
}

impl<R: ModelRuntime> RuntimeBackend<R> {
    fn generate_with_runtime(
        &mut self,
        context: GenerationContext<'_>,
        mut on_token: Option<&mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>>,
    ) -> InferenceDraft {
        let endpoint_override = self.runtime_endpoint_override.clone();
        let mut override_runtime = match self.override_runtime(endpoint_override.as_deref()) {
            Ok(runtime) => runtime,
            Err(error) => {
                self.last_error = Some(error.clone());
                return InferenceDraft::new(
                    format!("Runtime backend error: {}", error.message()),
                    vec![ReasoningStep::new(
                        "runtime_endpoint_override_error",
                        error.message(),
                        0.0,
                    )],
                );
            }
        };
        let using_endpoint_override = override_runtime.is_some();
        let forwarded_endpoint = using_endpoint_override
            .then(|| endpoint_override.clone())
            .flatten();
        let runtime = override_runtime.as_mut().unwrap_or(&mut self.runtime);
        let runtime_metadata = runtime.metadata();
        let runtime_architecture = runtime.architecture();
        let import_selection = runtime_kv_import_selection_from_context(
            &context,
            &runtime_metadata,
            runtime_architecture,
        );
        let import_report = sanitize_runtime_kv_blocks(
            import_selection.blocks,
            &runtime_metadata,
            runtime_architecture,
            true,
            "imported_kv_blocks",
        );
        let imported_kv_blocks =
            if runtime_metadata.supports_kv_import && !import_report.accepted.is_empty() {
                match runtime.import_kv(&import_report.accepted) {
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
            self.generation_max_tokens.unwrap_or(self.max_tokens),
            runtime_metadata.clone(),
            runtime_architecture,
        )
        .with_imported_kv_blocks(import_report.accepted.clone());

        let result = if let Some(on_token) = on_token.as_mut() {
            runtime.generate_stream(request, &mut |token| {
                on_token(&DraftToken {
                    text: token.text.clone(),
                    logprob: token.logprob,
                    entropy: token.entropy,
                })
            })
        } else {
            runtime.generate(request)
        };

        match result {
            Ok(response) => {
                self.last_error = None;
                draft_from_response(
                    runtime,
                    response,
                    &context,
                    runtime_metadata,
                    runtime_architecture,
                    imported_kv_blocks,
                    import_report,
                    import_selection.weak_runtime_kv_skipped,
                    import_selection.budget_limited_candidates_skipped,
                    forwarded_endpoint.as_deref(),
                )
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

    fn override_runtime(&self, base_url: Option<&str>) -> Result<Option<R>, RuntimeError> {
        let Some(base_url) = base_url else {
            return Ok(None);
        };
        self.runtime
            .clone_for_endpoint_override(base_url)?
            .ok_or_else(|| {
                RuntimeError::new(format!(
                    "runtime endpoint override is not supported for {base_url}"
                ))
            })
            .map(Some)
    }
}

fn draft_from_response<R: ModelRuntime>(
    runtime: &mut R,
    response: RuntimeResponse,
    context: &GenerationContext<'_>,
    runtime_metadata: super::RuntimeMetadata,
    runtime_architecture: crate::runtime_manifest::TransformerRuntimeArchitecture,
    imported_kv_blocks: usize,
    import_report: RuntimeKvSafetyReport,
    weak_runtime_kv_skipped: usize,
    budget_limited_candidates_skipped: usize,
    forwarded_endpoint: Option<&str>,
) -> InferenceDraft {
    let RuntimeResponse {
        answer,
        tokens: response_tokens,
        trace: response_trace,
        mut diagnostics,
        exported_kv_blocks: response_exported_kv_blocks,
    } = response;
    let runtime_reported_imported_kv_blocks = diagnostics.imported_kv_blocks;
    let runtime_reported_kv_segments = diagnostics.has_runtime_kv_segment_signal();
    let effective_imported_kv_blocks = if runtime_reported_kv_segments {
        runtime_reported_imported_kv_blocks
    } else {
        imported_kv_blocks
    };
    let runtime_contract_violations = validate_runtime_response_contract(
        &diagnostics,
        &runtime_metadata,
        runtime_architecture,
        context.hardware_plan,
    );
    populate_runtime_device_execution(&mut diagnostics, context.hardware_plan);
    let trace = if response_trace.is_empty() {
        trace_from_tokens(&response_tokens)
    } else {
        response_trace
    };
    let tokens = response_tokens
        .into_iter()
        .map(|token| DraftToken {
            text: token.text,
            logprob: token.logprob,
            entropy: token.entropy,
        })
        .collect();
    let mut trace = trace;
    if effective_imported_kv_blocks > 0 {
        trace.push(ReasoningStep::new(
            "runtime_kv_import",
            format!("imported {effective_imported_kv_blocks} KV blocks"),
            0.78,
        ));
    }
    if weak_runtime_kv_skipped > 0 {
        trace.push(ReasoningStep::new(
            "runtime_kv_import_selection",
            format!("skipped {weak_runtime_kv_skipped} weak runtime KV candidates before import"),
            0.70,
        ));
    }
    if budget_limited_candidates_skipped > 0 {
        let candidate_noun = if budget_limited_candidates_skipped == 1 {
            "candidate"
        } else {
            "candidates"
        };
        trace.push(ReasoningStep::new(
                "runtime_kv_import_budget",
                format!(
                    "skipped {budget_limited_candidates_skipped} runtime KV {candidate_noun} under compute budget"
                ),
                0.72,
            ));
    }
    push_kv_safety_trace(&mut trace, "runtime_kv_import_safety", &import_report);
    if let Some(endpoint) = forwarded_endpoint {
        trace.push(ReasoningStep::new(
            "runtime_endpoint_override",
            format!("forwarded generation to {endpoint}"),
            0.84,
        ));
    }
    for violation in &runtime_contract_violations {
        trace.push(ReasoningStep::new(
            "runtime_contract_violation",
            violation.clone(),
            0.05,
        ));
    }
    let exported_kv_blocks =
        if runtime_metadata.supports_kv_export && runtime_contract_violations.is_empty() {
            let raw_blocks = if response_exported_kv_blocks.is_empty() {
                runtime.export_kv()
            } else {
                Ok(response_exported_kv_blocks)
            };
            match raw_blocks {
                Ok(blocks) => {
                    let export_report = sanitize_runtime_kv_blocks(
                        blocks,
                        &runtime_metadata,
                        runtime_architecture,
                        false,
                        "exported_kv_blocks",
                    );
                    let accepted = export_report.accepted;
                    if !accepted.is_empty() {
                        trace.push(ReasoningStep::new(
                            "runtime_kv_export",
                            format!("exported {} KV blocks", accepted.len()),
                            0.74,
                        ));
                    }
                    push_kv_safety_trace(
                        &mut trace,
                        "runtime_kv_export_safety",
                        &RuntimeKvSafetyReport {
                            accepted: Vec::new(),
                            rejected: export_report.rejected,
                            truncated: export_report.truncated,
                        },
                    );
                    accepted
                }
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
    if !runtime_contract_violations.is_empty() {
        diagnostics.selected_adapter = None;
        diagnostics = diagnostics.clear_device_execution();
        diagnostics = diagnostics.clear_kv_precision();
    }
    diagnostics.imported_kv_blocks = effective_imported_kv_blocks;
    diagnostics.weak_runtime_kv_imports_skipped = weak_runtime_kv_skipped;
    diagnostics.budget_limited_runtime_kv_imports_skipped = budget_limited_candidates_skipped;
    diagnostics.exported_kv_blocks = exported_kv_blocks.len();
    if runtime_contract_violations.is_empty() {
        populate_runtime_kv_precision(&mut diagnostics, &runtime_metadata);
    }
    InferenceDraft::new(answer, trace)
        .with_tokens(tokens)
        .with_exported_kv_blocks(exported_kv_blocks)
        .with_runtime_diagnostics(diagnostics)
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

fn push_kv_safety_trace(
    trace: &mut Vec<ReasoningStep>,
    label: &str,
    report: &RuntimeKvSafetyReport,
) {
    if report.rejected_count() == 0 {
        return;
    }

    let mut reasons = report.rejected.clone();
    if report.truncated > 0 {
        reasons.push(format!(
            "truncated {} blocks above runtime limit",
            report.truncated
        ));
    }
    trace.push(ReasoningStep::new(
        label,
        format!(
            "rejected {} unsafe KV blocks: {}",
            report.rejected_count(),
            reasons.join("; ")
        ),
        0.18,
    ));
}
