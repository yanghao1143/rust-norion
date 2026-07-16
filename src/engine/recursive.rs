use crate::recursive_scheduler::RecursiveChunk;
use crate::reflection::{DraftToken, InferenceDraft, ReasoningStep, RuntimeDiagnostics};
use crate::runtime::RuntimeError;

use super::metrics::average;
use super::text::compact;
use super::types::{
    GenerationContext, InferenceBackend, generation_cancelled_draft, stream_observer_error_draft,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct RecursiveExecutionReceipt {
    pub dispatched_waves: usize,
    pub parallel_waves: usize,
    pub max_dispatch_width: usize,
}

impl RecursiveExecutionReceipt {
    pub fn merge(&mut self, other: Self) {
        self.dispatched_waves = self.dispatched_waves.saturating_add(other.dispatched_waves);
        self.parallel_waves = self.parallel_waves.saturating_add(other.parallel_waves);
        self.max_dispatch_width = self.max_dispatch_width.max(other.max_dispatch_width);
    }
}

pub(super) fn generate_with_recursive_schedule<B: InferenceBackend>(
    backend: &mut B,
    context: GenerationContext<'_>,
) -> (InferenceDraft, usize, RecursiveExecutionReceipt) {
    let mut never_cancel = || false;
    generate_with_recursive_schedule_cancelable(backend, context, &mut never_cancel)
}

pub(super) fn generate_with_recursive_schedule_cancelable<B: InferenceBackend>(
    backend: &mut B,
    context: GenerationContext<'_>,
    should_cancel: &mut dyn FnMut() -> bool,
) -> (InferenceDraft, usize, RecursiveExecutionReceipt) {
    if !context.recursive_schedule.requires_recursion {
        return (
            backend.generate_cancelable(context, should_cancel),
            1,
            RecursiveExecutionReceipt::default(),
        );
    }

    let mut chunk_drafts = Vec::with_capacity(context.recursive_schedule.chunks.len());
    let mut runtime_calls = 0usize;
    let mut execution_receipt = RecursiveExecutionReceipt::default();
    for wave in &context.recursive_schedule.execution_waves {
        let prompts = context.recursive_schedule.chunks[wave.start_chunk..wave.end_chunk]
            .iter()
            .map(|chunk| recursive_chunk_prompt(context.prompt, chunk))
            .collect::<Vec<_>>();
        let contexts = prompts
            .iter()
            .map(|prompt| context.with_prompt(prompt))
            .collect::<Vec<_>>();
        let expected_drafts = contexts.len();
        let wave_result = backend.generate_wave_cancelable_with_receipt(&contexts, should_cancel);
        let mut wave_drafts = wave_result.drafts;
        let cancelled = wave_result.cancelled;
        if wave_result.dispatch_width > 0 {
            execution_receipt.dispatched_waves =
                execution_receipt.dispatched_waves.saturating_add(1);
            execution_receipt.parallel_waves = execution_receipt
                .parallel_waves
                .saturating_add(usize::from(wave_result.dispatch_width > 1));
            execution_receipt.max_dispatch_width = execution_receipt
                .max_dispatch_width
                .max(wave_result.dispatch_width);
        }
        runtime_calls = runtime_calls.saturating_add(wave_drafts.len());
        if wave_drafts.len() > expected_drafts {
            return (
                recursive_wave_contract_error_draft(wave.wave, expected_drafts, wave_drafts.len()),
                runtime_calls,
                execution_receipt,
            );
        }
        if cancelled {
            return (
                wave_drafts.pop().unwrap_or_else(generation_cancelled_draft),
                runtime_calls,
                execution_receipt,
            );
        }
        if wave_drafts.len() != expected_drafts {
            return (
                recursive_wave_contract_error_draft(wave.wave, expected_drafts, wave_drafts.len()),
                runtime_calls,
                execution_receipt,
            );
        }
        chunk_drafts.append(&mut wave_drafts);
    }

    let mut merge_inputs = chunk_drafts
        .iter()
        .enumerate()
        .map(|(index, draft)| format!("chunk_{index}: {}", compact(&draft.answer, 600)))
        .collect::<Vec<_>>();
    let mut merge_drafts = Vec::new();

    for round in &context.recursive_schedule.merge_rounds {
        let groups = merge_inputs
            .chunks(context.recursive_schedule.merge_fan_in.max(2))
            .map(|items| items.join("\n"))
            .collect::<Vec<_>>();
        let mut next_inputs = Vec::new();

        for (group_index, group) in groups.iter().enumerate() {
            let prompt = recursive_merge_prompt(context.prompt, round.round, group_index, group);
            runtime_calls = runtime_calls.saturating_add(1);
            let draft = backend.generate_cancelable(context.with_prompt(&prompt), should_cancel);
            if should_cancel() {
                return (draft, runtime_calls, execution_receipt);
            }
            next_inputs.push(format!(
                "merge_r{}_g{}: {}",
                round.round,
                group_index,
                compact(&draft.answer, 600)
            ));
            merge_drafts.push(draft);
        }

        merge_inputs = next_inputs;
    }

    (
        merge_recursive_drafts(context.prompt, chunk_drafts, merge_drafts),
        runtime_calls,
        execution_receipt,
    )
}

pub(super) fn generate_with_recursive_schedule_stream_checked<B: InferenceBackend>(
    backend: &mut B,
    context: GenerationContext<'_>,
    on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
) -> (InferenceDraft, usize, RecursiveExecutionReceipt) {
    let mut never_cancel = || false;
    generate_with_recursive_schedule_stream_checked_cancelable(
        backend,
        context,
        on_token,
        &mut never_cancel,
    )
}

pub(super) fn generate_with_recursive_schedule_stream_checked_cancelable<B: InferenceBackend>(
    backend: &mut B,
    context: GenerationContext<'_>,
    on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
    should_cancel: &mut dyn FnMut() -> bool,
) -> (InferenceDraft, usize, RecursiveExecutionReceipt) {
    if !context.recursive_schedule.requires_recursion {
        return (
            backend.generate_stream_checked_cancelable(context, on_token, should_cancel),
            1,
            RecursiveExecutionReceipt::default(),
        );
    }

    let (draft, runtime_calls, execution_receipt) =
        generate_with_recursive_schedule_cancelable(backend, context, should_cancel);
    if should_cancel() {
        return (draft, runtime_calls, execution_receipt);
    }
    for token in &draft.tokens {
        if should_cancel() {
            return (
                generation_cancelled_draft(),
                runtime_calls,
                execution_receipt,
            );
        }
        if let Err(error) = on_token(token) {
            return (
                stream_observer_error_draft(error),
                runtime_calls,
                execution_receipt,
            );
        }
    }
    (draft, runtime_calls, execution_receipt)
}

fn recursive_chunk_prompt(prompt: &str, chunk: &RecursiveChunk) -> String {
    let chunk_text = prompt_chunk_text(prompt, chunk);
    format!(
        "Noiron recursive chunk {} covering estimated tokens {}..{} with left overlap {} and right overlap {}.\nOriginal prompt anchor: {}\nChunk text:\n{}\nTask: produce a concise, reusable chunk summary with key facts, constraints, and unresolved dependencies for later merge.",
        chunk.index,
        chunk.start_token,
        chunk.end_token,
        chunk.overlap_left,
        chunk.overlap_right,
        compact(prompt, 1_200),
        chunk_text
    )
}

fn prompt_chunk_text(prompt: &str, chunk: &RecursiveChunk) -> String {
    if prompt.chars().any(char::is_whitespace) {
        let words = prompt.split_whitespace().collect::<Vec<_>>();
        if chunk.end_token <= words.len() {
            return words
                .get(chunk.start_token..chunk.end_token)
                .unwrap_or(&[])
                .join(" ");
        }
    }

    let divisor = if prompt.is_ascii() { 4 } else { 2 };
    let start = chunk.start_token.saturating_mul(divisor);
    let end = chunk.end_token.saturating_mul(divisor);
    let text = prompt
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect::<String>();
    if text.is_empty() {
        compact(prompt, 1_200)
    } else {
        text
    }
}

fn recursive_merge_prompt(prompt: &str, round: usize, group_index: usize, group: &str) -> String {
    format!(
        "Noiron recursive merge round {round} group {group_index}.\nOriginal prompt anchor: {}\nChunk or prior-merge summaries:\n{group}\nTask: merge these summaries into one coherent answer fragment, preserve conflicts, and keep reusable long-context memory cues.",
        compact(prompt, 1_200)
    )
}

fn recursive_wave_contract_error_draft(
    wave: usize,
    expected_drafts: usize,
    actual_drafts: usize,
) -> InferenceDraft {
    let detail = format!(
        "recursive wave {wave} returned {actual_drafts} drafts for {expected_drafts} contexts"
    );
    InferenceDraft::new(
        format!("Runtime backend error: {detail}"),
        vec![ReasoningStep::new(
            "runtime_recursive_wave_contract_error",
            detail,
            0.0,
        )],
    )
}

fn merge_recursive_drafts(
    prompt: &str,
    chunk_drafts: Vec<InferenceDraft>,
    merge_drafts: Vec<InferenceDraft>,
) -> InferenceDraft {
    let final_answer = merge_drafts
        .last()
        .or_else(|| chunk_drafts.last())
        .map(|draft| draft.answer.clone())
        .unwrap_or_default();
    let answer = format!(
        "Recursive Noiron merged answer for '{}'. Final merge: {}",
        compact(prompt, 160),
        final_answer
    );
    let mut trace = vec![ReasoningStep::new(
        "recursive_runtime",
        format!(
            "executed {} chunk drafts and {} merge drafts",
            chunk_drafts.len(),
            merge_drafts.len()
        ),
        0.82,
    )];
    let mut tokens = Vec::new();
    let mut exported_kv_blocks = Vec::new();
    let mut diagnostics = Vec::new();

    for draft in chunk_drafts.iter().chain(merge_drafts.iter()) {
        trace.extend(draft.trace.clone());
        tokens.extend(draft.tokens.clone());
        exported_kv_blocks.extend(draft.exported_kv_blocks.clone());
        diagnostics.push(draft.runtime_diagnostics.clone());
    }

    InferenceDraft::new(answer, trace)
        .with_tokens(tokens)
        .with_exported_kv_blocks(exported_kv_blocks)
        .with_runtime_diagnostics(merge_runtime_diagnostics(&diagnostics))
}

fn merge_runtime_diagnostics(diagnostics: &[RuntimeDiagnostics]) -> RuntimeDiagnostics {
    let mut merged = RuntimeDiagnostics::default();
    let mut forward_energy_total = 0.0;
    let mut forward_energy_count = 0;
    let mut kv_influence_total = 0.0;
    let mut kv_influence_count = 0;
    let mut saw_device_execution_signal = false;
    let mut saw_control_plane_filled_device_execution = false;
    let mut all_device_execution_runtime_reported = true;

    for diagnostic in diagnostics {
        if merged.model_id.is_none() {
            merged.model_id = diagnostic.model_id.clone();
        }
        if merged.selected_adapter.is_none() {
            merged.selected_adapter = diagnostic.selected_adapter.clone();
        }
        merged.model_fallback_configured |= diagnostic.model_fallback_configured;
        merged.model_fallback_primary_failed |= diagnostic.model_fallback_primary_failed;
        merged.model_fallback_used |= diagnostic.model_fallback_used;
        merged.model_fallback_attempts = merged
            .model_fallback_attempts
            .saturating_add(diagnostic.model_fallback_attempts);
        merged.model_fallback_failures = merged
            .model_fallback_failures
            .saturating_add(diagnostic.model_fallback_failures);
        merged.model_fallback_quarantined = merged
            .model_fallback_quarantined
            .saturating_add(diagnostic.model_fallback_quarantined);
        merged.model_fallback_cooldown_skipped = merged
            .model_fallback_cooldown_skipped
            .saturating_add(diagnostic.model_fallback_cooldown_skipped);
        merged.model_fallback_all_failed |= diagnostic.model_fallback_all_failed;
        merge_runtime_diagnostic_text(&mut merged.device_profile, &diagnostic.device_profile);
        merge_runtime_diagnostic_text(&mut merged.primary_lane, &diagnostic.primary_lane);
        merge_runtime_diagnostic_text(&mut merged.fallback_lane, &diagnostic.fallback_lane);
        merge_runtime_diagnostic_text(&mut merged.memory_mode, &diagnostic.memory_mode);
        if diagnostic.has_device_execution_signal() {
            saw_device_execution_signal = true;
            saw_control_plane_filled_device_execution |=
                diagnostic.has_control_plane_filled_device_execution_signal();
            all_device_execution_runtime_reported &=
                diagnostic.has_runtime_reported_device_execution_signal();
        }
        merge_runtime_diagnostic_kv_precision(
            &mut merged.hot_kv_precision_bits,
            diagnostic.hot_kv_precision_bits,
        );
        merge_runtime_diagnostic_kv_precision(
            &mut merged.cold_kv_precision_bits,
            diagnostic.cold_kv_precision_bits,
        );
        merged.layer_count += diagnostic.layer_count;
        merged.global_layers += diagnostic.global_layers;
        merged.local_window_layers += diagnostic.local_window_layers;
        merged.convolutional_fusion_layers += diagnostic.convolutional_fusion_layers;
        merged.hidden_size = merged.hidden_size.max(diagnostic.hidden_size);
        merged.local_window_tokens = merged
            .local_window_tokens
            .max(diagnostic.local_window_tokens);
        merged.imported_kv_blocks += diagnostic.imported_kv_blocks;
        merged.weak_runtime_kv_imports_skipped += diagnostic.weak_runtime_kv_imports_skipped;
        merged.budget_limited_runtime_kv_imports_skipped +=
            diagnostic.budget_limited_runtime_kv_imports_skipped;
        merged.exported_kv_blocks += diagnostic.exported_kv_blocks;
        merged.runtime_kv_segments_included += diagnostic.runtime_kv_segments_included;
        merged.runtime_kv_segments_skipped += diagnostic.runtime_kv_segments_skipped;
        merged.runtime_kv_segments_rejected += diagnostic.runtime_kv_segments_rejected;

        if let Some(value) = diagnostic.forward_energy.filter(|value| value.is_finite()) {
            forward_energy_total += value;
            forward_energy_count += 1;
        }
        if let Some(value) = diagnostic.kv_influence.filter(|value| value.is_finite()) {
            kv_influence_total += value;
            kv_influence_count += 1;
        }
    }

    let mut selected_fallback_models = diagnostics.iter().filter_map(|diagnostic| {
        diagnostic
            .model_fallback_selected_model
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    });
    if let Some(first) = selected_fallback_models.next()
        && selected_fallback_models.all(|candidate| candidate == first)
    {
        merged.model_fallback_selected_model = Some(first.to_owned());
    }

    if let Some(diagnostic) = diagnostics
        .iter()
        .rev()
        .find(|diagnostic| diagnostic.adapter_cache_mode.is_some())
    {
        merged.adapter_cache_mode = diagnostic.adapter_cache_mode.clone();
    }
    if let Some(diagnostic) = diagnostics.iter().rev().find(|diagnostic| {
        diagnostic.has_adapter_stream_trace_signal()
            && diagnostic.has_adapter_stream_gate_summary_signal()
            && diagnostic.has_adapter_stream_write_gate_signal()
    }) {
        merged.adapter_stream_trace_id = diagnostic.adapter_stream_trace_id.clone();
        merged.adapter_stream_gate_summary_digest =
            diagnostic.adapter_stream_gate_summary_digest.clone();
        merged.adapter_stream_read_only = diagnostic.adapter_stream_read_only;
        merged.adapter_stream_write_allowed = diagnostic.adapter_stream_write_allowed;
        merged.adapter_stream_applied = diagnostic.adapter_stream_applied;
    }

    merged.forward_energy = average(forward_energy_total, forward_energy_count);
    merged.kv_influence = average(kv_influence_total, kv_influence_count);
    if !merged.has_valid_kv_precision_signal() {
        merged = merged.clear_kv_precision();
    }
    if merged.has_device_execution_signal() && saw_device_execution_signal {
        merged.device_execution_source = if all_device_execution_runtime_reported {
            Some(RuntimeDiagnostics::runtime_reported_device_execution_source().to_owned())
        } else if saw_control_plane_filled_device_execution {
            Some(RuntimeDiagnostics::control_plane_filled_device_execution_source().to_owned())
        } else {
            None
        };
    }
    merged
}

fn merge_runtime_diagnostic_text(merged: &mut Option<String>, next: &Option<String>) {
    let Some(next) = next.as_deref().filter(|value| !value.trim().is_empty()) else {
        return;
    };

    match merged.as_deref() {
        None => *merged = Some(next.to_owned()),
        Some(current) if current == next => {}
        Some(_) => *merged = None,
    }
}

fn merge_runtime_diagnostic_kv_precision(merged: &mut Option<u8>, next: Option<u8>) {
    let Some(next) = next.filter(|value| matches!(value, 4 | 8)) else {
        return;
    };

    match *merged {
        None => *merged = Some(next),
        Some(current) if current == next => {}
        Some(_) => *merged = None,
    }
}
