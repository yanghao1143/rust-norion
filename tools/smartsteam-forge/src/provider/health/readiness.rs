use super::ProviderHealth;

impl ProviderHealth {
    pub fn readiness_error(&self) -> Option<String> {
        if !self.ok {
            return Some(format!(
                "后端 /health ok=false (backend ok=false) summary={}",
                self.summary()
            ));
        }
        if let Some(error) = &self.error {
            return Some(format!("后端报告错误 (backend reported error): {error}"));
        }
        if self.is_busy() {
            return Some(format!(
                "后端正在忙 (backend engine is busy): {} summary={}",
                self.busy_detail(),
                self.summary()
            ));
        }
        if self.readiness_ok == Some(false) {
            return Some(format!(
                "后端 readiness 失败 (backend readiness failed): {} summary={}",
                failure_list_or_unknown(&self.readiness_failures),
                self.summary()
            ));
        }
        if let Some(error) = self.experience_hygiene_error() {
            return Some(format!(
                "后端经验库卫生检查失败 (backend experience hygiene failed): {error} summary={}",
                self.summary()
            ));
        }
        if self.requires_gemma_runtime() && self.gemma_runtime_reachable != Some(true) {
            let server = self.gemma_runtime_server.as_deref().unwrap_or("unknown");
            return Some(format!(
                "Gemma runtime 不可达 (Gemma runtime is not reachable) server={server} summary={}",
                self.summary()
            ));
        }
        None
    }

    pub fn require_ready(&self) -> Result<(), String> {
        match self.readiness_error() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub fn safe_device_error(&self) -> Option<String> {
        if self.safe_device_ok == Some(false) {
            return Some(format!(
                "safe-device 未通过：Gemma 12B 当前不是 GPU-first / device 不安全 (backend safe-device failed): {} summary={}",
                failure_list_or_unknown(&self.safe_device_failures),
                self.summary()
            ));
        }
        let warnings = self.readiness_warnings.join("|").to_lowercase();
        let gemma_runtime = self
            .runtime_mode
            .as_deref()
            .map(|runtime| runtime.starts_with("gemma"))
            .unwrap_or(false)
            || self.gemma_runtime_server.is_some();
        let cpu_first_lane = self
            .device_primary_lane
            .as_deref()
            .map(is_cpu_or_disk_lane)
            .unwrap_or(false);
        if warnings.contains("gemma_12b_device")
            || warnings.contains("cpu/disk-first")
            || (gemma_runtime && cpu_first_lane)
        {
            let device = self.device_profile.as_deref().unwrap_or("unknown");
            let lane = self.device_primary_lane.as_deref().unwrap_or("unknown");
            return Some(format!(
                "Gemma 12B 后端不是 GPU-first：device={device} lane={lane}，当前是 CPU/disk-first。请改用 GPU-backed --gemma-runtime-server；只有 tiny CPU fallback 测试才临时关闭 safe-device guard。"
            ));
        }
        None
    }

    pub fn require_safe_device(&self) -> Result<(), String> {
        match self.safe_device_error() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn requires_gemma_runtime(&self) -> bool {
        self.runtime_mode.as_deref() == Some("gemma-http") || self.gemma_runtime_server.is_some()
    }

    fn is_busy(&self) -> bool {
        self.engine_busy == Some(true) || number_is_positive(self.active_engine_requests.as_deref())
    }

    fn busy_detail(&self) -> String {
        if self.active_requests.is_empty() {
            let active = self.active_engine_requests.as_deref().unwrap_or("unknown");
            return format!("active_engine_requests={active}");
        }

        self.active_requests
            .iter()
            .take(3)
            .map(|request| {
                let request_id = request.request_id.as_deref().unwrap_or("unknown");
                let endpoint = request.endpoint.as_deref().unwrap_or("unknown");
                let elapsed = request.elapsed_ms.as_deref().unwrap_or("unknown");
                let prompt = request.prompt_preview.as_deref().unwrap_or("unknown");
                format!(
                    "request_id={request_id} endpoint={endpoint} elapsed_ms={elapsed} prompt_preview=\"{prompt}\""
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn experience_hygiene_error(&self) -> Option<String> {
        if number_is_positive(self.experience_hygiene.quarantine_candidates.as_deref()) {
            let candidates = self
                .experience_hygiene
                .quarantine_candidates
                .as_deref()
                .unwrap_or("unknown");
            let mut message = format!(
                "experience_hygiene quarantine_candidates={candidates}. Run /doctor, inspect /v1/experience-hygiene, then quarantine dirty records before sending prompts."
            );
            if let Some(repair) = self.repairable_legacy_metadata_lessons() {
                message.push_str(&format!(
                    " experience_repair repairable_legacy_metadata_lessons={repair}. After quarantine, dry-run repair with --experience-repair or POST /v1/experience-repair."
                ));
            }
            if let Some(repair) = self.repairable_index_records() {
                message.push_str(&format!(
                    " experience_repair repairable_index_records={repair}. After quarantine, dry-run repair with --experience-repair or POST /v1/experience-repair."
                ));
            }
            return Some(message);
        }

        if let Some(repair) = self.repairable_legacy_metadata_lessons() {
            let projected = self
                .experience_hygiene
                .repair
                .projected_findings_after_repair
                .as_deref()
                .unwrap_or("unknown");
            return Some(format!(
                "experience_repair repairable_legacy_metadata_lessons={repair} projected_findings_after_repair={projected}. Dry-run with --experience-repair or POST /v1/experience-repair before sending prompts."
            ));
        }

        if let Some(repair) = self.repairable_index_records() {
            let projected = self
                .experience_hygiene
                .repair
                .projected_findings_after_repair
                .as_deref()
                .unwrap_or("unknown");
            return Some(format!(
                "experience_repair repairable_index_records={repair} projected_findings_after_repair={projected}. Dry-run with --experience-repair or POST /v1/experience-repair before sending prompts."
            ));
        }

        if self.experience_hygiene.index.retrieval_ready == Some(false) {
            let risk = self
                .experience_hygiene
                .index
                .risk_level
                .as_deref()
                .unwrap_or("unknown");
            let score = self
                .experience_hygiene
                .index
                .quality_score
                .as_deref()
                .unwrap_or("unknown");
            return Some(format!(
                "experience_index retrieval_ready=false risk_level={risk} quality_score={score}. Run --audit or --experience-cleanup-audit before sending prompts."
            ));
        }

        if self.experience_hygiene.clean == Some(false) {
            let findings = self
                .experience_hygiene
                .findings
                .as_deref()
                .unwrap_or("unknown");
            return Some(format!(
                "experience_hygiene findings={findings}. Inspect /v1/experience-hygiene before sending prompts."
            ));
        }

        match (
            self.experience_hygiene.checked,
            self.experience_hygiene.error.as_deref(),
        ) {
            (Some(false), Some("experience_file_missing")) => None,
            (Some(false), Some(error)) => Some(format!("experience_hygiene check failed: {error}")),
            _ => None,
        }
    }

    fn repairable_legacy_metadata_lessons(&self) -> Option<&str> {
        let value = self
            .experience_hygiene
            .repair
            .repairable_legacy_metadata_lessons
            .as_deref()?;
        number_is_positive(Some(value)).then_some(value)
    }

    fn repairable_index_records(&self) -> Option<&str> {
        let value = self
            .experience_hygiene
            .repair
            .repairable_index_records
            .as_deref()?;
        number_is_positive(Some(value)).then_some(value)
    }
}

fn number_is_positive(value: Option<&str>) -> bool {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value > 0)
        .unwrap_or(false)
}

fn is_cpu_or_disk_lane(lane: &str) -> bool {
    matches!(
        lane,
        "cpu-portable" | "cpu-vector" | "disk-backed-streaming"
    )
}

fn failure_list_or_unknown(failures: &[String]) -> String {
    if failures.is_empty() {
        "unknown".to_owned()
    } else {
        failures.join("|")
    }
}
