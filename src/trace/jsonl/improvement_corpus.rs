use std::io;
use std::path::Path;

use crate::improvement_corpus::ImprovementCorpusReport;

use super::writer::append_line;

pub fn improvement_corpus_trace_json_line(report: &ImprovementCorpusReport) -> String {
    report.json_line()
}

pub fn append_improvement_corpus_trace_jsonl(
    path: impl AsRef<Path>,
    report: &ImprovementCorpusReport,
) -> io::Result<()> {
    let line = improvement_corpus_trace_json_line(report);
    append_line(path, &line)
}
