use std::io;
use std::path::Path;

use crate::self_goal_proposal::SelfGoalQueueApplyReport;

use super::writer::append_line;

pub fn self_goal_queue_apply_trace_json_line(report: &SelfGoalQueueApplyReport) -> String {
    report.json_line()
}

pub fn append_self_goal_queue_apply_trace_jsonl(
    path: impl AsRef<Path>,
    report: &SelfGoalQueueApplyReport,
) -> io::Result<()> {
    let line = self_goal_queue_apply_trace_json_line(report);
    append_line(path, &line)
}
