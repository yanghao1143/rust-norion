#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemmaBusinessCycleSmokeReportGate {
    pub passed: bool,
    pub schema: Option<String>,
    pub case_count: u64,
    pub passed_cases: u64,
    pub runtime_token_count: u64,
    pub feedback_applied: u64,
    pub rust_check_feedback_applied: u64,
    pub external_feedbacks: u64,
    pub feedback_memory_updates: u64,
    pub replay_rust_check_passed: u64,
    pub replay_live_memory_feedback_applied: u64,
    pub replay_live_evolution_items: u64,
    pub checked_trace_lines: u64,
    pub failures: Vec<String>,
}
