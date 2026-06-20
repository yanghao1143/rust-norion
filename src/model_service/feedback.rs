use rust_norion::{MemoryUpdateReport, NoironEngine, RewardAction, RustSnippetCheckReport};

use super::request::{ModelServiceFeedbackRequest, ModelServiceRustCheckRequest};

pub(crate) fn model_service_feedback_memory_ids(
    engine: &NoironEngine,
    request: &ModelServiceFeedbackRequest,
) -> Vec<u64> {
    let mut memory_ids = Vec::new();
    if let Some(memory_id) = request.memory_id {
        push_unique_u64(&mut memory_ids, memory_id);
    }
    if let Some(experience_id) = request.experience_id
        && let Some(record) = engine
            .experience
            .records()
            .iter()
            .find(|record| record.id == experience_id)
    {
        if let Some(memory_id) = record.stored_memory_id {
            push_unique_u64(&mut memory_ids, memory_id);
        }
        for memory_id in &record.used_memory_ids {
            push_unique_u64(&mut memory_ids, *memory_id);
        }
        for memory_id in &record.gist_memory_ids {
            push_unique_u64(&mut memory_ids, *memory_id);
        }
        for memory_id in &record.stored_runtime_kv_memory_ids {
            push_unique_u64(&mut memory_ids, *memory_id);
        }
    }
    memory_ids
}

pub(crate) fn apply_model_service_feedback(
    engine: &mut NoironEngine,
    request: &ModelServiceFeedbackRequest,
    memory_ids: &[u64],
) -> Vec<MemoryUpdateReport> {
    memory_ids
        .iter()
        .map(|memory_id| match request.action {
            RewardAction::Reinforce => engine.cache.reinforce(*memory_id, request.amount),
            RewardAction::Penalize => engine.cache.penalize(*memory_id, request.amount),
            RewardAction::Hold => unreachable!("feedback parser rejects hold actions"),
        })
        .collect()
}

pub(crate) fn annotate_model_service_feedback_experience(
    engine: &mut NoironEngine,
    request: &ModelServiceFeedbackRequest,
    updates: &[MemoryUpdateReport],
) -> bool {
    annotate_model_service_feedback_experience_with_source(engine, request, updates, "external")
}

pub(crate) fn annotate_model_service_feedback_experience_with_source(
    engine: &mut NoironEngine,
    request: &ModelServiceFeedbackRequest,
    updates: &[MemoryUpdateReport],
    source: &str,
) -> bool {
    let Some(experience_id) = request.experience_id else {
        return false;
    };
    let note = model_service_feedback_note(source, request, updates);
    let Some(record) = engine.experience.record_mut(experience_id) else {
        return false;
    };
    record.process_reward.notes.insert(0, note);
    true
}

pub(crate) fn annotate_model_service_rust_check_experience(
    engine: &mut NoironEngine,
    request: &ModelServiceRustCheckRequest,
    report: &RustSnippetCheckReport,
) -> bool {
    let Some(experience_id) = request.experience_id else {
        return false;
    };
    let Some(record) = engine.experience.record_mut(experience_id) else {
        return false;
    };
    record
        .process_reward
        .notes
        .insert(0, model_service_rust_check_note(report));
    true
}

pub(crate) fn model_service_rust_check_feedback_request(
    request: &ModelServiceRustCheckRequest,
    report: &RustSnippetCheckReport,
) -> ModelServiceFeedbackRequest {
    ModelServiceFeedbackRequest {
        action: if report.passed {
            RewardAction::Reinforce
        } else {
            RewardAction::Penalize
        },
        amount: request
            .amount
            .unwrap_or(if report.passed { 0.45 } else { 0.35 }),
        experience_id: request.experience_id,
        memory_id: request.memory_id,
    }
}

fn model_service_feedback_note(
    source: &str,
    request: &ModelServiceFeedbackRequest,
    updates: &[MemoryUpdateReport],
) -> String {
    let reinforced = usize::from(request.action == RewardAction::Reinforce) * updates.len();
    let penalized = usize::from(request.action == RewardAction::Penalize) * updates.len();
    let reinforcement_amount = if request.action == RewardAction::Reinforce {
        updates
            .iter()
            .map(|update| update.requested_amount.max(0.0))
            .sum::<f32>()
    } else {
        0.0
    };
    let penalty_amount = if request.action == RewardAction::Penalize {
        updates
            .iter()
            .map(|update| update.requested_amount.max(0.0))
            .sum::<f32>()
    } else {
        0.0
    };
    let applied = updates.iter().filter(|update| update.was_applied()).count();
    let removed = updates.iter().filter(|update| update.removed).count();
    let missing = updates.len().saturating_sub(applied);
    let strength_delta = updates
        .iter()
        .map(|update| update.strength_delta.abs())
        .sum::<f32>();

    format!(
        "memory_feedback:{source}:reinforced={reinforced}:penalized={penalized}:reinforcement_amount={reinforcement_amount:.6}:penalty_amount={penalty_amount:.6}:applied={applied}:removed={removed}:missing={missing}:strength_delta={strength_delta:.6}"
    )
}

fn model_service_rust_check_note(report: &RustSnippetCheckReport) -> String {
    let status_code = report
        .status_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "rust_check:passed={}:label={}:edition={}:status_code={}:diagnostic_chars={}",
        report.passed,
        report.feedback_label(),
        report.edition,
        status_code,
        report.diagnostic_chars()
    )
}

fn push_unique_u64(values: &mut Vec<u64>, value: u64) {
    if !values.contains(&value) {
        values.push(value);
    }
}
