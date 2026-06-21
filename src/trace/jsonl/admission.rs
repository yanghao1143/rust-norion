use std::io;
use std::path::Path;

use crate::self_evolution::SelfEvolutionAdmissionReport;

use super::writer::append_line;

pub fn self_evolution_admission_trace_json_line(report: &SelfEvolutionAdmissionReport) -> String {
    report.json_line()
}

pub fn append_self_evolution_admission_trace_jsonl(
    path: impl AsRef<Path>,
    report: &SelfEvolutionAdmissionReport,
) -> io::Result<()> {
    let line = self_evolution_admission_trace_json_line(report);
    append_line(path, &line)
}
