use crate::json::{
    json_array_field, json_bool_field, json_number_field, json_object_field, json_object_items,
    json_string_array_field, json_string_field,
};

use super::types::{
    BackendActiveRequest, BackendExperienceHygiene, BackendExperienceIndex, BackendHealth,
    BackendLastInference, BackendResult,
};

pub(super) fn parse_backend_health(body: &str) -> BackendHealth {
    BackendHealth {
        ok: json_bool_field(body, "ok").unwrap_or(false),
        service: json_string_field(body, "service"),
        requests_seen: json_number_field(body, "requests_seen"),
        active_engine_requests: json_number_field(body, "active_engine_requests"),
        engine_busy: json_bool_field(body, "engine_busy"),
        runtime_mode: json_string_field(body, "runtime_mode"),
        gemma_runtime_server: json_string_field(body, "gemma_runtime_server"),
        gemma_runtime_reachable: json_bool_field(body, "gemma_runtime_reachable"),
        gemma_runtime_model: json_string_field(body, "gemma_runtime_model"),
        gemma_runtime_context_window: json_number_field(body, "gemma_runtime_context_window"),
        gemma_runtime_train_context_window: json_number_field(
            body,
            "gemma_runtime_train_context_window",
        ),
        gemma_runtime_vocab_size: json_number_field(body, "gemma_runtime_vocab_size"),
        gemma_runtime_metadata_error: json_string_field(body, "gemma_runtime_metadata_error"),
        readiness_ok: json_bool_field(body, "readiness_ok"),
        safe_device_ok: json_bool_field(body, "safe_device_ok"),
        readiness_failures: json_string_array_field(body, "readiness_failures").unwrap_or_default(),
        safe_device_failures: json_string_array_field(body, "safe_device_failures")
            .unwrap_or_default(),
        device_primary_lane: json_string_field(body, "device_primary_lane"),
        device_memory_mode: json_string_field(body, "device_memory_mode"),
        experience_hygiene: parse_backend_experience_hygiene(body),
        active_requests: parse_backend_active_requests(body),
        last_inference: parse_backend_last_inference(body),
        error: json_string_field(body, "error"),
    }
}

fn parse_backend_experience_hygiene(body: &str) -> Option<BackendExperienceHygiene> {
    let hygiene = json_object_field(body, "experience_hygiene")?;
    let repair = json_object_field(hygiene, "repair");
    let index = json_object_field(hygiene, "index");
    Some(BackendExperienceHygiene {
        experience_file: json_string_field(hygiene, "experience_file"),
        checked: json_bool_field(hygiene, "checked"),
        clean: json_bool_field(hygiene, "clean"),
        findings: json_number_field(hygiene, "findings"),
        quarantine_candidates: json_number_field(hygiene, "quarantine_candidates"),
        repairable_legacy_metadata_lessons: repair
            .and_then(|repair| json_number_field(repair, "repairable_legacy_metadata_lessons")),
        repairable_index_records: repair
            .and_then(|repair| json_number_field(repair, "repairable_index_records")),
        index: index.map(parse_backend_experience_index),
    })
}

fn parse_backend_experience_index(index: &str) -> BackendExperienceIndex {
    BackendExperienceIndex {
        total_records: json_number_field(index, "total_records"),
        noisy_records: json_number_field(index, "noisy_records"),
        duplicate_outputs: json_number_field(index, "duplicate_outputs"),
        quality_score: json_number_field(index, "quality_score"),
        retrieval_ready: json_bool_field(index, "retrieval_ready"),
        risk_level: json_string_field(index, "risk_level"),
    }
}

fn parse_backend_active_requests(body: &str) -> Vec<BackendActiveRequest> {
    let Some(active_requests) = json_array_field(body, "active_requests") else {
        return Vec::new();
    };
    json_object_items(active_requests)
        .into_iter()
        .map(|item| BackendActiveRequest {
            request_id: json_number_field(item, "request_id"),
            endpoint: json_string_field(item, "endpoint"),
            elapsed_ms: json_number_field(item, "elapsed_ms"),
            prompt_preview: json_string_field(item, "prompt_preview"),
        })
        .collect()
}

fn parse_backend_last_inference(body: &str) -> Option<BackendLastInference> {
    let last = json_object_field(body, "last_inference")?;
    Some(BackendLastInference {
        request_id: json_number_field(last, "request_id"),
        endpoint: json_string_field(last, "endpoint"),
        elapsed_ms: json_number_field(last, "elapsed_ms"),
        runtime_model: json_string_field(last, "runtime_model"),
        runtime_token_count: json_number_field(last, "runtime_token_count"),
        quality: json_number_field(last, "quality"),
        process_reward: json_number_field(last, "process_reward"),
        action: json_string_field(last, "action"),
        error: json_string_field(last, "error"),
    })
}

pub(super) fn parse_backend_result(body: &str) -> Result<BackendResult, String> {
    let ok = json_bool_field(body, "ok").unwrap_or(false);
    let error = json_string_field(body, "error");
    if !ok {
        return Ok(BackendResult {
            ok,
            answer: String::new(),
            runtime_model: None,
            elapsed_ms: None,
            business_cycle_passed: None,
            feedback_applied: None,
            rust_check_passed: None,
            self_improve_passed: None,
            error,
        });
    }

    let answer = json_string_field(body, "answer")
        .ok_or_else(|| "backend response missing answer".to_owned())?;
    Ok(BackendResult {
        ok,
        answer,
        runtime_model: json_string_field(body, "runtime_model"),
        elapsed_ms: json_number_field(body, "elapsed_ms"),
        business_cycle_passed: json_bool_field(body, "passed"),
        feedback_applied: json_number_field(body, "feedback_applied"),
        rust_check_passed: json_bool_field(body, "rust_check_passed"),
        self_improve_passed: json_bool_field(body, "self_improve_passed"),
        error,
    })
}
