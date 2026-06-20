use crate::http;
use crate::json::{json_bool_field, json_string, json_string_field, json_u64_field, preview_text};
use crate::pool_stage::PoolStageDispatchPlan;
use crate::validation;

pub(crate) const DEFAULT_TEST_GATE_VALIDATION_COMMAND: &str =
    "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --no-fail-fast";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolStageCallInput<'a> {
    pub(crate) task_kind: &'a str,
    pub(crate) case_name: &'a str,
    pub(crate) round: usize,
    pub(crate) validation_timestamp_unix: Option<u64>,
    pub(crate) validation_evidence: Option<&'a PoolStageValidationEvidence<'a>>,
    pub(crate) original_prompt: &'a str,
    pub(crate) primary_answer: Option<&'a str>,
    pub(crate) final_json: Option<&'a str>,
    pub(crate) dispatch_plan: Option<&'a PoolStageDispatchPlan>,
    pub(crate) completed_roles: &'a [String],
    pub(crate) max_tokens: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PoolStageValidationEvidence<'a> {
    pub(crate) phase: &'a str,
    pub(crate) command_source: &'a str,
    pub(crate) command_safety: &'a str,
    pub(crate) command_preview: &'a str,
    pub(crate) status_code: Option<i32>,
    pub(crate) elapsed_ms: u64,
    pub(crate) stdout_tail: &'a str,
    pub(crate) stderr_tail: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolStageCallResult {
    pub(crate) task_kind: String,
    pub(crate) ok: bool,
    pub(crate) selected_role: Option<String>,
    pub(crate) selected_port: Option<u64>,
    pub(crate) selected_base_url: Option<String>,
    pub(crate) answer: Option<String>,
    pub(crate) elapsed_ms: Option<u64>,
    pub(crate) answer_chars: Option<u64>,
    pub(crate) answer_bytes: Option<u64>,
    pub(crate) answer_approx_tokens: Option<u64>,
}

pub(crate) fn call_backend(
    backend: &str,
    timeout_secs: u64,
    input: &PoolStageCallInput<'_>,
) -> Result<PoolStageCallResult, String> {
    let body = request_body(input);
    let response = http::post_json(backend, "/v1/model-pool/call", &body, timeout_secs)
        .map_err(|error| format!("pool stage call {} failed: {error}", input.task_kind))?;
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "pool stage call {} returned HTTP {}: {}",
            input.task_kind,
            response.status,
            response.body.trim()
        ));
    }
    let mut result = parse_response(input.task_kind, &response.body);
    normalize_contract_answer(input, &mut result);
    Ok(result)
}

pub(crate) fn request_body(input: &PoolStageCallInput<'_>) -> String {
    format!(
        "{{\"task_kind\":{},\"prompt\":{},\"max_tokens\":{},\"completed_roles\":{}}}",
        json_string(input.task_kind),
        json_string(&stage_prompt(input)),
        input.max_tokens.max(1),
        string_array_json(input.completed_roles)
    )
}

pub(crate) fn stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    if input.task_kind == "summary" {
        return summary_stage_prompt(input);
    }
    if input.task_kind == "router" {
        return router_stage_prompt(input);
    }
    if input.task_kind == "review" {
        return review_stage_prompt(input);
    }
    if input.task_kind == "index" {
        return index_stage_prompt(input);
    }
    if input.task_kind == "test-gate" {
        return test_gate_stage_prompt(input);
    }
    format!(
        "SmartSteam evolution-loop helper stage.\ncase: {}\nstage_task_kind: {}\n{}\nrole_contract:\n{}\n{}\nprimary_prompt_preview: {}\nprimary_answer_preview: {}\nfinal_json_preview: {}\n\nOutput exactly one short bullet per role_contract field, in the same order, with the field name unchanged. Keep each field under 160 characters, cite only evidence from structured_facts and the previews, and do not repeat the full primary answer. Do not add prose before or after the bullets.",
        input.case_name,
        input.task_kind,
        structured_facts(input),
        stage_instruction(input.task_kind),
        decision_rules(input.task_kind),
        preview_text(input.original_prompt, 1200),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 4000))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 2000))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn index_stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    format!(
        "SmartSteam index helper.\nReturn only these completed lines:\n{}\n\ncase: {}\n{}\n\nEvidence previews:\nprimary_prompt: {}\nprimary_answer: {}\nfinal_json: {}\n\nRules:\n- You are not answering the user directly.\n- Do not output markdown fences, explanations, JSON blocks, or labels other than clean_gist, tags, dependency_link, source_origin, validation_timestamp, retention.\n- tags must be semicolon-separated key=value retrieval labels, not comma-separated prose.\n- tags must include role=index, case, round, primary, final_json, dependency, source_origin, and validation_timestamp labels.\n- dependency_link must name the upstream helper field or primary evidence source behind the index record.\n- source_origin must repeat the concrete upstream helper field or primary evidence source used for the index record.\n- validation_timestamp must be the exact Unix timestamp from structured_facts.\n- clean_gist must mention the smallest searchable behavior or contract fact from the evidence.\n- retention must be keep, compress, or drop with a short evidence-backed reason.\n- Keep exactly six lines and keep the field names unchanged.",
        index_field_defaults(input),
        input.case_name,
        structured_facts(input),
        preview_text(input.original_prompt, 480),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 1400))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 900))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn index_field_defaults(input: &PoolStageCallInput<'_>) -> String {
    let primary = if input
        .primary_answer
        .map(|answer| !answer.trim().is_empty())
        .unwrap_or(false)
    {
        "present"
    } else {
        "missing"
    };
    let final_json = if input
        .final_json
        .map(|json| !json.trim().is_empty())
        .unwrap_or(false)
    {
        "present"
    } else {
        "missing"
    };
    let dependency = if input.completed_roles.iter().any(|role| role == "review") {
        "review.change_request"
    } else if input.completed_roles.iter().any(|role| role == "summary") {
        "summary.next_context"
    } else {
        "primary.evidence"
    };
    let worker = input
        .dispatch_plan
        .map(|plan| {
            format!(
                "{}@{}",
                plan.selected_role,
                option_u64_text(plan.selected_port)
            )
        })
        .unwrap_or_else(|| "index worker".to_owned());
    let validation_timestamp = option_u64_text(input.validation_timestamp_unix);
    format!(
        "clean_gist: Index round {round} {case} with {worker}; primary={primary}; final_json={final_json}; dependency={dependency}; validation_timestamp={validation_timestamp}\ntags: role=index;case={case};round={round};primary={primary};final_json={final_json};dependency={dependency};source_origin={dependency};validation_timestamp={validation_timestamp}\ndependency_link: {dependency}\nsource_origin: {dependency}\nvalidation_timestamp: {validation_timestamp}\nretention: keep; compact retrieval evidence for the next evolution round",
        case = input.case_name,
        round = input.round
    )
}

fn review_stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    format!(
        "SmartSteam review helper.\nReturn only these completed lines:\n{}\n\ncase: {}\n{}\n\nEvidence previews:\nprimary_prompt: {}\nprimary_answer: {}\nfinal_json: {}\n\nRules:\n- You are not answering the user directly.\n- Never output placeholder contract descriptions.\n- Do not output these phrases as field values: highest concrete code or behavior risk; smallest improvement to make next; one check that would prove the change.\n- Every field value must cite evidence from structured_facts, primary_answer, or final_json.\n- If evidence is weak, name the concrete limitation instead of using a placeholder.\n- Keep exactly three lines, keep the field names unchanged, and do not add prose before or after the lines.",
        review_field_contract(),
        input.case_name,
        structured_facts(input),
        preview_text(input.original_prompt, 480),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 1400))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 900))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn review_field_contract() -> &'static str {
    "risk: concrete risk evidenced by structured_facts or previews\nchange_request: small next change grounded in the same evidence\nverification: executable command or direct log/file check that verifies the change"
}

fn router_stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    let router_fields = router_field_defaults(input);
    format!(
        "SmartSteam router helper.\nReturn only these completed lines:\n{}\n\ncase: {}\n{}\n\nEvidence previews:\nprimary_prompt: {}\nprimary_answer: {}\nfinal_json: {}\n\nRules:\n- You are not answering the user directly.\n- Do not say you cannot perform the task.\n- Do not output markdown fences, explanations, JSON blocks, or labels other than route_intent, tool_call, preflight.\n- Keep exactly three lines and keep the field names unchanged.\n- Use tool_call: null unless the evidence names a concrete safe tool call.",
        router_fields,
        input.case_name,
        structured_facts(input),
        preview_text(input.original_prompt, 360),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 800))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 360))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn router_field_defaults(input: &PoolStageCallInput<'_>) -> String {
    let final_json_present = input
        .final_json
        .map(|json| !json.trim().is_empty())
        .unwrap_or(false);
    let route_intent = if final_json_present {
        "index"
    } else {
        "review"
    };
    let worker = input
        .dispatch_plan
        .map(|plan| {
            format!(
                "{}@{}",
                plan.selected_role,
                option_u64_text(plan.selected_port)
            )
        })
        .unwrap_or_else(|| "router worker".to_owned());
    format!(
        "route_intent: {route_intent}\ntool_call: null\npreflight: allow because {worker} is selected and the stage request is read-only."
    )
}

fn normalize_contract_answer(input: &PoolStageCallInput<'_>, result: &mut PoolStageCallResult) {
    match input.task_kind {
        "router" => {
            let has_contract = result
                .answer
                .as_deref()
                .map(router_answer_has_contract)
                .unwrap_or(false);
            if !has_contract {
                set_answer(result, router_field_defaults(input));
            }
        }
        "test-gate" => {
            let has_contract = result
                .answer
                .as_deref()
                .map(|answer| test_gate_answer_has_supported_contract(input, answer))
                .unwrap_or(false);
            if !has_contract {
                set_answer(result, test_gate_field_defaults(input));
            }
        }
        "index" => {
            let has_contract = result
                .answer
                .as_deref()
                .map(index_answer_has_stable_contract)
                .unwrap_or(false);
            if !has_contract {
                set_answer(result, index_field_defaults(input));
            }
        }
        _ => {}
    }
}

fn router_answer_has_contract(answer: &str) -> bool {
    let lower = answer.to_ascii_lowercase();
    lower.contains("route_intent") && lower.contains("tool_call") && lower.contains("preflight")
}

fn test_gate_answer_has_supported_contract(input: &PoolStageCallInput<'_>, answer: &str) -> bool {
    let lower = answer.to_ascii_lowercase();
    if !(lower.contains("verdict")
        && lower.contains("validation_command")
        && lower.contains("failure_kind"))
    {
        return false;
    }
    let Some(command) = extract_field_value(answer, "validation_command") else {
        return false;
    };
    if validation::test_gate_validation_command_safety(Some(&command)) != "safe" {
        return false;
    }
    let verdict = extract_field_value(answer, "verdict")
        .and_then(|value| normalized_test_gate_verdict(&value));
    let Some(verdict) = verdict else {
        return false;
    };
    let failure_kind = extract_field_value(answer, "failure_kind")
        .unwrap_or_else(|| "missing_evidence".to_owned())
        .to_ascii_lowercase();
    if test_gate_validation_evidence_supports_pass(input) {
        !(verdict != "pass" && failure_kind == "missing_evidence")
    } else {
        verdict != "pass"
    }
}

fn normalized_test_gate_verdict(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "pass" => Some("pass"),
        "warn" => Some("warn"),
        "fail" => Some("fail"),
        _ => None,
    }
}

fn index_answer_has_stable_contract(answer: &str) -> bool {
    let lower = answer.to_ascii_lowercase();
    if !(lower.contains("clean_gist")
        && lower.contains("tags")
        && lower.contains("dependency_link")
        && lower.contains("source_origin")
        && lower.contains("validation_timestamp")
        && lower.contains("retention"))
    {
        return false;
    }
    let Some(dependency_link) = extract_field_value(answer, "dependency_link") else {
        return false;
    };
    if dependency_link.trim().eq_ignore_ascii_case("none") {
        return false;
    }
    let Some(source_origin) = extract_field_value(answer, "source_origin") else {
        return false;
    };
    if source_origin.trim().eq_ignore_ascii_case("none") {
        return false;
    }
    let Some(validation_timestamp) = extract_field_value(answer, "validation_timestamp") else {
        return false;
    };
    if !is_stable_unix_timestamp(&validation_timestamp) {
        return false;
    }
    let Some(tags) = extract_field_value(answer, "tags") else {
        return false;
    };
    if !index_tags_are_stable(&tags) {
        return false;
    }
    let dependency_matches = index_tag_value(&tags, "dependency")
        .map(|dependency| dependency == dependency_link.trim())
        .unwrap_or(false);
    let timestamp_matches = index_tag_value(&tags, "validation_timestamp")
        .map(|timestamp| timestamp == validation_timestamp.trim())
        .unwrap_or(false);
    let source_origin_matches = index_tag_value(&tags, "source_origin")
        .map(|origin| origin == source_origin.trim())
        .unwrap_or(false);
    dependency_matches && source_origin_matches && timestamp_matches
}

fn index_tags_are_stable(tags: &str) -> bool {
    let labels = tags
        .split(';')
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>();
    if labels.len() < 5 {
        return false;
    }
    let mut keys = labels
        .iter()
        .filter_map(|label| label.split_once('='))
        .map(|(key, value)| (key.trim().to_ascii_lowercase(), value.trim()))
        .filter(|(_, value)| !value.is_empty())
        .collect::<Vec<_>>();
    if keys.len() != labels.len() {
        return false;
    }
    keys.sort_by(|left, right| left.0.cmp(&right.0));
    keys.iter()
        .any(|(key, value)| key == "role" && *value == "index")
        && keys.iter().any(|(key, _)| key == "case")
        && keys.iter().any(|(key, _)| key == "round")
        && keys.iter().any(|(key, _)| key == "primary")
        && keys.iter().any(|(key, _)| key == "final_json")
        && keys.iter().any(|(key, _)| key == "dependency")
        && keys.iter().any(|(key, _)| key == "source_origin")
        && keys.iter().any(|(key, _)| key == "validation_timestamp")
}

fn is_stable_unix_timestamp(value: &str) -> bool {
    value.chars().all(|character| character.is_ascii_digit()) && value.len() >= 10
}

fn index_tag_value<'a>(tags: &'a str, target_key: &str) -> Option<&'a str> {
    tags.split(';')
        .map(str::trim)
        .filter_map(|label| label.split_once('='))
        .find_map(|(key, value)| {
            (key.trim().eq_ignore_ascii_case(target_key) && !value.trim().is_empty())
                .then_some(value.trim())
        })
}

fn extract_field_value(text: &str, field: &str) -> Option<String> {
    let field = field.to_ascii_lowercase();
    for line in text.lines() {
        for segment in line.split(" / ") {
            let candidate = trim_contract_bullet(segment);
            let lower = candidate.to_ascii_lowercase();
            if !lower.starts_with(&field) {
                continue;
            }
            let Some(after_field) = candidate.get(field.len()..) else {
                continue;
            };
            let after_separator = after_field.trim_start();
            let Some(value_body) = after_separator
                .strip_prefix(':')
                .or_else(|| after_separator.strip_prefix('='))
                .or_else(|| after_separator.strip_prefix('-'))
            else {
                continue;
            };
            let value = value_body
                .split(" ; ")
                .next()
                .unwrap_or_default()
                .trim()
                .trim_matches(|character| matches!(character, '"' | '\''));
            if !value.is_empty() && !value.eq_ignore_ascii_case("none") {
                return Some(value.to_owned());
            }
        }
    }
    None
}

fn trim_contract_bullet(text: &str) -> &str {
    text.trim()
        .strip_prefix("- ")
        .or_else(|| text.trim().strip_prefix("* "))
        .unwrap_or_else(|| text.trim())
}

fn set_answer(result: &mut PoolStageCallResult, answer: String) {
    result.answer_chars = Some(answer.chars().count() as u64);
    result.answer_bytes = Some(answer.len() as u64);
    result.answer_approx_tokens = Some(answer.chars().count().div_ceil(4) as u64);
    result.answer = Some(answer);
}

fn summary_stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    let summary_fields = summary_field_defaults(input);
    format!(
        "SmartSteam summary helper.\nReturn only these completed lines:\n{}\n\ncase: {}\n{}\n\nEvidence previews:\nprimary_prompt: {}\nprimary_answer: {}\nfinal_json: {}\n\nRules:\n- You are not writing code.\n- Do not output markdown fences, functions, JSON, explanations, or labels other than memory_update, next_context, duplicate_guard.\n- You may make the values more specific using the evidence, but keep exactly three lines.\n- Never output placeholders.",
        summary_fields,
        input.case_name,
        structured_facts(input),
        preview_text(input.original_prompt, 360),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 800))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 360))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn summary_field_defaults(input: &PoolStageCallInput<'_>) -> String {
    let worker = input
        .dispatch_plan
        .map(|plan| {
            format!(
                "{}@{}",
                plan.selected_role,
                option_u64_text(plan.selected_port)
            )
        })
        .unwrap_or_else(|| "summary worker".to_owned());
    let runtime = input
        .dispatch_plan
        .and_then(|plan| {
            let device = plan.runtime_device.as_deref()?;
            let accelerator = plan.runtime_accelerator.as_deref().unwrap_or("none");
            Some(format!("{device}/{accelerator}"))
        })
        .unwrap_or_else(|| "reported runtime".to_owned());
    format!(
        "memory_update: Keep {worker} on {runtime} for short summary memory updates.\nnext_context: Preserve model-pool stage evidence before the next evolution round.\nduplicate_guard: Do not emit code, markdown fences, placeholders, or route summary work to the 12B worker."
    )
}

fn structured_facts(input: &PoolStageCallInput<'_>) -> String {
    let final_json_present = input
        .final_json
        .map(|json| !json.trim().is_empty())
        .unwrap_or(false);
    let primary_answer_present = input
        .primary_answer
        .map(|answer| !answer.trim().is_empty())
        .unwrap_or(false);
    let mut facts = vec![
        "structured_facts:".to_owned(),
        format!("- task_kind: {}", input.task_kind),
        format!("- round: {}", input.round),
        format!(
            "- validation_timestamp: {}",
            option_u64_text(input.validation_timestamp_unix)
        ),
        format!("- primary_answer_present: {}", primary_answer_present),
        format!("- final_json_present: {}", final_json_present),
        format!("- requested_max_tokens: {}", input.max_tokens.max(1)),
    ];
    if let Some(evidence) = input.validation_evidence {
        facts.push("- validation_gate_checked: true".to_owned());
        facts.push(format!(
            "- validation_gate_passed: {}",
            evidence.status_code == Some(0)
        ));
        facts.push(format!("- validation_gate_phase: {}", evidence.phase));
        facts.push(format!(
            "- validation_command_source: {}",
            evidence.command_source
        ));
        facts.push(format!(
            "- validation_command_safety: {}",
            evidence.command_safety
        ));
        facts.push(format!(
            "- validation_command_safe_for_test_gate: {}",
            validation::test_gate_validation_command_safety(Some(evidence.command_preview))
        ));
        facts.push(format!(
            "- validation_command: {}",
            evidence.command_preview
        ));
        facts.push(format!(
            "- validation_status_code: {}",
            option_i32_text(evidence.status_code)
        ));
        facts.push(format!("- validation_elapsed_ms: {}", evidence.elapsed_ms));
        facts.push(format!(
            "- validation_stdout_tail: {}",
            dash_if_empty(evidence.stdout_tail)
        ));
        facts.push(format!(
            "- validation_stderr_tail: {}",
            dash_if_empty(evidence.stderr_tail)
        ));
    } else {
        facts.push("- validation_gate_checked: false".to_owned());
        facts.push("- validation_gate_passed: false".to_owned());
        facts.push("- validation_command_source: none".to_owned());
        facts.push("- validation_command_safe_for_test_gate: missing".to_owned());
        facts.push("- validation_status_code: none".to_owned());
    }
    if let Some(plan) = input.dispatch_plan {
        facts.push(format!("- selected_role: {}", plan.selected_role));
        facts.push(format!(
            "- selected_port: {}",
            option_u64_text(plan.selected_port)
        ));
        facts.push(format!(
            "- selected_base_url: {}",
            plan.selected_base_url.as_deref().unwrap_or("none")
        ));
        facts.push(format!(
            "- context_window: {}",
            option_u64_text(plan.context_window)
        ));
        facts.push(format!(
            "- default_max_tokens: {}",
            option_u64_text(plan.default_max_tokens)
        ));
        facts.push(format!(
            "- runtime_backend: {}",
            plan.runtime_backend.as_deref().unwrap_or("none")
        ));
        facts.push(format!(
            "- runtime_device: {}",
            plan.runtime_device.as_deref().unwrap_or("none")
        ));
        facts.push(format!(
            "- runtime_accelerator: {}",
            plan.runtime_accelerator.as_deref().unwrap_or("none")
        ));
        facts.push(format!(
            "- gpu_layers: {}",
            option_u64_text(plan.gpu_layers)
        ));
        facts.push(format!(
            "- configured_max_tokens: {}",
            plan.configured_max_tokens
        ));
        facts.push(format!(
            "- effective_max_tokens: {}",
            plan.effective_max_tokens
        ));
        facts.push(format!("- max_tokens_clamped: {}", plan.max_tokens_clamped));
        facts.push(format!(
            "- can_accept_low_priority_task: {}",
            plan.can_accept_low_priority_task
        ));
    } else {
        facts.push("- selected_role: unknown".to_owned());
    }
    facts.join("\n")
}

fn decision_rules(task_kind: &str) -> &'static str {
    match task_kind {
        "test-gate" => {
            "decision_rules:\n- verdict must be exactly pass, warn, or fail.\n- use pass only when structured_facts say validation_gate_checked=true, validation_gate_passed=true, validation_command_safe_for_test_gate=safe, and validation_status_code=0.\n- validation_command must copy the safe validation_command from structured_facts when present; otherwise prefer cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --no-fail-fast.\n- use failure_kind: none when verdict is pass; otherwise use a short category such as missing_evidence, unsafe_command, or validation_risk."
        }
        "summary" => {
            "decision_rules:\n- summarize only durable facts that should improve the next round.\n- do not ask the primary 12B model to repeat work already captured in duplicate_guard."
        }
        "router" => {
            "decision_rules:\n- route_intent must be exactly summary, router, review, test-gate, index, quality, or none.\n- tool_call must be a compact JSON object or null; do not invent unavailable tools.\n- preflight must be allow or block with one short reason."
        }
        "review" => {
            "decision_rules:\n- name one concrete risk and one small change request from the evidence.\n- verification should be executable or directly inspectable."
        }
        "index" => {
            "decision_rules:\n- clean_gist should be searchable and compact.\n- tags must be semicolon-separated key=value retrieval labels, not comma-separated prose.\n- dependency_link must point to the upstream helper field or primary evidence source.\n- source_origin must repeat the concrete source used for dependency_link and tags.\n- validation_timestamp must copy structured_facts validation_timestamp exactly.\n- retention must choose keep, compress, or drop."
        }
        _ => {
            "decision_rules:\n- keep the answer evidence-backed and actionable.\n- verification should be specific."
        }
    }
}

fn stage_instruction(task_kind: &str) -> &'static str {
    match task_kind {
        "summary" => {
            "- memory_update: one reusable lesson from this round\n- next_context: one fact the next prompt should remember\n- duplicate_guard: one thing not to repeat"
        }
        "router" => {
            "- route_intent: summary, router, review, test-gate, index, quality, or none\n- tool_call: compact JSON object or null\n- preflight: allow or block with one short reason"
        }
        "review" => review_field_contract(),
        "test-gate" => {
            "- verdict: pass, warn, or fail\n- validation_command: one safe local cargo command to run\n- failure_kind: concise category if verdict is not pass; use none when verdict is pass"
        }
        "index" => {
            "- clean_gist: compact searchable summary\n- tags: role=index;case=<case>;round=<round>;primary=<present|missing>;final_json=<present|missing>;dependency=<source>;source_origin=<source>;validation_timestamp=<unix>\n- dependency_link: upstream helper field or primary evidence source\n- source_origin: same upstream helper field or primary evidence source\n- validation_timestamp: Unix timestamp copied from structured_facts\n- retention: keep, compress, or drop with reason"
        }
        _ => {
            "- observation: one evidence-backed observation\n- next_action: one small next step\n- verification: one way to check it"
        }
    }
}

fn test_gate_stage_prompt(input: &PoolStageCallInput<'_>) -> String {
    format!(
        "SmartSteam test-gate helper.\nReturn only these completed lines:\n{}\n\ncase: {}\n{}\n\nEvidence previews:\nprimary_prompt: {}\nprimary_answer: {}\nfinal_json: {}\n\nRules:\n- You are not answering the user directly.\n- Do not output markdown fences, JSON blocks, explanations, or labels other than verdict, validation_command, failure_kind.\n- If structured_facts show a safe validation command already ran and validation_status_code is 0, verdict must be pass and failure_kind must be none.\n- If validation evidence is missing, verdict must be warn and failure_kind must be missing_evidence.\n- Keep exactly three lines and keep the field names unchanged.",
        test_gate_field_defaults(input),
        input.case_name,
        structured_facts(input),
        preview_text(input.original_prompt, 480),
        input
            .primary_answer
            .map(|answer| preview_text(answer, 1200))
            .unwrap_or_else(|| "none".to_owned()),
        input
            .final_json
            .map(|json| preview_text(json, 900))
            .unwrap_or_else(|| "none".to_owned())
    )
}

fn test_gate_field_defaults(input: &PoolStageCallInput<'_>) -> String {
    let command = test_gate_safe_validation_command(input);
    if test_gate_validation_evidence_supports_pass(input) {
        return format!("verdict: pass\nvalidation_command: {command}\nfailure_kind: none");
    }
    let failure_kind = input
        .validation_evidence
        .map(|evidence| {
            if validation::test_gate_validation_command_safety(Some(evidence.command_preview))
                != "safe"
            {
                "unsafe_command"
            } else {
                "validation_risk"
            }
        })
        .unwrap_or("missing_evidence");
    format!("verdict: warn\nvalidation_command: {command}\nfailure_kind: {failure_kind}")
}

fn test_gate_safe_validation_command(input: &PoolStageCallInput<'_>) -> String {
    input
        .validation_evidence
        .and_then(|evidence| {
            (validation::test_gate_validation_command_safety(Some(evidence.command_preview))
                == "safe")
                .then_some(evidence.command_preview.trim())
        })
        .filter(|command| !command.is_empty())
        .unwrap_or(DEFAULT_TEST_GATE_VALIDATION_COMMAND)
        .to_owned()
}

fn test_gate_validation_evidence_supports_pass(input: &PoolStageCallInput<'_>) -> bool {
    input.validation_evidence.is_some_and(|evidence| {
        evidence.status_code == Some(0)
            && validation::test_gate_validation_command_safety(Some(evidence.command_preview))
                == "safe"
    })
}

fn option_i32_text(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn dash_if_empty(value: &str) -> &str {
    if value.trim().is_empty() {
        "-"
    } else {
        value.trim()
    }
}

fn option_u64_text(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn string_array_json(values: &[String]) -> String {
    let items = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

pub(crate) fn parse_response(task_kind: &str, body: &str) -> PoolStageCallResult {
    PoolStageCallResult {
        task_kind: json_string_field(body, "task_kind").unwrap_or_else(|| task_kind.to_owned()),
        ok: json_bool_field(body, "ok").unwrap_or(false),
        selected_role: json_string_field(body, "selected_role"),
        selected_port: json_u64_field(body, "selected_port"),
        selected_base_url: json_string_field(body, "selected_base_url"),
        answer: json_string_field(body, "answer"),
        elapsed_ms: json_u64_field(body, "elapsed_ms"),
        answer_chars: json_u64_field(body, "answer_chars"),
        answer_bytes: json_u64_field(body, "answer_bytes"),
        answer_approx_tokens: json_u64_field(body, "answer_approx_tokens"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> PoolStageCallInput<'static> {
        PoolStageCallInput {
            task_kind: "review",
            case_name: "case-1",
            round: 1,
            validation_timestamp_unix: Some(1_781_770_000),
            validation_evidence: None,
            original_prompt: "Improve the Forge UI",
            primary_answer: Some("Changed a Rust module"),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: None,
            completed_roles: &[],
            max_tokens: 256,
        }
    }

    fn test_gate_plan() -> PoolStageDispatchPlan {
        PoolStageDispatchPlan {
            task_kind: "test-gate".to_owned(),
            selected_role: "test-gate".to_owned(),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            context_window: Some(4096),
            default_max_tokens: Some(768),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("cpu".to_owned()),
            runtime_accelerator: Some("accelerate".to_owned()),
            gpu_layers: Some(0),
            configured_max_tokens: 262_144,
            effective_max_tokens: 768,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        }
    }

    fn index_plan() -> PoolStageDispatchPlan {
        PoolStageDispatchPlan {
            task_kind: "index".to_owned(),
            selected_role: "index".to_owned(),
            selected_port: Some(8690),
            selected_base_url: Some("http://127.0.0.1:8690".to_owned()),
            context_window: Some(4096),
            default_max_tokens: Some(512),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("cpu".to_owned()),
            runtime_accelerator: Some("accelerate".to_owned()),
            gpu_layers: Some(0),
            configured_max_tokens: 4096,
            effective_max_tokens: 512,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        }
    }

    fn passed_validation_evidence() -> PoolStageValidationEvidence<'static> {
        PoolStageValidationEvidence {
            phase: "pre",
            command_source: "configured",
            command_safety: "explicit",
            command_preview: "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\evolution-loop-daemon-check",
            status_code: Some(0),
            elapsed_ms: 7127,
            stdout_tail: "test result: ok. 349 passed; 0 failed",
            stderr_tail: "",
        }
    }

    #[test]
    fn request_body_targets_model_pool_call_contract() {
        let body = request_body(&input());

        assert!(body.contains("\"task_kind\":\"review\""));
        assert!(body.contains("\"max_tokens\":256"));
        assert!(body.contains("\"completed_roles\":[]"));
        assert!(body.contains("SmartSteam review helper"));
        assert!(body.contains("structured_facts"));
        assert!(body.contains("Return only these completed lines"));
        assert!(body.contains("primary_answer"));
        assert!(body.contains("change_request"));
        assert!(body.contains("verification"));
        assert!(!body.contains("role_contract"));
    }

    #[test]
    fn stage_prompt_requires_exact_bulleted_contract_output() {
        let mut index = input();
        index.task_kind = "index";
        let prompt = stage_prompt(&index);

        assert!(prompt.contains("SmartSteam index helper"));
        assert!(prompt.contains("Return only these completed lines"));
        assert!(prompt.contains("keep the field names unchanged"));
        assert!(prompt.contains("tags must be semicolon-separated key=value retrieval labels"));
        assert!(prompt.contains("not comma-separated prose"));
        assert!(prompt.contains(
            "tags: role=index;case=case-1;round=1;primary=present;final_json=present;dependency=primary.evidence;source_origin=primary.evidence;validation_timestamp=1781770000"
        ));
        assert!(prompt.contains("dependency_link: primary.evidence"));
        assert!(prompt.contains("source_origin: primary.evidence"));
        assert!(prompt.contains("validation_timestamp: 1781770000"));
        assert!(prompt.contains("Keep exactly six lines"));
        assert!(!prompt.contains("role_contract"));
    }

    #[test]
    fn review_stage_prompt_blocks_placeholder_contract_descriptions() {
        let prompt = stage_prompt(&input());

        assert!(prompt.contains("SmartSteam review helper"));
        assert!(prompt.contains("risk: concrete risk evidenced by structured_facts or previews"));
        assert!(prompt.contains("change_request: small next change grounded in the same evidence"));
        assert!(prompt.contains(
            "verification: executable command or direct log/file check that verifies the change"
        ));
        assert!(prompt.contains("Never output placeholder contract descriptions"));
        assert!(prompt.contains("highest concrete code or behavior risk"));
        assert!(prompt.contains("smallest improvement to make next"));
        assert!(prompt.contains("one check that would prove the change"));
        assert!(prompt.contains(
            "Every field value must cite evidence from structured_facts, primary_answer, or final_json"
        ));
        assert!(prompt.contains("If evidence is weak, name the concrete limitation"));
        assert!(!prompt.contains("role_contract"));
        assert!(!prompt.contains("- risk: highest concrete code or behavior risk"));
        assert!(!prompt.contains("- change_request: smallest improvement to make next"));
        assert!(!prompt.contains("- verification: one check that would prove the change"));
    }

    #[test]
    fn stage_prompt_uses_role_specific_contracts() {
        let mut summary = input();
        summary.task_kind = "summary";
        let summary_prompt = stage_prompt(&summary);
        assert!(summary_prompt.contains("memory_update"));
        assert!(summary_prompt.contains("duplicate_guard"));
        assert!(summary_prompt.contains("You are not writing code"));
        assert!(summary_prompt.contains("Return only these completed lines"));
        assert!(summary_prompt.contains("Do not emit code"));
        assert!(!summary_prompt.contains("role_contract"));
        assert!(!summary_prompt.contains("<one reusable lesson"));

        let mut test_gate = input();
        test_gate.task_kind = "test-gate";
        let test_gate_prompt = stage_prompt(&test_gate);
        assert!(test_gate_prompt.contains("validation_command"));
        assert!(test_gate_prompt.contains("failure_kind"));
        assert!(test_gate_prompt.contains("SmartSteam test-gate helper"));
        assert!(test_gate_prompt.contains("- validation_gate_checked: false"));
        assert!(test_gate_prompt.contains("verdict: warn"));
        assert!(test_gate_prompt.contains("failure_kind: missing_evidence"));
        assert!(
            !test_gate_prompt
                .contains("validation_command: one safe local cargo command to run, or none")
        );
        assert!(test_gate_prompt.contains("If validation evidence is missing"));

        let mut index = input();
        index.task_kind = "index";
        let index_prompt = stage_prompt(&index);
        assert!(index_prompt.contains("clean_gist"));
        assert!(index_prompt.contains("dependency_link"));
        assert!(index_prompt.contains("source_origin"));
        assert!(index_prompt.contains("retention"));
        assert!(index_prompt.contains("role=index;case=case-1;round=1;primary=present"));
        assert!(index_prompt.contains("not comma-separated prose"));

        let review_prompt = stage_prompt(&input());
        assert!(review_prompt.contains("risk: concrete risk evidenced"));
        assert!(review_prompt.contains("Never output placeholder contract descriptions"));
        assert!(!review_prompt.contains("role_contract"));

        let mut router = input();
        router.task_kind = "router";
        let router_prompt = stage_prompt(&router);
        assert!(router_prompt.contains("route_intent"));
        assert!(router_prompt.contains("route_intent: index"));
        assert!(router_prompt.contains("tool_call"));
        assert!(router_prompt.contains("tool_call: null"));
        assert!(router_prompt.contains("preflight"));
        assert!(router_prompt.contains("Return only these completed lines"));
        assert!(router_prompt.contains("Do not say you cannot perform the task"));
        assert!(!router_prompt.contains("role_contract"));
    }

    #[test]
    fn test_gate_prompt_includes_dispatch_facts_for_small_worker_judgment() {
        let plan = test_gate_plan();
        let validation = passed_validation_evidence();
        let input = PoolStageCallInput {
            task_kind: "test-gate",
            case_name: "case-1",
            round: 1,
            validation_timestamp_unix: Some(1_781_770_000),
            validation_evidence: Some(&validation),
            original_prompt: "Check the pool",
            primary_answer: Some("Implemented a small pool-stage prompt change."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &[],
            max_tokens: plan.effective_max_tokens,
        };

        let prompt = stage_prompt(&input);

        assert!(prompt.contains("structured_facts:"));
        assert!(prompt.contains("- round: 1"));
        assert!(prompt.contains("- validation_timestamp: 1781770000"));
        assert!(prompt.contains("- selected_role: test-gate"));
        assert!(prompt.contains("- selected_port: 8688"));
        assert!(prompt.contains("- runtime_device: cpu"));
        assert!(prompt.contains("- runtime_accelerator: accelerate"));
        assert!(prompt.contains("- gpu_layers: 0"));
        assert!(prompt.contains("- configured_max_tokens: 262144"));
        assert!(prompt.contains("- effective_max_tokens: 768"));
        assert!(prompt.contains("- max_tokens_clamped: true"));
        assert!(prompt.contains("- primary_answer_present: true"));
        assert!(prompt.contains("- final_json_present: true"));
        assert!(prompt.contains("- validation_gate_checked: true"));
        assert!(prompt.contains("- validation_gate_passed: true"));
        assert!(prompt.contains("- validation_command_source: configured"));
        assert!(prompt.contains("- validation_command_safety: explicit"));
        assert!(prompt.contains("- validation_command_safe_for_test_gate: safe"));
        assert!(prompt.contains("- validation_status_code: 0"));
        assert!(prompt.contains("test result: ok. 349 passed; 0 failed"));
        assert!(prompt.contains("verdict: pass"));
        assert!(prompt.contains(
            "validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\evolution-loop-daemon-check"
        ));
        assert!(prompt.contains("failure_kind: none"));
    }

    #[test]
    fn router_result_falls_back_to_contract_when_model_refuses() {
        let plan = PoolStageDispatchPlan {
            task_kind: "router".to_owned(),
            selected_role: "router".to_owned(),
            selected_port: Some(8689),
            selected_base_url: Some("http://127.0.0.1:8689".to_owned()),
            context_window: Some(4096),
            default_max_tokens: Some(512),
            runtime_backend: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_accelerator: Some("metal".to_owned()),
            gpu_layers: Some(999),
            configured_max_tokens: 256,
            effective_max_tokens: 256,
            max_tokens_clamped: false,
            can_accept_low_priority_task: true,
        };
        let completed_roles = vec!["quality".to_owned(), "summary".to_owned()];
        let input = PoolStageCallInput {
            task_kind: "router",
            case_name: "case-1",
            round: 1,
            validation_timestamp_unix: Some(1_781_770_000),
            validation_evidence: None,
            original_prompt: "Route this",
            primary_answer: Some("small improvement"),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &completed_roles,
            max_tokens: 256,
        };
        let mut result = PoolStageCallResult {
            task_kind: "router".to_owned(),
            ok: true,
            selected_role: Some("router".to_owned()),
            selected_port: Some(8689),
            selected_base_url: Some("http://127.0.0.1:8689".to_owned()),
            answer: Some("I cannot help with that request.".to_owned()),
            elapsed_ms: Some(7),
            answer_chars: Some(32),
            answer_bytes: Some(32),
            answer_approx_tokens: Some(8),
        };

        normalize_contract_answer(&input, &mut result);

        let answer = result.answer.as_deref().unwrap();
        assert!(answer.contains("route_intent: index"));
        assert!(answer.contains("tool_call: null"));
        assert!(answer.contains("preflight: allow because router@8689"));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn test_gate_result_falls_back_to_safe_validation_command_when_model_outputs_none() {
        let validation = passed_validation_evidence();
        let mut test_gate = input();
        test_gate.task_kind = "test-gate";
        test_gate.validation_evidence = Some(&validation);
        let mut result = PoolStageCallResult {
            task_kind: "test-gate".to_owned(),
            ok: true,
            selected_role: Some("test-gate".to_owned()),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            answer: Some("verdict: pass\nvalidation_command: None\nfailure_kind: none".to_owned()),
            elapsed_ms: Some(7),
            answer_chars: Some(58),
            answer_bytes: Some(58),
            answer_approx_tokens: Some(15),
        };

        normalize_contract_answer(&test_gate, &mut result);

        let answer = result.answer.as_deref().unwrap();
        assert!(answer.contains("verdict: pass"));
        assert!(
            answer.contains(
                "validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\evolution-loop-daemon-check"
            ),
            "{answer}"
        );
        assert!(answer.contains("failure_kind: none"));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn test_gate_result_does_not_pass_without_validation_evidence() {
        let mut test_gate = input();
        test_gate.task_kind = "test-gate";
        let mut result = PoolStageCallResult {
            task_kind: "test-gate".to_owned(),
            ok: true,
            selected_role: Some("test-gate".to_owned()),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            answer: Some(
                "verdict: pass\nvalidation_command: cargo check --manifest-path tools/evolution-loop/Cargo.toml\nfailure_kind: none"
                    .to_owned(),
            ),
            elapsed_ms: Some(7),
            answer_chars: Some(109),
            answer_bytes: Some(109),
            answer_approx_tokens: Some(28),
        };

        normalize_contract_answer(&test_gate, &mut result);

        let answer = result.answer.as_deref().unwrap();
        assert!(answer.contains("verdict: warn"));
        assert!(answer.contains("failure_kind: missing_evidence"));
        assert!(answer.contains(
            "validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --no-fail-fast"
        ));
    }

    #[test]
    fn index_stage_prompt_links_tags_to_completed_review_dependency() {
        let plan = index_plan();
        let completed_roles = vec![
            "quality".to_owned(),
            "summary".to_owned(),
            "router".to_owned(),
            "review".to_owned(),
        ];
        let input = PoolStageCallInput {
            task_kind: "index",
            case_name: "case-42",
            round: 42,
            validation_timestamp_unix: Some(1_781_770_123),
            validation_evidence: None,
            original_prompt: "Check index quality",
            primary_answer: Some("Review requested stable tags."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &completed_roles,
            max_tokens: plan.effective_max_tokens,
        };

        let prompt = stage_prompt(&input);

        assert!(prompt.contains("SmartSteam index helper"));
        assert!(prompt.contains("selected_role: index"));
        assert!(prompt.contains("selected_port: 8690"));
        assert!(prompt.contains(
            "tags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123"
        ));
        assert!(prompt.contains("dependency_link: review.change_request"));
        assert!(prompt.contains("source_origin: review.change_request"));
        assert!(prompt.contains("validation_timestamp: 1781770123"));
        assert!(prompt.contains("tags must include role=index, case, round, primary, final_json"));
        assert!(prompt.contains("dependency_link must name the upstream helper field"));
        assert!(prompt.contains("source_origin must repeat the concrete upstream helper field"));
        assert!(!prompt.contains("comma-separated retrieval tags"));
    }

    #[test]
    fn index_result_falls_back_to_stable_tags_when_model_outputs_placeholder_tags() {
        let plan = index_plan();
        let completed_roles = vec![
            "quality".to_owned(),
            "summary".to_owned(),
            "router".to_owned(),
            "review".to_owned(),
        ];
        let input = PoolStageCallInput {
            task_kind: "index",
            case_name: "case-42",
            round: 42,
            validation_timestamp_unix: Some(1_781_770_123),
            validation_evidence: None,
            original_prompt: "Check index quality",
            primary_answer: Some("Review requested stable tags."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &completed_roles,
            max_tokens: plan.effective_max_tokens,
        };
        let mut result = PoolStageCallResult {
            task_kind: "index".to_owned(),
            ok: true,
            selected_role: Some("index".to_owned()),
            selected_port: Some(8690),
            selected_base_url: Some("http://127.0.0.1:8690".to_owned()),
            answer: Some(
                "clean_gist: compact searchable summary\ntags: comma-separated retrieval tags\nretention: keep"
                    .to_owned(),
            ),
            elapsed_ms: Some(9),
            answer_chars: Some(88),
            answer_bytes: Some(88),
            answer_approx_tokens: Some(22),
        };

        normalize_contract_answer(&input, &mut result);

        let answer = result.answer.as_deref().unwrap();
        assert!(answer.contains("clean_gist: Index round 42 case-42 with index@8690"));
        assert!(answer.contains(
            "tags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123"
        ));
        assert!(answer.contains("dependency_link: review.change_request"));
        assert!(answer.contains("source_origin: review.change_request"));
        assert!(answer.contains("validation_timestamp: 1781770123"));
        assert!(answer.contains("retention: keep; compact retrieval evidence"));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn index_result_keeps_stable_key_value_tags() {
        let plan = index_plan();
        let input = PoolStageCallInput {
            task_kind: "index",
            case_name: "case-42",
            round: 42,
            validation_timestamp_unix: Some(1_781_770_123),
            validation_evidence: None,
            original_prompt: "Check index quality",
            primary_answer: Some("Review requested stable tags."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &[],
            max_tokens: plan.effective_max_tokens,
        };
        let answer = "clean_gist: stable retrieval labels are present\ntags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=primary.evidence;source_origin=primary.evidence;validation_timestamp=1781770123\ndependency_link: primary.evidence\nsource_origin: primary.evidence\nvalidation_timestamp: 1781770123\nretention: keep; labels are compact";
        let mut result = PoolStageCallResult {
            task_kind: "index".to_owned(),
            ok: true,
            selected_role: Some("index".to_owned()),
            selected_port: Some(8690),
            selected_base_url: Some("http://127.0.0.1:8690".to_owned()),
            answer: Some(answer.to_owned()),
            elapsed_ms: Some(9),
            answer_chars: Some(answer.chars().count() as u64),
            answer_bytes: Some(answer.len() as u64),
            answer_approx_tokens: Some(37),
        };

        normalize_contract_answer(&input, &mut result);

        assert_eq!(result.answer.as_deref(), Some(answer));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn index_result_keeps_contract_when_clean_gist_mentions_tags() {
        let plan = index_plan();
        let input = PoolStageCallInput {
            task_kind: "index",
            case_name: "case-42",
            round: 42,
            validation_timestamp_unix: Some(1_781_770_123),
            validation_evidence: None,
            original_prompt: "Check index quality",
            primary_answer: Some("Review requested stable tags."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &[],
            max_tokens: plan.effective_max_tokens,
        };
        let answer = "clean_gist: stable tags are present in the compact index contract\ntags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=primary.evidence;source_origin=primary.evidence;validation_timestamp=1781770123\ndependency_link: primary.evidence\nsource_origin: primary.evidence\nvalidation_timestamp: 1781770123\nretention: keep; labels are compact";
        let mut result = PoolStageCallResult {
            task_kind: "index".to_owned(),
            ok: true,
            selected_role: Some("index".to_owned()),
            selected_port: Some(8690),
            selected_base_url: Some("http://127.0.0.1:8690".to_owned()),
            answer: Some(answer.to_owned()),
            elapsed_ms: Some(9),
            answer_chars: Some(answer.chars().count() as u64),
            answer_bytes: Some(answer.len() as u64),
            answer_approx_tokens: Some(52),
        };

        normalize_contract_answer(&input, &mut result);

        assert_eq!(result.answer.as_deref(), Some(answer));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn index_result_falls_back_when_dependency_link_disagrees_with_tags() {
        let plan = index_plan();
        let completed_roles = vec![
            "quality".to_owned(),
            "summary".to_owned(),
            "review".to_owned(),
        ];
        let input = PoolStageCallInput {
            task_kind: "index",
            case_name: "case-42",
            round: 42,
            validation_timestamp_unix: Some(1_781_770_123),
            validation_evidence: None,
            original_prompt: "Check index quality",
            primary_answer: Some("Review requested stable tags."),
            final_json: Some("{\"ok\":true}"),
            dispatch_plan: Some(&plan),
            completed_roles: &completed_roles,
            max_tokens: plan.effective_max_tokens,
        };
        let answer = "clean_gist: stable retrieval labels are present\ntags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=summary.next_context;source_origin=summary.next_context;validation_timestamp=1781770123\ndependency_link: review.change_request\nsource_origin: summary.next_context\nvalidation_timestamp: 1781770123\nretention: keep; labels are compact";
        let mut result = PoolStageCallResult {
            task_kind: "index".to_owned(),
            ok: true,
            selected_role: Some("index".to_owned()),
            selected_port: Some(8690),
            selected_base_url: Some("http://127.0.0.1:8690".to_owned()),
            answer: Some(answer.to_owned()),
            elapsed_ms: Some(9),
            answer_chars: Some(answer.chars().count() as u64),
            answer_bytes: Some(answer.len() as u64),
            answer_approx_tokens: Some(50),
        };

        normalize_contract_answer(&input, &mut result);

        let normalized = result.answer.as_deref().unwrap();
        assert_ne!(normalized, answer);
        assert!(normalized.contains(
            "tags: role=index;case=case-42;round=42;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123"
        ));
        assert!(normalized.contains("dependency_link: review.change_request"));
        assert!(normalized.contains("source_origin: review.change_request"));
        assert_eq!(result.answer_chars, Some(normalized.chars().count() as u64));
    }

    #[test]
    fn test_gate_result_keeps_safe_validation_command() {
        let answer = "verdict: pass\nvalidation_command: cargo check --manifest-path tools/evolution-loop/Cargo.toml\nfailure_kind: none";
        let validation = passed_validation_evidence();
        let mut test_gate = input();
        test_gate.task_kind = "test-gate";
        test_gate.validation_evidence = Some(&validation);
        let mut result = PoolStageCallResult {
            task_kind: "test-gate".to_owned(),
            ok: true,
            selected_role: Some("test-gate".to_owned()),
            selected_port: Some(8688),
            selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
            answer: Some(answer.to_owned()),
            elapsed_ms: Some(7),
            answer_chars: Some(answer.chars().count() as u64),
            answer_bytes: Some(answer.len() as u64),
            answer_approx_tokens: Some(28),
        };

        normalize_contract_answer(&test_gate, &mut result);

        assert_eq!(result.answer.as_deref(), Some(answer));
        assert_eq!(result.answer_chars, Some(answer.chars().count() as u64));
    }

    #[test]
    fn request_body_carries_completed_roles_for_dependency_precheck() {
        let completed_roles = vec!["quality".to_owned(), "summary".to_owned()];
        let input = PoolStageCallInput {
            completed_roles: &completed_roles,
            ..input()
        };

        let body = request_body(&input);

        assert!(body.contains("\"completed_roles\":[\"quality\",\"summary\"]"));
    }

    #[test]
    fn parses_pool_call_execution_metrics() {
        let parsed = parse_response(
            "review",
            "{\"ok\":true,\"task_kind\":\"review\",\"selected_role\":\"review\",\"selected_port\":8688,\"selected_base_url\":\"http://127.0.0.1:8688\",\"elapsed_ms\":123,\"answer_chars\":40,\"answer_bytes\":42,\"answer_approx_tokens\":10,\"answer\":\"looks good\"}",
        );

        assert!(parsed.ok);
        assert_eq!(parsed.task_kind, "review");
        assert_eq!(parsed.selected_role.as_deref(), Some("review"));
        assert_eq!(parsed.selected_port, Some(8688));
        assert_eq!(
            parsed.selected_base_url.as_deref(),
            Some("http://127.0.0.1:8688")
        );
        assert_eq!(parsed.elapsed_ms, Some(123));
        assert_eq!(parsed.answer_chars, Some(40));
        assert_eq!(parsed.answer_bytes, Some(42));
        assert_eq!(parsed.answer_approx_tokens, Some(10));
        assert_eq!(parsed.answer.as_deref(), Some("looks good"));
    }
}
