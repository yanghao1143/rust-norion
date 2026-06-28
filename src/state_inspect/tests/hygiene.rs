use super::*;

#[test]
fn state_inspection_exposes_experience_hygiene_findings() {
    let mut engine = NoironEngine::new();
    let polluted_id = engine.experience.record(polluted_input());
    engine.experience.record(clean_input());

    let report = StateInspectionReport::from_engine(&engine, 4);

    assert_eq!(report.experience_count, 2);
    assert_eq!(report.experience_hygiene_finding_count, 1);
    assert_eq!(report.experience_hygiene_quarantine_candidate_count, 1);
    assert_eq!(report.experience_hygiene_findings.len(), 1);
    assert_eq!(
        report.experience_hygiene_findings[0].experience_id,
        polluted_id
    );
    assert_eq!(
        report.experience_hygiene_findings[0].severity,
        crate::experience::ExperienceHygieneSeverity::QuarantineCandidate
    );
    assert!(report.experience_hygiene_findings[0]
        .markers
        .contains(&"merge_requests".to_owned()));
    assert!(report
        .summary_line()
        .contains("experience_hygiene_quarantine_candidates=1"));

    let passing_gate = StateInspectionGate {
        max_experience_hygiene_quarantine_candidates: Some(1),
        ..StateInspectionGate::default()
    };
    assert!(report.evaluate(&passing_gate).passed());

    let failing_gate = StateInspectionGate {
        max_experience_hygiene_quarantine_candidates: Some(0),
        ..StateInspectionGate::default()
    };
    let gate_report = report.evaluate(&failing_gate);

    assert!(!gate_report.passed());
    assert!(gate_report
        .failures
        .contains(&"experience_hygiene_quarantine_candidate_count 1 above maximum 0".to_owned()));
}

#[test]
fn state_inspection_exposes_repairable_legacy_metadata_debt() {
    let mut engine = NoironEngine::new();
    engine.experience.record(ExperienceInput {
        prompt: "Rust for loop answer".to_owned(),
        lesson: "accepted_pattern quality=0.820 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        gist_records: vec![clean_gist(
            "asthought Use Rust range loops with println output when showing simple examples",
        )],
        quality: 0.82,
        ..experience_input("legacy repairable")
    });

    let report = StateInspectionReport::from_engine(&engine, 4);

    assert_eq!(report.experience_hygiene_legacy_metadata_lesson_count, 1);
    assert_eq!(
        report.experience_hygiene_legacy_metadata_without_clean_gist_count,
        0
    );
    assert_eq!(report.experience_repairable_legacy_metadata_lesson_count, 1);
    assert_eq!(report.experience_repairable_index_record_count, 0);
    assert_eq!(report.experience_repair_projected_hygiene_finding_count, 0);
    assert_eq!(report.experience_repair_projected_hygiene_watch_count, 0);
    assert_eq!(
        report.experience_repair_projected_hygiene_quarantine_candidate_count,
        0
    );
    assert_eq!(
        report.experience_repair_projected_legacy_metadata_lesson_count,
        0
    );
    assert_eq!(
        report.experience_repair_skipped_quarantine_candidate_count,
        0
    );
    assert_eq!(report.experience_repair_skipped_missing_clean_gist_count, 0);
    assert!(report
        .summary_line()
        .contains("experience_repairable_legacy_metadata_lessons=1"));
    assert!(report
        .summary_line()
        .contains("experience_repairable_index_records=0"));
    assert!(report
        .summary_line()
        .contains("experience_repair_projected_legacy_metadata_lessons=0"));

    let passing_gate = StateInspectionGate {
        max_experience_repairable_legacy_metadata_lessons: Some(1),
        max_experience_repairable_index_records: Some(0),
        max_experience_repair_projected_legacy_metadata_lessons: Some(0),
        max_experience_repair_skipped_missing_clean_gist: Some(0),
        ..StateInspectionGate::default()
    };
    assert!(report.evaluate(&passing_gate).passed());

    let failing_gate = StateInspectionGate {
        max_experience_repairable_legacy_metadata_lessons: Some(0),
        ..StateInspectionGate::default()
    };
    let gate_report = report.evaluate(&failing_gate);

    assert!(!gate_report.passed());
    assert!(gate_report.failures.contains(
        &"experience_repairable_legacy_metadata_lesson_count 1 above maximum 0".to_owned()
    ));
}

#[test]
fn state_inspection_exposes_repairable_index_debt() {
    let mut engine = NoironEngine::new();
    engine.experience.record(ExperienceInput {
        prompt: "Router context guidance".to_owned(),
        lesson: "assistant: prefer token-window feedback for router stability".to_owned(),
        gist_records: vec![clean_gist(
            "prefer token-window feedback for router stability",
        )],
        quality: 0.84,
        ..experience_input("role-labeled index residue")
    });

    let report = StateInspectionReport::from_engine(&engine, 4);

    assert_eq!(report.experience_repairable_legacy_metadata_lesson_count, 0);
    assert_eq!(report.experience_repairable_index_record_count, 1);
    assert_eq!(report.experience_index_noisy_record_count, 1);
    assert_eq!(report.experience_index_duplicate_output_count, 0);
    assert!(report
        .summary_line()
        .contains("experience_repairable_index_records=1"));

    let passing_gate = StateInspectionGate {
        max_experience_repairable_index_records: Some(1),
        ..StateInspectionGate::default()
    };
    assert!(report.evaluate(&passing_gate).passed());

    let failing_gate = StateInspectionGate {
        max_experience_repairable_index_records: Some(0),
        ..StateInspectionGate::default()
    };
    let gate_report = report.evaluate(&failing_gate);

    assert!(!gate_report.passed());
    assert!(gate_report
        .failures
        .contains(&"experience_repairable_index_record_count 1 above maximum 0".to_owned()));
}

#[test]
fn state_inspection_exposes_unrepairable_legacy_metadata_debt() {
    let mut engine = NoironEngine::new();
    engine.experience.record(ExperienceInput {
        prompt: "Rust loop answer without clean gist".to_owned(),
        lesson: "accepted_pattern quality=0.740 overlap=0.610 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.74,
        ..experience_input("legacy without gist")
    });

    let report = StateInspectionReport::from_engine(&engine, 4);

    assert_eq!(report.experience_repairable_legacy_metadata_lesson_count, 0);
    assert_eq!(
        report.experience_repair_projected_legacy_metadata_lesson_count,
        1
    );
    assert_eq!(report.experience_repair_skipped_missing_clean_gist_count, 1);
    assert!(report
        .summary_line()
        .contains("experience_repair_skipped_missing_clean_gist=1"));

    let passing_gate = StateInspectionGate {
        max_experience_repair_projected_legacy_metadata_lessons: Some(1),
        max_experience_repair_skipped_missing_clean_gist: Some(1),
        ..StateInspectionGate::default()
    };
    assert!(report.evaluate(&passing_gate).passed());

    let failing_gate = StateInspectionGate {
        max_experience_repair_projected_legacy_metadata_lessons: Some(0),
        max_experience_repair_skipped_missing_clean_gist: Some(0),
        ..StateInspectionGate::default()
    };
    let gate_report = report.evaluate(&failing_gate);

    assert!(!gate_report.passed());
    assert!(gate_report.failures.contains(
        &"experience_repair_projected_legacy_metadata_lesson_count 1 above maximum 0".to_owned()
    ));
    assert!(gate_report.failures.contains(
        &"experience_repair_skipped_missing_clean_gist_count 1 above maximum 0".to_owned()
    ));
}

#[test]
fn state_inspection_exposes_experience_index_noise() {
    let mut engine = NoironEngine::new();
    engine.experience.record(ExperienceInput {
        prompt: "long prompt ".repeat(260),
        lesson: "long lesson ".repeat(260),
        quality: 0.88,
        ..experience_input("long")
    });

    let report = StateInspectionReport::from_engine(&engine, 4);

    assert_eq!(report.experience_count, 1);
    assert_eq!(report.experience_hygiene_quarantine_candidate_count, 0);
    assert_eq!(report.experience_index_compacted_record_count, 1);
    assert_eq!(report.experience_index_overlong_record_count, 1);
    assert_eq!(report.experience_index_overlong_without_clean_gist_count, 1);
    assert!(report.experience_index_max_record_chars > 2_400);
    assert_eq!(report.experience_index_noisy_record_count, 1);
    assert!(report.experience_index_max_noise_penalty > 0.0);
    assert!(report.experience_index_quality_score < 0.5);
    assert!(!report.experience_index_retrieval_ready);
    assert_eq!(report.experience_index_risk_level, "blocked");
    assert_eq!(report.experience_index_findings.len(), 1);
    assert_eq!(report.experience_index_findings[0].experience_id, 1);
    assert_eq!(
        report.experience_index_findings[0].reason,
        "overlong_single_document_without_clean_gist"
    );
    assert!(report.experience_index_findings[0].compacted);
    assert!(report
        .summary_line()
        .contains("experience_index_compacted_records=1"));
    assert!(report
        .summary_line()
        .contains("experience_index_overlong_records=1"));
    assert!(report
        .summary_line()
        .contains("experience_index_overlong_without_clean_gist=1"));
    assert!(report
        .summary_line()
        .contains("experience_index_max_record_chars="));
    assert!(report
        .summary_line()
        .contains("experience_index_noisy_records=1"));
    assert!(report
        .summary_line()
        .contains("experience_index_quality_score="));
    assert!(report
        .summary_line()
        .contains("experience_index_retrieval_ready=false"));
    assert!(report
        .summary_line()
        .contains("experience_index_risk_level=blocked"));

    let passing_gate = StateInspectionGate {
        max_experience_index_overlong_records: Some(1),
        max_experience_index_overlong_without_clean_gist: Some(1),
        max_experience_index_record_chars: Some(report.experience_index_max_record_chars),
        max_experience_index_noisy_records: Some(1),
        max_experience_index_noise_penalty: Some(report.experience_index_max_noise_penalty),
        min_experience_index_quality_score: Some(report.experience_index_quality_score),
        ..StateInspectionGate::default()
    };
    assert!(report.evaluate(&passing_gate).passed());

    let failing_gate = StateInspectionGate {
        max_experience_index_overlong_records: Some(0),
        max_experience_index_overlong_without_clean_gist: Some(0),
        max_experience_index_record_chars: Some(2_400),
        max_experience_index_noisy_records: Some(0),
        max_experience_index_noise_penalty: Some(0.01),
        min_experience_index_quality_score: Some(0.92),
        require_experience_index_retrieval_ready: true,
        ..StateInspectionGate::default()
    };
    let gate_report = report.evaluate(&failing_gate);

    assert!(!gate_report.passed());
    assert!(gate_report
        .failures
        .contains(&"experience_index_noisy_record_count 1 above maximum 0".to_owned()));
    assert!(gate_report
        .failures
        .contains(&"experience_index_overlong_record_count 1 above maximum 0".to_owned()));
    assert!(gate_report.failures.contains(
        &"experience_index_overlong_without_clean_gist_count 1 above maximum 0".to_owned()
    ));
    assert!(gate_report.failures.iter().any(|failure| {
        failure.starts_with("experience_index_max_record_chars ")
            && failure.ends_with(" above maximum 2400")
    }));
    assert!(gate_report.failures.iter().any(|failure| {
        failure.starts_with("experience_index_max_noise_penalty ")
            && failure.ends_with(" above maximum 0.010000")
    }));
    assert!(gate_report.failures.iter().any(|failure| {
        failure.starts_with("experience_index_quality_score ")
            && failure.ends_with(" below required 0.920000")
    }));
    assert!(gate_report
        .failures
        .contains(&"experience_index_retrieval_ready false but required true".to_owned()));
}

fn polluted_input() -> ExperienceInput {
    ExperienceInput {
        prompt: "Conversation transcript:\nuser: rust for loop\nassistant: ok\nuser: Bash command\nssh -o ConnectTimeout=8 host\nassistant: merge_requests on gitlab.local"
            .to_owned(),
        lesson: "polluted shell transcript leaked into a coding answer".to_owned(),
        quality: 0.91,
        ..experience_input("polluted")
    }
}

fn clean_input() -> ExperienceInput {
    ExperienceInput {
        prompt: "Rust clean loop".to_owned(),
        lesson: "use for item in items".to_owned(),
        quality: 0.82,
        ..experience_input("clean")
    }
}

fn clean_gist(summary: &str) -> GistRecord {
    GistRecord {
        level: GistLevel::Document,
        title: "repair".to_owned(),
        summary: summary.to_owned(),
        source_tokens: 32,
        importance: 0.9,
    }
}

fn experience_input(lesson: &str) -> ExperienceInput {
    ExperienceInput {
        prompt: "state hygiene".to_owned(),
        profile: TaskProfile::Coding,
        lesson: lesson.to_owned(),
        quality: 0.75,
        contradictions: Vec::new(),
        reflection_issues: vec![ReflectionIssue::new(
            "needs_grounding",
            ReflectionSeverity::Warning,
            "test hygiene",
        )],
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.5,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::default(),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: crate::reflection::RuntimeDiagnostics::default(),
        runtime_token_metrics: ExperienceRuntimeTokenMetrics::default(),
        process_reward: ProcessRewardReport {
            total: 0.5,
            action: RewardAction::Hold,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    }
}
