mod backend;
mod capability;
mod command;
mod contract;
mod device;
mod http;
mod internal_transport;
mod kv_import;
mod kv_safety;
mod native_adapter;
#[cfg(feature = "runtime-tonic")]
mod tonic_transport;
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
pub use capability::{
    RuntimeAdapterCapability, RuntimeAdapterFallback, RuntimeAdapterFallbackReason,
    RuntimeAdapterLanguage, RuntimeAdapterRegistry, RuntimeAdapterRequirement,
    RuntimeAdapterSelection,
};
pub use command::{CommandPromptMode, CommandRuntime, CommandTextOutputFilter, CommandWireFormat};
#[cfg(test)]
use command::{filter_command_text_output, parse_mistralrs_cli_stats};
pub use http::MistralRsHttpRuntime;
#[cfg(feature = "runtime-tonic")]
pub(crate) use http::benchmark_chat_completion_request_bytes;
pub use internal_transport::{
    INTERNAL_RUNTIME_PROTO_PACKAGE, INTERNAL_RUNTIME_PROTO_SCHEMA, InternalRuntimeEnvelope,
    InternalRuntimeLoopback, InternalRuntimeMethod, runtime_transport_manifest_digest,
    runtime_transport_proto_digest,
};
pub use native_adapter::{
    ChunkedKvCacheMode, ChunkedKvHookDecision, ChunkedKvHookRecord, ChunkedKvSegment,
    MockRustNativeAdapter, RustNativeAdapterComparisonReport, RustNativeAdapterDeviceExecution,
    RustNativeAdapterModeComparison, RustNativeAdapterReport, RustNativeAdapterRequest,
    RustNativeAdapterStreamEvent, RustNativeInferenceAdapter, RustNativeModelRuntime,
};
#[cfg(feature = "runtime-tonic")]
pub use tonic_transport::{
    TonicRuntimeClient, TonicRuntimeModelClient, TonicRuntimeServer, TonicRuntimeService,
    proto as tonic_runtime_proto, tonic_status_to_runtime_error,
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
