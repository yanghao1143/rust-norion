use crate::engine::NoironEngine;
use crate::experience::{ExperienceMatch, ExperienceRecord, recursive_runtime_calls_from_notes};
use crate::hardware::HardwarePlan;
use crate::hierarchy::TaskProfile;
use crate::runtime::RuntimeAdapterObservation;

pub(super) fn inspection_hardware_plan(engine: &NoironEngine) -> HardwarePlan {
    engine.hardware_allocator.plan(
        engine.hardware_snapshot,
        TaskProfile::General,
        1,
        engine.hierarchy.current(),
    )
}

pub(super) fn runtime_kv_precision_mismatch_count(
    engine: &NoironEngine,
    hardware_plan: &HardwarePlan,
) -> usize {
    engine
        .experience
        .records()
        .iter()
        .filter(|record| {
            let diagnostics = &record.runtime_diagnostics;
            diagnostics.has_device_execution_signal()
                && diagnostics.has_valid_kv_precision_signal()
                && (diagnostics.hot_kv_precision_bits
                    != Some(hardware_plan.execution.hot_kv_precision_bits)
                    || diagnostics.cold_kv_precision_bits
                        != Some(hardware_plan.execution.cold_kv_precision_bits))
        })
        .count()
}

pub(super) fn has_runtime_architecture_evidence(record: &ExperienceRecord) -> bool {
    record.runtime_diagnostics.has_runtime_architecture_signal()
        && record.runtime_diagnostics.has_valid_kv_precision_signal()
}

pub(super) fn runtime_adapter_selection_mismatch_count(
    engine: &NoironEngine,
    hardware_plan: &HardwarePlan,
) -> usize {
    let matches = runtime_adapter_experience_matches(engine);
    let observations =
        RuntimeAdapterObservation::from_experiences_for_hardware(&matches, "", hardware_plan);
    let Some(best_adapter) = observations
        .iter()
        .filter(|observation| observation.score >= 0.50)
        .map(|observation| observation.adapter.as_str())
        .next()
    else {
        return 0;
    };

    let Some(selected_adapter) =
        latest_runtime_selected_adapter_for_hardware(engine, hardware_plan)
    else {
        return 1;
    };

    usize::from(selected_adapter != best_adapter)
}

fn latest_runtime_selected_adapter_for_hardware<'a>(
    engine: &'a NoironEngine,
    hardware_plan: &HardwarePlan,
) -> Option<&'a str> {
    engine
        .experience
        .records()
        .iter()
        .rev()
        .filter(|record| record_matches_hardware_plan(record, hardware_plan))
        .filter_map(|record| record.runtime_diagnostics.selected_adapter.as_deref())
        .find(|adapter| {
            hardware_plan
                .execution
                .adapter_hints
                .iter()
                .any(|hint| hint.as_str() == *adapter)
        })
}

fn runtime_adapter_experience_matches(engine: &NoironEngine) -> Vec<ExperienceMatch> {
    engine
        .experience
        .records()
        .iter()
        .filter_map(|record| {
            let selected_adapter = record.runtime_diagnostics.selected_adapter.clone()?;
            Some(ExperienceMatch {
                id: record.id,
                prompt: record.prompt.clone(),
                lesson: record.lesson.clone(),
                quality: record.quality,
                score: runtime_adapter_record_score(record),
                gist_hints: Vec::new(),
                reflection_issue_codes: Vec::new(),
                revision_actions: record.revision_actions.clone(),
                process_reward: record.process_reward.total,
                reward_action: record.process_reward.action,
                runtime_model_id: record.runtime_diagnostics.model_id.clone(),
                runtime_selected_adapter: Some(selected_adapter),
                runtime_device_profile: record.runtime_diagnostics.device_profile.clone(),
                runtime_primary_lane: record.runtime_diagnostics.primary_lane.clone(),
                runtime_fallback_lane: record.runtime_diagnostics.fallback_lane.clone(),
                runtime_memory_mode: record.runtime_diagnostics.memory_mode.clone(),
                runtime_device_execution_source: record
                    .runtime_diagnostics
                    .device_execution_source
                    .clone(),
                runtime_forward_energy: record.runtime_diagnostics.forward_energy,
                runtime_kv_influence: record.runtime_diagnostics.kv_influence,
                runtime_uncertainty_perplexity: record.runtime_token_metrics.uncertainty_perplexity,
                recursive_runtime_calls: recursive_runtime_calls_from_notes(
                    &record.process_reward.notes,
                ),
            })
        })
        .collect()
}

fn runtime_adapter_record_score(record: &ExperienceRecord) -> f32 {
    let reward_bonus = record.process_reward.total.clamp(0.0, 1.0) * 0.20;
    let issue_penalty = (record.reflection_issues.len() as f32 * 0.03).min(0.18);
    let contradiction_penalty = (record.contradictions.len() as f32 * 0.05).min(0.25);
    (record.quality * 0.80 + reward_bonus - issue_penalty - contradiction_penalty).clamp(0.0, 1.0)
}

fn record_matches_hardware_plan(record: &ExperienceRecord, hardware_plan: &HardwarePlan) -> bool {
    let diagnostics = &record.runtime_diagnostics;
    runtime_diagnostic_matches(
        diagnostics.device_profile.as_deref(),
        hardware_plan.device.as_str(),
    ) && runtime_diagnostic_matches(
        diagnostics.primary_lane.as_deref(),
        hardware_plan.execution.primary_lane.as_str(),
    ) && runtime_diagnostic_matches(
        diagnostics.fallback_lane.as_deref(),
        hardware_plan.execution.fallback_lane.as_str(),
    ) && runtime_diagnostic_matches(
        diagnostics.memory_mode.as_deref(),
        hardware_plan.execution.memory_mode.as_str(),
    )
}

fn runtime_diagnostic_matches(actual: Option<&str>, expected: &str) -> bool {
    actual.map(|actual| actual == expected).unwrap_or(true)
}

pub(super) fn has_text(value: Option<&str>) -> bool {
    value.map(|value| !value.trim().is_empty()).unwrap_or(false)
}
