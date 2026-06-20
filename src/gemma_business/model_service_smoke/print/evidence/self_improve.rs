use crate::gemma_business::model_service_smoke::print::ModelServiceSmokeReport;

pub(in crate::gemma_business::model_service_smoke::print) fn print_self_improve_evidence(
    report: &ModelServiceSmokeReport<'_>,
) {
    println!(
        "gemma_model_service_smoke_self_improve: passed={} replay_applied={} live_memory_feedback_updates={} live_memory_feedback_applied={} live_evolution_items={} live_evolution_online_reward_feedbacks={}",
        report.replay.self_improvement_replay_evidence(),
        report.replay.applied,
        report.replay.live_memory_feedback_updates,
        report.replay.live_memory_feedback_applied,
        report.replay.live_evolution_items,
        report.replay.live_evolution_online_reward_feedbacks
    );
}
