use crate::gist_memory::GistRecord;
use crate::hardware::RuntimeAdapterHint;
use crate::hierarchy::TaskProfile;
use crate::reflection::{ReflectionIssue, ReflectionSeverity, RuntimeDiagnostics};
use crate::{
    DevelopmentEvidenceUseSurface, DevelopmentPollutionEvent,
    admit_development_evidence_for_current_use, classify_development_pollution_event,
    gate_development_evidence_surface,
};

use super::evidence::evidence_notes_by_kind;
use super::hygiene::cross_task_transcript_pollution;
use super::index::record_index_document;
use super::model::{ExperienceMatch, ExperienceRecord, ExperienceRetrievalReport};
use super::noise::{ExperienceRetrievalNoise, retrieval_noise, transcript_anchor_penalty};
use super::relevance::{lexical_overlap, task_anchor_penalty};

pub(super) fn retrieve_lessons(
    records: &[ExperienceRecord],
    prompt: &str,
    profile: TaskProfile,
    limit: usize,
) -> Vec<ExperienceMatch> {
    retrieve_report(records, prompt, profile, limit).matches
}

pub(super) fn retrieve_report(
    records: &[ExperienceRecord],
    prompt: &str,
    profile: TaskProfile,
    limit: usize,
) -> ExperienceRetrievalReport {
    let limit = limit.max(1);
    let mut skipped_cross_task_pollution = 0usize;
    let mut development_evidence_surface_blocked_candidates = 0usize;
    let mut retrieval_noise_penalized_candidates = 0usize;
    let mut retrieval_noise_filtered_candidates = 0usize;
    let mut suppressed_prompt_index_candidates = 0usize;
    let mut max_retrieval_noise_penalty = 0.0f32;
    let mut matches = Vec::new();

    for record in records {
        if cross_task_transcript_pollution(record, prompt) {
            skipped_cross_task_pollution += 1;
            continue;
        }
        if development_evidence_retrieval_blocked(record) {
            development_evidence_surface_blocked_candidates += 1;
            continue;
        }
        let gist_text = record
            .gist_records
            .iter()
            .map(|gist| format!("{} {}", gist.title, gist.summary))
            .collect::<Vec<_>>()
            .join(" ");
        let reflection_text = record
            .reflection_issues
            .iter()
            .map(|issue| format!("{} {}", issue.code, issue.detail))
            .chain(record.revision_actions.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");
        let runtime_text = runtime_diagnostics_text(&record.runtime_diagnostics);
        let recursive_runtime_calls =
            recursive_runtime_calls_from_notes(&record.process_reward.notes);
        let recursive_text = recursive_runtime_calls
            .map(|calls| format!("recursive_runtime_calls {calls}"))
            .unwrap_or_default();
        let index_document = record_index_document(record);
        let retrieval_noise = retrieval_noise(record);
        if retrieval_noise.suppress_prompt_index() {
            suppressed_prompt_index_candidates += 1;
        }
        let retrieval_noise_penalty = retrieval_noise.penalty();
        let observable_noise_penalty = index_document.noise_penalty + retrieval_noise_penalty;
        if observable_noise_penalty > 0.0 {
            retrieval_noise_penalized_candidates += 1;
            max_retrieval_noise_penalty = max_retrieval_noise_penalty.max(observable_noise_penalty);
        }
        let signal_text = retrieval_signal_text(
            record,
            &index_document.text,
            &gist_text,
            &reflection_text,
            &runtime_text,
            &recursive_text,
            retrieval_noise,
        );
        let overlap = lexical_overlap(prompt, &signal_text);
        let profile_bonus = if record.profile == profile { 0.16 } else { 0.0 };
        let gist_bonus = record
            .gist_records
            .iter()
            .map(|gist| gist.importance)
            .fold(0.0, f32::max)
            * 0.08;
        let reward_bonus = record.process_reward.total * 0.08;
        let contradiction_penalty = (record.contradictions.len() as f32 * 0.08).min(0.32);
        let reflection_penalty = reflection_issue_penalty(&record.reflection_issues);
        let effective_quality = record.quality * retrieval_noise.effective_quality_multiplier();
        let anchor_penalty = transcript_anchor_penalty(&record.prompt, prompt);
        let task_penalty = task_anchor_penalty(prompt, &signal_text);
        let metadata_prompt_penalty =
            metadata_prompt_anchor_penalty(prompt, &record.prompt, retrieval_noise);
        let score =
            (overlap * 0.52 + effective_quality * 0.36 + profile_bonus + gist_bonus + reward_bonus
                - index_document.noise_penalty
                - retrieval_noise_penalty
                - anchor_penalty
                - task_penalty
                - metadata_prompt_penalty
                - contradiction_penalty
                - reflection_penalty)
                .clamp(0.0, 1.0);

        if score < 0.12 {
            if observable_noise_penalty > 0.0 {
                retrieval_noise_filtered_candidates += 1;
            }
            continue;
        }

        matches.push(ExperienceMatch {
            id: record.id,
            prompt: record.prompt.clone(),
            lesson: record.lesson.clone(),
            quality: record.quality,
            score,
            gist_hints: record
                .gist_records
                .iter()
                .take(3)
                .map(GistRecord::hint)
                .collect(),
            reflection_issue_codes: record
                .reflection_issues
                .iter()
                .map(|issue| issue.code.clone())
                .collect(),
            revision_actions: record.revision_actions.clone(),
            process_reward: record.process_reward.total,
            reward_action: record.process_reward.action,
            used_memory_count: record.used_memory_ids.len(),
            route_threshold: record.route_budget.threshold,
            route_attention_tokens: record.route_budget.attention_tokens,
            route_fast_tokens: record.route_budget.fast_tokens,
            route_attention_fraction: record.route_budget.attention_fraction,
            runtime_model_id: record.runtime_diagnostics.model_id.clone(),
            runtime_selected_adapter: record
                .runtime_diagnostics
                .selected_adapter
                .as_deref()
                .and_then(RuntimeAdapterHint::canonical_name)
                .map(str::to_owned),
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
            recursive_runtime_calls,
        });
    }

    matches.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matches.truncate(limit);

    ExperienceRetrievalReport {
        prompt: prompt.to_owned(),
        profile,
        total_records: records.len(),
        requested_limit: limit,
        skipped_cross_task_pollution,
        development_evidence_surface_blocked_candidates,
        retrieval_noise_penalized_candidates,
        retrieval_noise_filtered_candidates,
        suppressed_prompt_index_candidates,
        max_retrieval_noise_penalty,
        matches,
    }
}

fn development_evidence_retrieval_blocked(record: &ExperienceRecord) -> bool {
    let Some(reason) = development_pollution_reason_for_record(record) else {
        return false;
    };
    let payload = [
        record.prompt.as_str(),
        record.lesson.as_str(),
        &record.process_reward.notes.join(" "),
        &record.revision_actions.join(" "),
    ]
    .join(" ");
    let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
        format!("experience-{}", record.id),
        "experience_record",
        payload,
        reason,
    ));
    let admission = admit_development_evidence_for_current_use(&finding);
    !gate_development_evidence_surface(
        &admission,
        DevelopmentEvidenceUseSurface::ExperienceRetrieval,
    )
    .allowed
}

fn development_pollution_reason_for_record(record: &ExperienceRecord) -> Option<&'static str> {
    let text = [
        record.prompt.as_str(),
        record.lesson.as_str(),
        &record.process_reward.notes.join(" "),
        &record.revision_actions.join(" "),
    ]
    .join(" ")
    .to_ascii_lowercase();
    [
        "development_evidence_contamination",
        "reasoning_genome_hygiene_violation",
        "stale_or_polluted_claim",
        "polluted_claim",
    ]
    .into_iter()
    .find(|reason| text.contains(reason))
}

fn retrieval_signal_text(
    record: &ExperienceRecord,
    index_text: &str,
    gist_text: &str,
    reflection_text: &str,
    runtime_text: &str,
    recursive_text: &str,
    retrieval_noise: ExperienceRetrievalNoise,
) -> String {
    if retrieval_noise.suppress_prompt_index() {
        let lesson_text = if retrieval_noise.metadata_lesson_like {
            ""
        } else {
            &record.lesson
        };
        format!(
            "lesson:{} gist:{} reflection:{} runtime:{} recursive:{}",
            lesson_text, gist_text, reflection_text, runtime_text, recursive_text
        )
    } else {
        format!(
            "{} {} {} {} {}",
            index_text, gist_text, reflection_text, runtime_text, recursive_text
        )
    }
}

fn metadata_prompt_anchor_penalty(
    prompt: &str,
    record_prompt: &str,
    retrieval_noise: ExperienceRetrievalNoise,
) -> f32 {
    if retrieval_noise.metadata_lesson_like {
        task_anchor_penalty(prompt, record_prompt) * 0.60
    } else {
        0.0
    }
}

fn runtime_diagnostics_text(diagnostics: &RuntimeDiagnostics) -> String {
    let mut parts = [
        diagnostics.model_id.as_deref().unwrap_or_default(),
        diagnostics
            .selected_adapter
            .as_deref()
            .and_then(RuntimeAdapterHint::canonical_name)
            .unwrap_or_default(),
        diagnostics.device_profile.as_deref().unwrap_or_default(),
        diagnostics.primary_lane.as_deref().unwrap_or_default(),
        diagnostics.fallback_lane.as_deref().unwrap_or_default(),
        diagnostics.memory_mode.as_deref().unwrap_or_default(),
        diagnostics
            .device_execution_source
            .as_deref()
            .unwrap_or_default(),
    ]
    .into_iter()
    .filter(|item| !item.is_empty())
    .map(ToOwned::to_owned)
    .collect::<Vec<_>>();
    if diagnostics.has_valid_kv_precision_signal() {
        parts.push(format!(
            "kv_bits={}/{}",
            diagnostics.hot_kv_precision_bits.unwrap_or_default(),
            diagnostics.cold_kv_precision_bits.unwrap_or_default()
        ));
    }
    parts.join(" ")
}

pub fn recursive_runtime_calls_from_notes(notes: &[String]) -> Option<usize> {
    evidence_notes_by_kind(notes, "recursive")
        .filter_map(|note| note.field_positive_usize("runtime_calls"))
        .max()
        .or_else(|| {
            evidence_notes_by_kind(notes, "latency")
                .filter_map(|note| note.field_positive_usize("recursive_runtime_calls"))
                .max()
        })
}

fn reflection_issue_penalty(issues: &[ReflectionIssue]) -> f32 {
    issues
        .iter()
        .map(|issue| match issue.severity {
            ReflectionSeverity::Info => 0.01,
            ReflectionSeverity::Warning => 0.04,
            ReflectionSeverity::Critical => 0.14,
        })
        .sum::<f32>()
        .min(0.36)
}
