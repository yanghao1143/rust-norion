use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime_manifest::TransformerRuntimeArchitecture;

use super::RuntimeMetadata;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RuntimeKvSafetyReport {
    pub accepted: Vec<RuntimeKvBlock>,
    pub rejected: Vec<String>,
    pub truncated: usize,
}

impl RuntimeKvSafetyReport {
    pub fn rejected_count(&self) -> usize {
        self.rejected.len().saturating_add(self.truncated)
    }
}

pub(super) fn sanitize_runtime_kv_blocks(
    blocks: Vec<RuntimeKvBlock>,
    metadata: &RuntimeMetadata,
    architecture: TransformerRuntimeArchitecture,
    require_dimensions: bool,
    field: &str,
) -> RuntimeKvSafetyReport {
    let max_blocks = match field {
        "imported_kv_blocks" => metadata.max_kv_import_blocks,
        "exported_kv_blocks" => metadata.max_kv_export_blocks,
        _ => 0,
    };
    let dimensions = require_dimensions
        .then_some(metadata.embedding_dimensions)
        .filter(|value| *value > 0);
    let max_layers = architecture.layer_count.max(1);
    let max_heads = architecture.kv_heads.max(1);
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut truncated = 0;
    let limit = if max_blocks == 0 {
        usize::MAX
    } else {
        max_blocks
    };

    for (index, block) in blocks.into_iter().enumerate() {
        if accepted.len() >= limit {
            truncated += 1;
            continue;
        }
        match block.validate_shape(max_layers, max_heads, dimensions) {
            Ok(()) => accepted.push(block),
            Err(error) => rejected.push(format!("{field}[{index}]: {error}")),
        }
    }

    RuntimeKvSafetyReport {
        accepted,
        rejected,
        truncated,
    }
}
