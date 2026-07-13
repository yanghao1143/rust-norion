use rust_norion::{
    MemoryUpdateReport, NoironEngine, ReflectionIssue, ReflectionSeverity, RewardAction,
    RustSnippetCheckReport, TenantScope,
};

use super::request::{ModelServiceFeedbackRequest, ModelServiceRustCheckRequest};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelServiceExperienceFeedbackUpdate {
    pub(crate) applied: bool,
    pub(crate) reward_before: f32,
    pub(crate) reward_after: f32,
    pub(crate) reward_delta: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceBehaviorModelOutcomeUpdate {
    pub(crate) applied: bool,
    pub(crate) ok: bool,
    pub(crate) model: String,
    pub(crate) task_kind: String,
    pub(crate) error: Option<String>,
}

pub(crate) fn model_service_feedback_memory_ids(
    engine: &NoironEngine,
    request: &ModelServiceFeedbackRequest,
) -> Vec<u64> {
    let Some(scope) = request.tenant_scope.as_ref() else {
        return Vec::new();
    };
    let visible_memory_ids = scoped_memory_ids(engine, scope);
    let mut memory_ids = Vec::new();
    if let Some(memory_id) = request.memory_id {
        push_visible_unique_u64(&mut memory_ids, memory_id, &visible_memory_ids);
    }
    if let Some(experience_id) = request.experience_id
        && let Some(record) = engine
            .experience
            .records()
            .iter()
            .find(|record| record.id == experience_id)
    {
        if let Some(memory_id) = record.stored_memory_id {
            push_visible_unique_u64(&mut memory_ids, memory_id, &visible_memory_ids);
        }
        for memory_id in &record.used_memory_ids {
            push_visible_unique_u64(&mut memory_ids, *memory_id, &visible_memory_ids);
        }
        for memory_id in &record.gist_memory_ids {
            push_visible_unique_u64(&mut memory_ids, *memory_id, &visible_memory_ids);
        }
        for memory_id in &record.stored_runtime_kv_memory_ids {
            push_visible_unique_u64(&mut memory_ids, *memory_id, &visible_memory_ids);
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

pub(crate) fn apply_model_service_behavior_feedback(
    engine: &mut NoironEngine,
    request: &ModelServiceFeedbackRequest,
) -> Option<ModelServiceExperienceFeedbackUpdate> {
    let experience_id = request.experience_id?;
    let record = engine.experience.record_mut(experience_id)?;
    let reward_before = record.process_reward.total;
    let reward_after = match request.action {
        RewardAction::Reinforce => (reward_before + request.amount).clamp(0.0, 1.0),
        RewardAction::Penalize => (reward_before - request.amount).clamp(0.0, 1.0),
        RewardAction::Hold => return None,
    };
    record.process_reward.total = reward_after;
    record.process_reward.components.reflection = match request.action {
        RewardAction::Reinforce => {
            (record.process_reward.components.reflection + request.amount).clamp(0.0, 1.0)
        }
        RewardAction::Penalize => {
            (record.process_reward.components.reflection - request.amount).clamp(0.0, 1.0)
        }
        RewardAction::Hold => unreachable!("feedback parser rejects hold actions"),
    };
    record.process_reward.action = request.action;
    record
        .reflection_issues
        .retain(|issue| issue.code != "generated_code_behavior_unverified");
    let evidence = request.evidence.as_deref().unwrap_or("none");
    match request.action {
        RewardAction::Reinforce => {
            if !record
                .revision_actions
                .iter()
                .any(|action| action == "generated_code_behavior_validated")
            {
                record
                    .revision_actions
                    .push("generated_code_behavior_validated".to_owned());
            }
        }
        RewardAction::Penalize => {
            record.reflection_issues.push(ReflectionIssue::new(
                "generated_code_behavior_failed",
                ReflectionSeverity::Critical,
                evidence,
            ));
            if !record
                .revision_actions
                .iter()
                .any(|action| action == "repair_generated_code_from_behavior_evidence")
            {
                record
                    .revision_actions
                    .push("repair_generated_code_from_behavior_evidence".to_owned());
            }
        }
        RewardAction::Hold => unreachable!("feedback parser rejects hold actions"),
    }
    let source = request.source.as_deref().unwrap_or("external");
    record.process_reward.notes.insert(
        0,
        format!(
            "behavior_feedback:{source}:action={}:amount={:.6}:reward_before={reward_before:.6}:reward_after={reward_after:.6}:evidence={evidence}",
            request.action.as_str(),
            request.amount,
        ),
    );
    Some(ModelServiceExperienceFeedbackUpdate {
        applied: true,
        reward_before,
        reward_after,
        reward_delta: reward_after - reward_before,
    })
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
        capability_token: None,
        source: Some("rust_check".to_owned()),
        evidence: None,
        tenant_scope: request.tenant_scope.clone(),
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

fn push_visible_unique_u64(values: &mut Vec<u64>, value: u64, visible_memory_ids: &[u64]) {
    if visible_memory_ids.contains(&value) {
        push_unique_u64(values, value);
    }
}

fn scoped_memory_ids(engine: &NoironEngine, scope: &TenantScope) -> Vec<u64> {
    engine
        .cache
        .entries_scoped(scope)
        .into_iter()
        .map(|entry| entry.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use rust_norion::{HeuristicBackend, InferenceRequest, TaskProfile, TenantResourceLane};

    use super::*;

    #[test]
    fn feedback_memory_ids_filter_cross_scope_targets() {
        let mut engine = NoironEngine::new();
        let tenant_a = TenantScope::new("tenant-a", "workspace", "session-a");
        let tenant_b = TenantScope::new("tenant-b", "workspace", "session-b");
        let memory_a = engine.cache.store_scoped_or_fuse(
            &tenant_a,
            TenantResourceLane::KvMemory,
            "memory-a",
            vec![1.0, 0.0],
            0.8,
        );
        let memory_b = engine.cache.store_scoped_or_fuse(
            &tenant_b,
            TenantResourceLane::KvMemory,
            "memory-b",
            vec![0.0, 1.0],
            0.8,
        );

        let same_scope = ModelServiceFeedbackRequest {
            action: RewardAction::Reinforce,
            amount: 0.5,
            experience_id: None,
            memory_id: Some(memory_a),
            capability_token: None,
            source: None,
            evidence: None,
            tenant_scope: Some(tenant_a.clone()),
        };
        assert_eq!(
            model_service_feedback_memory_ids(&engine, &same_scope),
            vec![memory_a]
        );

        let cross_scope = ModelServiceFeedbackRequest {
            memory_id: Some(memory_b),
            ..same_scope
        };
        assert!(model_service_feedback_memory_ids(&engine, &cross_scope).is_empty());
    }

    #[test]
    fn behavior_feedback_updates_experience_reward_and_failure_evidence() {
        let mut engine = NoironEngine::new();
        let scope = TenantScope::new("tenant", "workspace", "session");
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("generate gomoku", TaskProfile::Coding)
                .with_tenant_scope(scope.clone()),
            &mut backend,
        );
        let request = ModelServiceFeedbackRequest {
            action: RewardAction::Penalize,
            amount: 0.4,
            experience_id: Some(outcome.experience_id),
            memory_id: None,
            capability_token: Some("token".to_owned()),
            source: Some("browser_behavior_validation".to_owned()),
            evidence: Some("gomoku_premature_winner_after_four_white".to_owned()),
            tenant_scope: Some(scope),
        };

        let before = engine
            .experience
            .records()
            .iter()
            .find(|record| record.id == outcome.experience_id)
            .unwrap()
            .process_reward
            .total;
        let update = apply_model_service_behavior_feedback(&mut engine, &request).unwrap();
        let record = engine
            .experience
            .records()
            .iter()
            .find(|record| record.id == outcome.experience_id)
            .unwrap();

        assert!(update.applied);
        assert!(update.reward_after < before);
        assert_eq!(record.process_reward.action, RewardAction::Penalize);
        assert!(record.reflection_issues.iter().any(|issue| {
            issue.code == "generated_code_behavior_failed"
                && issue.detail == "gomoku_premature_winner_after_four_white"
        }));
        assert!(
            record.process_reward.notes[0]
                .starts_with("behavior_feedback:browser_behavior_validation:action=penalize")
        );
    }
}
