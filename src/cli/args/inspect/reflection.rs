use super::InspectFlagParse;
use crate::cli::args::values::{parse_f32, parse_usize};

pub(crate) fn parse(
    parser: &mut InspectFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--inspect-min-process-reward-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_process_reward_experiences = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-process-reward-positive" if index + 1 < raw.len() => {
            *parser.inspect_min_process_reward_positive = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-process-reward-reinforce" if index + 1 < raw.len() => {
            *parser.inspect_min_process_reward_reinforce = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-process-reward-total" if index + 1 < raw.len() => {
            *parser.inspect_min_process_reward_total = Some(parse_f32(&raw[index + 1], 0.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-pool-dispatch-clamped" if index + 1 < raw.len() => {
            *parser.inspect_max_pool_dispatch_clamped = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-pool-dispatch-low-priority" if index + 1 < raw.len() => {
            *parser.inspect_max_pool_dispatch_low_priority = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-external-semantic-context-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_external_semantic_context_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-external-semantic-contexts" if index + 1 < raw.len() => {
            *parser.inspect_min_external_semantic_contexts = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
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
        "--inspect-min-live-memory-feedback-reinforced" if index + 1 < raw.len() => {
            *parser.inspect_min_live_memory_feedback_reinforced =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-live-memory-feedback-penalized" if index + 1 < raw.len() => {
            *parser.inspect_min_live_memory_feedback_penalized =
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
        "--inspect-max-live-memory-feedback-missing" if index + 1 < raw.len() => {
            *parser.inspect_max_live_memory_feedback_missing =
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
        "--inspect-min-business-contract-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_business_contract_experiences =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-business-contract-passed" if index + 1 < raw.len() => {
            *parser.inspect_min_business_contract_passed = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-business-contract-failed" if index + 1 < raw.len() => {
            *parser.inspect_max_business_contract_failed = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-business-contract-missing-signals" if index + 1 < raw.len() => {
            *parser.inspect_max_business_contract_missing_signals =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-business-contract-protocol-leaks" if index + 1 < raw.len() => {
            *parser.inspect_max_business_contract_protocol_leaks =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-business-contract-substitutions" if index + 1 < raw.len() => {
            *parser.inspect_max_business_contract_substitutions =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-business-contract-evasive-denials" if index + 1 < raw.len() => {
            *parser.inspect_max_business_contract_evasive_denials =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-business-contract-missing-handling-signals" if index + 1 < raw.len() => {
            *parser.inspect_max_business_contract_missing_handling_signals =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        _ => None,
    }
}
