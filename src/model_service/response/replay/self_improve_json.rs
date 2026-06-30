use rust_norion::{
    ExperienceReplayReport, SelfEvolutionAdmissionReport, SelfEvolutionAdmissionReviewPacketRefs,
    SelfEvolutionValidationLane, StateInspectionGateReport, StateInspectionReport,
    TraceSchemaGateReport,
};

use super::super::super::json::{service_json_string, service_json_string_array};
use super::super::super::request::ModelServiceSelfImproveRequest;
use super::super::gates::{option_state_gate_service_json, option_trace_gate_service_json};
use super::super::state::model_service_state_json;
use super::replay_json::model_service_replay_json;

pub(crate) fn model_service_self_improve_response_json(
    request_id: usize,
    request: &ModelServiceSelfImproveRequest,
    report: &ExperienceReplayReport,
    inspection: &StateInspectionReport,
    state_gate_report: Option<&StateInspectionGateReport>,
    trace_gate_report: Option<&TraceSchemaGateReport>,
    self_evolution_admission_report: &SelfEvolutionAdmissionReport,
) -> String {
    let summary = SelfImproveGateSummary::from_reports(
        request,
        report,
        state_gate_report,
        trace_gate_report,
        self_evolution_admission_report,
    );

    format!(
        "{{\"ok\":true,\"request_id\":{},\"limit\":{},\"self_improve\":{},\"replay\":{},\"state\":{},\"state_gate\":{},\"trace_gate\":{},\"self_evolution_admission\":{}}}",
        request_id,
        request.limit,
        self_improve_summary_json(summary),
        model_service_replay_json(report),
        model_service_state_json(inspection),
        option_state_gate_service_json(state_gate_report),
        option_trace_gate_service_json(trace_gate_report),
        self_evolution_admission_json(self_evolution_admission_report)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelfImproveGateSummary {
    passed: bool,
    replay_passed: bool,
    replay_planned: usize,
    replay_applied: usize,
    state_gate_checked: bool,
    state_gate_passed: bool,
    trace_gate_checked: bool,
    trace_gate_passed: bool,
    state_gate: bool,
    business_gate: bool,
    business_cycle_gate: bool,
    model_service_gate: bool,
    self_evolution_admission_checked: bool,
    self_evolution_admission_admitted_for_human_review: bool,
    self_evolution_admission_human_approval_required: bool,
    self_evolution_admission_blocked: bool,
    self_evolution_admission_blocked_reasons: usize,
    self_evolution_admission_trace_events: usize,
    self_evolution_admission_trace_admitted: usize,
    self_evolution_admission_trace_blocked: usize,
}

impl SelfImproveGateSummary {
    fn from_reports(
        request: &ModelServiceSelfImproveRequest,
        report: &ExperienceReplayReport,
        state_gate_report: Option<&StateInspectionGateReport>,
        trace_gate_report: Option<&TraceSchemaGateReport>,
        self_evolution_admission_report: &SelfEvolutionAdmissionReport,
    ) -> Self {
        let replay_passed = report.applied > 0;
        let state_gate_checked = state_gate_report.is_some();
        let state_gate_passed = state_gate_report
            .map(|report| report.passed)
            .unwrap_or(true);
        let trace_gate_checked = trace_gate_report.is_some();
        let trace_gate_passed = trace_gate_report
            .map(|report| report.passed)
            .unwrap_or(true);
        let self_evolution_admission_trace_events = trace_gate_report
            .map(|report| report.self_evolution_admission_events)
            .unwrap_or(0);
        let self_evolution_admission_trace_admitted = trace_gate_report
            .map(|report| report.self_evolution_admission_admitted)
            .unwrap_or(0);
        let self_evolution_admission_trace_blocked = trace_gate_report
            .map(|report| report.self_evolution_admission_blocked)
            .unwrap_or(0);

        Self {
            passed: replay_passed && state_gate_passed && trace_gate_passed,
            replay_passed,
            replay_planned: report.planned,
            replay_applied: report.applied,
            state_gate_checked,
            state_gate_passed,
            trace_gate_checked,
            trace_gate_passed,
            state_gate: request.inspect.state_gate,
            business_gate: request.inspect.business_gate,
            business_cycle_gate: request.inspect.business_cycle_gate,
            model_service_gate: request.inspect.model_service_gate,
            self_evolution_admission_checked: true,
            self_evolution_admission_admitted_for_human_review: self_evolution_admission_report
                .admitted_for_human_review,
            self_evolution_admission_human_approval_required: self_evolution_admission_report
                .human_approval_required,
            self_evolution_admission_blocked: !self_evolution_admission_report
                .blocked_reasons
                .is_empty(),
            self_evolution_admission_blocked_reasons: self_evolution_admission_report
                .blocked_reasons
                .len(),
            self_evolution_admission_trace_events,
            self_evolution_admission_trace_admitted,
            self_evolution_admission_trace_blocked,
        }
    }
}

fn self_improve_summary_json(summary: SelfImproveGateSummary) -> String {
    format!(
        "{{\"passed\":{},\"replay_passed\":{},\"replay_planned\":{},\"replay_applied\":{},\"state_gate_checked\":{},\"state_gate_passed\":{},\"trace_gate_checked\":{},\"trace_gate_passed\":{},\"state_gate\":{},\"business_gate\":{},\"business_cycle_gate\":{},\"model_service_gate\":{},\"self_evolution_admission_checked\":{},\"self_evolution_admission_admitted_for_human_review\":{},\"self_evolution_admission_human_approval_required\":{},\"self_evolution_admission_blocked\":{},\"self_evolution_admission_blocked_reasons\":{},\"self_evolution_admission_trace_events\":{},\"self_evolution_admission_trace_admitted\":{},\"self_evolution_admission_trace_blocked\":{}}}",
        summary.passed,
        summary.replay_passed,
        summary.replay_planned,
        summary.replay_applied,
        summary.state_gate_checked,
        summary.state_gate_passed,
        summary.trace_gate_checked,
        summary.trace_gate_passed,
        summary.state_gate,
        summary.business_gate,
        summary.business_cycle_gate,
        summary.model_service_gate,
        summary.self_evolution_admission_checked,
        summary.self_evolution_admission_admitted_for_human_review,
        summary.self_evolution_admission_human_approval_required,
        summary.self_evolution_admission_blocked,
        summary.self_evolution_admission_blocked_reasons,
        summary.self_evolution_admission_trace_events,
        summary.self_evolution_admission_trace_admitted,
        summary.self_evolution_admission_trace_blocked
    )
}

fn self_evolution_admission_json(report: &SelfEvolutionAdmissionReport) -> String {
    let review_packet = self_evolution_admission_review_packet_json(&report.review_packet);
    let validation = self_evolution_validation_json(report);
    format!(
        "{{\"candidate_id\":{},\"summary\":{},\"read_only\":{},\"report_only\":{},\"preview_only\":{},\"policy_valid\":{},\"mutation_write_allowed\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"model_weight_write_allowed\":{},\"git_write_allowed\":{},\"human_approval_required\":{},\"admitted_for_human_review\":{},\"review_packet\":{},\"validation_passed\":{},\"validation\":{},\"rust_validation_passed\":{},\"rust_check_items\":{},\"rust_check_passed\":{},\"rust_check_failed\":{},\"benchmark_gate_passed\":{},\"benchmark_gate_failures\":{},\"rollback_budget_clean\":{},\"drift_rollbacks\":{},\"rollback_router_threshold_delta\":{:.6},\"rollback_hierarchy_weight_delta\":{:.6},\"adaptive_preview_evidence_present\":{},\"adaptive_preview_source_count\":{},\"adaptive_preview_readiness\":{{\"router_threshold_preview_ready\":{},\"hierarchy_adjustment_preview_ready\":{},\"kv_fusion_policy_observation_preview_ready\":{},\"blocked_reasons\":{}}},\"adaptive_preview_read_only\":{},\"adaptive_preview_report_only\":{},\"adaptive_preview_preview_only\":{},\"adaptive_preview_write_allowed\":{},\"adaptive_preview_applied\":{},\"blocked_reasons\":{},\"telemetry\":{}}}",
        service_json_string(&report.candidate_id),
        service_json_string(&report.summary_line()),
        report.read_only,
        report.report_only,
        report.preview_only,
        report.policy_valid,
        report.mutation_write_allowed,
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        report.model_weight_write_allowed,
        report.git_write_allowed,
        report.human_approval_required,
        report.admitted_for_human_review,
        review_packet,
        report.validation_passed,
        validation,
        report.rust_validation_passed,
        report.rust_check_items,
        report.rust_check_passed,
        report.rust_check_failed,
        report.benchmark_gate_passed,
        service_json_string_array(&report.benchmark_gate_failures),
        report.rollback_budget_clean,
        report.drift_rollbacks,
        report.rollback_router_threshold_delta,
        report.rollback_hierarchy_weight_delta,
        report.adaptive_preview_evidence_present,
        report.adaptive_preview_source_count,
        report.router_threshold_preview_ready,
        report.hierarchy_adjustment_preview_ready,
        report.kv_fusion_policy_observation_preview_ready,
        service_json_string_array(&report.adaptive_preview_blocked_reasons),
        report.adaptive_preview_read_only,
        report.adaptive_preview_report_only,
        report.adaptive_preview_preview_only,
        report.adaptive_preview_write_allowed,
        report.adaptive_preview_applied,
        service_json_string_array(&report.blocked_reasons),
        service_json_string_array(&report.telemetry)
    )
}

fn self_evolution_validation_json(report: &SelfEvolutionAdmissionReport) -> String {
    format!(
        "{{\"passed\":{},\"compiler\":{},\"tests\":{},\"benchmarks\":{},\"experiments\":{}}}",
        report.validation_passed,
        self_evolution_validation_lane_json(report.validation.compiler),
        self_evolution_validation_lane_json(report.validation.tests),
        self_evolution_validation_lane_json(report.validation.benchmarks),
        self_evolution_validation_lane_json(report.validation.experiments)
    )
}

fn self_evolution_validation_lane_json(lane: SelfEvolutionValidationLane) -> String {
    format!(
        "{{\"items\":{},\"passed\":{},\"failed\":{},\"validation_passed\":{}}}",
        lane.items,
        lane.passed,
        lane.failed,
        lane.passed_at_least(1, true)
    )
}

fn self_evolution_admission_review_packet_json(
    review_packet: &SelfEvolutionAdmissionReviewPacketRefs,
) -> String {
    format!(
        "{{\"approval_review_packet_ids\":{},\"evidence_ids\":{},\"rollback_anchor_ids\":{},\"content_digests\":{},\"source_report_schemas\":{},\"approval_review_packet_count\":{},\"evidence_count\":{},\"rollback_anchor_count\":{},\"content_digest_count\":{},\"source_report_schema_count\":{},\"read_only\":true,\"approval_tokens_included\":false}}",
        service_json_string_array(&review_packet.approval_review_packet_ids),
        service_json_string_array(&review_packet.evidence_ids),
        service_json_string_array(&review_packet.rollback_anchor_ids),
        service_json_string_array(&review_packet.content_digests),
        service_json_string_array(&review_packet.source_report_schemas),
        review_packet.approval_review_packet_ids.len(),
        review_packet.evidence_ids.len(),
        review_packet.rollback_anchor_ids.len(),
        review_packet.content_digests.len(),
        review_packet.source_report_schemas.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_service::request::ModelServiceInspectRequest;
    use rust_norion::{SelfEvolutionValidationEvidence, SelfEvolutionValidationLane};

    #[test]
    fn self_improve_summary_json_renders_gate_outcome() {
        let json = self_improve_summary_json(SelfImproveGateSummary {
            passed: true,
            replay_passed: true,
            replay_planned: 3,
            replay_applied: 2,
            state_gate_checked: true,
            state_gate_passed: true,
            trace_gate_checked: true,
            trace_gate_passed: true,
            state_gate: true,
            business_gate: false,
            business_cycle_gate: true,
            model_service_gate: true,
            self_evolution_admission_checked: true,
            self_evolution_admission_admitted_for_human_review: false,
            self_evolution_admission_human_approval_required: true,
            self_evolution_admission_blocked: true,
            self_evolution_admission_blocked_reasons: 2,
            self_evolution_admission_trace_events: 2,
            self_evolution_admission_trace_admitted: 1,
            self_evolution_admission_trace_blocked: 1,
        });

        assert!(json.contains("\"passed\":true"));
        assert!(json.contains("\"replay_planned\":3"));
        assert!(json.contains("\"replay_applied\":2"));
        assert!(json.contains("\"state_gate_checked\":true"));
        assert!(json.contains("\"business_gate\":false"));
        assert!(json.contains("\"self_evolution_admission_checked\":true"));
        assert!(json.contains("\"self_evolution_admission_blocked_reasons\":2"));
        assert!(json.contains("\"self_evolution_admission_trace_events\":2"));
        assert!(json.contains("\"self_evolution_admission_trace_admitted\":1"));
        assert!(json.contains("\"self_evolution_admission_trace_blocked\":1"));
    }

    #[test]
    fn self_improve_summary_json_keeps_failed_replay_visible() {
        let json = self_improve_summary_json(SelfImproveGateSummary {
            passed: false,
            replay_passed: false,
            replay_planned: 1,
            replay_applied: 0,
            state_gate_checked: false,
            state_gate_passed: true,
            trace_gate_checked: false,
            trace_gate_passed: true,
            state_gate: false,
            business_gate: false,
            business_cycle_gate: false,
            model_service_gate: false,
            self_evolution_admission_checked: true,
            self_evolution_admission_admitted_for_human_review: false,
            self_evolution_admission_human_approval_required: true,
            self_evolution_admission_blocked: true,
            self_evolution_admission_blocked_reasons: 1,
            self_evolution_admission_trace_events: 0,
            self_evolution_admission_trace_admitted: 0,
            self_evolution_admission_trace_blocked: 0,
        });

        assert!(json.contains("\"passed\":false"));
        assert!(json.contains("\"replay_passed\":false"));
        assert!(json.contains("\"replay_applied\":0"));
        assert!(json.contains("\"trace_gate_checked\":false"));
        assert!(json.contains("\"self_evolution_admission_blocked\":true"));
        assert!(json.contains("\"self_evolution_admission_trace_events\":0"));
    }

    #[test]
    fn self_improve_summary_from_reports_records_trace_admission_counts() {
        let request = ModelServiceSelfImproveRequest {
            limit: 2,
            inspect: ModelServiceInspectRequest {
                trace_gate: Some(true),
                ..ModelServiceInspectRequest::default()
            },
        };
        let replay = ExperienceReplayReport {
            planned: 2,
            applied: 1,
            ..ExperienceReplayReport::default()
        };
        let trace_gate = TraceSchemaGateReport {
            passed: true,
            checked_lines: 5,
            rust_check_events: 0,
            rust_check_passed: 0,
            rust_check_failed: 0,
            rust_check_feedback_updates: 0,
            rust_check_feedback_applied: 0,
            business_contract_events: 0,
            business_contract_event_passed: 0,
            business_contract_event_failed: 0,
            business_contract_event_missing_signals: 0,
            business_contract_event_protocol_leaks: 0,
            business_contract_event_substitutions: 0,
            business_contract_event_evasive_denials: 0,
            business_contract_event_raw_passed: 0,
            business_contract_event_raw_failed: 0,
            business_contract_event_response_normalized: 0,
            business_contract_event_sanitized: 0,
            business_contract_event_canonical_fallbacks: 0,
            runtime_error_events: 0,
            runtime_timeout_events: 0,
            self_evolution_admission_events: 3,
            self_evolution_admission_admitted: 2,
            self_evolution_admission_blocked: 1,
            self_evolution_admission_review_packets: 3,
            self_evolution_admission_evidence_ids: 6,
            self_evolution_admission_missing_review_packet_refs: 0,
            improvement_corpus_events: 0,
            improvement_corpus_episodes: 0,
            improvement_corpus_active_adaptation: 0,
            improvement_corpus_compiler_passed: 0,
            improvement_corpus_test_passed: 0,
            improvement_corpus_benchmark_passed: 0,
            improvement_corpus_privacy_rejected: 0,
            improvement_corpus_secret_leaks: 0,
            failures: Vec::new(),
            ..TraceSchemaGateReport::default()
        };
        let admission = blocked_admission_report();

        let summary = SelfImproveGateSummary::from_reports(
            &request,
            &replay,
            None,
            Some(&trace_gate),
            &admission,
        );

        assert!(summary.passed);
        assert_eq!(summary.self_evolution_admission_trace_events, 3);
        assert_eq!(summary.self_evolution_admission_trace_admitted, 2);
        assert_eq!(summary.self_evolution_admission_trace_blocked, 1);
    }

    #[test]
    fn self_evolution_admission_json_exposes_read_only_review_boundary() {
        let report = blocked_admission_report();

        let json = self_evolution_admission_json(&report);

        assert!(json.contains("\"candidate_id\":\"service-candidate\""));
        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"report_only\":true"));
        assert!(json.contains("\"preview_only\":true"));
        assert!(json.contains("\"memory_store_write_allowed\":false"));
        assert!(json.contains("\"ndkv_write_allowed\":false"));
        assert!(json.contains("\"model_weight_write_allowed\":false"));
        assert!(json.contains("\"admitted_for_human_review\":false"));
        assert!(json.contains("\"review_packet\":{"));
        assert!(json.contains("\"validation_passed\":false"));
        assert!(json.contains("\"validation\":{\"passed\":false"));
        assert!(json.contains(
            "\"compiler\":{\"items\":1,\"passed\":1,\"failed\":0,\"validation_passed\":true}"
        ));
        assert!(json.contains(
            "\"tests\":{\"items\":2,\"passed\":1,\"failed\":1,\"validation_passed\":false}"
        ));
        assert!(json.contains(
            "\"benchmarks\":{\"items\":3,\"passed\":2,\"failed\":1,\"validation_passed\":false}"
        ));
        assert!(json.contains(
            "\"experiments\":{\"items\":4,\"passed\":4,\"failed\":0,\"validation_passed\":true}"
        ));
        assert!(json.contains("\"approval_review_packet_count\":1"));
        assert!(json.contains("\"evidence_count\":2"));
        assert!(json.contains("\"approval_tokens_included\":false"));
        assert!(json.contains("approval-review:service-candidate"));
        assert!(json.contains("\"benchmark_gate_passed\":false"));
        assert!(json.contains("\"drift_rollbacks\":1"));
        assert!(json.contains("\"rollback_router_threshold_delta\":0.125000"));
        assert!(json.contains("\"rollback_hierarchy_weight_delta\":0.250000"));
        assert!(json.contains("\"adaptive_preview_readiness\":{"));
        assert!(json.contains("\"router_threshold_preview_ready\":false"));
        assert!(json.contains("\"hierarchy_adjustment_preview_ready\":true"));
        assert!(json.contains("\"kv_fusion_policy_observation_preview_ready\":false"));
        assert!(json.contains("hierarchy_preview_ready_but_kv_policy_missing"));
        assert!(json.contains("self_evolution_admission_benchmark_gate_failed"));
        assert!(json.contains("self_evolution_admission_adaptive_preview_evidence_missing"));
        assert!(
            json.contains("self_evolution_admission_model_service_benchmark_gate_evidence_missing")
        );
    }

    fn blocked_admission_report() -> SelfEvolutionAdmissionReport {
        SelfEvolutionAdmissionReport {
            candidate_id: "service-candidate".to_owned(),
            read_only: true,
            report_only: true,
            preview_only: true,
            policy_valid: true,
            mutation_write_allowed: false,
            memory_store_write_allowed: false,
            ndkv_write_allowed: false,
            model_weight_write_allowed: false,
            git_write_allowed: false,
            human_approval_required: true,
            admitted_for_human_review: false,
            rust_check_items: 1,
            rust_check_passed: 1,
            rust_check_failed: 0,
            rust_validation_passed: true,
            validation: SelfEvolutionValidationEvidence::from_lanes(
                SelfEvolutionValidationLane::new(1, 1, 0),
                SelfEvolutionValidationLane::new(2, 1, 1),
                SelfEvolutionValidationLane::new(3, 2, 1),
                SelfEvolutionValidationLane::new(4, 4, 0),
            ),
            validation_passed: false,
            benchmark_gate_passed: false,
            benchmark_gate_failures: vec![
                "self_evolution_admission_model_service_benchmark_gate_evidence_missing".to_owned(),
            ],
            rollback_budget_clean: true,
            drift_rollbacks: 1,
            rollback_router_threshold_delta: 0.125,
            rollback_hierarchy_weight_delta: 0.25,
            adaptive_preview_evidence_present: false,
            router_threshold_preview_ready: false,
            hierarchy_adjustment_preview_ready: true,
            kv_fusion_policy_observation_preview_ready: false,
            adaptive_preview_source_count: 0,
            adaptive_preview_read_only: true,
            adaptive_preview_report_only: true,
            adaptive_preview_preview_only: true,
            adaptive_preview_write_allowed: false,
            adaptive_preview_applied: false,
            adaptive_preview_blocked_reasons: vec![
                "hierarchy_preview_ready_but_kv_policy_missing".to_owned(),
            ],
            review_packet: SelfEvolutionAdmissionReviewPacketRefs {
                approval_review_packet_ids: vec!["approval-review:service-candidate".to_owned()],
                evidence_ids: vec![
                    "rust-check:service-candidate:items-1:passed-1:failed-0".to_owned(),
                    "benchmark-gate:service-candidate:passed-false:failures-1".to_owned(),
                ],
                rollback_anchor_ids: vec!["rollback-budget:service-candidate:drift-1".to_owned()],
                content_digests: vec!["fnv64:service-candidate".to_owned()],
                source_report_schemas: vec![
                    "rust-norion-self-evolution-admission-v1".to_owned(),
                    "rust-norion-benchmark-gate-v1".to_owned(),
                ],
            },
            blocked_reasons: vec![
                "self_evolution_admission_benchmark_gate_failed".to_owned(),
                "self_evolution_admission_adaptive_preview_evidence_missing".to_owned(),
            ],
            telemetry: vec!["self_evolution_admission=true".to_owned()],
        }
    }
}
