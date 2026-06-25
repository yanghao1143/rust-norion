use super::*;
use crate::kv_exchange::RuntimeKvBlock;

#[test]
fn empty_answer_is_rejected() {
    let report = Reflector::new().reflect("build a router", &InferenceDraft::new("", vec![]));

    assert!(!report.store_as_memory);
    assert!(
        report
            .contradictions
            .iter()
            .any(|item| item == "empty_answer")
    );
    assert!(report.issues.iter().any(
        |issue| issue.code == "empty_answer" && issue.severity == ReflectionSeverity::Critical
    ));
    assert!(
        report
            .revision_actions
            .iter()
            .any(|action| action == "reject_empty_answer")
    );
}

#[test]
fn useful_answer_can_be_stored() {
    let draft = InferenceDraft::new(
        "Build a Rust router that observes quality metrics and adjusts the entropy threshold.",
        vec![ReasoningStep::new("plan", "route by entropy", 0.9)],
    );

    let report = Reflector::new().reflect("Rust router quality metrics", &draft);

    assert!(report.quality > 0.46);
    assert!(report.store_as_memory);
    assert!(report.lesson.contains("reuse_response:"));
    assert!(report.lesson.contains("Rust router"));
    assert!(!report.lesson.contains("accepted_pattern"));
}

#[test]
fn short_low_risk_answer_gets_repaired_and_rechecked() {
    let draft = InferenceDraft::new(
        "Rust routes.",
        vec![ReasoningStep::new("draft", "short but grounded", 0.86)],
    );

    let report = Reflector::new().reflect("Explain Rust Noiron adaptive routing decisions", &draft);

    assert_eq!(report.revision_passes, 1);
    assert!(report.revised_answer.contains("Reflection repair"));
    assert!(
        report
            .revision_actions
            .iter()
            .any(|action| action == "expand_short_answer")
    );
    assert!(
        report
            .revision_actions
            .iter()
            .any(|action| action == "reflection_repair_applied")
    );
    assert!(report.quality > 0.46);
}

#[test]
fn conflicting_and_uncertain_draft_gets_structured_actions() {
    let mut token = DraftToken::new("maybe");
    token.entropy = Some(3.6);
    token.logprob = Some(-3.4);
    let draft = InferenceDraft::new(
        "The result is certain and guaranteed, but maybe unknown. repeat repeat repeat repeat repeat repeat.",
        vec![ReasoningStep::new("verify", "weak evidence", 0.10)],
    )
    .with_tokens(vec![token]);

    let report = Reflector::new().reflect("verify result carefully", &draft);

    assert!(!report.store_as_memory);
    assert_eq!(report.revision_passes, 0);
    assert!(report.critical_issue_count() >= 2);
    assert!(
        report
            .issue_codes()
            .iter()
            .any(|code| code == "conflicting_certainty_markers")
    );
    assert!(
        report
            .revision_actions
            .iter()
            .any(|action| action == "increase_attention_or_resample")
    );
    assert!(report.revised_answer.contains("Reflection note"));
}

#[test]
fn exported_kv_builder_syncs_runtime_diagnostics_count() {
    let draft = InferenceDraft::new(
        "Runtime KV blocks exported by a backend must match diagnostics for trace gates.",
        vec![ReasoningStep::new(
            "runtime",
            "exported two KV blocks",
            0.91,
        )],
    )
    .with_exported_kv_blocks(vec![
        RuntimeKvBlock::new(1, 0, 0, 1, vec![0.1], vec![0.2]),
        RuntimeKvBlock::new(1, 1, 1, 2, vec![0.3], vec![0.4]),
    ])
    .with_runtime_diagnostics(RuntimeDiagnostics {
        exported_kv_blocks: 99,
        ..RuntimeDiagnostics::default()
    });

    assert_eq!(draft.exported_kv_blocks.len(), 2);
    assert_eq!(draft.runtime_diagnostics.exported_kv_blocks, 2);
}

#[test]
fn runtime_kv_activity_counts_as_forward_signal_without_promoting_weak_skip_to_exchange() {
    let imported = RuntimeDiagnostics {
        imported_kv_blocks: 1,
        ..RuntimeDiagnostics::default()
    };
    assert_eq!(imported.runtime_kv_activity_count(), 1);
    assert!(imported.has_runtime_kv_exchange_signal());
    assert!(imported.has_runtime_kv_activity_signal());
    assert!(imported.has_forward_signal());

    let exported = RuntimeDiagnostics {
        exported_kv_blocks: 2,
        ..RuntimeDiagnostics::default()
    };
    assert_eq!(exported.runtime_kv_activity_count(), 2);
    assert!(exported.has_runtime_kv_exchange_signal());
    assert!(exported.has_runtime_kv_activity_signal());
    assert!(exported.has_forward_signal());

    let weak_skip = RuntimeDiagnostics {
        weak_runtime_kv_imports_skipped: 3,
        ..RuntimeDiagnostics::default()
    };
    assert_eq!(weak_skip.runtime_kv_activity_count(), 3);
    assert!(!weak_skip.has_runtime_kv_exchange_signal());
    assert!(weak_skip.has_runtime_kv_activity_signal());
    assert!(weak_skip.has_forward_signal());

    let segment_signal = RuntimeDiagnostics {
        runtime_kv_segments_included: 1,
        runtime_kv_segments_skipped: 1,
        runtime_kv_segments_rejected: 1,
        ..RuntimeDiagnostics::default()
    };
    assert_eq!(segment_signal.runtime_kv_activity_count(), 3);
    assert!(!segment_signal.has_runtime_kv_exchange_signal());
    assert!(segment_signal.has_runtime_kv_activity_signal());
    assert!(segment_signal.has_forward_signal());
}
