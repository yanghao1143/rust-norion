use std::io;
use std::path::Path;

use norion_agent::{ToolBuildReportHealthStatus, ToolBuildReportHistoryGateRecord};

use crate::privacy_redaction::stable_redaction_digest;
use crate::trace::tool_build_report::AGENT_TOOL_BUILD_REPORT_TRACE_SCHEMA;

use super::json::option_string_json;
use super::writer::append_line;

pub fn agent_tool_build_report_trace_json_line(
    record: &ToolBuildReportHistoryGateRecord,
) -> String {
    let summary = &record.gate_decision.report_summary;
    let reliability = summary.reliability();
    let report_digest = stable_redaction_digest([
        "agent-tool-build-report-history-gate",
        &summary.telemetry.join("\n"),
        &record.telemetry.join("\n"),
    ]);
    let health = match record.gate_decision.report_health.status {
        ToolBuildReportHealthStatus::Stable => "Stable",
        ToolBuildReportHealthStatus::Watch => "Watch",
        ToolBuildReportHealthStatus::Repair => "Repair",
    };

    format!(
        "{{\
         \"schema\":\"{}\",\
         \"report_kind\":\"history_gate\",\
         \"records\":{},\
         \"requested\":{},\
         \"received\":{},\
         \"built\":{},\
         \"held\":{},\
         \"rejected\":{},\
         \"missing_requests\":{},\
         \"unexpected_receipts\":{},\
         \"duplicate_receipts\":{},\
         \"diagnostics\":{},\
         \"clean\":{},\
         \"reliable\":{},\
         \"open_tool_build_boundary\":{},\
         \"promote_memory_note\":{},\
         \"promote_adaptive_state\":{},\
         \"finalize_eval\":{},\
         \"requires_repair_first\":{},\
         \"repair_tasks\":{},\
         \"reason_count\":{},\
         \"health\":\"{}\",\
         \"report_digest\":{},\
         \"read_only\":true,\
         \"write_allowed\":false,\
         \"applied\":false\
         }}",
        AGENT_TOOL_BUILD_REPORT_TRACE_SCHEMA,
        record.records(),
        summary.requested,
        summary.received,
        summary.built,
        summary.held,
        summary.rejected,
        summary.missing_requests,
        summary.unexpected_receipts,
        summary.duplicate_receipts,
        summary.diagnostics,
        summary.is_clean,
        reliability.reliable,
        record.gate_decision.can_open_tool_build_boundary,
        record.gate_decision.can_promote_memory_note,
        record.gate_decision.can_promote_adaptive_state,
        record.gate_decision.can_finalize_eval,
        record.gate_decision.requires_repair_first,
        record.gate_decision.repair_tasks.len(),
        record.gate_decision.reasons.len(),
        health,
        option_string_json(Some(&report_digest))
    )
}

pub fn append_agent_tool_build_report_trace_jsonl(
    path: impl AsRef<Path>,
    record: &ToolBuildReportHistoryGateRecord,
) -> io::Result<()> {
    let line = agent_tool_build_report_trace_json_line(record);
    append_line(path, &line)
}
