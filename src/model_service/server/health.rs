mod hygiene;
mod readiness;
mod runtime;

use rust_norion::{HardwareAllocator, HierarchyWeights};

use self::hygiene::experience_hygiene_health_status;
use self::readiness::health_readiness_report;
use self::runtime::{option_bool_json, option_usize_json, runtime_health_status};
use super::state::{
    ModelServiceActiveRequestTelemetry, ModelServiceLastInferenceTelemetry, ModelServiceServerState,
};
use crate::Args;
use crate::model_service::json::{
    option_f32_service_json, option_str_service_json, service_json_string,
    service_json_string_array,
};

pub(super) fn model_service_health_json(
    request_id: usize,
    state: &ModelServiceServerState,
    args: &Args,
) -> String {
    let active_engine_requests = state.active_engine_requests();
    let stream_backpressure_rejections = state.stream_backpressure_rejections();
    let runtime = runtime_health_status(args);
    let experience_hygiene = experience_hygiene_health_status(args);
    let probe = args.effective_probe_report();
    let plan = HardwareAllocator::new().plan(
        probe.snapshot(),
        args.profile,
        args.prompt_token_estimate(),
        HierarchyWeights::default(),
    );
    let readiness = health_readiness_report(
        args,
        runtime.gemma_reachable,
        runtime.gemma_context_window,
        active_engine_requests,
        &plan,
        &experience_hygiene,
    );

    format!(
        "{{\"ok\":true,\"service\":\"rust-norion\",\"requests_seen\":{},\"active_engine_requests\":{},\"stream_backpressure_rejections\":{},\"engine_busy\":{},\"active_requests\":{},\"runtime_mode\":\"{}\",\"gemma_runtime_server\":{},\"gemma_runtime_reachable\":{},\"gemma_runtime_model\":{},\"gemma_runtime_context_window\":{},\"gemma_runtime_train_context_window\":{},\"gemma_runtime_vocab_size\":{},\"gemma_runtime_metadata_error\":{},\"experience_hygiene\":{},\"readiness_ok\":{},\"safe_device_ok\":{},\"readiness_failures\":{},\"safe_device_failures\":{},\"device_profile\":{},\"device_reason\":{},\"device_accelerators\":{},\"device_pressure\":{:.6},\"device_primary_lane\":{},\"device_fallback_lane\":{},\"device_memory_mode\":{},\"device_adapter_hints\":{},\"device_parallel_chunks\":{},\"device_kv_prefetch\":{},\"device_hot_kv_bits\":{},\"device_cold_kv_bits\":{},\"device_allow_disk_spill\":{},\"device_plan_summary\":{},\"device_probe_summary\":{},\"readiness_warnings\":{},\"last_inference\":{}}}",
        request_id.saturating_sub(1),
        active_engine_requests,
        stream_backpressure_rejections,
        active_engine_requests > 0,
        active_requests_json(&state.active_requests()),
        runtime.mode,
        option_str_service_json(args.gemma_runtime_server.as_deref()),
        option_bool_json(runtime.gemma_reachable),
        option_str_service_json(runtime.gemma_model.as_deref()),
        option_usize_json(runtime.gemma_context_window),
        option_usize_json(runtime.gemma_train_context_window),
        option_usize_json(runtime.gemma_vocab_size),
        option_str_service_json(runtime.gemma_metadata_error.as_deref()),
        experience_hygiene.json(),
        readiness.readiness_failures.is_empty(),
        readiness.safe_device_failures.is_empty(),
        service_json_string_array(&readiness.readiness_failures),
        service_json_string_array(&readiness.safe_device_failures),
        service_json_string(probe.device.as_str()),
        service_json_string(&probe.reason),
        probe.accelerator_count,
        plan.pressure,
        service_json_string(plan.execution.primary_lane.as_str()),
        service_json_string(plan.execution.fallback_lane.as_str()),
        service_json_string(plan.execution.memory_mode.as_str()),
        service_json_string_array(
            &plan
                .execution
                .adapter_hints
                .iter()
                .map(|adapter| adapter.as_str().to_owned())
                .collect::<Vec<_>>()
        ),
        plan.execution.max_parallel_chunks,
        plan.execution.kv_prefetch_blocks,
        plan.execution.hot_kv_precision_bits,
        plan.execution.cold_kv_precision_bits,
        plan.execution.allow_disk_spill,
        service_json_string(&plan.runtime_contract_summary()),
        service_json_string(&probe.summary()),
        service_json_string_array(&readiness.warnings),
        last_inference_json(state.last_inference().as_ref())
    )
}

fn active_requests_json(active_requests: &[ModelServiceActiveRequestTelemetry]) -> String {
    let items = active_requests
        .iter()
        .map(|request| {
            format!(
                "{{\"request_id\":{},\"endpoint\":{},\"elapsed_ms\":{},\"prompt_preview\":{},\"cancel_requested\":{},\"repair_factor\":{},\"retag_label\":{},\"cancel_reason\":{}}}",
                request.request_id,
                service_json_string(&request.endpoint),
                request.elapsed_ms(),
                service_json_string(&request.prompt_preview),
                request.cancel_requested,
                option_str_service_json(request.repair_factor.as_deref()),
                option_str_service_json(request.retag_label.as_deref()),
                option_str_service_json(request.cancel_reason.as_deref())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn last_inference_json(telemetry: Option<&ModelServiceLastInferenceTelemetry>) -> String {
    let Some(telemetry) = telemetry else {
        return "null".to_owned();
    };
    format!(
        "{{\"request_id\":{},\"endpoint\":{},\"elapsed_ms\":{},\"runtime_model\":{},\"runtime_token_count\":{},\"used_memory_count\":{},\"route_threshold\":{:.6},\"route_attention_tokens\":{},\"route_fast_tokens\":{},\"route_attention_fraction\":{:.6},\"runtime_kv_influence\":{},\"runtime_imported_kv_blocks\":{},\"runtime_weak_kv_imports_skipped\":{},\"runtime_budget_limited_kv_imports_skipped\":{},\"runtime_kv_budget_pressure\":{:.6},\"runtime_exported_kv_blocks\":{},\"runtime_kv_segments_included\":{},\"runtime_kv_segments_skipped\":{},\"runtime_kv_segments_rejected\":{},\"runtime_kv_segment_yield\":{},\"quality\":{:.6},\"process_reward\":{:.6},\"action\":{},\"error\":{},\"cancelled\":{},\"timeout\":{},\"retryable\":{},\"runtime_error_note\":{}}}",
        telemetry.request_id,
        service_json_string(&telemetry.endpoint),
        telemetry.elapsed_ms,
        option_str_service_json(telemetry.runtime_model.as_deref()),
        telemetry.runtime_token_count,
        telemetry.used_memory_count,
        telemetry.route_threshold,
        telemetry.route_attention_tokens,
        telemetry.route_fast_tokens,
        telemetry.route_attention_fraction,
        option_f32_service_json(telemetry.runtime_kv_influence),
        telemetry.runtime_imported_kv_blocks,
        telemetry.runtime_weak_kv_imports_skipped,
        telemetry.runtime_budget_limited_kv_imports_skipped,
        telemetry.runtime_kv_budget_pressure,
        telemetry.runtime_exported_kv_blocks,
        telemetry.runtime_kv_segments_included,
        telemetry.runtime_kv_segments_skipped,
        telemetry.runtime_kv_segments_rejected,
        option_f32_service_json(telemetry.runtime_kv_segment_yield),
        telemetry.quality,
        telemetry.process_reward,
        service_json_string(&telemetry.action),
        option_str_service_json(telemetry.error.as_deref()),
        telemetry.cancelled,
        telemetry.timeout,
        telemetry.retryable,
        option_str_service_json(telemetry.runtime_error_note.as_deref())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Args;
    use rust_norion::{
        ExperienceInput, ExperienceRuntimeTokenMetrics, ExperienceStore, GistLevel, GistRecord,
        HierarchyWeights, LiveInferenceEvolution, ProcessRewardReport, RouteBudget,
        RuntimeDiagnostics, TaskProfile,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn health_json_reports_manual_discrete_gpu_plan() {
        let args = Args::parse(vec![
            "--device".to_owned(),
            "discrete-gpu".to_owned(),
            "--gemma-12b-runtime".to_owned(),
        ]);
        let state = ModelServiceServerState::default();

        let body = model_service_health_json(1, &state, &args);

        assert!(body.contains("\"runtime_mode\":\"gemma-command\""));
        assert!(body.contains("\"stream_backpressure_rejections\":0"));
        assert!(body.contains("\"device_profile\":\"discrete\""));
        assert!(body.contains("\"device_primary_lane\":\"discrete-gpu\""));
        assert!(body.contains("\"device_memory_mode\":\"gpu-resident\""));
        assert!(body.contains("\"readiness_ok\":true"));
        assert!(body.contains("\"safe_device_ok\":true"));
        assert!(!body.contains("gemma_12b_device"));
    }

    #[test]
    fn health_json_warns_when_gemma_12b_is_cpu_first() {
        let args = Args::parse(vec![
            "--device".to_owned(),
            "cpu".to_owned(),
            "--gemma-12b-runtime".to_owned(),
        ]);
        let state = ModelServiceServerState::default();

        let body = model_service_health_json(1, &state, &args);

        assert!(body.contains("\"device_profile\":\"cpu\""));
        assert!(body.contains("\"device_primary_lane\":\"cpu-vector\""));
        assert!(body.contains("\"readiness_ok\":true"));
        assert!(body.contains("\"safe_device_ok\":false"));
        assert!(body.contains("\"safe_device_failures\":["));
        assert!(body.contains("gemma_12b_device"));
        assert!(body.contains("CPU/disk-first"));
    }

    #[test]
    fn health_json_reports_unreachable_gemma_http_runtime() {
        let args = Args::parse(vec![
            "--gemma-runtime-server".to_owned(),
            "http://127.0.0.1:9".to_owned(),
        ]);
        let state = ModelServiceServerState::default();

        let body = model_service_health_json(7, &state, &args);

        assert!(body.contains("\"requests_seen\":6"));
        assert!(body.contains("\"runtime_mode\":\"gemma-http\""));
        assert!(body.contains("\"gemma_runtime_reachable\":false"));
        assert!(body.contains("\"readiness_ok\":false"));
        assert!(body.contains("\"readiness_failures\":["));
        assert!(body.contains("gemma_runtime"));
    }

    #[test]
    fn health_json_reports_active_request_details() {
        let args = Args::parse(vec![]);
        let state = ModelServiceServerState::default();
        let _active = state.begin_engine_request(42, "chat-stream", "帮我用 Rust 写一个 for 循环");

        let body = model_service_health_json(43, &state, &args);

        assert!(body.contains("\"active_engine_requests\":1"));
        assert!(body.contains("\"engine_busy\":true"));
        assert!(body.contains("\"active_requests\":[{"));
        assert!(body.contains("\"request_id\":42"));
        assert!(body.contains("\"endpoint\":\"chat-stream\""));
        assert!(body.contains("\"prompt_preview\":\"帮我用 Rust 写一个 for 循环\""));
        assert!(body.contains("\"cancel_requested\":false"));
        assert!(body.contains("\"repair_factor\":null"));
    }

    #[test]
    fn health_json_reports_cancel_repair_factor_and_retag() {
        let args = Args::parse(vec![]);
        let state = ModelServiceServerState::default();
        let _active = state.begin_engine_request(42, "business-cycle-stream", "stalled round");
        state.request_cancel(
            42,
            "operator_runtime_splice",
            "repair_factor:runtime_splice",
        );

        let body = model_service_health_json(43, &state, &args);

        assert!(body.contains("\"active_engine_requests\":1"));
        assert!(body.contains("\"request_id\":42"));
        assert!(body.contains("\"cancel_requested\":true"));
        assert!(body.contains("\"repair_factor\":\"runtime_request_splice\""));
        assert!(body.contains("\"retag_label\":\"repair_factor:runtime_splice\""));
        assert!(body.contains("\"cancel_reason\":\"operator_runtime_splice\""));
    }

    #[test]
    fn health_json_reports_missing_experience_hygiene_without_creating_file() {
        let experience_path = temp_health_path("missing-experience");
        let args = Args::parse(vec![
            "--experience".to_owned(),
            experience_path.display().to_string(),
        ]);
        let state = ModelServiceServerState::default();

        let body = model_service_health_json(1, &state, &args);

        assert!(body.contains("\"experience_hygiene\":{"));
        assert!(body.contains(&format!(
            "\"experience_file\":{}",
            service_json_string(&experience_path.display().to_string())
        )));
        assert!(body.contains("\"checked\":false"));
        assert!(body.contains("\"clean\":null"));
        assert!(body.contains("\"findings\":null"));
        assert!(body.contains("\"quarantine_candidates\":null"));
        assert!(body.contains("\"error\":\"experience_file_missing\""));
        assert!(!experience_path.exists());
    }

    #[test]
    fn health_json_defers_large_experience_hygiene_scan() {
        let experience_path = temp_health_path("large-experience");
        fs::write(&experience_path, vec![b'x'; 1_000_001]).unwrap();
        let args = Args::parse(vec![
            "--experience".to_owned(),
            experience_path.display().to_string(),
        ]);
        let state = ModelServiceServerState::default();

        let body = model_service_health_json(1, &state, &args);

        assert!(body.contains("\"experience_hygiene\":{"));
        assert!(body.contains("\"checked\":false"));
        assert!(body.contains("\"clean\":null"));
        assert!(body.contains("\"index\":null"));
        assert!(body.contains("experience_hygiene_deferred_large_file"));
        assert!(body.contains("\"readiness_ok\":true"));

        let _ = fs::remove_file(experience_path);
    }

    #[test]
    fn health_json_reports_idle_status_and_last_inference_visibility() {
        let experience_path = temp_health_path("last-inference-missing-experience");
        let args = Args::parse(vec![
            "--experience".to_owned(),
            experience_path.display().to_string(),
        ]);
        let state = ModelServiceServerState::default();

        let idle_body = model_service_health_json(1, &state, &args);

        assert!(idle_body.contains("\"engine_busy\":false"));
        assert!(idle_body.contains("\"active_requests\":[]"));
        assert!(idle_body.contains("\"readiness_ok\":true"));
        assert!(idle_body.contains("\"safe_device_ok\":true"));
        assert!(idle_body.contains("\"last_inference\":null"));

        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
            77,
            "generate",
            "backend unavailable",
            false,
            true,
            true,
            Some("backend unavailable"),
        ));
        let body = model_service_health_json(78, &state, &args);

        assert!(body.contains("\"last_inference\":{"));
        assert!(body.contains("\"request_id\":77"));
        assert!(body.contains("\"endpoint\":\"generate\""));
        assert!(body.contains("\"runtime_model\":null"));
        assert!(body.contains("\"runtime_token_count\":0"));
        assert!(body.contains("\"used_memory_count\":0"));
        assert!(body.contains("\"route_threshold\":0.000000"));
        assert!(body.contains("\"route_attention_tokens\":0"));
        assert!(body.contains("\"route_fast_tokens\":0"));
        assert!(body.contains("\"route_attention_fraction\":0.000000"));
        assert!(body.contains("\"runtime_kv_influence\":null"));
        assert!(body.contains("\"runtime_imported_kv_blocks\":0"));
        assert!(body.contains("\"runtime_weak_kv_imports_skipped\":0"));
        assert!(body.contains("\"runtime_budget_limited_kv_imports_skipped\":0"));
        assert!(body.contains("\"runtime_kv_budget_pressure\":0.000000"));
        assert!(body.contains("\"runtime_exported_kv_blocks\":0"));
        assert!(body.contains("\"runtime_kv_segments_included\":0"));
        assert!(body.contains("\"runtime_kv_segments_skipped\":0"));
        assert!(body.contains("\"runtime_kv_segments_rejected\":0"));
        assert!(body.contains("\"runtime_kv_segment_yield\":null"));
        assert!(body.contains("\"action\":\"error\""));
        assert!(body.contains("\"error\":\"backend unavailable\""));
        assert!(body.contains("\"cancelled\":false"));
        assert!(body.contains("\"timeout\":true"));
        assert!(body.contains("\"retryable\":true"));
        assert!(body.contains("\"runtime_error_note\":\"backend unavailable\""));

        state.record_inference(ModelServiceLastInferenceTelemetry::error_with_state(
            79,
            "generate-stream",
            "request cancelled",
            true,
            false,
            false,
            None,
        ));
        let cancelled_body = model_service_health_json(80, &state, &args);
        assert!(cancelled_body.contains("\"endpoint\":\"generate-stream\""));
        assert!(cancelled_body.contains("\"cancelled\":true"));
        assert!(cancelled_body.contains("\"timeout\":false"));
        assert!(cancelled_body.contains("\"retryable\":false"));
        assert!(cancelled_body.contains("\"runtime_error_note\":null"));
        assert!(!experience_path.exists());
    }

    #[test]
    fn health_json_warns_for_dirty_experience_hygiene() {
        let experience_path = temp_health_path("dirty-experience");
        let mut store = ExperienceStore::new();
        store.record(ExperienceInput {
            prompt: "Conversation transcript:\nuser: audit GitLab merge_requests over ssh\nassistant: ok"
                .to_owned(),
            profile: TaskProfile::Coding,
            lesson: "ssh -o ConnectTimeout=8 gitlab.local merge_requests bash command".to_owned(),
            quality: 0.94,
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
            hierarchy: HierarchyWeights::new(0.33, 0.34, 0.33),
            used_memory_ids: Vec::new(),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: RuntimeDiagnostics::default(),
            runtime_token_metrics: ExperienceRuntimeTokenMetrics::default(),
            process_reward: ProcessRewardReport::default(),
            live_evolution: LiveInferenceEvolution::default(),
        });
        store.save_to_disk_kv(&experience_path).unwrap();
        let args = Args::parse(vec![
            "--experience".to_owned(),
            experience_path.display().to_string(),
        ]);
        let state = ModelServiceServerState::default();

        let body = model_service_health_json(1, &state, &args);

        assert!(body.contains("\"experience_hygiene\":{"));
        assert!(body.contains("\"checked\":true"));
        assert!(body.contains("\"clean\":false"));
        assert!(body.contains("\"findings\":1"));
        assert!(body.contains("\"quarantine_candidates\":1"));
        assert!(body.contains("experience_hygiene: 1 quarantine candidates"));

        let _ = fs::remove_file(experience_path);
    }

    #[test]
    fn health_json_warns_for_repairable_legacy_metadata() {
        let experience_path = temp_health_path("repairable-experience");
        let mut store = ExperienceStore::new();
        store.record(ExperienceInput {
            prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant:"
                .to_owned(),
            lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
                .to_owned(),
            gist_records: vec![GistRecord {
                level: GistLevel::Document,
                title: "rust-loop".to_owned(),
                summary: "这是一个 Rust for 循环示例，使用 for i in 0..10 并 println 输出"
                    .to_owned(),
                source_tokens: 24,
                importance: 0.84,
            }],
            ..experience_input("repairable legacy metadata", 0.78)
        });
        store.save_to_disk_kv(&experience_path).unwrap();
        let args = Args::parse(vec![
            "--experience".to_owned(),
            experience_path.display().to_string(),
        ]);
        let state = ModelServiceServerState::default();

        let body = model_service_health_json(1, &state, &args);

        assert!(body.contains("\"experience_hygiene\":{"));
        assert!(body.contains("\"checked\":true"));
        assert!(body.contains("\"clean\":false"));
        assert!(body.contains("\"legacy_metadata_lessons\":1"));
        assert!(body.contains("\"repair\":{"));
        assert!(body.contains("\"repairable_legacy_metadata_lessons\":1"));
        assert!(body.contains("\"repairable_index_records\":0"));
        assert!(body.contains("\"projected_findings_after_repair\":0"));
        assert!(body.contains("\"skipped_quarantine_candidates\":0"));
        assert!(body.contains("\"skipped_missing_clean_gist\":0"));
        assert!(body.contains("experience_repair: 1 legacy metadata lessons can be repaired"));

        let _ = fs::remove_file(experience_path);
    }

    #[test]
    fn health_json_reports_experience_index_quality() {
        let experience_path = temp_health_path("index-quality-experience");
        let mut store = ExperienceStore::new();
        let duplicate_lesson =
            "when the local Gemma worker is busy, route cheap repository indexing to the index helper before retrying quality ".repeat(2);
        store.record(ExperienceInput {
            lesson: duplicate_lesson.clone(),
            ..experience_input("index duplicate source", 0.91)
        });
        let duplicate_id = store.record(ExperienceInput {
            lesson: duplicate_lesson.clone(),
            ..experience_input("index duplicate copy", 0.92)
        });
        let duplicate = store.record_mut(duplicate_id).unwrap();
        assert!(duplicate.lesson.starts_with("duplicate_reference:"));
        duplicate.lesson = duplicate_lesson.clone();
        duplicate.quality = 0.92;
        duplicate
            .process_reward
            .notes
            .retain(|note| !note.starts_with("experience_index:duplicate_reference:"));
        store.save_to_disk_kv(&experience_path).unwrap();
        let loaded = ExperienceStore::load_from_disk_kv(&experience_path).unwrap();
        assert!(
            loaded
                .records()
                .iter()
                .find(|record| record.id == duplicate_id)
                .unwrap()
                .lesson
                == duplicate_lesson,
            "{:?}",
            loaded
                .records()
                .iter()
                .map(|record| record.lesson.clone())
                .collect::<Vec<_>>()
        );
        let args = Args::parse(vec![
            "--experience".to_owned(),
            experience_path.display().to_string(),
        ]);
        let state = ModelServiceServerState::default();

        let body = model_service_health_json(1, &state, &args);

        assert!(body.contains("\"experience_hygiene\":{"));
        assert!(body.contains("\"clean\":true"));
        assert!(body.contains("\"index\":{"));
        assert!(body.contains("\"repairable_index_records\":1"), "{body}");
        assert!(body.contains("\"duplicate_outputs\":1"), "{body}");
        assert!(body.contains("\"quality_score\":0.580000"), "{body}");
        assert!(body.contains("\"retrieval_ready\":true"));
        assert!(body.contains("\"risk_level\":\"degraded\""), "{body}");
        assert!(body.contains("experience_repair: 1 index records can be repaired"));
        assert!(body.contains(
            "experience_index: risk_level=degraded quality_score=0.580 noisy_records=1 duplicate_outputs=1"
        ));

        let _ = fs::remove_file(experience_path);
    }

    fn experience_input(lesson: &str, quality: f32) -> ExperienceInput {
        ExperienceInput {
            prompt: "health test prompt".to_owned(),
            profile: TaskProfile::Coding,
            lesson: lesson.to_owned(),
            quality,
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
            hierarchy: HierarchyWeights::new(0.33, 0.34, 0.33),
            used_memory_ids: Vec::new(),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: RuntimeDiagnostics::default(),
            runtime_token_metrics: ExperienceRuntimeTokenMetrics::default(),
            process_reward: ProcessRewardReport::default(),
            live_evolution: LiveInferenceEvolution::default(),
        }
    }

    fn temp_health_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-health-{label}-{}-{nanos}.ndkv",
            std::process::id()
        ))
    }
}
