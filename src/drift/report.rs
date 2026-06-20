use super::DriftSeverity;

#[derive(Debug, Clone)]
pub struct DriftReport {
    pub severity: DriftSeverity,
    pub allow_memory_write: bool,
    pub allow_runtime_kv_write: bool,
    pub penalize_used_memory: bool,
    pub rollback_adaptive: bool,
    pub notes: Vec<String>,
}

impl DriftReport {
    pub fn summary(&self) -> String {
        format!(
            "severity={} memory_write={} runtime_kv_write={} penalize_used_memory={} rollback_adaptive={} notes={}",
            self.severity.as_str(),
            self.allow_memory_write,
            self.allow_runtime_kv_write,
            self.penalize_used_memory,
            self.rollback_adaptive,
            self.notes.len()
        )
    }
}
