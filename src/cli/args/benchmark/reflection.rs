use super::BenchmarkFlagParse;
use crate::cli::args::values::parse_usize;

pub(crate) fn parse(
    parser: &mut BenchmarkFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--benchmark-min-reflection-issue-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_reflection_issue_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-reflection-issues" if index + 1 < raw.len() => {
            *parser.benchmark_min_reflection_issues = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-critical-reflection-issue-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_critical_reflection_issue_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-critical-reflection-issues" if index + 1 < raw.len() => {
            *parser.benchmark_min_critical_reflection_issues =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-revision-action-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_revision_action_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-revision-actions" if index + 1 < raw.len() => {
            *parser.benchmark_min_revision_actions = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-reflection-issue-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_reflection_issue_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-critical-reflection-issue-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_critical_reflection_issue_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-revision-action-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_revision_action_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        _ => None,
    }
}
