use super::*;

#[test]
fn inspect_threshold_flags_imply_state_gate() {
    let args = Args::parse(vec![
        "--inspect-min-runtime-kv-memories".to_owned(),
        "1".to_owned(),
        "--inspect-min-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-max-experience-hygiene-quarantine-candidates".to_owned(),
        "0".to_owned(),
        "--inspect-max-experience-repairable-legacy-metadata-lessons".to_owned(),
        "0".to_owned(),
        "--inspect-max-experience-repairable-index-records".to_owned(),
        "0".to_owned(),
        "--inspect-max-experience-repair-projected-legacy-metadata-lessons".to_owned(),
        "0".to_owned(),
        "--inspect-max-experience-repair-skipped-missing-clean-gist".to_owned(),
        "0".to_owned(),
        "--inspect-max-experience-index-overlong-records".to_owned(),
        "2".to_owned(),
        "--inspect-max-experience-index-overlong-without-clean-gist".to_owned(),
        "0".to_owned(),
        "--inspect-max-experience-index-record-chars".to_owned(),
        "12000".to_owned(),
        "--inspect-max-experience-index-noisy-records".to_owned(),
        "0".to_owned(),
        "--inspect-max-experience-index-noise-penalty".to_owned(),
        "0.05".to_owned(),
        "--inspect-min-experience-index-quality-score".to_owned(),
        "0.92".to_owned(),
        "--inspect-require-experience-index-retrieval-ready".to_owned(),
        "--inspect-min-runtime-uncertainty-tokens".to_owned(),
        "4".to_owned(),
        "--inspect-min-rust-check-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-rust-check-passed".to_owned(),
        "1".to_owned(),
        "--inspect-max-rust-check-failed".to_owned(),
        "0".to_owned(),
        "--inspect-min-rust-check-diagnostic-chars".to_owned(),
        "42".to_owned(),
        "--inspect-min-evolution-replay-rust-check-items".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-replay-rust-check-passed".to_owned(),
        "1".to_owned(),
        "--inspect-max-evolution-replay-rust-check-failed".to_owned(),
        "0".to_owned(),
        "--inspect-min-evolution-replay-rust-check-live-memory-feedback-updates".to_owned(),
        "2".to_owned(),
        "--inspect-min-evolution-replay-rust-check-live-memory-feedback-applied".to_owned(),
        "2".to_owned(),
        "--inspect-min-evolution-replay-rust-check-live-memory-feedback-strength-delta".to_owned(),
        "0.36".to_owned(),
    ]);

    assert!(args.inspect_state);
    assert!(args.inspect_gate);
    assert_eq!(args.inspect_min_runtime_kv_memories, Some(1));
    assert_eq!(args.inspect_min_experiences, Some(1));
    assert_eq!(
        args.state_inspection_gate()
            .max_experience_hygiene_quarantine_candidates,
        Some(0)
    );
    assert_eq!(
        args.state_inspection_gate()
            .max_experience_repairable_legacy_metadata_lessons,
        Some(0)
    );
    assert_eq!(
        args.state_inspection_gate()
            .max_experience_repairable_index_records,
        Some(0)
    );
    assert_eq!(
        args.state_inspection_gate()
            .max_experience_repair_projected_legacy_metadata_lessons,
        Some(0)
    );
    assert_eq!(
        args.state_inspection_gate()
            .max_experience_repair_skipped_missing_clean_gist,
        Some(0)
    );
    assert_eq!(
        args.state_inspection_gate()
            .max_experience_index_overlong_records,
        Some(2)
    );
    assert_eq!(
        args.state_inspection_gate()
            .max_experience_index_overlong_without_clean_gist,
        Some(0)
    );
    assert_eq!(
        args.state_inspection_gate()
            .max_experience_index_record_chars,
        Some(12000)
    );
    assert_eq!(
        args.state_inspection_gate()
            .max_experience_index_noisy_records,
        Some(0)
    );
    assert_eq!(
        args.state_inspection_gate()
            .max_experience_index_noise_penalty,
        Some(0.05)
    );
    assert_eq!(
        args.state_inspection_gate()
            .min_experience_index_quality_score,
        Some(0.92)
    );
    assert!(
        args.state_inspection_gate()
            .require_experience_index_retrieval_ready
    );
    assert_eq!(
        args.state_inspection_gate().min_runtime_uncertainty_tokens,
        Some(4)
    );
    assert_eq!(
        args.state_inspection_gate().min_rust_check_experiences,
        Some(1)
    );
    assert_eq!(args.state_inspection_gate().min_rust_check_passed, Some(1));
    assert_eq!(args.state_inspection_gate().max_rust_check_failed, Some(0));
    assert_eq!(
        args.state_inspection_gate().min_rust_check_diagnostic_chars,
        Some(42)
    );
    assert_eq!(
        args.state_inspection_gate()
            .min_evolution_replay_rust_check_items,
        Some(1)
    );
    assert_eq!(
        args.state_inspection_gate()
            .min_evolution_replay_rust_check_passed,
        Some(1)
    );
    assert_eq!(
        args.state_inspection_gate()
            .max_evolution_replay_rust_check_failed,
        Some(0)
    );
    assert_eq!(
        args.state_inspection_gate()
            .min_evolution_replay_rust_check_live_memory_feedback_updates,
        Some(2)
    );
    assert_eq!(
        args.state_inspection_gate()
            .min_evolution_replay_rust_check_live_memory_feedback_applied,
        Some(2)
    );
    assert_eq!(
        args.state_inspection_gate()
            .min_evolution_replay_rust_check_live_memory_feedback_strength_delta,
        Some(0.36)
    );
}

#[test]
fn inspect_all_devices_flag_implies_state_gate() {
    let args = Args::parse(vec![
        "--inspect-state".to_owned(),
        "--benchmark-all-devices".to_owned(),
        "--inspect-min-runtime-uncertainty-token-device-profiles".to_owned(),
        "2".to_owned(),
    ]);

    assert!(args.inspect_state);
    assert!(args.inspect_gate);
    assert!(args.benchmark_all_devices);
    assert_eq!(
        args.state_inspection_matrix_gate()
            .min_runtime_uncertainty_token_device_profiles,
        Some(2)
    );
}

#[test]
fn parses_read_only_experience_hygiene_flags() {
    let args = Args::parse(vec![
        "--experience".to_owned(),
        "custom-experience.ndkv".to_owned(),
        "--experience-hygiene".to_owned(),
        "--experience-hygiene-limit".to_owned(),
        "7".to_owned(),
    ]);

    assert!(args.experience_hygiene);
    assert_eq!(args.experience_hygiene_limit, 7);
    assert_eq!(
        args.experience_path,
        std::path::PathBuf::from("custom-experience.ndkv")
    );
    assert!(!args.inspect_state);
}

#[test]
fn parses_read_only_experience_retrieval_flags() {
    let args = Args::parse(vec![
        "--experience".to_owned(),
        "custom-experience.ndkv".to_owned(),
        "--experience-retrieval".to_owned(),
        "--experience-retrieval-limit".to_owned(),
        "9".to_owned(),
        "--profile".to_owned(),
        "coding".to_owned(),
        "帮我用rust输出for循环".to_owned(),
    ]);

    assert!(args.experience_retrieval);
    assert_eq!(args.experience_retrieval_limit, 9);
    assert_eq!(args.profile, TaskProfile::Coding);
    assert_eq!(args.prompt, "帮我用rust输出for循环");
    assert_eq!(
        args.experience_path,
        std::path::PathBuf::from("custom-experience.ndkv")
    );
    assert!(!args.experience_hygiene);
    assert!(!args.inspect_state);
}

#[test]
fn parses_experience_repair_flags() {
    let args = Args::parse(vec![
        "--experience".to_owned(),
        "custom-experience.ndkv".to_owned(),
        "--experience-repair".to_owned(),
        "--experience-repair-limit".to_owned(),
        "11".to_owned(),
        "--experience-repair-apply".to_owned(),
        "--experience-repair-backup-path".to_owned(),
        "repair-backup.ndkv".to_owned(),
    ]);

    assert!(args.experience_repair);
    assert!(args.experience_repair_apply);
    assert_eq!(args.experience_repair_limit, 11);
    assert_eq!(
        args.experience_repair_backup_path,
        Some(std::path::PathBuf::from("repair-backup.ndkv"))
    );
    assert_eq!(
        args.experience_path,
        std::path::PathBuf::from("custom-experience.ndkv")
    );
    assert!(!args.experience_hygiene);
    assert!(!args.experience_retrieval);
}

#[test]
fn parses_experience_cleanup_audit_flags() {
    let args = Args::parse(vec![
        "--experience".to_owned(),
        "custom-experience.ndkv".to_owned(),
        "--experience-cleanup-audit".to_owned(),
        "--experience-cleanup-audit-limit".to_owned(),
        "13".to_owned(),
        "--experience-cleanup-audit-path".to_owned(),
        "target/audit.md".to_owned(),
    ]);

    assert!(args.experience_cleanup_audit);
    assert_eq!(args.experience_cleanup_audit_limit, 13);
    assert_eq!(
        args.experience_cleanup_audit_path,
        Some(std::path::PathBuf::from("target/audit.md"))
    );
    assert_eq!(
        args.experience_path,
        std::path::PathBuf::from("custom-experience.ndkv")
    );
    assert!(!args.prompt.contains("target/audit.md"));
    assert!(!args.experience_hygiene);
    assert!(!args.experience_repair);
    assert!(!args.experience_retrieval);
}

#[test]
fn parses_development_pollution_report_flags() {
    let args = Args::parse(vec![
        "--development-pollution".to_owned(),
        "--development-pollution-event-id".to_owned(),
        "window-305".to_owned(),
        "--development-pollution-source-kind".to_owned(),
        "issue_comment".to_owned(),
        "--development-pollution-reason".to_owned(),
        "development_evidence_contamination".to_owned(),
        "--development-pollution-hit-count".to_owned(),
        "2".to_owned(),
        "--development-pollution-current-proof".to_owned(),
        "--development-pollution-ttl".to_owned(),
        "next_release".to_owned(),
        "--development-pollution-scope".to_owned(),
        "prompt".to_owned(),
        "raw issue comment body".to_owned(),
    ]);

    assert!(args.development_pollution);
    assert_eq!(args.development_pollution_event_id, "window-305");
    assert_eq!(args.development_pollution_source_kind, "issue_comment");
    assert_eq!(
        args.development_pollution_reason,
        "development_evidence_contamination"
    );
    assert_eq!(args.development_pollution_hit_count, 2);
    assert!(args.development_pollution_current_proof);
    assert_eq!(
        args.development_pollution_ttl.as_deref(),
        Some("next_release")
    );
    assert_eq!(args.development_pollution_scope, "prompt");
    assert_eq!(args.prompt, "raw issue comment body");
    assert!(!args.experience_hygiene);
    assert!(!args.inspect_state);
}

#[test]
fn parses_development_pollution_dirty_worktree_flag() {
    let args = Args::parse(vec!["--development-pollution-dirty-worktree".to_owned()]);

    assert!(args.development_pollution);
    assert!(args.development_pollution_dirty_worktree);
    assert_eq!(args.development_pollution_scope, "worktree");
    assert!(!args.experience_hygiene);
    assert!(!args.inspect_state);
}

#[test]
fn parses_experience_hygiene_quarantine_apply_flags() {
    let args = Args::parse(vec![
        "--experience-hygiene-quarantine".to_owned(),
        "--experience-hygiene-apply".to_owned(),
        "--experience-hygiene-quarantine-path".to_owned(),
        "quarantine.ndkv".to_owned(),
        "--experience-hygiene-backup-path".to_owned(),
        "backup.ndkv".to_owned(),
    ]);

    assert!(args.experience_hygiene);
    assert!(args.experience_hygiene_quarantine);
    assert!(args.experience_hygiene_apply);
    assert_eq!(
        args.experience_hygiene_quarantine_path,
        Some(std::path::PathBuf::from("quarantine.ndkv"))
    );
    assert_eq!(
        args.experience_hygiene_backup_path,
        Some(std::path::PathBuf::from("backup.ndkv"))
    );
}
