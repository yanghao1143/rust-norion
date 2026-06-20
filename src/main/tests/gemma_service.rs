use super::*;

#[path = "gemma_service/answer_contract.rs"]
mod answer_contract;
#[path = "gemma_service/business_smoke_cli.rs"]
mod business_smoke_cli;
#[path = "gemma_service/http_smoke.rs"]
mod http_smoke;
#[path = "gemma_service/model_service_smoke_cli.rs"]
mod model_service_smoke_cli;
#[path = "gemma_service/preflight.rs"]
mod preflight;
#[path = "gemma_service/prompt_contract.rs"]
mod prompt_contract;
#[path = "gemma_service/reports.rs"]
mod reports;
#[path = "gemma_service/runtime_cli.rs"]
mod runtime_cli;
#[path = "gemma_service/rust_feedback.rs"]
mod rust_feedback;
#[path = "gemma_service/service_protocol.rs"]
mod service_protocol;

fn write_minimal_gemma_snapshot(snapshot_dir: &Path) {
    fs::create_dir_all(snapshot_dir).unwrap();
    File::create(snapshot_dir.join("config.json")).unwrap();
    File::create(snapshot_dir.join("tokenizer.json")).unwrap();
    File::create(snapshot_dir.join("model-00001-of-00001.safetensors")).unwrap();
}
