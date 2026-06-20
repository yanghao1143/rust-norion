use super::InspectFlagParse;
use crate::cli::args::values::{parse_f32, parse_usize};

pub(crate) fn parse(
    parser: &mut InspectFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--inspect-min-reflection-issue-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_reflection_issue_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-critical-reflection-issue-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_critical_reflection_issue_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-revision-action-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_revision_action_experiences = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-live-memory-feedback-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_live_memory_feedback_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-live-memory-feedback-updates" if index + 1 < raw.len() => {
            *parser.inspect_min_live_memory_feedback_updates =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-live-memory-feedback-detail-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_live_memory_feedback_detail_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-live-memory-feedback-applied" if index + 1 < raw.len() => {
            *parser.inspect_min_live_memory_feedback_applied =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-live-memory-feedback-strength-delta" if index + 1 < raw.len() => {
            *parser.inspect_min_live_memory_feedback_strength_delta =
                Some(parse_f32(&raw[index + 1], 0.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-rust-check-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_rust_check_experiences = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-rust-check-passed" if index + 1 < raw.len() => {
            *parser.inspect_min_rust_check_passed = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-rust-check-failed" if index + 1 < raw.len() => {
            *parser.inspect_max_rust_check_failed = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-rust-check-diagnostic-chars" if index + 1 < raw.len() => {
            *parser.inspect_min_rust_check_diagnostic_chars = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        _ => None,
    }
}
