pub mod proto {
    tonic::include_proto!("norion.runtime.v1");
}

use std::sync::Mutex;

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
    type GenerateStreamStream = tokio_stream::wrappers::UnboundedReceiverStream<
        Result<proto::RuntimeTokenEvent, tonic::Status>,
    >;

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
        let mut loopback = self.lock_loopback()?;
        let runtime_request = runtime_request_from_proto(&loopback, &request);
        let response = loopback
            .generate(&envelope, runtime_request)
            .map_err(runtime_error_to_status)?;
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
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let mut loopback = self.lock_loopback()?;
        let runtime_request = runtime_request_from_proto(&loopback, &request);
        loopback
            .generate_stream(&envelope, runtime_request, &mut |token| {
                tx.send(Ok(proto::RuntimeTokenEvent {
                    envelope: Some(proto_envelope.clone()),
                    token: Some(proto::RuntimeToken::from(token)),
                }))
                .map_err(|_| RuntimeError::new("tonic runtime transport client cancelled stream"))
            })
            .map_err(runtime_error_to_status)?;
        drop(tx);

        Ok(tonic::Response::new(
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx),
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

fn runtime_error_to_status(error: RuntimeError) -> tonic::Status {
    tonic::Status::failed_precondition(error.message().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(first.token.unwrap().text, "alpha");
        assert_eq!(second.token.unwrap().text, "beta");
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

    fn service() -> TonicRuntimeService<TonicProofRuntime> {
        TonicRuntimeService::with_manifest_digest(
            TonicProofRuntime::default(),
            "runtime-a",
            "sha256:manifest-a",
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
