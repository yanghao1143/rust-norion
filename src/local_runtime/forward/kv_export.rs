use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime::RuntimeRequest;
use crate::transformer::AttentionKind;

use super::math::scaled;
use super::model::LocalForwardState;

pub(in crate::local_runtime) fn export_forward_kv(
    forward: &LocalForwardState,
    request: &RuntimeRequest,
) -> Vec<RuntimeKvBlock> {
    if forward.vector.is_empty() {
        return Vec::new();
    }

    let layer_count = request.runtime_architecture.layer_count.max(1);
    let kv_heads = request.runtime_architecture.kv_heads.max(1);
    let block_count = forward
        .layer_summaries
        .len()
        .clamp(1, 4)
        .min(request.recursive_schedule.chunk_count().max(1) + 2);
    let midpoint = (forward.vector.len() / 2).max(1);
    let key = forward
        .vector
        .iter()
        .copied()
        .take(midpoint)
        .collect::<Vec<_>>();
    let value = forward
        .vector
        .iter()
        .copied()
        .skip(midpoint)
        .collect::<Vec<_>>();
    let value = if value.is_empty() { key.clone() } else { value };
    let paired_len = key.len().min(value.len()).max(1);
    let key = key.into_iter().take(paired_len).collect::<Vec<_>>();
    let value = value.into_iter().take(paired_len).collect::<Vec<_>>();

    (0..block_count)
        .map(|index| {
            let summary = forward.layer_summaries.get(index);
            let layer = summary.map(|summary| summary.layer_index).unwrap_or(index) % layer_count;
            let head = summary
                .map(|summary| {
                    let attention_offset = match summary.attention {
                        AttentionKind::Global => 0,
                        AttentionKind::LocalWindow => 2,
                        AttentionKind::ConvolutionalFusion => 4,
                    };
                    (attention_offset + summary.window_size + index) % kv_heads
                })
                .unwrap_or(index % kv_heads);
            let compute_scale = summary
                .map(|summary| summary.compute_fraction + summary.activation)
                .unwrap_or(1.0)
                .clamp(0.25, 1.50);
            RuntimeKvBlock::new(
                layer,
                head,
                index,
                index + 1,
                scaled(&key, compute_scale + index as f32 * 0.02),
                scaled(&value, compute_scale - index as f32 * 0.015),
            )
        })
        .collect()
}
