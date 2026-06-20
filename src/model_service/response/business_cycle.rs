mod cycle_json;
mod feedback_json;
mod gate_flags;
mod generate_json;
mod pool_dispatch_json;
mod rust_check_json;
mod self_improve_json;

use self::cycle_json::business_cycle_gate_json;
use self::feedback_json::business_cycle_feedback_json;
use self::gate_flags::BusinessCycleGateFlags;
use self::generate_json::business_cycle_generate_json;
use self::pool_dispatch_json::{
    option_pool_dispatch_service_json, pool_stage_dispatch_service_json,
};
use self::rust_check_json::option_business_cycle_rust_check_json;
use self::self_improve_json::business_cycle_self_improve_json;
use super::super::types::ModelServiceBusinessCycleReport;
use super::gates::{option_state_gate_service_json, option_trace_gate_service_json};
use super::replay::option_replay_service_json;
use super::state::model_service_state_json;
use crate::gemma_business::eval_adapter::{
    project_root_business_cycle_eval, root_business_cycle_eval_json,
};

pub(crate) fn model_service_business_cycle_response_json(
    request_id: usize,
    report: &ModelServiceBusinessCycleReport,
) -> String {
    let gate_flags = BusinessCycleGateFlags::from_report(report);

    let response = format!(
        "{{\"ok\":true,\"request_id\":{},\"pool_dispatch\":{},\"pool_stage_dispatch\":{},\"business_cycle\":{},\"generate\":{},\"feedback\":{},\"rust_check\":{},\"self_improve\":{},\"replay\":{},\"state\":{},\"state_gate\":{},\"trace_gate\":{}}}",
        request_id,
        option_pool_dispatch_service_json(
            report.pool_dispatch.as_ref(),
            report.pool_dispatch_forwarded
        ),
        pool_stage_dispatch_service_json(&report.pool_stage_dispatch),
        business_cycle_gate_json(&gate_flags),
        business_cycle_generate_json(report),
        business_cycle_feedback_json(report, gate_flags.feedback_applied),
        option_business_cycle_rust_check_json(report),
        business_cycle_self_improve_json(report, gate_flags.self_improve_passed),
        option_replay_service_json(report.replay_report.as_ref()),
        model_service_state_json(&report.inspection),
        option_state_gate_service_json(report.state_gate_report.as_ref()),
        option_trace_gate_service_json(report.trace_gate_report.as_ref())
    );
    let projection = project_root_business_cycle_eval(None, Some(&response), None);
    append_eval_section(response, &root_business_cycle_eval_json(&projection))
}

fn append_eval_section(response: String, eval_json: &str) -> String {
    let Some(prefix) = response.strip_suffix('}') else {
        return response;
    };
    format!("{prefix},\"eval\":{eval_json}}}")
}

#[cfg(test)]
mod tests {
    use super::append_eval_section;

    #[test]
    fn appends_eval_as_additive_final_json_section() {
        let json = append_eval_section(
            "{\"ok\":true,\"business_cycle\":{\"passed\":true}}".to_owned(),
            "{\"report_only\":true,\"failure_kind\":\"none\"}",
        );

        assert_eq!(
            json,
            "{\"ok\":true,\"business_cycle\":{\"passed\":true},\"eval\":{\"report_only\":true,\"failure_kind\":\"none\"}}"
        );
    }
}
