use rust_norion::MemoryUpdateReport;

use super::super::super::json::{
    option_u64_service_json, service_memory_update_array, service_u64_array,
};
use super::super::super::types::ModelServiceBusinessCycleReport;
use super::super::update_stats::{
    memory_update_missing_count, memory_update_removed_count, memory_update_strength_delta,
};

pub(super) fn business_cycle_feedback_json(
    report: &ModelServiceBusinessCycleReport,
    applied: usize,
) -> String {
    feedback_json(FeedbackJsonInput {
        action: report.feedback_request.action.as_str(),
        amount: report.feedback_request.amount,
        experience_id: report.feedback_request.experience_id,
        memory_ids: &report.feedback_memory_ids,
        applied,
        updates: &report.feedback_updates,
    })
}

fn feedback_json(input: FeedbackJsonInput<'_>) -> String {
    format!(
        "{{\"action\":\"{}\",\"amount\":{:.6},\"experience_id\":{},\"memory_ids\":{},\"applied\":{},\"missing\":{},\"removed\":{},\"strength_delta\":{:.6},\"updates\":{}}}",
        input.action,
        input.amount,
        option_u64_service_json(input.experience_id),
        service_u64_array(input.memory_ids),
        input.applied,
        memory_update_missing_count(input.updates),
        memory_update_removed_count(input.updates),
        memory_update_strength_delta(input.updates),
        service_memory_update_array(input.updates)
    )
}

struct FeedbackJsonInput<'a> {
    action: &'a str,
    amount: f32,
    experience_id: Option<u64>,
    memory_ids: &'a [u64],
    applied: usize,
    updates: &'a [MemoryUpdateReport],
}

#[cfg(test)]
mod tests {
    use rust_norion::{MemoryUpdateAction, MemoryUpdateReport};

    use super::*;

    #[test]
    fn feedback_json_renders_memory_update_evidence() {
        let updates = [
            MemoryUpdateReport::applied(11, MemoryUpdateAction::Reinforce, 0.5, 0.25, 0.75, false),
            MemoryUpdateReport::applied(12, MemoryUpdateAction::Reinforce, 0.5, 0.2, 0.0, true),
            MemoryUpdateReport::missing(13, MemoryUpdateAction::Reinforce, 0.5),
        ];

        let json = feedback_json(FeedbackJsonInput {
            action: "reinforce",
            amount: 0.5,
            experience_id: Some(99),
            memory_ids: &[11, 12, 13],
            applied: 2,
            updates: &updates,
        });

        assert!(json.contains("\"action\":\"reinforce\""));
        assert!(json.contains("\"amount\":0.500000"));
        assert!(json.contains("\"experience_id\":99"));
        assert!(json.contains("\"memory_ids\":[11,12,13]"));
        assert!(json.contains("\"applied\":2"));
        assert!(json.contains("\"missing\":1"));
        assert!(json.contains("\"removed\":1"));
        assert!(json.contains("\"strength_delta\":0.700000"));
        assert!(json.contains("\"updates\":["));
    }

    #[test]
    fn feedback_json_renders_missing_experience_id_as_null() {
        let json = feedback_json(FeedbackJsonInput {
            action: "penalize",
            amount: 0.25,
            experience_id: None,
            memory_ids: &[],
            applied: 0,
            updates: &[],
        });

        assert!(json.contains("\"action\":\"penalize\""));
        assert!(json.contains("\"experience_id\":null"));
        assert!(json.contains("\"memory_ids\":[]"));
        assert!(json.contains("\"applied\":0"));
        assert!(json.contains("\"missing\":0"));
        assert!(json.contains("\"updates\":[]"));
    }
}
