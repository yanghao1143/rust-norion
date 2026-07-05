use crate::engine::GenerationContext;
use crate::infini_memory::InfiniMemoryItem;
use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime_manifest::TransformerRuntimeArchitecture;
use crate::tenant_scope::{TenantResourceLane, TenantScopedKey};
use crate::tiered_cache::MemoryTier;
use std::cmp::Ordering;

use super::types::RuntimeMetadata;

const MIN_RUNTIME_KV_IMPORT_STRENGTH: f32 = 0.45;

#[derive(Debug, Default)]
pub(super) struct RuntimeKvImportSelection {
    pub(super) blocks: Vec<RuntimeKvBlock>,
    pub(super) weak_runtime_kv_skipped: usize,
    pub(super) budget_limited_candidates_skipped: usize,
}

pub(super) fn runtime_kv_import_selection_from_context(
    context: &GenerationContext<'_>,
    metadata: &RuntimeMetadata,
    architecture: TransformerRuntimeArchitecture,
) -> RuntimeKvImportSelection {
    if !metadata.supports_kv_import {
        return RuntimeKvImportSelection::default();
    }

    let dimensions = if metadata.embedding_dimensions > 0 {
        Some(metadata.embedding_dimensions)
    } else {
        None
    };
    let manifest_limit = if metadata.max_kv_import_blocks > 0 {
        metadata.max_kv_import_blocks
    } else {
        context.hardware_plan.execution.kv_prefetch_blocks
    };
    let prefetch_limit = runtime_kv_import_prefetch_limit(context, manifest_limit);

    let mut weak_runtime_kv_skipped = 0;
    let mut candidates = runtime_kv_import_candidates(context)
        .into_iter()
        .filter(|candidate| !candidate.vector.is_empty())
        .filter(|candidate| {
            let has_signal = runtime_kv_candidate_has_import_signal(candidate);
            if !has_signal && is_runtime_kv_candidate_key(candidate.key) {
                weak_runtime_kv_skipped += 1;
            }
            has_signal
        })
        .filter(|memory| {
            context
                .tier_plan
                .placement_for(memory.id)
                .map(|placement| placement.tier != MemoryTier::ColdDisk)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    let budget_limited_candidates_skipped = candidates.len().saturating_sub(prefetch_limit);
    if budget_limited_candidates_skipped > 0 {
        candidates.sort_by(compare_runtime_kv_import_candidates);
    }
    let blocks = candidates
        .into_iter()
        .take(prefetch_limit)
        .enumerate()
        .map(|(index, candidate)| {
            let key = fit_runtime_vector(candidate.vector, dimensions);
            let weighted = candidate
                .vector
                .iter()
                .map(|value| value * candidate.weight)
                .collect::<Vec<_>>();
            let value = fit_runtime_vector(&weighted, dimensions);

            let kv_heads = architecture.kv_heads.max(1);
            let layer_count = architecture.layer_count.max(1);
            RuntimeKvBlock::new(
                (index / kv_heads) % layer_count,
                index % kv_heads,
                index,
                index + 1,
                key,
                value,
            )
        })
        .collect();

    RuntimeKvImportSelection {
        blocks,
        weak_runtime_kv_skipped,
        budget_limited_candidates_skipped,
    }
}

#[derive(Debug, Clone, Copy)]
struct RuntimeKvImportCandidate<'a> {
    id: u64,
    key: &'a str,
    vector: &'a [f32],
    weight: f32,
    source_strength: Option<f32>,
}

fn runtime_kv_import_candidates<'a>(
    context: &'a GenerationContext<'_>,
) -> Vec<RuntimeKvImportCandidate<'a>> {
    let has_infini_decisions = !context.infini_memory_plan.local_window().is_empty()
        || !context.infini_memory_plan.global_memory().is_empty()
        || !context.infini_memory_plan.skipped().is_empty();

    if has_infini_decisions {
        return context
            .infini_memory_plan
            .local_window()
            .iter()
            .chain(context.infini_memory_plan.global_memory())
            .map(|item| candidate_from_infini_item(item, active_memory_strength(context, item.id)))
            .collect();
    }

    context
        .memories
        .iter()
        .map(|memory| RuntimeKvImportCandidate {
            id: memory.id,
            key: &memory.key,
            vector: &memory.vector,
            weight: memory.strength,
            source_strength: Some(memory.strength),
        })
        .collect()
}

fn candidate_from_infini_item(
    item: &InfiniMemoryItem,
    source_strength: Option<f32>,
) -> RuntimeKvImportCandidate<'_> {
    RuntimeKvImportCandidate {
        id: item.id,
        key: &item.key,
        vector: &item.vector,
        weight: item.score.max(0.05),
        source_strength,
    }
}

fn active_memory_strength(context: &GenerationContext<'_>, id: u64) -> Option<f32> {
    context
        .memories
        .iter()
        .find(|memory| memory.id == id)
        .map(|memory| memory.strength)
}

fn runtime_kv_candidate_has_import_signal(candidate: &RuntimeKvImportCandidate<'_>) -> bool {
    if !is_runtime_kv_candidate_key(candidate.key) {
        return true;
    }

    let strength = candidate.source_strength.unwrap_or(candidate.weight);
    strength.is_finite() && strength >= MIN_RUNTIME_KV_IMPORT_STRENGTH
}

fn is_runtime_kv_candidate_key(key: &str) -> bool {
    key.starts_with("runtime_kv:")
        || TenantScopedKey::parse(key)
            .is_some_and(|scoped| scoped.lane == TenantResourceLane::RuntimeKv)
}

fn compare_runtime_kv_import_candidates(
    left: &RuntimeKvImportCandidate<'_>,
    right: &RuntimeKvImportCandidate<'_>,
) -> Ordering {
    is_runtime_kv_candidate_key(right.key)
        .cmp(&is_runtime_kv_candidate_key(left.key))
        .then_with(|| {
            runtime_kv_candidate_signal(right)
                .partial_cmp(&runtime_kv_candidate_signal(left))
                .unwrap_or(Ordering::Equal)
        })
}

fn runtime_kv_candidate_signal(candidate: &RuntimeKvImportCandidate<'_>) -> f32 {
    let signal = candidate.source_strength.unwrap_or(candidate.weight);
    if signal.is_finite() {
        signal.max(candidate.weight).max(0.0)
    } else if candidate.weight.is_finite() {
        candidate.weight.max(0.0)
    } else {
        0.0
    }
}

fn runtime_kv_import_prefetch_limit(
    context: &GenerationContext<'_>,
    manifest_limit: usize,
) -> usize {
    let device_limit = context
        .hardware_plan
        .execution
        .kv_prefetch_blocks
        .min(manifest_limit)
        .max(1);
    let attention_fraction = finite_unit(context.route_budget.attention_fraction);
    if context.route_budget.attention_tokens == 0 || attention_fraction <= 0.05 {
        return device_limit.min(1);
    }

    let pressure = finite_unit(context.hardware_plan.pressure);
    if pressure >= 0.70 || attention_fraction < 0.25 {
        return device_limit.min(1);
    }
    if pressure >= 0.45 || attention_fraction < 0.50 {
        return device_limit.min(((device_limit + 1) / 2).max(1));
    }
    device_limit
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn fit_runtime_vector(vector: &[f32], dimensions: Option<usize>) -> Vec<f32> {
    let Some(dimensions) = dimensions else {
        return vector.to_vec();
    };
    let mut out = vector.iter().copied().take(dimensions).collect::<Vec<_>>();
    out.resize(dimensions, 0.0);
    out
}
