use rust_norion::ExperienceReplayReport;

use super::super::super::json::service_json_string;

pub(crate) fn model_service_replay_json(report: &ExperienceReplayReport) -> String {
    format!(
        "{{\"summary\":{},\"planned\":{},\"applied\":{},\"router_updates\":{},\"hierarchy_updates\":{},\"memory_updates\":{},\"runtime_kv_budget_pressure_items\":{},\"avg_runtime_kv_budget_pressure\":{:.3},\"max_runtime_kv_budget_pressure\":{:.3},\"runtime_kv_weak_import_pressure_items\":{},\"avg_runtime_kv_weak_import_pressure\":{:.3},\"max_runtime_kv_weak_import_pressure\":{:.3},\"recursive_runtime_items\":{},\"recursive_runtime_calls\":{},\"avg_recursive_call_pressure\":{:.3},\"max_recursive_call_pressure\":{:.3},\"live_memory_feedback_items\":{},\"live_memory_feedback_updates\":{},\"live_memory_feedback_applied\":{},\"live_memory_feedback_missing\":{},\"live_memory_feedback_strength_delta\":{:.6},\"rust_check_items\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"rust_check_diagnostic_chars\":{},\"rust_check_live_memory_feedback_items\":{},\"rust_check_live_memory_feedback_updates\":{},\"rust_check_live_memory_feedback_applied\":{},\"rust_check_live_memory_feedback_missing\":{},\"rust_check_live_memory_feedback_strength_delta\":{:.6},\"business_contract_items\":{},\"business_contract_passed\":{},\"business_contract_failed\":{},\"business_contract_raw_passed\":{},\"business_contract_raw_failed\":{},\"business_contract_response_normalized\":{},\"business_contract_sanitized\":{},\"business_contract_canonical_fallbacks\":{},\"pool_dispatch_items\":{},\"pool_dispatch_forwarded\":{},\"pool_dispatch_clamped\":{},\"pool_dispatch_low_priority\":{},\"live_evolution_items\":{},\"live_evolution_online_reward_feedbacks\":{},\"live_evolution_memory_updates\":{}}}",
        service_json_string(&report.summary()),
        report.planned,
        report.applied,
        report.router_updates,
        report.hierarchy_updates,
        report.applied_memory_updates,
        report.runtime_kv_budget_pressure_items,
        report.average_runtime_kv_budget_pressure,
        report.max_runtime_kv_budget_pressure,
        report.runtime_kv_weak_import_pressure_items,
        report.average_runtime_kv_weak_import_pressure,
        report.max_runtime_kv_weak_import_pressure,
        report.recursive_runtime_items,
        report.recursive_runtime_calls,
        report.average_recursive_call_pressure,
        report.max_recursive_call_pressure,
        report.live_memory_feedback_items,
        report.live_memory_feedback_updates,
        report.live_memory_feedback_applied,
        report.live_memory_feedback_missing,
        report.live_memory_feedback_strength_delta,
        report.rust_check_items,
        report.rust_check_passed,
        report.rust_check_failed,
        report.rust_check_diagnostic_chars,
        report.rust_check_live_memory_feedback_items,
        report.rust_check_live_memory_feedback_updates,
        report.rust_check_live_memory_feedback_applied,
        report.rust_check_live_memory_feedback_missing,
        report.rust_check_live_memory_feedback_strength_delta,
        report.business_contract_items,
        report.business_contract_passed,
        report.business_contract_failed,
        report.business_contract_raw_passed,
        report.business_contract_raw_failed,
        report.business_contract_response_normalized,
        report.business_contract_sanitized,
        report.business_contract_canonical_fallbacks,
        report.pool_dispatch_items,
        report.pool_dispatch_forwarded,
        report.pool_dispatch_clamped,
        report.pool_dispatch_low_priority,
        report.live_evolution_items,
        report.live_evolution_online_reward_feedbacks,
        report.live_evolution_memory_updates
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_json_renders_core_and_business_evidence() {
        let report = ExperienceReplayReport {
            planned: 3,
            applied: 2,
            router_updates: 1,
            applied_memory_updates: 4,
            runtime_kv_budget_pressure_items: 12,
            average_runtime_kv_budget_pressure: 0.345,
            max_runtime_kv_budget_pressure: 0.789,
            runtime_kv_weak_import_pressure_items: 13,
            average_runtime_kv_weak_import_pressure: 0.234,
            max_runtime_kv_weak_import_pressure: 0.678,
            recursive_runtime_items: 2,
            recursive_runtime_calls: 5,
            average_recursive_call_pressure: 0.123,
            max_recursive_call_pressure: 0.456,
            business_contract_items: 6,
            business_contract_response_normalized: 7,
            pool_dispatch_items: 8,
            pool_dispatch_forwarded: 9,
            pool_dispatch_clamped: 10,
            pool_dispatch_low_priority: 11,
            live_evolution_memory_updates: 8,
            ..ExperienceReplayReport::default()
        };

        let json = model_service_replay_json(&report);

        assert!(json.contains("\"planned\":3"));
        assert!(json.contains("\"applied\":2"));
        assert!(json.contains("\"router_updates\":1"));
        assert!(json.contains("\"memory_updates\":4"));
        assert!(json.contains("\"runtime_kv_budget_pressure_items\":12"));
        assert!(json.contains("\"avg_runtime_kv_budget_pressure\":0.345"));
        assert!(json.contains("\"max_runtime_kv_budget_pressure\":0.789"));
        assert!(json.contains("\"runtime_kv_weak_import_pressure_items\":13"));
        assert!(json.contains("\"avg_runtime_kv_weak_import_pressure\":0.234"));
        assert!(json.contains("\"max_runtime_kv_weak_import_pressure\":0.678"));
        assert!(json.contains("\"recursive_runtime_items\":2"));
        assert!(json.contains("\"recursive_runtime_calls\":5"));
        assert!(json.contains("\"avg_recursive_call_pressure\":0.123"));
        assert!(json.contains("\"max_recursive_call_pressure\":0.456"));
        assert!(json.contains("\"business_contract_items\":6"));
        assert!(json.contains("\"business_contract_response_normalized\":7"));
        assert!(json.contains("\"pool_dispatch_items\":8"));
        assert!(json.contains("\"pool_dispatch_forwarded\":9"));
        assert!(json.contains("\"pool_dispatch_clamped\":10"));
        assert!(json.contains("\"pool_dispatch_low_priority\":11"));
        assert!(json.contains("\"live_evolution_memory_updates\":8"));
    }
}
