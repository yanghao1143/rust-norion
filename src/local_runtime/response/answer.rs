use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime::{RuntimeRequest, RuntimeTokenId};
use crate::runtime_manifest::RuntimeManifest;

use super::super::forward::LocalForwardState;
use super::super::tokenizer::compact;
use super::ResponseEvidence;

pub(super) fn build_answer(
    request: &RuntimeRequest,
    manifest: &RuntimeManifest,
    tokens: &[RuntimeTokenId],
    imported_kv_blocks: &[RuntimeKvBlock],
    forward: &LocalForwardState,
    evidence: &ResponseEvidence,
) -> String {
    let transformer_counts = &evidence.transformer_counts;
    let profile_hint = evidence.profile_hint;

    format!(
        "Local Transformer runtime result for '{}'. The self-developed runtime used manifest {}, {} prompt tokens, {} imported KV blocks, {} memory hints, and {} experience hints. It executed {} deterministic Transformer layers with state energy {:.3} and KV influence {:.3}: {} global, {} local-window, and {} convolutional-fusion layers. Hardware execution targeted {} with {} memory and {} fallback. Profile policy: {profile_hint}. Noiron keeps model weights fixed here while adapting routing thresholds, reinforced KV memory, hierarchy weights, reflection rewards, and reusable experience around the runtime.",
        compact(&request.prompt, 96),
        manifest.metadata.model_id,
        tokens.len(),
        imported_kv_blocks.len(),
        request.memory_hints.len(),
        request.experience_hints.len(),
        forward.layer_summaries.len(),
        forward.energy,
        forward.kv_influence,
        transformer_counts.global,
        transformer_counts.local,
        transformer_counts.convolution,
        request.hardware_plan.execution.primary_lane.as_str(),
        request.hardware_plan.execution.memory_mode.as_str(),
        request.hardware_plan.execution.fallback_lane.as_str(),
    )
}
