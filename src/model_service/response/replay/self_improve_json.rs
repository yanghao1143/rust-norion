use rust_norion::{
    ExperienceReplayReport, StateInspectionGateReport, StateInspectionReport, TraceSchemaGateReport,
};

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
) -> String {
    let summary =
        SelfImproveGateSummary::from_reports(request, report, state_gate_report, trace_gate_report);

    format!(
        "{{\"ok\":true,\"request_id\":{},\"limit\":{},\"self_improve\":{},\"replay\":{},\"state\":{},\"state_gate\":{},\"trace_gate\":{}}}",
        request_id,
        request.limit,
        self_improve_summary_json(summary),
        model_service_replay_json(report),
        model_service_state_json(inspection),
        option_state_gate_service_json(state_gate_report),
        option_trace_gate_service_json(trace_gate_report)
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
}

impl SelfImproveGateSummary {
    fn from_reports(
        request: &ModelServiceSelfImproveRequest,
        report: &ExperienceReplayReport,
        state_gate_report: Option<&StateInspectionGateReport>,
        trace_gate_report: Option<&TraceSchemaGateReport>,
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
        }
    }
}

fn self_improve_summary_json(summary: SelfImproveGateSummary) -> String {
    format!(
        "{{\"passed\":{},\"replay_passed\":{},\"replay_planned\":{},\"replay_applied\":{},\"state_gate_checked\":{},\"state_gate_passed\":{},\"trace_gate_checked\":{},\"trace_gate_passed\":{},\"state_gate\":{},\"business_gate\":{},\"business_cycle_gate\":{},\"model_service_gate\":{}}}",
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
        summary.model_service_gate
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        });

        assert!(json.contains("\"passed\":true"));
        assert!(json.contains("\"replay_planned\":3"));
        assert!(json.contains("\"replay_applied\":2"));
        assert!(json.contains("\"state_gate_checked\":true"));
        assert!(json.contains("\"business_gate\":false"));
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
        });

        assert!(json.contains("\"passed\":false"));
        assert!(json.contains("\"replay_passed\":false"));
        assert!(json.contains("\"replay_applied\":0"));
        assert!(json.contains("\"trace_gate_checked\":false"));
    }
}
