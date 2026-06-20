use crate::backend::{
    BackendActiveRequest, BackendExperienceHygiene, BackendExperienceIndex, BackendHealth,
    BackendLastInference,
};

pub(super) fn backend_health_json(health: &BackendHealth) -> String {
    format!(
        "{{\"ok\":{},\"service\":{},\"requests_seen\":{},\"active_engine_requests\":{},\"engine_busy\":{},\"active_requests\":{},\"runtime_mode\":{},\"gemma_runtime_server\":{},\"gemma_runtime_reachable\":{},\"gemma_runtime_model\":{},\"gemma_runtime_context_window\":{},\"gemma_runtime_train_context_window\":{},\"gemma_runtime_vocab_size\":{},\"gemma_runtime_metadata_error\":{},\"readiness_ok\":{},\"safe_device_ok\":{},\"readiness_failures\":{},\"safe_device_failures\":{},\"device_primary_lane\":{},\"device_memory_mode\":{},\"experience_hygiene\":{},\"last_inference\":{},\"error\":{}}}",
        health.ok,
        option_json_string(health.service.as_deref()),
        option_json_number(health.requests_seen.as_deref()),
        option_json_number(health.active_engine_requests.as_deref()),
        option_json_bool(health.engine_busy),
        active_requests_json(&health.active_requests),
        option_json_string(health.runtime_mode.as_deref()),
        option_json_string(health.gemma_runtime_server.as_deref()),
        option_json_bool(health.gemma_runtime_reachable),
        option_json_string(health.gemma_runtime_model.as_deref()),
        option_json_number(health.gemma_runtime_context_window.as_deref()),
        option_json_number(health.gemma_runtime_train_context_window.as_deref()),
        option_json_number(health.gemma_runtime_vocab_size.as_deref()),
        option_json_string(health.gemma_runtime_metadata_error.as_deref()),
        option_json_bool(health.readiness_ok),
        option_json_bool(health.safe_device_ok),
        json_string_array(&health.readiness_failures),
        json_string_array(&health.safe_device_failures),
        option_json_string(health.device_primary_lane.as_deref()),
        option_json_string(health.device_memory_mode.as_deref()),
        experience_hygiene_json(health.experience_hygiene.as_ref()),
        last_inference_json(health.last_inference.as_ref()),
        option_json_string(health.error.as_deref())
    )
}

fn active_requests_json(active_requests: &[BackendActiveRequest]) -> String {
    let items = active_requests
        .iter()
        .map(|request| {
            format!(
                "{{\"request_id\":{},\"endpoint\":{},\"elapsed_ms\":{},\"prompt_preview\":{}}}",
                option_json_number(request.request_id.as_deref()),
                option_json_string(request.endpoint.as_deref()),
                option_json_number(request.elapsed_ms.as_deref()),
                option_json_string(request.prompt_preview.as_deref())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn last_inference_json(last: Option<&BackendLastInference>) -> String {
    let Some(last) = last else {
        return "null".to_owned();
    };
    format!(
        "{{\"request_id\":{},\"endpoint\":{},\"elapsed_ms\":{},\"runtime_model\":{},\"runtime_token_count\":{},\"quality\":{},\"process_reward\":{},\"action\":{},\"error\":{}}}",
        option_json_number(last.request_id.as_deref()),
        option_json_string(last.endpoint.as_deref()),
        option_json_number(last.elapsed_ms.as_deref()),
        option_json_string(last.runtime_model.as_deref()),
        option_json_number(last.runtime_token_count.as_deref()),
        option_json_number(last.quality.as_deref()),
        option_json_number(last.process_reward.as_deref()),
        option_json_string(last.action.as_deref()),
        option_json_string(last.error.as_deref())
    )
}

fn option_json_string(value: Option<&str>) -> String {
    value
        .map(crate::json::json_string)
        .unwrap_or_else(|| "null".to_owned())
}

fn option_json_number(value: Option<&str>) -> String {
    value.unwrap_or("null").to_owned()
}

fn option_json_bool(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn json_string_array(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| crate::json::json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn experience_hygiene_json(hygiene: Option<&BackendExperienceHygiene>) -> String {
    let Some(hygiene) = hygiene else {
        return "null".to_owned();
    };
    format!(
        "{{\"experience_file\":{},\"checked\":{},\"clean\":{},\"findings\":{},\"quarantine_candidates\":{},\"repairable_legacy_metadata_lessons\":{},\"repairable_index_records\":{},\"index\":{}}}",
        option_json_string(hygiene.experience_file.as_deref()),
        option_json_bool(hygiene.checked),
        option_json_bool(hygiene.clean),
        option_json_number(hygiene.findings.as_deref()),
        option_json_number(hygiene.quarantine_candidates.as_deref()),
        option_json_number(hygiene.repairable_legacy_metadata_lessons.as_deref()),
        option_json_number(hygiene.repairable_index_records.as_deref()),
        experience_index_json(hygiene.index.as_ref())
    )
}

fn experience_index_json(index: Option<&BackendExperienceIndex>) -> String {
    let Some(index) = index else {
        return "null".to_owned();
    };
    format!(
        "{{\"total_records\":{},\"noisy_records\":{},\"duplicate_outputs\":{},\"quality_score\":{},\"retrieval_ready\":{},\"risk_level\":{}}}",
        option_json_number(index.total_records.as_deref()),
        option_json_number(index.noisy_records.as_deref()),
        option_json_number(index.duplicate_outputs.as_deref()),
        option_json_number(index.quality_score.as_deref()),
        option_json_bool(index.retrieval_ready),
        option_json_string(index.risk_level.as_deref())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_backend_health_detail_fields() {
        let health = BackendHealth {
            ok: true,
            service: Some("rust-norion".to_owned()),
            requests_seen: Some("3".to_owned()),
            active_engine_requests: Some("1".to_owned()),
            engine_busy: Some(true),
            runtime_mode: Some("built-in".to_owned()),
            gemma_runtime_server: None,
            gemma_runtime_reachable: None,
            gemma_runtime_model: Some("gemma-test.gguf".to_owned()),
            gemma_runtime_context_window: Some("262144".to_owned()),
            gemma_runtime_train_context_window: Some("262144".to_owned()),
            gemma_runtime_vocab_size: Some("262144".to_owned()),
            gemma_runtime_metadata_error: None,
            readiness_ok: Some(true),
            safe_device_ok: Some(true),
            readiness_failures: Vec::new(),
            safe_device_failures: Vec::new(),
            device_primary_lane: Some("discrete-gpu".to_owned()),
            device_memory_mode: Some("gpu-resident".to_owned()),
            experience_hygiene: Some(BackendExperienceHygiene {
                experience_file: Some("D:\\state\\experience.ndkv".to_owned()),
                checked: Some(true),
                clean: Some(true),
                findings: Some("0".to_owned()),
                quarantine_candidates: Some("0".to_owned()),
                repairable_legacy_metadata_lessons: Some("0".to_owned()),
                repairable_index_records: Some("0".to_owned()),
                index: Some(BackendExperienceIndex {
                    total_records: Some("12".to_owned()),
                    noisy_records: Some("1".to_owned()),
                    duplicate_outputs: Some("1".to_owned()),
                    quality_score: Some("0.58".to_owned()),
                    retrieval_ready: Some(true),
                    risk_level: Some("degraded".to_owned()),
                }),
            }),
            active_requests: vec![BackendActiveRequest {
                request_id: Some("42".to_owned()),
                endpoint: Some("chat-stream".to_owned()),
                elapsed_ms: Some("123".to_owned()),
                prompt_preview: Some("hello".to_owned()),
            }],
            last_inference: Some(BackendLastInference {
                request_id: Some("7".to_owned()),
                endpoint: Some("chat".to_owned()),
                elapsed_ms: Some("9".to_owned()),
                runtime_model: Some("gemma".to_owned()),
                runtime_token_count: Some("19".to_owned()),
                quality: Some("0.8".to_owned()),
                process_reward: Some("0.7".to_owned()),
                action: Some("reinforce".to_owned()),
                error: None,
            }),
            error: None,
        };

        let body = backend_health_json(&health);

        assert!(body.contains(r#""runtime_mode":"built-in""#));
        assert!(body.contains(r#""gemma_runtime_context_window":262144"#));
        assert!(body.contains(r#""request_id":42"#));
        assert!(body.contains(r#""experience_file":"D:\\state\\experience.ndkv""#));
        assert!(body.contains(r#""repairable_index_records":0"#));
        assert!(body.contains(r#""risk_level":"degraded""#));
        assert!(body.contains(r#""runtime_token_count":19"#));
    }
}
