use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

use crate::engine::InferenceOutcome;
use crate::hierarchy::TaskProfile;

use super::core::{trace_json_line, trace_json_line_with_case};

pub(super) fn append_line(path: impl AsRef<Path>, line: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")
}

pub fn append_trace_jsonl(
    path: impl AsRef<Path>,
    prompt: &str,
    profile: TaskProfile,
    elapsed_ms: u128,
    outcome: &InferenceOutcome,
) -> io::Result<()> {
    let line = trace_json_line(prompt, profile, elapsed_ms, outcome);
    append_line(path, &line)
}

pub fn append_trace_jsonl_with_case(
    path: impl AsRef<Path>,
    case_name: &str,
    prompt: &str,
    profile: TaskProfile,
    elapsed_ms: u128,
    outcome: &InferenceOutcome,
) -> io::Result<()> {
    let line = trace_json_line_with_case(Some(case_name), prompt, profile, elapsed_ms, outcome);
    append_line(path, &line)
}
