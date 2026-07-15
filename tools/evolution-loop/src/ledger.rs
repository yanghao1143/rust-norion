use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::helper_feedback;
use crate::json::{
    json_array_field, json_f64_field, json_object_field, json_string, json_string_array,
    json_string_field, preview_text,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RoundRecord {
    pub(crate) round: usize,
    pub(crate) case_name: String,
    pub(crate) prompt: String,
    pub(crate) started_unix: u64,
    pub(crate) finished_unix: u64,
    pub(crate) success: bool,
    pub(crate) error: Option<String>,
    pub(crate) runtime_tokens: Option<u64>,
    pub(crate) runtime_model: Option<String>,
    pub(crate) answer: Option<String>,
    pub(crate) elapsed_ms: Option<u64>,
    pub(crate) business_cycle_passed: Option<bool>,
    pub(crate) feedback_applied: Option<u64>,
    pub(crate) rust_check_checked: Option<bool>,
    pub(crate) rust_check_passed: Option<bool>,
    pub(crate) rust_check_feedback_applied: Option<u64>,
    pub(crate) validation_checked: Option<bool>,
    pub(crate) validation_passed: Option<bool>,
    pub(crate) validation_command_source: Option<String>,
    pub(crate) validation_command_safety: Option<String>,
    pub(crate) validation_command_preview: Option<String>,
    pub(crate) validation_phase: Option<String>,
    pub(crate) validation_status_code: Option<i32>,
    pub(crate) validation_elapsed_ms: Option<u64>,
    pub(crate) validation_stdout_tail: Option<String>,
    pub(crate) validation_stderr_tail: Option<String>,
    pub(crate) self_improve_passed: Option<bool>,
    pub(crate) state_gate_checked: Option<bool>,
    pub(crate) state_gate_passed: Option<bool>,
    pub(crate) trace_gate_checked: Option<bool>,
    pub(crate) trace_gate_passed: Option<bool>,
    pub(crate) delta_chars: usize,
    pub(crate) stages: Vec<String>,
    pub(crate) meta: Vec<String>,
    pub(crate) allocation_evidence: Vec<String>,
    pub(crate) final_json: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LedgerHygiene {
    pub(crate) records: usize,
    pub(crate) unique_rounds: usize,
    pub(crate) duplicate_rounds: usize,
    pub(crate) non_monotonic_rounds: usize,
    pub(crate) missing_rounds: usize,
    pub(crate) round_gaps: usize,
    pub(crate) max_round: Option<usize>,
}

impl LedgerHygiene {
    pub(crate) fn state_consistency_failures(&self) -> Vec<String> {
        let mut failures = Vec::new();
        if self.duplicate_rounds > 0 {
            failures.push(format!(
                "ledger has {} duplicate round record(s)",
                self.duplicate_rounds
            ));
        }
        if self.non_monotonic_rounds > 0 {
            failures.push(format!(
                "ledger has {} non-monotonic round record(s)",
                self.non_monotonic_rounds
            ));
        }
        if self.missing_rounds > 0 {
            failures.push(format!(
                "ledger has {} record(s) without valid round",
                self.missing_rounds
            ));
        }
        if self.round_gaps > 0 {
            failures.push(format!(
                "ledger has {} missing round number(s) before max round {}",
                self.round_gaps,
                self.max_round.unwrap_or_default()
            ));
        }
        failures
    }
}

pub(crate) fn next_round(path: &Path) -> Result<usize, String> {
    let hygiene = read_ledger_hygiene(path)?;
    hygiene
        .max_round
        .unwrap_or_default()
        .checked_add(1)
        .ok_or_else(|| format!("ledger {} max round is too large", path.display()))
}

pub(crate) fn read_ledger_hygiene(path: &Path) -> Result<LedgerHygiene, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LedgerHygiene::default());
        }
        Err(error) => return Err(format!("read ledger {} failed: {error}", path.display())),
    };
    Ok(ledger_hygiene(
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(round_from_record_json),
    ))
}

pub(crate) fn ledger_hygiene<I>(rounds: I) -> LedgerHygiene
where
    I: IntoIterator<Item = Option<usize>>,
{
    let mut hygiene = LedgerHygiene::default();
    let mut seen_rounds = BTreeSet::new();
    let mut previous_round = None;

    for round in rounds {
        hygiene.records += 1;
        match round {
            Some(round) if round > 0 => {
                if !seen_rounds.insert(round) {
                    hygiene.duplicate_rounds += 1;
                }
                if previous_round.is_some_and(|previous| round <= previous) {
                    hygiene.non_monotonic_rounds += 1;
                }
                previous_round = Some(round);
            }
            Some(_) | None => hygiene.missing_rounds += 1,
        }
    }

    hygiene.unique_rounds = seen_rounds.len();
    hygiene.max_round = seen_rounds.iter().next_back().copied();
    if let Some(max_round) = hygiene.max_round {
        hygiene.round_gaps = max_round.saturating_sub(hygiene.unique_rounds);
    }
    hygiene
}

pub(crate) fn append_record(path: &Path, record: &RoundRecord) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "create ledger directory {} failed: {error}",
                parent.display()
            )
        })?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("open ledger {} failed: {error}", path.display()))?;
    writeln!(file, "{}", record_json(record))
        .map_err(|error| format!("write ledger {} failed: {error}", path.display()))
}

pub(crate) fn record_json(record: &RoundRecord) -> String {
    let generate_json = record
        .final_json
        .as_deref()
        .and_then(|final_json| json_object_field(final_json, "generate"));
    let quality_score = generate_json
        .as_deref()
        .and_then(|generate| json_f64_field(generate, "quality"));
    let process_reward = generate_json
        .as_deref()
        .and_then(|generate| json_f64_field(generate, "process_reward"));
    let reward_action = generate_json
        .as_deref()
        .and_then(|generate| json_string_field(generate, "action"));
    let eval_json = record
        .final_json
        .as_deref()
        .and_then(|final_json| json_object_field(final_json, "eval"));
    let pool_stage_dispatch_json = record
        .final_json
        .as_deref()
        .and_then(|final_json| json_array_field(final_json, "pool_stage_dispatch"));
    let helper_stage_feedback = helper_feedback::feedback_by_role_json(&record.meta);
    let helper_stage_contract = helper_feedback::contract_by_role_json(&record.meta);
    let reflection_risk = json_object_field(&helper_stage_contract, "review")
        .and_then(|review| json_object_field(&review, "fields"))
        .and_then(|fields| json_string_field(&fields, "risk"));
    let drift_detected = drift_detected_from_reward(reward_action.as_deref());
    format!(
        "{{\"round\":{},\"case\":{},\"prompt_preview\":{},\"started_unix\":{},\"finished_unix\":{},\"success\":{},\"strategy\":\"single\",\"quality_score\":{},\"process_reward\":{},\"reward_action\":{},\"reflection_risk\":{},\"drift_detected\":{},\"cost\":{},\"cost_basis\":{},\"error\":{},\"runtime_tokens\":{},\"runtime_model\":{},\"answer\":{},\"elapsed_ms\":{},\"business_cycle_passed\":{},\"feedback_applied\":{},\"rust_check_checked\":{},\"rust_check_passed\":{},\"rust_check_feedback_applied\":{},\"validation_checked\":{},\"validation_passed\":{},\"validation_command_source\":{},\"validation_command_safety\":{},\"validation_command_preview\":{},\"validation_phase\":{},\"validation_status_code\":{},\"validation_elapsed_ms\":{},\"validation_stdout_tail\":{},\"validation_stderr_tail\":{},\"self_improve_passed\":{},\"state_gate_checked\":{},\"state_gate_passed\":{},\"trace_gate_checked\":{},\"trace_gate_passed\":{},\"delta_chars\":{},\"stages\":{},\"meta\":{},\"helper_stage_feedback_by_role\":{},\"helper_stage_contract_by_role\":{},\"allocation_evidence\":{},\"final_json_pool_stage_dispatch\":{},\"eval\":{},\"final_preview\":{}}}",
        record.round,
        json_string(&record.case_name),
        json_string(&preview_text(&record.prompt, 240)),
        record.started_unix,
        record.finished_unix,
        record.success,
        option_f64_json(quality_score),
        option_f64_json(process_reward),
        option_json_string(reward_action.as_deref()),
        option_json_string(reflection_risk.as_deref()),
        option_bool_json(drift_detected),
        option_u64_json(record.runtime_tokens),
        option_json_string(record.runtime_tokens.map(|_| "runtime_tokens")),
        option_json_string(record.error.as_deref()),
        option_u64_json(record.runtime_tokens),
        option_json_string(record.runtime_model.as_deref()),
        option_json_string(
            record
                .answer
                .as_deref()
                .map(|value| preview_text(value, 600))
                .as_deref()
        ),
        option_u64_json(record.elapsed_ms),
        option_bool_json(record.business_cycle_passed),
        option_u64_json(record.feedback_applied),
        option_bool_json(record.rust_check_checked),
        option_bool_json(record.rust_check_passed),
        option_u64_json(record.rust_check_feedback_applied),
        option_bool_json(record.validation_checked),
        option_bool_json(record.validation_passed),
        option_json_string(record.validation_command_source.as_deref()),
        option_json_string(record.validation_command_safety.as_deref()),
        option_json_string(
            record
                .validation_command_preview
                .as_deref()
                .map(|value| preview_text(value, 240))
                .as_deref()
        ),
        option_json_string(record.validation_phase.as_deref()),
        option_i32_json(record.validation_status_code),
        option_u64_json(record.validation_elapsed_ms),
        option_json_string(record.validation_stdout_tail.as_deref()),
        option_json_string(record.validation_stderr_tail.as_deref()),
        option_bool_json(record.self_improve_passed),
        option_bool_json(record.state_gate_checked),
        option_bool_json(record.state_gate_passed),
        option_bool_json(record.trace_gate_checked),
        option_bool_json(record.trace_gate_passed),
        record.delta_chars,
        json_string_array(&record.stages),
        json_string_array(&record.meta),
        helper_stage_feedback,
        helper_stage_contract,
        json_string_array(&record.allocation_evidence),
        option_json_array(pool_stage_dispatch_json.as_deref()),
        option_json_object(eval_json.as_deref()),
        option_json_string(
            record
                .final_json
                .as_deref()
                .map(|value| preview_text(value, 1200))
                .as_deref()
        )
    )
}

fn round_from_record_json(line: &str) -> Option<usize> {
    let needle = "\"round\"";
    let after_field = line.get(line.find(needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?;
    let digits = after_colon
        .trim_start()
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    digits.parse::<usize>().ok()
}

fn option_json_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_f64_json(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_i32_json(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_json_object(value: Option<&str>) -> String {
    value.unwrap_or("null").to_owned()
}

fn option_json_array(value: Option<&str>) -> String {
    value.unwrap_or("null").to_owned()
}

fn drift_detected_from_reward(reward_action: Option<&str>) -> Option<bool> {
    if reward_action.is_some_and(|action| action.eq_ignore_ascii_case("penalize")) {
        return Some(true);
    }
    reward_action
        .filter(|action| action.eq_ignore_ascii_case("reinforce"))
        .map(|_| false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_json_contains_core_fields() {
        let json = record_json(&RoundRecord {
            round: 2,
            case_name: "case-2".to_owned(),
            prompt: "prompt".to_owned(),
            started_unix: 10,
            finished_unix: 20,
            success: true,
            error: None,
            runtime_tokens: Some(42),
            runtime_model: Some("google/gemma".to_owned()),
            answer: Some("answer".to_owned()),
            elapsed_ms: Some(99),
            business_cycle_passed: Some(true),
            feedback_applied: Some(2),
            rust_check_checked: Some(true),
            rust_check_passed: Some(true),
            rust_check_feedback_applied: Some(1),
            validation_checked: Some(true),
            validation_passed: Some(true),
            validation_command_source: Some("test-gate".to_owned()),
            validation_command_safety: Some("safe".to_owned()),
            validation_command_preview: Some(
                "cargo test --manifest-path tools/evolution-loop/Cargo.toml".to_owned(),
            ),
            validation_phase: Some("pre".to_owned()),
            validation_status_code: Some(0),
            validation_elapsed_ms: Some(123),
            validation_stdout_tail: Some("ok".to_owned()),
            validation_stderr_tail: Some("warn".to_owned()),
            self_improve_passed: Some(true),
            state_gate_checked: Some(false),
            state_gate_passed: Some(true),
            trace_gate_checked: Some(false),
            trace_gate_passed: Some(true),
            delta_chars: 7,
            stages: vec!["generate:start".to_owned()],
            meta: vec!["m".to_owned()],
            allocation_evidence: vec!["pool_route task_kind:review route_allowed:false".to_owned()],
            final_json: Some("{\"ok\":true}".to_owned()),
        });

        assert!(json.contains("\"round\":2"));
        assert!(json.contains("\"runtime_tokens\":42"));
        assert!(json.contains("\"runtime_model\":\"google/gemma\""));
        assert!(json.contains("\"answer\":\"answer\""));
        assert!(json.contains("\"feedback_applied\":2"));
        assert!(json.contains("\"rust_check_feedback_applied\":1"));
        assert!(json.contains("\"validation_checked\":true"));
        assert!(json.contains("\"validation_passed\":true"));
        assert!(json.contains("\"validation_command_source\":\"test-gate\""));
        assert!(json.contains("\"validation_command_safety\":\"safe\""));
        assert!(json.contains(
            "\"validation_command_preview\":\"cargo test --manifest-path tools/evolution-loop/Cargo.toml\""
        ));
        assert!(json.contains("\"validation_phase\":\"pre\""));
        assert!(json.contains("\"validation_status_code\":0"));
        assert!(json.contains("\"validation_elapsed_ms\":123"));
        assert!(json.contains("\"validation_stdout_tail\":\"ok\""));
        assert!(json.contains("\"validation_stderr_tail\":\"warn\""));
        assert!(json.contains("\"stages\":[\"generate:start\"]"));
        assert!(json.contains("\"helper_stage_feedback_by_role\":{}"));
        assert!(json.contains("\"helper_stage_contract_by_role\":{}"));
        assert!(json.contains(
            "\"allocation_evidence\":[\"pool_route task_kind:review route_allowed:false\"]"
        ));
    }

    #[test]
    fn record_json_indexes_helper_stage_feedback_by_role() {
        let json = record_json(&RoundRecord {
            round: 4,
            case_name: "case-4".to_owned(),
            prompt: "prompt".to_owned(),
            started_unix: 10,
            finished_unix: 20,
            success: true,
            error: None,
            runtime_tokens: Some(42),
            runtime_model: Some("google/gemma".to_owned()),
            answer: Some("answer".to_owned()),
            elapsed_ms: Some(99),
            business_cycle_passed: Some(true),
            feedback_applied: Some(2),
            rust_check_checked: Some(true),
            rust_check_passed: Some(true),
            rust_check_feedback_applied: Some(1),
            validation_checked: Some(true),
            validation_passed: Some(true),
            validation_command_source: None,
            validation_command_safety: None,
            validation_command_preview: None,
            validation_phase: None,
            validation_status_code: None,
            validation_elapsed_ms: None,
            validation_stdout_tail: None,
            validation_stderr_tail: None,
            self_improve_passed: Some(true),
            state_gate_checked: Some(false),
            state_gate_passed: Some(true),
            trace_gate_checked: Some(false),
            trace_gate_passed: Some(true),
            delta_chars: 7,
            stages: vec!["pool_stage_call:executed".to_owned()],
            meta: vec![
                "pool_stage_call_answer task_kind=summary role=summary elapsed_ms=111 answer_approx_tokens=4 preview=memory_update: keep Metal evidence".to_owned(),
                "pool_stage_call_answer task_kind=test-gate role=test-gate elapsed_ms=222 answer_approx_tokens=8 preview=validation_command: cargo test".to_owned(),
                "pool_stage_call_skipped task_kind=review role=review reason=busy".to_owned(),
            ],
            allocation_evidence: vec![],
            final_json: Some("{\"ok\":true}".to_owned()),
        });

        assert!(json.contains("\"helper_stage_feedback_by_role\":{\"summary\""));
        assert!(json.contains("\"summary\":[\"task_kind=summary elapsed_ms=111"));
        assert!(json.contains("memory_update: keep Metal evidence"));
        assert!(json.contains("\"test-gate\":[\"task_kind=test-gate elapsed_ms=222"));
        assert!(json.contains("validation_command: cargo test"));
        assert!(json.contains("\"helper_stage_contract_by_role\":{\"summary\""));
        assert!(json.contains("\"fields\":{\"memory_update\":\"keep Metal evidence\""));
        assert!(json.contains("\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test\""));
        assert!(!json.contains("\"review\":["));
    }

    #[test]
    fn record_json_exposes_report_only_eval_from_final_json() {
        let json = record_json(&RoundRecord {
            round: 3,
            case_name: "case-3".to_owned(),
            prompt: "prompt".to_owned(),
            started_unix: 10,
            finished_unix: 20,
            success: true,
            error: None,
            runtime_tokens: Some(42),
            runtime_model: Some("google/gemma".to_owned()),
            answer: Some("answer".to_owned()),
            elapsed_ms: Some(99),
            business_cycle_passed: Some(true),
            feedback_applied: Some(2),
            rust_check_checked: Some(true),
            rust_check_passed: Some(true),
            rust_check_feedback_applied: Some(1),
            validation_checked: Some(true),
            validation_passed: Some(true),
            validation_command_source: None,
            validation_command_safety: None,
            validation_command_preview: None,
            validation_phase: None,
            validation_status_code: None,
            validation_elapsed_ms: None,
            validation_stdout_tail: None,
            validation_stderr_tail: None,
            self_improve_passed: Some(true),
            state_gate_checked: Some(false),
            state_gate_passed: Some(true),
            trace_gate_checked: Some(false),
            trace_gate_passed: Some(true),
            delta_chars: 7,
            stages: vec!["generate:final".to_owned()],
            meta: vec![],
            allocation_evidence: vec![],
            final_json: Some(
                "{\"ok\":true,\"pool_stage_dispatch\":[{\"task_kind\":\"summary\"},{\"task_kind\":\"index\"}],\"eval\":{\"report_only\":true,\"failure_kind\":\"none\"}}"
                    .to_owned(),
            ),
        });

        assert!(json.contains(
            "\"final_json_pool_stage_dispatch\":[{\"task_kind\":\"summary\"},{\"task_kind\":\"index\"}]"
        ));
        assert!(json.contains("\"eval\":{\"report_only\":true,\"failure_kind\":\"none\"}"));
        assert!(json.contains("\"final_preview\""));
    }

    #[test]
    fn record_json_projects_live_reward_and_review_into_profile_replay() {
        let padding = "x".repeat(1_500);
        let json = record_json(&RoundRecord {
            round: 5,
            case_name: "live-reward".to_owned(),
            prompt: "review the current routing policy".to_owned(),
            started_unix: 10,
            finished_unix: 12,
            success: true,
            error: None,
            runtime_tokens: Some(42),
            runtime_model: Some("apple-quality".to_owned()),
            answer: Some("keep the candidate held".to_owned()),
            elapsed_ms: Some(900),
            business_cycle_passed: Some(true),
            feedback_applied: Some(2),
            rust_check_checked: Some(true),
            rust_check_passed: Some(true),
            rust_check_feedback_applied: Some(1),
            validation_checked: Some(true),
            validation_passed: Some(true),
            validation_command_source: Some("test-gate".to_owned()),
            validation_command_safety: Some("safe".to_owned()),
            validation_command_preview: Some(
                "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml".to_owned(),
            ),
            validation_phase: Some("post".to_owned()),
            validation_status_code: Some(0),
            validation_elapsed_ms: Some(123),
            validation_stdout_tail: Some("ok".to_owned()),
            validation_stderr_tail: None,
            self_improve_passed: Some(true),
            state_gate_checked: Some(true),
            state_gate_passed: Some(true),
            trace_gate_checked: Some(true),
            trace_gate_passed: Some(true),
            delta_chars: 24,
            stages: vec!["generate:final".to_owned()],
            meta: vec![
                "pool_stage_call_answer task_kind=review role=review elapsed_ms=9 answer_approx_tokens=12 preview=risk: latency regression / change_request: keep profile candidate held / verification: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"
                    .to_owned(),
            ],
            allocation_evidence: vec![],
            final_json: Some(format!(
                "{{\"ok\":true,\"pool_stage_dispatch\":[{{\"task_kind\":\"summary\",\"padding\":\"{padding}\"}}],\"generate\":{{\"quality\":0.8125,\"process_reward\":0.625,\"action\":\"reinforce\"}}}}"
            )),
        });

        assert!(json.contains("\"strategy\":\"single\""), "{json}");
        assert!(json.contains("\"quality_score\":0.8125"), "{json}");
        assert!(json.contains("\"process_reward\":0.625"), "{json}");
        assert!(json.contains("\"reward_action\":\"reinforce\""), "{json}");
        assert!(
            json.contains("\"reflection_risk\":\"latency regression\""),
            "{json}"
        );
        assert!(json.contains("\"drift_detected\":false"), "{json}");
        assert!(json.contains("\"cost\":42"), "{json}");
        assert!(json.contains("\"cost_basis\":\"runtime_tokens\""), "{json}");
        assert!(!json.contains("reward_placeholder"), "{json}");
        assert!(!json.contains("reflection_placeholder"), "{json}");

        let replay = crate::profile_scoring::OfflineReplayReport::from_outcome_jsonl(
            "round-ledger.jsonl",
            &json,
            1,
            &crate::profile_scoring::ScoringConfig::default(),
        );
        assert_eq!(replay.baseline.samples, 1);
        assert!((replay.baseline.quality_avg - 0.8125).abs() < f64::EPSILON);
        assert_eq!(replay.baseline.latency_avg_ms, 900.0);
        assert_eq!(replay.baseline.cost_avg, 42.0);
        assert_eq!(replay.baseline.drift_penalty, 0.0);
        assert_eq!(replay.candidate.samples, 0);
        assert!(!replay.allow_switch);
    }

    #[test]
    fn drift_projection_uses_reward_outcome_not_mandatory_review_risk() {
        assert_eq!(drift_detected_from_reward(Some("reinforce")), Some(false));
        assert_eq!(drift_detected_from_reward(Some("penalize")), Some(true));
        assert_eq!(drift_detected_from_reward(Some("hold")), None);
    }

    #[test]
    fn reads_next_round_from_existing_ledger() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-evolution-ledger-test-{}.jsonl",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            "{\"round\":1,\"success\":true}\n{\"round\":4,\"success\":false}\n",
        )
        .unwrap();

        assert_eq!(next_round(&path).unwrap(), 5);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_ledger_starts_at_round_one() {
        let path = std::env::temp_dir().join(format!(
            "smartsteam-evolution-missing-{}.jsonl",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        assert_eq!(next_round(&path).unwrap(), 1);
    }

    #[test]
    fn ledger_hygiene_detects_duplicate_non_monotonic_missing_and_gaps() {
        let hygiene = ledger_hygiene([Some(1), Some(3), Some(3), None, Some(2)]);

        assert_eq!(hygiene.records, 5);
        assert_eq!(hygiene.unique_rounds, 3);
        assert_eq!(hygiene.duplicate_rounds, 1);
        assert_eq!(hygiene.non_monotonic_rounds, 2);
        assert_eq!(hygiene.missing_rounds, 1);
        assert_eq!(hygiene.round_gaps, 0);

        let failures = hygiene.state_consistency_failures();
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("duplicate round"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("non-monotonic round"))
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("without valid round"))
        );
    }

    #[test]
    fn ledger_hygiene_detects_skipped_round_numbers() {
        let hygiene = ledger_hygiene([Some(1), Some(4)]);

        assert_eq!(hygiene.unique_rounds, 2);
        assert_eq!(hygiene.round_gaps, 2);
        assert!(
            hygiene
                .state_consistency_failures()
                .iter()
                .any(|failure| failure.contains("missing round number"))
        );
    }
}
