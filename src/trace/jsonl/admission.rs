use std::io;
use std::path::Path;

use crate::self_evolution::{
    SelfEvolutionAdmissionReport, SelfEvolutionExperimentRecord,
    SelfEvolutionOperatorApprovalReport, SelfEvolutionRollbackReplayGateReport,
    SelfEvolutionRollbackReplayPlan,
};

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

pub fn self_evolution_experiment_trace_json_line(record: &SelfEvolutionExperimentRecord) -> String {
    record.json_line()
}

pub fn append_self_evolution_experiment_trace_jsonl(
    path: impl AsRef<Path>,
    record: &SelfEvolutionExperimentRecord,
) -> io::Result<()> {
    let line = self_evolution_experiment_trace_json_line(record);
    append_line(path, &line)
}

pub fn self_evolution_rollback_replay_trace_json_line(
    plan: &SelfEvolutionRollbackReplayPlan,
) -> String {
    plan.json_line()
}

pub fn append_self_evolution_rollback_replay_trace_jsonl(
    path: impl AsRef<Path>,
    plan: &SelfEvolutionRollbackReplayPlan,
) -> io::Result<()> {
    let line = self_evolution_rollback_replay_trace_json_line(plan);
    append_line(path, &line)
}

pub fn self_evolution_rollback_replay_gate_trace_json_line(
    report: &SelfEvolutionRollbackReplayGateReport,
) -> String {
    report.json_line()
}

pub fn append_self_evolution_rollback_replay_gate_trace_jsonl(
    path: impl AsRef<Path>,
    report: &SelfEvolutionRollbackReplayGateReport,
) -> io::Result<()> {
    let line = self_evolution_rollback_replay_gate_trace_json_line(report);
    append_line(path, &line)
}

pub fn self_evolution_operator_approval_trace_json_line(
    report: &SelfEvolutionOperatorApprovalReport,
) -> String {
    report.json_line()
}

pub fn append_self_evolution_operator_approval_trace_jsonl(
    path: impl AsRef<Path>,
    report: &SelfEvolutionOperatorApprovalReport,
) -> io::Result<()> {
    let line = self_evolution_operator_approval_trace_json_line(report);
    append_line(path, &line)
}
