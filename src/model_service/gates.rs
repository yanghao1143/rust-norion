use rust_norion::{
    StateInspectionGate, StateInspectionGateReport, StateInspectionReport, TraceSchemaGateReport,
    evaluate_trace_schema_jsonl,
};

use crate::Args;
use crate::gemma_business::state_gate::{
    business_cycle_state_gate, gemma_business_cycle_state_gate, gemma_business_smoke_state_gate,
    gemma_model_service_smoke_state_gate,
};

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
}
