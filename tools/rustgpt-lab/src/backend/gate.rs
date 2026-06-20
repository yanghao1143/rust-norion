use super::types::BackendHealth;

pub(crate) fn backend_prompt_block_reason(health: &BackendHealth) -> Option<String> {
    if !health.ok {
        return Some(format!(
            "backend unavailable{}",
            health
                .error
                .as_deref()
                .map(|error| format!(": {error}"))
                .unwrap_or_default()
        ));
    }

    if health.engine_busy == Some(true) {
        return Some(format!(
            "backend engine is busy; wait for the active Gemma request to finish{}",
            active_request_summary(health)
                .map(|summary| format!(" ({summary})"))
                .unwrap_or_default()
        ));
    }

    let requires_gemma = health.runtime_mode.as_deref() == Some("gemma-http")
        && health.gemma_runtime_server.is_some();
    if requires_gemma && health.gemma_runtime_reachable == Some(false) {
        return Some(
            "Gemma runtime is not reachable; wait for 127.0.0.1:8686 to come back".to_owned(),
        );
    }

    if health.readiness_ok == Some(false) {
        return Some(format!(
            "backend readiness failed{}",
            failure_suffix(&health.readiness_failures)
        ));
    }

    if health.safe_device_ok == Some(false) {
        return Some(format!(
            "safe-device gate failed{}",
            failure_suffix(&health.safe_device_failures)
        ));
    }

    if let Some(reason) = experience_hygiene_block_reason(health) {
        return Some(reason);
    }

    None
}

fn active_request_summary(health: &BackendHealth) -> Option<String> {
    let active = health.active_requests.first()?;
    let mut parts = Vec::new();
    if let Some(request_id) = active.request_id.as_deref() {
        parts.push(format!("#{request_id}"));
    }
    if let Some(endpoint) = active.endpoint.as_deref() {
        parts.push(endpoint.to_owned());
    }
    if let Some(elapsed_ms) = active.elapsed_ms.as_deref() {
        parts.push(format!("{elapsed_ms}ms"));
    }
    if let Some(prompt) = active
        .prompt_preview
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("prompt=\"{}\"", preview_text(prompt, 80)));
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

fn failure_suffix(failures: &[String]) -> String {
    if failures.is_empty() {
        String::new()
    } else {
        format!(": {}", failures.join(", "))
    }
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let text = if normalized.is_empty() {
        text.trim().to_owned()
    } else {
        normalized
    };
    if text.chars().count() <= max_chars {
        return text;
    }
    let keep = max_chars.saturating_sub(3);
    let mut out = text.chars().take(keep).collect::<String>();
    out.push_str("...");
    out
}

fn experience_hygiene_block_reason(health: &BackendHealth) -> Option<String> {
    let hygiene = health.experience_hygiene.as_ref()?;
    if let Some(count) = positive_number(hygiene.quarantine_candidates.as_deref()) {
        return Some(format!(
            "backend experience hygiene failed: quarantine_candidates={count}; run /status or status-gemma-lab.cmd, then quarantine dirty records before chatting"
        ));
    }
    if let Some(count) = positive_number(hygiene.repairable_legacy_metadata_lessons.as_deref()) {
        return Some(format!(
            "backend experience repair required: repairable_legacy_metadata_lessons={count}; dry-run repair before chatting"
        ));
    }
    if let Some(count) = positive_number(hygiene.repairable_index_records.as_deref()) {
        return Some(format!(
            "backend experience repair required: repairable_index_records={count}; dry-run repair before chatting"
        ));
    }
    if let Some(index) = &hygiene.index
        && (index.retrieval_ready == Some(false) || index.risk_level.as_deref() == Some("blocked"))
    {
        let risk = index.risk_level.as_deref().unwrap_or("unknown");
        let score = index.quality_score.as_deref().unwrap_or("unknown");
        return Some(format!(
            "backend experience index blocked: risk_level={risk} quality_score={score}; run cleanup audit before chatting"
        ));
    }
    if hygiene.clean == Some(false) {
        let findings = hygiene.findings.as_deref().unwrap_or("unknown");
        return Some(format!(
            "backend experience hygiene failed: clean=false findings={findings}; inspect hygiene before chatting"
        ));
    }
    None
}

fn positive_number(value: Option<&str>) -> Option<&str> {
    let value = value?;
    value
        .parse::<u64>()
        .ok()
        .filter(|number| *number > 0)
        .map(|_| value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{
        BackendActiveRequest, BackendExperienceHygiene, BackendExperienceIndex, BackendHealth,
    };

    #[test]
    fn busy_health_blocks_send_with_active_request_summary() {
        let health = BackendHealth {
            ok: true,
            engine_busy: Some(true),
            active_requests: vec![BackendActiveRequest {
                request_id: Some("42".to_owned()),
                endpoint: Some("chat-stream".to_owned()),
                elapsed_ms: Some("1234".to_owned()),
                prompt_preview: Some("联调 prompt".to_owned()),
            }],
            ..backend_health_ok()
        };

        let reason = backend_prompt_block_reason(&health).unwrap();

        assert!(reason.contains("backend engine is busy"));
        assert!(reason.contains("#42 chat-stream 1234ms"));
    }

    #[test]
    fn readiness_and_safe_device_gates_block_send() {
        let readiness = BackendHealth {
            readiness_ok: Some(false),
            readiness_failures: vec!["experience_hygiene".to_owned()],
            ..backend_health_ok()
        };
        let safe_device = BackendHealth {
            safe_device_ok: Some(false),
            safe_device_failures: vec!["cpu-first".to_owned()],
            ..backend_health_ok()
        };

        assert_eq!(
            backend_prompt_block_reason(&readiness).as_deref(),
            Some("backend readiness failed: experience_hygiene")
        );
        assert_eq!(
            backend_prompt_block_reason(&safe_device).as_deref(),
            Some("safe-device gate failed: cpu-first")
        );
    }

    #[test]
    fn gemma_runtime_reachability_blocks_send() {
        let health = BackendHealth {
            runtime_mode: Some("gemma-http".to_owned()),
            gemma_runtime_server: Some("http://127.0.0.1:8686".to_owned()),
            gemma_runtime_reachable: Some(false),
            ..backend_health_ok()
        };

        let reason = backend_prompt_block_reason(&health).unwrap();

        assert!(reason.contains("Gemma runtime is not reachable"));
    }

    #[test]
    fn experience_hygiene_debt_blocks_send_even_when_readiness_is_ok() {
        let quarantine = BackendHealth {
            experience_hygiene: Some(BackendExperienceHygiene {
                quarantine_candidates: Some("4".to_owned()),
                ..clean_hygiene()
            }),
            ..backend_health_ok()
        };
        let repairable_index = BackendHealth {
            experience_hygiene: Some(BackendExperienceHygiene {
                clean: Some(false),
                repairable_index_records: Some("1".to_owned()),
                ..clean_hygiene()
            }),
            ..backend_health_ok()
        };
        let dirty = BackendHealth {
            experience_hygiene: Some(BackendExperienceHygiene {
                clean: Some(false),
                findings: Some("2".to_owned()),
                ..clean_hygiene()
            }),
            ..backend_health_ok()
        };

        assert_eq!(
            backend_prompt_block_reason(&quarantine).as_deref(),
            Some(
                "backend experience hygiene failed: quarantine_candidates=4; run /status or status-gemma-lab.cmd, then quarantine dirty records before chatting"
            )
        );
        assert_eq!(
            backend_prompt_block_reason(&repairable_index).as_deref(),
            Some(
                "backend experience repair required: repairable_index_records=1; dry-run repair before chatting"
            )
        );
        assert_eq!(
            backend_prompt_block_reason(&dirty).as_deref(),
            Some(
                "backend experience hygiene failed: clean=false findings=2; inspect hygiene before chatting"
            )
        );
    }

    #[test]
    fn blocked_experience_index_blocks_send_even_when_hygiene_is_clean() {
        let health = BackendHealth {
            experience_hygiene: Some(BackendExperienceHygiene {
                index: Some(BackendExperienceIndex {
                    retrieval_ready: Some(false),
                    risk_level: Some("blocked".to_owned()),
                    quality_score: Some("0.340000".to_owned()),
                    ..clean_index()
                }),
                ..clean_hygiene()
            }),
            ..backend_health_ok()
        };

        assert_eq!(
            backend_prompt_block_reason(&health).as_deref(),
            Some(
                "backend experience index blocked: risk_level=blocked quality_score=0.340000; run cleanup audit before chatting"
            )
        );
    }

    #[test]
    fn blocked_experience_index_risk_blocks_send_even_when_retrieval_ready_is_unknown() {
        let health = BackendHealth {
            experience_hygiene: Some(BackendExperienceHygiene {
                index: Some(BackendExperienceIndex {
                    retrieval_ready: None,
                    risk_level: Some("blocked".to_owned()),
                    quality_score: Some("0.410000".to_owned()),
                    ..clean_index()
                }),
                ..clean_hygiene()
            }),
            ..backend_health_ok()
        };

        assert_eq!(
            backend_prompt_block_reason(&health).as_deref(),
            Some(
                "backend experience index blocked: risk_level=blocked quality_score=0.410000; run cleanup audit before chatting"
            )
        );
    }

    fn backend_health_ok() -> BackendHealth {
        BackendHealth {
            ok: true,
            service: Some("rust-norion".to_owned()),
            requests_seen: Some("0".to_owned()),
            active_engine_requests: Some("0".to_owned()),
            engine_busy: Some(false),
            runtime_mode: Some("built-in".to_owned()),
            gemma_runtime_server: None,
            gemma_runtime_reachable: None,
            gemma_runtime_model: None,
            gemma_runtime_context_window: None,
            gemma_runtime_train_context_window: None,
            gemma_runtime_vocab_size: None,
            gemma_runtime_metadata_error: None,
            readiness_ok: Some(true),
            safe_device_ok: Some(true),
            readiness_failures: Vec::new(),
            safe_device_failures: Vec::new(),
            device_primary_lane: None,
            device_memory_mode: None,
            experience_hygiene: None,
            active_requests: Vec::new(),
            last_inference: None,
            error: None,
        }
    }

    fn clean_hygiene() -> BackendExperienceHygiene {
        BackendExperienceHygiene {
            experience_file: Some("test-experience.ndkv".to_owned()),
            checked: Some(true),
            clean: Some(true),
            findings: Some("0".to_owned()),
            quarantine_candidates: Some("0".to_owned()),
            repairable_legacy_metadata_lessons: Some("0".to_owned()),
            repairable_index_records: Some("0".to_owned()),
            index: Some(clean_index()),
        }
    }

    fn clean_index() -> BackendExperienceIndex {
        BackendExperienceIndex {
            total_records: Some("0".to_owned()),
            noisy_records: Some("0".to_owned()),
            duplicate_outputs: Some("0".to_owned()),
            quality_score: Some("1.000000".to_owned()),
            retrieval_ready: Some(true),
            risk_level: Some("clean".to_owned()),
        }
    }
}
