use super::LocalTransformerRuntime;
use super::forward::run_transformer_forward;
use super::tokenizer::{embed_tokens, local_tokenize, stable_hash};
use crate::engine::{InferenceRequest, NoironEngine};
use crate::hierarchy::TaskProfile;
use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime::{ModelRuntime, RuntimeBackend, RuntimeMetadata, RuntimeTokenId};
use crate::runtime_manifest::{RuntimeManifest, TransformerRuntimeArchitecture};
use request::runtime_request;

mod request;

#[test]
fn local_runtime_exposes_tokens_embeddings_and_metadata() {
    let runtime = LocalTransformerRuntime::new(16_384, 32);

    let metadata = runtime.metadata();
    let manifest = runtime.manifest();
    let tokens = runtime.tokenize("Rust Noiron local runtime").unwrap();
    let embedding = runtime.embed(&tokens).unwrap();

    assert_eq!(metadata.model_id, "noiron-local-transformer");
    assert_eq!(metadata.native_context_window, 16_384);
    assert_eq!(metadata.embedding_dimensions, 32);
    assert!(metadata.supports_kv_import);
    assert!(metadata.supports_kv_export);
    assert!(manifest.validate().passed());
    assert!(manifest.supports_device(crate::hardware::DeviceClass::CpuOnly));
    assert_eq!(tokens.len(), 4);
    assert_eq!(embedding.dimensions, 32);
    assert!(embedding.values.iter().any(|value| *value > 0.0));
}

#[test]
fn local_runtime_can_be_configured_from_manifest() {
    let manifest = RuntimeManifest::self_developed(
        "noiron-v2-transformer",
        "noiron-v2-tokenizer",
        131_072,
        48,
    )
    .with_architecture(TransformerRuntimeArchitecture::new(12, 48, 6, 3, 16_384))
    .with_kv_policy(crate::runtime_manifest::RuntimeKvPolicy {
        import_enabled: true,
        export_enabled: true,
        max_import_blocks: 1,
        max_export_blocks: 2,
    })
    .with_supported_devices(vec![
        crate::hardware::DeviceClass::CpuOnly,
        crate::hardware::DeviceClass::UnifiedMemory,
        crate::hardware::DeviceClass::Mobile,
    ]);
    let mut runtime = LocalTransformerRuntime::with_manifest(manifest);

    let imported = runtime
        .import_kv(&[
            RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2]),
            RuntimeKvBlock::new(1, 0, 1, 2, vec![0.3], vec![0.4]),
        ])
        .unwrap();
    let metadata = runtime.metadata();

    assert_eq!(imported, 1);
    assert_eq!(metadata.model_id, "noiron-v2-transformer");
    assert_eq!(metadata.native_context_window, 131_072);
    assert_eq!(runtime.manifest().architecture.layer_count, 12);
    assert_eq!(runtime.manifest().architecture.local_window_tokens, 16_384);
    assert!(
        runtime
            .manifest()
            .supports_device(crate::hardware::DeviceClass::Mobile)
    );
    assert!(
        !runtime
            .manifest()
            .supports_device(crate::hardware::DeviceClass::Server)
    );

    let request = runtime_request(
        "manifest configured runtime",
        TaskProfile::Coding,
        metadata,
        runtime.architecture(),
    );
    let response = runtime.generate(request).unwrap();
    let exported = runtime.export_kv().unwrap();

    assert!(response.answer.contains("manifest noiron-v2-transformer"));
    assert_eq!(
        response.diagnostics.model_id.as_deref(),
        Some("noiron-v2-transformer")
    );
    assert_eq!(response.diagnostics.layer_count, 12);
    assert_eq!(response.diagnostics.hidden_size, 48);
    assert_eq!(response.diagnostics.local_window_tokens, 16_384);
    assert!(response.diagnostics.forward_energy.unwrap() > 0.0);
    assert_eq!(response.diagnostics.hot_kv_precision_bits, Some(8));
    assert_eq!(response.diagnostics.cold_kv_precision_bits, Some(4));
    assert!(response.diagnostics.has_valid_kv_precision_signal());
    assert!(response.diagnostics.has_forward_signal());
    assert!(exported.len() <= 2);
}

#[test]
fn local_runtime_generates_tokens_and_exports_kv() {
    let mut runtime = LocalTransformerRuntime::default();
    let mut request = runtime_request(
        "Build local Noiron runtime",
        TaskProfile::Coding,
        runtime.metadata(),
        runtime.architecture(),
    );
    request.memory_hints = vec!["hot memory".to_owned()];
    request.experience_hints = vec!["reuse prior route".to_owned()];

    let response = runtime.generate(request).unwrap();
    let exported = runtime.export_kv().unwrap();

    assert!(response.answer.contains("Local Transformer runtime"));
    assert!(response.answer.contains("deterministic Transformer layers"));
    assert!(!response.tokens.is_empty());
    assert!(!exported.is_empty());
    assert!(
        response
            .trace
            .iter()
            .any(|step| step.label == "local_transformer_forward")
    );
}

#[test]
fn local_runtime_uses_observed_adapter_when_device_allows_it() {
    let mut runtime = LocalTransformerRuntime::default();
    let mut hardware_plan = crate::hardware::HardwarePlan::default();
    hardware_plan.execution.adapter_hints = vec![
        crate::hardware::RuntimeAdapterHint::CpuSimd,
        crate::hardware::RuntimeAdapterHint::PortableRust,
    ];
    let mut request = runtime_request(
        "Select observed local adapter",
        TaskProfile::Coding,
        runtime.metadata(),
        runtime.architecture(),
    );
    request.runtime_adapter_observations = vec![crate::runtime::RuntimeAdapterObservation::new(
        crate::hardware::RuntimeAdapterHint::CpuSimd,
        0.89,
        0.88,
        0.91,
        Some(0.18),
        Some(0.42),
        42,
    )];
    request.hardware_plan = hardware_plan;

    let response = runtime.generate(request).unwrap();

    assert_eq!(
        response.diagnostics.selected_adapter.as_deref(),
        Some("cpu-simd")
    );
}

#[test]
fn imported_kv_changes_local_forward_state() {
    let request = runtime_request(
        "Build local Noiron runtime",
        TaskProfile::Coding,
        RuntimeMetadata::default(),
        TransformerRuntimeArchitecture::new(6, 16, 4, 2, 128),
    );
    let tokens = local_tokenize(&request.prompt)
        .into_iter()
        .map(|text| RuntimeTokenId::new((stable_hash(&text) % 1_000_000) as u32, text))
        .collect::<Vec<_>>();
    let embedding = embed_tokens(&tokens, 16);
    let no_kv = run_transformer_forward(&embedding, &[], &request, request.runtime_architecture);
    let with_kv = run_transformer_forward(
        &embedding,
        &[RuntimeKvBlock::new(
            0,
            0,
            0,
            1,
            vec![1.0, 0.5, 0.25, 0.125],
            vec![0.75, 0.375, 0.1875, 0.09375],
        )],
        &request,
        request.runtime_architecture,
    );

    assert!(with_kv.kv_influence > no_kv.kv_influence);
    assert_ne!(with_kv.vector, no_kv.vector);
}

#[test]
fn engine_can_use_local_runtime_end_to_end() {
    let outcome = LocalTransformerRuntime::run_once(
        "Build a Rust Noiron local Transformer runtime with KV exchange",
        TaskProfile::Coding,
    );

    assert!(outcome.answer.contains("Local Transformer runtime"));
    assert!(outcome.exported_runtime_kv_blocks > 0);
    assert!(outcome.runtime_diagnostics.has_forward_signal());
    assert_eq!(
        outcome.runtime_diagnostics.model_id.as_deref(),
        Some("noiron-local-transformer")
    );
    assert!(!outcome.stored_runtime_kv_memory_ids.is_empty());
}

#[test]
fn engine_uses_local_runtime_embeddings_for_memory_vectors() {
    let mut engine = NoironEngine::new();
    let runtime = LocalTransformerRuntime::new(128, 24);
    let mut backend = RuntimeBackend::new(runtime);

    let outcome = engine.infer(
        InferenceRequest::new(
            "Store a reusable Noiron memory using model-side embeddings",
            TaskProfile::Coding,
        ),
        &mut backend,
    );

    assert!(outcome.stored_memory_id.is_some());
    assert_eq!(
        outcome.embedding_diagnostics.query.source,
        crate::engine::EmbeddingSource::Runtime
    );
    assert_eq!(outcome.embedding_diagnostics.query.dimensions, 24);
    assert!(outcome.embedding_diagnostics.runtime_embedding_available());
    assert!(!outcome.embedding_diagnostics.fallback_embedding_used());
    assert!(
        engine
            .cache
            .entries()
            .iter()
            .any(|entry| entry.vector.len() == 24)
    );
}
