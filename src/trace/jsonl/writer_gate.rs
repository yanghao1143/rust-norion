use std::io;
use std::path::Path;

use crate::writer_gate::UnifiedWriterGateReport;

use super::writer::append_line;

pub fn unified_writer_gate_trace_json_line(report: &UnifiedWriterGateReport) -> String {
    report.json_line()
}

pub fn append_unified_writer_gate_trace_jsonl(
    path: impl AsRef<Path>,
    report: &UnifiedWriterGateReport,
) -> io::Result<()> {
    let line = unified_writer_gate_trace_json_line(report);
    append_line(path, &line)
}
