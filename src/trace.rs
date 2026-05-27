use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

use crate::engine::InferenceOutcome;
use crate::hierarchy::TaskProfile;

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
         \"router_threshold_after\":{:.6},\
         \"route\":{{\"threshold\":{:.6},\"attention_fraction\":{:.6},\"attention_tokens\":{},\"fast_tokens\":{}}},\
         \"hierarchy\":{{\"global\":{:.6},\"local\":{:.6},\"convolution\":{:.6}}},\
         \"hardware\":{{\"device\":\"{}\",\"tier\":\"{}\",\"pressure\":{:.6},\"latency_budget_ms\":{},\"execution\":{{\"primary_lane\":\"{}\",\"fallback_lane\":\"{}\",\"memory_mode\":\"{}\",\"max_parallel_chunks\":{},\"kv_prefetch_blocks\":{},\"hot_kv_bits\":{},\"cold_kv_bits\":{},\"disk_spill\":{},\"adapter_hints\":{}}}}},\
         \"recursive\":{{\"required\":{},\"prompt_tokens\":{},\"native_window\":{},\"chunks\":{},\"merge_rounds\":{},\"execution_waves\":{},\"max_parallel_chunks\":{},\"chunk_tokens\":{},\"overlap_tokens\":{}}},\
         \"tiers\":{{\"hot_gpu\":{},\"warm_ram\":{},\"cold_disk\":{}}},\
         \"infini_memory\":{{\"local_window\":{},\"global_memory\":{},\"sparse_skipped\":{},\"local_tokens\":{},\"global_tokens\":{},\"skipped_tokens\":{}}},\
         \"transformer\":{{\"global\":{},\"local\":{},\"convolution\":{}}},\
         \"stream_windows\":{},\
         \"memory\":{{\"used\":{},\"stored\":{},\"gist_records\":{},\"gist_stored\":{},\"runtime_kv_exported\":{},\"runtime_kv_stored\":{}}},\
         \"drift\":{{\"severity\":\"{}\",\"memory_write\":{},\"runtime_kv_write\":{},\"penalize_used_memory\":{},\"rollback_adaptive\":{},\"notes\":{}}},\
         \"process_reward\":{{\"total\":{:.6},\"action\":\"{}\",\"route\":{:.6},\"memory\":{:.6},\"hierarchy\":{:.6},\"reflection\":{:.6},\"latency\":{:.6},\"admission\":{:.6},\"notes\":{}}},\
         \"retention\":{{\"before\":{},\"after\":{},\"decayed\":{},\"removed\":{}}},\
         \"experience_id\":{}\
         }}",
        option_string_json(case_name),
        profile,
        prompt.chars().count(),
        json_escape(&compact(prompt, 160)),
        elapsed_ms,
        outcome.report.quality,
        outcome.metrics.perplexity,
        outcome.router_threshold_after,
        outcome.route_budget.threshold,
        outcome.route_budget.attention_fraction,
        outcome.route_budget.attention_tokens,
        outcome.route_budget.fast_tokens,
        outcome.hierarchy.global,
        outcome.hierarchy.local,
        outcome.hierarchy.convolution,
        outcome.hardware_plan.device.as_str(),
        outcome.hardware_plan.tier.as_str(),
        outcome.hardware_plan.pressure,
        option_u64_json(outcome.hardware_plan.latency_budget_ms),
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
        tier_counts.hot_gpu,
        tier_counts.warm_ram,
        tier_counts.cold_disk,
        infini_counts.local_window,
        infini_counts.global_memory,
        infini_counts.skipped,
        infini_counts.local_tokens,
        infini_counts.global_tokens,
        infini_counts.skipped_tokens,
        transformer_counts.global,
        transformer_counts.local,
        transformer_counts.convolution,
        outcome.stream_reports.len(),
        outcome.used_memories.len(),
        option_u64_json(outcome.stored_memory_id),
        outcome.gist_records.len(),
        outcome.stored_gist_memory_ids.len(),
        outcome.exported_runtime_kv_blocks,
        outcome.stored_runtime_kv_memory_ids.len(),
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
        outcome.retention_report.before,
        outcome.retention_report.after,
        outcome.retention_report.decayed,
        outcome.retention_report.removed.len(),
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

fn option_string_json(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_owned())
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
        assert!(line.contains("\"route\":"));
        assert!(line.contains("\"hierarchy\":"));
        assert!(line.contains("\"primary_lane\":"));
        assert!(line.contains("\"adapter_hints\":"));
        assert!(line.contains("\"execution_waves\":"));
        assert!(line.contains("\"max_parallel_chunks\":"));
        assert!(line.contains("\"drift\":"));
        assert!(line.contains("\"process_reward\":"));
        assert!(line.contains("\"runtime_kv_exported\":"));
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
}
