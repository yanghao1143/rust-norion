pub mod proto {
    tonic::include_proto!("norion.runtime.v1");
}

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tonic::transport::{Channel, Endpoint};

use crate::agent_team::AgentTeamPlan;
use crate::hardware::HardwarePlan;
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::kv_exchange::RuntimeKvBlock;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::reflection::ReasoningStep;
use crate::router::RouteBudget;
#[cfg(test)]
use crate::runtime_manifest::TransformerRuntimeArchitecture;
use crate::toolsmith::ToolsmithPlan;
use crate::transformer::TransformerRefactorPlan;

#[cfg(test)]
use super::INTERNAL_RUNTIME_PROTO_PACKAGE;
use super::{
    InternalRuntimeEnvelope, InternalRuntimeLoopback, InternalRuntimeMethod, ModelRuntime,
    RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeResponse, RuntimeToken,
};

pub type TonicRuntimeClient<T> = proto::runtime_transport_client::RuntimeTransportClient<T>;
pub type TonicRuntimeServer<R> =
    proto::runtime_transport_server::RuntimeTransportServer<TonicRuntimeService<R>>;

const TONIC_STREAM_BUFFER_CAPACITY: usize = 1024;

#[derive(Debug)]
pub struct TonicRuntimeModelClient<T> {
    client: Mutex<TonicRuntimeClient<T>>,
    runtime: tokio::runtime::Runtime,
    runtime_id: String,
    manifest_digest: String,
    request_counter: Mutex<u64>,
    metadata: RuntimeMetadata,
}

impl<T> TonicRuntimeModelClient<T> {
    pub fn new(
        client: TonicRuntimeClient<T>,
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
        metadata: RuntimeMetadata,
    ) -> Result<Self, RuntimeError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .map_err(|error| {
                RuntimeError::new(format!("tonic runtime client runtime init failed: {error}"))
            })?;
        let runtime_id = runtime_id.into();
        let manifest_digest = manifest_digest.into();
        if !is_tonic_transport_id(&runtime_id) {
            return Err(RuntimeError::new(
                "tonic runtime client runtime_id is invalid",
            ));
        }
        if !manifest_digest.starts_with("sha256:") {
            return Err(RuntimeError::new(
                "tonic runtime client manifest_digest must be sha256",
            ));
        }
        Ok(Self {
            client: Mutex::new(client),
            runtime,
            runtime_id,
            manifest_digest,
            request_counter: Mutex::new(0),
            metadata,
        })
    }

    pub fn refresh_metadata(&mut self) -> Result<RuntimeMetadata, RuntimeError>
    where
        T: tonic::client::GrpcService<tonic::body::Body> + Send + 'static,
        T::Error: Into<tonic::codegen::StdError>,
        T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
        <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
    {
        let envelope = self.envelope(InternalRuntimeMethod::GetRuntimeMetadata)?;
        let metadata = {
            let mut client = self.lock_client()?;
            self.runtime
                .block_on(client.get_runtime_metadata(proto::RuntimeMetadataRequest {
                    envelope: Some(envelope),
                }))
                .map_err(tonic_status_to_runtime_error)?
        };
        self.metadata = metadata_response_from_proto(metadata.into_inner());
        Ok(self.metadata.clone())
    }

    fn envelope(
        &self,
        method: InternalRuntimeMethod,
    ) -> Result<proto::RuntimeEnvelope, RuntimeError> {
        let mut counter = self
            .request_counter
            .lock()
            .map_err(|_| RuntimeError::new("tonic runtime client counter lock poisoned"))?;
        *counter = counter.saturating_add(1);
        Ok(proto::RuntimeEnvelope {
            runtime_id: self.runtime_id.clone(),
            manifest_digest: self.manifest_digest.clone(),
            request_id: format!("request-{}-{}", *counter, method.as_str()),
            trace_id: format!("trace-{}-{}", *counter, method.as_str()),
            deadline_ms: 0,
            cancel_requested: false,
        })
    }

    fn lock_client(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, TonicRuntimeClient<T>>, RuntimeError> {
        self.client
            .lock()
            .map_err(|_| RuntimeError::new("tonic runtime client lock poisoned"))
    }
}

impl TonicRuntimeModelClient<Channel> {
    pub fn connect_lazy(
        endpoint: Endpoint,
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
        metadata: RuntimeMetadata,
    ) -> Result<Self, RuntimeError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .map_err(|error| {
                RuntimeError::new(format!("tonic runtime client runtime init failed: {error}"))
            })?;
        let client = runtime.block_on(async { TonicRuntimeClient::new(endpoint.connect_lazy()) });
        Self::new_with_runtime(client, runtime, runtime_id, manifest_digest, metadata)
    }

    fn new_with_runtime(
        client: TonicRuntimeClient<Channel>,
        runtime: tokio::runtime::Runtime,
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
        metadata: RuntimeMetadata,
    ) -> Result<Self, RuntimeError> {
        let runtime_id = runtime_id.into();
        let manifest_digest = manifest_digest.into();
        if !is_tonic_transport_id(&runtime_id) {
            return Err(RuntimeError::new(
                "tonic runtime client runtime_id is invalid",
            ));
        }
        if !manifest_digest.starts_with("sha256:") {
            return Err(RuntimeError::new(
                "tonic runtime client manifest_digest must be sha256",
            ));
        }
        Ok(Self {
            client: Mutex::new(client),
            runtime,
            runtime_id,
            manifest_digest,
            request_counter: Mutex::new(0),
            metadata,
        })
    }
}

impl<T> ModelRuntime for TonicRuntimeModelClient<T>
where
    T: tonic::client::GrpcService<tonic::body::Body> + Send + 'static,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    fn metadata(&self) -> RuntimeMetadata {
        self.metadata.clone()
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        let envelope = self.envelope(InternalRuntimeMethod::ImportKv)?;
        let blocks = blocks.iter().map(proto::RuntimeKvBlock::from).collect();
        let mut client = self.lock_client()?;
        let response = self
            .runtime
            .block_on(client.import_kv(proto::ImportKvRequest {
                envelope: Some(envelope),
                blocks,
            }))
            .map_err(tonic_status_to_runtime_error)?;
        Ok(response.into_inner().imported_blocks as usize)
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        let envelope = self.envelope(InternalRuntimeMethod::ExportKv)?;
        let mut client = self.lock_client()?;
        let response = self
            .runtime
            .block_on(client.export_kv(proto::ExportKvRequest {
                envelope: Some(envelope),
            }))
            .map_err(tonic_status_to_runtime_error)?;
        Ok(response
            .into_inner()
            .blocks
            .into_iter()
            .map(runtime_kv_block_from_proto)
            .collect())
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        let envelope = self.envelope(InternalRuntimeMethod::Generate)?;
        let trace_envelope = envelope.clone();
        let started = Instant::now();
        let mut client = self.lock_client()?;
        let response = self
            .runtime
            .block_on(client.generate(generate_request_to_proto(request, envelope)))
            .map_err(tonic_status_to_runtime_error)?;
        let mut response = generate_response_from_proto(response.into_inner());
        response.trace.push(client_trace_step(
            &trace_envelope,
            &self.metadata.model_id,
            "ok",
            started.elapsed().as_micros(),
        ));
        Ok(response)
    }

    fn generate_stream(
        &mut self,
        request: RuntimeRequest,
        on_token: &mut dyn FnMut(&RuntimeToken) -> Result<(), RuntimeError>,
    ) -> Result<RuntimeResponse, RuntimeError> {
        let envelope = self.envelope(InternalRuntimeMethod::GenerateStream)?;
        let trace_envelope = envelope.clone();
        let started = Instant::now();
        let mut response = RuntimeResponse::new("");
        let mut client = self.lock_client()?;
        let mut stream = self
            .runtime
            .block_on(client.generate_stream(generate_request_to_proto(request, envelope)))
            .map_err(tonic_status_to_runtime_error)?
            .into_inner();

        loop {
            let event = self
                .runtime
                .block_on(stream.message())
                .map_err(tonic_status_to_runtime_error)?;
            let Some(event) = event else {
                break;
            };
            response.trace.extend(
                event
                    .trace
                    .into_iter()
                    .map(|step| ReasoningStep::new(step.label, step.content, step.confidence)),
            );
            let Some(token) = event.token else {
                continue;
            };
            let token = runtime_token_from_proto(token);
            on_token(&token)?;
            response.answer.push_str(&token.text);
            response.tokens.push(token);
        }
        response.trace.push(client_trace_step(
            &trace_envelope,
            &self.metadata.model_id,
            "ok",
            started.elapsed().as_micros(),
        ));
        Ok(response)
    }
}

impl From<&RuntimeToken> for proto::RuntimeToken {
    fn from(token: &RuntimeToken) -> Self {
        Self {
            text: token.text.clone(),
            logprob: token.logprob,
            entropy: token.entropy,
        }
    }
}

impl From<&RuntimeKvBlock> for proto::RuntimeKvBlock {
    fn from(block: &RuntimeKvBlock) -> Self {
        Self {
            layer: block.layer as u32,
            head: block.head as u32,
            token_start: block.token_start as u32,
            token_end: block.token_end as u32,
            key: block.key.clone(),
            value: block.value.clone(),
        }
    }
}

impl From<&ReasoningStep> for proto::TraceStep {
    fn from(step: &ReasoningStep) -> Self {
        Self {
            label: step.label.clone(),
            content: step.content.clone(),
            confidence: step.confidence,
        }
    }
}

pub fn tonic_status_to_runtime_error(status: tonic::Status) -> RuntimeError {
    RuntimeError::new(format!(
        "tonic runtime transport error code={} message={}",
        status.code(),
        status.message()
    ))
}

#[derive(Debug)]
pub struct TonicRuntimeService<R> {
    loopback: Mutex<InternalRuntimeLoopback<R>>,
}

impl<R: ModelRuntime> TonicRuntimeService<R> {
    pub fn new(runtime: R, runtime_id: impl Into<String>) -> Result<Self, RuntimeError> {
        Ok(Self::from_loopback(InternalRuntimeLoopback::new(
            runtime, runtime_id,
        )?))
    }

    pub fn with_manifest_digest(
        runtime: R,
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
    ) -> Result<Self, RuntimeError> {
        Ok(Self::from_loopback(
            InternalRuntimeLoopback::with_manifest_digest(runtime, runtime_id, manifest_digest)?,
        ))
    }

    pub fn from_loopback(loopback: InternalRuntimeLoopback<R>) -> Self {
        Self {
            loopback: Mutex::new(loopback),
        }
    }
}

#[tonic::async_trait]
impl<R> proto::runtime_transport_server::RuntimeTransport for TonicRuntimeService<R>
where
    R: ModelRuntime + Send + 'static,
{
    type GenerateStreamStream =
        tokio_stream::wrappers::ReceiverStream<Result<proto::RuntimeTokenEvent, tonic::Status>>;

    async fn get_runtime_metadata(
        &self,
        request: tonic::Request<proto::RuntimeMetadataRequest>,
    ) -> Result<tonic::Response<proto::RuntimeMetadataResponse>, tonic::Status> {
        let request = request.into_inner();
        let envelope =
            envelope_from_proto(request.envelope, InternalRuntimeMethod::GetRuntimeMetadata)?;
        let loopback = self.lock_loopback()?;
        let metadata = loopback
            .metadata(&envelope)
            .map_err(runtime_error_to_status)?;
        Ok(tonic::Response::new(metadata_to_proto(metadata)))
    }

    async fn generate(
        &self,
        request: tonic::Request<proto::GenerateRequest>,
    ) -> Result<tonic::Response<proto::GenerateResponse>, tonic::Status> {
        let request = request.into_inner();
        let envelope =
            envelope_from_proto(request.envelope.clone(), InternalRuntimeMethod::Generate)?;
        let started = Instant::now();
        let mut loopback = self.lock_loopback()?;
        let model_id = loopback.runtime().metadata().model_id;
        let runtime_request = runtime_request_from_proto(&loopback, &request);
        let mut response = loopback
            .generate(&envelope, runtime_request)
            .map_err(runtime_error_to_status)?;
        if runtime_deadline_exceeded(&envelope, started) {
            return Err(tonic::Status::deadline_exceeded(
                "tonic runtime transport deadline exceeded",
            ));
        }
        response.trace.push(server_trace_step(
            &envelope,
            &model_id,
            "ok",
            started.elapsed().as_micros(),
        ));
        Ok(tonic::Response::new(generate_response_to_proto(response)))
    }

    async fn generate_stream(
        &self,
        request: tonic::Request<proto::GenerateRequest>,
    ) -> Result<tonic::Response<Self::GenerateStreamStream>, tonic::Status> {
        let request = request.into_inner();
        let envelope = envelope_from_proto(
            request.envelope.clone(),
            InternalRuntimeMethod::GenerateStream,
        )?;
        let proto_envelope = envelope_to_proto(&envelope);
        let (tx, rx) = tokio::sync::mpsc::channel(TONIC_STREAM_BUFFER_CAPACITY);
        let started = Instant::now();

        let mut loopback = self.lock_loopback()?;
        let model_id = loopback.runtime().metadata().model_id;
        let runtime_request = runtime_request_from_proto(&loopback, &request);
        let mut response = loopback
            .generate_stream(&envelope, runtime_request, &mut |token| {
                tx.try_send(Ok(proto::RuntimeTokenEvent {
                    envelope: Some(proto_envelope.clone()),
                    token: Some(proto::RuntimeToken::from(token)),
                    trace: Vec::new(),
                }))
                .map_err(|_| {
                    RuntimeError::new(
                        "tonic runtime transport stream backpressure: response channel full",
                    )
                })
            })
            .map_err(runtime_error_to_status)?;
        if runtime_deadline_exceeded(&envelope, started) {
            return Err(tonic::Status::deadline_exceeded(
                "tonic runtime transport deadline exceeded",
            ));
        }
        response.trace.push(server_trace_step(
            &envelope,
            &model_id,
            "ok",
            started.elapsed().as_micros(),
        ));
        tx.try_send(Ok(proto::RuntimeTokenEvent {
            envelope: Some(proto_envelope),
            token: None,
            trace: response.trace.iter().map(proto::TraceStep::from).collect(),
        }))
        .map_err(|_| {
            runtime_error_to_status(RuntimeError::new(
                "tonic runtime transport stream backpressure: trace channel full",
            ))
        })?;
        drop(tx);

        Ok(tonic::Response::new(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        ))
    }

    async fn import_kv(
        &self,
        request: tonic::Request<proto::ImportKvRequest>,
    ) -> Result<tonic::Response<proto::ImportKvResponse>, tonic::Status> {
        let request = request.into_inner();
        let envelope = envelope_from_proto(request.envelope, InternalRuntimeMethod::ImportKv)?;
        let blocks = request
            .blocks
            .into_iter()
            .map(runtime_kv_block_from_proto)
            .collect::<Vec<_>>();
        let mut loopback = self.lock_loopback()?;
        let imported_blocks = loopback
            .import_kv(&envelope, &blocks)
            .map_err(runtime_error_to_status)?;
        Ok(tonic::Response::new(proto::ImportKvResponse {
            imported_blocks: imported_blocks as u64,
        }))
    }

    async fn export_kv(
        &self,
        request: tonic::Request<proto::ExportKvRequest>,
    ) -> Result<tonic::Response<proto::ExportKvResponse>, tonic::Status> {
        let request = request.into_inner();
        let envelope = envelope_from_proto(request.envelope, InternalRuntimeMethod::ExportKv)?;
        let mut loopback = self.lock_loopback()?;
        let blocks = loopback
            .export_kv(&envelope)
            .map_err(runtime_error_to_status)?
            .iter()
            .map(proto::RuntimeKvBlock::from)
            .collect();
        Ok(tonic::Response::new(proto::ExportKvResponse { blocks }))
    }
}

impl<R> TonicRuntimeService<R> {
    fn lock_loopback(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, InternalRuntimeLoopback<R>>, tonic::Status> {
        self.loopback
            .lock()
            .map_err(|_| tonic::Status::internal("tonic runtime transport lock poisoned"))
    }
}

fn envelope_from_proto(
    envelope: Option<proto::RuntimeEnvelope>,
    method: InternalRuntimeMethod,
) -> Result<InternalRuntimeEnvelope, tonic::Status> {
    let envelope = envelope
        .ok_or_else(|| tonic::Status::invalid_argument("tonic runtime envelope is required"))?;
    let mut internal = InternalRuntimeEnvelope::new(
        method,
        envelope.runtime_id,
        envelope.manifest_digest,
        envelope.request_id,
        envelope.trace_id,
    );
    if envelope.deadline_ms > 0 {
        internal = internal.with_deadline_ms(envelope.deadline_ms);
    }
    Ok(internal.with_cancel_requested(envelope.cancel_requested))
}

fn envelope_to_proto(envelope: &InternalRuntimeEnvelope) -> proto::RuntimeEnvelope {
    proto::RuntimeEnvelope {
        runtime_id: envelope.runtime_id.clone(),
        manifest_digest: envelope.manifest_digest.clone(),
        request_id: envelope.request_id.clone(),
        trace_id: envelope.trace_id.clone(),
        deadline_ms: envelope.deadline_ms.unwrap_or_default(),
        cancel_requested: envelope.cancel_requested,
    }
}

fn runtime_request_from_proto<R: ModelRuntime>(
    loopback: &InternalRuntimeLoopback<R>,
    request: &proto::GenerateRequest,
) -> RuntimeRequest {
    let metadata = loopback.runtime().metadata();
    let architecture = loopback.runtime().architecture();
    RuntimeRequest {
        prompt: request.prompt.clone(),
        profile: TaskProfile::General,
        tenant_scope: None,
        runtime_metadata: metadata,
        runtime_architecture: architecture,
        memory_hints: Vec::new(),
        infini_memory_hints: Vec::new(),
        experience_hints: Vec::new(),
        runtime_adapter_observations: Vec::new(),
        toolsmith_plan: ToolsmithPlan::default(),
        agent_team_plan: AgentTeamPlan::default(),
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 0,
            fast_tokens: request.max_tokens as usize,
            attention_fraction: 0.0,
        },
        hierarchy: HierarchyWeights::default(),
        transformer_plan: TransformerRefactorPlan::default(),
        recursive_schedule: RecursiveSchedule::default(),
        hardware_plan: HardwarePlan::default(),
        imported_kv_blocks: request
            .imported_kv_blocks
            .iter()
            .cloned()
            .map(runtime_kv_block_from_proto)
            .collect(),
        max_tokens: (request.max_tokens as usize).max(1),
    }
}

fn metadata_to_proto(metadata: RuntimeMetadata) -> proto::RuntimeMetadataResponse {
    proto::RuntimeMetadataResponse {
        model_id: metadata.model_id,
        tokenizer: metadata.tokenizer,
        native_context_window: metadata.native_context_window as u64,
        embedding_dimensions: metadata.embedding_dimensions as u64,
        supports_kv_import: metadata.supports_kv_import,
        supports_kv_export: metadata.supports_kv_export,
    }
}

fn metadata_response_from_proto(response: proto::RuntimeMetadataResponse) -> RuntimeMetadata {
    RuntimeMetadata::new(
        response.model_id,
        response.tokenizer,
        response.native_context_window as usize,
        response.embedding_dimensions as usize,
    )
    .with_kv_exchange(response.supports_kv_import, response.supports_kv_export)
}

fn generate_request_to_proto(
    request: RuntimeRequest,
    envelope: proto::RuntimeEnvelope,
) -> proto::GenerateRequest {
    proto::GenerateRequest {
        envelope: Some(envelope),
        prompt: request.prompt,
        max_tokens: request.max_tokens as u64,
        imported_kv_blocks: request
            .imported_kv_blocks
            .iter()
            .map(proto::RuntimeKvBlock::from)
            .collect(),
    }
}

fn generate_response_to_proto(response: RuntimeResponse) -> proto::GenerateResponse {
    proto::GenerateResponse {
        answer: response.answer,
        tokens: response
            .tokens
            .iter()
            .map(proto::RuntimeToken::from)
            .collect(),
        trace: response.trace.iter().map(proto::TraceStep::from).collect(),
        exported_kv_blocks: response
            .exported_kv_blocks
            .iter()
            .map(proto::RuntimeKvBlock::from)
            .collect(),
    }
}

fn generate_response_from_proto(response: proto::GenerateResponse) -> RuntimeResponse {
    let mut runtime_response = RuntimeResponse::new(response.answer);
    runtime_response.tokens = response
        .tokens
        .into_iter()
        .map(runtime_token_from_proto)
        .collect();
    runtime_response.trace = response
        .trace
        .into_iter()
        .map(|step| ReasoningStep::new(step.label, step.content, step.confidence))
        .collect();
    runtime_response.exported_kv_blocks = response
        .exported_kv_blocks
        .into_iter()
        .map(runtime_kv_block_from_proto)
        .collect();
    runtime_response
}

fn runtime_token_from_proto(token: proto::RuntimeToken) -> RuntimeToken {
    RuntimeToken {
        text: token.text,
        logprob: token.logprob,
        entropy: token.entropy,
    }
}

fn client_trace_step(
    envelope: &proto::RuntimeEnvelope,
    model_id: &str,
    status: &str,
    latency_us: u128,
) -> ReasoningStep {
    ReasoningStep::new(
        "tonic_runtime_client_transport",
        format!(
            "transport_kind=tonic runtime_id={} model_id={} manifest_digest={} request_id={} trace_id={} status={} latency_us={}",
            envelope.runtime_id,
            model_id,
            envelope.manifest_digest,
            envelope.request_id,
            envelope.trace_id,
            status,
            latency_us
        ),
        0.88,
    )
}

fn server_trace_step(
    envelope: &InternalRuntimeEnvelope,
    model_id: &str,
    status: &str,
    latency_us: u128,
) -> ReasoningStep {
    ReasoningStep::new(
        "tonic_runtime_server_transport",
        format!(
            "transport_kind=tonic runtime_id={} model_id={} manifest_digest={} request_id={} trace_id={} deadline_ms={} status={} latency_us={} tonic_codegen_ready=true",
            envelope.runtime_id,
            model_id,
            envelope.manifest_digest,
            envelope.request_id,
            envelope.trace_id,
            envelope
                .deadline_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            status,
            latency_us
        ),
        0.88,
    )
}

fn runtime_kv_block_from_proto(block: proto::RuntimeKvBlock) -> RuntimeKvBlock {
    RuntimeKvBlock::new(
        block.layer as usize,
        block.head as usize,
        block.token_start as usize,
        block.token_end as usize,
        block.key,
        block.value,
    )
}

fn is_tonic_transport_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_' | '.'))
}

fn runtime_error_to_status(error: RuntimeError) -> tonic::Status {
    if error.message().contains("backpressure") {
        return tonic::Status::resource_exhausted(error.message().to_owned());
    }
    tonic::Status::failed_precondition(error.message().to_owned())
}

fn runtime_deadline_exceeded(envelope: &InternalRuntimeEnvelope, started: Instant) -> bool {
    envelope
        .deadline_ms
        .is_some_and(|deadline_ms| started.elapsed() > Duration::from_millis(deadline_ms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{InferenceRequest, NoironEngine};
    use crate::runtime::RuntimeBackend;
    use crate::tenant_scope::TenantScope;
    use proto::runtime_transport_server::RuntimeTransport;
    use tokio_stream::StreamExt;

    #[derive(Debug, Default)]
    struct TonicProofRuntime {
        imported: Vec<RuntimeKvBlock>,
        stream_calls: usize,
    }

    impl ModelRuntime for TonicProofRuntime {
        fn metadata(&self) -> RuntimeMetadata {
            RuntimeMetadata::new("tonic-proof-runtime", "proof-tokenizer", 2048, 2)
                .with_kv_exchange(true, true)
        }

        fn architecture(&self) -> TransformerRuntimeArchitecture {
            TransformerRuntimeArchitecture::new(2, 2, 1, 1, 128)
        }

        fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
            self.imported = blocks.to_vec();
            Ok(self.imported.len())
        }

        fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
            Ok(self.imported.clone())
        }

        fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
            let mut response = RuntimeResponse::new(format!(
                "tonic:{}:{}",
                request.prompt,
                request.imported_kv_blocks.len()
            ));
            response.tokens = vec![RuntimeToken {
                text: "tonic".to_owned(),
                logprob: Some(-0.2),
                entropy: Some(0.4),
            }];
            response.exported_kv_blocks = request.imported_kv_blocks;
            response.trace.push(ReasoningStep::new(
                "tonic_proof_runtime",
                format!(
                    "package={} max_tokens={}",
                    INTERNAL_RUNTIME_PROTO_PACKAGE, request.max_tokens
                ),
                0.91,
            ));
            Ok(response)
        }

        fn generate_stream(
            &mut self,
            _request: RuntimeRequest,
            on_token: &mut dyn FnMut(&RuntimeToken) -> Result<(), RuntimeError>,
        ) -> Result<RuntimeResponse, RuntimeError> {
            self.stream_calls += 1;
            let mut response = RuntimeResponse::new("streamed");
            for text in ["alpha", "beta"] {
                let token = RuntimeToken::new(text);
                on_token(&token)?;
                response.tokens.push(token);
            }
            Ok(response)
        }
    }

    #[derive(Debug, Default)]
    struct SlowGenerateRuntime;

    impl ModelRuntime for SlowGenerateRuntime {
        fn metadata(&self) -> RuntimeMetadata {
            RuntimeMetadata::new("slow-proof-runtime", "proof-tokenizer", 2048, 2)
        }

        fn architecture(&self) -> TransformerRuntimeArchitecture {
            TransformerRuntimeArchitecture::new(2, 2, 1, 1, 128)
        }

        fn generate(&mut self, _request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
            std::thread::sleep(std::time::Duration::from_millis(5));
            Ok(RuntimeResponse::new("late"))
        }
    }

    #[derive(Debug, Default)]
    struct FloodStreamRuntime;

    impl ModelRuntime for FloodStreamRuntime {
        fn metadata(&self) -> RuntimeMetadata {
            RuntimeMetadata::new("flood-proof-runtime", "proof-tokenizer", 2048, 2)
        }

        fn architecture(&self) -> TransformerRuntimeArchitecture {
            TransformerRuntimeArchitecture::new(2, 2, 1, 1, 128)
        }

        fn generate(&mut self, _request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
            Ok(RuntimeResponse::new("flood"))
        }

        fn generate_stream(
            &mut self,
            _request: RuntimeRequest,
            on_token: &mut dyn FnMut(&RuntimeToken) -> Result<(), RuntimeError>,
        ) -> Result<RuntimeResponse, RuntimeError> {
            for index in 0..=TONIC_STREAM_BUFFER_CAPACITY {
                on_token(&RuntimeToken::new(format!("t{index}")))?;
            }
            Ok(RuntimeResponse::new("flood"))
        }
    }

    #[tokio::test]
    async fn tonic_service_uses_generated_proto_and_loopback_generate_path() {
        let service = service();

        let metadata = service
            .get_runtime_metadata(tonic::Request::new(proto::RuntimeMetadataRequest {
                envelope: Some(envelope(InternalRuntimeMethod::GetRuntimeMetadata)),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(metadata.model_id, "tonic-proof-runtime");
        assert!(metadata.supports_kv_import);

        let response = service
            .generate(tonic::Request::new(proto::GenerateRequest {
                envelope: Some(envelope(InternalRuntimeMethod::Generate)),
                prompt: "hello".to_owned(),
                max_tokens: 7,
                imported_kv_blocks: vec![proto_kv_block()],
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.answer, "tonic:hello:1");
        assert_eq!(response.tokens[0].text, "tonic");
        assert_eq!(response.trace[0].label, "tonic_proof_runtime");
        assert!(response.trace.iter().any(|step| {
            step.label == "tonic_runtime_server_transport"
                && step.content.contains("transport_kind=tonic")
                && step.content.contains("runtime_id=runtime-a")
                && step.content.contains("model_id=tonic-proof-runtime")
                && step.content.contains("manifest_digest=sha256:manifest-a")
                && step.content.contains("request_id=request-Generate")
                && step.content.contains("trace_id=trace-Generate")
                && step.content.contains("deadline_ms=1000")
                && step.content.contains("status=ok")
                && step.content.contains("latency_us=")
                && step.content.contains("tonic_codegen_ready=true")
        }));
        assert_eq!(response.exported_kv_blocks.len(), 1);
    }

    #[tokio::test]
    async fn tonic_service_reuses_loopback_manifest_gate_before_runtime() {
        let service = service();
        let mut envelope = envelope(InternalRuntimeMethod::Generate);
        envelope.manifest_digest = "sha256:wrong".to_owned();

        let error = service
            .generate(tonic::Request::new(proto::GenerateRequest {
                envelope: Some(envelope),
                prompt: "blocked".to_owned(),
                max_tokens: 1,
                imported_kv_blocks: Vec::new(),
            }))
            .await
            .unwrap_err();

        assert_eq!(error.code(), tonic::Code::FailedPrecondition);
        assert!(error.message().contains("manifest_digest mismatch"));
    }

    #[tokio::test]
    async fn tonic_service_maps_elapsed_deadline_to_deadline_exceeded() {
        let service = TonicRuntimeService::with_manifest_digest(
            SlowGenerateRuntime,
            "runtime-a",
            "sha256:manifest-a",
        )
        .unwrap();
        let mut envelope = envelope(InternalRuntimeMethod::Generate);
        envelope.deadline_ms = 1;

        let error = service
            .generate(tonic::Request::new(proto::GenerateRequest {
                envelope: Some(envelope),
                prompt: "late".to_owned(),
                max_tokens: 1,
                imported_kv_blocks: Vec::new(),
            }))
            .await
            .unwrap_err();

        assert_eq!(error.code(), tonic::Code::DeadlineExceeded);
        assert!(error.message().contains("deadline exceeded"));
    }

    #[tokio::test]
    async fn tonic_service_imports_exports_kv_through_loopback() {
        let service = service();

        let imported = service
            .import_kv(tonic::Request::new(proto::ImportKvRequest {
                envelope: Some(envelope(InternalRuntimeMethod::ImportKv)),
                blocks: vec![proto_kv_block()],
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(imported.imported_blocks, 1);

        let exported = service
            .export_kv(tonic::Request::new(proto::ExportKvRequest {
                envelope: Some(envelope(InternalRuntimeMethod::ExportKv)),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(exported.blocks.len(), 1);
        assert_eq!(exported.blocks[0].key, vec![0.1, 0.2]);
    }

    #[tokio::test]
    async fn tonic_stream_emits_tokens_and_propagates_cancel_gate() {
        let service = service();

        let mut stream = service
            .generate_stream(tonic::Request::new(proto::GenerateRequest {
                envelope: Some(envelope(InternalRuntimeMethod::GenerateStream)),
                prompt: "stream".to_owned(),
                max_tokens: 2,
                imported_kv_blocks: Vec::new(),
            }))
            .await
            .unwrap()
            .into_inner();

        let first = stream.next().await.unwrap().unwrap();
        let second = stream.next().await.unwrap().unwrap();
        let trace = stream.next().await.unwrap().unwrap();
        assert_eq!(first.token.unwrap().text, "alpha");
        assert_eq!(second.token.unwrap().text, "beta");
        assert!(trace.token.is_none());
        assert!(trace.trace.iter().any(|step| {
            step.label == "tonic_runtime_server_transport"
                && step.content.contains("transport_kind=tonic")
                && step.content.contains("status=ok")
                && step.content.contains("latency_us=")
                && step.content.contains("tonic_codegen_ready=true")
        }));
        assert!(stream.next().await.is_none());

        let mut cancelled = envelope(InternalRuntimeMethod::GenerateStream);
        cancelled.cancel_requested = true;
        let error = service
            .generate_stream(tonic::Request::new(proto::GenerateRequest {
                envelope: Some(cancelled),
                prompt: "cancel".to_owned(),
                max_tokens: 2,
                imported_kv_blocks: Vec::new(),
            }))
            .await
            .unwrap_err();
        assert_eq!(error.code(), tonic::Code::FailedPrecondition);
        assert!(error.message().contains("cancel_requested"));
    }

    #[tokio::test]
    async fn tonic_stream_maps_full_response_channel_to_backpressure() {
        let service = TonicRuntimeService::with_manifest_digest(
            FloodStreamRuntime,
            "runtime-a",
            "sha256:manifest-a",
        )
        .unwrap();

        let error = service
            .generate_stream(tonic::Request::new(proto::GenerateRequest {
                envelope: Some(envelope(InternalRuntimeMethod::GenerateStream)),
                prompt: "stream".to_owned(),
                max_tokens: TONIC_STREAM_BUFFER_CAPACITY as u64,
                imported_kv_blocks: Vec::new(),
            }))
            .await
            .unwrap_err();

        assert_eq!(error.code(), tonic::Code::ResourceExhausted);
        assert!(error.message().contains("backpressure"));
        assert!(error.message().contains("response channel full"));
    }

    #[test]
    fn tonic_model_client_refreshes_metadata_and_generates_via_model_runtime() {
        let mut runtime = client_runtime("sha256:manifest-a");

        let metadata = runtime.refresh_metadata().unwrap();
        assert_eq!(metadata.model_id, "tonic-proof-runtime");
        assert_eq!(runtime.metadata().tokenizer, "proof-tokenizer");

        let response = runtime
            .generate(runtime_request("client generate"))
            .unwrap();

        assert_eq!(response.answer, "tonic:client generate:1");
        assert_eq!(response.tokens[0].text, "tonic");
        assert_eq!(response.exported_kv_blocks.len(), 1);
        assert!(
            response
                .trace
                .iter()
                .any(|step| step.label == "tonic_proof_runtime")
        );
        assert!(response.trace.iter().any(|step| {
            step.label == "tonic_runtime_client_transport"
                && step.content.contains("transport_kind=tonic")
                && step.content.contains("runtime_id=runtime-a")
                && step.content.contains("model_id=tonic-proof-runtime")
                && step.content.contains("manifest_digest=sha256:manifest-a")
                && step.content.contains("status=ok")
                && step.content.contains("latency_us=")
        }));
        assert!(response.trace.iter().any(|step| {
            step.label == "tonic_runtime_server_transport"
                && step.content.contains("transport_kind=tonic")
                && step.content.contains("runtime_id=runtime-a")
                && step.content.contains("model_id=tonic-proof-runtime")
                && step.content.contains("status=ok")
                && step.content.contains("tonic_codegen_ready=true")
        }));
    }

    #[test]
    fn tonic_model_client_drives_runtime_backend_through_noiron_engine_infer() {
        let mut runtime = client_runtime("sha256:manifest-a");
        runtime.refresh_metadata().unwrap();
        let mut backend = RuntimeBackend::new(runtime).with_max_tokens(16);
        let mut engine = NoironEngine::new();
        let prompt = "engine tonic proof";

        let outcome = engine.infer(
            InferenceRequest::new(prompt, TaskProfile::Coding)
                .with_max_tokens(Some(16))
                .with_tenant_scope(TenantScope::local_single_user()),
            &mut backend,
        );

        assert!(backend.last_error().is_none());
        assert_eq!(outcome.raw_answer, "tonic:engine tonic proof:0");
        assert_eq!(outcome.recursive_runtime_calls, 1);
        assert_eq!(outcome.runtime_token_metrics.token_count, 1);
        assert_eq!(outcome.runtime_token_metrics.entropy_count, 1);
        assert_eq!(outcome.runtime_token_metrics.logprob_count, 1);
        assert_eq!(outcome.runtime_token_metrics.average_entropy, Some(0.4));
        assert_eq!(outcome.runtime_token_metrics.average_neg_logprob, Some(0.2));
        assert_eq!(outcome.runtime_diagnostics.hot_kv_precision_bits, Some(8));
        assert_eq!(outcome.runtime_diagnostics.cold_kv_precision_bits, Some(4));
    }

    #[test]
    fn tonic_model_client_imports_exports_kv_through_model_runtime() {
        let mut runtime = client_runtime("sha256:manifest-a");

        assert_eq!(runtime.import_kv(&[runtime_kv_block()]).unwrap(), 1);
        let exported = runtime.export_kv().unwrap();

        assert_eq!(exported.len(), 1);
        assert_eq!(exported[0].value, vec![0.3, 0.4]);
    }

    #[test]
    fn tonic_model_client_streams_tokens_through_model_runtime_observer() {
        let mut runtime = client_runtime("sha256:manifest-a");
        let mut seen = Vec::new();

        let response = runtime
            .generate_stream(runtime_request("client stream"), &mut |token| {
                seen.push(token.text.clone());
                Ok(())
            })
            .unwrap();

        assert_eq!(seen, vec!["alpha", "beta"]);
        assert_eq!(response.answer, "alphabeta");
        assert_eq!(response.tokens.len(), 2);
        assert!(
            response
                .trace
                .iter()
                .any(|step| step.label == "tonic_runtime_server_transport")
        );
        assert!(
            response
                .trace
                .iter()
                .any(|step| step.label == "tonic_runtime_client_transport")
        );
    }

    #[test]
    fn tonic_model_client_preserves_manifest_gate_errors() {
        let mut runtime = client_runtime("sha256:wrong");

        let error = runtime.generate(runtime_request("blocked")).unwrap_err();

        assert!(error.message().contains("manifest_digest mismatch"));
    }

    fn service() -> TonicRuntimeService<TonicProofRuntime> {
        TonicRuntimeService::with_manifest_digest(
            TonicProofRuntime::default(),
            "runtime-a",
            "sha256:manifest-a",
        )
        .unwrap()
    }

    fn client_runtime(
        manifest_digest: &str,
    ) -> TonicRuntimeModelClient<TonicRuntimeServer<TonicProofRuntime>> {
        let server = TonicRuntimeServer::new(service());
        let client = TonicRuntimeClient::new(server);
        TonicRuntimeModelClient::new(
            client,
            "runtime-a",
            manifest_digest,
            RuntimeMetadata::default(),
        )
        .unwrap()
    }

    fn envelope(method: InternalRuntimeMethod) -> proto::RuntimeEnvelope {
        proto::RuntimeEnvelope {
            runtime_id: "runtime-a".to_owned(),
            manifest_digest: "sha256:manifest-a".to_owned(),
            request_id: format!("request-{}", method.as_str()),
            trace_id: format!("trace-{}", method.as_str()),
            deadline_ms: 1_000,
            cancel_requested: false,
        }
    }

    fn runtime_request(prompt: &str) -> RuntimeRequest {
        RuntimeRequest {
            prompt: prompt.to_owned(),
            profile: TaskProfile::General,
            tenant_scope: None,
            runtime_metadata: RuntimeMetadata::new("client-runtime", "client-tokenizer", 2048, 2)
                .with_kv_exchange(true, true),
            runtime_architecture: TransformerRuntimeArchitecture::new(2, 2, 1, 1, 128),
            memory_hints: Vec::new(),
            infini_memory_hints: Vec::new(),
            experience_hints: Vec::new(),
            runtime_adapter_observations: Vec::new(),
            toolsmith_plan: ToolsmithPlan::default(),
            agent_team_plan: AgentTeamPlan::default(),
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 0,
                fast_tokens: 4,
                attention_fraction: 0.0,
            },
            hierarchy: HierarchyWeights::default(),
            transformer_plan: TransformerRefactorPlan::default(),
            recursive_schedule: RecursiveSchedule::default(),
            hardware_plan: HardwarePlan::default(),
            imported_kv_blocks: vec![runtime_kv_block()],
            max_tokens: 4,
        }
    }

    fn runtime_kv_block() -> RuntimeKvBlock {
        RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1, 0.2], vec![0.3, 0.4])
    }

    fn proto_kv_block() -> proto::RuntimeKvBlock {
        proto::RuntimeKvBlock {
            layer: 0,
            head: 0,
            token_start: 0,
            token_end: 1,
            key: vec![0.1, 0.2],
            value: vec![0.3, 0.4],
        }
    }
}
