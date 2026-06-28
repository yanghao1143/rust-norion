use rust_norion::{
    evaluate_trace_schema_jsonl, StateInspectionGate, StateInspectionGateReport,
    StateInspectionReport, TraceSchemaGateReport,
};

use crate::gemma_business::state_gate::{
    business_cycle_state_gate, gemma_business_cycle_state_gate, gemma_business_smoke_state_gate,
    gemma_model_service_smoke_state_gate,
};
use crate::Args;

use super::request::ModelServiceInspectRequest;

pub(crate) fn model_service_state_gate_report_for_request(
    request: &ModelServiceInspectRequest,
    inspection: &StateInspectionReport,
    args: &Args,
) -> Option<StateInspectionGateReport> {
    let gate = model_service_state_gate_for_request(request, args)?;
    Some(inspection.evaluate(&gate))
}

pub(crate) fn model_service_state_gate_requires_full_experience_scan(
    request: &ModelServiceInspectRequest,
    args: &Args,
) -> bool {
    model_service_state_gate_for_request(request, args)
        .as_ref()
        .is_some_and(state_gate_requires_full_experience_scan)
}

fn model_service_state_gate_for_request(
    request: &ModelServiceInspectRequest,
    args: &Args,
) -> Option<StateInspectionGate> {
    let state_gate_enabled = request.state_gate
        || request.business_gate
        || request.business_cycle_gate
        || request.model_service_gate;
    if !state_gate_enabled {
        return None;
    }

    let gate = if request.model_service_gate {
        gemma_model_service_smoke_state_gate(args)
    } else if request.business_gate && request.business_cycle_gate {
        gemma_business_cycle_state_gate(args)
    } else if request.business_cycle_gate {
        business_cycle_state_gate(args)
    } else if request.business_gate {
        gemma_business_smoke_state_gate(args)
    } else {
        args.state_inspection_gate()
    };
    Some(gate)
}

fn state_gate_requires_full_experience_scan(gate: &StateInspectionGate) -> bool {
    gate.max_experience_hygiene_quarantine_candidates
        .or(gate.max_experience_repairable_legacy_metadata_lessons)
        .or(gate.max_experience_repairable_index_records)
        .or(gate.max_experience_repair_projected_legacy_metadata_lessons)
        .or(gate.max_experience_repair_skipped_missing_clean_gist)
        .or(gate.max_experience_index_overlong_records)
        .or(gate.max_experience_index_overlong_without_clean_gist)
        .or(gate.max_experience_index_record_chars)
        .or(gate.max_experience_index_noisy_records)
        .is_some()
        || gate.max_experience_index_noise_penalty.is_some()
        || gate.min_experience_index_quality_score.is_some()
        || gate.require_experience_index_retrieval_ready
}

pub(crate) fn model_service_trace_gate_report_for_request(
    request: &ModelServiceInspectRequest,
    args: &Args,
) -> std::io::Result<Option<TraceSchemaGateReport>> {
    let trace_gate_enabled = request
        .trace_gate
        .unwrap_or_else(|| args.trace_schema_gate_path.is_some());
    if !trace_gate_enabled {
        return Ok(None);
    }

    if let Some(path) = &args.trace_schema_gate_path {
        evaluate_trace_schema_jsonl(path).map(Some)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "trace_gate requested but no trace schema gate path is configured",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_only_state_gate_uses_online_inspection() {
        let gate = StateInspectionGate {
            min_runtime_tokens: Some(1),
            min_runtime_imported_kv_blocks: Some(1),
            max_runtime_errors: Some(0),
            ..StateInspectionGate::default()
        };

        assert!(!state_gate_requires_full_experience_scan(&gate));
    }

    #[test]
    fn experience_index_gate_requires_full_inspection() {
        let gate = StateInspectionGate {
            max_experience_index_noisy_records: Some(0),
            ..StateInspectionGate::default()
        };

        assert!(state_gate_requires_full_experience_scan(&gate));
    }

    #[test]
    fn fht_dke_gate_uses_online_inspection() {
        let gate = StateInspectionGate {
            min_fht_dke_budget_experiences: Some(1),
            min_fht_dke_enabled_experiences: Some(1),
            min_fht_dke_routed_tokens: Some(128),
            max_fht_dke_token_split_invalid: Some(0),
            min_fht_dke_attention_threshold: Some(0.25),
            max_fht_dke_attention_threshold: Some(1.0),
            min_fht_dke_route_pressure: Some(0.1),
            max_fht_dke_route_pressure: Some(1.0),
            ..StateInspectionGate::default()
        };

        assert!(!state_gate_requires_full_experience_scan(&gate));
    }

    #[test]
    fn self_evolving_memory_writeback_gate_uses_online_inspection() {
        for gate in [
            StateInspectionGate {
                min_self_evolving_memory_writeback_experiences: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_self_evolving_memory_writeback_attempted_records: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_self_evolving_memory_writeback_accepted_records: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                max_self_evolving_memory_writeback_rejected_records: Some(0),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_self_evolving_memory_writeback_write_allowed: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_self_evolving_memory_writeback_durable_write_allowed: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_self_evolving_memory_writeback_applied: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_self_evolving_memory_writeback_applied_to_disk: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_self_evolving_memory_writeback_snapshot_changes: Some(1),
                ..StateInspectionGate::default()
            },
        ] {
            assert!(!state_gate_requires_full_experience_scan(&gate));
        }
    }

    #[test]
    fn feedback_context_gate_uses_online_inspection() {
        for gate in [
            StateInspectionGate {
                min_process_reward_experiences: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_process_reward_positive: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_process_reward_reinforce: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_process_reward_total: Some(0.5),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_external_semantic_context_experiences: Some(1),
                ..StateInspectionGate::default()
            },
            StateInspectionGate {
                min_external_semantic_contexts: Some(1),
                ..StateInspectionGate::default()
            },
        ] {
            assert!(!state_gate_requires_full_experience_scan(&gate));
        }
    }
}
