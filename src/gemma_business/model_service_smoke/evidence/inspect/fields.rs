use crate::gemma_business::model_service_smoke::evidence::field;

pub(super) fn runtime_tokens(body: &str) -> u64 {
    field(body, "runtime_tokens")
}

pub(super) fn evolution_external_feedbacks(body: &str) -> u64 {
    field(body, "evolution_external_feedbacks")
}

pub(super) fn evolution_external_feedback_memory_updates(body: &str) -> u64 {
    field(body, "evolution_external_feedback_memory_updates")
}

pub(super) fn rust_check_passed(body: &str) -> u64 {
    field(body, "rust_check_passed")
}

pub(super) fn rust_check_experiences(body: &str) -> u64 {
    field(body, "rust_check_experiences")
}

pub(super) fn evolution_replay_rust_check_items(body: &str) -> u64 {
    field(body, "evolution_replay_rust_check_items")
}

pub(super) fn evolution_replay_rust_check_passed(body: &str) -> u64 {
    field(body, "evolution_replay_rust_check_passed")
}

pub(super) fn evolution_replay_runs(body: &str) -> u64 {
    field(body, "evolution_replay_runs")
}

pub(super) fn evolution_replay_items(body: &str) -> u64 {
    field(body, "evolution_replay_items")
}
