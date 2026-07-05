use super::*;
use crate::drift::{DriftReport, DriftSeverity};
use crate::memory_admission::{
    MemoryAdmissionInput, MemoryAdmissionPreview, TRACE_SEGMENT_REPLAY_PRIOR_SCHEMA,
};
use crate::process_reward::{ProcessRewardComponents, ProcessRewardReport, RewardAction};
use crate::reflection::{ReflectionIssue, ReflectionReport, ReflectionSeverity};

#[test]
fn issue_245_trace_segment_replay_prior_stays_digest_only_preview() {
    let preview = trace_segment_preview(
        "raw prompt should not appear in trace segment prior",
        clean_report(),
        ProcessRewardReport {
            total: 0.91,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Reinforce,
            notes: Vec::new(),
        },
        stable_drift(),
    );

    assert_eq!(preview.trace_segment_replay_prior_count(), 1);
    assert!(preview.is_read_only_preview());
    let prior = &preview.trace_segment_replay_priors[0];
    assert_eq!(prior.schema, TRACE_SEGMENT_REPLAY_PRIOR_SCHEMA);
    assert!(prior.proposed_for_retrieval);
    assert!(prior.blocked_reasons.is_empty());
    assert!(prior.input_hash.starts_with("fnv64:"));
    assert!(prior.prompt_digest.starts_with("fnv64:"));
    assert!(prior.router_decision_digest.starts_with("fnv64:"));
    assert!(prior.final_draft_digest.starts_with("fnv64:"));
    assert!(prior.source_scope_digest.starts_with("redaction-digest:"));
    assert_eq!(prior.source_scope_digest, prior.target_scope_digest);
    assert_eq!(prior.mobile_movement_review_digest, None);
    assert!(prior.process_reward_milli >= 900);
    assert!(prior.similarity_milli >= 800);
    assert!(!prior.scheduler_phase_ids.is_empty());
    assert!(
        prior
            .tool_call_ids
            .iter()
            .all(|id| id.starts_with("tool:redacted:"))
    );
    assert!(
        prior
            .source_trace_ids
            .iter()
            .any(|id| id.starts_with("trace:"))
    );
    assert!(!prior.write_allowed);
    assert!(!prior.applied);

    let summary = prior.summary();
    for marker in [
        "trace_segment_replay_prior",
        "router_decision_digest=fnv64:",
        "scheduler_phase_ids=",
        "tool_call_ids=tool:redacted:",
        "process_reward_milli=",
        "verifier=pass",
        "final_draft_digest=fnv64:",
        "source_trace_ids=",
        "source_scope=redaction-digest:",
        "target_scope=redaction-digest:",
        "mobile_movement_review=none",
        "rollback=memory_admission:coding:stable",
        "proposed_for_retrieval=true",
        "read_only=true",
        "write_allowed=false",
        "applied=false",
    ] {
        assert!(summary.contains(marker), "{summary}");
    }
    assert!(!summary.contains("raw prompt"));
    assert!(!summary.contains("raw answer"));
}

#[test]
fn issue_245_low_score_or_polluted_trace_prior_is_blocked() {
    let preview = trace_segment_preview(
        "polluted trace prompt should stay private",
        ReflectionReport {
            quality: 0.22,
            contradictions: vec!["conflicting unsafe answer".to_owned()],
            issues: vec![ReflectionIssue::new(
                "unsafe_trace",
                ReflectionSeverity::Critical,
                "critical trace issue",
            )],
            revision_actions: vec!["rollback".to_owned()],
            revision_passes: 1,
            revised_answer: "raw answer should not appear".to_owned(),
            store_as_memory: true,
            lesson: "do not replay polluted trace".to_owned(),
        },
        ProcessRewardReport {
            total: 0.18,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Penalize,
            notes: Vec::new(),
        },
        DriftReport {
            severity: DriftSeverity::Rollback,
            allow_memory_write: false,
            allow_runtime_kv_write: false,
            penalize_used_memory: true,
            rollback_adaptive: true,
            notes: Vec::new(),
        },
    );

    let prior = &preview.trace_segment_replay_priors[0];
    assert!(!prior.proposed_for_retrieval);
    for reason in [
        "trace_segment_quarantine",
        "trace_segment_verifier_reject",
        "trace_segment_reward_penalize",
        "trace_segment_rollback_requested",
        "trace_segment_critical_reflection",
        "trace_segment_contradiction",
        "trace_segment_similarity_below_threshold",
    ] {
        assert!(
            prior.blocked_reasons.contains(&reason.to_owned()),
            "{:?}",
            prior.blocked_reasons
        );
    }
    let summary = prior.summary();
    assert!(summary.contains("proposed_for_retrieval=false"));
    assert!(summary.contains("blockers=trace_segment_"));
    assert!(!summary.contains("raw answer"));
    assert!(!summary.contains("polluted trace prompt"));
}

#[test]
fn issue_245_trace_json_exposes_trace_segment_prior_contract() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("issue 245 trace segment replay", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "issue 245 trace segment replay",
        TaskProfile::Coding,
        5,
        &outcome,
    );

    let failures = evaluate_trace_schema_line(&line);
    assert!(failures.is_empty(), "{failures:?}");
    let admission = json_object_after_field(&line, "memory_admission").unwrap();
    assert_eq!(
        extract_json_usize_field(admission, "trace_segment_priors"),
        Some(1)
    );
    let summaries =
        extract_json_string_array_field(admission, "trace_segment_prior_summaries").unwrap();
    assert_eq!(summaries.len(), 1);
    let summary = &summaries[0];
    assert!(summary.contains(TRACE_SEGMENT_REPLAY_PRIOR_SCHEMA));
    assert!(summary.contains("router_decision_digest=fnv64:"));
    assert!(summary.contains("final_draft_digest=fnv64:"));
    assert!(summary.contains("source_trace_ids="));
    assert!(summary.contains("rollback="));
    assert!(summary.contains("read_only=true"));
    assert!(summary.contains("write_allowed=false"));
    assert!(summary.contains("applied=false"));
    assert!(!summary.contains("issue 245 trace segment replay"));
}

fn trace_segment_preview(
    prompt: &str,
    report: ReflectionReport,
    reward: ProcessRewardReport,
    drift: DriftReport,
) -> MemoryAdmissionPreview {
    MemoryAdmissionPreview::from_feedback(MemoryAdmissionInput {
        prompt,
        profile: TaskProfile::Coding,
        report: &report,
        process_reward: &reward,
        drift_report: &drift,
        stored_memory: true,
        gist_records: 0,
        stored_gist_memories: 0,
        imported_runtime_kv_blocks: 0,
        exported_runtime_kv_blocks: 0,
        stored_runtime_kv_memories: 0,
        weak_runtime_kv_imports_skipped: 0,
        runtime_kv_hold: false,
        runtime_kv_influence: None,
        budget_limited_runtime_kv_imports_skipped: 0,
        runtime_kv_segments_included: 0,
        runtime_kv_segments_skipped: 0,
        runtime_kv_segments_rejected: 0,
        used_memories: 1,
        memory_feedback_updates: 0,
        runtime_adapter_observations: 1,
        runtime_adapter_current_signal: true,
        runtime_adapter_selection_mismatch: false,
        runtime_adapter_best_score: Some(0.88),
        runtime_adapter_best_reward: Some(0.91),
        runtime_adapter_best_quality: Some(0.86),
        toolsmith_blueprints: 1,
        toolsmith_ready: 1,
        toolsmith_held: 0,
        toolsmith_rejected: 0,
        toolsmith_gate_passed: true,
        trace_segment_source_scope: None,
        trace_segment_target_scope: None,
        trace_segment_movement_review: None,
    })
}

fn clean_report() -> ReflectionReport {
    ReflectionReport {
        quality: 0.86,
        contradictions: Vec::new(),
        issues: Vec::new(),
        revision_actions: Vec::new(),
        revision_passes: 0,
        revised_answer: "raw answer should not appear".to_owned(),
        store_as_memory: true,
        lesson: "reuse digest-only trace prior".to_owned(),
    }
}

fn stable_drift() -> DriftReport {
    DriftReport {
        severity: DriftSeverity::Stable,
        allow_memory_write: true,
        allow_runtime_kv_write: true,
        penalize_used_memory: false,
        rollback_adaptive: false,
        notes: Vec::new(),
    }
}
