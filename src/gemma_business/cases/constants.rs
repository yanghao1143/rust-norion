pub const GEMMA_BUSINESS_SMOKE_PROMPT: &str = "Answer in one Chinese sentence. Treat runtime_model_experiences as an audit telemetry field, not a Rust API, and include that exact field name while confirming the local Gemma 4 12B RuntimeBackend handled this Rust coding business request.";
pub const GEMMA_BUSINESS_SMOKE_DIR: &str = "target/gemma-business-smoke";
pub const GEMMA_BUSINESS_CYCLE_SMOKE_DIR: &str = "target/gemma-business-cycle-smoke";
pub const GEMMA_MODEL_SERVICE_SMOKE_DIR: &str = "target/gemma-model-service-smoke";
pub const GEMMA_BUSINESS_CYCLE_SMOKE_REPORT_FILE: &str = "gemma-business-cycle-smoke-report.json";
pub const GEMMA4_12B_SMOKE_COLD_START_TIMEOUT_MS: u64 = 360_000;
pub const GEMMA_SMOKE_DEFAULT_KEEP_RUNS: usize = 5;
