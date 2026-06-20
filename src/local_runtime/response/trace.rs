use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::ReasoningStep;
use crate::runtime::{RuntimeRequest, RuntimeTokenId};
use crate::runtime_manifest::RuntimeManifest;

use super::super::forward::LocalForwardState;
use super::ResponseEvidence;

pub(super) fn build_trace(
    request: &RuntimeRequest,
    manifest: &RuntimeManifest,
    tokens: &[RuntimeTokenId],
    imported_kv_blocks: &[RuntimeKvBlock],
    exported_kv_blocks: &[RuntimeKvBlock],
    forward: &LocalForwardState,
    evidence: &ResponseEvidence,
) -> Vec<ReasoningStep> {
    vec![
        ReasoningStep::new(
            "local_tokenizer",
            format!("tokenized {} prompt tokens", tokens.len()),
            0.84,
        ),
        ReasoningStep::new(
            "local_transformer_forward",
            format!(
                "executed {} layers with hidden size {} and energy {:.3} and KV influence {:.3}",
                forward.layer_summaries.len(),
                manifest.architecture.hidden_size,
                forward.energy,
                forward.kv_influence
            ),
            0.83,
        ),
        ReasoningStep::new(
            "local_transformer_plan",
            format!(
                "planned {} global, {} local, {} convolution layers",
                evidence.transformer_counts.global,
                evidence.transformer_counts.local,
                evidence.transformer_counts.convolution
            ),
            0.82,
        ),
        ReasoningStep::new(
            "local_device_execution",
            format!(
                "primary {} fallback {} memory {} adapter {:?}",
                request.hardware_plan.execution.primary_lane.as_str(),
                request.hardware_plan.execution.fallback_lane.as_str(),
                request.hardware_plan.execution.memory_mode.as_str(),
                evidence.selected_adapter
            ),
            0.80,
        ),
        ReasoningStep::new(
            "local_kv_exchange",
            format!(
                "imported {} blocks and prepared {} export blocks",
                imported_kv_blocks.len(),
                exported_kv_blocks.len()
            ),
            0.80,
        ),
    ]
}
