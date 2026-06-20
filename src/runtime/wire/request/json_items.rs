use crate::hierarchy::TaskProfile;
use crate::kv_exchange::RuntimeKvBlock;
use crate::toolsmith::ToolBlueprint;
use crate::transformer::AttentionKind;

use super::super::super::{RuntimeAdapterObservation, RuntimeRequest};
use super::super::json::{json_f32_array, json_str_array, json_string, option_f32_json};

pub(super) fn transformer_layers_json(request: &RuntimeRequest) -> String {
    request
        .transformer_plan
        .layers
        .iter()
        .map(|layer| {
            format!(
                "{{\"layer_index\":{},\"attention\":{},\"compute_fraction\":{:.6},\"window_size\":{}}}",
                layer.layer_index,
                json_string(attention_kind_str(layer.attention)),
                layer.compute_fraction,
                layer.window_size
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn recursive_chunks_json(request: &RuntimeRequest) -> String {
    request
        .recursive_schedule
        .chunks
        .iter()
        .map(|chunk| {
            format!(
                "{{\"index\":{},\"start_token\":{},\"end_token\":{},\"estimated_tokens\":{},\"overlap_left\":{},\"overlap_right\":{}}}",
                chunk.index,
                chunk.start_token,
                chunk.end_token,
                chunk.estimated_tokens,
                chunk.overlap_left,
                chunk.overlap_right
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn recursive_merge_rounds_json(request: &RuntimeRequest) -> String {
    request
        .recursive_schedule
        .merge_rounds
        .iter()
        .map(|round| {
            format!(
                "{{\"round\":{},\"input_units\":{},\"output_units\":{}}}",
                round.round, round.input_units, round.output_units
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn recursive_execution_waves_json(request: &RuntimeRequest) -> String {
    request
        .recursive_schedule
        .execution_waves
        .iter()
        .map(|wave| {
            format!(
                "{{\"wave\":{},\"start_chunk\":{},\"end_chunk\":{},\"chunk_count\":{}}}",
                wave.wave, wave.start_chunk, wave.end_chunk, wave.chunk_count
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn runtime_adapter_observations_json(items: &[RuntimeAdapterObservation]) -> String {
    items
        .iter()
        .map(runtime_adapter_observation_json)
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn imported_kv_blocks_json(blocks: &[RuntimeKvBlock]) -> String {
    blocks
        .iter()
        .map(runtime_kv_block_json)
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn tool_blueprints_json(blueprints: &[ToolBlueprint]) -> String {
    blueprints
        .iter()
        .map(tool_blueprint_json)
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn task_profile_str(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn attention_kind_str(attention: AttentionKind) -> &'static str {
    match attention {
        AttentionKind::Global => "global",
        AttentionKind::LocalWindow => "local_window",
        AttentionKind::ConvolutionalFusion => "convolutional_fusion",
    }
}

fn runtime_adapter_observation_json(observation: &RuntimeAdapterObservation) -> String {
    format!(
        "{{\"adapter\":{},\"score\":{:.6},\"reward\":{:.6},\"quality\":{:.6},\"forward_energy\":{},\"kv_influence\":{},\"experience_id\":{}}}",
        json_string(observation.adapter.as_str()),
        observation.score,
        observation.reward,
        observation.quality,
        option_f32_json(observation.forward_energy),
        option_f32_json(observation.kv_influence),
        observation.experience_id
    )
}

fn runtime_kv_block_json(block: &RuntimeKvBlock) -> String {
    format!(
        "{{\"layer\":{},\"head\":{},\"token_start\":{},\"token_end\":{},\"key\":{},\"value\":{}}}",
        block.layer,
        block.head,
        block.token_start,
        block.token_end,
        json_f32_array(&block.key),
        json_f32_array(&block.value)
    )
}

fn tool_blueprint_json(blueprint: &ToolBlueprint) -> String {
    format!(
        "{{\"id\":{},\"name\":{},\"intent\":{},\"trigger\":{},\"rust_crate\":{},\"entrypoint\":{},\"status\":{},\"allowed_io\":{},\"denied_capabilities\":{},\"build_steps\":{},\"validation_steps\":{},\"source_outline\":{},\"gate_notes\":{}}}",
        json_string(&blueprint.id),
        json_string(&blueprint.name),
        json_string(blueprint.intent.as_str()),
        json_string(&blueprint.trigger),
        json_string(&blueprint.rust_crate),
        json_string(&blueprint.entrypoint),
        json_string(blueprint.status.as_str()),
        json_str_array(blueprint.allowed_io.iter().map(String::as_str)),
        json_str_array(blueprint.denied_capabilities.iter().map(String::as_str)),
        json_str_array(blueprint.build_steps.iter().map(String::as_str)),
        json_str_array(blueprint.validation_steps.iter().map(String::as_str)),
        json_str_array(blueprint.source_outline.iter().map(String::as_str)),
        json_str_array(blueprint.gate_notes.iter().map(String::as_str))
    )
}
