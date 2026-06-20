use std::io;
use std::path::Path;

use crate::kv_cache::MemoryUpdateReport;
use crate::process_reward::RewardAction;
use crate::rust_validation::RustSnippetCheckReport;

use super::super::fields::json_escape;
use super::json::{
    option_i32_json, option_string_json, option_u64_json, string_array_json, u64_array_json,
};
use super::summary::memory_update_summary;
use super::writer::append_line;

#[allow(clippy::too_many_arguments)]
pub fn rust_check_trace_json_line(
    case_name: Option<&str>,
    report: &RustSnippetCheckReport,
    action: RewardAction,
    amount: f32,
    experience_id: Option<u64>,
    memory_id: Option<u64>,
    memory_ids: &[u64],
    updates: &[MemoryUpdateReport],
) -> String {
    let applied = updates.iter().filter(|update| update.was_applied()).count();
    let missing = updates.len().saturating_sub(applied);
    let removed = updates.iter().filter(|update| update.removed).count();
    let strength_delta = updates
        .iter()
        .map(|update| update.strength_delta.abs())
        .sum::<f32>();
    let update_summaries = updates
        .iter()
        .map(memory_update_summary)
        .collect::<Vec<_>>();
    format!(
        "{{\
         \"schema\":\"rust-norion-rust-check-v1\",\
         \"case\":{},\
         \"rust_check\":{{\"passed\":{},\"label\":\"{}\",\"edition\":\"{}\",\"status_code\":{},\"diagnostic_chars\":{},\"source_path\":\"{}\",\"metadata_path\":\"{}\"}},\
         \"feedback\":{{\"action\":\"{}\",\"amount\":{:.6},\"experience_id\":{},\"memory_id\":{},\"memory_ids\":{},\"applied\":{},\"missing\":{},\"removed\":{},\"strength_delta\":{:.6},\"update_summaries\":{}}}\
         }}",
        option_string_json(case_name),
        report.passed,
        report.feedback_label(),
        json_escape(&report.edition),
        option_i32_json(report.status_code),
        report.diagnostic_chars(),
        json_escape(&report.source_path.display().to_string()),
        json_escape(&report.metadata_path.display().to_string()),
        action.as_str(),
        amount,
        option_u64_json(experience_id),
        option_u64_json(memory_id),
        u64_array_json(memory_ids),
        applied,
        missing,
        removed,
        strength_delta,
        string_array_json(&update_summaries)
    )
}

#[allow(clippy::too_many_arguments)]
pub fn append_rust_check_trace_jsonl(
    path: impl AsRef<Path>,
    case_name: Option<&str>,
    report: &RustSnippetCheckReport,
    action: RewardAction,
    amount: f32,
    experience_id: Option<u64>,
    memory_id: Option<u64>,
    memory_ids: &[u64],
    updates: &[MemoryUpdateReport],
) -> io::Result<()> {
    let line = rust_check_trace_json_line(
        case_name,
        report,
        action,
        amount,
        experience_id,
        memory_id,
        memory_ids,
        updates,
    );
    append_line(path, &line)
}
