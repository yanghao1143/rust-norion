use super::ProviderHealth;

impl ProviderHealth {
    pub fn summary(&self) -> String {
        let reachable = self
            .gemma_runtime_reachable
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        let busy = self
            .engine_busy
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        let runtime = self.runtime_mode.as_deref().unwrap_or("unknown");
        let active = self.active_engine_requests.as_deref().unwrap_or("unknown");
        let service = self.service.as_deref().unwrap_or("unknown");
        let mut summary = format!(
            "service={service} ok={} runtime={runtime} gemma_reachable={reachable} busy={busy} active={active}",
            self.ok
        );
        if let Some(readiness_ok) = self.readiness_ok {
            summary.push_str(&format!(" readiness_ok={readiness_ok}"));
        }
        if let Some(safe_device_ok) = self.safe_device_ok {
            summary.push_str(&format!(" safe_device_ok={safe_device_ok}"));
        }
        if let Some(device) = &self.device_profile {
            let lane = self.device_primary_lane.as_deref().unwrap_or("unknown");
            let memory = self.device_memory_mode.as_deref().unwrap_or("unknown");
            summary.push_str(&format!(" device={device} lane={lane} memory={memory}"));
        }
        if let Some(accelerators) = &self.device_accelerators {
            summary.push_str(&format!(" accelerators={accelerators}"));
        }
        if let Some(pressure) = &self.device_pressure {
            summary.push_str(&format!(" device_pressure={pressure}"));
        }
        if let Some(plan) = &self.device_plan_summary {
            summary.push_str(&format!(" device_plan=\"{plan}\""));
        }
        if let Some(probe) = &self.device_probe_summary {
            summary.push_str(&format!(" device_probe=\"{probe}\""));
        }
        if self.experience_hygiene.checked.is_some()
            || self.experience_hygiene.clean.is_some()
            || self.experience_hygiene.quarantine_candidates.is_some()
            || self
                .experience_hygiene
                .repair
                .repairable_legacy_metadata_lessons
                .is_some()
            || self
                .experience_hygiene
                .repair
                .repairable_index_records
                .is_some()
            || self.experience_hygiene.index.retrieval_ready.is_some()
            || self.experience_hygiene.index.risk_level.is_some()
            || self.experience_hygiene.error.is_some()
        {
            let checked = self
                .experience_hygiene
                .checked
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_owned());
            let clean = self
                .experience_hygiene
                .clean
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_owned());
            let candidates = self
                .experience_hygiene
                .quarantine_candidates
                .as_deref()
                .unwrap_or("unknown");
            summary.push_str(&format!(
                " experience_hygiene_checked={checked} experience_hygiene_clean={clean} experience_hygiene_quarantine_candidates={candidates}"
            ));
            if let Some(findings) = &self.experience_hygiene.findings {
                summary.push_str(&format!(" experience_hygiene_findings={findings}"));
            }
            if let Some(watch) = &self.experience_hygiene.watch {
                summary.push_str(&format!(" experience_hygiene_watch={watch}"));
            }
            if let Some(legacy) = &self.experience_hygiene.legacy_metadata_lessons {
                summary.push_str(&format!(" legacy_metadata_lessons={legacy}"));
            }
            if let Some(missing) = &self.experience_hygiene.legacy_metadata_without_clean_gist {
                summary.push_str(&format!(" legacy_metadata_without_clean_gist={missing}"));
            }
            if let Some(repairable) = &self
                .experience_hygiene
                .repair
                .repairable_legacy_metadata_lessons
            {
                summary.push_str(&format!(" repairable_legacy_metadata_lessons={repairable}"));
            }
            if let Some(repairable) = &self.experience_hygiene.repair.repairable_index_records {
                summary.push_str(&format!(" repairable_index_records={repairable}"));
            }
            if let Some(projected) = &self
                .experience_hygiene
                .repair
                .projected_findings_after_repair
            {
                summary.push_str(&format!(" projected_findings_after_repair={projected}"));
            }
            if let Some(retrieval_ready) = self.experience_hygiene.index.retrieval_ready {
                summary.push_str(&format!(
                    " experience_index_retrieval_ready={retrieval_ready}"
                ));
            }
            if let Some(risk) = &self.experience_hygiene.index.risk_level {
                summary.push_str(&format!(" experience_index_risk_level={risk}"));
            }
            if let Some(score) = &self.experience_hygiene.index.quality_score {
                summary.push_str(&format!(" experience_index_quality_score={score}"));
            }
            if let Some(noisy) = &self.experience_hygiene.index.noisy_records {
                summary.push_str(&format!(" experience_index_noisy_records={noisy}"));
            }
            if let Some(duplicates) = &self.experience_hygiene.index.duplicate_outputs {
                summary.push_str(&format!(" experience_index_duplicate_outputs={duplicates}"));
            }
            if let Some(error) = &self.experience_hygiene.error {
                summary.push_str(&format!(" experience_hygiene_error={error}"));
            }
        }
        if let Some(active_request) = self.active_requests.first() {
            let request_id = active_request.request_id.as_deref().unwrap_or("unknown");
            let endpoint = active_request.endpoint.as_deref().unwrap_or("unknown");
            let elapsed = active_request.elapsed_ms.as_deref().unwrap_or("unknown");
            let prompt = active_request
                .prompt_preview
                .as_deref()
                .unwrap_or("unknown");
            summary.push_str(&format!(
                " active_request=#{request_id}:{endpoint}:{elapsed}ms active_prompt=\"{prompt}\""
            ));
        }
        if !self.readiness_warnings.is_empty() {
            summary.push_str(&format!(" warnings={}", self.readiness_warnings.join("|")));
        }
        if let Some(last) = &self.last_inference {
            let endpoint = last.endpoint.as_deref().unwrap_or("unknown");
            let elapsed = last.elapsed_ms.as_deref().unwrap_or("unknown");
            let tokens = last.runtime_token_count.as_deref().unwrap_or("unknown");
            summary.push_str(&format!(
                " last_endpoint={endpoint} last_elapsed_ms={elapsed} last_tokens={tokens}"
            ));
        }
        if let Some(error) = &self.error {
            summary.push_str(&format!(" error={error}"));
        }
        summary
    }
}
