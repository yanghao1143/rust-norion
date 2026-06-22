use std::env;

#[cfg(test)]
use std::io::{Read, Write};
#[cfg(test)]
use std::net::{TcpListener, TcpStream};
#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
use rust_norion::{
    CommandWireFormat, DeviceClass, GemmaRuntimeQuantizationMode, InferenceBackend,
    InferenceRequest, RewardAction, StateInspectionGateReport, StateInspectionReport, TaskProfile,
    TraceSchemaGateReport, default_benchmark_cases,
};
#[cfg(test)]
use rust_norion::{
    DevicePlanGateReport, HeuristicBackend, ModelRuntime, NoironEngine,
    ProductionKernelConformanceGate, RuntimeBackend, RuntimeManifestDeviceGateReport,
    evaluate_trace_schema_jsonl,
};

mod cli;
#[path = "main/dispatch.rs"]
mod dispatch;
mod engine_config;
mod gemma_business;
#[path = "main/inference_output.rs"]
mod inference_output;
mod inference_runner;
mod model_service;
mod path_utils;

pub(crate) use cli::args::Args;
#[cfg(test)]
use cli::benchmark::{
    benchmark_self_evolution_admission_report, run_benchmark, run_benchmark_for_args,
    run_production_benchmark_all_devices, run_production_kernel_conformance_all_devices,
};
pub(crate) use cli::display::{option_bool_display, option_path_display, option_u64_display};
#[cfg(test)]
use cli::roundtrip::{run_persistent_roundtrip, run_persistent_roundtrip_all_devices};
#[cfg(test)]
use cli::state::device_scoped_path;
#[cfg(test)]
use cli::state::{run_state_inspection, run_state_inspection_all_devices};
#[cfg(test)]
use engine_config::configure_engine;
#[cfg(test)]
use gemma_business::GEMMA_BUSINESS_CYCLE_SMOKE_REPORT_FILE;
#[cfg(test)]
use gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
#[cfg(test)]
use gemma_business::audit::{
    GemmaModelServiceAnswerAudit, gemma_business_smoke_runtime_failure_parts,
};
#[cfg(test)]
use gemma_business::audit::{
    GemmaModelServiceBusinessNormalizationKind, business_answer_contains_signal,
    gemma_business_smoke_answer_failure, gemma_business_smoke_runtime_failure_text,
    gemma_model_service_answer_failure, gemma_model_service_business_normalization,
    normalize_gemma_model_service_business_answer,
};
#[cfg(test)]
use gemma_business::contract::record_gemma_business_smoke_contract;
#[cfg(test)]
use gemma_business::gemma_business_smoke_case;
#[cfg(test)]
use gemma_business::paths::{gemma_smoke_base_dir, prune_gemma_smoke_run_dirs};
#[cfg(test)]
use gemma_business::preflight::gemma_business_smoke_preflight_failures;
#[cfg(test)]
use gemma_business::regression::evaluate_gemma_business_cycle_smoke_report_gate_body;
#[cfg(test)]
use gemma_business::regression::{
    evaluate_gemma_business_cycle_smoke_report_gate, gemma_business_regression_report_path,
};
#[cfg(test)]
use gemma_business::smoke_gate::run_gemma_business_smoke_replay;
#[cfg(test)]
use gemma_business::smoke_report::GemmaModelServiceRuntimeAudit;
#[cfg(test)]
use gemma_business::smoke_report::gemma_business_cycle_smoke_report_json;
#[cfg(test)]
use gemma_business::smoke_report::{
    GemmaBusinessCycleCaseResult, gemma_business_cycle_smoke_aggregate_response_json,
    gemma_business_cycle_smoke_matrix_report_json,
};
#[cfg(test)]
use gemma_business::state_gate::gemma_business_smoke_state_gate;
#[cfg(test)]
use gemma_business::state_gate::{
    gemma_business_cycle_state_gate, gemma_model_service_smoke_state_gate,
};
#[cfg(test)]
use gemma_business::{
    GEMMA_BUSINESS_CYCLE_SMOKE_DIR, GEMMA_BUSINESS_SMOKE_DIR, GEMMA_BUSINESS_SMOKE_PROMPT,
    GEMMA_MODEL_SERVICE_SMOKE_DIR, GEMMA_SMOKE_DEFAULT_KEEP_RUNS,
    GEMMA4_12B_SMOKE_COLD_START_TIMEOUT_MS,
};
#[cfg(test)]
use inference_runner::run_timed_inference;
#[cfg(test)]
use model_service::http::split_http_head_body;
#[cfg(test)]
use model_service::json::json_bool_field;
#[cfg(test)]
use model_service::json::json_f32_field;
#[cfg(test)]
use model_service::json::service_u64_array;
#[cfg(test)]
use model_service::json::{
    json_string_field, json_u64_array_field, json_u64_field, service_json_string,
    service_json_string_array,
};
#[cfg(test)]
use model_service::request::{
    ModelServiceBusinessCycleRequest, ModelServiceChatMessage, ModelServiceChatRequest,
    ModelServiceFeedbackRequest, ModelServiceHttpRequest, ModelServiceInspectRequest,
    ModelServiceReplayRequest, ModelServiceRequest, ModelServiceRustCheckRequest,
    ModelServiceSelfImproveRequest, parse_model_service_http_request,
};
#[cfg(test)]
use model_service::response::model_service_state_response_json;
#[cfg(test)]
use model_service::server::run_model_service_for_args;

const DEFAULT_MODEL_SERVICE_BIND: &str = "127.0.0.1:7878";

fn main() -> std::io::Result<()> {
    let args = Args::parse(env::args().skip(1).collect());
    dispatch::run(args)
}

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
