use std::fs;

use super::evolution_candidate_status::EVOLUTION_CANDIDATES_FILE;
use super::evolution_status_contract::{
    validate_read_only_enriched_status, validate_read_only_status,
};
use super::evolution_status_enriched_json::render_enriched_evolution_status_json;
use super::evolution_status_summary::summarize_evolution_status;
use super::status_json::json_top_level_object_field;

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    const SAMPLE_STATUS: &str = r#"{
        "daemon": {
            "schema_version": 1,
            "read_only": true,
            "starts_process": false,
            "sends_prompt": false,
            "running": false,
            "pid": null,
            "pid_file_exists": true,
            "stale_pid_file": true,
            "stale_pid": 112704,
            "last_stop_reason": "stopping: runtime token budget reached (36/32)"
        },
        "loop": {
            "schema_version": 1,
            "read_only": true,
            "starts_process": false,
            "sends_prompt": false,
            "touches_remote": false,
            "backend_endpoint": "127.0.0.1:7979",
            "ledger": {
                "total_records": 4,
                "invalid_records": 0,
                "duplicate_rounds": 0,
                "round_gaps": 0,
                "success_count": 4,
                "feedback_applied_total": 16,
                "runtime_tokens_total": 36,
                "latest": {
                    "round": 4,
                    "case": "smartsteam-evolution-loop-0004",
                    "success": true,
                    "runtime_tokens": 9,
                    "feedback_applied": 4
                }
            },
            "report": {
                "path": "D:\\rust-norion\\target\\evolution\\daemon-live-run2-20260615\\report.json",
                "exists": true
            },
            "backend": {"checked": true, "ok": true, "error": ""},
            "remote_chain": {
                "checked": true,
                "exists": true,
                "ready": true,
                "remote_runtime": {
                    "probed": true,
                    "touches_remote": true,
                    "worker_count": 6,
                    "cpu_or_no_gpu_count": 3,
                    "cpu_or_no_gpu_roles": ["summary", "review", "test-gate"],
                    "backend_metadata_may_differ_roles": ["summary", "review", "test-gate"],
                    "acceleration_ok": false,
                    "acceleration_next_step": ".\\tools\\smartsteam-forge\\run-remote-gemma-unattended.cmd -RestartRemote -SkipBuild",
                    "error": ""
                },
                "error": ""
            },
            "model_pool": {
                "available": true,
                "launch_allowed": true,
                "reason": "ready",
                "worker_count": 6,
                "healthy_worker_count": 6,
                "min_context_tokens": 4096
            },
            "readiness": {"ready": true, "failures": []},
            "next_round_decision_status_v1": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "report_only": true,
                "decision": "safe-to-continue-after-current-round",
                "active_round": 5,
                "latest_done_round": 4,
                "ledger_latest_round": 4,
                "reason_codes": ["latest_round_report_gate_passed"],
                "evidence_ids": ["round:4:report-gate-pass"],
                "side_effects": {
                    "starts_daemon": false,
                    "stops_daemon": false,
                    "touches_remote": false,
                    "sends_prompt": false,
                    "starts_stream": false,
                    "replays_prompt": false,
                    "writes_ndkv": false
                }
            },
            "next_step": "ready: run budgeted -Forever or inspect report gate"
        }
    }"#;

    #[test]
    fn summary_renders_daemon_ledger_and_pool_state() {
        validate_read_only_status(SAMPLE_STATUS).unwrap();

        let summary = summarize_evolution_status(
            SAMPLE_STATUS,
            "target\\evolution\\daemon-live-run2-20260615",
        )
        .unwrap();

        assert!(summary.contains("SmartSteam evolution daemon"));
        assert!(summary.contains("read_only=true starts_process=false sends_prompt=false"));
        assert!(summary.contains("daemon running=false"));
        assert!(summary.contains("stale_pid_file=true"));
        assert!(summary.contains("stale_pid=112704"));
        assert!(
            summary.contains("last_stop_reason=stopping: runtime token budget reached (36/32)")
        );
        assert!(
            summary
                .contains("ledger records=4 success=4/4 runtime_tokens=36 feedback=16 ready=true")
        );
        assert!(summary.contains("latest round=4 case=smartsteam-evolution-loop-0004 success=true runtime_tokens=9 feedback=4"));
        assert!(summary.contains("report exists=true"));
        assert!(summary.contains("backend checked=true ok=true"));
        assert!(summary.contains("readiness ready=true failures=none"));
        assert!(summary.contains("remote_chain checked=true ready=true"));
        assert!(
            summary.contains(
                "remote_runtime probed=true touches_remote=true workers=6 cpu_or_no_gpu=3"
            )
        );
        assert!(summary.contains("cpu_or_no_gpu_roles=summary,review,test-gate"));
        assert!(summary.contains("backend_metadata_may_differ_roles=summary,review,test-gate"));
        assert!(summary.contains("acceleration_ok=false"));
        assert!(summary.contains(
            "next_step=.\\tools\\smartsteam-forge\\run-remote-gemma-unattended.cmd -RestartRemote -SkipBuild"
        ));
        assert!(summary.contains("model_pool available=true launch_allowed=true workers=6/6 min_context_tokens=4096 reason=ready"));
        assert!(summary.contains("next_round_decision_status decision=safe-to-continue-after-current-round read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=5 done_round=4 ledger_round=4"));
        assert!(summary.contains("reason_codes=latest_round_report_gate_passed"));
        assert!(summary.contains(
            "unattended_start_plan can_start=true candidate_lifecycle_ready=true readiness_start_ready=true readiness_blocks_start=false readiness_blocking_failures=none block_reason=none"
        ));
        assert!(summary.contains("current_state=not_running_ready_to_start"));
        assert!(summary.contains(
            "unattended_start_acceleration ok=false blocks_start=false cpu_or_no_gpu_roles=summary,review,test-gate"
        ));
        assert!(summary.contains(
            "next_step=.\\tools\\smartsteam-forge\\run-remote-gemma-unattended.cmd -RestartRemote -SkipBuild"
        ));
        assert!(
            summary.contains("stale_pid_file=true stale_pid=112704 stale_pid_blocks_start=false")
        );
        assert!(summary.contains("stale_pid_cleanup_command=.\\tools\\smartsteam-forge\\evolution-daemon.cmd -Stop -WorkDir 'target\\evolution\\daemon-live-run2-20260615'"));
        assert!(summary.contains("unattended_start_check=.\\tools\\smartsteam-forge\\evolution-daemon.cmd -StartCheck -WorkDir 'target\\evolution\\daemon-live-run2-20260615' -Backend '127.0.0.1:7979'"));
        assert!(summary.contains("unattended_start_command=.\\tools\\smartsteam-forge\\evolution-daemon.cmd -Start -WorkDir 'target\\evolution\\daemon-live-run2-20260615' -Backend '127.0.0.1:7979'"));
    }

    #[test]
    fn status_surfaces_paused_polluted_worker_window_without_blocking_healthy_pool() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-worker-window-status-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let status = r#"{
            "daemon": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "running": true,
                "pid": 4242,
                "pid_file_exists": true,
                "stale_pid_file": false
            },
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "backend_endpoint": "127.0.0.1:7979",
                "supervisor": {
                    "running": true,
                    "pid": 5151,
                    "healthy": true,
                    "error": ""
                },
                "remote_chain": {
                    "checked": true,
                    "exists": true,
                    "ready": true,
                    "remote_runtime": {
                        "probed": true,
                        "touches_remote": false,
                        "worker_count": 6,
                        "cpu_or_no_gpu_count": 0,
                        "cpu_or_no_gpu_roles": [],
                        "backend_metadata_may_differ_roles": [],
                        "acceleration_ok": true,
                        "acceleration_next_step": "",
                        "error": ""
                    },
                    "error": ""
                },
                "model_pool": {
                    "available": true,
                    "launch_allowed": true,
                    "reason": "ready",
                    "worker_count": 6,
                    "healthy_worker_count": 6,
                    "min_context_tokens": 65536
                },
                "readiness": {"ready": true, "failures": []},
                "worker_window_status": {
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "starts_clean_room_replacement": false,
                    "mutates_worker_window_status": false,
                    "worker_windows": [
                        {
                            "window_id": "R21-service-cli",
                            "status": "clean",
                            "clean_room_replacement_required": false,
                            "business_task_ids": ["R21-service-cli"]
                        },
                        {
                            "window_id": "R20-eval-paused",
                            "status": "paused",
                            "polluted": true,
                            "stale": false,
                            "clean_room_replacement_required": true,
                            "replacement_window_id": "R21-eval-replacement",
                            "reason_codes": ["polluted_context", "paused_by_owner"],
                            "business_task_ids": ["R20-eval-test"],
                            "evidence_result_ids": ["R21-eval-revalidated"]
                        }
                    ]
                },
                "next_step": "ready: run budgeted -Forever or inspect report gate"
            }
        }"#;

        validate_read_only_status(status).unwrap();
        let summary = summarize_evolution_status(status, &work_dir.to_string_lossy()).unwrap();
        let text =
            render_enriched_evolution_status_json(status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&text).unwrap();
        assert!(summary.contains("daemon running=true pid=4242"));
        assert!(summary.contains("supervisor running=true pid=5151 healthy=true error="));
        assert!(summary.contains("remote_chain checked=true ready=true exists=true error="));
        assert!(summary.contains("remote_runtime probed=true touches_remote=false workers=6"));
        assert!(summary.contains(
            "model_pool available=true launch_allowed=true workers=6/6 min_context_tokens=65536 reason=ready"
        ));
        assert!(summary.contains("worker_window_status read_only=true starts_process=false sends_prompt=false starts_clean_room_replacement=false mutates_worker_window_status=false total=2 replacement_required=1 statuses=clean:1,paused:1"));
        assert!(summary.contains("worker_window id=R20-eval-paused status=paused paused=true polluted=true stale=false archived=false completed_evidence_only=false clean_room_replacement=false assignment_allowed=false original_window_blocks_assignment=true clean_room_replacement_required=true future_work_requires_fresh_clean_room=true replacement_window_id=R21-eval-replacement"));
        assert!(summary.contains("reason_codes=polluted_context,paused_by_owner"));
        assert!(text.contains("\"worker_window_status\":{"));
        assert!(text.contains("\"starts_clean_room_replacement\":false"));
        assert!(text.contains("\"mutates_worker_window_status\":false"));
        assert!(text.contains("\"clean_room_replacement_required_count\":1"));
        assert!(text.contains("\"status_counts\":\"clean:1,paused:1\""));
        assert!(text.contains("\"clean_room_replacement_required\":true"));
        assert!(text.contains("\"assignment_allowed\":false"));
        assert!(text.contains("\"future_work_requires_fresh_clean_room\":true"));
        assert!(!text.contains("\"starts_clean_room_replacement\":true"));
        assert!(!text.contains("\"mutates_worker_window_status\":true"));
    }

    #[test]
    fn status_consumes_worker_window_replacement_report_with_healthy_daemon_and_pool() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-worker-window-report-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let report_path = work_dir.join("report.json");
        fs::write(
            &report_path,
            r#"{
                "report_gate": {"passed": true, "failures": []},
                "ledger_gate_report_v1": {
                    "allow_next_round": true,
                    "gate_blocked": false
                },
                "model_pool_alignment": {"alignment_ok": true},
                "worker_window_replacement_report_v1": {
                    "schema": "worker_window_replacement_report_v1",
                    "consumer_surface": "clean_room_worker_window_replacement_status",
                    "read_only": true,
                    "status_loaded": true,
                    "source": "external_worker_window_status_json",
                    "source_path": "docs/runbooks/smartsteam-worker-window-status-r21.example.json",
                    "source_status": {
                        "schema": "worker_window_status_v1",
                        "side_effects_allowed": false,
                        "windows": [
                            {
                                "window_id": "019ee199-7ecc-7e63-b210-63b838c283b4",
                                "status": "paused",
                                "polluted": true,
                                "completed_evidence_only": true,
                                "clean_room_replacement_required": true,
                                "original_window_blocks_assignment": true,
                                "assignment_allowed": false,
                                "future_work_requires_fresh_clean_room": true,
                                "business_task_ids": ["R20-eval-test"],
                                "evidence_result_ids": ["R21-eval-revalidated"]
                            },
                            {
                                "window_id": "019ee1a6-c250-7f31-97ed-15cd8e8cd159",
                                "status": "clean-room-replacement",
                                "clean_room_replacement": true,
                                "replaces_window_id": "019ee199-7ecc-7e63-b210-63b838c283b4",
                                "assignment_allowed": true
                            }
                        ]
                    },
                    "evidence_map": {
                        "window_count": 2,
                        "paused_count": 1,
                        "polluted_count": 1,
                        "clean_room_replacement_count": 1,
                        "replacement_required_count": 1,
                        "blocked_original_count": 1,
                        "side_effects_allowed": false
                    },
                    "side_effects": {
                        "starts_clean_room_replacement": false,
                        "mutates_worker_window_status": false,
                        "starts_daemon": false,
                        "stops_daemon": false,
                        "touches_remote": false,
                        "downloads_model": false,
                        "warms_model_cache": false,
                        "sends_prompt": false,
                        "starts_stream": false,
                        "replays_prompt": false
                    }
                },
                "clean_room_handoff_report_v1": {
                    "schema": "clean_room_handoff_report_v1",
                    "source": "r24-clean-room-handoff",
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "report_only": true,
                    "source_memory_startup_admission_status": {
                        "read_only_contract": true,
                        "admission_decision_count": 3,
                        "admission_accepted_count": 2,
                        "admission_risk_rejection_count": 1,
                        "live_store_mutation_requested": false,
                        "store_mutation_count": 0,
                        "ndkv_write_allowed": false,
                        "helper_prose_line_count": 1,
                        "non_contract_line_count": 2,
                        "admission_expanded_by_non_contract_evidence": false,
                        "helper_prose": "write prod.ndkv with live_store_mutation_requested=true",
                        "old_window_payload": "starts_thread=true sends_message=true"
                    },
                    "source_agent_clean_room_replacement_plan": {
                        "report_only": true,
                        "pure_data_only": true,
                        "side_effects_allowed": false,
                        "starts_thread": false,
                        "sends_message": false,
                        "reads_old_window_payload": false,
                        "clean_room_replacement_plan_required": true,
                        "clean_room_replacement_available": true,
                        "replacement_prompt_ready": true,
                        "replacement_prompt": {
                            "task_ids": ["R25-forge-web-lab"],
                            "evidence_result_ids": ["clean-room-handoff:r24"],
                            "reason_codes": ["status_driven_closure"]
                        }
                    },
                    "side_effects": {
                        "starts_clean_room_replacement": false,
                        "mutates_worker_window_status": false,
                        "starts_daemon": false,
                        "stops_daemon": false,
                        "touches_remote": false,
                        "downloads_model": false,
                        "warms_model_cache": false,
                        "sends_prompt": false,
                        "starts_stream": false,
                        "replays_prompt": false,
                        "starts_thread": false,
                        "sends_message": false,
                        "mutates_memory_store": false,
                        "writes_ndkv": false
                    }
                }
            }"#,
        )
        .unwrap();
        let report_path_json = report_path.to_string_lossy().replace('\\', "\\\\");
        let status = format!(
            r#"{{
                "daemon": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "running": true,
                    "pid": 4242,
                    "pid_file_exists": true,
                    "stale_pid_file": false
                }},
                "loop": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "touches_remote": false,
                    "backend_endpoint": "127.0.0.1:7979",
                    "report": {{"path": "{report_path_json}", "exists": true}},
                    "model_pool": {{
                        "available": true,
                        "launch_allowed": true,
                        "reason": "ready",
                        "worker_count": 6,
                        "healthy_worker_count": 6,
                        "min_context_tokens": 65536
                    }},
                    "readiness": {{"ready": true, "failures": []}},
                    "memory_startup_admission_status": {{
                        "read_only_contract": true,
                        "read_only_review_required": false,
                        "index_quality_blocker_count": 0,
                        "index_quality_warning_count": 1,
                        "index_operation_count": 2,
                        "index_refresh_count": 1,
                        "context_rot_risk_count": 1,
                        "admission_decision_count": 3,
                        "admission_accepted_count": 2,
                        "admission_risk_rejection_count": 1,
                        "live_store_mutation_requested": false,
                        "store_mutation_count": 0,
                        "ndkv_write_allowed": false,
                        "helper_prose_line_count": 1,
                        "non_contract_line_count": 2,
                        "admission_expanded_by_non_contract_evidence": false
                    }},
                    "next_step": "ready: run budgeted -Forever or inspect report gate"
                }}
            }}"#
        );

        validate_read_only_status(&status).unwrap();
        let summary = summarize_evolution_status(&status, &work_dir.to_string_lossy()).unwrap();
        let text =
            render_enriched_evolution_status_json(&status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&text).unwrap();
        assert!(summary.contains("daemon running=true pid=4242"));
        assert!(summary.contains(
            "model_pool available=true launch_allowed=true workers=6/6 min_context_tokens=65536 reason=ready"
        ));
        assert!(summary.contains("worker_window_replacement_report read_only=true starts_process=false sends_prompt=false status_loaded=true total=2 paused=1 polluted=1 clean_room_replacement=1 replacement_required=1 blocked_original=1 starts_clean_room_replacement=false mutates_worker_window_status=false source=external_worker_window_status_json"));
        assert!(summary.contains("worker_window_report_source id=019ee199-7ecc-7e63-b210-63b838c283b4 status=paused paused=true polluted=true stale=false archived=false completed_evidence_only=true clean_room_replacement=false assignment_allowed=false original_window_blocks_assignment=true clean_room_replacement_required=true future_work_requires_fresh_clean_room=true"));
        assert!(summary.contains("worker_window_report_source id=019ee1a6-c250-7f31-97ed-15cd8e8cd159 status=clean-room-replacement paused=false polluted=false stale=false archived=false completed_evidence_only=false clean_room_replacement=true assignment_allowed=true original_window_blocks_assignment=false clean_room_replacement_required=false future_work_requires_fresh_clean_room=false"));
        assert!(summary.contains("clean_room_handoff_report read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true source=r24-clean-room-handoff memory_admission_safe=true no_live_write=true no_ndkv_write=true agent_replacement_plan_required=true replacement_prompt_ready=true starts_clean_room_replacement=false mutates_worker_window_status=false mutates_memory_store=false writes_ndkv=false"));
        assert!(summary.contains("clean_room_handoff_agent status_loaded=true report_only=true pure_data_only=true side_effects_allowed=false reads_old_window_payload=false starts_thread=false sends_message=false prompt_tasks=1 prompt_evidence_results=1 prompt_reason_codes=1"));
        assert!(summary.contains("unified_status read_only=true starts_process=false sends_prompt=false starts_daemon=false stops_daemon=false touches_remote=false downloads_model=false warms_model_cache=false starts_stream=false replays_prompt=false daemon_healthy=true supervisor_healthy=false model_pool_healthy=true worker_replacement_required=true memory_admission_safe=true no_live_write=true no_ndkv_write=true clean_room_handoff_loaded=true clean_room_handoff_safe=true"));
        assert!(summary.contains("unified_memory_startup_admission status_loaded=true safe=true read_only_contract=true admission_decisions=3 admission_accepted=2 admission_risk_rejections=1 live_store_mutation_requested=false store_mutations=0 ndkv_write_allowed=false helper_prose_lines=1 non_contract_lines=2 admission_expanded_by_non_contract=false"));
        assert!(summary.contains("unified_clean_room_handoff status_loaded=true safe=true report_only=true memory_admission_safe=true agent_replacement_safe=true starts_clean_room_replacement=false mutates_worker_window_status=false mutates_memory_store=false writes_ndkv=false"));
        assert!(text.contains("\"worker_window_replacement_report\":{"));
        assert!(text.contains("\"clean_room_handoff_report\":{"));
        assert!(text.contains("\"unified_status\":{"));
        assert!(text.contains("\"worker_replacement_required\":true"));
        assert!(text.contains("\"memory_admission_safe\":true"));
        assert!(text.contains("\"no_live_write\":true"));
        assert!(text.contains("\"no_ndkv_write\":true"));
        assert!(text.contains("\"clean_room_handoff_loaded\":true"));
        assert!(text.contains("\"clean_room_handoff_safe\":true"));
        assert!(text.contains("\"status_loaded\":true"));
        assert!(text.contains("\"replacement_required_count\":1"));
        assert!(text.contains("\"completed_evidence_only\":true"));
        assert!(text.contains("\"assignment_allowed\":false"));
        assert!(text.contains("\"future_work_requires_fresh_clean_room\":true"));
        assert!(text.contains("\"starts_clean_room_replacement\":false"));
        assert!(text.contains("\"mutates_worker_window_status\":false"));
        assert!(text.contains("\"mutates_memory_store\":false"));
        assert!(text.contains("\"writes_ndkv\":false"));
        assert!(text.contains("\"memory_startup_admission\":{"));
        assert!(text.contains("\"clean_room_handoff\":{"));
        assert!(text.contains("\"prompt_task_count\":1"));
        assert!(text.contains("\"admission_decision_count\":3"));
        assert!(text.contains("\"store_mutation_count\":0"));
        assert!(text.contains("\"ndkv_write_allowed\":false"));
        assert!(text.contains("\"admission_expanded_by_non_contract_evidence\":false"));
        assert!(text.contains("\"model_pool\": {"));
        assert!(text.contains("\"healthy_worker_count\": 6"));
        assert!(text.contains("\"daemon\": {"));
        assert!(!text.contains("prod.ndkv"));
        assert!(!text.contains("starts_thread=true"));
        assert!(!text.contains("sends_message=true"));
        assert!(!text.contains("\"starts_clean_room_replacement\":true"));
        assert!(!text.contains("\"mutates_worker_window_status\":true"));
        assert!(!text.contains("\"mutates_memory_store\":true"));
        assert!(!text.contains("\"writes_ndkv\":true"));
    }

    #[test]
    fn summary_tolerates_optional_status_sections() {
        let status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false, "running": true},
            "loop": {"read_only": true, "starts_process": false, "sends_prompt": false, "touches_remote": false}
        }"#;

        let summary = summarize_evolution_status(status, "target\\evolution\\daemon").unwrap();

        assert!(summary.contains("daemon running=true"));
        assert!(!summary.contains("ledger records="));
        assert!(!summary.contains("next_round_decision_status decision="));

        let text =
            render_enriched_evolution_status_json(status, "target\\evolution\\daemon").unwrap();

        validate_read_only_enriched_status(&text).unwrap();
        assert!(text.contains("\"next_round_decision_status\":{"));
        assert!(text.contains("\"decision\":null"));
        assert!(text.contains("\"active_round\":\"unknown\""));
    }

    #[test]
    fn status_consumes_next_round_decision_report_when_loop_status_is_absent() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-r36-next-round-report-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let report_path = work_dir.join("report.json");
        fs::write(
            &report_path,
            include_str!("../../fixtures/r36-next-round-decision-report-v1.example.json"),
        )
        .unwrap();
        let report_path_json = report_path.to_string_lossy().replace('\\', "\\\\");
        let status = format!(
            r#"{{
                "daemon": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "running": true,
                    "pid": 230636
                }},
                "loop": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "touches_remote": false,
                    "report": {{"path": "{report_path_json}", "exists": true}},
                    "readiness": {{"ready": true, "failures": []}}
                }}
            }}"#
        );

        let summary = summarize_evolution_status(&status, &work_dir.to_string_lossy()).unwrap();
        let text =
            render_enriched_evolution_status_json(&status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&text).unwrap();
        assert!(summary.contains("daemon running=true pid=230636"));
        assert!(summary.contains("next_round_decision_status decision=safe-to-continue-after-current-round read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=366 done_round=366 ledger_round=366"));
        assert!(text.contains("\"next_round_decision_status\":{"));
        assert!(text.contains("\"decision\":\"safe-to-continue-after-current-round\""));
        assert!(text.contains("\"active_round\":366"));
        assert!(text.contains("\"done_round\":366"));
        assert!(text.contains("\"ledger_round\":366"));
        assert!(!text.contains("\"starts_process\":true"));
        assert!(!text.contains("\"sends_prompt\":true"));
        assert!(!text.contains("\"starts_stream\":true"));
        assert!(!text.contains("\"writes_ndkv\":true"));
    }

    #[test]
    fn status_consumes_live_status_bundle_next_round_decision_variants() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-r37-live-status-bundle-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let fixture =
            include_str!("../../fixtures/r37-live-status-bundle-next-round-decision.example.json");
        let safe_status = json_top_level_object_field(fixture, "safe_to_wait_status").unwrap();
        let blocked_status =
            json_top_level_object_field(fixture, "blocked_operator_attention_status").unwrap();

        validate_read_only_status(safe_status).unwrap();
        validate_read_only_status(blocked_status).unwrap();
        let safe_summary =
            summarize_evolution_status(safe_status, &work_dir.to_string_lossy()).unwrap();
        let safe_text =
            render_enriched_evolution_status_json(safe_status, &work_dir.to_string_lossy())
                .unwrap();
        let blocked_summary =
            summarize_evolution_status(blocked_status, &work_dir.to_string_lossy()).unwrap();
        let blocked_text =
            render_enriched_evolution_status_json(blocked_status, &work_dir.to_string_lossy())
                .unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&safe_text).unwrap();
        validate_read_only_enriched_status(&blocked_text).unwrap();
        assert!(safe_summary.contains("daemon running=true pid=199264"));
        assert!(safe_summary.contains("daemon_round_transition status=normal_in_progress latest_round_state=round_in_progress round_in_progress=true read_only=true starts_process=false report_only=true observed_round_done=false active_round=369 done_round=368 ledger_round=368"));
        assert!(safe_summary.contains("next_round_decision_status decision=safe-to-wait/current-round-active read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=369 done_round=368 ledger_round=368"));
        assert!(safe_summary.contains("reason_codes=current_round_active"));
        assert!(safe_summary.contains("evidence_ids=active-round:369,ledger:latest-round-368"));
        assert!(safe_text.contains("\"live_status_bundle\""));
        assert!(safe_text.contains("\"decision\":\"safe-to-wait/current-round-active\""));
        assert!(safe_text.contains("\"active_round\":369"));
        assert!(safe_text.contains("\"done_round\":368"));
        assert!(safe_text.contains("\"ledger_round\":368"));
        assert!(!safe_text.contains("\"starts_process\":true"));
        assert!(!safe_text.contains("\"sends_prompt\":true"));
        assert!(!safe_text.contains("\"starts_stream\":true"));
        assert!(!safe_text.contains("\"writes_ndkv\":true"));

        assert!(blocked_summary.contains("next_round_decision_status decision=operator-attention-blocked read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=368 done_round=368 ledger_round=368"));
        assert!(
            blocked_summary.contains("reason_codes=report_gate_failed_operator_attention_required")
        );
        assert!(blocked_summary.contains("evidence_ids=round:368:report-gate-failed"));
        assert!(blocked_text.contains("\"decision\":\"operator-attention-blocked\""));
        assert!(
            blocked_text
                .contains("\"reason_codes\":[\"report_gate_failed_operator_attention_required\"]")
        );
        assert!(!blocked_text.contains("\"starts_process\":true"));
        assert!(!blocked_text.contains("\"sends_prompt\":true"));
        assert!(!blocked_text.contains("\"starts_stream\":true"));
        assert!(!blocked_text.contains("\"writes_ndkv\":true"));
    }

    #[test]
    fn status_consumes_current_next_round_decision_report_v1_locations() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-r39-next-round-report-v1-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let fixture = include_str!(
            "../../fixtures/r39-current-next-round-decision-report-v1-status.example.json"
        );
        let current_status = json_top_level_object_field(fixture, "current_status").unwrap();
        let top_level_only_status =
            json_top_level_object_field(fixture, "top_level_report_only_status").unwrap();

        validate_read_only_status(current_status).unwrap();
        validate_read_only_status(top_level_only_status).unwrap();
        let current_summary =
            summarize_evolution_status(current_status, &work_dir.to_string_lossy()).unwrap();
        let current_text =
            render_enriched_evolution_status_json(current_status, &work_dir.to_string_lossy())
                .unwrap();
        let top_level_summary =
            summarize_evolution_status(top_level_only_status, &work_dir.to_string_lossy()).unwrap();
        let top_level_text = render_enriched_evolution_status_json(
            top_level_only_status,
            &work_dir.to_string_lossy(),
        )
        .unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&current_text).unwrap();
        validate_read_only_enriched_status(&top_level_text).unwrap();
        assert!(current_summary.contains("daemon running=true pid=199264"));
        assert!(current_summary.contains("daemon_round_transition status=round_done_waiting_ledger_commit latest_round_state=round_done_waiting_ledger_commit round_in_progress=false read_only=true starts_process=false report_only=true observed_round_done=true active_round=371 done_round=371 ledger_round=370 ledger_commit_pending=true"));
        assert!(current_summary.contains("next_round_decision_status decision=safe-to-continue-after-current-round read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=371 done_round=371 ledger_round=370"));
        assert!(current_summary.contains("reason_codes=latest_done_round_waiting_ledger_commit"));
        assert!(
            current_summary
                .contains("evidence_ids=active-round:371,done-round:371,ledger:latest-round-370")
        );
        assert!(!current_summary.contains("active_round=999"));
        assert!(current_text.contains("\"next_round_decision_report_v1\""));
        assert!(current_text.contains("\"decision\":\"safe-to-continue-after-current-round\""));
        assert!(current_text.contains("\"active_round\":371"));
        assert!(current_text.contains("\"done_round\":371"));
        assert!(current_text.contains("\"ledger_round\":370"));
        assert!(!current_text.contains("\"starts_process\":true"));
        assert!(!current_text.contains("\"sends_prompt\":true"));
        assert!(!current_text.contains("\"starts_stream\":true"));
        assert!(!current_text.contains("\"writes_ndkv\":true"));

        assert!(top_level_summary.contains("next_round_decision_status decision=safe-to-continue-after-current-round read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=371 done_round=371 ledger_round=370"));
        assert!(top_level_text.contains("\"next_round_decision_status\":{"));
        assert!(top_level_text.contains("\"decision\":\"safe-to-continue-after-current-round\""));
        assert!(top_level_text.contains("\"active_round\":371"));
        assert!(top_level_text.contains("\"ledger_round\":370"));
        assert!(!top_level_text.contains("\"writes_ndkv\":true"));
    }

    #[test]
    fn status_consumes_next_round_downstream_status_consumers_v1_locations() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-r43-next-round-downstream-consumers-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let fixture = include_str!(
            "../../fixtures/r43-next-round-downstream-status-consumers-v1.example.json"
        );
        let root_status = json_top_level_object_field(fixture, "root_status").unwrap();
        let live_status =
            json_top_level_object_field(fixture, "live_status_bundle_status").unwrap();

        validate_read_only_status(root_status).unwrap();
        validate_read_only_status(live_status).unwrap();
        let root_summary =
            summarize_evolution_status(root_status, &work_dir.to_string_lossy()).unwrap();
        let root_text =
            render_enriched_evolution_status_json(root_status, &work_dir.to_string_lossy())
                .unwrap();
        let live_summary =
            summarize_evolution_status(live_status, &work_dir.to_string_lossy()).unwrap();
        let live_text =
            render_enriched_evolution_status_json(live_status, &work_dir.to_string_lossy())
                .unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&root_text).unwrap();
        validate_read_only_enriched_status(&live_text).unwrap();
        assert!(root_summary.contains("next_round_downstream_status_consumers read_only=true starts_process=false sends_prompt=false report_only=true side_effects=false starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=373 done_round=372 ledger_round=372 round_id_evidence_active_round=373 round_id_evidence_done_round=372 round_id_evidence_ledger_round=372"));
        assert!(root_summary.contains("consumers=service_cli_display,forge_operator_display,agent_assignment_acceptance,memory_self_improve_admission_visibility"));
        assert!(root_summary.contains("next_round_downstream_consumer id=forge_operator_display required=true satisfied=true read_only=true report_only=true side_effects=false"));
        assert!(root_text.contains("\"next_round_downstream_status_consumers\":{"));
        assert!(root_text.contains("\"transition_kind\":\"normal_in_progress\""));
        assert!(root_text.contains("\"id\":\"memory_self_improve_admission_visibility\""));
        assert!(!root_text.contains("\"side_effects\":true"));
        assert!(!root_text.contains("\"starts_process\":true"));
        assert!(!root_text.contains("\"sends_prompt\":true"));
        assert!(!root_text.contains("\"writes_ndkv\":true"));

        assert!(live_summary.contains("next_round_decision_status decision=safe-to-continue-after-current-round read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=374 done_round=373 ledger_round=373"));
        assert!(live_summary.contains("next_round_downstream_status_consumers read_only=true starts_process=false sends_prompt=false report_only=true side_effects=false starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=374 done_round=373 ledger_round=373 round_id_evidence_active_round=374 round_id_evidence_done_round=373 round_id_evidence_ledger_round=373"));
        assert!(!live_summary.contains("active_round=999"));
        assert!(live_text.contains("\"next_round_downstream_status_consumers\":{"));
        assert!(live_text.contains("\"transition_kind\":\"round_done_waiting_ledger_commit\""));
        assert!(live_text.contains("\"decision\":\"safe-to-continue-after-current-round\""));
        assert!(!live_text.contains("\"active_round\":999"));
        assert!(!live_text.contains("\"side_effects\":true"));
    }

    #[test]
    fn status_replays_post_r44_strict_safe_to_wait_shape() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-r45-post-r44-safe-to-wait-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let fixture = include_str!("../../fixtures/r45-post-r44-safe-to-wait-status.example.json");
        let root_status = json_top_level_object_field(fixture, "root_status").unwrap();
        let live_status =
            json_top_level_object_field(fixture, "live_status_bundle_status").unwrap();

        validate_read_only_status(root_status).unwrap();
        validate_read_only_status(live_status).unwrap();
        let root_summary =
            summarize_evolution_status(root_status, &work_dir.to_string_lossy()).unwrap();
        let root_text =
            render_enriched_evolution_status_json(root_status, &work_dir.to_string_lossy())
                .unwrap();
        let live_summary =
            summarize_evolution_status(live_status, &work_dir.to_string_lossy()).unwrap();
        let live_text =
            render_enriched_evolution_status_json(live_status, &work_dir.to_string_lossy())
                .unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&root_text).unwrap();
        validate_read_only_enriched_status(&live_text).unwrap();

        for summary in [&root_summary, &live_summary] {
            assert!(summary.contains("daemon running=true pid=209816"));
            assert!(summary.contains("daemon_round_transition status=normal_in_progress latest_round_state=round_in_progress round_in_progress=true read_only=true starts_process=false report_only=true observed_round_done=false active_round=380 done_round=379 ledger_round=379"));
            assert!(summary.contains("next_round_decision_status decision=safe-to-wait/current-round-active read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=380 done_round=379 ledger_round=379"));
            assert!(summary.contains(
                "reason_codes=current_round_active,latest_completed_ledger_report_gate_passed"
            ));
            assert!(summary.contains("evidence_ids=active-round:380,done-round:379,ledger:latest-round-379,round:379:report-gate-passed"));
            assert!(!summary.contains("decision=operator-attention-blocked"));
            assert!(!summary.contains("active_round=999"));
            assert!(!summary.contains("legacy_status_must_not_win"));
        }

        for text in [&root_text, &live_text] {
            let decision = json_top_level_object_field(text, "next_round_decision_status").unwrap();
            let downstream =
                json_top_level_object_field(text, "next_round_downstream_status_consumers")
                    .unwrap();

            assert!(decision.contains("\"decision\":\"safe-to-wait/current-round-active\""));
            assert!(decision.contains("\"active_round\":380"));
            assert!(decision.contains("\"done_round\":379"));
            assert!(decision.contains("\"ledger_round\":379"));
            assert!(!decision.contains("\"decision\":\"operator-attention-blocked\""));
            assert!(!decision.contains("\"active_round\":999"));
            assert!(!decision.contains("legacy_status_must_not_win"));

            assert!(downstream.contains("\"active_round\":380"));
            assert!(downstream.contains("\"done_round\":379"));
            assert!(downstream.contains("\"ledger_round\":379"));
            assert!(downstream.contains("\"source_schema\":\"daemon_round_transition_status_v1\""));
            assert!(downstream.contains("\"transition_kind\":\"normal_in_progress\""));
            assert!(downstream.contains("\"id\":\"forge_operator_display\""));
            assert!(!downstream.contains("\"side_effects\":true"));
            assert!(!downstream.contains("\"starts_process\":true"));
            assert!(!downstream.contains("\"sends_prompt\":true"));
            assert!(!downstream.contains("\"starts_stream\":true"));
            assert!(!downstream.contains("\"writes_ndkv\":true"));
        }

        assert!(root_summary.contains("next_round_downstream_status_consumers read_only=true starts_process=false sends_prompt=false report_only=true side_effects=false starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=380 done_round=379 ledger_round=379 round_id_evidence_active_round=380 round_id_evidence_done_round=379 round_id_evidence_ledger_round=379 round_id_evidence_source_schema=daemon_round_transition_status_v1"));
        assert!(root_summary.contains("evidence_ids=status-root:downstream-consumers,daemon-round-transition-status-v1:active-380:done-379:ledger-379"));
        assert!(live_summary.contains("next_round_downstream_status_consumers read_only=true starts_process=false sends_prompt=false report_only=true side_effects=false starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=380 done_round=379 ledger_round=379 round_id_evidence_active_round=380 round_id_evidence_done_round=379 round_id_evidence_ledger_round=379 round_id_evidence_source_schema=daemon_round_transition_status_v1"));
        assert!(live_summary.contains("evidence_ids=live-status-bundle:downstream-consumers,daemon-round-transition-status-v1:active-380:done-379:ledger-379"));
    }

    #[test]
    fn status_consumes_self_improve_proposal_panel_next_to_daemon_and_pool() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-r26-proposal-panel-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let report_path = work_dir.join("report.json");
        fs::write(
            &report_path,
            include_str!("../../fixtures/r26-self-improve-proposal-artifact.example.json"),
        )
        .unwrap();
        let report_path_json = report_path.to_string_lossy().replace('\\', "\\\\");
        let status = format!(
            r#"{{
                "daemon": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "running": true,
                    "pid": 230076
                }},
                "loop": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "touches_remote": false,
                    "backend_endpoint": "127.0.0.1:7979",
                    "report": {{"path": "{report_path_json}", "exists": true}},
                    "model_pool": {{
                        "available": true,
                        "launch_allowed": true,
                        "reason": "ready",
                        "worker_count": 6,
                        "healthy_worker_count": 6,
                        "min_context_tokens": 65536
                    }},
                    "readiness": {{"ready": true, "failures": []}}
                }}
            }}"#
        );

        let summary = summarize_evolution_status(&status, &work_dir.to_string_lossy()).unwrap();
        let text =
            render_enriched_evolution_status_json(&status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&text).unwrap();
        assert!(summary.contains("daemon running=true pid=230076"));
        assert!(summary.contains(
            "model_pool available=true launch_allowed=true workers=6/6 min_context_tokens=65536 reason=ready"
        ));
        assert!(summary.contains("self_improve_proposal_panel read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true source=r26-self-improve-proposal-artifact candidate=2 validated=1 admitted=1 quarantined=1 promoted=1 repair_required=1 starts_daemon=false stops_daemon=false starts_stream=false replays_prompt=false touches_remote=false downloads_model=false warms_model_cache=false"));
        assert!(summary.contains("self_improve_proposal_guidance read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true convert_advisory_to_business_evidence=true repair_unvalidated_or_unaccepted=false requires_validation_and_memory_admission=true"));
        assert!(summary.contains("self_improve_proposal_action_plan read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true action_required=true primary_action=convert_advisory_to_evidence_backed_business_improvement actions=convert_advisory_to_evidence_backed_business_improvement,require_checked_passed_validation_and_accepted_memory_admission requires_validation_and_memory_admission=true auto_apply=false"));
        assert!(summary.contains("self_improve_proposal_action_assignment read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true action_required=true primary_action=convert_advisory_to_evidence_backed_business_improvement actions=convert_advisory_to_evidence_backed_business_improvement,require_checked_passed_validation_and_accepted_memory_admission target_count=1 first_target=self-improve-r26-proposal-memory-index first_source_round=26 first_evidence_ids=round-26:self-improve-proposal-guidance first_memory_admission_decision=quarantined first_validation_checked=true first_validation_passed=true first_memory_admission_accepted=false first_evidence_backed_business_improvement=false first_advisory_only=true first_require_repair=false first_missing_requirements=accepted_memory_admission,evidence_backed_business_improvement requires_validation_and_memory_admission=true auto_apply=false"));
        assert!(summary.contains("unified_self_improve_proposal status_loaded=true safe=true report_only=true candidate=2 validated=1 admitted=1 quarantined=1 promoted=1 repair_required=1 starts_daemon=false starts_stream=false replays_prompt=false sends_prompt=false guidance_loaded=true convert_advisory_to_business_evidence=true repair_unvalidated_or_unaccepted=false requires_validation_and_memory_admission=true"));
        assert!(summary.contains("action_plan_loaded=true action_required=true primary_action=convert_advisory_to_evidence_backed_business_improvement actions=convert_advisory_to_evidence_backed_business_improvement,require_checked_passed_validation_and_accepted_memory_admission action_plan_requires_validation_and_memory_admission=true"));
        assert!(summary.contains("action_assignment_loaded=true action_assignment_targets=1 action_assignment_first_target=self-improve-r26-proposal-memory-index action_assignment_first_round=26 action_assignment_first_evidence_ids=round-26:self-improve-proposal-guidance action_assignment_first_memory_admission=quarantined action_assignment_first_validation_checked=true action_assignment_first_validation_passed=true action_assignment_first_memory_accepted=false action_assignment_first_business_evidence=false action_assignment_first_advisory_only=true action_assignment_first_require_repair=false action_assignment_first_missing=accepted_memory_admission,evidence_backed_business_improvement"));
        assert!(text.contains("\"self_improve_proposal_panel\":{"));
        assert!(text.contains("\"prompt_guidance\":{"));
        assert!(text.contains("\"action_plan\":{"));
        assert!(text.contains("\"action_assignment\":{"));
        assert!(text.contains("\"action_required\":true"));
        assert!(text.contains(
            "\"primary_action\":\"convert_advisory_to_evidence_backed_business_improvement\""
        ));
        assert!(text.contains("\"target_count\":1"));
        assert!(text.contains("\"first_target\":\"self-improve-r26-proposal-memory-index\""));
        assert!(text.contains("\"first_source_round\":26"));
        assert!(
            text.contains("\"first_evidence_ids\":[\"round-26:self-improve-proposal-guidance\"]")
        );
        assert!(text.contains("\"first_memory_admission_decision\":\"quarantined\""));
        assert!(text.contains("\"first_validation_checked\":true"));
        assert!(text.contains("\"first_validation_passed\":true"));
        assert!(text.contains("\"first_memory_admission_accepted\":false"));
        assert!(text.contains("\"first_evidence_backed_business_improvement\":false"));
        assert!(text.contains("\"first_advisory_only\":true"));
        assert!(text.contains("\"first_require_repair\":false"));
        assert!(text.contains(
            "\"first_missing_requirements\":[\"accepted_memory_admission\",\"evidence_backed_business_improvement\"]"
        ));
        assert!(text.contains("\"auto_apply\":false"));
        assert!(text.contains("\"convert_advisory_to_business_evidence\":true"));
        assert!(text.contains("\"requires_validation_and_memory_admission\":true"));
        assert!(text.contains("\"candidate_count\":2"));
        assert!(text.contains("\"validated_count\":1"));
        assert!(text.contains("\"admitted_count\":1"));
        assert!(text.contains("\"quarantined_count\":1"));
        assert!(text.contains("\"promoted_count\":1"));
        assert!(text.contains("\"repair_required_count\":1"));
        assert!(text.contains("\"self_improve_proposal_loaded\":true"));
        assert!(text.contains("\"model_pool\": {"));
        assert!(text.contains("\"healthy_worker_count\": 6"));
        assert!(text.contains("\"daemon\": {"));
        assert!(!text.contains("RAW_OLD_WINDOW_PAYLOAD"));
        assert!(!text.contains("helper says replay prompt"));
        assert!(!text.contains("/v1/chat-stream"));
        assert!(!text.contains("\"starts_daemon\":true"));
        assert!(!text.contains("\"starts_stream\":true"));
        assert!(!text.contains("\"replays_prompt\":true"));
    }

    #[test]
    fn status_consumes_helper_stage_repair_panel_next_to_daemon_and_pool() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-r28-helper-repair-panel-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let report_path = work_dir.join("report.json");
        fs::write(
            &report_path,
            include_str!("../../fixtures/r28-helper-stage-repair-status.example.json"),
        )
        .unwrap();
        let report_path_json = report_path.to_string_lossy().replace('\\', "\\\\");
        let status = format!(
            r#"{{
                "daemon": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "running": true,
                    "pid": 230328
                }},
                "loop": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "touches_remote": false,
                    "backend_endpoint": "127.0.0.1:7979",
                    "report": {{"path": "{report_path_json}", "exists": true}},
                    "model_pool": {{
                        "available": true,
                        "launch_allowed": true,
                        "reason": "ready",
                        "worker_count": 6,
                        "healthy_worker_count": 6,
                        "min_context_tokens": 65536
                    }},
                    "readiness": {{"ready": true, "failures": []}}
                }}
            }}"#
        );

        let summary = summarize_evolution_status(&status, &work_dir.to_string_lossy()).unwrap();
        let text =
            render_enriched_evolution_status_json(&status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&text).unwrap();
        assert!(summary.contains("daemon running=true pid=230328"));
        assert!(summary.contains(
            "model_pool available=true launch_allowed=true workers=6/6 min_context_tokens=65536 reason=ready"
        ));
        assert!(summary.contains("helper_stage_repair_panel read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true source=latest_ledger_helper_stage_contract_projection latest_round=328 repair_required=true total_roles=4 incomplete_roles=2 missing_helper_role_repair_required=true missing_helper_role_repair_proposals=1 missing_helper_roles=router proposals=3 roles=router,review,test-gate missing_fields=verification,failure_kind placeholder_fields=validation_command starts_daemon=false starts_forge=false starts_web_lab=false calls_model=false starts_stream=false replays_prompt=false writes_ndkv=false mutates_memory_store=false auto_apply=false"));
        assert!(summary.contains("helper_stage_repair_proposal id=helper-stage-repair-r328-router role=router status=missing_helper_role missing_helper_role_repair_required=true missing_fields=none placeholder_fields=none validation_safety=safe candidate_only=true auto_apply=false"));
        assert!(summary.contains("unified_helper_stage_repair status_loaded=true safe=true report_only=true repair_required=true proposals=3 incomplete_roles=2 missing_helper_role_repair_required=true missing_helper_role_repair_proposals=1 missing_helper_roles=router roles=router,review,test-gate starts_daemon=false starts_forge=false starts_web_lab=false calls_model=false starts_stream=false replays_prompt=false sends_prompt=false auto_apply=false"));
        assert!(text.contains("\"helper_stage_repair_panel\":{"));
        assert!(text.contains("\"helper_stage_repair_loaded\":true"));
        assert!(text.contains("\"helper_stage_repair_safe\":true"));
        assert!(text.contains("\"helper_stage_repair_required\":true"));
        assert!(text.contains("\"missing_helper_role_repair_required\":true"));
        assert!(text.contains("\"missing_helper_role_repair_proposal_count\":1"));
        assert!(text.contains("\"missing_helper_roles\":[\"router\"]"));
        assert!(text.contains("\"proposal_count\":3"));
        assert!(text.contains("\"roles\":[\"router\",\"review\",\"test-gate\"]"));
        assert!(text.contains("\"starts_web_lab\":false"));
        assert!(text.contains("\"calls_model\":false"));
        assert!(text.contains("\"auto_apply\":false"));
        assert!(!text.contains("RAW_OLD_WINDOW_PAYLOAD"));
        assert!(!text.contains("helper says replay prompt"));
        assert!(!text.contains("/v1/chat-stream"));
        assert!(!text.contains("\"starts_daemon\":true"));
        assert!(!text.contains("\"starts_forge\":true"));
        assert!(!text.contains("\"starts_web_lab\":true"));
        assert!(!text.contains("\"calls_model\":true"));
        assert!(!text.contains("\"starts_stream\":true"));
        assert!(!text.contains("\"replays_prompt\":true"));
    }

    #[test]
    fn summary_marks_stdout_tail_as_history_when_daemon_is_not_running() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-log-tail-history-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let status = r#"{
            "daemon": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "running": false,
                "stdout_tail": [
                    "[round 37] stage gates:done",
                    "[round 37] done [DONE]"
                ]
            },
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "ledger": {
                    "total_records": 36,
                    "invalid_records": 0,
                    "duplicate_rounds": 0,
                    "round_gaps": 0,
                    "success_count": 36,
                    "feedback_applied_total": 144,
                    "runtime_tokens_total": 2692,
                    "latest": {
                        "round": 36,
                        "case": "smartsteam-evolution-loop-0036",
                        "success": true,
                        "runtime_tokens": 64,
                        "feedback_applied": 4
                    }
                },
                "report": {"exists": false, "path": "missing-report.json"},
                "readiness": {"ready": true, "failures": []}
            }
        }"#;

        let summary = summarize_evolution_status(status, "target\\evolution\\daemon").unwrap();
        let text =
            render_enriched_evolution_status_json(status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(summary.contains("daemon running=false"));
        assert!(summary.contains("latest round=36 case=smartsteam-evolution-loop-0036"));
        assert!(summary.contains(
            "daemon_log_tail latest_stdout_round=37 ledger_latest_round=36 stale_when_not_running=true round_ahead_of_ledger=true note=stdout_tail_is_history_not_active_process"
        ));
        assert!(text.contains("\"daemon_log_tail\":"));
        assert!(text.contains("\"latest_stdout_round\":37"));
        assert!(text.contains("\"ledger_latest_round\":36"));
        assert!(text.contains("\"stale_when_not_running\":true"));
        assert!(text.contains("\"round_ahead_of_ledger\":true"));
        assert!(text.contains("\"note\":\"stdout_tail_is_history_not_active_process\""));
    }

    #[test]
    fn summary_distinguishes_active_round_from_done_waiting_ledger_commit() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-round-transition-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let status = r#"{
            "daemon": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "running": true,
                "pid": 235440,
                "pid_file_exists": true,
                "stale_pid_file": false
            },
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "ledger": {
                    "total_records": 334,
                    "invalid_records": 0,
                    "duplicate_rounds": 0,
                    "round_gaps": 0,
                    "success_count": 334,
                    "feedback_applied_total": 1336,
                    "runtime_tokens_total": 8192,
                    "latest": {
                        "round": 334,
                        "case": "smartsteam-evolution-loop-0334",
                        "success": true,
                        "runtime_tokens": 64,
                        "feedback_applied": 4
                    }
                },
                "report": {"exists": false, "path": "missing-report.json"},
                "latest_done_round": 334,
                "round_in_progress": true,
                "context_hygiene_status": {
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "report_only": true,
                    "completed_window_evidence_non_actionable": true,
                    "future_work_requires_fresh_clean_room": true,
                    "reads_old_window_payload": false,
                    "reason_codes": ["completed_worker_evidence_only"]
                },
                "daemon_round_transition_status_v1": {
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "report_only": true,
                    "observed_round_done": false,
                    "transition_kind": "normal_in_progress",
                    "round_in_progress": true,
                    "active_round": 335,
                    "latest_done_round": 334,
                    "ledger_latest_round": 334,
                    "ledger_commit_pending": false,
                    "ledger_lag_rounds": 1,
                    "activity_reason": "generate:start",
                    "evidence_ids": ["active-round:335", "ledger:latest-round-334"],
                    "reason_codes": ["daemon_round_in_progress"],
                    "side_effects": {
                        "starts_daemon": false,
                        "stops_daemon": false,
                        "touches_remote": false,
                        "sends_prompt": false,
                        "starts_stream": false,
                        "replays_prompt": false,
                        "mutates_active_round": false,
                        "writes_ndkv": false
                    }
                },
                "backend": {
                    "checked": true,
                    "ok": true,
                    "readiness_ok": true,
                    "safe_device_ok": true,
                    "engine_busy": true,
                    "active_engine_requests": 1,
                    "gemma_runtime_model": "Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf",
                    "error": ""
                },
                "readiness": {"ready": true, "failures": []}
            }
        }"#;

        validate_read_only_status(status).unwrap();
        let summary = summarize_evolution_status(status, &work_dir.to_string_lossy()).unwrap();
        let text =
            render_enriched_evolution_status_json(status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&text).unwrap();
        assert!(summary.contains("daemon running=true pid=235440"));
        assert!(summary.contains("latest round=334 case=smartsteam-evolution-loop-0334"));
        assert!(summary.contains("daemon_round_transition status=normal_in_progress latest_round_state=round_in_progress round_in_progress=true read_only=true starts_process=false report_only=true observed_round_done=false active_round=335 done_round=334 ledger_round=334 ledger_commit_pending=false ledger_lag_rounds=1"));
        assert!(summary.contains("activity_reason=generate:start"));
        assert!(summary.contains("reason_codes=daemon_round_in_progress"));
        assert!(summary.contains("context_hygiene_status read_only=true starts_process=false sends_prompt=false report_only=true completed_window_evidence_non_actionable=true future_work_requires_fresh_clean_room=true reads_old_window_payload=false reason_codes=completed_worker_evidence_only"));
        assert!(summary.contains("backend checked=true ok=true readiness_ok=true safe_device_ok=true engine_busy=true active_requests=1"));
        assert!(text.contains("\"daemon_round_transition_status\":{"));
        assert!(text.contains("\"latest_round_state\":\"round_in_progress\""));
        assert!(text.contains("\"round_in_progress\":true"));
        assert!(text.contains("\"active_round\":335"));
        assert!(text.contains("\"done_round\":334"));
        assert!(text.contains("\"ledger_commit_pending\":false"));
        assert!(text.contains("\"writes_ndkv\":false"));
        assert!(text.contains("\"context_hygiene_status\":{"));
        assert!(text.contains("\"completed_window_evidence_non_actionable\":true"));
    }

    #[test]
    fn summary_surfaces_candidate_backlog_counts_and_latest_item() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-status-backlog-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        fs::write(
            work_dir.join(EVOLUTION_CANDIDATES_FILE),
            [
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-one","status":"new","round":"1","case":"case-1","model":"model-a","answer_preview":"first candidate"}"#,
                "not json",
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-two","status":"new","round":"2","case":"case-2","model":"model-b","answer_preview":"second candidate ready for implementation"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"smartsteam-candidate-two","status":"accepted","note":"ready"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_validation.v1","candidate_id":"smartsteam-candidate-two","command":"cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml","status_code":0,"passed":true,"note":"green","validated_unix":123}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false, "running": false},
            "loop": {"read_only": true, "starts_process": false, "sends_prompt": false, "touches_remote": false}
        }"#;

        let summary = summarize_evolution_status(status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(summary.contains("candidate_backlog path="));
        assert!(
            summary.contains("total=2 new=1 accepted=1 implemented=0 rejected=0 other=0 invalid=1")
        );
        assert!(summary.contains("candidate_backlog_validation ready=false accepted_pending=1 implemented_validated=0 implemented_unvalidated=0 implemented_failed=0"));
        assert!(summary.contains("daemon_start_gate candidate_lifecycle_ready=false blocks_unattended_start=true accepted_pending=1 implemented_unvalidated=0 implemented_failed=0 invalid=1"));
        assert!(summary.contains(
            "unattended_start_plan can_start=false candidate_lifecycle_ready=false readiness_start_ready=true readiness_blocks_start=false readiness_blocking_failures=none block_reason=candidate_backlog_not_ready"
        ));
        assert!(summary.contains("current_state=not_running_blocked"));
        assert!(
            summary.contains(
                "next_step=blocked: resolve candidate backlog before unattended evolution"
            )
        );
        assert!(summary.contains(
            "candidate_backlog_latest id=smartsteam-candidate-two status=accepted round=2 case=case-2 model=model-b preview=second candidate ready for implementation"
        ));
        assert!(summary.contains("candidate_backlog_latest_validation passed=true status_code=0 validated_unix=123 command=cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml"));
    }

    #[test]
    fn summary_candidate_lifecycle_gate_ready_after_implemented_validation() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-status-lifecycle-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        fs::write(
            work_dir.join(EVOLUTION_CANDIDATES_FILE),
            [
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-one","status":"new","round":"1","case":"case-1","model":"model-a","answer_preview":"candidate"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"smartsteam-candidate-one","status":"implemented","note":"done"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_validation.v1","candidate_id":"smartsteam-candidate-one","command":"cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml","status_code":0,"passed":true,"note":"green","validated_unix":456}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false, "running": false},
            "loop": {"read_only": true, "starts_process": false, "sends_prompt": false, "touches_remote": false}
        }"#;

        let summary = summarize_evolution_status(status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(
            summary.contains("total=1 new=0 accepted=0 implemented=1 rejected=0 other=0 invalid=0")
        );
        assert!(summary.contains("candidate_backlog_validation ready=true accepted_pending=0 implemented_validated=1 implemented_unvalidated=0 implemented_failed=0"));
        assert!(summary.contains("daemon_start_gate candidate_lifecycle_ready=true blocks_unattended_start=false accepted_pending=0 implemented_unvalidated=0 implemented_failed=0 invalid=0"));
        assert!(summary.contains(
            "candidate_backlog_latest_validation passed=true status_code=0 validated_unix=456"
        ));
    }

    #[test]
    fn summary_overrides_ready_next_step_when_candidate_backlog_has_invalid_records() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-status-invalid-backlog-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        fs::write(work_dir.join(EVOLUTION_CANDIDATES_FILE), "not json").unwrap();
        let status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false, "running": false},
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "readiness": {"ready": true, "failures": []},
                "next_step": "ready: run budgeted -Forever or inspect report gate"
            }
        }"#;

        let summary = summarize_evolution_status(status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(summary.contains("candidate_backlog_validation ready=false accepted_pending=0 implemented_validated=0 implemented_unvalidated=0 implemented_failed=0"));
        assert!(summary.contains("daemon_start_gate candidate_lifecycle_ready=false blocks_unattended_start=true accepted_pending=0 implemented_unvalidated=0 implemented_failed=0 invalid=1"));
        assert!(summary.contains(
            "unattended_start_plan can_start=false candidate_lifecycle_ready=false readiness_start_ready=true readiness_blocks_start=false readiness_blocking_failures=none block_reason=candidate_backlog_not_ready"
        ));
        assert!(summary.contains("current_state=not_running_blocked"));
        assert!(
            summary.contains(
                "next_step=blocked: resolve candidate backlog before unattended evolution"
            )
        );
        assert!(!summary.contains("next_step=ready: run budgeted -Forever"));
    }

    #[test]
    fn summary_explains_busy_backend_readiness_failure() {
        let status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false, "running": true},
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "backend": {
                    "checked": true,
                    "ok": true,
                    "readiness_ok": false,
                    "safe_device_ok": true,
                    "engine_busy": true,
                    "active_engine_requests": 1,
                    "gemma_runtime_model": "gemma-4-12b-it-Q8_0.gguf",
                    "error": ""
                },
                "readiness": {
                    "ready": false,
                    "failures": ["backend_not_ready"]
                }
            }
        }"#;

        let summary = summarize_evolution_status(status, "target\\evolution\\daemon").unwrap();

        assert!(summary.contains("engine_busy=true active_requests=1"));
        assert!(summary.contains("model=gemma-4-12b-it-Q8_0.gguf"));
        assert!(summary.contains("readiness ready=false failures=backend_not_ready"));
    }

    #[test]
    fn status_start_plan_blocks_when_backend_readiness_is_not_ready() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-status-readiness-block-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let status = r#"{
            "daemon": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "running": false
            },
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "backend_endpoint": "127.0.0.1:65534",
                "report": {"exists": false, "path": "missing-report.json"},
                "readiness": {
                    "ready": false,
                    "failures": [
                        "ledger_missing",
                        "rounds_below_minimum",
                        "feedback_below_minimum",
                        "backend_not_ready"
                    ]
                },
                "next_step": "ready: run budgeted -Forever or inspect report gate"
            }
        }"#;

        let summary = summarize_evolution_status(status, &work_dir.to_string_lossy()).unwrap();
        let text =
            render_enriched_evolution_status_json(status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(summary.contains(
            "unattended_start_plan can_start=false candidate_lifecycle_ready=true readiness_start_ready=false readiness_blocks_start=true readiness_blocking_failures=backend_not_ready block_reason=readiness_not_ready"
        ));
        assert!(summary.contains(
            "next_step=blocked: fix readiness before unattended evolution (backend_not_ready)"
        ));
        assert!(text.contains("\"can_start\":false"));
        assert!(text.contains("\"readiness_start_gate\":{"));
        assert!(text.contains("\"status_ready\":false"));
        assert!(text.contains("\"start_ready\":false"));
        assert!(text.contains("\"blocks_start\":true"));
        assert!(text.contains(
            "\"failures\":[\"ledger_missing\",\"rounds_below_minimum\",\"feedback_below_minimum\",\"backend_not_ready\"]"
        ));
        assert!(text.contains("\"start_blocking_failures\":[\"backend_not_ready\"]"));
        assert!(text.contains("\"readiness_start_ready\":false"));
        assert!(text.contains("\"readiness_blocks_start\":true"));
        assert!(text.contains("\"readiness_blocking_failures\":\"backend_not_ready\""));
        assert!(text.contains("\"block_reason\":\"readiness_not_ready\""));
        assert!(text.contains(
            "\"next_step\":\"blocked: fix readiness before unattended evolution (backend_not_ready)\""
        ));
    }

    #[test]
    fn start_plan_blocks_duplicate_start_when_daemon_is_running() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-status-running-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let status = r#"{
            "daemon": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "running": true,
                "pid": 4242,
                "pid_file_exists": true,
                "stale_pid_file": false
            },
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "backend_endpoint": "127.0.0.1:7979",
                "readiness": {"ready": true, "failures": []},
                "next_step": "ready: run budgeted -Forever or inspect report gate"
            }
        }"#;

        let summary = summarize_evolution_status(status, &work_dir.to_string_lossy()).unwrap();
        let text =
            render_enriched_evolution_status_json(status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(summary.contains(
            "unattended_start_plan can_start=false candidate_lifecycle_ready=true readiness_start_ready=true readiness_blocks_start=false readiness_blocking_failures=none block_reason=already_running"
        ));
        assert!(summary.contains("current_state=running"));
        assert!(summary.contains(
            "unattended_start_check=.\\tools\\smartsteam-forge\\evolution-daemon.cmd -StartCheck"
        ));
        assert!(text.contains("\"candidate_lifecycle_ready\":true"));
        assert!(text.contains("\"can_start\":false"));
        assert!(text.contains("\"current_state\":\"running\""));
        assert!(text.contains("\"block_reason\":\"already_running\""));
        assert!(text.contains(
            "\"next_step\":\"running: monitor JsonStatus; duplicate unattended start is blocked\""
        ));
        assert!(text.contains("\"stale_pid_file\":false"));
        assert!(text.contains("\"stale_pid_blocks_start\":false"));
    }

    #[test]
    fn json_mode_surfaces_machine_readable_report_gate_status() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-json-report-status-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let report_path = work_dir.join("report.json");
        fs::write(
            &report_path,
            r#"{
                "report_gate": {"passed": false, "failures": ["model_pool_alignment"]},
                "strict_report_gate": {
                    "passed": false,
                    "failures": ["runtime response failures 1 above maximum 0"]
                },
                "ledger_gate_report_v1": {
                    "allow_next_round": false,
                    "gate_blocked": true
                },
                "model_pool_alignment": {
                    "alignment_ok": false,
                    "route_dependency_failures": ["index:dependency_health_failed:required_roles=summary,router missing_roles=router"],
                    "route_blocked_or_failed": ["router:missing_dependency"],
                    "missing_status_roles": ["router"],
                    "missing_status_helper_roles": ["router"]
                },
                "test_gate": {
                    "latest_verdict": "fail",
                    "latest_validation_command_safety": "unsafe"
                }
            }"#,
        )
        .unwrap();
        let report_path_json = report_path.to_string_lossy().replace('\\', "\\\\");
        let status = format!(
            r#"{{
                "daemon": {{"read_only": true, "starts_process": false, "sends_prompt": false, "running": false}},
                "loop": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "touches_remote": false,
                    "backend_endpoint": "127.0.0.1:7979",
                    "report": {{"path": "{report_path_json}", "exists": true}},
                    "readiness": {{"ready": false, "failures": ["model_pool_alignment"]}},
                    "next_step": "fix status failures before unattended evolution"
                }}
            }}"#
        );

        let text =
            render_enriched_evolution_status_json(&status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(text.contains("\"report_gate_status\":"));
        assert!(text.contains("\"report_gate_preflight\":"));
        assert!(text.contains("\"report_gate_start_gate\":"));
        assert!(text.contains("\"report_exists\":true"));
        assert!(text.contains("\"report_read_ok\":true"));
        assert!(text.contains("\"report_gate_passed\":false"));
        assert!(text.contains("\"strict_report_gate_passed\":false"));
        assert!(text.contains(
            "\"strict_report_gate_failure_reasons\":[\"runtime response failures 1 above maximum 0\"]"
        ));
        assert!(text.contains("\"ledger_gate_allow_next_round\":false"));
        assert!(text.contains("\"ledger_gate_blocked\":true"));
        assert!(text.contains("\"model_pool_alignment_ok\":false"));
        assert!(text.contains("\"model_pool_route_dependency_failure_count\":1"));
        assert!(text.contains(
            "\"model_pool_route_dependency_failures\":[\"index:dependency_health_failed:required_roles=summary,router missing_roles=router\"]"
        ));
        assert!(
            text.contains("\"model_pool_route_blocked_or_failed\":[\"router:missing_dependency\"]")
        );
        assert!(text.contains("\"model_pool_missing_status_roles\":[\"router\"]"));
        assert!(text.contains("\"model_pool_missing_status_helper_roles\":[\"router\"]"));
        assert!(text.contains("\"test_gate_verdict\":\"fail\""));
        assert!(text.contains("\"test_gate_validation_command_safety\":\"unsafe\""));
        assert!(text.contains("\"can_continue_unattended\":false"));
        assert!(text.contains("\"unattended_start_plan\":"));
        assert!(text.contains("\"can_start\":false"));
        assert!(text.contains("\"current_state\":\"not_running_blocked\""));
        assert!(text.contains("\"block_reason\":\"report_gate_not_ready\""));
        assert!(text.contains("\"report_gate_continuation_state\":\"blocked\""));
        assert!(text.contains("\"report_gate_can_continue_unattended\":false"));
        assert!(text.contains("\"report_gate_blocks_continuation\":true"));
        assert!(text.contains("\"continuation_block_reason\":\"report_gate_not_ready\""));
        assert!(
            text.contains("\"next_step\":\"blocked: fix report gate before unattended evolution\"")
        );
    }

    #[test]
    fn json_mode_next_step_prefers_start_plan_when_ready_to_start() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-json-next-step-ready-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        let status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false, "running": false},
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "backend_endpoint": "127.0.0.1:7979",
                "report": {"exists": false, "path": "missing-report.json"},
                "readiness": {"ready": true, "failures": []},
                "next_step": "ready: run budgeted -Forever or inspect report gate"
            }
        }"#;

        let text =
            render_enriched_evolution_status_json(status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(text.contains("\"can_start\":true"));
        assert!(text.contains("\"current_state\":\"not_running_ready_to_start\""));
        assert!(text.contains("\"next_step\":\"ready_to_start: run StartCheck before Start: .\\\\tools\\\\smartsteam-forge\\\\evolution-daemon.cmd -StartCheck -WorkDir"));
        assert!(!text.contains("\"next_step\":\"ready: run budgeted -Forever"));
    }

    #[test]
    fn summary_reads_latest_answer_from_ledger_before_report_exists() {
        let ledger_path = std::env::temp_dir().join(format!(
            "smartsteam-forge-ledger-latest-{}.jsonl",
            std::process::id()
        ));
        fs::write(
            &ledger_path,
            r#"{"round":7,"runtime_model":"google/gemma-4-12B-it","runtime_tokens":64,"elapsed_ms":1234,"feedback_applied":4,"self_improve_passed":true,"answer":"**Improvement Candidate:** add a router compression gate"}"#,
        )
        .unwrap();
        let status = format!(
            r#"{{
                "daemon": {{"read_only": true, "starts_process": false, "sends_prompt": false, "running": true}},
                "loop": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "touches_remote": false,
                    "ledger": {{
                        "path": "{}",
                        "total_records": 7,
                        "invalid_records": 0,
                        "duplicate_rounds": 0,
                        "round_gaps": 0,
                        "success_count": 7,
                        "feedback_applied_total": 28,
                        "runtime_tokens_total": 448,
                        "latest": {{"round": 7, "case": "smartsteam-evolution-loop-0007", "success": true, "runtime_tokens": 64, "feedback_applied": 4}}
                    }},
                    "report": {{"exists": false, "path": "missing-report.json"}}
                }}
            }}"#,
            ledger_path.to_string_lossy().replace('\\', "\\\\")
        );

        let summary = summarize_evolution_status(&status, "target\\evolution\\daemon").unwrap();
        let _ = fs::remove_file(&ledger_path);

        assert!(summary.contains("latest_model_output round=7 model=google/gemma-4-12B-it runtime_tokens=64 elapsed_ms=1234 feedback=4 self_improve_passed=true"));
        assert!(summary.contains(
            "latest_answer_preview=**Improvement Candidate:** add a router compression gate"
        ));
    }

    #[test]
    fn json_mode_wraps_status_with_candidate_start_gate() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-json-status-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        fs::write(
            work_dir.join(EVOLUTION_CANDIDATES_FILE),
            [
                r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-one","status":"new","round":"1","case":"case-1","model":"model-a","answer_preview":"candidate"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"smartsteam-candidate-one","status":"implemented","note":"done"}"#,
                r#"{"schema":"smartsteam.evolution_candidate_validation.v1","candidate_id":"smartsteam-candidate-one","command":"cargo test","status_code":0,"passed":true,"validated_unix":456}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        validate_read_only_status(SAMPLE_STATUS).unwrap();

        let text =
            render_enriched_evolution_status_json(SAMPLE_STATUS, &work_dir.to_string_lossy())
                .unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        validate_read_only_enriched_status(&text).unwrap();
        assert!(text.contains("\"schema\":\"smartsteam.forge.evolution_status.v1\""));
        assert!(text.contains("\"read_only\":true"));
        assert!(text.contains("\"starts_process\":false"));
        assert!(text.contains("\"sends_prompt\":false"));
        assert!(text.contains("\"evolution_status\":"));
        assert!(text.contains("\"daemon_log_tail\":"));
        assert!(text.contains("\"report_gate_status\":"));
        assert!(text.contains("\"next_round_decision_status\":"));
        assert!(text.contains("\"decision\":\"safe-to-continue-after-current-round\""));
        assert!(text.contains("\"evidence_ids\":[\"round:4:report-gate-pass\"]"));
        assert!(text.contains("\"daemon\""));
        assert!(text.contains("\"loop\""));
        assert!(text.contains("\"candidate_backlog\":"));
        assert!(text.contains("\"validation_ready\":true"));
        assert!(text.contains("\"candidate_id\":\"smartsteam-candidate-one\""));
        assert!(text.contains("\"daemon_start_gate\":"));
        assert!(text.contains("\"report_gate_start_gate\":"));
        assert!(text.contains("\"readiness_start_gate\":"));
        assert!(text.contains("\"status_ready\":true"));
        assert!(text.contains("\"start_ready\":true"));
        assert!(text.contains("\"blocks_start\":false"));
        assert!(text.contains("\"start_blocking_failures\":[]"));
        assert!(text.contains("\"block_reason\":null"));
        assert!(text.contains("\"candidate_lifecycle_ready\":true"));
        assert!(text.contains("\"blocks_unattended_start\":false"));
        assert!(text.contains("\"unattended_start_plan\":"));
        assert!(text.contains("\"can_start\":"));
        assert!(text.contains("\"current_state\":"));
        assert!(text.contains("\"block_reason\":"));
        assert!(text.contains("\"remote_runtime_acceleration_ok\":false"));
        assert!(text.contains("\"remote_runtime_acceleration_blocks_start\":false"));
        assert!(text.contains(
            "\"remote_runtime_cpu_or_no_gpu_roles\":[\"summary\",\"review\",\"test-gate\"]"
        ));
        assert!(text.contains("\"report_gate_continuation_state\":"));
        assert!(text.contains("\"report_gate_can_continue_unattended\":"));
        assert!(text.contains("\"report_gate_blocks_continuation\":"));
        assert!(text.contains("\"continuation_block_reason\":"));
        assert!(text.contains("\"stale_pid_file\":true"));
        assert!(text.contains("\"stale_pid\":112704"));
        assert!(text.contains("\"stale_pid_blocks_start\":false"));
        assert!(text.contains(
            "\"stale_pid_cleanup_command\":\".\\\\tools\\\\smartsteam-forge\\\\evolution-daemon.cmd -Stop -WorkDir"
        ));
        assert!(text.contains("evolution-daemon.cmd -StartCheck -WorkDir"));
        assert!(text.contains("-Backend '127.0.0.1:7979'"));
    }

    #[test]
    fn json_mode_overrides_next_step_when_candidate_gate_blocks_start() {
        let work_dir = std::env::temp_dir().join(format!(
            "smartsteam-forge-json-status-blocked-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&work_dir).unwrap();
        fs::write(
            work_dir.join(EVOLUTION_CANDIDATES_FILE),
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-dirty","status":"accepted","round":"1","case":"case-1","model":"model-a","answer_preview":"dirty"}"#,
        )
        .unwrap();
        let status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false, "running": false},
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "readiness": {"ready": true, "failures": []},
                "next_step": "ready: run budgeted -Forever or inspect report gate"
            }
        }"#;

        let text =
            render_enriched_evolution_status_json(status, &work_dir.to_string_lossy()).unwrap();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(text.contains("\"candidate_lifecycle_ready\":false"));
        assert!(text.contains("\"blocks_unattended_start\":true"));
        assert!(text.contains("\"accepted_pending\":1"));
        assert!(text.contains("\"unattended_start_plan\":"));
        assert!(text.contains("\"can_start\":false"));
        assert!(text.contains("\"current_state\":\"not_running_blocked\""));
        assert!(text.contains("\"block_reason\":\"candidate_backlog_not_ready\""));
        assert!(text.contains("\"stale_pid_file\":false"));
        assert!(text.contains("\"stale_pid\":null"));
        assert!(text.contains("\"stale_pid_cleanup_command\":null"));
        assert!(text.contains(
            "\"next_step\":\"blocked: resolve candidate backlog before unattended evolution\""
        ));
        assert!(text.contains("\"evolution_status\":{"));
    }
}
