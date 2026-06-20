use crate::engine::GenerationContext;
use crate::infini_memory::InfiniMemoryItem;
use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime_manifest::TransformerRuntimeArchitecture;
use crate::tiered_cache::MemoryTier;

use super::types::RuntimeMetadata;

pub(super) fn runtime_kv_blocks_from_context(
    context: &GenerationContext<'_>,
    metadata: &RuntimeMetadata,
    architecture: TransformerRuntimeArchitecture,
) -> Vec<RuntimeKvBlock> {
    if !metadata.supports_kv_import {
        return Vec::new();
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
    let prefetch_limit = context
        .hardware_plan
        .execution
        .kv_prefetch_blocks
        .min(manifest_limit)
        .max(1);

    runtime_kv_import_candidates(context)
        .into_iter()
        .filter(|candidate| !candidate.vector.is_empty())
        .filter(|memory| {
            context
                .tier_plan
                .placement_for(memory.id)
                .map(|placement| placement.tier != MemoryTier::ColdDisk)
                .unwrap_or(true)
        })
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
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct RuntimeKvImportCandidate<'a> {
    id: u64,
    vector: &'a [f32],
    weight: f32,
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
            .map(candidate_from_infini_item)
            .collect();
    }

    context
        .memories
        .iter()
        .map(|memory| RuntimeKvImportCandidate {
            id: memory.id,
            vector: &memory.vector,
            weight: memory.strength,
        })
        .collect()
}

fn candidate_from_infini_item(item: &InfiniMemoryItem) -> RuntimeKvImportCandidate<'_> {
    RuntimeKvImportCandidate {
        id: item.id,
        vector: &item.vector,
        weight: item.score.max(0.05),
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
