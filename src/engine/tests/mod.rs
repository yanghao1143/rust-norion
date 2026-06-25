#![allow(clippy::too_many_arguments)]

use super::*;
use crate::experience_replay::{ExperienceReplayItem, RecursiveReplayStats};
use crate::hardware::{DeviceClass, RuntimeAdapterHint};
use crate::local_runtime::LocalTransformerRuntime;
use crate::process_reward::ProcessRewardComponents;
use crate::production_runtime::{
    ProductionForwardKernel, ProductionKernelContext, ProductionKernelOutput,
    ProductionTransformerRuntime,
};
use crate::reflection::{DraftToken, ReflectionIssue, ReflectionSeverity};
use crate::runtime::{RuntimeBackend, RuntimeError, RuntimeToken};
use crate::runtime_manifest::{
    RuntimeAssetPaths, RuntimeKvPolicy, RuntimeManifest, TransformerRuntimeArchitecture,
};
use crate::tiered_cache::TierMigrationAction;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod hardware_runtime;
mod inference;
mod persistence;
mod recursive;
mod replay;
mod reward_drift;
mod runtime_memory;

fn temp_path(label: &str, extension: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rust-norion-{label}-{}-{nanos}.{extension}",
        std::process::id()
    ))
}

fn cleanup(path: std::path::PathBuf) {
    let _ = fs::remove_file(path);
}

fn feedback_base_metrics() -> GenerationMetrics {
    GenerationMetrics {
        perplexity: 14.0,
        semantic_consistency: 0.74,
        contradiction_count: 0,
        token_count: 96,
    }
}

fn clean_feedback_reflection(quality: f32) -> ReflectionReport {
    ReflectionReport {
        quality,
        contradictions: Vec::new(),
        issues: Vec::new(),
        revision_actions: Vec::new(),
        revision_passes: 0,
        revised_answer: "stable rust noiron feedback path".to_owned(),
        store_as_memory: true,
        lesson: "online reward feedback should scale with process reward strength".to_owned(),
    }
}

fn stable_feedback_drift_report() -> DriftReport {
    DriftReport {
        severity: crate::drift::DriftSeverity::Stable,
        allow_memory_write: true,
        allow_runtime_kv_write: true,
        penalize_used_memory: false,
        rollback_adaptive: false,
        notes: Vec::new(),
    }
}

fn feedback_reward_report(total: f32, action: RewardAction) -> ProcessRewardReport {
    ProcessRewardReport {
        total,
        action,
        components: ProcessRewardComponents::default(),
        notes: Vec::new(),
    }
}

fn memory_strength(engine: &NoironEngine, memory_id: u64) -> f32 {
    engine
        .cache
        .entries()
        .iter()
        .find(|entry| entry.id == memory_id)
        .map(|entry| entry.strength)
        .unwrap()
}

fn perturbed_vector(vector: &[f32], salt: usize) -> Vec<f32> {
    let mut out = vector.to_vec();
    let len = out.len().max(1);
    out[(salt * 13 + 7) % len] += 1.0;
    out[(salt * 29 + 11) % len] += 1.0;
    let norm = out.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut out {
            *value /= norm;
        }
    }
    out
}

fn replay_runtime_diagnostics(kv_influence: f32) -> RuntimeDiagnostics {
    RuntimeDiagnostics {
        model_id: Some("self-transformer-replay-test".to_owned()),
        selected_adapter: Some(RuntimeAdapterHint::CpuSimd.as_str().to_owned()),
        layer_count: 6,
        hidden_size: 128,
        local_window_tokens: 4096,
        forward_energy: Some(0.22),
        kv_influence: Some(kv_influence),
        imported_kv_blocks: 1,
        exported_kv_blocks: 1,
        ..RuntimeDiagnostics::default()
    }
}

fn replay_runtime_segment_diagnostics(
    kv_influence: f32,
    included: usize,
    skipped: usize,
    rejected: usize,
) -> RuntimeDiagnostics {
    RuntimeDiagnostics {
        runtime_kv_segments_included: included,
        runtime_kv_segments_skipped: skipped,
        runtime_kv_segments_rejected: rejected,
        ..replay_runtime_diagnostics(kv_influence)
    }
}

fn replay_memory_input(
    prompt: &str,
    lesson: &str,
    reward: f32,
    memory_id: u64,
    reflection_issues: Vec<ReflectionIssue>,
    revision_actions: Vec<String>,
    runtime_diagnostics: RuntimeDiagnostics,
    reward_notes: Vec<String>,
) -> ExperienceInput {
    ExperienceInput {
        prompt: prompt.to_owned(),
        profile: TaskProfile::Coding,
        lesson: lesson.to_owned(),
        quality: reward,
        contradictions: Vec::new(),
        reflection_issues,
        revision_actions,
        stored_memory_id: None,
        router_threshold_after: 0.55,
        stream_windows: 2,
        route_budget: RouteBudget {
            threshold: 0.55,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: vec![memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics,
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: reward,
            action: if reward >= 0.72 {
                RewardAction::Reinforce
            } else if reward <= 0.42 {
                RewardAction::Penalize
            } else {
                RewardAction::Hold
            },
            components: ProcessRewardComponents::default(),
            notes: reward_notes,
        },
        live_evolution: Default::default(),
    }
}

fn replay_memory_input_with_live_evolution(
    prompt: &str,
    lesson: &str,
    reward: f32,
    memory_id: u64,
    live_evolution: LiveInferenceEvolution,
) -> ExperienceInput {
    ExperienceInput {
        live_evolution,
        ..replay_memory_input(
            prompt,
            lesson,
            reward,
            memory_id,
            Vec::new(),
            Vec::new(),
            RuntimeDiagnostics::default(),
            Vec::new(),
        )
    }
}

fn create_runtime_assets(label: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = temp_asset_dir(label);
    fs::create_dir_all(&dir).unwrap();
    let weights = dir.join("weights.noiron");
    let tokenizer = dir.join("tokenizer.noiron");
    write_asset(&weights, b"weights");
    write_asset(&tokenizer, b"tokenizer");
    (dir, weights, tokenizer)
}

fn write_asset(path: &Path, bytes: &[u8]) {
    let mut file = File::create(path).unwrap();
    file.write_all(bytes).unwrap();
}

fn temp_asset_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rust-norion-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn replay_item_with_recursive_calls(
    recursive_runtime_calls: Option<usize>,
) -> ExperienceReplayItem {
    ExperienceReplayItem {
        experience_id: 42,
        profile: TaskProfile::LongDocument,
        action: RewardAction::Reinforce,
        reward: 0.86,
        quality: 0.88,
        contradiction_count: 0,
        reflection_issue_count: 0,
        critical_reflection_issue_count: 0,
        revision_action_count: 0,
        stream_windows: 2,
        route_budget: RouteBudget {
            threshold: 0.54,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.5,
        },
        memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics::default(),
        live_evolution: Default::default(),
        recursive_runtime_calls,
        recursive_stats: recursive_runtime_calls.map(|runtime_calls| RecursiveReplayStats {
            chunks: Some(4),
            merge_rounds: Some(2),
            waves: Some(2),
            parallel: Some(2),
            runtime_calls: Some(runtime_calls),
        }),
        live_memory_feedback: None,
        rust_check_stats: None,
        rust_check_live_memory_feedback: None,
        business_contract_stats: None,
        pool_dispatch_stats: None,
        priority: 0.86,
        lesson: "long-context recursive replay path".to_owned(),
    }
}

#[derive(Debug, Clone)]
struct EngineForwardKernel;

impl ProductionForwardKernel for EngineForwardKernel {
    fn generate(
        &self,
        context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(ProductionKernelOutput::new(
                "Rust production kernel answer keeps Noiron routing, reflection, diagnostics, and reusable runtime KV memory aligned for future local inference.",
            )
            .with_tokens(vec![
                RuntimeToken {
                    text: "production".to_owned(),
                    logprob: Some(-0.20),
                    entropy: Some(0.18),
                },
                RuntimeToken {
                    text: "kernel".to_owned(),
                    logprob: Some(-0.25),
                    entropy: Some(0.22),
                },
                RuntimeToken {
                    text: "memory".to_owned(),
                    logprob: Some(-0.18),
                    entropy: Some(0.20),
                },
            ])
            .with_trace(vec![ReasoningStep::new(
                "production_kernel",
                format!(
                    "adapter={} assets={} imported_kv={}",
                    context.device_gate.runtime_adapter_name(),
                    context.assets.summary_line(),
                    context.imported_kv_blocks.len()
                ),
                0.92,
            )])
            .with_diagnostics(RuntimeDiagnostics {
                model_id: Some(context.manifest.metadata.model_id.clone()),
                selected_adapter: context
                    .device_gate
                    .runtime_adapter
                    .map(|adapter| adapter.as_str().to_owned()),
                layer_count: context.manifest.architecture.layer_count,
                hidden_size: context.manifest.architecture.hidden_size,
                local_window_tokens: context.manifest.architecture.local_window_tokens,
                forward_energy: Some(0.31),
                kv_influence: Some(0.22),
                imported_kv_blocks: context.imported_kv_blocks.len(),
                exported_kv_blocks: 1,
                ..RuntimeDiagnostics::default()
            }
            .with_layer_modes(2, 3, 1))
            .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                3,
                1,
                0,
                3,
                vec![0.11, 0.22, 0.33],
                vec![0.44, 0.55, 0.66],
            )]))
    }
}
