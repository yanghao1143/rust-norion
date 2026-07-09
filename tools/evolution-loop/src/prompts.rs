use std::fs;
use std::path::Path;

use crate::args::Config;
use crate::pool_artifacts;
use crate::pool_stage;
use crate::report;

const DEFAULT_PROMPTS: &[&str] = &[
    "Review the latest SmartSteam Forge local Gemma integration. Return one concrete improvement, one risk, and one verification step for the next iteration.",
    "Analyze rust-norion's business-cycle loop as a coding assistant. Suggest a small change that improves reliability without increasing coupling.",
    "Act as a strict Rust reviewer for SmartSteam. Identify the highest-value test or gate to add before the next unattended evolution round.",
    "Summarize one lesson the system should remember about local Gemma inference speed, context budget, and safe device usage.",
];

#[cfg(test)]
pub(crate) fn load_prompts(config: &Config) -> Result<Vec<String>, String> {
    let prompts = load_base_prompts(config)?;
    prompts
        .into_iter()
        .map(|prompt| prompt_with_current_context(config, &prompt))
        .collect()
}

pub(crate) fn load_base_prompts(config: &Config) -> Result<Vec<String>, String> {
    let prompts = if let Some(prompt) = &config.prompt {
        let prompt = prompt.trim();
        if !prompt.is_empty() {
            vec![prompt.to_owned()]
        } else if let Some(path) = &config.prompt_file {
            load_non_empty_prompt_file(path)?
        } else {
            default_prompts()
        }
    } else if let Some(path) = &config.prompt_file {
        load_non_empty_prompt_file(path)?
    } else {
        default_prompts()
    };
    Ok(prompts)
}

#[cfg(test)]
pub(crate) fn prompt_with_current_context(config: &Config, prompt: &str) -> Result<String, String> {
    prompt_with_current_context_limited(config, prompt, None)
}

pub(crate) fn prompt_with_current_context_limited(
    config: &Config,
    prompt: &str,
    max_context_chars: Option<usize>,
) -> Result<String, String> {
    if !config.report_context {
        return Ok(prompt.to_owned());
    }
    let mut contexts = Vec::new();
    if let Some(context) = report::prompt_context_with_auto_accept(
        &config.ledger_path,
        config.auto_accept_validated_self_improve_memory,
    )? {
        contexts.push(context);
    }
    let pool_manifest = if let Some(path) = &config.pool_manifest_json_path {
        let summary = pool_artifacts::load_manifest(Some(path))?;
        if let Some(summary) = summary.as_ref() {
            contexts.push(format!(
                "Current SmartSteam model pool manifest:\n{}",
                pool_artifacts::manifest_context_text(summary)
            ));
        }
        summary
    } else {
        None
    };
    let pool_status = if let Some(path) = &config.pool_status_json_path {
        let summary = pool_artifacts::load_status(Some(path))?;
        if let Some(summary) = summary.as_ref() {
            contexts.push(format!(
                "Current SmartSteam model pool status:\n{}",
                pool_artifacts::status_context_text(summary)
            ));
        }
        summary
    } else {
        None
    };
    let mut pool_routes = Vec::new();
    if let Some(path) = &config.pool_route_json_path {
        let summary = pool_artifacts::load_route(Some(path))?;
        if let Some(summary) = summary {
            contexts.push(format!(
                "Current SmartSteam model pool route plan:\n{}",
                pool_artifacts::route_context_text(&summary)
            ));
            pool_routes.push(summary);
        }
    }
    for (task_kind, route) in pool_stage::route_summaries(config)? {
        contexts.push(format!(
            "Current SmartSteam model pool stage route [{task_kind}]:\n{}",
            pool_artifacts::route_context_text(&route)
        ));
        pool_routes.push(route);
    }
    if pool_manifest.is_some() || pool_status.is_some() || !pool_routes.is_empty() {
        let alignment = pool_artifacts::alignment_summary(
            pool_manifest.as_ref(),
            pool_status.as_ref(),
            &pool_routes,
        );
        contexts.push(format!(
            "Current SmartSteam model pool alignment:\n{}",
            pool_artifacts::alignment_context_text(&alignment)
        ));
    }
    if contexts.is_empty() {
        return Ok(prompt.to_owned());
    }
    let context = limit_context_chars(&contexts.join("\n\n"), max_context_chars);
    Ok(with_report_context(prompt, &context))
}

fn default_prompts() -> Vec<String> {
    DEFAULT_PROMPTS
        .iter()
        .map(|prompt| (*prompt).to_owned())
        .collect()
}

fn load_non_empty_prompt_file(path: &Path) -> Result<Vec<String>, String> {
    let prompts = load_prompt_file(path)?;
    if prompts.is_empty() {
        Err(format!(
            "prompt file had no non-empty lines: {}",
            path.display()
        ))
    } else {
        Ok(prompts)
    }
}

fn with_report_context(prompt: &str, context: &str) -> String {
    format!(
        "{prompt}\n\nPrevious SmartSteam evolution ledger summary:\n{context}\n\nUse summary evidence. Pick one new testable improvement. Hard exclusions: completed, invalid, blocked, repeated. Do not rephrase exclusions. Choose a different concrete topic."
    )
}

fn limit_context_chars(context: &str, max_chars: Option<usize>) -> String {
    let Some(max_chars) = max_chars.filter(|max_chars| *max_chars > 0) else {
        return context.to_owned();
    };
    if context.chars().count() <= max_chars {
        return context.to_owned();
    }
    if max_chars < 80 {
        return context.chars().take(max_chars).collect();
    }

    let marker = "\n\n[context truncated for selected worker context budget]\n\n";
    let marker_chars = marker.chars().count();
    let remaining = max_chars.saturating_sub(marker_chars);
    let head_chars = remaining.saturating_mul(2) / 3;
    let tail_chars = remaining.saturating_sub(head_chars);
    let head = context.chars().take(head_chars).collect::<String>();
    let tail_start = context.chars().count().saturating_sub(tail_chars);
    let tail = context.chars().skip(tail_start).collect::<String>();
    format!("{head}{marker}{tail}")
}

pub(crate) fn approximate_prompt_tokens(text: &str) -> usize {
    // Conservative enough for mixed Chinese/ASCII prompts and avoids tokenizer coupling here.
    text.chars().count().div_ceil(2)
}

fn load_prompt_file(path: &Path) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("read prompt file {} failed: {error}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "smartsteam-evolution-{name}-{}.jsonl",
            std::process::id()
        ))
    }

    #[test]
    fn default_prompts_are_available() {
        let prompts = load_prompts(&Config {
            ledger_path: unique_temp_path("missing-ledger"),
            ..Config::default()
        })
        .unwrap();

        assert!(prompts.len() >= 3);
        assert!(prompts[0].contains("SmartSteam"));
    }

    #[test]
    fn prompt_file_must_have_non_empty_lines() {
        let path = unique_temp_path("empty-prompts");
        let _ = fs::remove_file(&path);
        fs::write(&path, "\n# skip\n").unwrap();

        let result = load_prompts(&Config {
            prompt_file: Some(path.clone()),
            ledger_path: unique_temp_path("missing-ledger"),
            ..Config::default()
        });

        assert!(result.unwrap_err().contains("no non-empty lines"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn injects_previous_report_context_when_ledger_exists() {
        let ledger = unique_temp_path("ledger-context");
        let _ = fs::remove_file(&ledger);
        fs::write(
            &ledger,
            "{\"round\":1,\"case\":\"case-1\",\"success\":true,\"feedback_applied\":2,\"self_improve_passed\":true,\"eval\":{\"report_only\":true,\"failure_kind\":\"chain_not_ready\"}}\n",
        )
        .unwrap();

        let prompts = load_prompts(&Config {
            prompt: Some("next task".to_owned()),
            ledger_path: ledger.clone(),
            ..Config::default()
        })
        .unwrap();

        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].contains("next task"));
        assert!(prompts[0].contains("previous_rounds=1"));
        assert!(prompts[0].contains("feedback_total=2"));
        assert!(prompts[0].contains("eval_report_only=records:1 report_only:1"));
        assert!(prompts[0].contains("failure_kinds:chain_not_ready:1"));
        let _ = fs::remove_file(ledger);
    }

    #[test]
    fn injects_repeated_successful_advice_guard() {
        let ledger = unique_temp_path("ledger-repeated-success");
        let _ = fs::remove_file(&ledger);
        fs::write(
            &ledger,
            "{\"round\":1,\"case\":\"repeat-1\",\"success\":true,\"feedback_applied\":4,\"answer\":\"**Improvement:** Increase the test-gate selected max tokens from 768 to 1024. **Verifiable Evidence:** review requested it.\"}\n\
{\"round\":2,\"case\":\"repeat-2\",\"success\":true,\"feedback_applied\":4,\"answer\":\"**Improvement:** Increase the test-gate selected max tokens from 768 to 1024. **Verifiable Evidence:** route budget shows it.\"}\n\
{\"round\":3,\"case\":\"other\",\"success\":true,\"feedback_applied\":4,\"answer\":\"**Improvement:** Add a ledger report field. **Verifiable Evidence:** JSON contains it.\"}\n\
{\"round\":4,\"case\":\"repeat-3\",\"success\":true,\"feedback_applied\":4,\"answer\":\"**Improvement:** Increase the test-gate selected max tokens from 768 to 1024. **Verifiable Evidence:** prompt context shows it.\"}\n",
        )
        .unwrap();

        let prompts = load_prompts(&Config {
            prompt: Some("next task".to_owned()),
            ledger_path: ledger.clone(),
            ..Config::default()
        })
        .unwrap();

        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].contains("recent_repeated_successful_answer=count:3"));
        assert!(prompts[0].contains("preview_redacted:true"));
        assert!(prompts[0].contains("next_advice_should_not_repeat_recent_successful_answer:true"));
        assert!(
            prompts[0]
                .contains("next_advice_must_not_use_repeated_answer_preview_as_evidence:true")
        );
        assert!(
            !prompts[0].contains("Increase the test-gate selected max tokens"),
            "{}",
            prompts[0]
        );
        assert!(prompts[0].contains("Do not rephrase exclusions"));
        assert!(prompts[0].contains("Choose a different concrete topic"));
        assert!(prompts[0].contains("Hard exclusions:"));
        let _ = fs::remove_file(ledger);
    }

    #[test]
    fn prompt_context_can_be_refreshed_between_rounds() {
        let ledger = unique_temp_path("ledger-refresh-context");
        let _ = fs::remove_file(&ledger);
        let config = Config {
            ledger_path: ledger.clone(),
            ..Config::default()
        };

        let first = prompt_with_current_context(&config, "next task").unwrap();
        assert_eq!(first, "next task");

        fs::write(
            &ledger,
            "{\"round\":1,\"case\":\"helper\",\"success\":true,\"feedback_applied\":2,\"meta\":[\"pool_stage_call_answer task_kind=summary role=summary elapsed_ms=111 answer_approx_tokens=4 preview=memory_update: keep Metal evidence\"]}\n",
        )
        .unwrap();

        let second = prompt_with_current_context(&config, "next task").unwrap();

        assert!(second.contains("next task"));
        assert!(second.contains("previous_rounds=1"));
        assert!(second.contains("recent_helper_stage_feedback_by_role="));
        assert!(second.contains("summary:task_kind=summary"));
        assert!(second.contains("memory_update: keep Metal evidence"));
        let _ = fs::remove_file(ledger);
    }

    #[test]
    fn auto_accepted_self_improve_memory_feeds_next_prompt_context() {
        let ledger = unique_temp_path("ledger-auto-accept-context");
        let _ = fs::remove_file(&ledger);
        fs::write(
            &ledger,
            "{\"round\":33,\"case\":\"helper-proposal\",\"success\":true,\"feedback_applied\":4,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"promote validated helper proposal into memory evidence\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n",
        )
        .unwrap();

        let prompt = prompt_with_current_context(
            &Config {
                prompt: Some("next task".to_owned()),
                ledger_path: ledger.clone(),
                auto_accept_validated_self_improve_memory: true,
                ..Config::default()
            },
            "next task",
        )
        .unwrap();

        assert!(prompt.contains("self_improve_proposal_acceptance="));
        assert!(prompt.contains("accepted_memory:1"));
        assert!(prompt.contains("evidence_backed_business:1"));
        assert!(prompt.contains("advisory_only:0"));
        let _ = fs::remove_file(ledger);
    }

    #[test]
    fn can_disable_report_context_injection() {
        let ledger = unique_temp_path("no-context");
        let _ = fs::remove_file(&ledger);
        fs::write(
            &ledger,
            "{\"round\":1,\"case\":\"case-1\",\"success\":true,\"feedback_applied\":2}\n",
        )
        .unwrap();

        let prompts = load_prompts(&Config {
            prompt: Some("next task".to_owned()),
            ledger_path: ledger.clone(),
            report_context: false,
            ..Config::default()
        })
        .unwrap();

        assert_eq!(prompts, vec!["next task".to_owned()]);
        let _ = fs::remove_file(ledger);
    }

    #[test]
    fn can_limit_report_context_for_small_worker_windows() {
        let ledger = unique_temp_path("limited-context");
        let _ = fs::remove_file(&ledger);
        fs::write(
            &ledger,
            format!(
                "{{\"round\":1,\"case\":\"case-1\",\"success\":true,\"feedback_applied\":2,\"answer\":\"{}\"}}\n",
                "long-context ".repeat(400)
            ),
        )
        .unwrap();

        let prompt = prompt_with_current_context_limited(
            &Config {
                prompt: Some("next task".to_owned()),
                ledger_path: ledger.clone(),
                ..Config::default()
            },
            "next task",
            Some(240),
        )
        .unwrap();

        assert!(prompt.contains("next task"));
        assert!(prompt.contains("context truncated for selected worker context budget"));
        assert!(prompt.chars().count() < 520);
        let _ = fs::remove_file(ledger);
    }

    #[test]
    fn injects_pool_status_context_when_artifact_is_provided() {
        let ledger = unique_temp_path("ledger-with-pool");
        let pool_status = unique_temp_path("pool-status");
        let _ = fs::remove_file(&ledger);
        let _ = fs::remove_file(&pool_status);
        fs::write(
            &ledger,
            "{\"round\":1,\"case\":\"case-1\",\"success\":true,\"feedback_applied\":2}\n",
        )
        .unwrap();
        fs::write(
            &pool_status,
            "{\"launch_allowed\":false,\"launch_block_reason\":\"quality_worker_down\",\"chain_classification\":\"quality_worker_down\",\"min_context_tokens\":262144,\"workers\":[{\"port\":8686,\"role\":\"quality\",\"tcp_reachable\":false,\"health_ok\":false},{\"port\":8687,\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();

        let prompts = load_prompts(&Config {
            prompt: Some("next task".to_owned()),
            ledger_path: ledger.clone(),
            pool_status_json_path: Some(pool_status.clone()),
            ..Config::default()
        })
        .unwrap();

        assert!(prompts[0].contains("previous_rounds=1"));
        assert!(prompts[0].contains("Current SmartSteam model pool status"));
        assert!(prompts[0].contains("launch_allowed:false"));
        assert!(prompts[0].contains("workers_reachable:1/2"));
        assert!(prompts[0].contains("roles:quality:unreachable,summary:healthy"));
        assert!(prompts[0].contains("available_roles:summary"));
        assert!(prompts[0].contains("advice:safe_to_enable_pool_workers:false"));
        assert!(prompts[0].contains("next_step:start_or_fix_quality_worker_8686"));
        let _ = fs::remove_file(ledger);
        let _ = fs::remove_file(pool_status);
    }

    #[test]
    fn injects_pool_manifest_context_when_artifact_is_provided() {
        let ledger = unique_temp_path("ledger-with-manifest");
        let manifest = unique_temp_path("pool-manifest");
        let _ = fs::remove_file(&ledger);
        let _ = fs::remove_file(&manifest);
        fs::write(
            &ledger,
            "{\"round\":1,\"case\":\"case-1\",\"success\":true,\"feedback_applied\":2}\n",
        )
        .unwrap();
        fs::write(
            &manifest,
            "{\"contract_version\":\"gemma-chain.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"manifest_kind\":\"rust-norion.model-pool\",\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"target_host\":\"apple_silicon\",\"avoid_extra_12b\":true,\"max_quality_12b_workers\":1,\"helper_roles\":[\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"recommended_launch_order\":[\"quality\",\"summary\",\"router\",\"review\",\"index\",\"test-gate\"],\"next_step_when_quality_ready\":\"run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls\"},\"workers\":[{\"role\":\"quality\",\"port\":8686,\"default_context_tokens\":262144,\"default_max_tokens\":262144,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":99},{\"role\":\"summary\",\"port\":8687,\"default_context_tokens\":8192,\"default_max_tokens\":768,\"runtime_backend\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_accelerator\":\"metal\",\"gpu_layers\":80}]}\n",
        )
        .unwrap();

        let prompts = load_prompts(&Config {
            prompt: Some("next task".to_owned()),
            ledger_path: ledger.clone(),
            pool_manifest_json_path: Some(manifest.clone()),
            ..Config::default()
        })
        .unwrap();

        assert!(prompts[0].contains("Current SmartSteam model pool manifest"));
        assert!(prompts[0].contains("policy:one_quality_plus_small_helpers"));
        assert!(prompts[0].contains("avoid_extra_12b:true"));
        assert!(prompts[0].contains("max_quality_12b_workers:1"));
        assert!(prompts[0].contains("helper_roles:summary,router,review,index,test-gate"));
        assert!(
            prompts[0]
                .contains("recommended_launch_order:quality,summary,router,review,index,test-gate")
        );
        assert!(prompts[0].contains("quality@8686"));
        assert!(prompts[0].contains("summary@8687"));
        let _ = fs::remove_file(ledger);
        let _ = fs::remove_file(manifest);
    }

    #[test]
    fn injects_pool_route_context_when_artifact_is_provided() {
        let ledger = unique_temp_path("ledger-with-route");
        let pool_route = unique_temp_path("pool-route");
        let _ = fs::remove_file(&ledger);
        let _ = fs::remove_file(&pool_route);
        fs::write(
            &ledger,
            "{\"round\":1,\"case\":\"case-1\",\"success\":true,\"feedback_applied\":2}\n",
        )
        .unwrap();
        fs::write(
            &pool_route,
            "{\"task_kind\":\"review\",\"route_allowed\":false,\"route_block_reason\":\"model_pool_launch_blocked:quality_worker_down\",\"selected_role\":null,\"role_candidates\":[\"review\",\"quality\"],\"candidate_workers\":[{\"role\":\"quality\",\"health_ok\":false,\"role_ready\":false},{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();

        let prompts = load_prompts(&Config {
            prompt: Some("next task".to_owned()),
            ledger_path: ledger.clone(),
            pool_route_json_path: Some(pool_route.clone()),
            ..Config::default()
        })
        .unwrap();

        assert!(prompts[0].contains("Current SmartSteam model pool route plan"));
        assert!(prompts[0].contains("task_kind:review"));
        assert!(prompts[0].contains("route_allowed:false"));
        assert!(prompts[0].contains("role_candidates:review,quality"));
        let _ = fs::remove_file(ledger);
        let _ = fs::remove_file(pool_route);
    }

    #[test]
    fn injects_pool_alignment_and_stage_route_context_when_artifacts_are_provided() {
        let dir = std::env::temp_dir().join(format!(
            "smartsteam-prompt-pool-alignment-{}",
            std::process::id()
        ));
        let ledger = dir.join("ledger.jsonl");
        let manifest = dir.join("pool-manifest.json");
        let status = dir.join("pool-status.json");
        let route = dir.join("pool-route-review.json");
        let summary = dir.join("pool-route-summary.json");
        let index = dir.join("pool-route-index.json");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            &ledger,
            "{\"round\":1,\"case\":\"case-1\",\"success\":true,\"feedback_applied\":2}\n",
        )
        .unwrap();
        fs::write(
            &manifest,
            "{\"capacity_policy\":{\"policy\":\"one_quality_plus_small_helpers\",\"max_quality_12b_workers\":1,\"quality_role\":\"quality\",\"helper_roles\":[\"summary\",\"review\",\"index\"]},\"workers\":[{\"role\":\"quality\",\"port\":8686},{\"role\":\"summary\",\"port\":8687},{\"role\":\"review\",\"port\":8688},{\"role\":\"index\",\"port\":8690}]}\n",
        )
        .unwrap();
        fs::write(
            &status,
            "{\"workers\":[{\"role\":\"quality\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"summary\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"review\",\"tcp_reachable\":true,\"health_ok\":true},{\"role\":\"index\",\"tcp_reachable\":true,\"health_ok\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &route,
            "{\"task_kind\":\"review\",\"route_allowed\":true,\"selected_role\":\"review\",\"candidate_workers\":[{\"role\":\"review\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &summary,
            "{\"task_kind\":\"summary\",\"route_allowed\":true,\"selected_role\":\"summary\",\"candidate_workers\":[{\"role\":\"summary\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();
        fs::write(
            &index,
            "{\"task_kind\":\"index\",\"route_allowed\":true,\"selected_role\":\"index\",\"candidate_workers\":[{\"role\":\"index\",\"health_ok\":true,\"role_ready\":true}]}\n",
        )
        .unwrap();

        let prompts = load_prompts(&Config {
            prompt: Some("next task".to_owned()),
            ledger_path: ledger.clone(),
            pool_manifest_json_path: Some(manifest),
            pool_status_json_path: Some(status),
            pool_route_json_path: Some(route),
            pool_stage_route_task_kinds: vec!["summary".to_owned(), "index".to_owned()],
            ..Config::default()
        })
        .unwrap();

        assert!(prompts[0].contains("Current SmartSteam model pool stage route [summary]"));
        assert!(prompts[0].contains("Current SmartSteam model pool stage route [index]"));
        assert!(prompts[0].contains("Current SmartSteam model pool alignment"));
        assert!(prompts[0].contains("alignment_ok:true"));
        assert!(prompts[0].contains("manifest_roles:quality,summary,review,index"));
        assert!(prompts[0].contains("status_roles:quality,summary,review,index"));
        assert!(prompts[0].contains("route_blocked_or_failed:none"));
        let _ = fs::remove_dir_all(dir);
    }
}
