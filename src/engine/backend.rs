use crate::experience::render_experience_hint;
use crate::hierarchy::TaskProfile;
use crate::reflection::{DraftToken, InferenceDraft, ReasoningStep, RuntimeDiagnostics};

use super::text::compact;
use super::types::{GenerationContext, InferenceBackend};

#[derive(Debug, Clone)]
pub struct HeuristicBackend;

impl InferenceBackend for HeuristicBackend {
    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        let prompt_anchor = context
            .prompt
            .split_once('\n')
            .filter(|(control, _)| control.starts_with("[noiron-dna "))
            .map(|(_, prompt)| prompt)
            .unwrap_or(context.prompt);
        let memory_summary = if context.memories.is_empty() {
            "no prior memory".to_owned()
        } else {
            context
                .memories
                .iter()
                .take(2)
                .map(|item| format!("{} ({:.2})", item.key, item.similarity))
                .collect::<Vec<_>>()
                .join("; ")
        };
        let profile_hint = match context.profile {
            TaskProfile::General => "balanced global/local/convolution routing",
            TaskProfile::Coding => "strong local-window attention for syntax and interfaces",
            TaskProfile::Writing => "strong global attention for long-range continuity",
            TaskProfile::LongDocument => "strong convolutional fusion for long context compression",
        };
        let tier_counts = context.tier_plan.counts();
        let infini_counts = context.infini_memory_plan.counts();
        let recursive_schedule = context.recursive_schedule;
        let hardware_plan = context.hardware_plan;
        let transformer_counts = context.transformer_plan.counts();
        let toolsmith_summary = context.toolsmith_plan.summary();
        let agent_team_summary = context.agent_team_plan.summary();
        let agent_team_aggregation = context.agent_team_plan.aggregation.summary();
        let agent_team_messages = if context.agent_team_plan.messages.is_empty() {
            "none".to_owned()
        } else {
            context.agent_team_plan.message_summaries(3).join("; ")
        };
        let toolsmith_blueprints = if context.toolsmith_plan.blueprints.is_empty() {
            "none".to_owned()
        } else {
            context
                .toolsmith_plan
                .blueprints
                .iter()
                .take(2)
                .map(|blueprint| blueprint.summary())
                .collect::<Vec<_>>()
                .join("; ")
        };
        let experience_summary = if context.experiences.is_empty() {
            "no prior experience".to_owned()
        } else {
            context
                .experiences
                .iter()
                .take(2)
                .map(render_experience_hint)
                .collect::<Vec<_>>()
                .join("; ")
        };

        let answer = format!(
            "Prototype inference result: keep Noiron as a control layer around the model backend. \
             Use multi-factor routing for projection, local-window attention, global attention, \
             and convolutional fusion decisions; reinforced KV fusion for local memory; task-aware \
             hierarchy weights for compute allocation; and reflection to score each draft before \
             storing it. Profile hint: {profile_hint}. Prompt anchor: {}. Memory hints: {memory_summary}. \
             Experience hints: {experience_summary}. \
             Route budget: {:.0}% attention, {} fast tokens, {} attention tokens. \
             Tier plan: {} hot GPU, {} warm RAM, {} cold disk memories. \
             Infini memory: {} local-window ({} tokens), {} global ({} tokens), {} sparse-skipped ({} tokens) memories. \
             Recursive schedule: required={}, {} chunks, {} merge rounds, {} execution waves, max parallel {}, {} prompt tokens, native window {}. \
             Hardware plan: {}. \
             Transformer plan: template {}, {} global, {} local, {} convolution layers. \
             Toolsmith plan: {toolsmith_summary}. Tool blueprints: {toolsmith_blueprints}. \
             Agent team: {agent_team_summary}. Team aggregation: {agent_team_aggregation}. \
             Team messages: {agent_team_messages}.",
            compact(prompt_anchor, 120),
            context.route_budget.attention_fraction * 100.0,
            context.route_budget.fast_tokens,
            context.route_budget.attention_tokens,
            tier_counts.hot_gpu,
            tier_counts.warm_ram,
            tier_counts.cold_disk,
            infini_counts.local_window,
            infini_counts.local_tokens,
            infini_counts.global_memory,
            infini_counts.global_tokens,
            infini_counts.skipped,
            infini_counts.skipped_tokens,
            recursive_schedule.requires_recursion,
            recursive_schedule.chunk_count(),
            recursive_schedule.merge_round_count(),
            recursive_schedule.execution_wave_count(),
            recursive_schedule.max_parallel_chunks,
            recursive_schedule.prompt_tokens,
            recursive_schedule.native_window_tokens,
            hardware_plan.summary(),
            context.transformer_plan.template_name(),
            transformer_counts.global,
            transformer_counts.local,
            transformer_counts.convolution
        );

        InferenceDraft::new(
            answer.clone(),
            vec![
                ReasoningStep::new(
                    "route",
                    "combined entropy, task profile, context, cache, and latency signals",
                    0.82,
                ),
                ReasoningStep::new("memory", "looked up similar reinforced KV memories", 0.78),
                ReasoningStep::new(
                    "recursive_schedule",
                    "planned single-pass or chunk/merge control for native-window limits",
                    0.77,
                ),
                ReasoningStep::new(
                    "reflection",
                    "draft will be scored before reinforcement",
                    0.84,
                ),
                ReasoningStep::new(
                    "toolsmith",
                    "planned Rust-only tool blueprints behind local safety gates",
                    0.80,
                ),
                ReasoningStep::new(
                    "agent_team",
                    "coordinated read-only sub-agent lanes through a summarized blackboard",
                    0.82,
                ),
            ],
        )
        .with_tokens(heuristic_tokens(&answer))
        .with_runtime_diagnostics(heuristic_runtime_diagnostics())
    }
}

fn heuristic_tokens(answer: &str) -> Vec<DraftToken> {
    let mut tokens = Vec::new();
    let mut token = String::new();

    for character in answer.chars() {
        token.push(character);
        if character.is_whitespace() && token.chars().any(|item| !item.is_whitespace()) {
            tokens.push(DraftToken::new(std::mem::take(&mut token)));
        }
    }

    if !token.is_empty() {
        tokens.push(DraftToken::new(token));
    }

    tokens
}

fn heuristic_runtime_diagnostics() -> RuntimeDiagnostics {
    RuntimeDiagnostics {
        model_id: Some("rust-norion-heuristic-local".to_owned()),
        ..RuntimeDiagnostics::default()
    }
}
