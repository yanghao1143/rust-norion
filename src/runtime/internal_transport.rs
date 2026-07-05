use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::ReasoningStep;
use crate::runtime_manifest::TransformerRuntimeArchitecture;
use sha2::{Digest, Sha256};

use super::{
    ModelRuntime, RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeResponse, RuntimeToken,
};

pub const INTERNAL_RUNTIME_PROTO_PACKAGE: &str = "norion.runtime.v1";
pub const INTERNAL_RUNTIME_PROTO_SCHEMA: &str =
    include_str!("../../proto/norion/runtime/v1/runtime.proto");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternalRuntimeMethod {
    GetRuntimeMetadata,
    Generate,
    GenerateStream,
    ImportKv,
    ExportKv,
}

impl InternalRuntimeMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GetRuntimeMetadata => "GetRuntimeMetadata",
            Self::Generate => "Generate",
            Self::GenerateStream => "GenerateStream",
            Self::ImportKv => "ImportKv",
            Self::ExportKv => "ExportKv",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalRuntimeEnvelope {
    pub package: String,
    pub method: InternalRuntimeMethod,
    pub runtime_id: String,
    pub manifest_digest: String,
    pub request_id: String,
    pub trace_id: String,
    pub deadline_ms: Option<u64>,
    pub cancel_requested: bool,
}

impl InternalRuntimeEnvelope {
    pub fn new(
        method: InternalRuntimeMethod,
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self {
            package: INTERNAL_RUNTIME_PROTO_PACKAGE.to_owned(),
            method,
            runtime_id: runtime_id.into(),
            manifest_digest: manifest_digest.into(),
            request_id: request_id.into(),
            trace_id: trace_id.into(),
            deadline_ms: None,
            cancel_requested: false,
        }
    }

    pub fn metadata(
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self::new(
            InternalRuntimeMethod::GetRuntimeMetadata,
            runtime_id,
            manifest_digest,
            request_id,
            trace_id,
        )
    }

    pub fn generate(
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self::new(
            InternalRuntimeMethod::Generate,
            runtime_id,
            manifest_digest,
            request_id,
            trace_id,
        )
    }

    pub fn generate_stream(
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self::new(
            InternalRuntimeMethod::GenerateStream,
            runtime_id,
            manifest_digest,
            request_id,
            trace_id,
        )
    }

    pub fn import_kv(
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self::new(
            InternalRuntimeMethod::ImportKv,
            runtime_id,
            manifest_digest,
            request_id,
            trace_id,
        )
    }

    pub fn export_kv(
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self::new(
            InternalRuntimeMethod::ExportKv,
            runtime_id,
            manifest_digest,
            request_id,
            trace_id,
        )
    }

    pub fn with_deadline_ms(mut self, deadline_ms: u64) -> Self {
        self.deadline_ms = Some(deadline_ms);
        self
    }

    pub fn with_cancel_requested(mut self, cancel_requested: bool) -> Self {
        self.cancel_requested = cancel_requested;
        self
    }
}

#[derive(Debug, Clone)]
pub struct InternalRuntimeLoopback<R> {
    runtime: R,
    runtime_id: String,
    manifest_digest: String,
}

impl<R: ModelRuntime> InternalRuntimeLoopback<R> {
    pub fn new(runtime: R, runtime_id: impl Into<String>) -> Result<Self, RuntimeError> {
        let metadata = runtime.metadata();
        let architecture = runtime.architecture();
        let manifest_digest = runtime_transport_manifest_digest(&metadata, architecture);
        Self::with_manifest_digest(runtime, runtime_id, manifest_digest)
    }

    pub fn with_manifest_digest(
        runtime: R,
        runtime_id: impl Into<String>,
        manifest_digest: impl Into<String>,
    ) -> Result<Self, RuntimeError> {
        let runtime_id = runtime_id.into();
        let manifest_digest = manifest_digest.into();
        if !is_transport_id(&runtime_id) {
            return Err(RuntimeError::new(
                "internal runtime transport runtime_id is invalid",
            ));
        }
        if !manifest_digest.starts_with("sha256:") {
            return Err(RuntimeError::new(
                "internal runtime transport manifest_digest must be sha256",
            ));
        }
        Ok(Self {
            runtime,
            runtime_id,
            manifest_digest,
        })
    }

    pub fn runtime(&self) -> &R {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut R {
        &mut self.runtime
    }

    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    pub fn manifest_digest(&self) -> &str {
        &self.manifest_digest
    }

    pub fn metadata(
        &self,
        envelope: &InternalRuntimeEnvelope,
    ) -> Result<RuntimeMetadata, RuntimeError> {
        self.validate(envelope, InternalRuntimeMethod::GetRuntimeMetadata)?;
        Ok(self.runtime.metadata())
    }

    pub fn generate(
        &mut self,
        envelope: &InternalRuntimeEnvelope,
        request: RuntimeRequest,
    ) -> Result<RuntimeResponse, RuntimeError> {
        self.validate(envelope, InternalRuntimeMethod::Generate)?;
        let mut response = self.runtime.generate(request)?;
        response.trace.push(self.transport_trace(envelope));
        Ok(response)
    }

    pub fn generate_stream(
        &mut self,
        envelope: &InternalRuntimeEnvelope,
        request: RuntimeRequest,
        on_token: &mut dyn FnMut(&RuntimeToken) -> Result<(), RuntimeError>,
    ) -> Result<RuntimeResponse, RuntimeError> {
        self.validate(envelope, InternalRuntimeMethod::GenerateStream)?;
        let mut response = self.runtime.generate_stream(request, on_token)?;
        response.trace.push(self.transport_trace(envelope));
        Ok(response)
    }

    pub fn import_kv(
        &mut self,
        envelope: &InternalRuntimeEnvelope,
        blocks: &[RuntimeKvBlock],
    ) -> Result<usize, RuntimeError> {
        self.validate(envelope, InternalRuntimeMethod::ImportKv)?;
        self.runtime.import_kv(blocks)
    }

    pub fn export_kv(
        &mut self,
        envelope: &InternalRuntimeEnvelope,
    ) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        self.validate(envelope, InternalRuntimeMethod::ExportKv)?;
        self.runtime.export_kv()
    }

    fn validate(
        &self,
        envelope: &InternalRuntimeEnvelope,
        expected_method: InternalRuntimeMethod,
    ) -> Result<(), RuntimeError> {
        if envelope.package != INTERNAL_RUNTIME_PROTO_PACKAGE {
            return Err(RuntimeError::new(format!(
                "internal runtime transport package mismatch: expected {} got {}",
                INTERNAL_RUNTIME_PROTO_PACKAGE, envelope.package
            )));
        }
        if envelope.method != expected_method {
            return Err(RuntimeError::new(format!(
                "internal runtime transport method mismatch: expected {} got {}",
                expected_method.as_str(),
                envelope.method.as_str()
            )));
        }
        if envelope.runtime_id != self.runtime_id {
            return Err(RuntimeError::new(
                "internal runtime transport runtime_id mismatch before activation",
            ));
        }
        if envelope.manifest_digest != self.manifest_digest {
            return Err(RuntimeError::new(
                "internal runtime transport manifest_digest mismatch before activation",
            ));
        }
        if !is_transport_id(&envelope.request_id) || !is_transport_id(&envelope.trace_id) {
            return Err(RuntimeError::new(
                "internal runtime transport request_id or trace_id is invalid",
            ));
        }
        if envelope.deadline_ms == Some(0) {
            return Err(RuntimeError::new(
                "internal runtime transport deadline_ms must be positive",
            ));
        }
        if envelope.cancel_requested {
            return Err(RuntimeError::new(
                "internal runtime transport cancel_requested before activation",
            ));
        }
        Ok(())
    }

    fn transport_trace(&self, envelope: &InternalRuntimeEnvelope) -> ReasoningStep {
        ReasoningStep::new(
            "internal_runtime_transport",
            format!(
                "proto_package={} method={} runtime_id={} manifest_digest={} request_id={} trace_id={} deadline_ms={} proto_contract_ready=true loopback_ready=true tonic_codegen_ready=false http_edge_preserved=true",
                envelope.package,
                envelope.method.as_str(),
                self.runtime_id,
                self.manifest_digest,
                envelope.request_id,
                envelope.trace_id,
                envelope
                    .deadline_ms
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_owned())
            ),
            0.86,
        )
    }
}

pub fn runtime_transport_manifest_digest(
    metadata: &RuntimeMetadata,
    architecture: TransformerRuntimeArchitecture,
) -> String {
    sha256_text_digest(&format!(
        "package={};model_id={};tokenizer={};native_context_window={};embedding_dimensions={};kv_import={};kv_export={};max_kv_import_blocks={};max_kv_export_blocks={};kv_bits={}/{};{}",
        INTERNAL_RUNTIME_PROTO_PACKAGE,
        metadata.model_id,
        metadata.tokenizer,
        metadata.native_context_window,
        metadata.embedding_dimensions,
        metadata.supports_kv_import,
        metadata.supports_kv_export,
        metadata.max_kv_import_blocks,
        metadata.max_kv_export_blocks,
        metadata.hot_kv_precision_bits,
        metadata.cold_kv_precision_bits,
        architecture.summary()
    ))
}

pub fn runtime_transport_proto_digest() -> String {
    sha256_text_digest(INTERNAL_RUNTIME_PROTO_SCHEMA)
}

fn sha256_text_digest(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity("sha256:".len() + digest.len() * 2);
    out.push_str("sha256:");
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn is_transport_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_' | '.'))
}
