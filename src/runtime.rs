mod backend;
mod command;
mod contract;
mod device;
mod http;
mod kv_import;
mod kv_safety;
mod native_adapter;
mod types;
mod wire;

#[cfg(test)]
use crate::engine::{GenerationContext, InferenceBackend};
#[cfg(test)]
use crate::hardware::{
    ComputeLane, DeviceClass, DeviceMemoryMode, HardwarePlan, RuntimeAdapterHint,
};
#[cfg(test)]
use crate::kv_exchange::RuntimeKvBlock;
#[cfg(test)]
use crate::reflection::RuntimeDiagnostics;

pub use backend::RuntimeBackend;
pub use command::{CommandPromptMode, CommandRuntime, CommandTextOutputFilter, CommandWireFormat};
#[cfg(test)]
use command::{filter_command_text_output, parse_mistralrs_cli_stats};
pub use http::MistralRsHttpRuntime;
pub use native_adapter::{
    ChunkedKvCacheMode, ChunkedKvHookDecision, ChunkedKvHookRecord, ChunkedKvSegment,
    MockRustNativeAdapter, RustNativeAdapterReport, RustNativeAdapterRequest,
    RustNativeInferenceAdapter,
};
pub use types::{
    ModelRuntime, RuntimeAdapterObservation, RuntimeEmbedding, RuntimeError, RuntimeMetadata,
    RuntimeRequest, RuntimeResponse, RuntimeToken, RuntimeTokenId,
};
#[cfg(test)]
use wire::{
    extract_json_array_field, extract_json_number_field, extract_json_string_field,
    format_runtime_prompt,
};
use wire::{
    format_runtime_payload, option_f32_display, option_usize_display, runtime_kv_blocks_summary,
};
pub use wire::{parse_runtime_response_json, runtime_request_json};

#[cfg(test)]
mod tests;
