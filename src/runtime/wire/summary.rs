use crate::kv_exchange::RuntimeKvBlock;
use crate::toolsmith::ToolBlueprint;

use super::super::RuntimeAdapterObservation;

pub(in crate::runtime) fn option_f32_display(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "none".to_owned())
}

pub(in crate::runtime) fn option_usize_display(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

pub(super) fn bullet_list(items: &[String]) -> String {
    if items.is_empty() {
        return "- none".to_owned();
    }

    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn bullet_runtime_adapter_observations(items: &[RuntimeAdapterObservation]) -> String {
    if items.is_empty() {
        return "- none".to_owned();
    }

    items
        .iter()
        .map(|item| format!("- {}", item.summary()))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn bullet_runtime_kv_blocks(items: &[RuntimeKvBlock]) -> String {
    if items.is_empty() {
        return "- none".to_owned();
    }

    items
        .iter()
        .map(runtime_kv_block_summary)
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(in crate::runtime) fn runtime_kv_blocks_summary(items: &[RuntimeKvBlock]) -> String {
    if items.is_empty() {
        return "none".to_owned();
    }

    items
        .iter()
        .map(runtime_kv_block_summary)
        .collect::<Vec<_>>()
        .join("\n")
}

fn runtime_kv_block_summary(block: &RuntimeKvBlock) -> String {
    format!(
        "layer={} head={} tokens={}..{} key_dims={} value_dims={}",
        block.layer,
        block.head,
        block.token_start,
        block.token_end,
        block.key.len(),
        block.value.len()
    )
}

pub(super) fn bullet_tool_blueprints(items: &[ToolBlueprint]) -> String {
    if items.is_empty() {
        return "- none".to_owned();
    }

    items
        .iter()
        .map(|item| format!("- {}", item.summary()))
        .collect::<Vec<_>>()
        .join("\n")
}
