use super::*;

#[test]
fn inspection_report_summarizes_full_width_evidence_notes() {
    let mut engine = NoironEngine::new();

    engine.experience.record(ExperienceInput {
        prompt: "inspect full-width evidence".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "full-width evidence separators should remain inspectable".to_owned(),
        quality: 0.88,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
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
        hierarchy: HierarchyWeights::new(0.34, 0.33, 0.33),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: crate::reflection::RuntimeDiagnostics::default(),
        runtime_token_metrics: ExperienceRuntimeTokenMetrics::default(),
        process_reward: ProcessRewardReport {
            total: 0.88,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![
                "ｒｅｃｕｒｓｉｖｅ：ｃｈｕｎｋｓ＝５：ｍｅｒｇｅ＿ｒｏｕｎｄｓ＝２：ｗａｖｅｓ＝３：ｐａｒａｌｌｅｌ＝２：ｒｕｎｔｉｍｅ＿ｃａｌｌｓ＝９"
                    .to_owned(),
                "ｍｅｍｏｒｙ＿ｆｅｅｄｂａｃｋ：ｒｅｉｎｆｏｒｃｅｄ＝２：ｐｅｎａｌｉｚｅｄ＝１：ｒｅｉｎｆｏｒｃｅｍｅｎｔ＿ａｍｏｕｎｔ＝１．４０００００：ｐｅｎａｌｔｙ＿ａｍｏｕｎｔ＝０．３０００００：ａｐｐｌｉｅｄ＝２：ｒｅｍｏｖｅｄ＝０：ｍｉｓｓｉｎｇ＝１：ｓｔｒｅｎｇｔｈ＿ｄｅｌｔａ＝０．５１００００"
                    .to_owned(),
                "ｒｕｓｔ＿ｃｈｅｃｋ：ｐａｓｓｅｄ＝ｔｒｕｅ：ｌａｂｅｌ＝rustc_passed：ｅｄｉｔｉｏｎ＝２０２１：ｓｔａｔｕｓ＿ｃｏｄｅ＝０：ｄｉａｇｎｏｓｔｉｃ＿ｃｈａｒｓ＝４２"
                    .to_owned(),
                "ｂｕｓｉｎｅｓｓ＿ｃｏｎｔｒａｃｔ：ｃａｓｅ＝gemma-service-rust-feedback：ｐａｓｓｅｄ＝ｔｒｕｅ：ｒｅｑｕｉｒｅｄ＝４：ｍａｔｃｈｅｄ＝４：ｍｉｓｓｉｎｇ＝０：ｐｒｏｔｏｃｏｌ＿ｌｅａｋ＝ｆａｌｓｅ：ｓｕｂｓｔｉｔｕｔｅｄ＿ｒｕｎｔｉｍｅ＿ｍｏｄｅｌ＿ｅｘｐｅｒｉｅｎｃｅｓ＝ｆａｌｓｅ：ｅｖａｓｉｖｅ＿ｄｅｎｉａｌ＝ｆａｌｓｅ：ｈａｎｄｌｉｎｇ＿ｓｉｇｎａｌ＝ｔｒｕｅ：ｒａｗ＿ｐａｓｓｅｄ＝ｆａｌｓｅ：ｎｏｒｍａｌｉｚａｔｉｏｎ＝ｃａｎｏｎｉｃａｌ＿ｆａｌｌｂａｃｋ：ｒｅｓｐｏｎｓｅ＿ｎｏｒｍａｌｉｚｅｄ＝ｔｒｕｅ：ｃａｎｏｎｉｃａｌ＿ｆａｌｌｂａｃｋ＝ｔｒｕｅ"
                    .to_owned(),
                "ｐｏｏｌ＿ｄｉｓｐａｔｃｈ：ｓｅｌｅｃｔｅｄ＿ｒｏｌｅ＝ｒｅｖｉｅｗ：ｓｅｌｅｃｔｅｄ＿ｐｏｒｔ＝８６８８：ｓｅｌｅｃｔｅｄ＿ｅｎｄｐｏｉｎｔ＝http://127.0.0.1:8688：ｃｏｎｔｅｘｔ＿ｗｉｎｄｏｗ＝８１９２：ｄｅｆａｕｌｔ＿ｍａｘ＿ｔｏｋｅｎｓ＝１０２４：ｃｏｎｆｉｇｕｒｅｄ＿ｍａｘ＿ｔｏｋｅｎｓ＝４０９６：ｅｆｆｅｃｔｉｖｅ＿ｍａｘ＿ｔｏｋｅｎｓ＝１０２４：ｍａｘ＿ｔｏｋｅｎｓ＿ｃｌａｍｐｅｄ＝ｔｒｕｅ：ｌｏｗ＿ｐｒｉｏｒｉｔｙ＝ｔｒｕｅ：ｆｏｒｗａｒｄｅｄ＝ｔｒｕｅ：ｄｉｓｐａｔｃｈ＿ｍｏｄｅ＝runtime_endpoint_override：ｄｉｓｐａｔｃｈ＿ｒｅａｓｏｎ＝selected_worker_ready"
                    .to_owned(),
                "ｅｘｔｅｒｎａｌ＿ｓｅｍａｎｔｉｃ＿ｃｏｎｔｅｘｔｓ：ｃｏｕｎｔ＝４：ｒｏｕｔｅ＿ｃａｎｄｉｄａｔｅｓ＝４：ｆｕｓｉｏｎ＿ｃａｎｄｉｄａｔｅｓ＝４"
                    .to_owned(),
                "ｓｅｌｆ＿ｅｖｏｌｖｉｎｇ＿ｍｅｍｏｒｙ＿ｗｒｉｔｅｂａｃｋ：ａｔｔｅｍｐｔｅｄ＿ｒｅｃｏｒｄｓ＝３：ａｃｃｅｐｔｅｄ＿ｒｅｃｏｒｄｓ＝３：ｒｅｃｏｒｄｓ＿ｂｅｆｏｒｅ＝１：ｒｅｃｏｒｄｓ＿ａｆｔｅｒ＝４：ｔｏｏｌ＿ｒｅｌｉａｂｉｌｉｔｙ＿ａｆｔｅｒ＝１：ｔｏｏｌ＿ｏｂｓｅｒｖａｔｉｏｎｓ＿ａｆｔｅｒ＝２：ｍａｉｎｔｅｎａｎｃｅ＿ａｃｔｉｏｎｓ＝１：ｍｅｒｇｅｄ＿ｄｕｐｌｉｃａｔｅ＿ｅｐｉｓｏｄｅｓ＝１：ｗｒｉｔｅ＿ａｌｌｏｗｅｄ＝ｔｒｕｅ：ｄｕｒａｂｌｅ＿ｗｒｉｔｅ＿ａｌｌｏｗｅｄ＝ｔｒｕｅ：ａｐｐｌｉｅｄ＝ｔｒｕｅ：ａｐｐｌｉｅｄ＿ｔｏ＿ｄｉｓｋ＝ｔｒｕｅ：ｓｎａｐｓｈｏｔ＿ｃｈａｎｇｅｓ＝１"
                    .to_owned(),
                "ｆｈｔ＿ｄｋｅ＿ｂｕｄｇｅｔ：ｅｎａｂｌｅｄ＝ｔｒｕｅ：ｔｏｔａｌ＿ｔｏｋｅｎｓ＝４０９６：ｄｅｎｓｅ＿ｔｏｋｅｎｓ＝２０４８：ｒｏｕｔｅｄ＿ｔｏｋｅｎｓ＝２０４８：ｋｖ＿ｅｘｃｈａｎｇｅ＿ｂｌｏｃｋｓ＝７：ｔｏｋｅｎ＿ｓｐｌｉｔ＿ｖａｌｉｄ＝ｔｒｕｅ：ａｔｔｅｎｔｉｏｎ＿ｔｈｒｅｓｈｏｌｄ＝０．６２５：ｒｏｕｔｅ＿ｐｒｅｓｓｕｒｅ＝０．７５"
                    .to_owned(),
                "ｒｕｎｔｉｍｅ＿ｅｒｒｏｒ：ｌａｂｅｌ＝runtime_error：ｔｉｍｅｏｕｔ＝ｔｒｕｅ：ｍｅｓｓａｇｅ＿ｃｈａｒｓ＝４８".to_owned(),
            ],
        },
        live_evolution: Default::default(),
    });

    let report = StateInspectionReport::from_engine(&engine, 1);

    assert_eq!(report.live_memory_feedback_experience_count, 1);
    assert_eq!(report.live_memory_feedback_update_count, 3);
    assert_eq!(report.live_memory_feedback_reinforced_count, 2);
    assert_eq!(report.live_memory_feedback_penalized_count, 1);
    assert_eq!(report.live_memory_feedback_detail_experience_count, 1);
    assert_eq!(report.live_memory_feedback_applied_count, 2);
    assert_eq!(report.live_memory_feedback_removed_count, 0);
    assert_eq!(report.live_memory_feedback_missing_count, 1);
    assert!((report.live_memory_feedback_strength_delta - 0.51).abs() < 0.0001);
    assert_eq!(report.rust_check_experience_count, 1);
    assert_eq!(report.rust_check_passed_count, 1);
    assert_eq!(report.rust_check_failed_count, 0);
    assert_eq!(report.rust_check_diagnostic_chars, 42);
    assert_eq!(report.business_contract_experience_count, 1);
    assert_eq!(report.business_contract_passed_count, 1);
    assert_eq!(report.business_contract_failed_count, 0);
    assert_eq!(report.business_contract_required_signals, 4);
    assert_eq!(report.business_contract_matched_signals, 4);
    assert_eq!(report.business_contract_missing_signals, 0);
    assert_eq!(report.business_contract_protocol_leaks, 0);
    assert_eq!(report.business_contract_substitutions, 0);
    assert_eq!(report.business_contract_evasive_denials, 0);
    assert_eq!(report.business_contract_missing_handling_signals, 0);
    assert_eq!(report.business_contract_raw_passed_count, 0);
    assert_eq!(report.business_contract_raw_failed_count, 1);
    assert_eq!(report.business_contract_response_normalized_count, 1);
    assert_eq!(report.business_contract_sanitized_count, 0);
    assert_eq!(report.business_contract_canonical_fallback_count, 1);
    assert_eq!(report.pool_dispatch_experience_count, 1);
    assert_eq!(report.pool_dispatch_item_count, 1);
    assert_eq!(report.pool_dispatch_forwarded_count, 1);
    assert_eq!(report.pool_dispatch_clamped_count, 1);
    assert_eq!(report.pool_dispatch_low_priority_count, 1);
    assert_eq!(report.external_semantic_context_experience_count, 1);
    assert_eq!(report.external_semantic_context_count, 4);
    assert_eq!(report.self_evolving_memory_writeback_experience_count, 1);
    assert_eq!(report.self_evolving_memory_writeback_attempted_records, 3);
    assert_eq!(report.self_evolving_memory_writeback_accepted_records, 3);
    assert_eq!(report.self_evolving_memory_writeback_rejected_records(), 0);
    assert_eq!(report.self_evolving_memory_writeback_records_before, 1);
    assert_eq!(report.self_evolving_memory_writeback_records_after, 4);
    assert_eq!(
        report.self_evolving_memory_writeback_tool_reliability_after,
        1
    );
    assert_eq!(
        report.self_evolving_memory_writeback_tool_observations_after,
        2
    );
    assert_eq!(report.self_evolving_memory_writeback_maintenance_actions, 1);
    assert_eq!(
        report.self_evolving_memory_writeback_merged_duplicate_episodes,
        1
    );
    assert_eq!(report.self_evolving_memory_writeback_write_allowed, 1);
    assert_eq!(
        report.self_evolving_memory_writeback_durable_write_allowed,
        1
    );
    assert_eq!(report.self_evolving_memory_writeback_applied, 1);
    assert_eq!(report.self_evolving_memory_writeback_applied_to_disk, 1);
    assert_eq!(report.self_evolving_memory_writeback_snapshot_changes, 1);
    assert_eq!(report.fht_dke_budget_experience_count, 1);
    assert_eq!(report.fht_dke_enabled_experience_count, 1);
    assert_eq!(report.fht_dke_total_tokens, 4096);
    assert_eq!(report.fht_dke_dense_tokens, 2048);
    assert_eq!(report.fht_dke_routed_tokens, 2048);
    assert_eq!(report.fht_dke_kv_exchange_blocks, 7);
    assert_eq!(report.fht_dke_token_split_valid_count, 1);
    assert_eq!(report.fht_dke_token_split_invalid_count, 0);
    assert_eq!(report.fht_dke_attention_threshold_experience_count, 1);
    assert!((report.fht_dke_attention_threshold_avg - 0.625).abs() < 0.0001);
    assert!((report.fht_dke_attention_threshold_max - 0.625).abs() < 0.0001);
    assert_eq!(report.fht_dke_route_pressure_experience_count, 1);
    assert!((report.fht_dke_route_pressure_avg - 0.75).abs() < 0.0001);
    assert!((report.fht_dke_route_pressure_max - 0.75).abs() < 0.0001);
    assert_eq!(report.runtime_error_experience_count, 1);
    assert_eq!(report.runtime_error_count, 1);
    assert_eq!(report.runtime_timeout_experience_count, 1);
    assert_eq!(report.runtime_timeout_count, 1);
    assert_eq!(report.runtime_error_message_chars, 48);
    assert!(report
        .summary_line()
        .contains("live_memory_feedback_updates=3"));
    assert!(report.summary_line().contains("rust_check_passed=1"));
    assert!(report
        .summary_line()
        .contains("business_contract_raw_failed=1"));
    assert!(report.summary_line().contains("runtime_errors=1"));
    assert!(report
        .summary_line()
        .contains("external_semantic_contexts=4"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_writeback_applied=1"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_writeback_applied_to_disk=1"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_writeback_rejected_records=0"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_writeback_tool_observations_after=2"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_writeback_maintenance_actions=1"));
    assert!(report
        .summary_line()
        .contains("fht_dke_budget_experiences=1"));
    assert!(report.summary_line().contains("fht_dke_total_tokens=4096"));
    assert!(report
        .summary_line()
        .contains("fht_dke_attention_threshold_avg=0.625"));
    assert!(report
        .summary_line()
        .contains("fht_dke_route_pressure_max=0.750"));

    let top = &report.top_experiences[0];
    assert_eq!(top.recursive_runtime_calls, Some(9));
    assert_eq!(top.live_memory_feedback_updates, 3);
    assert_eq!(top.live_memory_feedback_reinforced, 2);
    assert_eq!(top.live_memory_feedback_penalized, 1);
    assert_eq!(top.live_memory_feedback_applied, 2);
    assert_eq!(top.live_memory_feedback_removed, 0);
    assert_eq!(top.live_memory_feedback_missing, 1);
    assert!((top.live_memory_feedback_strength_delta - 0.51).abs() < 0.0001);
    assert!(top.live_memory_feedback_detail);
    assert_eq!(top.rust_check_passed, 1);
    assert_eq!(top.rust_check_failed, 0);
    assert_eq!(top.rust_check_diagnostic_chars, 42);
    assert_eq!(top.business_contract_passed, 1);
    assert_eq!(top.business_contract_failed, 0);
    assert_eq!(top.business_contract_raw_failed, 1);
    assert_eq!(top.business_contract_response_normalized, 1);
    assert_eq!(top.business_contract_canonical_fallbacks, 1);
    assert_eq!(top.pool_dispatch_items, 1);
    assert_eq!(top.pool_dispatch_selected_roles, vec!["review".to_owned()]);
    assert_eq!(top.pool_dispatch_forwarded, 1);
    assert_eq!(top.pool_dispatch_clamped, 1);
    assert_eq!(top.pool_dispatch_low_priority, 1);
    assert_eq!(top.external_semantic_contexts, 4);
    assert_eq!(top.self_evolving_memory_writeback_attempted_records, 3);
    assert_eq!(top.self_evolving_memory_writeback_accepted_records, 3);
    assert_eq!(top.self_evolving_memory_writeback_rejected_records(), 0);
    assert_eq!(top.self_evolving_memory_writeback_records_after, 4);
    assert_eq!(top.self_evolving_memory_writeback_write_allowed, 1);
    assert_eq!(top.self_evolving_memory_writeback_durable_write_allowed, 1);
    assert_eq!(top.self_evolving_memory_writeback_applied, 1);
    assert_eq!(top.self_evolving_memory_writeback_applied_to_disk, 1);
    assert_eq!(top.self_evolving_memory_writeback_snapshot_changes, 1);
    assert_eq!(top.runtime_errors, 1);
    assert_eq!(top.runtime_timeouts, 1);
    assert_eq!(top.runtime_error_message_chars, 48);

    let passing_gate = StateInspectionGate {
        min_experiences: Some(1),
        min_live_memory_feedback_experiences: Some(1),
        min_live_memory_feedback_updates: Some(3),
        min_live_memory_feedback_reinforced: Some(2),
        min_live_memory_feedback_penalized: Some(1),
        min_live_memory_feedback_detail_experiences: Some(1),
        min_live_memory_feedback_applied: Some(2),
        max_live_memory_feedback_missing: Some(1),
        min_live_memory_feedback_strength_delta: Some(0.51),
        min_rust_check_experiences: Some(1),
        min_rust_check_passed: Some(1),
        max_rust_check_failed: Some(0),
        min_rust_check_diagnostic_chars: Some(42),
        min_business_contract_experiences: Some(1),
        min_business_contract_passed: Some(1),
        max_business_contract_failed: Some(0),
        max_business_contract_missing_signals: Some(0),
        max_business_contract_protocol_leaks: Some(0),
        max_business_contract_substitutions: Some(0),
        max_business_contract_evasive_denials: Some(0),
        max_business_contract_missing_handling_signals: Some(0),
        min_external_semantic_context_experiences: Some(1),
        min_external_semantic_contexts: Some(4),
        min_self_evolving_memory_writeback_experiences: Some(1),
        min_self_evolving_memory_writeback_attempted_records: Some(3),
        min_self_evolving_memory_writeback_accepted_records: Some(3),
        max_self_evolving_memory_writeback_rejected_records: Some(0),
        min_self_evolving_memory_writeback_applied_to_disk: Some(1),
        min_self_evolving_memory_writeback_snapshot_changes: Some(1),
        min_fht_dke_budget_experiences: Some(1),
        min_fht_dke_enabled_experiences: Some(1),
        min_fht_dke_routed_tokens: Some(2048),
        max_fht_dke_token_split_invalid: Some(0),
        min_fht_dke_attention_threshold: Some(0.5),
        max_fht_dke_attention_threshold: Some(0.75),
        min_fht_dke_route_pressure: Some(0.5),
        max_fht_dke_route_pressure: Some(0.80),
        max_runtime_errors: Some(1),
        max_runtime_timeouts: Some(1),
        ..Default::default()
    };
    assert!(report.evaluate(&passing_gate).passed());

    let failing_gate = StateInspectionGate {
        min_rust_check_diagnostic_chars: Some(43),
        min_external_semantic_contexts: Some(5),
        min_self_evolving_memory_writeback_experiences: Some(2),
        min_self_evolving_memory_writeback_attempted_records: Some(4),
        min_self_evolving_memory_writeback_accepted_records: Some(4),
        min_self_evolving_memory_writeback_applied_to_disk: Some(2),
        min_self_evolving_memory_writeback_snapshot_changes: Some(2),
        min_fht_dke_budget_experiences: Some(2),
        min_fht_dke_enabled_experiences: Some(2),
        min_fht_dke_routed_tokens: Some(2049),
        max_fht_dke_token_split_invalid: Some(0),
        min_fht_dke_attention_threshold: Some(0.7),
        max_fht_dke_attention_threshold: Some(0.6),
        min_fht_dke_route_pressure: Some(0.8),
        max_fht_dke_route_pressure: Some(0.7),
        max_runtime_errors: Some(0),
        ..Default::default()
    };
    let failing_report = report.evaluate(&failing_gate);
    assert!(!failing_report.passed());
    assert!(failing_report
        .failures
        .contains(&"rust_check_diagnostic_chars 42 below required 43".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_error_count 1 above maximum 0".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"external_semantic_context_count 4 below required 5".to_owned()));
    assert!(failing_report.failures.contains(
        &"self_evolving_memory_writeback_experience_count 1 below required 2".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"self_evolving_memory_writeback_attempted_records 3 below required 4".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"self_evolving_memory_writeback_accepted_records 3 below required 4".to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"self_evolving_memory_writeback_applied_to_disk 1 below required 2".to_owned()));
    assert!(failing_report.failures.contains(
        &"self_evolving_memory_writeback_snapshot_changes 1 below required 2".to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"fht_dke_budget_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"fht_dke_enabled_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"fht_dke_routed_tokens 2048 below required 2049".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"fht_dke_attention_threshold_avg 0.625000 below required 0.700000".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"fht_dke_attention_threshold_max 0.625000 above maximum 0.600000".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"fht_dke_route_pressure_avg 0.750000 below required 0.800000".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"fht_dke_route_pressure_max 0.750000 above maximum 0.700000".to_owned()));

    let mut rejected_writeback_report = report.clone();
    rejected_writeback_report.self_evolving_memory_writeback_accepted_records = 2;
    let rejected_gate = StateInspectionGate {
        max_self_evolving_memory_writeback_rejected_records: Some(0),
        ..Default::default()
    };
    let rejected_eval = rejected_writeback_report.evaluate(&rejected_gate);
    assert!(!rejected_eval.passed());
    assert!(rejected_eval
        .failures
        .contains(&"self_evolving_memory_writeback_rejected_records 1 above maximum 0".to_owned()));

    let mut impossible_writeback_report = report.clone();
    impossible_writeback_report.self_evolving_memory_writeback_accepted_records = 4;
    let impossible_eval = impossible_writeback_report.evaluate(&StateInspectionGate::default());
    assert!(!impossible_eval.passed());
    assert!(impossible_eval.failures.contains(
        &"self_evolving_memory_writeback_accepted_records 4 exceeds attempted_records 3".to_owned()
    ));

    let mut empty_accepted_writeback_report = report.clone();
    empty_accepted_writeback_report.self_evolving_memory_writeback_accepted_records = 0;
    let empty_accepted_eval =
        empty_accepted_writeback_report.evaluate(&StateInspectionGate::default());
    assert!(!empty_accepted_eval.passed());
    assert!(empty_accepted_eval.failures.contains(
        &"self_evolving_memory_writeback_accepted_records must be positive when attempted_records is positive"
            .to_owned()
    ));

    let mut missing_attempted_writeback_report = report.clone();
    missing_attempted_writeback_report.self_evolving_memory_writeback_attempted_records = 0;
    missing_attempted_writeback_report.self_evolving_memory_writeback_accepted_records = 0;
    missing_attempted_writeback_report.self_evolving_memory_writeback_applied_to_disk = 0;
    missing_attempted_writeback_report.self_evolving_memory_writeback_snapshot_changes = 0;
    let missing_attempted_eval =
        missing_attempted_writeback_report.evaluate(&StateInspectionGate::default());
    assert!(!missing_attempted_eval.passed());
    assert!(missing_attempted_eval.failures.contains(
        &"self_evolving_memory_writeback_attempted_records 0 below writeback_experience_count 1"
            .to_owned()
    ));

    let mut impossible_records_after_report = report.clone();
    impossible_records_after_report.self_evolving_memory_writeback_records_after = 2;
    let impossible_records_after_eval =
        impossible_records_after_report.evaluate(&StateInspectionGate::default());
    assert!(!impossible_records_after_eval.passed());
    assert!(impossible_records_after_eval.failures.contains(
        &"self_evolving_memory_writeback_records_after 2 below accepted_records 3".to_owned()
    ));

    let mut missing_write_allow_report = report.clone();
    missing_write_allow_report.self_evolving_memory_writeback_write_allowed = 0;
    let missing_write_allow_eval =
        missing_write_allow_report.evaluate(&StateInspectionGate::default());
    assert!(!missing_write_allow_eval.passed());
    assert!(missing_write_allow_eval.failures.contains(
        &"self_evolving_memory_writeback_write_allowed 0 does not match applied_to_disk 1"
            .to_owned()
    ));

    let mut impossible_writeback_apply_report = report.clone();
    impossible_writeback_apply_report.self_evolving_memory_writeback_snapshot_changes = 0;
    let impossible_apply_eval =
        impossible_writeback_apply_report.evaluate(&StateInspectionGate::default());
    assert!(!impossible_apply_eval.passed());
    assert!(impossible_apply_eval.failures.contains(
        &"self_evolving_memory_writeback_applied_to_disk 1 does not match snapshot_changes 0"
            .to_owned()
    ));

    let mut impossible_fht_report = report.clone();
    impossible_fht_report.fht_dke_routed_tokens = 2049;
    let impossible_fht_eval = impossible_fht_report.evaluate(&StateInspectionGate::default());
    assert!(!impossible_fht_eval.passed());
    assert!(impossible_fht_eval
        .failures
        .contains(&"fht_dke dense+routed tokens 4097 does not match total_tokens 4096".to_owned()));

    let mut impossible_threshold_report = report.clone();
    impossible_threshold_report.fht_dke_attention_threshold_avg = 1.25;
    impossible_threshold_report.fht_dke_attention_threshold_max = 1.25;
    let impossible_threshold_eval =
        impossible_threshold_report.evaluate(&StateInspectionGate::default());
    assert!(!impossible_threshold_eval.passed());
    assert!(impossible_threshold_eval.failures.contains(
        &"fht_dke_attention_threshold_avg 1.250000 must stay within 0.0..=1.0".to_owned()
    ));

    let mut impossible_pressure_report = report.clone();
    impossible_pressure_report.fht_dke_route_pressure_avg = 0.90;
    impossible_pressure_report.fht_dke_route_pressure_max = 0.80;
    let impossible_pressure_eval =
        impossible_pressure_report.evaluate(&StateInspectionGate::default());
    assert!(!impossible_pressure_eval.passed());
    assert!(impossible_pressure_eval
        .failures
        .contains(&"fht_dke_route_pressure_avg 0.900000 exceeds max 0.800000".to_owned()));
}

#[test]
fn inspection_report_keeps_audit_evidence_without_outcome() {
    let mut engine = NoironEngine::new();

    engine.experience.record(ExperienceInput {
        prompt: "inspect outcome-less audit evidence".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "audit evidence without trusted outcomes should remain visible".to_owned(),
        quality: 0.77,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
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
        hierarchy: HierarchyWeights::new(0.34, 0.33, 0.33),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: crate::reflection::RuntimeDiagnostics::default(),
        runtime_token_metrics: ExperienceRuntimeTokenMetrics::default(),
        process_reward: ProcessRewardReport {
            total: 0.77,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![
                "rust_check:label=legacy_rustc:diagnostic_chars=11".to_owned(),
                "rust_check:passed=maybe:label=broken_rustc:diagnostic_chars=7".to_owned(),
                "business_contract:case=legacy-audit:required=4:matched=3:missing=1:normalization=sanitized"
                    .to_owned(),
                "business_contract:case=broken-audit:passed=maybe:raw_passed=:handling_signal=:response_normalized=true:canonical_fallback=true"
                    .to_owned(),
            ],
        },
        live_evolution: Default::default(),
    });

    let report = StateInspectionReport::from_engine(&engine, 1);

    assert_eq!(report.rust_check_experience_count, 1);
    assert_eq!(report.rust_check_passed_count, 0);
    assert_eq!(report.rust_check_failed_count, 0);
    assert_eq!(report.rust_check_diagnostic_chars, 18);
    assert_eq!(report.business_contract_experience_count, 1);
    assert_eq!(report.business_contract_passed_count, 0);
    assert_eq!(report.business_contract_failed_count, 0);
    assert_eq!(report.business_contract_required_signals, 4);
    assert_eq!(report.business_contract_matched_signals, 3);
    assert_eq!(report.business_contract_missing_signals, 1);
    assert_eq!(report.business_contract_missing_handling_signals, 0);
    assert_eq!(report.business_contract_raw_passed_count, 0);
    assert_eq!(report.business_contract_raw_failed_count, 0);
    assert_eq!(report.business_contract_response_normalized_count, 1);
    assert_eq!(report.business_contract_sanitized_count, 1);
    assert_eq!(report.business_contract_canonical_fallback_count, 1);
    assert!(report.summary_line().contains("rust_check_experiences=1"));
    assert!(report.summary_line().contains("rust_check_passed=0"));
    assert!(report.summary_line().contains("rust_check_failed=0"));
    assert!(report
        .summary_line()
        .contains("rust_check_diagnostic_chars=18"));
    assert!(report
        .summary_line()
        .contains("business_contract_experiences=1"));
    assert!(report.summary_line().contains("business_contract_passed=0"));
    assert!(report.summary_line().contains("business_contract_failed=0"));
    assert!(report
        .summary_line()
        .contains("business_contract_required_signals=4"));
    assert!(report
        .summary_line()
        .contains("business_contract_response_normalized=1"));

    let top = &report.top_experiences[0];
    assert_eq!(top.rust_check_passed, 0);
    assert_eq!(top.rust_check_failed, 0);
    assert_eq!(top.rust_check_diagnostic_chars, 18);
    assert_eq!(top.business_contract_passed, 0);
    assert_eq!(top.business_contract_failed, 0);
    assert_eq!(top.business_contract_missing_signals, 1);
    assert_eq!(top.business_contract_missing_handling_signals, 0);
    assert_eq!(top.business_contract_raw_passed, 0);
    assert_eq!(top.business_contract_raw_failed, 0);
    assert_eq!(top.business_contract_response_normalized, 1);
    assert_eq!(top.business_contract_sanitized, 1);
    assert_eq!(top.business_contract_canonical_fallbacks, 1);
}

#[test]
fn inspection_report_summarizes_memory_experience_and_adaptive_state() {
    let mut engine = NoironEngine::new();
    let memory_id =
        engine
            .cache
            .store_or_fuse("inspectable reinforced memory", vec![1.0, 0.0, 0.0], 0.9);
    let fallback_memory_id =
        engine
            .cache
            .store_or_fuse("fallback embedding memory", vec![0.0, 1.0, 0.0, 0.0], 0.7);
    let runtime_kv_memory_id = engine.cache.store_or_fuse(
        "runtime_kv:l2h1:0-1 :: inspect runtime KV",
        vec![0.1, 0.2, 0.3, 0.4, 0.5],
        0.95,
    );
    engine.cache.reinforce(memory_id, 0.8);
    engine.cache.reinforce(runtime_kv_memory_id, 0.9);
    engine.evolution_ledger = EvolutionLedger {
        live_inference_runs: 3,
        live_router_threshold_mutations: 2,
        live_hierarchy_weight_mutations: 1,
        live_router_threshold_delta: 0.05,
        live_hierarchy_weight_delta: 0.04,
        live_online_reward_feedbacks: 3,
        live_online_reward_reinforcements: 2,
        live_online_reward_penalties: 1,
        live_online_reward_strength: 1.85,
        live_online_reward_reinforcement_strength: 1.20,
        live_online_reward_penalty_strength: 0.65,
        live_memory_reinforcements: 4,
        live_memory_penalties: 1,
        live_stored_memories: 2,
        live_stored_gist_memories: 3,
        live_stored_runtime_kv_memories: 1,
        live_reflection_issues: 5,
        live_critical_reflection_issues: 1,
        live_revision_actions: 6,
        replay_runs: 2,
        replay_items: 5,
        router_threshold_mutations: 3,
        hierarchy_weight_mutations: 4,
        router_threshold_delta: 0.17,
        hierarchy_weight_delta: 0.08,
        memory_reinforcements: 6,
        memory_penalties: 1,
        replay_live_memory_feedback_items: 2,
        replay_live_memory_feedback_reinforcements: 2,
        replay_live_memory_feedback_penalties: 1,
        replay_live_memory_feedback_detail_items: 2,
        replay_live_memory_feedback_applied: 3,
        replay_live_memory_feedback_removed: 1,
        replay_live_memory_feedback_missing: 1,
        replay_live_memory_feedback_strength_delta: 0.52,
        replay_live_evolution_items: 2,
        replay_live_evolution_router_threshold_mutations: 1,
        replay_live_evolution_hierarchy_weight_mutations: 1,
        replay_live_evolution_router_threshold_delta: 0.04,
        replay_live_evolution_hierarchy_weight_delta: 0.03,
        replay_live_evolution_online_reward_feedbacks: 2,
        replay_live_evolution_online_reward_reinforcements: 1,
        replay_live_evolution_online_reward_penalties: 1,
        replay_live_evolution_online_reward_strength: 1.25,
        replay_live_evolution_online_reward_reinforcement_strength: 0.75,
        replay_live_evolution_online_reward_penalty_strength: 0.50,
        replay_live_evolution_memory_updates: 3,
        replay_live_evolution_stored_memory_updates: 2,
        replay_live_evolution_reflection_issues: 2,
        replay_live_evolution_critical_reflection_issues: 1,
        replay_live_evolution_revision_actions: 2,
        recursive_replay_items: 8,
        recursive_runtime_calls: 9,
        drift_rollbacks: 2,
        rollback_router_threshold_delta: 0.03,
        rollback_hierarchy_weight_delta: 0.04,
        external_feedbacks: 2,
        external_feedback_reinforcements: 3,
        external_feedback_penalties: 1,
        external_feedback_memory_updates: 4,
        external_feedback_removed: 1,
        external_feedback_missing: 2,
        external_feedback_strength_delta: 0.31,
        replay_rust_check_items: 1,
        replay_rust_check_passed: 1,
        replay_rust_check_failed: 0,
        replay_rust_check_diagnostic_chars: 0,
        replay_rust_check_live_memory_feedback_items: 1,
        replay_rust_check_live_memory_feedback_updates: 2,
        replay_rust_check_live_memory_feedback_applied: 2,
        replay_rust_check_live_memory_feedback_strength_delta: 0.36,
        replay_business_contract_items: 3,
        replay_business_contract_passed: 3,
        replay_business_contract_failed: 0,
        replay_business_contract_raw_passed: 1,
        replay_business_contract_raw_failed: 2,
        replay_business_contract_response_normalized: 2,
        replay_business_contract_sanitized: 0,
        replay_business_contract_canonical_fallbacks: 2,
    };
    engine.set_memory_retention_policy(MemoryRetentionPolicy {
        stale_after: 12,
        decay_rate: 0.12,
        remove_below_strength: 0.08,
        remove_after_failures: 7,
    });
    engine.set_memory_compaction_policy(MemoryCompactionPolicy {
        similarity_threshold: 0.91,
        max_candidates: 64,
        max_merges: 4,
    });
    engine.experience.record(ExperienceInput {
        prompt: "inspect state".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "state inspection should expose learned control decisions".to_owned(),
        quality: 0.91,
        contradictions: Vec::new(),
        reflection_issues: vec![ReflectionIssue::new(
            "needs_grounding",
            ReflectionSeverity::Warning,
            "inspect warning",
        )],
        revision_actions: vec!["increase_prompt_grounding".to_owned()],
        stored_memory_id: Some(memory_id),
        router_threshold_after: 0.62,
        stream_windows: 2,
        route_budget: RouteBudget {
            threshold: 0.62,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: vec![memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: crate::reflection::RuntimeDiagnostics {
            model_id: Some("inspect-runtime".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            device_profile: Some("cpu".to_owned()),
            primary_lane: Some("cpu-vector".to_owned()),
            fallback_lane: Some("cpu-portable".to_owned()),
            memory_mode: Some("tiered-disk".to_owned()),
            device_execution_source: Some(
                crate::reflection::RuntimeDiagnostics::runtime_reported_device_execution_source()
                    .to_owned(),
            ),
            layer_count: 12,
            global_layers: 3,
            local_window_layers: 6,
            convolutional_fusion_layers: 3,
            hidden_size: 128,
            local_window_tokens: 4096,
            forward_energy: Some(0.34),
            kv_influence: Some(0.56),
            imported_kv_blocks: 2,
            exported_kv_blocks: 3,
            hot_kv_precision_bits: Some(8),
            cold_kv_precision_bits: Some(4),
            ..crate::reflection::RuntimeDiagnostics::default()
        },
        runtime_token_metrics: ExperienceRuntimeTokenMetrics {
            token_count: 4,
            entropy_count: 4,
            logprob_count: 3,
            average_entropy: Some(0.42),
            average_neg_logprob: Some(0.70),
            uncertainty_perplexity: Some(4.38),
        },
        process_reward: ProcessRewardReport {
            total: 0.88,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![
                "recursive:chunks=5:merge_rounds=2:waves=3:parallel=2:runtime_calls=9"
                    .to_owned(),
                "memory_feedback:reinforced=2:penalized=1:reinforcement_amount=1.400000:penalty_amount=0.300000:applied=2:removed=0:missing=1:strength_delta=0.510000"
                    .to_owned(),
                "rust_check:passed=true:label=rustc_passed:edition=2021:status_code=0:diagnostic_chars=42"
                    .to_owned(),
                "business_contract:case=gemma-service-rust-feedback:passed=true:required=4:matched=4:missing=0:has_runtime_model_experiences=true:protocol_leak=false:substituted_runtime_model_experiences=false:evasive_denial=false:handling_signal=true:raw_passed=false:normalization=canonical_fallback:response_normalized=true:canonical_fallback=true"
                    .to_owned(),
                "pool_dispatch:selected_role=review:selected_port=8688:selected_endpoint=http://127.0.0.1:8688:context_window=8192:default_max_tokens=1024:configured_max_tokens=4096:effective_max_tokens=1024:max_tokens_clamped=true:low_priority=true:forwarded=true:dispatch_mode=runtime_endpoint_override:dispatch_reason=selected_worker_ready"
                    .to_owned(),
                "runtime_error:label=runtime_error:timeout=true:message_chars=48".to_owned(),
            ],
        },
        live_evolution: Default::default(),
    });
    engine.experience.record(ExperienceInput {
        prompt: "inspect critical reflection state".to_owned(),
        profile: TaskProfile::General,
        lesson: "critical reflection diagnostics should remain inspectable".to_owned(),
        quality: 0.42,
        contradictions: vec!["unsupported claim".to_owned()],
        reflection_issues: vec![ReflectionIssue::new(
            "unsupported_claim",
            ReflectionSeverity::Critical,
            "critical inspect issue",
        )],
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.48,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.48,
            attention_tokens: 1,
            fast_tokens: 2,
            attention_fraction: 0.33,
        },
        hierarchy: HierarchyWeights::new(0.34, 0.33, 0.33),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: crate::reflection::RuntimeDiagnostics::default(),
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.12,
            action: RewardAction::Penalize,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });

    let report = StateInspectionReport::from_engine(&engine, 3);

    assert_eq!(report.memory_count, 3);
    assert_eq!(report.runtime_kv_memory_count, 1);
    assert_eq!(report.experience_count, 2);
    assert_eq!(report.process_reward_experience_count, 2);
    assert_eq!(report.process_reward_positive_count, 2);
    assert_eq!(report.process_reward_reinforce_count, 1);
    assert_eq!(report.process_reward_hold_count, 0);
    assert_eq!(report.process_reward_penalize_count, 1);
    assert!((report.process_reward_total - 1.0).abs() < 0.0001);
    assert_eq!(report.runtime_model_experience_count, 1);
    assert_eq!(report.runtime_adapter_experience_count, 1);
    assert_eq!(report.runtime_adapter_selection_mismatch_count, 0);
    assert_eq!(report.runtime_forward_energy_experience_count, 1);
    assert_eq!(report.runtime_kv_influence_experience_count, 1);
    assert_eq!(report.runtime_uncertainty_experience_count, 1);
    assert_eq!(report.runtime_uncertainty_token_count, 7);
    assert_eq!(report.runtime_kv_precision_experience_count, 1);
    assert_eq!(report.runtime_kv_precision_mismatch_count, 0);
    assert_eq!(report.runtime_device_execution_experience_count, 1);
    assert_eq!(report.runtime_kv_import_experience_count, 1);
    assert_eq!(report.runtime_imported_kv_blocks, 2);
    assert_eq!(report.runtime_kv_export_experience_count, 1);
    assert_eq!(report.runtime_kv_hold_experience_count, 1);
    assert_eq!(report.runtime_kv_held_blocks, 3);
    assert_eq!(report.reflection_issue_experience_count, 2);
    assert_eq!(report.critical_reflection_issue_experience_count, 1);
    assert_eq!(report.revision_action_experience_count, 1);
    assert_eq!(report.live_memory_feedback_experience_count, 1);
    assert_eq!(report.live_memory_feedback_update_count, 3);
    assert_eq!(report.live_memory_feedback_detail_experience_count, 1);
    assert_eq!(report.live_memory_feedback_applied_count, 2);
    assert_eq!(report.live_memory_feedback_removed_count, 0);
    assert_eq!(report.live_memory_feedback_missing_count, 1);
    assert!((report.live_memory_feedback_strength_delta - 0.51).abs() < 0.0001);
    assert_eq!(report.runtime_error_experience_count, 1);
    assert_eq!(report.runtime_error_count, 1);
    assert_eq!(report.runtime_timeout_experience_count, 1);
    assert_eq!(report.runtime_timeout_count, 1);
    assert_eq!(report.runtime_error_message_chars, 48);
    assert_eq!(report.rust_check_experience_count, 1);
    assert_eq!(report.rust_check_passed_count, 1);
    assert_eq!(report.rust_check_failed_count, 0);
    assert_eq!(report.rust_check_diagnostic_chars, 42);
    assert_eq!(report.business_contract_experience_count, 1);
    assert_eq!(report.business_contract_passed_count, 1);
    assert_eq!(report.business_contract_failed_count, 0);
    assert_eq!(report.business_contract_required_signals, 4);
    assert_eq!(report.business_contract_matched_signals, 4);
    assert_eq!(report.business_contract_missing_signals, 0);
    assert_eq!(report.business_contract_protocol_leaks, 0);
    assert_eq!(report.business_contract_substitutions, 0);
    assert_eq!(report.business_contract_evasive_denials, 0);
    assert_eq!(report.business_contract_missing_handling_signals, 0);
    assert_eq!(report.business_contract_raw_passed_count, 0);
    assert_eq!(report.business_contract_raw_failed_count, 1);
    assert_eq!(report.business_contract_response_normalized_count, 1);
    assert_eq!(report.business_contract_sanitized_count, 0);
    assert_eq!(report.business_contract_canonical_fallback_count, 1);
    assert_eq!(report.pool_dispatch_experience_count, 1);
    assert_eq!(report.pool_dispatch_item_count, 1);
    assert_eq!(report.pool_dispatch_forwarded_count, 1);
    assert_eq!(report.pool_dispatch_clamped_count, 1);
    assert_eq!(report.pool_dispatch_low_priority_count, 1);
    assert!(report
        .top_memories
        .iter()
        .any(|memory| memory.id == memory_id
            && memory.key.contains("inspectable")
            && memory.vector_dimensions == 3));
    assert_eq!(report.top_runtime_kv_memories.len(), 1);
    assert_eq!(report.top_runtime_kv_memories[0].id, runtime_kv_memory_id);
    assert!(report.top_runtime_kv_memories[0]
        .key
        .starts_with("runtime_kv:"));
    assert_eq!(report.top_runtime_kv_memories[0].vector_dimensions, 5);
    assert!(report
        .top_memories
        .iter()
        .any(|memory| memory.id == fallback_memory_id && memory.vector_dimensions == 4));
    assert_eq!(
        report.memory_vector_dimensions,
        vec![
            StateMemoryVectorDimensions {
                dimensions: 3,
                count: 1
            },
            StateMemoryVectorDimensions {
                dimensions: 4,
                count: 1
            },
            StateMemoryVectorDimensions {
                dimensions: 5,
                count: 1
            }
        ]
    );
    assert_eq!(
        report.runtime_kv_vector_dimensions,
        vec![StateMemoryVectorDimensions {
            dimensions: 5,
            count: 1
        }]
    );
    assert_eq!(report.memory_retention_policy.stale_after, 12);
    assert_eq!(report.memory_compaction_policy.max_merges, 4);
    assert_eq!(report.evolution_ledger.replay_runs, 2);
    assert_eq!(report.evolution_ledger.live_inference_runs, 3);
    assert_eq!(report.evolution_ledger.live_memory_updates(), 5);
    assert_eq!(report.evolution_ledger.live_stored_memory_updates(), 6);
    assert_eq!(report.evolution_ledger.live_revision_actions, 6);
    assert_eq!(report.evolution_ledger.memory_updates(), 7);
    assert_eq!(report.evolution_ledger.external_feedbacks, 2);
    assert_eq!(report.evolution_ledger.external_feedback_reinforcements, 3);
    assert_eq!(report.evolution_ledger.external_feedback_penalties, 1);
    assert_eq!(report.evolution_ledger.external_feedback_memory_updates, 4);
    assert_eq!(report.evolution_ledger.external_feedback_removed, 1);
    assert_eq!(report.evolution_ledger.external_feedback_missing, 2);
    assert!(
        (report.evolution_ledger.external_feedback_strength_delta - 0.31).abs()
            < STATE_INSPECTION_FLOAT_EPSILON
    );
    assert_eq!(report.evolution_ledger.replay_rust_check_items, 1);
    assert_eq!(report.evolution_ledger.replay_rust_check_passed, 1);
    assert_eq!(report.evolution_ledger.replay_rust_check_failed, 0);
    assert_eq!(
        report
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_updates,
        2
    );
    assert_eq!(
        report
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_applied,
        2
    );
    assert_eq!(report.evolution_ledger.replay_business_contract_items, 3);
    assert_eq!(report.evolution_ledger.replay_business_contract_passed, 3);
    assert_eq!(report.evolution_ledger.replay_business_contract_failed, 0);
    assert_eq!(
        report.evolution_ledger.replay_business_contract_raw_passed,
        1
    );
    assert_eq!(
        report.evolution_ledger.replay_business_contract_raw_failed,
        2
    );
    assert_eq!(
        report
            .evolution_ledger
            .replay_business_contract_response_normalized,
        2
    );
    assert_eq!(
        report
            .evolution_ledger
            .replay_business_contract_canonical_fallbacks,
        2
    );
    assert_eq!(report.evolution_ledger.recursive_replay_items, 8);
    assert_eq!(report.evolution_ledger.recursive_runtime_calls, 9);
    assert_eq!(report.evolution_ledger.drift_rollbacks, 2);
    assert_eq!(
        report.top_experiences[0].reward_action,
        RewardAction::Reinforce
    );
    assert_eq!(
        report.top_experiences[0].runtime_model_id.as_deref(),
        Some("inspect-runtime")
    );
    assert_eq!(
        report.top_experiences[0]
            .runtime_selected_adapter
            .as_deref(),
        Some("portable-rust")
    );
    assert_eq!(
        report.top_experiences[0].runtime_device_profile.as_deref(),
        Some("cpu")
    );
    assert_eq!(
        report.top_experiences[0].runtime_primary_lane.as_deref(),
        Some("cpu-vector")
    );
    assert_eq!(
        report.top_experiences[0].runtime_fallback_lane.as_deref(),
        Some("cpu-portable")
    );
    assert_eq!(
        report.top_experiences[0].runtime_memory_mode.as_deref(),
        Some("tiered-disk")
    );
    assert_eq!(report.top_experiences[0].runtime_layer_count, 12);
    assert_eq!(report.top_experiences[0].runtime_global_layers, 3);
    assert_eq!(report.top_experiences[0].runtime_local_window_layers, 6);
    assert_eq!(
        report.top_experiences[0].runtime_convolutional_fusion_layers,
        3
    );
    assert_eq!(report.top_experiences[0].runtime_hidden_size, 128);
    assert_eq!(report.top_experiences[0].runtime_local_window_tokens, 4096);
    assert_eq!(report.top_experiences[0].runtime_forward_energy, Some(0.34));
    assert_eq!(report.top_experiences[0].runtime_kv_influence, Some(0.56));
    assert_eq!(report.top_experiences[0].runtime_token_count, 4);
    assert_eq!(report.top_experiences[0].runtime_uncertainty_token_count, 7);
    assert_eq!(
        report.top_experiences[0].runtime_uncertainty_perplexity,
        Some(4.38)
    );
    assert_eq!(report.top_experiences[0].runtime_imported_kv_blocks, 2);
    assert_eq!(report.top_experiences[0].runtime_exported_kv_blocks, 3);
    assert_eq!(report.top_experiences[0].recursive_runtime_calls, Some(9));
    assert_eq!(report.top_experiences[0].live_memory_feedback_updates, 3);
    assert_eq!(report.top_experiences[0].live_memory_feedback_reinforced, 2);
    assert_eq!(report.top_experiences[0].live_memory_feedback_penalized, 1);
    assert_eq!(report.top_experiences[0].live_memory_feedback_applied, 2);
    assert_eq!(report.top_experiences[0].live_memory_feedback_removed, 0);
    assert_eq!(report.top_experiences[0].live_memory_feedback_missing, 1);
    assert!((report.top_experiences[0].live_memory_feedback_strength_delta - 0.51).abs() < 0.0001);
    assert!(report.top_experiences[0].live_memory_feedback_detail);
    assert_eq!(report.top_experiences[0].runtime_errors, 1);
    assert_eq!(report.top_experiences[0].runtime_timeouts, 1);
    assert_eq!(report.top_experiences[0].runtime_error_message_chars, 48);
    assert_eq!(report.top_experiences[0].rust_check_passed, 1);
    assert_eq!(report.top_experiences[0].rust_check_failed, 0);
    assert_eq!(report.top_experiences[0].rust_check_diagnostic_chars, 42);
    assert_eq!(report.top_experiences[0].business_contract_passed, 1);
    assert_eq!(report.top_experiences[0].business_contract_failed, 0);
    assert_eq!(
        report.top_experiences[0].business_contract_missing_signals,
        0
    );
    assert_eq!(
        report.top_experiences[0].business_contract_protocol_leaks,
        0
    );
    assert_eq!(report.top_experiences[0].business_contract_substitutions, 0);
    assert_eq!(
        report.top_experiences[0].business_contract_evasive_denials,
        0
    );
    assert_eq!(
        report.top_experiences[0].business_contract_missing_handling_signals,
        0
    );
    assert_eq!(report.top_experiences[0].business_contract_raw_passed, 0);
    assert_eq!(report.top_experiences[0].business_contract_raw_failed, 1);
    assert_eq!(
        report.top_experiences[0].business_contract_response_normalized,
        1
    );
    assert_eq!(report.top_experiences[0].business_contract_sanitized, 0);
    assert_eq!(
        report.top_experiences[0].business_contract_canonical_fallbacks,
        1
    );
    assert_eq!(report.top_experiences[0].pool_dispatch_items, 1);
    assert_eq!(
        report.top_experiences[0].pool_dispatch_selected_roles,
        vec!["review".to_owned()]
    );
    assert_eq!(report.top_experiences[0].pool_dispatch_forwarded, 1);
    assert_eq!(report.top_experiences[0].pool_dispatch_clamped, 1);
    assert_eq!(report.top_experiences[0].pool_dispatch_low_priority, 1);
    assert_eq!(report.top_experiences[0].reflection_issues, 1);
    assert_eq!(report.top_experiences[0].revision_actions, 1);
    assert!(report.summary_line().contains("memories=3"));
    assert!(report.summary_line().contains("runtime_kv_memories=1"));
    assert!(report
        .summary_line()
        .contains("process_reward_experiences=2"));
    assert!(report.summary_line().contains("process_reward_positive=2"));
    assert!(report.summary_line().contains("process_reward_reinforce=1"));
    assert!(report.summary_line().contains("process_reward_penalize=1"));
    assert!(report
        .summary_line()
        .contains("process_reward_total=1.000000"));
    assert!(report
        .summary_line()
        .contains("runtime_model_experiences=1"));
    assert!(report
        .summary_line()
        .contains("runtime_adapter_experiences=1"));
    assert!(report
        .summary_line()
        .contains("runtime_adapter_selection_mismatches=0"));
    assert!(report
        .summary_line()
        .contains("runtime_forward_energy_experiences=1"));
    assert!(report
        .summary_line()
        .contains("runtime_kv_influence_experiences=1"));
    assert!(report
        .summary_line()
        .contains("runtime_uncertainty_experiences=1"));
    assert!(report
        .summary_line()
        .contains("runtime_uncertainty_tokens=7"));
    assert!(report
        .summary_line()
        .contains("runtime_kv_precision_experiences=1"));
    assert!(report
        .summary_line()
        .contains("runtime_kv_precision_mismatches=0"));
    assert!(report
        .summary_line()
        .contains("runtime_device_execution_experiences=1"));
    assert_eq!(report.runtime_layer_mode_experience_count, 1);
    assert_eq!(report.runtime_all_layer_mode_experience_count, 1);
    assert_eq!(report.runtime_global_layers, 3);
    assert_eq!(report.runtime_local_window_layers, 6);
    assert_eq!(report.runtime_convolutional_fusion_layers, 3);
    assert!(report
        .summary_line()
        .contains("runtime_layer_mode_experiences=1"));
    assert!(report
        .summary_line()
        .contains("runtime_all_layer_mode_experiences=1"));
    assert!(report.summary_line().contains("runtime_global_layers=3"));
    assert!(report
        .summary_line()
        .contains("runtime_local_window_layers=6"));
    assert!(report
        .summary_line()
        .contains("runtime_convolutional_fusion_layers=3"));
    assert!(report
        .summary_line()
        .contains("runtime_kv_import_experiences=1"));
    assert!(report
        .summary_line()
        .contains("runtime_imported_kv_blocks=2"));
    assert!(report
        .summary_line()
        .contains("runtime_kv_export_experiences=1"));
    assert!(report
        .summary_line()
        .contains("runtime_kv_hold_experiences=1"));
    assert!(report.summary_line().contains("runtime_kv_held_blocks=3"));
    assert!(report
        .summary_line()
        .contains("runtime_error_experiences=1"));
    assert!(report.summary_line().contains("runtime_errors=1"));
    assert!(report
        .summary_line()
        .contains("runtime_timeout_experiences=1"));
    assert!(report.summary_line().contains("runtime_timeouts=1"));
    assert!(report
        .summary_line()
        .contains("runtime_error_message_chars=48"));
    assert!(report
        .summary_line()
        .contains("reflection_issue_experiences=2"));
    assert!(report
        .summary_line()
        .contains("critical_reflection_issue_experiences=1"));
    assert!(report
        .summary_line()
        .contains("revision_action_experiences=1"));
    assert!(report
        .summary_line()
        .contains("live_memory_feedback_experiences=1"));
    assert!(report
        .summary_line()
        .contains("live_memory_feedback_updates=3"));
    assert!(report
        .summary_line()
        .contains("live_memory_feedback_reinforced=2"));
    assert!(report
        .summary_line()
        .contains("live_memory_feedback_penalized=1"));
    assert!(report
        .summary_line()
        .contains("live_memory_feedback_detail_experiences=1"));
    assert!(report
        .summary_line()
        .contains("live_memory_feedback_applied=2"));
    assert!(report
        .summary_line()
        .contains("live_memory_feedback_missing=1"));
    assert!(report
        .summary_line()
        .contains("live_memory_feedback_strength_delta=0.510000"));
    assert!(report.summary_line().contains("rust_check_experiences=1"));
    assert!(report.summary_line().contains("rust_check_passed=1"));
    assert!(report.summary_line().contains("rust_check_failed=0"));
    assert!(report
        .summary_line()
        .contains("rust_check_diagnostic_chars=42"));
    assert!(report
        .summary_line()
        .contains("business_contract_experiences=1"));
    assert!(report.summary_line().contains("business_contract_passed=1"));
    assert!(report.summary_line().contains("business_contract_failed=0"));
    assert!(report
        .summary_line()
        .contains("business_contract_required_signals=4"));
    assert!(report
        .summary_line()
        .contains("business_contract_matched_signals=4"));
    assert!(report
        .summary_line()
        .contains("business_contract_missing_signals=0"));
    assert!(report
        .summary_line()
        .contains("business_contract_raw_failed=1"));
    assert!(report
        .summary_line()
        .contains("business_contract_response_normalized=1"));
    assert!(report
        .summary_line()
        .contains("business_contract_canonical_fallbacks=1"));
    assert!(report
        .summary_line()
        .contains("pool_dispatch_experiences=1"));
    assert!(report.summary_line().contains("pool_dispatch_items=1"));
    assert!(report.summary_line().contains("pool_dispatch_forwarded=1"));
    assert!(report.summary_line().contains("pool_dispatch_clamped=1"));
    assert!(report
        .summary_line()
        .contains("pool_dispatch_low_priority=1"));
    assert!(report
        .summary_line()
        .contains("memory_vector_dimensions=3:1|4:1|5:1"));
    assert!(report
        .summary_line()
        .contains("runtime_kv_vector_dimensions=5:1"));
    assert!(report
        .summary_line()
        .contains("evolution_router_threshold_mutations=3"));
    assert!(report
        .summary_line()
        .contains("evolution_hierarchy_weight_mutations=4"));
    assert!(report
        .summary_line()
        .contains("evolution_router_threshold_delta=0.170000"));
    assert!(report
        .summary_line()
        .contains("evolution_hierarchy_weight_delta=0.080000"));
    assert!(report.summary_line().contains("evolution_memory_updates=7"));
    assert!(report
        .summary_line()
        .contains("evolution_external_feedbacks=2"));
    assert!(report
        .summary_line()
        .contains("evolution_external_feedback_memory_updates=4"));
    assert!(report
        .summary_line()
        .contains("evolution_external_feedback_strength_delta=0.310000"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_rust_check_items=1"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_rust_check_passed=1"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_rust_check_live_memory_feedback_updates=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_business_contract_items=3"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_business_contract_passed=3"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_business_contract_raw_failed=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_business_contract_raw_audits=3"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_business_contract_response_normalized=2"));
    assert!(report
        .summary_line()
        .contains("evolution_live_inference_runs=3"));
    assert!(report
        .summary_line()
        .contains("evolution_live_online_reward_feedbacks=3"));
    assert!(report
        .summary_line()
        .contains("evolution_live_online_reward_strength=1.850000"));
    assert!(report
        .summary_line()
        .contains("evolution_live_online_reward_reinforcement_strength=1.200000"));
    assert!(report
        .summary_line()
        .contains("evolution_live_online_reward_penalty_strength=0.650000"));
    assert!(report
        .summary_line()
        .contains("evolution_live_memory_updates=5"));
    assert!(report
        .summary_line()
        .contains("evolution_live_stored_memory_updates=6"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_memory_feedback_updates=3"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_memory_feedback_detail_items=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_memory_feedback_applied=3"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_memory_feedback_strength_delta=0.520000"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_items=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_online_reward_feedbacks=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_online_reward_strength=1.250000"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_online_reward_reinforcement_strength=0.750000"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_online_reward_penalty_strength=0.500000"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_memory_updates=3"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_stored_memory_updates=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_reflection_issues=2"));
    assert!(report
        .summary_line()
        .contains("evolution_recursive_replay_items=8"));
    assert!(report
        .summary_line()
        .contains("evolution_drift_rollbacks=2"));
    assert!(report
        .summary_line()
        .contains("evolution_rollback_router_threshold_delta=0.030000"));

    let passing_gate = StateInspectionGate {
        min_memories: Some(3),
        min_runtime_kv_memories: Some(1),
        min_experiences: Some(1),
        max_experience_hygiene_quarantine_candidates: Some(0),
        max_experience_repairable_legacy_metadata_lessons: Some(0),
        max_experience_repairable_index_records: Some(0),
        max_experience_repair_projected_legacy_metadata_lessons: Some(0),
        max_experience_repair_skipped_missing_clean_gist: Some(0),
        max_experience_index_overlong_records: None,
        max_experience_index_overlong_without_clean_gist: None,
        max_experience_index_record_chars: None,
        max_experience_index_noisy_records: Some(0),
        max_experience_index_noise_penalty: Some(0.0),
        min_experience_index_quality_score: None,
        require_experience_index_retrieval_ready: false,
        min_runtime_model_experiences: Some(1),
        min_runtime_adapter_experiences: Some(1),
        max_runtime_adapter_selection_mismatches: Some(0),
        min_runtime_forward_energy_experiences: Some(1),
        min_runtime_kv_influence_experiences: Some(1),
        min_runtime_tokens: Some(4),
        min_runtime_uncertainty_experiences: Some(1),
        min_runtime_uncertainty_tokens: Some(7),
        min_runtime_architecture_experiences: Some(1),
        min_runtime_kv_precision_experiences: Some(1),
        max_runtime_kv_precision_mismatches: Some(0),
        max_runtime_errors: Some(1),
        max_runtime_timeouts: Some(1),
        max_runtime_error_message_chars: Some(48),
        min_runtime_device_execution_experiences: Some(1),
        min_runtime_layer_mode_experiences: Some(1),
        min_runtime_all_layer_mode_experiences: Some(1),
        min_runtime_global_layers: Some(3),
        min_runtime_local_window_layers: Some(6),
        min_runtime_convolutional_fusion_layers: Some(3),
        min_runtime_kv_import_experiences: Some(1),
        min_runtime_imported_kv_blocks: Some(2),
        min_self_evolving_memory_writeback_experiences: None,
        min_self_evolving_memory_writeback_attempted_records: None,
        min_self_evolving_memory_writeback_accepted_records: None,
        max_self_evolving_memory_writeback_rejected_records: None,
        min_self_evolving_memory_writeback_write_allowed: None,
        min_self_evolving_memory_writeback_durable_write_allowed: None,
        min_self_evolving_memory_writeback_applied: None,
        min_self_evolving_memory_writeback_applied_to_disk: None,
        min_self_evolving_memory_writeback_snapshot_changes: None,
        min_runtime_kv_weak_import_skip_experiences: None,
        min_weak_runtime_kv_imports_skipped: None,
        min_runtime_kv_weak_import_pressure_experiences: None,
        min_runtime_kv_weak_import_pressure: None,
        max_runtime_kv_weak_import_pressure: None,
        min_runtime_kv_budget_import_skip_experiences: None,
        min_budget_limited_runtime_kv_imports_skipped: None,
        min_runtime_kv_budget_pressure_experiences: None,
        min_runtime_kv_budget_pressure: None,
        max_runtime_kv_budget_pressure: None,
        min_runtime_kv_export_experiences: Some(1),
        min_runtime_kv_segment_experiences: None,
        min_runtime_kv_segments_included: None,
        max_runtime_kv_segments_skipped: None,
        max_runtime_kv_segments_rejected: None,
        min_runtime_kv_hold_experiences: Some(1),
        min_runtime_kv_held_blocks: Some(3),
        min_fht_dke_budget_experiences: None,
        min_fht_dke_enabled_experiences: None,
        min_fht_dke_routed_tokens: None,
        max_fht_dke_token_split_invalid: None,
        min_fht_dke_attention_threshold: None,
        max_fht_dke_attention_threshold: None,
        min_fht_dke_route_pressure: None,
        max_fht_dke_route_pressure: None,
        min_process_reward_experiences: Some(2),
        min_process_reward_positive: Some(2),
        min_process_reward_reinforce: Some(1),
        min_process_reward_total: Some(1.0),
        max_pool_dispatch_clamped: Some(1),
        max_pool_dispatch_low_priority: Some(1),
        min_external_semantic_context_experiences: None,
        min_external_semantic_contexts: None,
        min_reflection_issue_experiences: Some(2),
        min_critical_reflection_issue_experiences: Some(1),
        min_revision_action_experiences: Some(1),
        min_live_memory_feedback_experiences: Some(1),
        min_live_memory_feedback_updates: Some(3),
        min_live_memory_feedback_reinforced: Some(2),
        min_live_memory_feedback_penalized: Some(1),
        min_live_memory_feedback_detail_experiences: Some(1),
        min_live_memory_feedback_applied: Some(2),
        max_live_memory_feedback_missing: Some(1),
        min_live_memory_feedback_strength_delta: Some(0.51),
        min_rust_check_experiences: Some(1),
        min_rust_check_passed: Some(1),
        max_rust_check_failed: Some(0),
        min_rust_check_diagnostic_chars: Some(42),
        min_business_contract_experiences: Some(1),
        min_business_contract_passed: Some(1),
        max_business_contract_failed: Some(0),
        max_business_contract_missing_signals: Some(0),
        max_business_contract_protocol_leaks: Some(0),
        max_business_contract_substitutions: Some(0),
        max_business_contract_evasive_denials: Some(0),
        max_business_contract_missing_handling_signals: Some(0),
        min_router_observations: Some(0),
        min_evolution_live_inference_runs: Some(3),
        min_evolution_live_router_threshold_mutations: Some(2),
        min_evolution_live_hierarchy_weight_mutations: Some(1),
        min_evolution_live_router_threshold_delta: Some(0.05),
        min_evolution_live_hierarchy_weight_delta: Some(0.04),
        min_evolution_live_online_reward_feedbacks: Some(3),
        min_evolution_live_online_reward_reinforcements: Some(2),
        min_evolution_live_online_reward_penalties: Some(1),
        min_evolution_live_online_reward_strength: Some(1.85),
        min_evolution_live_online_reward_reinforcement_strength: Some(1.20),
        min_evolution_live_online_reward_penalty_strength: Some(0.65),
        min_evolution_live_memory_reinforcements: Some(4),
        min_evolution_live_memory_penalties: Some(1),
        min_evolution_live_stored_memories: Some(2),
        min_evolution_live_stored_gist_memories: Some(3),
        min_evolution_live_stored_runtime_kv_memories: Some(1),
        min_evolution_live_memory_updates: Some(5),
        min_evolution_live_stored_memory_updates: Some(6),
        min_evolution_live_reflection_issues: Some(5),
        min_evolution_live_critical_reflection_issues: Some(1),
        min_evolution_live_revision_actions: Some(6),
        min_evolution_replay_runs: Some(2),
        min_evolution_replay_items: Some(5),
        min_evolution_router_threshold_mutations: Some(3),
        min_evolution_hierarchy_weight_mutations: Some(4),
        min_evolution_router_threshold_delta: Some(0.17),
        min_evolution_hierarchy_weight_delta: Some(0.08),
        min_evolution_memory_updates: Some(7),
        min_evolution_external_feedbacks: Some(2),
        min_evolution_external_feedback_reinforcements: Some(3),
        min_evolution_external_feedback_penalties: Some(1),
        min_evolution_external_feedback_memory_updates: Some(4),
        min_evolution_external_feedback_strength_delta: Some(0.31),
        max_evolution_external_feedback_missing: Some(2),
        min_evolution_replay_live_memory_feedback_updates: Some(3),
        min_evolution_replay_live_memory_feedback_reinforcements: Some(2),
        min_evolution_replay_live_memory_feedback_penalties: Some(1),
        min_evolution_replay_live_memory_feedback_detail_items: Some(2),
        min_evolution_replay_live_memory_feedback_applied: Some(3),
        max_evolution_replay_live_memory_feedback_missing: Some(1),
        min_evolution_replay_live_memory_feedback_strength_delta: Some(0.52),
        min_evolution_replay_rust_check_items: Some(1),
        min_evolution_replay_rust_check_passed: Some(1),
        max_evolution_replay_rust_check_failed: Some(0),
        min_evolution_replay_rust_check_live_memory_feedback_updates: Some(2),
        min_evolution_replay_rust_check_live_memory_feedback_applied: Some(2),
        min_evolution_replay_rust_check_live_memory_feedback_strength_delta: Some(0.36),
        min_evolution_replay_business_contract_items: Some(3),
        min_evolution_replay_business_contract_passed: Some(3),
        max_evolution_replay_business_contract_failed: Some(0),
        min_evolution_replay_business_contract_raw_audits: Some(3),
        min_evolution_replay_live_evolution_items: Some(2),
        min_evolution_replay_live_evolution_online_reward_feedbacks: Some(2),
        min_evolution_replay_live_evolution_online_reward_reinforcements: Some(1),
        min_evolution_replay_live_evolution_online_reward_penalties: Some(1),
        min_evolution_replay_live_evolution_online_reward_strength: Some(1.25),
        min_evolution_replay_live_evolution_online_reward_reinforcement_strength: Some(0.75),
        min_evolution_replay_live_evolution_online_reward_penalty_strength: Some(0.50),
        min_evolution_replay_live_evolution_memory_updates: Some(3),
        min_evolution_replay_live_evolution_stored_memory_updates: Some(2),
        min_evolution_replay_live_evolution_reflection_issues: Some(2),
        min_evolution_replay_live_evolution_critical_reflection_issues: Some(1),
        min_evolution_replay_live_evolution_revision_actions: Some(2),
        min_evolution_recursive_replay_items: Some(8),
        min_evolution_recursive_runtime_calls: Some(9),
        max_evolution_drift_rollbacks: Some(2),
        max_evolution_rollback_router_threshold_delta: Some(0.03),
        max_evolution_rollback_hierarchy_weight_delta: Some(0.04),
        require_runtime_kv_dimensions: true,
    };
    let passing_report = report.evaluate(&passing_gate);
    assert!(passing_report.passed());
    assert_eq!(
        passing_report.summary_line(),
        "state_inspection_gate: passed=true failures=0"
    );

    let failing_gate = StateInspectionGate {
        min_memories: Some(4),
        min_runtime_kv_memories: Some(2),
        min_experiences: Some(2),
        max_experience_hygiene_quarantine_candidates: Some(0),
        max_experience_repairable_legacy_metadata_lessons: Some(0),
        max_experience_repairable_index_records: Some(0),
        max_experience_repair_projected_legacy_metadata_lessons: Some(0),
        max_experience_repair_skipped_missing_clean_gist: Some(0),
        max_experience_index_overlong_records: None,
        max_experience_index_overlong_without_clean_gist: None,
        max_experience_index_record_chars: None,
        max_experience_index_noisy_records: Some(0),
        max_experience_index_noise_penalty: Some(0.0),
        min_experience_index_quality_score: None,
        require_experience_index_retrieval_ready: false,
        min_runtime_model_experiences: Some(2),
        min_runtime_adapter_experiences: Some(2),
        max_runtime_adapter_selection_mismatches: Some(0),
        min_runtime_forward_energy_experiences: Some(2),
        min_runtime_kv_influence_experiences: Some(2),
        min_runtime_tokens: Some(5),
        min_runtime_uncertainty_experiences: Some(2),
        min_runtime_uncertainty_tokens: Some(8),
        min_runtime_architecture_experiences: Some(2),
        min_runtime_kv_precision_experiences: Some(2),
        max_runtime_kv_precision_mismatches: Some(0),
        max_runtime_errors: Some(0),
        max_runtime_timeouts: Some(0),
        max_runtime_error_message_chars: Some(47),
        min_runtime_device_execution_experiences: Some(2),
        min_runtime_layer_mode_experiences: Some(2),
        min_runtime_all_layer_mode_experiences: Some(2),
        min_runtime_global_layers: Some(4),
        min_runtime_local_window_layers: Some(7),
        min_runtime_convolutional_fusion_layers: Some(4),
        min_runtime_kv_import_experiences: Some(2),
        min_runtime_imported_kv_blocks: Some(3),
        min_self_evolving_memory_writeback_experiences: None,
        min_self_evolving_memory_writeback_attempted_records: None,
        min_self_evolving_memory_writeback_accepted_records: None,
        max_self_evolving_memory_writeback_rejected_records: None,
        min_self_evolving_memory_writeback_write_allowed: None,
        min_self_evolving_memory_writeback_durable_write_allowed: None,
        min_self_evolving_memory_writeback_applied: None,
        min_self_evolving_memory_writeback_applied_to_disk: None,
        min_self_evolving_memory_writeback_snapshot_changes: None,
        min_runtime_kv_weak_import_skip_experiences: None,
        min_weak_runtime_kv_imports_skipped: None,
        min_runtime_kv_weak_import_pressure_experiences: None,
        min_runtime_kv_weak_import_pressure: None,
        max_runtime_kv_weak_import_pressure: None,
        min_runtime_kv_budget_import_skip_experiences: None,
        min_budget_limited_runtime_kv_imports_skipped: None,
        min_runtime_kv_budget_pressure_experiences: None,
        min_runtime_kv_budget_pressure: None,
        max_runtime_kv_budget_pressure: None,
        min_runtime_kv_export_experiences: Some(2),
        min_runtime_kv_segment_experiences: None,
        min_runtime_kv_segments_included: None,
        max_runtime_kv_segments_skipped: None,
        max_runtime_kv_segments_rejected: None,
        min_runtime_kv_hold_experiences: Some(2),
        min_runtime_kv_held_blocks: Some(4),
        min_fht_dke_budget_experiences: None,
        min_fht_dke_enabled_experiences: None,
        min_fht_dke_routed_tokens: None,
        max_fht_dke_token_split_invalid: None,
        min_fht_dke_attention_threshold: None,
        max_fht_dke_attention_threshold: None,
        min_fht_dke_route_pressure: None,
        max_fht_dke_route_pressure: None,
        min_process_reward_experiences: Some(3),
        min_process_reward_positive: Some(3),
        min_process_reward_reinforce: Some(2),
        min_process_reward_total: Some(1.01),
        max_pool_dispatch_clamped: Some(0),
        max_pool_dispatch_low_priority: Some(0),
        min_external_semantic_context_experiences: None,
        min_external_semantic_contexts: None,
        min_reflection_issue_experiences: Some(3),
        min_critical_reflection_issue_experiences: Some(2),
        min_revision_action_experiences: Some(2),
        min_live_memory_feedback_experiences: Some(2),
        min_live_memory_feedback_updates: Some(4),
        min_live_memory_feedback_reinforced: Some(3),
        min_live_memory_feedback_penalized: Some(2),
        min_live_memory_feedback_detail_experiences: Some(2),
        min_live_memory_feedback_applied: Some(3),
        max_live_memory_feedback_missing: Some(0),
        min_live_memory_feedback_strength_delta: Some(0.52),
        min_rust_check_experiences: Some(2),
        min_rust_check_passed: Some(2),
        max_rust_check_failed: Some(0),
        min_rust_check_diagnostic_chars: Some(43),
        min_business_contract_experiences: Some(2),
        min_business_contract_passed: Some(2),
        max_business_contract_failed: Some(0),
        max_business_contract_missing_signals: Some(0),
        max_business_contract_protocol_leaks: Some(0),
        max_business_contract_substitutions: Some(0),
        max_business_contract_evasive_denials: Some(0),
        max_business_contract_missing_handling_signals: Some(0),
        min_router_observations: Some(1),
        min_evolution_live_inference_runs: Some(4),
        min_evolution_live_router_threshold_mutations: Some(3),
        min_evolution_live_hierarchy_weight_mutations: Some(2),
        min_evolution_live_router_threshold_delta: Some(0.06),
        min_evolution_live_hierarchy_weight_delta: Some(0.05),
        min_evolution_live_online_reward_feedbacks: Some(4),
        min_evolution_live_online_reward_reinforcements: Some(3),
        min_evolution_live_online_reward_penalties: Some(2),
        min_evolution_live_online_reward_strength: Some(1.86),
        min_evolution_live_online_reward_reinforcement_strength: Some(1.21),
        min_evolution_live_online_reward_penalty_strength: Some(0.66),
        min_evolution_live_memory_reinforcements: Some(5),
        min_evolution_live_memory_penalties: Some(2),
        min_evolution_live_stored_memories: Some(3),
        min_evolution_live_stored_gist_memories: Some(4),
        min_evolution_live_stored_runtime_kv_memories: Some(2),
        min_evolution_live_memory_updates: Some(6),
        min_evolution_live_stored_memory_updates: Some(7),
        min_evolution_live_reflection_issues: Some(6),
        min_evolution_live_critical_reflection_issues: Some(2),
        min_evolution_live_revision_actions: Some(7),
        min_evolution_replay_runs: Some(3),
        min_evolution_replay_items: Some(6),
        min_evolution_router_threshold_mutations: Some(4),
        min_evolution_hierarchy_weight_mutations: Some(5),
        min_evolution_router_threshold_delta: Some(0.18),
        min_evolution_hierarchy_weight_delta: Some(0.09),
        min_evolution_memory_updates: Some(8),
        min_evolution_external_feedbacks: Some(3),
        min_evolution_external_feedback_reinforcements: Some(4),
        min_evolution_external_feedback_penalties: Some(2),
        min_evolution_external_feedback_memory_updates: Some(5),
        min_evolution_external_feedback_strength_delta: Some(0.32),
        max_evolution_external_feedback_missing: Some(1),
        min_evolution_replay_live_memory_feedback_updates: Some(4),
        min_evolution_replay_live_memory_feedback_reinforcements: Some(3),
        min_evolution_replay_live_memory_feedback_penalties: Some(2),
        min_evolution_replay_live_memory_feedback_detail_items: Some(3),
        min_evolution_replay_live_memory_feedback_applied: Some(4),
        max_evolution_replay_live_memory_feedback_missing: Some(0),
        min_evolution_replay_live_memory_feedback_strength_delta: Some(0.53),
        min_evolution_replay_rust_check_items: Some(2),
        min_evolution_replay_rust_check_passed: Some(2),
        max_evolution_replay_rust_check_failed: Some(0),
        min_evolution_replay_rust_check_live_memory_feedback_updates: Some(3),
        min_evolution_replay_rust_check_live_memory_feedback_applied: Some(3),
        min_evolution_replay_rust_check_live_memory_feedback_strength_delta: Some(0.37),
        min_evolution_replay_business_contract_items: Some(4),
        min_evolution_replay_business_contract_passed: Some(4),
        max_evolution_replay_business_contract_failed: Some(0),
        min_evolution_replay_business_contract_raw_audits: Some(4),
        min_evolution_replay_live_evolution_items: Some(3),
        min_evolution_replay_live_evolution_online_reward_feedbacks: Some(3),
        min_evolution_replay_live_evolution_online_reward_reinforcements: Some(2),
        min_evolution_replay_live_evolution_online_reward_penalties: Some(2),
        min_evolution_replay_live_evolution_online_reward_strength: Some(1.26),
        min_evolution_replay_live_evolution_online_reward_reinforcement_strength: Some(0.76),
        min_evolution_replay_live_evolution_online_reward_penalty_strength: Some(0.51),
        min_evolution_replay_live_evolution_memory_updates: Some(4),
        min_evolution_replay_live_evolution_stored_memory_updates: Some(3),
        min_evolution_replay_live_evolution_reflection_issues: Some(3),
        min_evolution_replay_live_evolution_critical_reflection_issues: Some(2),
        min_evolution_replay_live_evolution_revision_actions: Some(3),
        min_evolution_recursive_replay_items: Some(9),
        min_evolution_recursive_runtime_calls: Some(10),
        max_evolution_drift_rollbacks: Some(1),
        max_evolution_rollback_router_threshold_delta: Some(0.02),
        max_evolution_rollback_hierarchy_weight_delta: Some(0.03),
        require_runtime_kv_dimensions: true,
    };
    let failing_report = report.evaluate(&failing_gate);
    assert!(!failing_report.passed());
    assert!(failing_report
        .failures
        .contains(&"memory_count 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_kv_memory_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_model_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_forward_energy_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_uncertainty_token_count 7 below required 8".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_kv_import_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_imported_kv_blocks 2 below required 3".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_kv_hold_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_kv_held_blocks 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"process_reward_experience_count 2 below required 3".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"process_reward_positive_count 2 below required 3".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"process_reward_reinforce_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"process_reward_total 1.000000 below required 1.010000".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"pool_dispatch_clamped_count 1 above maximum 0".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"pool_dispatch_low_priority_count 1 above maximum 0".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_error_count 1 above maximum 0".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_timeout_count 1 above maximum 0".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_error_message_chars 48 above maximum 47".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_device_execution_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_layer_mode_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_all_layer_mode_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_global_layers 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_local_window_layers 6 below required 7".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"runtime_convolutional_fusion_layers 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"reflection_issue_experience_count 2 below required 3".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"critical_reflection_issue_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"revision_action_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"live_memory_feedback_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"live_memory_feedback_update_count 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"live_memory_feedback_reinforced_count 2 below required 3".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"live_memory_feedback_penalized_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"live_memory_feedback_detail_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"live_memory_feedback_applied_count 2 below required 3".to_owned()));
    assert!(failing_report.failures.contains(
        &"live_memory_feedback_strength_delta 0.510000 below required 0.520000".to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"live_memory_feedback_missing_count 1 above maximum 0".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"rust_check_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"rust_check_passed_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"rust_check_diagnostic_chars 42 below required 43".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"business_contract_experience_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"business_contract_passed_count 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_inference_runs 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_router_threshold_mutations 2 below required 3".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_hierarchy_weight_mutations 1 below required 2".to_owned()));
    assert!(failing_report.failures.contains(
        &"evolution_live_router_threshold_delta 0.050000 below required 0.060000".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_live_hierarchy_weight_delta 0.040000 below required 0.050000".to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_online_reward_feedbacks 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_online_reward_reinforcements 2 below required 3".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_online_reward_penalties 1 below required 2".to_owned()));
    assert!(failing_report.failures.contains(
        &"evolution_live_online_reward_strength 1.850000 below required 1.860000".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_live_online_reward_reinforcement_strength 1.200000 below required 1.210000"
            .to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_live_online_reward_penalty_strength 0.650000 below required 0.660000"
            .to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_memory_reinforcements 4 below required 5".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_memory_penalties 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_stored_memories 2 below required 3".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_stored_gist_memories 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_stored_runtime_kv_memories 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_memory_updates 5 below required 6".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_stored_memory_updates 6 below required 7".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_reflection_issues 5 below required 6".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_critical_reflection_issues 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_live_revision_actions 6 below required 7".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_router_threshold_delta 0.170000 below required 0.180000".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_hierarchy_weight_delta 0.080000 below required 0.090000".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_memory_updates 7 below required 8".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_external_feedbacks 2 below required 3".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_external_feedback_reinforcements 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_external_feedback_penalties 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_external_feedback_memory_updates 4 below required 5".to_owned()));
    assert!(failing_report.failures.contains(
        &"evolution_external_feedback_strength_delta 0.310000 below required 0.320000".to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"evolution_external_feedback_missing 2 above maximum 1".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_replay_live_memory_feedback_updates 3 below required 4".to_owned()));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_memory_feedback_reinforcements 2 below required 3".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_memory_feedback_penalties 1 below required 2".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_memory_feedback_detail_items 2 below required 3".to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"evolution_replay_live_memory_feedback_applied 3 below required 4".to_owned()));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_memory_feedback_strength_delta 0.520000 below required 0.530000"
            .to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"evolution_replay_live_memory_feedback_missing 1 above maximum 0".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_replay_rust_check_items 1 below required 2".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_replay_rust_check_passed 1 below required 2".to_owned()));
    assert!(failing_report.failures.contains(
        &"evolution_replay_rust_check_live_memory_feedback_updates 2 below required 3".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_rust_check_live_memory_feedback_applied 2 below required 3".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_rust_check_live_memory_feedback_strength_delta 0.360000 below required 0.370000"
            .to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"evolution_replay_business_contract_items 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_replay_business_contract_passed 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_replay_business_contract_raw_audits 3 below required 4".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_replay_live_evolution_items 2 below required 3".to_owned()));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_evolution_online_reward_feedbacks 2 below required 3".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_evolution_online_reward_reinforcements 1 below required 2"
            .to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_evolution_online_reward_penalties 1 below required 2".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_evolution_online_reward_strength 1.250000 below required 1.260000"
            .to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_evolution_online_reward_reinforcement_strength 0.750000 below required 0.760000"
            .to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_evolution_online_reward_penalty_strength 0.500000 below required 0.510000"
            .to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"evolution_replay_live_evolution_memory_updates 3 below required 4".to_owned()));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_evolution_stored_memory_updates 2 below required 3".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_evolution_reflection_issues 2 below required 3".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_evolution_critical_reflection_issues 1 below required 2".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_replay_live_evolution_revision_actions 2 below required 3".to_owned()
    ));
    assert!(failing_report
        .failures
        .contains(&"evolution_recursive_replay_items 8 below required 9".to_owned()));
    assert!(failing_report
        .failures
        .contains(&"evolution_drift_rollbacks 2 above maximum 1".to_owned()));
    assert!(failing_report.failures.contains(
        &"evolution_rollback_router_threshold_delta 0.030000 above maximum 0.020000".to_owned()
    ));
    assert!(failing_report.failures.contains(
        &"evolution_rollback_hierarchy_weight_delta 0.040000 above maximum 0.030000".to_owned()
    ));
}
