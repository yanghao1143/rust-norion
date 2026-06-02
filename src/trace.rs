use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use crate::engine::InferenceOutcome;
use crate::hierarchy::TaskProfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceSchemaGateReport {
    pub passed: bool,
    pub checked_lines: usize,
    pub failures: Vec<String>,
}

impl TraceSchemaGateReport {
    pub fn summary_line(&self) -> String {
        format!(
            "trace_schema_gate: passed={} lines={} failures={}",
            self.passed,
            self.checked_lines,
            self.failures.len()
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct TraceRequiredField {
    name: &'static str,
    marker: &'static str,
}

const TRACE_REQUIRED_FIELDS: &[TraceRequiredField] = &[
    TraceRequiredField {
        name: "schema",
        marker: "\"schema\":\"rust-norion-trace-v1\"",
    },
    TraceRequiredField {
        name: "case",
        marker: "\"case\":",
    },
    TraceRequiredField {
        name: "profile",
        marker: "\"profile\":",
    },
    TraceRequiredField {
        name: "reflection",
        marker: "\"reflection\":{",
    },
    TraceRequiredField {
        name: "revision_passes",
        marker: "\"revision_passes\":",
    },
    TraceRequiredField {
        name: "route",
        marker: "\"route\":{",
    },
    TraceRequiredField {
        name: "runtime_tokens",
        marker: "\"runtime_tokens\":{",
    },
    TraceRequiredField {
        name: "runtime_diagnostics",
        marker: "\"runtime_diagnostics\":{",
    },
    TraceRequiredField {
        name: "runtime_adapter_observations",
        marker: "\"runtime_adapter_observations\":{",
    },
    TraceRequiredField {
        name: "runtime_adapter_observation_count",
        marker: "\"observation_count\":",
    },
    TraceRequiredField {
        name: "runtime_adapter_best_adapter",
        marker: "\"best_adapter\":",
    },
    TraceRequiredField {
        name: "runtime_adapter_best_score",
        marker: "\"best_score\":",
    },
    TraceRequiredField {
        name: "forward_energy",
        marker: "\"forward_energy\":",
    },
    TraceRequiredField {
        name: "kv_influence",
        marker: "\"kv_influence\":",
    },
    TraceRequiredField {
        name: "uncertainty_perplexity",
        marker: "\"uncertainty_perplexity\":",
    },
    TraceRequiredField {
        name: "hierarchy",
        marker: "\"hierarchy\":{",
    },
    TraceRequiredField {
        name: "hardware",
        marker: "\"hardware\":{",
    },
    TraceRequiredField {
        name: "runtime_device_contract",
        marker: "\"runtime_device_contract\":",
    },
    TraceRequiredField {
        name: "adapter_hints",
        marker: "\"adapter_hints\":",
    },
    TraceRequiredField {
        name: "local_kv_token_budget",
        marker: "\"local_kv_token_budget\":",
    },
    TraceRequiredField {
        name: "global_kv_token_budget",
        marker: "\"global_kv_token_budget\":",
    },
    TraceRequiredField {
        name: "recursive",
        marker: "\"recursive\":{",
    },
    TraceRequiredField {
        name: "execution_waves",
        marker: "\"execution_waves\":",
    },
    TraceRequiredField {
        name: "recursive_runtime_calls",
        marker: "\"runtime_calls\":",
    },
    TraceRequiredField {
        name: "tiers",
        marker: "\"tiers\":{",
    },
    TraceRequiredField {
        name: "infini_memory",
        marker: "\"infini_memory\":{",
    },
    TraceRequiredField {
        name: "infini_local_window",
        marker: "\"local_window\":",
    },
    TraceRequiredField {
        name: "infini_global_memory",
        marker: "\"global_memory\":",
    },
    TraceRequiredField {
        name: "infini_sparse_skipped",
        marker: "\"sparse_skipped\":",
    },
    TraceRequiredField {
        name: "infini_skipped_tokens",
        marker: "\"skipped_tokens\":",
    },
    TraceRequiredField {
        name: "transformer",
        marker: "\"transformer\":{",
    },
    TraceRequiredField {
        name: "toolsmith",
        marker: "\"toolsmith\":{",
    },
    TraceRequiredField {
        name: "toolsmith_blueprints",
        marker: "\"blueprint_summaries\":",
    },
    TraceRequiredField {
        name: "toolsmith_gate",
        marker: "\"gate_passed\":",
    },
    TraceRequiredField {
        name: "agent_team",
        marker: "\"agent_team\":{",
    },
    TraceRequiredField {
        name: "agent_team_isolation",
        marker: "\"isolation\":{",
    },
    TraceRequiredField {
        name: "agent_team_messages",
        marker: "\"messages\":",
    },
    TraceRequiredField {
        name: "agent_team_conflicts",
        marker: "\"conflicts\":",
    },
    TraceRequiredField {
        name: "agent_team_evolution",
        marker: "\"evolution_signals\":",
    },
    TraceRequiredField {
        name: "agent_team_collision_free",
        marker: "\"collision_free\":",
    },
    TraceRequiredField {
        name: "memory",
        marker: "\"memory\":{",
    },
    TraceRequiredField {
        name: "runtime_kv_exported",
        marker: "\"runtime_kv_exported\":",
    },
    TraceRequiredField {
        name: "runtime_kv_stored",
        marker: "\"runtime_kv_stored\":",
    },
    TraceRequiredField {
        name: "memory_feedback_reinforced",
        marker: "\"feedback_reinforced\":",
    },
    TraceRequiredField {
        name: "memory_feedback_penalized",
        marker: "\"feedback_penalized\":",
    },
    TraceRequiredField {
        name: "memory_feedback_reinforcement_amount",
        marker: "\"feedback_reinforcement_amount\":",
    },
    TraceRequiredField {
        name: "memory_feedback_penalty_amount",
        marker: "\"feedback_penalty_amount\":",
    },
    TraceRequiredField {
        name: "drift",
        marker: "\"drift\":{",
    },
    TraceRequiredField {
        name: "rollback_adaptive",
        marker: "\"rollback_adaptive\":",
    },
    TraceRequiredField {
        name: "process_reward",
        marker: "\"process_reward\":{",
    },
    TraceRequiredField {
        name: "auto_replay",
        marker: "\"auto_replay\":{",
    },
    TraceRequiredField {
        name: "auto_replay_router_updates",
        marker: "\"router_updates\":",
    },
    TraceRequiredField {
        name: "auto_replay_hierarchy_updates",
        marker: "\"hierarchy_updates\":",
    },
    TraceRequiredField {
        name: "auto_replay_router_threshold_mutations",
        marker: "\"router_threshold_mutations\":",
    },
    TraceRequiredField {
        name: "auto_replay_hierarchy_weight_mutations",
        marker: "\"hierarchy_weight_mutations\":",
    },
    TraceRequiredField {
        name: "auto_replay_router_threshold_delta",
        marker: "\"router_threshold_delta\":",
    },
    TraceRequiredField {
        name: "auto_replay_hierarchy_weight_delta",
        marker: "\"hierarchy_weight_delta\":",
    },
    TraceRequiredField {
        name: "auto_replay_memory_reinforcements",
        marker: "\"memory_reinforcements\":",
    },
    TraceRequiredField {
        name: "auto_replay_memory_penalties",
        marker: "\"memory_penalties\":",
    },
    TraceRequiredField {
        name: "auto_replay_live_memory_feedback_items",
        marker: "\"live_memory_feedback_items\":",
    },
    TraceRequiredField {
        name: "auto_replay_live_memory_feedback_updates",
        marker: "\"live_memory_feedback_updates\":",
    },
    TraceRequiredField {
        name: "auto_replay_live_memory_feedback_reinforcements",
        marker: "\"live_memory_feedback_reinforcements\":",
    },
    TraceRequiredField {
        name: "auto_replay_live_memory_feedback_penalties",
        marker: "\"live_memory_feedback_penalties\":",
    },
    TraceRequiredField {
        name: "auto_replay_recursive_runtime_calls",
        marker: "\"recursive_runtime_calls\":",
    },
    TraceRequiredField {
        name: "auto_replay_recursive_call_pressure",
        marker: "\"max_recursive_call_pressure\":",
    },
    TraceRequiredField {
        name: "evolution_ledger",
        marker: "\"evolution_ledger\":{",
    },
    TraceRequiredField {
        name: "evolution_replay_runs",
        marker: "\"replay_runs\":",
    },
    TraceRequiredField {
        name: "evolution_live_inference_runs",
        marker: "\"live_inference_runs\":",
    },
    TraceRequiredField {
        name: "evolution_live_memory_updates",
        marker: "\"cumulative_live_memory_updates\":",
    },
    TraceRequiredField {
        name: "evolution_live_stored_memory_updates",
        marker: "\"cumulative_live_stored_memory_updates\":",
    },
    TraceRequiredField {
        name: "evolution_router_threshold_mutations",
        marker: "\"cumulative_router_threshold_mutations\":",
    },
    TraceRequiredField {
        name: "evolution_hierarchy_weight_mutations",
        marker: "\"cumulative_hierarchy_weight_mutations\":",
    },
    TraceRequiredField {
        name: "evolution_memory_updates",
        marker: "\"cumulative_memory_updates\":",
    },
    TraceRequiredField {
        name: "evolution_replay_live_memory_feedback_updates",
        marker: "\"cumulative_replay_live_memory_feedback_updates\":",
    },
    TraceRequiredField {
        name: "evolution_recursive_runtime_calls",
        marker: "\"cumulative_recursive_runtime_calls\":",
    },
    TraceRequiredField {
        name: "evolution_drift_rollbacks",
        marker: "\"cumulative_drift_rollbacks\":",
    },
    TraceRequiredField {
        name: "evolution_rollback_router_threshold_delta",
        marker: "\"cumulative_rollback_router_threshold_delta\":",
    },
    TraceRequiredField {
        name: "evolution_rollback_hierarchy_weight_delta",
        marker: "\"cumulative_rollback_hierarchy_weight_delta\":",
    },
    TraceRequiredField {
        name: "retention",
        marker: "\"retention\":{",
    },
    TraceRequiredField {
        name: "remove_below_strength",
        marker: "\"remove_below_strength\":",
    },
    TraceRequiredField {
        name: "remove_after_failures",
        marker: "\"remove_after_failures\":",
    },
    TraceRequiredField {
        name: "memory_compaction",
        marker: "\"memory_compaction\":{",
    },
    TraceRequiredField {
        name: "similarity_threshold",
        marker: "\"similarity_threshold\":",
    },
    TraceRequiredField {
        name: "max_merges",
        marker: "\"max_merges\":",
    },
    TraceRequiredField {
        name: "experience_id",
        marker: "\"experience_id\":",
    },
];

pub fn evaluate_trace_schema_jsonl(path: impl AsRef<Path>) -> io::Result<TraceSchemaGateReport> {
    let content = fs::read_to_string(path)?;
    let mut checked_lines = 0;
    let mut failures = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        checked_lines += 1;
        failures.extend(
            evaluate_trace_schema_line(line)
                .into_iter()
                .map(|failure| format!("line {}: {failure}", index + 1)),
        );
    }

    if checked_lines == 0 {
        failures.push("trace file did not contain any non-empty JSONL records".to_owned());
    }

    Ok(TraceSchemaGateReport {
        passed: failures.is_empty(),
        checked_lines,
        failures,
    })
}

pub fn evaluate_trace_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let line = line.trim();

    if !line.starts_with('{') || !line.ends_with('}') {
        failures.push("record is not a single JSON object line".to_owned());
    }

    for field in TRACE_REQUIRED_FIELDS {
        if !line.contains(field.marker) {
            failures.push(format!("missing trace field {}", field.name));
        }
    }

    failures.extend(evaluate_trace_device_contract(line));
    failures.extend(evaluate_trace_adapter_observations(line));
    failures.extend(evaluate_trace_runtime_kv(line));

    failures
}

fn evaluate_trace_runtime_kv(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let runtime_kv_exported = extract_json_usize_field(line, "runtime_kv_exported").unwrap_or(0);
    let runtime_kv_stored = extract_json_usize_field(line, "runtime_kv_stored").unwrap_or(0);
    let diagnostic_exported = extract_json_usize_field(line, "exported_kv_blocks").unwrap_or(0);
    let memory_write = extract_json_bool_field(line, "memory_write").unwrap_or(false);
    let runtime_kv_write = extract_json_bool_field(line, "runtime_kv_write").unwrap_or(false);
    let revision_passes = extract_json_usize_field(line, "revision_passes").unwrap_or(0);

    if diagnostic_exported != runtime_kv_exported {
        failures.push(format!(
            "runtime_diagnostics exported_kv_blocks {diagnostic_exported} does not match memory runtime_kv_exported {runtime_kv_exported}"
        ));
    }

    if runtime_kv_stored > runtime_kv_exported {
        failures.push(format!(
            "runtime_kv_stored {runtime_kv_stored} exceeds runtime_kv_exported {runtime_kv_exported}"
        ));
    }

    if runtime_kv_stored > 0 && !runtime_kv_write {
        failures.push(format!(
            "runtime_kv_stored {runtime_kv_stored} requires runtime_kv_write=true"
        ));
    }

    if runtime_kv_stored > 0 && !memory_write {
        failures.push(format!(
            "runtime_kv_stored {runtime_kv_stored} requires memory_write=true"
        ));
    }

    if runtime_kv_stored > 0 && revision_passes > 0 {
        failures.push(format!(
            "runtime_kv_stored {runtime_kv_stored} requires revision_passes=0"
        ));
    }

    failures
}

fn evaluate_trace_adapter_observations(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let observation_count = extract_json_usize_field(line, "observation_count").unwrap_or(0);
    let best_adapter = extract_json_nullable_string_field(line, "best_adapter");
    let best_score = extract_json_nullable_f32_field(line, "best_score");
    let best_reward = extract_json_nullable_f32_field(line, "best_reward");
    let best_quality = extract_json_nullable_f32_field(line, "best_quality");
    let best_experience_id = extract_json_nullable_u64_field(line, "best_experience_id");

    if observation_count == 0 {
        if best_adapter.is_some()
            || best_score.is_some()
            || best_reward.is_some()
            || best_quality.is_some()
            || best_experience_id.is_some()
        {
            failures.push(
                "runtime_adapter_observations count is zero but best observation fields are populated"
                    .to_owned(),
            );
        }
        return failures;
    }

    let Some(best_adapter) = best_adapter else {
        failures.push(
            "runtime_adapter_observations count is positive but best_adapter is missing".to_owned(),
        );
        return failures;
    };

    for (name, value) in [
        ("best_score", best_score),
        ("best_reward", best_reward),
        ("best_quality", best_quality),
    ] {
        match value {
            Some(value) if (0.0..=1.0).contains(&value) => {}
            Some(value) => failures.push(format!(
                "runtime_adapter_observations {name} {value:.3} is outside 0..1"
            )),
            None => failures.push(format!(
                "runtime_adapter_observations count is positive but {name} is missing"
            )),
        }
    }

    if best_experience_id.is_none() {
        failures.push(
            "runtime_adapter_observations count is positive but best_experience_id is missing"
                .to_owned(),
        );
    }

    let adapter_hints = extract_json_string_array_field(line, "adapter_hints").unwrap_or_default();
    if !adapter_hints.iter().any(|adapter| adapter == &best_adapter) {
        failures.push(format!(
            "best_adapter {best_adapter} is outside trace adapter_hints"
        ));
    }

    if let Some(contract) = extract_json_string_field(line, "runtime_device_contract") {
        let contract_adapters = contract_value(&contract, "adapters")
            .map(split_contract_adapters)
            .unwrap_or_default();
        if !contract_adapters
            .iter()
            .any(|adapter| adapter == &best_adapter)
        {
            failures.push(format!(
                "best_adapter {best_adapter} is outside runtime_device_contract adapters"
            ));
        }
    }

    failures
}

fn evaluate_trace_device_contract(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(contract) = extract_json_string_field(line, "runtime_device_contract") else {
        return failures;
    };

    require_contract_string(
        &mut failures,
        &contract,
        "device",
        extract_json_string_field(line, "device"),
    );
    require_contract_string(
        &mut failures,
        &contract,
        "tier",
        extract_json_string_field(line, "tier"),
    );
    require_contract_string(
        &mut failures,
        &contract,
        "primary",
        extract_json_string_field(line, "primary_lane"),
    );
    require_contract_string(
        &mut failures,
        &contract,
        "fallback",
        extract_json_string_field(line, "fallback_lane"),
    );
    require_contract_string(
        &mut failures,
        &contract,
        "memory",
        extract_json_string_field(line, "memory_mode"),
    );
    require_contract_usize(
        &mut failures,
        &contract,
        "parallel_chunks",
        extract_json_usize_field(line, "max_parallel_chunks"),
    );
    require_contract_usize(
        &mut failures,
        &contract,
        "kv_prefetch",
        extract_json_usize_field(line, "kv_prefetch_blocks"),
    );
    require_contract_string(
        &mut failures,
        &contract,
        "kv_bits",
        match (
            extract_json_usize_field(line, "hot_kv_bits"),
            extract_json_usize_field(line, "cold_kv_bits"),
        ) {
            (Some(hot), Some(cold)) => Some(format!("{hot}/{cold}")),
            _ => None,
        },
    );
    require_contract_string(
        &mut failures,
        &contract,
        "disk_spill",
        extract_json_bool_field(line, "disk_spill").map(|value| value.to_string()),
    );
    require_contract_usize(
        &mut failures,
        &contract,
        "local_kv_tokens",
        extract_json_usize_field(line, "local_kv_token_budget"),
    );
    require_contract_usize(
        &mut failures,
        &contract,
        "global_kv_tokens",
        extract_json_usize_field(line, "global_kv_token_budget"),
    );

    let adapter_hints = extract_json_string_array_field(line, "adapter_hints").unwrap_or_default();
    if adapter_hints.is_empty() {
        failures.push("adapter_hints must not be empty".to_owned());
    }
    let contract_adapters = contract_value(&contract, "adapters")
        .map(split_contract_adapters)
        .unwrap_or_default();
    if contract_adapters.is_empty() {
        failures.push("runtime_device_contract missing adapters list".to_owned());
    }
    for adapter in &adapter_hints {
        if !contract_adapters
            .iter()
            .any(|contract_adapter| contract_adapter == adapter)
        {
            failures.push(format!(
                "runtime_device_contract adapters missing trace adapter_hint {adapter}"
            ));
        }
    }

    if let Some(selected_adapter) = extract_json_string_field(line, "selected_adapter") {
        if !adapter_hints
            .iter()
            .any(|adapter| adapter == &selected_adapter)
        {
            failures.push(format!(
                "selected_adapter {selected_adapter} is outside trace adapter_hints"
            ));
        }
        if !contract_adapters
            .iter()
            .any(|adapter| adapter == &selected_adapter)
        {
            failures.push(format!(
                "selected_adapter {selected_adapter} is outside runtime_device_contract adapters"
            ));
        }
    }

    failures
}

fn require_contract_string(
    failures: &mut Vec<String>,
    contract: &str,
    key: &str,
    expected: Option<String>,
) {
    let Some(expected) = expected else {
        return;
    };
    match contract_value(contract, key) {
        Some(actual) if actual == expected => {}
        Some(actual) => failures.push(format!(
            "runtime_device_contract {key}={actual} does not match trace value {expected}"
        )),
        None => failures.push(format!("runtime_device_contract missing {key}")),
    }
}

fn require_contract_usize(
    failures: &mut Vec<String>,
    contract: &str,
    key: &str,
    expected: Option<usize>,
) {
    require_contract_string(
        failures,
        contract,
        key,
        expected.map(|value| value.to_string()),
    );
}

fn contract_value<'a>(contract: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    contract
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&prefix))
}

fn split_contract_adapters(value: &str) -> Vec<String> {
    value
        .split('+')
        .filter(|item| !item.trim().is_empty())
        .map(|item| item.trim().to_owned())
        .collect()
}

fn extract_json_string_field(line: &str, field: &str) -> Option<String> {
    let value = value_after_json_field(line, field)?;
    parse_json_string(value).map(|(parsed, _)| parsed)
}

fn extract_json_nullable_string_field(line: &str, field: &str) -> Option<String> {
    let value = value_after_json_field(line, field)?;
    if value.starts_with("null") {
        return None;
    }
    parse_json_string(value).map(|(parsed, _)| parsed)
}

fn extract_json_usize_field(line: &str, field: &str) -> Option<usize> {
    let value = value_after_json_field(line, field)?;
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn extract_json_nullable_u64_field(line: &str, field: &str) -> Option<u64> {
    let value = value_after_json_field(line, field)?;
    if value.starts_with("null") {
        return None;
    }
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn extract_json_nullable_f32_field(line: &str, field: &str) -> Option<f32> {
    let value = value_after_json_field(line, field)?;
    if value.starts_with("null") {
        return None;
    }
    let number = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || matches!(ch, '-' | '+' | '.' | 'e' | 'E'))
        .collect::<String>();
    if number.is_empty() {
        return None;
    }
    number.parse::<f32>().ok().filter(|value| value.is_finite())
}

fn extract_json_bool_field(line: &str, field: &str) -> Option<bool> {
    let value = value_after_json_field(line, field)?;
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn extract_json_string_array_field(line: &str, field: &str) -> Option<Vec<String>> {
    let mut value = value_after_json_field(line, field)?;
    value = value.strip_prefix('[')?.trim_start();
    let mut out = Vec::new();

    loop {
        if let Some(rest) = value.strip_prefix(']') {
            let _ = rest;
            return Some(out);
        }

        let (item, consumed) = parse_json_string(value)?;
        out.push(item);
        value = value[consumed..].trim_start();

        if let Some(rest) = value.strip_prefix(',') {
            value = rest.trim_start();
        } else if value.starts_with(']') {
            continue;
        } else {
            return None;
        }
    }
}

fn value_after_json_field<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    let marker = format!("\"{field}\":");
    let start = line.find(&marker)? + marker.len();
    Some(line[start..].trim_start())
}

fn parse_json_string(value: &str) -> Option<(String, usize)> {
    let mut chars = value.char_indices();
    let (_, first) = chars.next()?;
    if first != '"' {
        return None;
    }

    let mut out = String::new();
    let mut escaped = false;
    for (index, ch) in chars {
        if escaped {
            match ch {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'b' => out.push('\u{0008}'),
                'f' => out.push('\u{000c}'),
                'u' => out.push_str("\\u"),
                other => out.push(other),
            }
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => return Some((out, index + ch.len_utf8())),
            other => out.push(other),
        }
    }

    None
}

pub fn trace_json_line(
    prompt: &str,
    profile: TaskProfile,
    elapsed_ms: u128,
    outcome: &InferenceOutcome,
) -> String {
    trace_json_line_with_case(None, prompt, profile, elapsed_ms, outcome)
}

pub fn trace_json_line_with_case(
    case_name: Option<&str>,
    prompt: &str,
    profile: TaskProfile,
    elapsed_ms: u128,
    outcome: &InferenceOutcome,
) -> String {
    let tier_counts = outcome.tier_plan.counts();
    let infini_counts = outcome.infini_memory_plan.counts();
    let transformer_counts = outcome.transformer_plan.counts();
    let adapter_hints = outcome
        .hardware_plan
        .execution
        .adapter_hints
        .iter()
        .map(|adapter| adapter.as_str().to_owned())
        .collect::<Vec<_>>();
    let reflection_issue_codes = outcome.report.issue_codes();
    let auto_replay = outcome.auto_replay_report.as_ref();
    let best_adapter_observation = outcome.runtime_adapter_observations.first();
    let toolsmith_blueprints = outcome
        .toolsmith_plan
        .blueprints
        .iter()
        .map(|blueprint| blueprint.summary())
        .collect::<Vec<_>>();
    let agent_team_messages = outcome.agent_team_plan.message_summaries(16);
    let agent_team_conflicts = outcome.agent_team_plan.conflict_summaries(8);
    let agent_team_evolution = outcome.agent_team_plan.evolution_summaries(8);

    format!(
        "{{\
         \"schema\":\"rust-norion-trace-v1\",\
         \"case\":{},\
         \"profile\":\"{:?}\",\
         \"prompt_chars\":{},\
         \"prompt_preview\":\"{}\",\
         \"elapsed_ms\":{},\
         \"quality\":{:.6},\
         \"perplexity\":{:.6},\
         \"reflection\":{{\"issues\":{},\"critical_issues\":{},\"max_severity\":\"{}\",\"issue_codes\":{},\"revision_actions\":{},\"revision_passes\":{}}},\
         \"router_threshold_after\":{:.6},\
         \"route\":{{\"threshold\":{:.6},\"attention_fraction\":{:.6},\"attention_tokens\":{},\"fast_tokens\":{}}},\
         \"runtime_tokens\":{{\"token_count\":{},\"entropy_count\":{},\"logprob_count\":{},\"average_entropy\":{},\"average_neg_logprob\":{},\"uncertainty_perplexity\":{},\"has_uncertainty_signal\":{}}},\
         \"runtime_diagnostics\":{{\"model_id\":{},\"selected_adapter\":{},\"layer_count\":{},\"hidden_size\":{},\"local_window_tokens\":{},\"forward_energy\":{},\"kv_influence\":{},\"imported_kv_blocks\":{},\"exported_kv_blocks\":{},\"has_forward_signal\":{}}},\
         \"runtime_adapter_observations\":{{\"observation_count\":{},\"best_adapter\":{},\"best_score\":{},\"best_reward\":{},\"best_quality\":{},\"best_forward_energy\":{},\"best_kv_influence\":{},\"best_experience_id\":{}}},\
         \"hierarchy\":{{\"global\":{:.6},\"local\":{:.6},\"convolution\":{:.6}}},\
         \"hardware\":{{\"device\":\"{}\",\"tier\":\"{}\",\"pressure\":{:.6},\"runtime_device_contract\":\"{}\",\"latency_budget_ms\":{},\"local_kv_token_budget\":{},\"global_kv_token_budget\":{},\"execution\":{{\"primary_lane\":\"{}\",\"fallback_lane\":\"{}\",\"memory_mode\":\"{}\",\"max_parallel_chunks\":{},\"kv_prefetch_blocks\":{},\"hot_kv_bits\":{},\"cold_kv_bits\":{},\"disk_spill\":{},\"adapter_hints\":{}}}}},\
         \"recursive\":{{\"required\":{},\"prompt_tokens\":{},\"native_window\":{},\"chunks\":{},\"merge_rounds\":{},\"execution_waves\":{},\"max_parallel_chunks\":{},\"chunk_tokens\":{},\"overlap_tokens\":{},\"runtime_calls\":{}}},\
         \"tiers\":{{\"hot_gpu\":{},\"warm_ram\":{},\"cold_disk\":{}}},\
         \"infini_memory\":{{\"local_window\":{},\"global_memory\":{},\"sparse_skipped\":{},\"local_tokens\":{},\"global_tokens\":{},\"skipped_tokens\":{}}},\
         \"transformer\":{{\"template\":\"{}\",\"global\":{},\"local\":{},\"convolution\":{}}},\
         \"toolsmith\":{{\"rust_only\":{},\"exploration_required\":{},\"blueprints\":{},\"ready\":{},\"held\":{},\"rejected\":{},\"gate_passed\":{},\"notes\":{},\"rejected_requests\":{},\"blueprint_summaries\":{}}},\
         \"agent_team\":{{\"enabled\":{},\"summary\":\"{}\",\"run_id\":\"{}\",\"main_thread_goal\":\"{}\",\"agents\":{},\"messages\":{},\"conflicts\":{},\"unresolved_conflicts\":{},\"evolution_signals\":{},\"collision_free\":{},\"isolation\":{{\"single_writer\":{},\"read_only_subagents\":{},\"namespace\":\"{}\",\"allowed_outputs\":{},\"denied_capabilities\":{}}},\"message_summaries\":{},\"conflict_summaries\":{},\"evolution_summaries\":{}}},\
         \"stream_windows\":{},\
         \"memory\":{{\"used\":{},\"stored\":{},\"gist_records\":{},\"gist_stored\":{},\"runtime_kv_exported\":{},\"runtime_kv_stored\":{},\"feedback_reinforced\":{},\"feedback_penalized\":{},\"feedback_reinforcement_amount\":{:.6},\"feedback_penalty_amount\":{:.6}}},\
         \"drift\":{{\"severity\":\"{}\",\"memory_write\":{},\"runtime_kv_write\":{},\"penalize_used_memory\":{},\"rollback_adaptive\":{},\"notes\":{}}},\
         \"process_reward\":{{\"total\":{:.6},\"action\":\"{}\",\"route\":{:.6},\"memory\":{:.6},\"hierarchy\":{:.6},\"reflection\":{:.6},\"latency\":{:.6},\"admission\":{:.6},\"notes\":{}}},\
         \"auto_replay\":{{\"applied\":{},\"router_updates\":{},\"hierarchy_updates\":{},\"router_threshold_mutations\":{},\"hierarchy_weight_mutations\":{},\"router_threshold_delta\":{:.6},\"hierarchy_weight_delta\":{:.6},\"reinforced\":{},\"penalized\":{},\"touched_memories\":{},\"memory_reinforcements\":{},\"memory_penalties\":{},\"live_memory_feedback_items\":{},\"live_memory_feedback_updates\":{},\"live_memory_feedback_reinforcements\":{},\"live_memory_feedback_penalties\":{},\"recursive_runtime_items\":{},\"recursive_runtime_calls\":{},\"avg_recursive_call_pressure\":{:.6},\"max_recursive_call_pressure\":{:.6}}},\
         \"evolution_ledger\":{{\"live_inference_runs\":{},\"cumulative_live_router_threshold_mutations\":{},\"cumulative_live_hierarchy_weight_mutations\":{},\"cumulative_live_router_threshold_delta\":{:.6},\"cumulative_live_hierarchy_weight_delta\":{:.6},\"cumulative_live_memory_reinforcements\":{},\"cumulative_live_memory_penalties\":{},\"cumulative_live_memory_updates\":{},\"cumulative_live_stored_memories\":{},\"cumulative_live_stored_gist_memories\":{},\"cumulative_live_stored_runtime_kv_memories\":{},\"cumulative_live_stored_memory_updates\":{},\"cumulative_live_reflection_issues\":{},\"cumulative_live_critical_reflection_issues\":{},\"cumulative_live_revision_actions\":{},\"replay_runs\":{},\"replay_items\":{},\"cumulative_router_threshold_mutations\":{},\"cumulative_hierarchy_weight_mutations\":{},\"cumulative_router_threshold_delta\":{:.6},\"cumulative_hierarchy_weight_delta\":{:.6},\"cumulative_memory_reinforcements\":{},\"cumulative_memory_penalties\":{},\"cumulative_memory_updates\":{},\"cumulative_replay_live_memory_feedback_items\":{},\"cumulative_replay_live_memory_feedback_updates\":{},\"cumulative_replay_live_memory_feedback_reinforcements\":{},\"cumulative_replay_live_memory_feedback_penalties\":{},\"cumulative_recursive_replay_items\":{},\"cumulative_recursive_runtime_calls\":{},\"cumulative_drift_rollbacks\":{},\"cumulative_rollback_router_threshold_delta\":{:.6},\"cumulative_rollback_hierarchy_weight_delta\":{:.6}}},\
         \"retention\":{{\"stale_after\":{},\"decay_rate\":{:.6},\"remove_below_strength\":{:.6},\"remove_after_failures\":{},\"before\":{},\"after\":{},\"decayed\":{},\"removed\":{}}},\
         \"memory_compaction\":{{\"similarity_threshold\":{:.6},\"max_candidates\":{},\"max_merges\":{},\"before\":{},\"after\":{},\"merged\":{},\"removed\":{}}},\
         \"experience_id\":{}\
         }}",
        option_string_json(case_name),
        profile,
        prompt.chars().count(),
        json_escape(&compact(prompt, 160)),
        elapsed_ms,
        outcome.report.quality,
        outcome.metrics.perplexity,
        outcome.report.issues.len(),
        outcome.report.critical_issue_count(),
        outcome.report.max_severity().as_str(),
        string_array_json(&reflection_issue_codes),
        string_array_json(&outcome.report.revision_actions),
        outcome.report.revision_passes,
        outcome.router_threshold_after,
        outcome.route_budget.threshold,
        outcome.route_budget.attention_fraction,
        outcome.route_budget.attention_tokens,
        outcome.route_budget.fast_tokens,
        outcome.runtime_token_metrics.token_count,
        outcome.runtime_token_metrics.entropy_count,
        outcome.runtime_token_metrics.logprob_count,
        option_f32_json(outcome.runtime_token_metrics.average_entropy),
        option_f32_json(outcome.runtime_token_metrics.average_neg_logprob),
        option_f32_json(outcome.runtime_token_metrics.uncertainty_perplexity),
        outcome.runtime_token_metrics.has_uncertainty_signal(),
        option_owned_string_json(outcome.runtime_diagnostics.model_id.as_deref()),
        option_owned_string_json(outcome.runtime_diagnostics.selected_adapter.as_deref()),
        outcome.runtime_diagnostics.layer_count,
        outcome.runtime_diagnostics.hidden_size,
        outcome.runtime_diagnostics.local_window_tokens,
        option_f32_json(outcome.runtime_diagnostics.forward_energy),
        option_f32_json(outcome.runtime_diagnostics.kv_influence),
        outcome.runtime_diagnostics.imported_kv_blocks,
        outcome.runtime_diagnostics.exported_kv_blocks,
        outcome.runtime_diagnostics.has_forward_signal(),
        outcome.runtime_adapter_observations.len(),
        option_owned_string_json(
            best_adapter_observation.map(|observation| observation.adapter.as_str())
        ),
        option_f32_json(best_adapter_observation.map(|observation| observation.score)),
        option_f32_json(best_adapter_observation.map(|observation| observation.reward)),
        option_f32_json(best_adapter_observation.map(|observation| observation.quality)),
        option_f32_json(
            best_adapter_observation.and_then(|observation| observation.forward_energy)
        ),
        option_f32_json(best_adapter_observation.and_then(|observation| observation.kv_influence)),
        option_u64_json(best_adapter_observation.map(|observation| observation.experience_id)),
        outcome.hierarchy.global,
        outcome.hierarchy.local,
        outcome.hierarchy.convolution,
        outcome.hardware_plan.device.as_str(),
        outcome.hardware_plan.tier.as_str(),
        outcome.hardware_plan.pressure,
        json_escape(&outcome.hardware_plan.runtime_contract_summary()),
        option_u64_json(outcome.hardware_plan.latency_budget_ms),
        outcome.hardware_plan.local_kv_token_budget,
        outcome.hardware_plan.global_kv_token_budget,
        outcome.hardware_plan.execution.primary_lane.as_str(),
        outcome.hardware_plan.execution.fallback_lane.as_str(),
        outcome.hardware_plan.execution.memory_mode.as_str(),
        outcome.hardware_plan.execution.max_parallel_chunks,
        outcome.hardware_plan.execution.kv_prefetch_blocks,
        outcome.hardware_plan.execution.hot_kv_precision_bits,
        outcome.hardware_plan.execution.cold_kv_precision_bits,
        outcome.hardware_plan.execution.allow_disk_spill,
        string_array_json(&adapter_hints),
        outcome.recursive_schedule.requires_recursion,
        outcome.recursive_schedule.prompt_tokens,
        outcome.recursive_schedule.native_window_tokens,
        outcome.recursive_schedule.chunk_count(),
        outcome.recursive_schedule.merge_round_count(),
        outcome.recursive_schedule.execution_wave_count(),
        outcome.recursive_schedule.max_parallel_chunks,
        outcome.recursive_schedule.chunk_tokens,
        outcome.recursive_schedule.overlap_tokens,
        outcome.recursive_runtime_calls,
        tier_counts.hot_gpu,
        tier_counts.warm_ram,
        tier_counts.cold_disk,
        infini_counts.local_window,
        infini_counts.global_memory,
        infini_counts.skipped,
        infini_counts.local_tokens,
        infini_counts.global_tokens,
        infini_counts.skipped_tokens,
        json_escape(outcome.transformer_plan.template_name()),
        transformer_counts.global,
        transformer_counts.local,
        transformer_counts.convolution,
        outcome.toolsmith_plan.rust_only,
        outcome.toolsmith_plan.exploration_required,
        outcome.toolsmith_plan.blueprint_count(),
        outcome.toolsmith_plan.ready_count(),
        outcome.toolsmith_plan.held_count(),
        outcome.toolsmith_plan.rejected_count(),
        outcome.toolsmith_plan.passed_rust_gate(),
        string_array_json(&outcome.toolsmith_plan.notes),
        string_array_json(&outcome.toolsmith_plan.rejected_requests),
        string_array_json(&toolsmith_blueprints),
        outcome.agent_team_plan.enabled,
        json_escape(&outcome.agent_team_plan.summary()),
        json_escape(&outcome.agent_team_plan.run_id),
        json_escape(&outcome.agent_team_plan.main_thread_goal),
        outcome.agent_team_plan.active_agent_count(),
        outcome.agent_team_plan.message_count(),
        outcome.agent_team_plan.conflict_count(),
        outcome.agent_team_plan.unresolved_conflict_count(),
        outcome.agent_team_plan.evolution_signal_count(),
        outcome.agent_team_plan.collision_free(),
        outcome.agent_team_plan.isolation.single_writer,
        outcome.agent_team_plan.isolation.read_only_subagents,
        json_escape(&outcome.agent_team_plan.isolation.namespace),
        string_array_json(&outcome.agent_team_plan.isolation.allowed_outputs),
        string_array_json(&outcome.agent_team_plan.isolation.denied_capabilities),
        string_array_json(&agent_team_messages),
        string_array_json(&agent_team_conflicts),
        string_array_json(&agent_team_evolution),
        outcome.stream_reports.len(),
        outcome.used_memories.len(),
        option_u64_json(outcome.stored_memory_id),
        outcome.gist_records.len(),
        outcome.stored_gist_memory_ids.len(),
        outcome.exported_runtime_kv_blocks,
        outcome.stored_runtime_kv_memory_ids.len(),
        outcome.memory_feedback.reinforced,
        outcome.memory_feedback.penalized,
        outcome.memory_feedback.reinforcement_amount,
        outcome.memory_feedback.penalty_amount,
        outcome.drift_report.severity.as_str(),
        outcome.drift_report.allow_memory_write,
        outcome.drift_report.allow_runtime_kv_write,
        outcome.drift_report.penalize_used_memory,
        outcome.drift_report.rollback_adaptive,
        string_array_json(&outcome.drift_report.notes),
        outcome.process_reward.total,
        outcome.process_reward.action.as_str(),
        outcome.process_reward.components.route,
        outcome.process_reward.components.memory,
        outcome.process_reward.components.hierarchy,
        outcome.process_reward.components.reflection,
        outcome.process_reward.components.latency,
        outcome.process_reward.components.admission,
        string_array_json(&outcome.process_reward.notes),
        auto_replay.map(|report| report.applied).unwrap_or(0),
        auto_replay.map(|report| report.router_updates).unwrap_or(0),
        auto_replay
            .map(|report| report.hierarchy_updates)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.router_threshold_mutations)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.hierarchy_weight_mutations)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.router_threshold_delta)
            .unwrap_or(0.0),
        auto_replay
            .map(|report| report.hierarchy_weight_delta)
            .unwrap_or(0.0),
        auto_replay.map(|report| report.reinforced).unwrap_or(0),
        auto_replay.map(|report| report.penalized).unwrap_or(0),
        auto_replay
            .map(|report| report.touched_memories)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.memory_reinforcements)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.memory_penalties)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.live_memory_feedback_items)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.live_memory_feedback_updates)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.live_memory_feedback_reinforcements)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.live_memory_feedback_penalties)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.recursive_runtime_items)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.recursive_runtime_calls)
            .unwrap_or(0),
        auto_replay
            .map(|report| report.average_recursive_call_pressure)
            .unwrap_or(0.0),
        auto_replay
            .map(|report| report.max_recursive_call_pressure)
            .unwrap_or(0.0),
        outcome.evolution_ledger.live_inference_runs,
        outcome.evolution_ledger.live_router_threshold_mutations,
        outcome.evolution_ledger.live_hierarchy_weight_mutations,
        outcome.evolution_ledger.live_router_threshold_delta,
        outcome.evolution_ledger.live_hierarchy_weight_delta,
        outcome.evolution_ledger.live_memory_reinforcements,
        outcome.evolution_ledger.live_memory_penalties,
        outcome.evolution_ledger.live_memory_updates(),
        outcome.evolution_ledger.live_stored_memories,
        outcome.evolution_ledger.live_stored_gist_memories,
        outcome.evolution_ledger.live_stored_runtime_kv_memories,
        outcome.evolution_ledger.live_stored_memory_updates(),
        outcome.evolution_ledger.live_reflection_issues,
        outcome.evolution_ledger.live_critical_reflection_issues,
        outcome.evolution_ledger.live_revision_actions,
        outcome.evolution_ledger.replay_runs,
        outcome.evolution_ledger.replay_items,
        outcome.evolution_ledger.router_threshold_mutations,
        outcome.evolution_ledger.hierarchy_weight_mutations,
        outcome.evolution_ledger.router_threshold_delta,
        outcome.evolution_ledger.hierarchy_weight_delta,
        outcome.evolution_ledger.memory_reinforcements,
        outcome.evolution_ledger.memory_penalties,
        outcome.evolution_ledger.memory_updates(),
        outcome.evolution_ledger.replay_live_memory_feedback_items,
        outcome
            .evolution_ledger
            .replay_live_memory_feedback_updates(),
        outcome
            .evolution_ledger
            .replay_live_memory_feedback_reinforcements,
        outcome
            .evolution_ledger
            .replay_live_memory_feedback_penalties,
        outcome.evolution_ledger.recursive_replay_items,
        outcome.evolution_ledger.recursive_runtime_calls,
        outcome.evolution_ledger.drift_rollbacks,
        outcome.evolution_ledger.rollback_router_threshold_delta,
        outcome.evolution_ledger.rollback_hierarchy_weight_delta,
        outcome.memory_retention_policy.stale_after,
        outcome.memory_retention_policy.decay_rate,
        outcome.memory_retention_policy.remove_below_strength,
        outcome.memory_retention_policy.remove_after_failures,
        outcome.retention_report.before,
        outcome.retention_report.after,
        outcome.retention_report.decayed,
        outcome.retention_report.removed.len(),
        outcome.memory_compaction_policy.similarity_threshold,
        outcome.memory_compaction_policy.max_candidates,
        outcome.memory_compaction_policy.max_merges,
        outcome.memory_compaction_report.before,
        outcome.memory_compaction_report.after,
        outcome.memory_compaction_report.merged.len(),
        outcome.memory_compaction_report.removed.len(),
        outcome.experience_id
    )
}

pub fn append_trace_jsonl(
    path: impl AsRef<Path>,
    prompt: &str,
    profile: TaskProfile,
    elapsed_ms: u128,
    outcome: &InferenceOutcome,
) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(
        file,
        "{}",
        trace_json_line(prompt, profile, elapsed_ms, outcome)
    )
}

pub fn append_trace_jsonl_with_case(
    path: impl AsRef<Path>,
    case_name: &str,
    prompt: &str,
    profile: TaskProfile,
    elapsed_ms: u128,
    outcome: &InferenceOutcome,
) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(
        file,
        "{}",
        trace_json_line_with_case(Some(case_name), prompt, profile, elapsed_ms, outcome)
    )
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_f32_json(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "null".to_owned())
}

fn option_string_json(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_owned())
}

fn option_owned_string_json(value: Option<&str>) -> String {
    option_string_json(value)
}

fn string_array_json(items: &[String]) -> String {
    let values = items
        .iter()
        .map(|item| format!("\"{}\"", json_escape(item)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{HeuristicBackend, InferenceRequest, NoironEngine};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn trace_line_contains_core_control_decisions() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("trace Rust Noiron routing", TaskProfile::Coding),
            &mut backend,
        );

        let line = trace_json_line(
            "trace Rust Noiron routing",
            TaskProfile::Coding,
            12,
            &outcome,
        );

        assert!(line.contains("\"schema\":\"rust-norion-trace-v1\""));
        assert!(line.contains("\"case\":null"));
        assert!(line.contains("\"reflection\":"));
        assert!(line.contains("\"issue_codes\":"));
        assert!(line.contains("\"revision_passes\":"));
        assert!(line.contains("\"route\":"));
        assert!(line.contains("\"runtime_tokens\":"));
        assert!(line.contains("\"average_entropy\":"));
        assert!(line.contains("\"average_neg_logprob\":"));
        assert!(line.contains("\"uncertainty_perplexity\":"));
        assert!(line.contains("\"runtime_diagnostics\":"));
        assert!(line.contains("\"runtime_adapter_observations\":"));
        assert!(line.contains("\"observation_count\":"));
        assert!(line.contains("\"best_adapter\":"));
        assert!(line.contains("\"best_score\":"));
        assert!(line.contains("\"forward_energy\":"));
        assert!(line.contains("\"kv_influence\":"));
        assert!(line.contains("\"has_forward_signal\":"));
        assert!(line.contains("\"hierarchy\":"));
        assert!(line.contains("\"primary_lane\":"));
        assert!(line.contains("\"runtime_device_contract\":"));
        assert!(line.contains("\"adapter_hints\":"));
        assert!(line.contains("\"local_kv_token_budget\":"));
        assert!(line.contains("\"global_kv_token_budget\":"));
        assert!(line.contains("\"execution_waves\":"));
        assert!(line.contains("\"runtime_calls\":"));
        assert!(line.contains("\"max_parallel_chunks\":"));
        assert!(line.contains("\"infini_memory\":"));
        assert!(line.contains("\"local_window\":"));
        assert!(line.contains("\"global_memory\":"));
        assert!(line.contains("\"sparse_skipped\":"));
        assert!(line.contains("\"skipped_tokens\":"));
        assert!(line.contains("\"template\":\"coding_local\""));
        assert!(line.contains("\"toolsmith\":"));
        assert!(line.contains("\"blueprint_summaries\":"));
        assert!(line.contains("\"gate_passed\":"));
        assert!(line.contains("\"agent_team\":"));
        assert!(line.contains("\"collision_free\":"));
        assert!(line.contains("\"drift\":"));
        assert!(line.contains("\"process_reward\":"));
        assert!(line.contains("\"auto_replay\":"));
        assert!(line.contains("\"router_updates\":"));
        assert!(line.contains("\"hierarchy_updates\":"));
        assert!(line.contains("\"router_threshold_mutations\":"));
        assert!(line.contains("\"hierarchy_weight_mutations\":"));
        assert!(line.contains("\"router_threshold_delta\":"));
        assert!(line.contains("\"hierarchy_weight_delta\":"));
        assert!(line.contains("\"memory_reinforcements\":"));
        assert!(line.contains("\"memory_penalties\":"));
        assert!(line.contains("\"live_memory_feedback_items\":"));
        assert!(line.contains("\"live_memory_feedback_updates\":"));
        assert!(line.contains("\"live_memory_feedback_reinforcements\":"));
        assert!(line.contains("\"live_memory_feedback_penalties\":"));
        assert!(line.contains("\"recursive_runtime_items\":"));
        assert!(line.contains("\"recursive_runtime_calls\":"));
        assert!(line.contains("\"avg_recursive_call_pressure\":"));
        assert!(line.contains("\"max_recursive_call_pressure\":"));
        assert!(line.contains("\"evolution_ledger\":"));
        assert!(line.contains("\"live_inference_runs\":"));
        assert!(line.contains("\"cumulative_live_memory_updates\":"));
        assert!(line.contains("\"cumulative_live_stored_memory_updates\":"));
        assert!(line.contains("\"cumulative_router_threshold_mutations\":"));
        assert!(line.contains("\"cumulative_hierarchy_weight_mutations\":"));
        assert!(line.contains("\"cumulative_memory_updates\":"));
        assert!(line.contains("\"cumulative_replay_live_memory_feedback_updates\":"));
        assert!(line.contains("\"cumulative_recursive_runtime_calls\":"));
        assert!(line.contains("\"cumulative_drift_rollbacks\":"));
        assert!(line.contains("\"cumulative_rollback_router_threshold_delta\":"));
        assert!(line.contains("\"cumulative_rollback_hierarchy_weight_delta\":"));
        assert!(line.contains("\"runtime_kv_exported\":"));
        assert!(line.contains("\"feedback_reinforced\":"));
        assert!(line.contains("\"feedback_penalized\":"));
        assert!(line.contains("\"feedback_reinforcement_amount\":"));
        assert!(line.contains("\"feedback_penalty_amount\":"));
        assert!(line.contains("\"stale_after\":"));
        assert!(line.contains("\"decay_rate\":"));
        assert!(line.contains("\"similarity_threshold\":"));
        assert!(line.contains("\"max_merges\":"));
        assert!(line.contains("\"memory_compaction\":"));
        assert!(line.ends_with('}'));
    }

    #[test]
    fn json_escape_handles_quotes_and_newlines() {
        assert_eq!(json_escape("a\"b\nc"), "a\\\"b\\nc");
    }

    #[test]
    fn trace_line_can_include_benchmark_case_name() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("trace benchmark case", TaskProfile::General),
            &mut backend,
        );

        let line = trace_json_line_with_case(
            Some("general_case"),
            "trace benchmark case",
            TaskProfile::General,
            3,
            &outcome,
        );

        assert!(line.contains("\"case\":\"general_case\""));
    }

    #[test]
    fn trace_schema_gate_accepts_generated_trace_line() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("trace schema gate", TaskProfile::Coding),
            &mut backend,
        );
        let line = trace_json_line("trace schema gate", TaskProfile::Coding, 5, &outcome);

        let failures = evaluate_trace_schema_line(&line);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn trace_schema_gate_accepts_runtime_kv_storage_consistency() {
        let line = runtime_kv_trace_line()
            .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":1", 1)
            .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1)
            .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":1", 1)
            .replacen("\"memory_write\":false", "\"memory_write\":true", 1)
            .replacen("\"runtime_kv_write\":false", "\"runtime_kv_write\":true", 1)
            .replacen("\"revision_passes\":1", "\"revision_passes\":0", 1);

        let failures = evaluate_trace_schema_line(&line);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn trace_schema_gate_rejects_runtime_kv_storage_without_write_permission() {
        let line = runtime_kv_trace_line()
            .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":1", 1)
            .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1)
            .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":1", 1)
            .replacen("\"memory_write\":false", "\"memory_write\":true", 1)
            .replacen("\"runtime_kv_write\":true", "\"runtime_kv_write\":false", 1)
            .replacen("\"revision_passes\":1", "\"revision_passes\":0", 1);

        let failures = evaluate_trace_schema_line(&line);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("runtime_kv_write=true")),
            "{failures:?}"
        );
    }

    #[test]
    fn trace_schema_gate_rejects_runtime_kv_storage_without_memory_write() {
        let line = runtime_kv_trace_line()
            .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":1", 1)
            .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1)
            .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":1", 1)
            .replacen("\"runtime_kv_write\":false", "\"runtime_kv_write\":true", 1)
            .replacen("\"memory_write\":true", "\"memory_write\":false", 1)
            .replacen("\"revision_passes\":1", "\"revision_passes\":0", 1);

        let failures = evaluate_trace_schema_line(&line);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("memory_write=true")),
            "{failures:?}"
        );
    }

    #[test]
    fn trace_schema_gate_rejects_runtime_kv_storage_after_revision() {
        let line = runtime_kv_trace_line()
            .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":1", 1)
            .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1)
            .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":1", 1)
            .replacen("\"memory_write\":false", "\"memory_write\":true", 1)
            .replacen("\"runtime_kv_write\":false", "\"runtime_kv_write\":true", 1)
            .replacen("\"revision_passes\":0", "\"revision_passes\":1", 1);

        let failures = evaluate_trace_schema_line(&line);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("revision_passes=0")),
            "{failures:?}"
        );
    }

    #[test]
    fn trace_schema_gate_rejects_runtime_kv_export_mismatch() {
        let line = runtime_kv_trace_line()
            .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":2", 1)
            .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1);

        let failures = evaluate_trace_schema_line(&line);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("exported_kv_blocks")),
            "{failures:?}"
        );
    }

    #[test]
    fn trace_schema_gate_rejects_runtime_kv_stored_above_exported() {
        let line = runtime_kv_trace_line()
            .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":1", 1)
            .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1)
            .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":2", 1)
            .replacen("\"memory_write\":false", "\"memory_write\":true", 1)
            .replacen("\"runtime_kv_write\":false", "\"runtime_kv_write\":true", 1)
            .replacen("\"revision_passes\":1", "\"revision_passes\":0", 1);

        let failures = evaluate_trace_schema_line(&line);

        assert!(
            failures.iter().any(|failure| failure.contains("exceeds")),
            "{failures:?}"
        );
    }

    #[test]
    fn trace_schema_gate_reports_missing_required_fields() {
        let failures = evaluate_trace_schema_line("{\"schema\":\"other\"}");

        assert!(failures.iter().any(|failure| failure.contains("schema")));
        assert!(failures.iter().any(|failure| failure.contains("route")));
        assert!(failures.iter().any(|failure| failure.contains("retention")));
    }

    #[test]
    fn trace_schema_gate_rejects_device_contract_mismatch() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("trace device contract mismatch", TaskProfile::General),
            &mut backend,
        );
        let line = trace_json_line(
            "trace device contract mismatch",
            TaskProfile::General,
            5,
            &outcome,
        );
        let actual_device = extract_json_string_field(&line, "device").unwrap();
        let wrong_device = if actual_device == "server" {
            "cpu"
        } else {
            "server"
        };
        let mismatched = line.replacen(
            &format!("\"device\":\"{actual_device}\""),
            &format!("\"device\":\"{wrong_device}\""),
            1,
        );

        let failures = evaluate_trace_schema_line(&mismatched);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("runtime_device_contract device=")),
            "{failures:?}"
        );
    }

    fn runtime_kv_trace_line() -> String {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("trace runtime kv consistency", TaskProfile::Coding),
            &mut backend,
        );
        trace_json_line(
            "trace runtime kv consistency",
            TaskProfile::Coding,
            5,
            &outcome,
        )
    }

    #[test]
    fn trace_schema_gate_rejects_selected_adapter_outside_device_contract() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("trace adapter contract mismatch", TaskProfile::General),
            &mut backend,
        );
        let line = trace_json_line(
            "trace adapter contract mismatch",
            TaskProfile::General,
            5,
            &outcome,
        );
        let mismatched = line.replacen(
            "\"selected_adapter\":null",
            "\"selected_adapter\":\"cuda\"",
            1,
        );

        let failures = evaluate_trace_schema_line(&mismatched);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("selected_adapter cuda")),
            "{failures:?}"
        );
    }

    #[test]
    fn trace_schema_gate_accepts_valid_runtime_adapter_observation() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("trace adapter observation", TaskProfile::Coding),
            &mut backend,
        );
        let line = trace_json_line(
            "trace adapter observation",
            TaskProfile::Coding,
            5,
            &outcome,
        )
        .replacen("\"observation_count\":0", "\"observation_count\":1", 1)
        .replacen(
            "\"best_adapter\":null",
            "\"best_adapter\":\"portable-rust\"",
            1,
        )
        .replacen("\"best_score\":null", "\"best_score\":0.510000", 1)
        .replacen("\"best_reward\":null", "\"best_reward\":0.500000", 1)
        .replacen("\"best_quality\":null", "\"best_quality\":0.800000", 1)
        .replacen("\"best_experience_id\":null", "\"best_experience_id\":7", 1);

        let failures = evaluate_trace_schema_line(&line);

        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn trace_schema_gate_rejects_runtime_adapter_observation_contract_mismatch() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("trace adapter observation mismatch", TaskProfile::General),
            &mut backend,
        );
        let line = trace_json_line(
            "trace adapter observation mismatch",
            TaskProfile::General,
            5,
            &outcome,
        )
        .replacen("\"observation_count\":0", "\"observation_count\":1", 1)
        .replacen(
            "\"best_adapter\":null",
            "\"best_adapter\":\"not-a-device-adapter\"",
            1,
        )
        .replacen("\"best_score\":null", "\"best_score\":0.510000", 1)
        .replacen("\"best_reward\":null", "\"best_reward\":0.500000", 1)
        .replacen("\"best_quality\":null", "\"best_quality\":0.800000", 1)
        .replacen("\"best_experience_id\":null", "\"best_experience_id\":7", 1);

        let failures = evaluate_trace_schema_line(&line);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("best_adapter not-a-device-adapter")),
            "{failures:?}"
        );
    }

    #[test]
    fn trace_schema_gate_rejects_incomplete_runtime_adapter_observation() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("trace adapter observation incomplete", TaskProfile::General),
            &mut backend,
        );
        let line = trace_json_line(
            "trace adapter observation incomplete",
            TaskProfile::General,
            5,
            &outcome,
        )
        .replacen("\"observation_count\":0", "\"observation_count\":1", 1)
        .replacen(
            "\"best_adapter\":null",
            "\"best_adapter\":\"portable-rust\"",
            1,
        );

        let failures = evaluate_trace_schema_line(&line);

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("best_score is missing")),
            "{failures:?}"
        );
    }

    #[test]
    fn trace_schema_jsonl_gate_checks_non_empty_records() {
        let path = temp_path("trace-schema");
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("trace schema jsonl", TaskProfile::General),
            &mut backend,
        );
        fs::write(
            &path,
            format!(
                "\n{}\n",
                trace_json_line("trace schema jsonl", TaskProfile::General, 8, &outcome)
            ),
        )
        .unwrap();

        let report = evaluate_trace_schema_jsonl(&path).unwrap();

        assert!(report.passed, "{:?}", report.failures);
        assert_eq!(report.checked_lines, 1);
        assert!(report.summary_line().contains("passed=true"));
        cleanup(path);
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-{label}-{}-{nanos}.jsonl",
            std::process::id()
        ))
    }

    fn cleanup(path: std::path::PathBuf) {
        let _ = fs::remove_file(path);
    }
}
