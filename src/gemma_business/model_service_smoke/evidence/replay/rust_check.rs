use super::fields::{
    rust_check_feedback_applied, rust_check_feedback_updates, rust_check_items, rust_check_passed,
};

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::gemma_business::model_service_smoke) struct RustCheckReplayEvidence {
    pub(in crate::gemma_business::model_service_smoke) items: u64,
    pub(in crate::gemma_business::model_service_smoke) passed: u64,
    pub(in crate::gemma_business::model_service_smoke) feedback_updates: u64,
    pub(in crate::gemma_business::model_service_smoke) feedback_applied: u64,
}

impl RustCheckReplayEvidence {
    pub(super) fn from_replay_body(body: &str) -> Self {
        Self {
            items: rust_check_items(body),
            passed: rust_check_passed(body),
            feedback_updates: rust_check_feedback_updates(body),
            feedback_applied: rust_check_feedback_applied(body),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RustCheckReplayEvidence;

    #[test]
    fn rust_check_replay_evidence_reads_replay_feedback_fields() {
        let body = "{\"rust_check_items\":2,\"rust_check_passed\":1,\"rust_check_live_memory_feedback_updates\":3,\"rust_check_live_memory_feedback_applied\":4}";

        let evidence = RustCheckReplayEvidence::from_replay_body(body);

        assert_eq!(evidence.items, 2);
        assert_eq!(evidence.passed, 1);
        assert_eq!(evidence.feedback_updates, 3);
        assert_eq!(evidence.feedback_applied, 4);
    }
}
