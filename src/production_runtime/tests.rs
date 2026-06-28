use super::*;
use crate::hardware::{
    DeviceClass, HardwareAllocator, HardwarePlan, HardwareSnapshot, RuntimeAdapterHint,
};
use crate::hierarchy::HierarchyWeights;
use crate::kv_exchange::RuntimeKvBlock;
use crate::local_runtime::LocalTransformerRuntime;
use crate::reflection::{ReasoningStep, RuntimeDiagnostics};
use crate::runtime::{ModelRuntime, RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeToken};
use crate::runtime_manifest::{
    RuntimeAdapterLifecycleRecord, RuntimeAdapterLifecycleState, RuntimeAssetPaths,
    RuntimeKvPolicy, RuntimeManifest, TransformerRuntimeArchitecture,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn production_runtime_accepts_manifest_assets_and_device_contract() {
    let (asset_dir, weights, tokenizer, config) = create_assets("production-runtime-assets");
    let manifest = production_manifest(&weights, &tokenizer)
        .with_assets(
            RuntimeAssetPaths::new()
                .with_weights(&weights)
                .with_tokenizer(&tokenizer)
                .with_config(&config),
        )
        .with_adapter_hints(vec![
            RuntimeAdapterHint::PortableRust,
            RuntimeAdapterHint::CpuSimd,
        ]);
    let plan = cpu_plan();

    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &plan).unwrap();

    assert_eq!(runtime.metadata().model_id, "noiron-production-transformer");
    assert_eq!(runtime.architecture().layer_count, 6);
    assert!(runtime.device_gate().passed());
    assert_eq!(runtime.assets().weights_bytes, 7);
    assert_eq!(runtime.assets().tokenizer_bytes, 9);
    assert_eq!(runtime.assets().config_bytes, Some(6));
    assert!(runtime.runtime_device_contract().contains("device=cpu"));
    assert!(runtime.selected_adapter().is_some());
    assert!(runtime.summary_line().contains("kernel=not-connected"));
    assert!(!runtime.kernel_connected());

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_exposes_bootstrap_tokens_and_embeddings() {
    let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-embed");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime =
        ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap();

    let tokens = runtime.tokenize("Rust Noiron runtime").unwrap();
    let embedding = runtime.embed(&tokens).unwrap();

    assert_eq!(tokens.len(), 3);
    assert_eq!(embedding.dimensions, 64);
    assert!(embedding.values.iter().any(|value| *value > 0.0));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_rejects_missing_assets() {
    let manifest = RuntimeManifest::self_developed("missing-production", "tokenizer", 4096, 64)
        .with_architecture(TransformerRuntimeArchitecture::new(6, 64, 4, 2, 1024));

    let error =
        ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap_err();

    assert!(error.message().contains("weights asset path is required"));
    assert!(error.message().contains("tokenizer asset path is required"));
}

#[test]
fn production_runtime_rejects_device_adapter_mismatch() {
    let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-mismatch");
    let manifest = production_manifest(&weights, &tokenizer)
        .with_adapter_hints(vec![RuntimeAdapterHint::Cuda]);

    let error =
        ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap_err();

    assert!(error.message().contains("device gate rejected"));
    assert!(error.message().contains("no adapter intersection"));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_rejects_retired_runtime_adapter() {
    let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-retired");
    let manifest = production_manifest(&weights, &tokenizer)
        .with_retired_adapter_hints(vec![RuntimeAdapterHint::PortableRust]);

    let error =
        ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap_err();

    assert!(error.message().contains("device gate rejected"));
    assert!(error.message().contains("retired_blocked"));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_rejects_quarantined_runtime_adapter() {
    let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-quarantined");
    let manifest = production_manifest(&weights, &tokenizer).with_adapter_lifecycle_records(vec![
        RuntimeAdapterLifecycleRecord::new(
            RuntimeAdapterHint::PortableRust,
            RuntimeAdapterLifecycleState::Quarantined,
            "polluted_runtime_source",
            "sha256:portable-runtime-source",
            "lineage:runtime:portable",
            "rollback:adapter:portable",
            "scope:local-runtime",
        ),
    ]);

    let error =
        ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap_err();

    assert!(error.message().contains("device gate rejected"));
    assert!(error.message().contains("state=quarantined"));
    assert!(
        error
            .message()
            .contains("source_digest=sha256:portable-runtime-source")
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_imports_kv_with_manifest_and_device_limits() {
    let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-import-kv");
    let manifest = production_manifest(&weights, &tokenizer).with_kv_policy(RuntimeKvPolicy {
        import_enabled: true,
        export_enabled: true,
        max_import_blocks: 1,
        max_export_blocks: 2,
    });
    let mut plan = cpu_plan();
    plan.execution.kv_prefetch_blocks = 1;
    let mut runtime =
        ProductionTransformerRuntime::from_manifest_for_plan(manifest, &plan).unwrap();
    let blocks = vec![
        RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2]),
        RuntimeKvBlock::new(0, 1, 1, 2, vec![0.3], vec![0.4]),
    ];

    let imported = runtime.import_kv(&blocks).unwrap();
    let exported = runtime.export_kv().unwrap();

    assert_eq!(imported, 1);
    assert_eq!(runtime.imported_kv_blocks().len(), 1);
    assert!(exported.is_empty());

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_rejects_invalid_imported_kv() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-invalid-import-kv");
    let manifest = production_manifest(&weights, &tokenizer);
    let mut runtime =
        ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap();

    runtime
        .import_kv(&[RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2])])
        .unwrap();
    assert_eq!(runtime.imported_kv_blocks().len(), 1);

    let cases = vec![
        (
            "layer",
            RuntimeKvBlock::new(6, 0, 0, 1, vec![0.1], vec![0.2]),
            "layer 6 exceeds manifest layer_count 6",
        ),
        (
            "head",
            RuntimeKvBlock::new(0, 2, 0, 1, vec![0.1], vec![0.2]),
            "head 2 exceeds manifest kv_heads 2",
        ),
        (
            "range",
            RuntimeKvBlock::new(0, 0, 2, 2, vec![0.1], vec![0.2]),
            "token range 2..2 is empty or reversed",
        ),
        (
            "dimension",
            RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2, 0.3]),
            "key/value dimensions differ",
        ),
        (
            "finite",
            RuntimeKvBlock::new(0, 0, 0, 1, vec![f32::NAN], vec![0.2]),
            "imported key contains non-finite value",
        ),
    ];

    for (label, block, expected) in cases {
        let mut invalid_runtime = runtime.clone();

        let error = invalid_runtime.import_kv(&[block]).unwrap_err();

        assert!(
            error.message().contains("invalid imported KV block 0"),
            "{label}: {}",
            error.message()
        );
        assert!(
            error.message().contains(expected),
            "{label}: {}",
            error.message()
        );
        assert!(invalid_runtime.imported_kv_blocks().is_empty(), "{label}");
    }

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_generate_errors_until_forward_kernel_is_connected() {
    let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-generate");
    let manifest = production_manifest(&weights, &tokenizer);
    let mut runtime =
        ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap();

    let error = runtime.generate(sample_request()).unwrap_err();

    assert!(error.message().contains("kernel is not connected"));
    assert!(error.message().contains("noiron-production-transformer"));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_can_generate_through_attached_forward_kernel() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-attached-kernel");
    let manifest = production_manifest(&weights, &tokenizer);
    let mut runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(MockForwardKernel);
    let blocks = vec![RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2])];
    runtime.import_kv(&blocks).unwrap();

    let response = runtime.generate(sample_request()).unwrap();
    let exported = runtime.export_kv().unwrap();

    assert!(runtime.kernel_connected());
    assert!(runtime.summary_line().contains("kernel=connected"));
    assert!(response.answer.contains("kernel answer"));
    assert_eq!(response.tokens.len(), 1);
    assert_eq!(response.trace[0].label, "production_kernel");
    assert_eq!(
        response.diagnostics.model_id.as_deref(),
        Some("noiron-production-transformer")
    );
    assert_eq!(
        response.diagnostics.selected_adapter.as_deref(),
        Some("portable-rust")
    );
    assert_eq!(response.diagnostics.imported_kv_blocks, 1);
    assert_eq!(response.diagnostics.exported_kv_blocks, 1);
    assert_eq!(response.diagnostics.runtime_kv_segments_included, 1);
    assert_eq!(response.diagnostics.runtime_kv_segments_skipped, 0);
    assert_eq!(response.diagnostics.runtime_kv_segments_rejected, 0);
    assert!(response.diagnostics.has_runtime_kv_segment_signal());
    assert_eq!(exported.len(), 1);
    assert_eq!(runtime.exported_kv_blocks().len(), 1);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_rejects_request_kv_precision_above_device_contract() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-request-kv-abi");
    let manifest = production_manifest(&weights, &tokenizer)
        .with_supported_devices(vec![DeviceClass::Microcontroller])
        .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust]);
    let mut request = sample_request();
    request.runtime_metadata = manifest.runtime_metadata();
    request.runtime_architecture = manifest.architecture;
    request.hardware_plan = device_plan(DeviceClass::Microcontroller);
    request.hardware_plan.execution.hot_kv_precision_bits = 8;
    request.hardware_plan.execution.cold_kv_precision_bits = 4;
    let mut runtime = ProductionTransformerRuntime::from_manifest_for_plan(
        manifest,
        &device_plan(DeviceClass::Microcontroller),
    )
    .unwrap()
    .with_kernel(MockForwardKernel);

    let error = runtime.generate(request).unwrap_err();

    assert!(
        error
            .message()
            .contains("production runtime request rejected")
    );
    assert!(
        error
            .message()
            .contains("request device hot KV precision 8 exceeds")
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn reference_production_kernel_generates_diagnostics_and_kv() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-reference-kernel");
    let manifest = production_manifest(&weights, &tokenizer);
    let mut runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(ReferenceProductionForwardKernel::new());
    runtime
        .import_kv(&[RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2])])
        .unwrap();

    let response = runtime.generate(sample_request()).unwrap();
    let exported = runtime.export_kv().unwrap();

    assert!(
        response
            .answer
            .contains("Reference production Transformer kernel result")
    );
    assert!(!response.tokens.is_empty());
    assert!(
        response
            .trace
            .iter()
            .any(|step| step.label == "reference_production_kernel")
    );
    assert_eq!(
        response.diagnostics.model_id.as_deref(),
        Some("noiron-production-transformer")
    );
    assert_eq!(
        response.diagnostics.selected_adapter.as_deref(),
        Some("portable-rust")
    );
    assert_eq!(response.diagnostics.layer_count, 6);
    assert!(response.diagnostics.forward_energy.unwrap() > 0.0);
    assert!(response.diagnostics.kv_influence.unwrap() > 0.0);
    assert_eq!(response.diagnostics.hot_kv_precision_bits, Some(8));
    assert_eq!(response.diagnostics.cold_kv_precision_bits, Some(4));
    assert!(response.diagnostics.has_valid_kv_precision_signal());
    assert_eq!(response.diagnostics.imported_kv_blocks, 1);
    assert_eq!(response.diagnostics.runtime_kv_segments_included, 1);
    assert_eq!(response.diagnostics.runtime_kv_segments_skipped, 0);
    assert_eq!(response.diagnostics.runtime_kv_segments_rejected, 0);
    assert!(response.diagnostics.has_runtime_kv_segment_signal());
    assert!(!exported.is_empty());
    assert!(exported.iter().all(|block| block.layer < 6));
    assert!(exported.iter().all(|block| block.head < 2));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn reference_production_kernel_passes_conformance_gate() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-reference");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(ReferenceProductionForwardKernel::new());

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(report.passed, "{report:?}");
    assert!(report.kernel_connected);
    assert!(report.token_count > 0);
    assert_eq!(report.uncertainty_token_count, report.token_count);
    assert!(report.average_entropy.unwrap() > 0.0);
    assert!(report.average_neg_logprob.unwrap() > 0.0);
    assert!(report.uncertainty_perplexity.unwrap() > 2.0);
    assert!(report.trace_steps > 0);
    assert!(report.imported_kv_blocks > 0);
    assert_eq!(report.weak_runtime_kv_imports_skipped, 0);
    assert_eq!(report.runtime_kv_weak_import_pressure, None);
    assert_eq!(report.budget_limited_runtime_kv_imports_skipped, 0);
    assert_eq!(report.runtime_kv_budget_pressure, None);
    assert!(report.exported_kv_blocks > 0);
    assert!(report.runtime_kv_segments_included > 0);
    assert_eq!(report.runtime_kv_segments_skipped, 0);
    assert_eq!(report.runtime_kv_segments_rejected, 0);
    assert!(report.runtime_kv_segment_count() > 0);
    assert!(report.forward_energy.unwrap() > 0.0);
    assert!(report.kv_influence.unwrap() >= 0.0);
    assert_eq!(report.manifest_hot_kv_bits, 8);
    assert_eq!(report.manifest_cold_kv_bits, 4);
    assert_eq!(report.device_hot_kv_bits, 8);
    assert_eq!(report.device_cold_kv_bits, 4);
    assert_eq!(report.request_runtime_hot_kv_bits, 8);
    assert_eq!(report.request_runtime_cold_kv_bits, 4);
    assert_eq!(report.request_device_hot_kv_bits, 8);
    assert_eq!(report.request_device_cold_kv_bits, 4);
    assert!(report.summary_line().contains("passed=true"));
    assert!(report.summary_line().contains("manifest_kv_bits=8/4"));
    assert!(report.summary_line().contains("request_device_kv_bits=8/4"));
    assert!(report.summary_line().contains("uncertainty_tokens="));
    assert!(report.summary_line().contains("uncertainty_perplexity="));
    assert!(
        report
            .summary_line()
            .contains("weak_runtime_kv_imports_skipped=0")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_weak_import_pressure=none")
    );
    assert!(
        report
            .summary_line()
            .contains("budget_limited_runtime_kv_imports_skipped=0")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_budget_pressure=none")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_segments_included=")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_segments_rejected=0")
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn reference_production_kernel_conformance_allows_q4_q4_tiny_device_contract() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-q4-tiny");
    let manifest = production_manifest(&weights, &tokenizer)
        .with_supported_devices(vec![DeviceClass::Microcontroller])
        .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust])
        .with_quantization(crate::runtime_manifest::RuntimeQuantizationPolicy {
            hot_kv: crate::kv_quant::QuantizationBits::Four,
            cold_kv: crate::kv_quant::QuantizationBits::Four,
            weights: None,
        });
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(
        manifest,
        &device_plan(DeviceClass::Microcontroller),
    )
    .unwrap()
    .with_kernel(ReferenceProductionForwardKernel::new());

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(report.passed, "{report:?}");
    assert_eq!(report.manifest_hot_kv_bits, 4);
    assert_eq!(report.manifest_cold_kv_bits, 4);
    assert_eq!(report.device_hot_kv_bits, 4);
    assert_eq!(report.device_cold_kv_bits, 4);
    assert_eq!(report.request_runtime_hot_kv_bits, 4);
    assert_eq!(report.request_runtime_cold_kv_bits, 4);
    assert_eq!(report.request_device_hot_kv_bits, 4);
    assert_eq!(report.request_device_cold_kv_bits, 4);
    assert!(report.summary_line().contains("manifest_kv_bits=4/4"));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_runtime_forward_kernel_wraps_local_runtime_for_production_boundary() {
    let (asset_dir, weights, tokenizer, _config) = create_assets("production-runtime-local-kernel");
    let manifest = production_manifest(&weights, &tokenizer);
    let local_runtime = LocalTransformerRuntime::with_manifest(manifest.clone());
    let mut runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(ModelRuntimeForwardKernel::new(local_runtime));
    runtime
        .import_kv(&[RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2])])
        .unwrap();

    let response = runtime.generate(sample_request()).unwrap();
    let exported = runtime.export_kv().unwrap();

    assert!(response.answer.contains("Local Transformer runtime result"));
    assert_eq!(
        response.diagnostics.model_id.as_deref(),
        Some("noiron-production-transformer")
    );
    assert_eq!(
        response.diagnostics.selected_adapter.as_deref(),
        Some("portable-rust")
    );
    assert_eq!(response.diagnostics.layer_count, 6);
    assert_eq!(response.diagnostics.hidden_size, 64);
    assert_eq!(response.diagnostics.imported_kv_blocks, 1);
    assert_eq!(response.diagnostics.runtime_kv_segments_included, 1);
    assert_eq!(response.diagnostics.runtime_kv_segments_skipped, 0);
    assert_eq!(response.diagnostics.runtime_kv_segments_rejected, 0);
    assert!(response.diagnostics.has_runtime_kv_segment_signal());
    assert_eq!(response.diagnostics.hot_kv_precision_bits, Some(8));
    assert_eq!(response.diagnostics.cold_kv_precision_bits, Some(4));
    assert!(response.diagnostics.has_valid_kv_precision_signal());
    assert!(!response.tokens.is_empty());
    assert!(!response.trace.is_empty());
    assert!(!exported.is_empty());
    assert!(exported.iter().all(|block| block.layer < 6));
    assert!(exported.iter().all(|block| block.head < 2));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn local_model_runtime_kernel_passes_conformance_gate() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-local-kernel-conformance");
    let manifest = production_manifest(&weights, &tokenizer);
    let local_runtime = LocalTransformerRuntime::with_manifest(manifest.clone());
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(ModelRuntimeForwardKernel::new(local_runtime));

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(report.passed, "{report:?}");
    assert_eq!(report.model_id, "noiron-production-transformer");
    assert_eq!(report.selected_adapter, "portable-rust");
    assert!(report.imported_kv_blocks > 0);
    assert!(report.exported_kv_blocks > 0);
    assert!(report.runtime_kv_segments_included > 0);
    assert_eq!(report.runtime_kv_segments_rejected, 0);
    assert!(report.forward_energy.unwrap() > 0.0);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn reference_production_kernel_conformance_matrix_covers_all_devices() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-reference-matrix");
    let manifest = production_manifest(&weights, &tokenizer)
        .with_supported_devices(DeviceClass::explicit_profiles().to_vec());
    let device_reports = DeviceClass::explicit_profiles()
        .iter()
        .copied()
        .map(|device| {
            let runtime = ProductionTransformerRuntime::from_manifest_for_plan(
                manifest.clone(),
                &device_plan(device),
            )
            .unwrap()
            .with_kernel(ReferenceProductionForwardKernel::new());
            ProductionKernelConformanceDeviceReport {
                device,
                report: runtime.conformance_report(ProductionKernelConformanceGate::default()),
            }
        })
        .collect();

    let report = ProductionKernelConformanceMatrixReport::evaluate(device_reports);

    assert!(report.passed, "{report:?}");
    assert_eq!(
        report.covered_devices(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(report.missing_devices().is_empty());
    assert!(report.failed_devices().is_empty());
    assert!(report.summary_line().contains("devices=12"));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn local_model_runtime_kernel_conformance_matrix_covers_all_devices() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-local-matrix");
    let manifest = production_manifest(&weights, &tokenizer)
        .with_supported_devices(DeviceClass::explicit_profiles().to_vec());
    let device_reports = DeviceClass::explicit_profiles()
        .iter()
        .copied()
        .map(|device| {
            let local_runtime = LocalTransformerRuntime::with_manifest(manifest.clone());
            let runtime = ProductionTransformerRuntime::from_manifest_for_plan(
                manifest.clone(),
                &device_plan(device),
            )
            .unwrap()
            .with_kernel(ModelRuntimeForwardKernel::new(local_runtime));
            ProductionKernelConformanceDeviceReport {
                device,
                report: runtime.conformance_report(ProductionKernelConformanceGate::default()),
            }
        })
        .collect();

    let report = ProductionKernelConformanceMatrixReport::evaluate(device_reports);

    assert!(report.passed, "{report:?}");
    assert_eq!(
        report.covered_devices(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(report.failed_devices().is_empty());

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_matrix_reports_missing_and_failed_devices() {
    let passing = ProductionKernelConformanceReport {
        passed: true,
        model_id: "model".to_owned(),
        selected_adapter: "portable-rust".to_owned(),
        kernel_connected: true,
        manifest_hot_kv_bits: 8,
        manifest_cold_kv_bits: 4,
        device_hot_kv_bits: 8,
        device_cold_kv_bits: 4,
        request_runtime_hot_kv_bits: 8,
        request_runtime_cold_kv_bits: 4,
        request_device_hot_kv_bits: 8,
        request_device_cold_kv_bits: 4,
        token_count: 1,
        uncertainty_token_count: 1,
        average_entropy: Some(0.3),
        average_neg_logprob: Some(0.2),
        uncertainty_perplexity: Some(3.4),
        trace_steps: 1,
        imported_kv_blocks: 1,
        weak_runtime_kv_imports_skipped: 0,
        runtime_kv_weak_import_pressure: None,
        budget_limited_runtime_kv_imports_skipped: 0,
        runtime_kv_budget_pressure: None,
        exported_kv_blocks: 1,
        runtime_kv_segments_included: 1,
        runtime_kv_segments_skipped: 0,
        runtime_kv_segments_rejected: 0,
        adapter_stream_read_only: None,
        adapter_stream_write_allowed: None,
        adapter_stream_applied: None,
        forward_energy: Some(1.0),
        kv_influence: Some(0.0),
        global_layers: 1,
        local_window_layers: 1,
        convolutional_fusion_layers: 1,
        failures: Vec::new(),
    };
    let failing = ProductionKernelConformanceReport::failed(
        "model",
        "portable-rust",
        false,
        "production forward kernel is not connected",
    );

    let report = ProductionKernelConformanceMatrixReport::evaluate(vec![
        ProductionKernelConformanceDeviceReport {
            device: DeviceClass::CpuOnly,
            report: passing,
        },
        ProductionKernelConformanceDeviceReport {
            device: DeviceClass::IntegratedGpu,
            report: failing,
        },
    ]);

    assert!(!report.passed);
    assert_eq!(report.covered_devices(), 2);
    assert_eq!(
        report.missing_devices().len(),
        DeviceClass::explicit_profiles().len() - 2
    );
    assert_eq!(report.failed_devices(), vec![DeviceClass::IntegratedGpu]);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("production_kernel_conformance_devices 2 below expected")
    }));
    assert!(report.failures.iter().any(|failure| {
        failure.contains("device integrated production kernel conformance failed")
    }));
}

#[test]
fn production_kernel_conformance_gate_fails_when_kernel_is_missing() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-missing");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime =
        ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan()).unwrap();

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(!report.passed);
    assert!(!report.kernel_connected);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("production forward kernel is not connected"))
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_fails_tokens_without_uncertainty_signal() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-no-uncertainty");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(NoUncertaintyForwardKernel);

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(!report.passed);
    assert_eq!(report.token_count, 1);
    assert_eq!(report.uncertainty_token_count, 0);
    assert_eq!(report.average_entropy, None);
    assert_eq!(report.average_neg_logprob, None);
    assert_eq!(report.uncertainty_perplexity, None);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel did not return any runtime token entropy/logprob")
    }));
    assert!(report.summary_line().contains("uncertainty_tokens=0"));
    assert!(
        report
            .summary_line()
            .contains("uncertainty_perplexity=none")
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_fails_imported_kv_without_segment_signal() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-no-segment-signal");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(MockForwardKernel);

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(!report.passed);
    assert!(report.imported_kv_blocks > 0);
    assert!(report.exported_kv_blocks > 0);
    assert!(report.runtime_kv_segments_included > 0);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime KV import is enabled but kernel reported no KV segment signal")
    }));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_fails_runtime_kv_import_skips() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-kv-skips");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(KvImportSkipForwardKernel);

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(!report.passed);
    assert_eq!(report.imported_kv_blocks, 1);
    assert_eq!(report.weak_runtime_kv_imports_skipped, 2);
    assert!((report.runtime_kv_weak_import_pressure.unwrap() - 0.666).abs() < 0.001);
    assert_eq!(report.budget_limited_runtime_kv_imports_skipped, 3);
    assert!((report.runtime_kv_budget_pressure.unwrap() - 0.75).abs() < 0.001);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel skipped 2 weak runtime KV imports during conformance")
    }));
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel skipped 3 runtime KV imports due to budget during conformance")
    }));
    assert!(
        report
            .summary_line()
            .contains("weak_runtime_kv_imports_skipped=2")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_weak_import_pressure=0.667")
    );
    assert!(
        report
            .summary_line()
            .contains("budget_limited_runtime_kv_imports_skipped=3")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_budget_pressure=0.750")
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_fails_adapter_stream_apply() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-adapter-apply");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(AdapterStreamApplyForwardKernel);

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(!report.passed);
    assert_eq!(report.adapter_stream_read_only, Some(false));
    assert_eq!(report.adapter_stream_write_allowed, Some(true));
    assert_eq!(report.adapter_stream_applied, Some(true));
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel adapter stream was not preview-only during conformance")
    }));
    assert!(
        report
            .summary_line()
            .contains("adapter_stream_write_allowed=true")
    );
    assert!(
        report
            .summary_line()
            .contains("adapter_stream_applied=true")
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_fails_adapter_stream_without_write_gate() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-adapter-missing-write-gate");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(AdapterStreamMissingWriteGateForwardKernel);

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(!report.passed);
    assert_eq!(report.adapter_stream_read_only, None);
    assert_eq!(report.adapter_stream_write_allowed, None);
    assert_eq!(report.adapter_stream_applied, None);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel reported adapter stream without write gate state")
    }));
    assert!(
        report
            .summary_line()
            .contains("adapter_stream_write_allowed=none")
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_fails_adapter_stream_gate_summary_without_write_gate() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-adapter-gate-summary-missing-write-gate");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(AdapterStreamGateSummaryMissingWriteGateForwardKernel);

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(!report.passed);
    assert_eq!(report.adapter_stream_read_only, None);
    assert_eq!(report.adapter_stream_write_allowed, None);
    assert_eq!(report.adapter_stream_applied, None);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel reported adapter stream without write gate state")
    }));
    assert!(
        report
            .summary_line()
            .contains("adapter_stream_write_allowed=none")
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_fails_malformed_adapter_stream_gate_summary() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-adapter-malformed-gate-summary");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(AdapterStreamMalformedGateSummaryForwardKernel);

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(!report.passed);
    assert_eq!(report.adapter_stream_read_only, Some(true));
    assert_eq!(report.adapter_stream_write_allowed, Some(false));
    assert_eq!(report.adapter_stream_applied, Some(false));
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel reported malformed adapter stream gate summary digest")
    }));

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_fails_partial_adapter_stream_write_gate() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-adapter-partial-write-gate");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(AdapterStreamPartialWriteGateForwardKernel);

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(!report.passed);
    assert_eq!(report.adapter_stream_read_only, Some(true));
    assert_eq!(report.adapter_stream_write_allowed, None);
    assert_eq!(report.adapter_stream_applied, Some(false));
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel reported partial adapter stream write gate state")
    }));
    assert!(
        report
            .summary_line()
            .contains("adapter_stream_write_allowed=none")
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_kernel_conformance_gate_fails_malformed_kernel_output() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-conformance-malformed");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(MalformedForwardKernel);

    let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

    assert!(!report.passed);
    assert_eq!(report.token_count, 0);
    assert_eq!(report.trace_steps, 0);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel did not return runtime token uncertainty records")
    }));
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("kernel did not return reasoning trace steps"))
    );
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel did not report positive finite forward_energy")
    }));
    assert!(report.failures.iter().any(|failure| {
        failure.contains("kernel did not report finite non-negative kv_influence")
    }));
    assert!(
        report.failures.iter().any(|failure| {
            failure.contains("kernel did not cover all Transformer layer modes")
        })
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("kernel exported no KV blocks"))
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_runtime_rejects_invalid_kernel_exported_kv() {
    let (asset_dir, weights, tokenizer, _config) =
        create_assets("production-runtime-invalid-kernel-kv");
    let manifest = production_manifest(&weights, &tokenizer);
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &cpu_plan())
        .unwrap()
        .with_kernel(MockForwardKernel);
    let mut runtime = runtime;

    runtime.generate(sample_request()).unwrap();
    assert_eq!(runtime.exported_kv_blocks().len(), 1);

    let cases = vec![
        (
            "layer",
            RuntimeKvBlock::new(6, 0, 0, 1, vec![0.1], vec![0.2]),
            "layer 6 exceeds manifest layer_count 6",
        ),
        (
            "head",
            RuntimeKvBlock::new(0, 2, 0, 1, vec![0.1], vec![0.2]),
            "head 2 exceeds manifest kv_heads 2",
        ),
        (
            "range",
            RuntimeKvBlock::new(0, 0, 2, 2, vec![0.1], vec![0.2]),
            "token range 2..2 is empty or reversed",
        ),
        (
            "dimension",
            RuntimeKvBlock::new(0, 0, 0, 1, vec![0.1], vec![0.2, 0.3]),
            "key/value dimensions differ",
        ),
        (
            "finite",
            RuntimeKvBlock::new(0, 0, 0, 1, vec![f32::NAN], vec![0.2]),
            "key contains non-finite value",
        ),
    ];

    for (label, block, expected) in cases {
        let mut invalid_runtime = runtime.clone().with_kernel(InvalidExportKernel { block });

        let error = invalid_runtime.generate(sample_request()).unwrap_err();

        assert!(
            error.message().contains("invalid exported KV block 0"),
            "{label}: {}",
            error.message()
        );
        assert!(
            error.message().contains(expected),
            "{label}: {}",
            error.message()
        );
        assert!(invalid_runtime.exported_kv_blocks().is_empty(), "{label}");
        assert!(invalid_runtime.export_kv().unwrap().is_empty(), "{label}");
    }

    fs::remove_dir_all(asset_dir).unwrap();
}

fn production_manifest(weights: &Path, tokenizer: &Path) -> RuntimeManifest {
    RuntimeManifest::self_developed(
        "noiron-production-transformer",
        "noiron-production-tokenizer",
        4096,
        64,
    )
    .with_architecture(TransformerRuntimeArchitecture::new(6, 64, 4, 2, 1024))
    .with_supported_devices(vec![DeviceClass::CpuOnly])
    .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust])
    .with_assets(
        RuntimeAssetPaths::new()
            .with_weights(weights)
            .with_tokenizer(tokenizer),
    )
}

fn cpu_plan() -> HardwarePlan {
    device_plan(DeviceClass::CpuOnly)
}

fn device_plan(device: DeviceClass) -> HardwarePlan {
    HardwareAllocator::new().plan(
        HardwareSnapshot::new(device, 0.20, 0.10, 0.30, 0.10),
        crate::hierarchy::TaskProfile::General,
        512,
        HierarchyWeights::default(),
    )
}

fn sample_request() -> RuntimeRequest {
    RuntimeRequest {
        prompt: "connect production runtime".to_owned(),
        profile: crate::hierarchy::TaskProfile::General,
        runtime_metadata: RuntimeMetadata::new(
            "noiron-production-transformer",
            "noiron-production-tokenizer",
            4096,
            64,
        ),
        runtime_architecture: TransformerRuntimeArchitecture::new(6, 64, 4, 2, 1024),
        memory_hints: Vec::new(),
        infini_memory_hints: Vec::new(),
        experience_hints: Vec::new(),
        runtime_adapter_observations: Vec::new(),
        toolsmith_plan: crate::toolsmith::ToolsmithPlan::default(),
        agent_team_plan: crate::agent_team::AgentTeamPlan::default(),
        route_budget: crate::router::RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::default(),
        transformer_plan: crate::transformer::TransformerRefactorPlan::default(),
        recursive_schedule: crate::recursive_scheduler::RecursiveSchedule::default(),
        hardware_plan: cpu_plan(),
        imported_kv_blocks: Vec::new(),
        max_tokens: 32,
    }
}

fn create_assets(name: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let asset_dir = temp_asset_dir(name);
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    let config = asset_dir.join("config.noiron");
    write_asset(&weights, b"weights");
    write_asset(&tokenizer, b"tokenizer");
    write_asset(&config, b"config");
    (asset_dir, weights, tokenizer, config)
}

fn write_asset(path: &Path, bytes: &[u8]) {
    let mut file = File::create(path).unwrap();
    file.write_all(bytes).unwrap();
}

fn temp_asset_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{name}-{unique}"))
}

#[derive(Debug, Clone)]
struct MockForwardKernel;

impl ProductionForwardKernel for MockForwardKernel {
    fn generate(
        &self,
        context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(ProductionKernelOutput::new(format!(
            "kernel answer for {} with {} imported KV blocks",
            context.manifest.metadata.model_id,
            context.imported_kv_blocks.len()
        ))
        .with_tokens(vec![RuntimeToken {
            text: "kernel".to_owned(),
            logprob: Some(-0.2),
            entropy: Some(0.3),
        }])
        .with_trace(vec![ReasoningStep::new(
            "production_kernel",
            context.device_gate.runtime_device_contract.clone(),
            0.88,
        )])
        .with_diagnostics(RuntimeDiagnostics {
            forward_energy: Some(0.42),
            kv_influence: Some(0.25),
            ..RuntimeDiagnostics::default()
        })
        .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
            1,
            0,
            0,
            1,
            vec![0.3],
            vec![0.4],
        )]))
    }
}

#[derive(Debug, Clone)]
struct InvalidExportKernel {
    block: RuntimeKvBlock,
}

impl ProductionForwardKernel for InvalidExportKernel {
    fn generate(
        &self,
        _context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(ProductionKernelOutput::new("invalid kernel export")
            .with_exported_kv_blocks(vec![self.block.clone()]))
    }
}

#[derive(Debug, Clone)]
struct KvImportSkipForwardKernel;

impl ProductionForwardKernel for KvImportSkipForwardKernel {
    fn generate(
        &self,
        _context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(
            ProductionKernelOutput::new("kernel reported KV import skips")
                .with_tokens(vec![RuntimeToken {
                    text: "kernel".to_owned(),
                    logprob: Some(-0.2),
                    entropy: Some(0.3),
                }])
                .with_trace(vec![ReasoningStep::new(
                    "production_kernel",
                    "reported weak and budget-limited KV skips",
                    0.86,
                )])
                .with_diagnostics(RuntimeDiagnostics {
                    forward_energy: Some(0.42),
                    kv_influence: Some(0.25),
                    runtime_kv_segments_included: 1,
                    weak_runtime_kv_imports_skipped: 2,
                    budget_limited_runtime_kv_imports_skipped: 3,
                    ..RuntimeDiagnostics::default()
                })
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    1,
                    0,
                    0,
                    1,
                    vec![0.3],
                    vec![0.4],
                )]),
        )
    }
}

#[derive(Debug, Clone)]
struct AdapterStreamApplyForwardKernel;

impl ProductionForwardKernel for AdapterStreamApplyForwardKernel {
    fn generate(
        &self,
        _context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(
            ProductionKernelOutput::new("kernel reported adapter stream apply")
                .with_tokens(vec![RuntimeToken {
                    text: "kernel".to_owned(),
                    logprob: Some(-0.2),
                    entropy: Some(0.3),
                }])
                .with_trace(vec![ReasoningStep::new(
                    "production_kernel",
                    "reported adapter stream apply flags",
                    0.86,
                )])
                .with_diagnostics(RuntimeDiagnostics {
                    adapter_cache_mode: Some("chunked_cache".to_owned()),
                    adapter_stream_trace_id: Some("trace-conformance".to_owned()),
                    adapter_stream_gate_summary_digest: Some("fnv64:0123456789abcdef".to_owned()),
                    adapter_stream_read_only: Some(false),
                    adapter_stream_write_allowed: Some(true),
                    adapter_stream_applied: Some(true),
                    forward_energy: Some(0.42),
                    kv_influence: Some(0.25),
                    runtime_kv_segments_included: 1,
                    ..RuntimeDiagnostics::default()
                })
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    1,
                    0,
                    0,
                    1,
                    vec![0.3],
                    vec![0.4],
                )]),
        )
    }
}

#[derive(Debug, Clone)]
struct AdapterStreamMissingWriteGateForwardKernel;

impl ProductionForwardKernel for AdapterStreamMissingWriteGateForwardKernel {
    fn generate(
        &self,
        _context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(
            ProductionKernelOutput::new("kernel reported adapter stream without write gate")
                .with_tokens(vec![RuntimeToken {
                    text: "kernel".to_owned(),
                    logprob: Some(-0.2),
                    entropy: Some(0.3),
                }])
                .with_trace(vec![ReasoningStep::new(
                    "production_kernel",
                    "reported adapter stream without write gate state",
                    0.86,
                )])
                .with_diagnostics(RuntimeDiagnostics {
                    adapter_stream_trace_id: Some("trace-conformance".to_owned()),
                    adapter_stream_gate_summary_digest: Some("fnv64:0123456789abcdef".to_owned()),
                    forward_energy: Some(0.42),
                    kv_influence: Some(0.25),
                    runtime_kv_segments_included: 1,
                    ..RuntimeDiagnostics::default()
                })
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    1,
                    0,
                    0,
                    1,
                    vec![0.3],
                    vec![0.4],
                )]),
        )
    }
}

#[derive(Debug, Clone)]
struct AdapterStreamGateSummaryMissingWriteGateForwardKernel;

impl ProductionForwardKernel for AdapterStreamGateSummaryMissingWriteGateForwardKernel {
    fn generate(
        &self,
        _context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(
            ProductionKernelOutput::new("kernel reported adapter stream gate summary only")
                .with_tokens(vec![RuntimeToken {
                    text: "kernel".to_owned(),
                    logprob: Some(-0.2),
                    entropy: Some(0.3),
                }])
                .with_trace(vec![ReasoningStep::new(
                    "production_kernel",
                    "reported adapter stream gate summary without write gate state",
                    0.86,
                )])
                .with_diagnostics(RuntimeDiagnostics {
                    adapter_stream_gate_summary_digest: Some("fnv64:0123456789abcdef".to_owned()),
                    forward_energy: Some(0.42),
                    kv_influence: Some(0.25),
                    runtime_kv_segments_included: 1,
                    ..RuntimeDiagnostics::default()
                })
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    1,
                    0,
                    0,
                    1,
                    vec![0.3],
                    vec![0.4],
                )]),
        )
    }
}

#[derive(Debug, Clone)]
struct AdapterStreamMalformedGateSummaryForwardKernel;

impl ProductionForwardKernel for AdapterStreamMalformedGateSummaryForwardKernel {
    fn generate(
        &self,
        _context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(
            ProductionKernelOutput::new("kernel reported malformed adapter stream gate summary")
                .with_tokens(vec![RuntimeToken {
                    text: "kernel".to_owned(),
                    logprob: Some(-0.2),
                    entropy: Some(0.3),
                }])
                .with_trace(vec![ReasoningStep::new(
                    "production_kernel",
                    "reported malformed adapter stream gate summary digest",
                    0.86,
                )])
                .with_diagnostics(RuntimeDiagnostics {
                    adapter_stream_gate_summary_digest: Some("bad-digest".to_owned()),
                    adapter_stream_read_only: Some(true),
                    adapter_stream_write_allowed: Some(false),
                    adapter_stream_applied: Some(false),
                    forward_energy: Some(0.42),
                    kv_influence: Some(0.25),
                    runtime_kv_segments_included: 1,
                    ..RuntimeDiagnostics::default()
                })
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    1,
                    0,
                    0,
                    1,
                    vec![0.3],
                    vec![0.4],
                )]),
        )
    }
}

#[derive(Debug, Clone)]
struct AdapterStreamPartialWriteGateForwardKernel;

impl ProductionForwardKernel for AdapterStreamPartialWriteGateForwardKernel {
    fn generate(
        &self,
        _context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(
            ProductionKernelOutput::new("kernel reported partial adapter stream write gate")
                .with_tokens(vec![RuntimeToken {
                    text: "kernel".to_owned(),
                    logprob: Some(-0.2),
                    entropy: Some(0.3),
                }])
                .with_trace(vec![ReasoningStep::new(
                    "production_kernel",
                    "reported partial adapter stream write gate state",
                    0.86,
                )])
                .with_diagnostics(RuntimeDiagnostics {
                    adapter_stream_read_only: Some(true),
                    adapter_stream_applied: Some(false),
                    forward_energy: Some(0.42),
                    kv_influence: Some(0.25),
                    runtime_kv_segments_included: 1,
                    ..RuntimeDiagnostics::default()
                })
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    1,
                    0,
                    0,
                    1,
                    vec![0.3],
                    vec![0.4],
                )]),
        )
    }
}

#[derive(Debug, Clone)]
struct NoUncertaintyForwardKernel;

impl ProductionForwardKernel for NoUncertaintyForwardKernel {
    fn generate(
        &self,
        _context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(
            ProductionKernelOutput::new("kernel returned tokens without uncertainty")
                .with_tokens(vec![RuntimeToken::new("plain")])
                .with_trace(vec![ReasoningStep::new(
                    "production_kernel",
                    "returned token text but no entropy or logprob",
                    0.86,
                )])
                .with_diagnostics(RuntimeDiagnostics {
                    forward_energy: Some(0.42),
                    kv_influence: Some(0.0),
                    ..RuntimeDiagnostics::default()
                })
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    1,
                    0,
                    0,
                    1,
                    vec![0.3],
                    vec![0.4],
                )]),
        )
    }
}

#[derive(Debug, Clone)]
struct MalformedForwardKernel;

impl ProductionForwardKernel for MalformedForwardKernel {
    fn generate(
        &self,
        _context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        Ok(ProductionKernelOutput::new("malformed kernel output"))
    }
}
