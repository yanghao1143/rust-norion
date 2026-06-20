use super::parse_provider_health;

#[test]
fn parses_runtime_health_summary() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":1,\"engine_busy\":true,\"active_requests\":[{\"request_id\":42,\"endpoint\":\"chat-stream\",\"elapsed_ms\":1234,\"prompt_preview\":\"Rust for loop\"}],\"runtime_mode\":\"gemma-http\",\"gemma_runtime_server\":\"http://127.0.0.1:8686\",\"gemma_runtime_reachable\":false,\"last_inference\":{\"endpoint\":\"chat\",\"elapsed_ms\":123,\"runtime_model\":\"gemma\",\"runtime_token_count\":42,\"error\":null}}",
    );

    assert!(health.ok);
    assert_eq!(health.service.as_deref(), Some("rust-norion"));
    assert_eq!(health.active_engine_requests.as_deref(), Some("1"));
    assert_eq!(health.engine_busy, Some(true));
    assert_eq!(health.active_requests.len(), 1);
    assert_eq!(health.active_requests[0].request_id.as_deref(), Some("42"));
    assert_eq!(
        health.active_requests[0].endpoint.as_deref(),
        Some("chat-stream")
    );
    assert_eq!(
        health.active_requests[0].prompt_preview.as_deref(),
        Some("Rust for loop")
    );
    assert_eq!(health.runtime_mode.as_deref(), Some("gemma-http"));
    assert_eq!(health.gemma_runtime_reachable, Some(false));
    assert!(health.summary().contains("active_request=#42:chat-stream"));
    assert!(health.summary().contains("active_prompt=\"Rust for loop\""));
    assert!(health.summary().contains("last_tokens=42"));
}

#[test]
fn parses_backend_device_health_fields() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"gemma-command\",\"gemma_runtime_reachable\":null,\"readiness_ok\":true,\"safe_device_ok\":false,\"readiness_failures\":[],\"safe_device_failures\":[\"gemma_12b_device: selected plan is CPU/disk-first\"],\"device_profile\":\"cpu\",\"device_accelerators\":0,\"device_pressure\":0.810000,\"device_primary_lane\":\"cpu-vector\",\"device_memory_mode\":\"tiered-disk\",\"device_plan_summary\":\"primary=cpu-vector fallback=disk-backed-streaming\",\"device_probe_summary\":\"device=cpu accelerators=0\",\"readiness_warnings\":[\"gemma_12b_device: selected plan is CPU/disk-first\"]}",
    );

    assert_eq!(health.readiness_ok, Some(true));
    assert_eq!(health.safe_device_ok, Some(false));
    assert!(health.readiness_failures.is_empty());
    assert_eq!(health.safe_device_failures.len(), 1);
    assert_eq!(health.device_profile.as_deref(), Some("cpu"));
    assert_eq!(health.device_accelerators.as_deref(), Some("0"));
    assert_eq!(health.device_pressure.as_deref(), Some("0.810000"));
    assert_eq!(health.device_primary_lane.as_deref(), Some("cpu-vector"));
    assert_eq!(health.device_memory_mode.as_deref(), Some("tiered-disk"));
    assert_eq!(
        health.device_plan_summary.as_deref(),
        Some("primary=cpu-vector fallback=disk-backed-streaming")
    );
    assert_eq!(
        health.device_probe_summary.as_deref(),
        Some("device=cpu accelerators=0")
    );
    assert_eq!(health.readiness_warnings.len(), 1);
    assert!(health.summary().contains("device=cpu"));
    assert!(health.summary().contains("accelerators=0"));
    assert!(health.summary().contains("device_pressure=0.810000"));
    assert!(health.summary().contains("device_plan="));
    assert!(health.summary().contains("gemma_12b_device"));
    assert!(
        health
            .require_safe_device()
            .unwrap_err()
            .contains("CPU/disk-first")
    );
}

#[test]
fn readiness_rejects_busy_backend() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":1,\"engine_busy\":true,\"active_requests\":[{\"request_id\":42,\"endpoint\":\"chat-stream\",\"elapsed_ms\":1234,\"prompt_preview\":\"Rust for loop\"}],\"runtime_mode\":\"built-in\",\"gemma_runtime_reachable\":null}",
    );

    let error = health.require_ready().unwrap_err();

    assert!(error.contains("busy"));
    assert!(error.contains("request_id=42"));
    assert!(error.contains("prompt_preview=\"Rust for loop\""));
}

#[test]
fn readiness_rejects_unreachable_gemma_http_runtime() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"gemma-http\",\"gemma_runtime_server\":\"http://127.0.0.1:8686\",\"gemma_runtime_reachable\":false,\"readiness_ok\":false,\"readiness_failures\":[\"gemma_runtime: configured Gemma HTTP runtime is not reachable\"]}",
    );

    assert!(
        health
            .require_ready()
            .unwrap_err()
            .contains("backend readiness failed")
    );
}

#[test]
fn readiness_accepts_idle_builtin_backend_without_gemma_probe() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"built-in\",\"gemma_runtime_reachable\":null}",
    );

    assert!(health.require_ready().is_ok());
}

#[test]
fn readiness_accepts_reachable_gemma_http_runtime() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"gemma-http\",\"gemma_runtime_server\":\"http://127.0.0.1:8686\",\"gemma_runtime_reachable\":true}",
    );

    assert!(health.require_ready().is_ok());
}

#[test]
fn safe_device_accepts_gpu_backed_gemma_runtime() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"gemma-command\",\"device_profile\":\"discrete\",\"device_primary_lane\":\"discrete-gpu\",\"device_memory_mode\":\"gpu-resident\",\"readiness_warnings\":[]}",
    );

    assert!(health.require_safe_device().is_ok());
}

#[test]
fn readiness_rejects_structured_backend_failure_without_legacy_busy_fields() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"runtime_mode\":\"built-in\",\"readiness_ok\":false,\"readiness_failures\":[\"engine_busy: wait\"]}",
    );

    assert!(health.require_ready().unwrap_err().contains("engine_busy"));
}

#[test]
fn parses_experience_hygiene_health_fields() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"built-in\",\"experience_hygiene\":{\"checked\":true,\"clean\":false,\"findings\":4,\"watch\":0,\"quarantine_candidates\":4,\"legacy_metadata_lessons\":860,\"legacy_metadata_without_clean_gist\":29,\"repair\":{\"repairable_legacy_metadata_lessons\":828,\"repairable_index_records\":1,\"projected_findings_after_repair\":32,\"projected_watch_after_repair\":28,\"projected_quarantine_candidates_after_repair\":4,\"projected_legacy_metadata_lessons_after_repair\":32,\"projected_legacy_metadata_without_clean_gist_after_repair\":29},\"index\":{\"total_records\":863,\"noisy_records\":1,\"duplicate_outputs\":1,\"quality_score\":0.580000,\"retrieval_ready\":false,\"risk_level\":\"blocked\"},\"error\":null},\"readiness_warnings\":[\"experience_hygiene: 4 quarantine candidates\"]}",
    );

    assert_eq!(health.experience_hygiene.checked, Some(true));
    assert_eq!(health.experience_hygiene.clean, Some(false));
    assert_eq!(health.experience_hygiene.findings.as_deref(), Some("4"));
    assert_eq!(health.experience_hygiene.watch.as_deref(), Some("0"));
    assert_eq!(
        health.experience_hygiene.quarantine_candidates.as_deref(),
        Some("4")
    );
    assert_eq!(
        health.experience_hygiene.legacy_metadata_lessons.as_deref(),
        Some("860")
    );
    assert_eq!(
        health
            .experience_hygiene
            .repair
            .repairable_legacy_metadata_lessons
            .as_deref(),
        Some("828")
    );
    assert_eq!(
        health
            .experience_hygiene
            .repair
            .repairable_index_records
            .as_deref(),
        Some("1")
    );
    assert_eq!(
        health
            .experience_hygiene
            .repair
            .projected_findings_after_repair
            .as_deref(),
        Some("32")
    );
    assert_eq!(
        health.experience_hygiene.index.total_records.as_deref(),
        Some("863")
    );
    assert_eq!(
        health.experience_hygiene.index.noisy_records.as_deref(),
        Some("1")
    );
    assert_eq!(
        health.experience_hygiene.index.duplicate_outputs.as_deref(),
        Some("1")
    );
    assert_eq!(
        health.experience_hygiene.index.quality_score.as_deref(),
        Some("0.580000")
    );
    assert_eq!(health.experience_hygiene.index.retrieval_ready, Some(false));
    assert_eq!(
        health.experience_hygiene.index.risk_level.as_deref(),
        Some("blocked")
    );
    assert!(health.error.is_none());
    assert!(
        health
            .summary()
            .contains("experience_hygiene_quarantine_candidates=4")
    );
    assert!(
        health
            .summary()
            .contains("repairable_legacy_metadata_lessons=828")
    );
    assert!(health.summary().contains("repairable_index_records=1"));
    assert!(
        health
            .summary()
            .contains("experience_index_retrieval_ready=false")
    );
    assert!(
        health
            .summary()
            .contains("experience_index_risk_level=blocked")
    );
}

#[test]
fn readiness_rejects_dirty_experience_hygiene() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"built-in\",\"experience_hygiene\":{\"checked\":true,\"clean\":false,\"findings\":4,\"quarantine_candidates\":4,\"error\":null}}",
    );

    let error = health.require_ready().unwrap_err();

    assert!(error.contains("experience hygiene failed"));
    assert!(error.contains("quarantine_candidates=4"));
    assert!(error.contains("/v1/experience-hygiene"));
    assert!(!error.contains("quarantine_candidates=0"));
}

#[test]
fn readiness_rejects_repairable_legacy_metadata_without_quarantine_candidates() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"built-in\",\"experience_hygiene\":{\"checked\":true,\"clean\":false,\"findings\":1,\"watch\":1,\"quarantine_candidates\":0,\"legacy_metadata_lessons\":1,\"legacy_metadata_without_clean_gist\":0,\"repair\":{\"repairable_legacy_metadata_lessons\":1,\"projected_findings_after_repair\":0,\"projected_watch_after_repair\":0,\"projected_quarantine_candidates_after_repair\":0,\"projected_legacy_metadata_lessons_after_repair\":0,\"projected_legacy_metadata_without_clean_gist_after_repair\":0},\"error\":null}}",
    );

    let error = health.require_ready().unwrap_err();

    assert!(error.contains("experience_repair"));
    assert!(error.contains("repairable_legacy_metadata_lessons=1"));
    assert!(error.contains("--experience-repair"));
    assert!(error.contains("projected_findings_after_repair=0"));
}

#[test]
fn readiness_rejects_repairable_index_records_without_quarantine_candidates() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"built-in\",\"experience_hygiene\":{\"checked\":true,\"clean\":false,\"findings\":1,\"watch\":1,\"quarantine_candidates\":0,\"legacy_metadata_lessons\":0,\"legacy_metadata_without_clean_gist\":0,\"repair\":{\"repairable_legacy_metadata_lessons\":0,\"repairable_index_records\":1,\"projected_findings_after_repair\":0,\"projected_watch_after_repair\":0,\"projected_quarantine_candidates_after_repair\":0,\"projected_legacy_metadata_lessons_after_repair\":0,\"projected_legacy_metadata_without_clean_gist_after_repair\":0},\"error\":null}}",
    );

    let error = health.require_ready().unwrap_err();

    assert!(error.contains("experience_repair"));
    assert!(error.contains("repairable_index_records=1"));
    assert!(error.contains("--experience-repair"));
    assert!(error.contains("projected_findings_after_repair=0"));
}

#[test]
fn readiness_rejects_blocked_experience_index_without_hygiene_debt() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"built-in\",\"experience_hygiene\":{\"checked\":true,\"clean\":true,\"findings\":0,\"watch\":0,\"quarantine_candidates\":0,\"repair\":{\"repairable_legacy_metadata_lessons\":0,\"repairable_index_records\":0},\"index\":{\"total_records\":42,\"noisy_records\":2,\"duplicate_outputs\":1,\"quality_score\":0.340000,\"retrieval_ready\":false,\"risk_level\":\"blocked\"},\"error\":null}}",
    );

    let error = health.require_ready().unwrap_err();

    assert!(error.contains("experience_index"));
    assert!(error.contains("retrieval_ready=false"));
    assert!(error.contains("risk_level=blocked"));
    assert!(error.contains("quality_score=0.340000"));
    assert!(error.contains("--experience-cleanup-audit"));
    assert!(error.contains("experience_index_noisy_records=2"));
}

#[test]
fn missing_experience_file_does_not_poison_top_level_error() {
    let health = parse_provider_health(
        "{\"ok\":true,\"service\":\"rust-norion\",\"active_engine_requests\":0,\"engine_busy\":false,\"runtime_mode\":\"built-in\",\"experience_hygiene\":{\"checked\":false,\"clean\":null,\"findings\":null,\"quarantine_candidates\":null,\"error\":\"experience_file_missing\"}}",
    );

    assert_eq!(health.experience_hygiene.checked, Some(false));
    assert_eq!(
        health.experience_hygiene.error.as_deref(),
        Some("experience_file_missing")
    );
    assert!(health.error.is_none());
    assert!(health.require_ready().is_ok());
}
