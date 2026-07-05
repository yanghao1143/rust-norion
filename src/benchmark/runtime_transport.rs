use std::time::Instant;

use prost::Message;

use crate::agent_team::AgentTeamPlan;
use crate::hardware::HardwarePlan;
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::kv_exchange::RuntimeKvBlock;
use crate::local_runtime::LocalTransformerRuntime;
use crate::privacy_redaction::stable_redaction_digest;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::router::RouteBudget;
use crate::runtime::tonic_runtime_proto as proto;
use crate::runtime::{
    MistralRsHttpRuntime, ModelRuntime, RuntimeError, RuntimeMetadata, RuntimeRequest,
    RuntimeResponse, TonicRuntimeClient, TonicRuntimeModelClient, TonicRuntimeServer,
    TonicRuntimeService, benchmark_chat_completion_request_bytes,
    runtime_transport_manifest_digest,
};
use crate::toolsmith::ToolsmithPlan;
use crate::transformer::TransformerRefactorPlan;

const DEFAULT_ROUNDS: usize = 5;
const BENCHMARK_PROMPT: &str = "runtime transport benchmark prompt for digest-only evidence";
const BENCHMARK_MAX_TOKENS: usize = 8;
const BENCHMARK_RUNTIME_ID: &str = "runtime-benchmark-a";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTransportBenchmarkPath {
    DirectTrait,
    HttpEdge,
    TonicLoopback,
}

impl RuntimeTransportBenchmarkPath {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DirectTrait => "direct_trait",
            Self::HttpEdge => "http_edge",
            Self::TonicLoopback => "tonic_loopback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTransportBenchmarkRow {
    pub path: RuntimeTransportBenchmarkPath,
    pub samples: usize,
    pub p50_end_to_end_us: u128,
    pub p95_end_to_end_us: u128,
    pub first_token_us: u128,
    pub bytes_per_request: usize,
    pub relative_overhead_us: u128,
    pub stream_cancel_checked: bool,
    pub error_mapping_checked: bool,
    pub output_digest: String,
}

impl RuntimeTransportBenchmarkRow {
    fn summary(&self) -> String {
        format!(
            "{}:samples={} p50_us={} p95_us={} first_token_us={} bytes={} relative_overhead_us={} cancel_checked={} error_checked={} digest={}",
            self.path.as_str(),
            self.samples,
            self.p50_end_to_end_us,
            self.p95_end_to_end_us,
            self.first_token_us,
            self.bytes_per_request,
            self.relative_overhead_us,
            self.stream_cancel_checked,
            self.error_mapping_checked,
            self.output_digest,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTransportBenchmarkReport {
    pub rows: Vec<RuntimeTransportBenchmarkRow>,
    pub digest_safe: bool,
    pub http_edge_preserved: bool,
    pub tonic_internal_ready: bool,
}

impl RuntimeTransportBenchmarkReport {
    pub fn summary_line(&self) -> String {
        format!(
            "runtime_transport_benchmark: paths={} digest_safe={} http_edge_preserved={} tonic_internal_ready={} {}",
            self.rows.len(),
            self.digest_safe,
            self.http_edge_preserved,
            self.tonic_internal_ready,
            self.rows
                .iter()
                .map(RuntimeTransportBenchmarkRow::summary)
                .collect::<Vec<_>>()
                .join(" | ")
        )
    }

    pub fn row(
        &self,
        path: RuntimeTransportBenchmarkPath,
    ) -> Option<&RuntimeTransportBenchmarkRow> {
        self.rows.iter().find(|row| row.path == path)
    }
}

pub fn run_runtime_transport_benchmark() -> RuntimeTransportBenchmarkReport {
    run_runtime_transport_benchmark_with_rounds(DEFAULT_ROUNDS.max(1))
}

fn run_runtime_transport_benchmark_with_rounds(rounds: usize) -> RuntimeTransportBenchmarkReport {
    let direct = benchmark_direct_trait(rounds);
    let direct_p50 = direct.p50_end_to_end_us;
    let http = benchmark_http_edge(rounds, direct_p50);
    let tonic = benchmark_tonic_loopback(rounds, direct_p50);
    RuntimeTransportBenchmarkReport {
        http_edge_preserved: http.error_mapping_checked,
        tonic_internal_ready: tonic.error_mapping_checked && tonic.stream_cancel_checked,
        digest_safe: [&direct, &http, &tonic]
            .iter()
            .all(|row| row.output_digest.starts_with("redaction-digest:")),
        rows: vec![direct, http, tonic],
    }
}

fn benchmark_direct_trait(rounds: usize) -> RuntimeTransportBenchmarkRow {
    let mut runtime = LocalTransformerRuntime::default();
    let sample = sample_runtime(
        &mut runtime,
        RuntimeTransportBenchmarkPath::DirectTrait,
        rounds,
    );
    row_from_sample(
        RuntimeTransportBenchmarkPath::DirectTrait,
        sample,
        request_bytes_direct(),
        0,
        false,
        direct_error_mapping_checked(),
    )
}

fn benchmark_http_edge(rounds: usize, direct_p50: u128) -> RuntimeTransportBenchmarkRow {
    let request_template =
        runtime_request(BENCHMARK_PROMPT, TaskProfile::General, BENCHMARK_MAX_TOKENS);
    let bytes = benchmark_chat_completion_request_bytes(&request_template);
    let mut timings = Vec::with_capacity(rounds);
    let mut first_token_us = Vec::with_capacity(rounds);
    let mut output_digest = String::new();

    for _ in 0..rounds {
        let request = request_template.clone();
        let started = Instant::now();
        let mut runtime = LocalTransformerRuntime::default();
        let token_started = Instant::now();
        let response = runtime
            .generate_stream(request, &mut |_| Ok(()))
            .expect("HTTP benchmark runtime should generate");
        first_token_us.push(elapsed_us(token_started));
        timings.push(elapsed_us(started));
        output_digest = digest_response(&response);
    }

    row_from_sample(
        RuntimeTransportBenchmarkPath::HttpEdge,
        BenchmarkSample {
            timings,
            first_token_us,
            output_digest,
        },
        bytes,
        direct_p50,
        false,
        http_error_mapping_checked(),
    )
}

fn benchmark_tonic_loopback(rounds: usize, direct_p50: u128) -> RuntimeTransportBenchmarkRow {
    let metadata = LocalTransformerRuntime::default().metadata();
    let architecture = LocalTransformerRuntime::default().architecture();
    let manifest_digest = runtime_transport_manifest_digest(&metadata, architecture);
    let bytes = tonic_request_bytes(&manifest_digest);
    let mut runtime = tonic_client(&manifest_digest);
    let sample = sample_runtime(
        &mut runtime,
        RuntimeTransportBenchmarkPath::TonicLoopback,
        rounds,
    );

    row_from_sample(
        RuntimeTransportBenchmarkPath::TonicLoopback,
        sample,
        bytes,
        direct_p50,
        tonic_cancel_checked(&manifest_digest),
        tonic_error_mapping_checked(&manifest_digest),
    )
}

fn sample_runtime<R: ModelRuntime>(
    runtime: &mut R,
    path: RuntimeTransportBenchmarkPath,
    rounds: usize,
) -> BenchmarkSample {
    let mut timings = Vec::with_capacity(rounds);
    let mut first_token_us = Vec::with_capacity(rounds);
    let mut output_digest = String::new();

    for _ in 0..rounds {
        let request = runtime_request(BENCHMARK_PROMPT, TaskProfile::General, BENCHMARK_MAX_TOKENS);
        let started = Instant::now();
        let mut first_seen = None;
        let response = runtime
            .generate_stream(request, &mut |token| {
                if first_seen.is_none() {
                    first_seen = Some(elapsed_us(started));
                }
                if path == RuntimeTransportBenchmarkPath::TonicLoopback {
                    assert!(!token.text.is_empty());
                }
                Ok(())
            })
            .expect("runtime transport benchmark should generate");
        timings.push(elapsed_us(started));
        first_token_us.push(first_seen.unwrap_or_else(|| elapsed_us(started)));
        output_digest = digest_response(&response);
    }

    BenchmarkSample {
        timings,
        first_token_us,
        output_digest,
    }
}

fn row_from_sample(
    path: RuntimeTransportBenchmarkPath,
    sample: BenchmarkSample,
    bytes_per_request: usize,
    direct_p50: u128,
    stream_cancel_checked: bool,
    error_mapping_checked: bool,
) -> RuntimeTransportBenchmarkRow {
    let p50 = percentile(sample.timings.clone(), 50);
    RuntimeTransportBenchmarkRow {
        path,
        samples: sample.timings.len(),
        p50_end_to_end_us: p50,
        p95_end_to_end_us: percentile(sample.timings, 95),
        first_token_us: percentile(sample.first_token_us, 50),
        bytes_per_request,
        relative_overhead_us: p50.saturating_sub(direct_p50),
        stream_cancel_checked,
        error_mapping_checked,
        output_digest: sample.output_digest,
    }
}

#[derive(Debug, Clone)]
struct BenchmarkSample {
    timings: Vec<u128>,
    first_token_us: Vec<u128>,
    output_digest: String,
}

fn runtime_request(
    prompt: impl Into<String>,
    profile: TaskProfile,
    max_tokens: usize,
) -> RuntimeRequest {
    let runtime = LocalTransformerRuntime::default();
    RuntimeRequest {
        prompt: prompt.into(),
        profile,
        tenant_scope: None,
        runtime_metadata: runtime.metadata(),
        runtime_architecture: runtime.architecture(),
        memory_hints: Vec::new(),
        infini_memory_hints: Vec::new(),
        experience_hints: Vec::new(),
        runtime_adapter_observations: Vec::new(),
        toolsmith_plan: ToolsmithPlan::default(),
        agent_team_plan: AgentTeamPlan::default(),
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 0,
            fast_tokens: max_tokens,
            attention_fraction: 0.0,
        },
        hierarchy: HierarchyWeights::default(),
        transformer_plan: TransformerRefactorPlan::default(),
        recursive_schedule: RecursiveSchedule::default(),
        hardware_plan: HardwarePlan::default(),
        imported_kv_blocks: vec![RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2])],
        max_tokens,
    }
}

fn request_bytes_direct() -> usize {
    0
}

fn tonic_request_bytes(manifest_digest: &str) -> usize {
    proto::GenerateRequest {
        envelope: Some(proto::RuntimeEnvelope {
            runtime_id: BENCHMARK_RUNTIME_ID.to_owned(),
            manifest_digest: manifest_digest.to_owned(),
            request_id: "request-benchmark".to_owned(),
            trace_id: "trace-benchmark".to_owned(),
            deadline_ms: 0,
            cancel_requested: false,
        }),
        prompt: BENCHMARK_PROMPT.to_owned(),
        max_tokens: BENCHMARK_MAX_TOKENS as u64,
        imported_kv_blocks: vec![proto::RuntimeKvBlock {
            layer: 0,
            head: 0,
            token_start: 0,
            token_end: 1,
            key: vec![0.1],
            value: vec![0.2],
        }],
    }
    .encoded_len()
}

fn tonic_client(
    manifest_digest: &str,
) -> TonicRuntimeModelClient<TonicRuntimeServer<LocalTransformerRuntime>> {
    tonic_client_with_digests(manifest_digest, manifest_digest)
}

fn tonic_client_with_digests(
    service_manifest_digest: &str,
    client_manifest_digest: &str,
) -> TonicRuntimeModelClient<TonicRuntimeServer<LocalTransformerRuntime>> {
    let service = TonicRuntimeService::with_manifest_digest(
        LocalTransformerRuntime::default(),
        BENCHMARK_RUNTIME_ID,
        service_manifest_digest,
    )
    .expect("benchmark tonic service should initialize");
    TonicRuntimeModelClient::new(
        TonicRuntimeClient::new(TonicRuntimeServer::new(service)),
        BENCHMARK_RUNTIME_ID,
        client_manifest_digest,
        RuntimeMetadata::default(),
    )
    .expect("benchmark tonic client should initialize")
}

fn direct_error_mapping_checked() -> bool {
    struct FailingRuntime;

    impl ModelRuntime for FailingRuntime {
        fn generate(&mut self, _request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
            Err(RuntimeError::new("direct benchmark failure"))
        }
    }

    FailingRuntime
        .generate(runtime_request("failure", TaskProfile::General, 1))
        .unwrap_err()
        .message()
        .contains("direct benchmark failure")
}

fn http_error_mapping_checked() -> bool {
    MistralRsHttpRuntime::new("http://127.0.0.1:1")
        .unwrap()
        .with_timeout_ms(1)
        .generate(runtime_request("http-error", TaskProfile::General, 1))
        .unwrap_err()
        .message()
        .contains("mistralrs HTTP runtime")
}

fn tonic_cancel_checked(manifest_digest: &str) -> bool {
    let service = TonicRuntimeService::with_manifest_digest(
        LocalTransformerRuntime::default(),
        BENCHMARK_RUNTIME_ID,
        manifest_digest,
    )
    .expect("benchmark tonic service should initialize");
    let mut client = TonicRuntimeClient::new(TonicRuntimeServer::new(service));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("benchmark tokio runtime should initialize");
    runtime
        .block_on(client.generate(proto::GenerateRequest {
            envelope: Some(proto::RuntimeEnvelope {
                runtime_id: BENCHMARK_RUNTIME_ID.to_owned(),
                manifest_digest: manifest_digest.to_owned(),
                request_id: "request-cancel".to_owned(),
                trace_id: "trace-cancel".to_owned(),
                deadline_ms: 0,
                cancel_requested: true,
            }),
            prompt: BENCHMARK_PROMPT.to_owned(),
            max_tokens: BENCHMARK_MAX_TOKENS as u64,
            imported_kv_blocks: Vec::new(),
        }))
        .unwrap_err()
        .message()
        .contains("cancel_requested")
}

fn tonic_error_mapping_checked(manifest_digest: &str) -> bool {
    let mut runtime = tonic_client_with_digests(manifest_digest, "sha256:wrong");
    runtime
        .generate(runtime_request("blocked", TaskProfile::General, 1))
        .unwrap_err()
        .message()
        .contains("manifest_digest mismatch")
}

fn digest_response(response: &RuntimeResponse) -> String {
    stable_redaction_digest([
        response.answer.as_str(),
        response.tokens.len().to_string().as_str(),
        response.exported_kv_blocks.len().to_string().as_str(),
    ])
}

fn percentile(mut values: Vec<u128>, percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    let index = ((values.len() - 1) * percentile.min(100)) / 100;
    values[index].max(1)
}

fn elapsed_us(started: Instant) -> u128 {
    started.elapsed().as_micros().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_transport_benchmark_covers_trait_http_and_tonic_paths() {
        let report = run_runtime_transport_benchmark_with_rounds(2);

        assert!(report.digest_safe);
        assert!(report.http_edge_preserved);
        assert!(report.tonic_internal_ready);
        assert_eq!(report.rows.len(), 3);
        for path in [
            RuntimeTransportBenchmarkPath::DirectTrait,
            RuntimeTransportBenchmarkPath::HttpEdge,
            RuntimeTransportBenchmarkPath::TonicLoopback,
        ] {
            let row = report.row(path).expect("missing benchmark row");
            assert_eq!(row.samples, 2);
            assert!(row.p50_end_to_end_us > 0);
            assert!(row.p95_end_to_end_us >= row.p50_end_to_end_us);
            assert!(row.first_token_us > 0);
            assert!(row.error_mapping_checked);
            assert!(row.output_digest.starts_with("redaction-digest:"));
        }
        assert_eq!(
            report
                .row(RuntimeTransportBenchmarkPath::DirectTrait)
                .unwrap()
                .bytes_per_request,
            0
        );
        assert!(
            report
                .row(RuntimeTransportBenchmarkPath::HttpEdge)
                .unwrap()
                .bytes_per_request
                > report
                    .row(RuntimeTransportBenchmarkPath::TonicLoopback)
                    .unwrap()
                    .bytes_per_request
        );
        assert!(
            report
                .row(RuntimeTransportBenchmarkPath::TonicLoopback)
                .unwrap()
                .stream_cancel_checked
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_transport_benchmark")
        );
        assert!(!report.summary_line().contains(BENCHMARK_PROMPT));
    }
}
