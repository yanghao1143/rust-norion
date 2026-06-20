#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInspectionGateReport {
    pub passed: bool,
    pub failures: Vec<String>,
}

impl StateInspectionGateReport {
    pub fn passed(&self) -> bool {
        self.passed
    }

    pub fn summary_line(&self) -> String {
        format!(
            "state_inspection_gate: passed={} failures={}",
            self.passed,
            self.failures.len()
        )
    }
}
