use super::fields::{
    evolution_replay_items, evolution_replay_runs, evolution_replay_rust_check_items,
    evolution_replay_rust_check_passed,
};

pub(super) struct InspectReplayEvidence {
    pub(super) rust_check_items: u64,
    pub(super) rust_check_passed: u64,
    pub(super) runs: u64,
    pub(super) items: u64,
}

impl InspectReplayEvidence {
    pub(super) fn from_body(body: &str) -> Self {
        Self {
            rust_check_items: evolution_replay_rust_check_items(body),
            rust_check_passed: evolution_replay_rust_check_passed(body),
            runs: evolution_replay_runs(body),
            items: evolution_replay_items(body),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InspectReplayEvidence;

    #[test]
    fn inspect_replay_evidence_reads_replay_counters() {
        let body = "{\"evolution_replay_rust_check_items\":2,\"evolution_replay_rust_check_passed\":1,\"evolution_replay_runs\":3,\"evolution_replay_items\":4}";

        let evidence = InspectReplayEvidence::from_body(body);

        assert_eq!(evidence.rust_check_items, 2);
        assert_eq!(evidence.rust_check_passed, 1);
        assert_eq!(evidence.runs, 3);
        assert_eq!(evidence.items, 4);
    }
}
