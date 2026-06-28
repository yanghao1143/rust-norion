use super::*;
use crate::hardware::{
    ComputeLane, DeviceClass, DeviceExecutionPlan, DeviceMemoryMode, RuntimeAdapterHint,
};
use crate::kv_quant::QuantizationBits;
use crate::runtime::{RuntimeAdapterObservation, RuntimeMetadata};
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn self_developed_manifest_validates_and_covers_all_devices() {
    let manifest =
        RuntimeManifest::self_developed("noiron-dev-transformer", "noiron-tokenizer", 65_536, 256)
            .with_assets(
                RuntimeAssetPaths::new()
                    .with_weights("weights.noiron")
                    .with_tokenizer("tokenizer.noiron"),
            );

    let validation = manifest.validate();

    assert!(validation.passed(), "{validation:?}");
    for device in DeviceClass::explicit_profiles() {
        assert!(manifest.supports_device(*device), "{device:?}");
    }
    let metadata = manifest.runtime_metadata();
    assert!(metadata.supports_kv_import);
    assert!(metadata.supports_kv_export);
    assert_eq!(metadata.max_kv_import_blocks, 8);
    assert_eq!(metadata.max_kv_export_blocks, 4);
    assert_eq!(metadata.hot_kv_precision_bits, 8);
    assert_eq!(metadata.cold_kv_precision_bits, 4);
    assert_eq!(metadata.native_context_window, 65_536);
    assert_eq!(metadata.embedding_dimensions, 256);
}

#[test]
fn manifest_selects_first_supported_device_adapter() {
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128)
        .with_adapter_hints(vec![
            RuntimeAdapterHint::Wgpu,
            RuntimeAdapterHint::PortableRust,
        ]);
    let execution = DeviceExecutionPlan {
        primary_lane: ComputeLane::DiscreteGpu,
        fallback_lane: ComputeLane::CpuVector,
        memory_mode: DeviceMemoryMode::GpuResident,
        adapter_hints: vec![RuntimeAdapterHint::Cuda, RuntimeAdapterHint::Wgpu],
        max_parallel_chunks: 4,
        kv_prefetch_blocks: 4,
        hot_kv_precision_bits: 8,
        cold_kv_precision_bits: 4,
        allow_disk_spill: true,
    };

    assert_eq!(
        manifest.preferred_adapter_for(&execution),
        Some(RuntimeAdapterHint::Wgpu)
    );
}

#[test]
fn manifest_does_not_pick_adapter_outside_device_plan() {
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128)
        .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust]);
    let execution = DeviceExecutionPlan {
        primary_lane: ComputeLane::DiscreteGpu,
        fallback_lane: ComputeLane::CpuVector,
        memory_mode: DeviceMemoryMode::GpuResident,
        adapter_hints: vec![RuntimeAdapterHint::Cuda, RuntimeAdapterHint::Wgpu],
        max_parallel_chunks: 4,
        kv_prefetch_blocks: 4,
        hot_kv_precision_bits: 8,
        cold_kv_precision_bits: 4,
        allow_disk_spill: true,
    };

    assert_eq!(manifest.preferred_adapter_for(&execution), None);
}

#[test]
fn manifest_uses_runtime_observations_within_device_adapter_bounds() {
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128)
        .with_adapter_hints(vec![
            RuntimeAdapterHint::Wgpu,
            RuntimeAdapterHint::Vulkan,
            RuntimeAdapterHint::PortableRust,
        ]);
    let execution = DeviceExecutionPlan {
        primary_lane: ComputeLane::DiscreteGpu,
        fallback_lane: ComputeLane::CpuVector,
        memory_mode: DeviceMemoryMode::GpuResident,
        adapter_hints: vec![
            RuntimeAdapterHint::Cuda,
            RuntimeAdapterHint::Vulkan,
            RuntimeAdapterHint::Wgpu,
        ],
        max_parallel_chunks: 4,
        kv_prefetch_blocks: 4,
        hot_kv_precision_bits: 8,
        cold_kv_precision_bits: 4,
        allow_disk_spill: true,
    };
    let observations = vec![
        RuntimeAdapterObservation::new(
            RuntimeAdapterHint::Wgpu,
            0.62,
            0.60,
            0.70,
            Some(0.20),
            Some(0.10),
            1,
        ),
        RuntimeAdapterObservation::new(
            RuntimeAdapterHint::Vulkan,
            0.91,
            0.90,
            0.92,
            Some(0.16),
            Some(0.40),
            2,
        ),
        RuntimeAdapterObservation::new(
            RuntimeAdapterHint::PortableRust,
            0.99,
            0.99,
            0.99,
            Some(0.10),
            Some(0.60),
            3,
        ),
    ];

    assert_eq!(
        manifest.preferred_adapter_with_observations(&execution, &observations),
        Some(RuntimeAdapterHint::Vulkan)
    );
}

#[test]
fn manifest_blocks_retired_runtime_adapters_from_selection() {
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128)
        .with_adapter_hints(vec![
            RuntimeAdapterHint::Cuda,
            RuntimeAdapterHint::Wgpu,
            RuntimeAdapterHint::PortableRust,
        ])
        .with_retired_adapter_hints(vec![RuntimeAdapterHint::Cuda]);
    let execution = DeviceExecutionPlan {
        primary_lane: ComputeLane::DiscreteGpu,
        fallback_lane: ComputeLane::CpuVector,
        memory_mode: DeviceMemoryMode::GpuResident,
        adapter_hints: vec![RuntimeAdapterHint::Cuda, RuntimeAdapterHint::Wgpu],
        max_parallel_chunks: 4,
        kv_prefetch_blocks: 4,
        hot_kv_precision_bits: 8,
        cold_kv_precision_bits: 4,
        allow_disk_spill: true,
    };
    let observations = vec![
        RuntimeAdapterObservation::new(
            RuntimeAdapterHint::Cuda,
            0.99,
            0.99,
            0.99,
            Some(0.10),
            Some(0.70),
            1,
        ),
        RuntimeAdapterObservation::new(
            RuntimeAdapterHint::Wgpu,
            0.55,
            0.55,
            0.55,
            Some(0.20),
            Some(0.20),
            2,
        ),
    ];

    assert_eq!(
        manifest.preferred_adapter_for(&execution),
        Some(RuntimeAdapterHint::Wgpu)
    );
    assert_eq!(
        manifest.preferred_adapter_with_observations(&execution, &observations),
        Some(RuntimeAdapterHint::Wgpu)
    );
}

#[test]
fn manifest_blocks_quarantined_runtime_adapters_with_lifecycle_evidence() {
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128)
        .with_adapter_hints(vec![
            RuntimeAdapterHint::Cuda,
            RuntimeAdapterHint::Wgpu,
            RuntimeAdapterHint::PortableRust,
        ])
        .with_adapter_lifecycle_records(vec![RuntimeAdapterLifecycleRecord::new(
            RuntimeAdapterHint::Cuda,
            RuntimeAdapterLifecycleState::Quarantined,
            "polluted_runtime_source",
            "sha256:cuda-polluted-source",
            "lineage:runtime:cuda",
            "rollback:adapter:cuda",
            "scope:local-runtime",
        )]);
    let execution = DeviceExecutionPlan {
        primary_lane: ComputeLane::DiscreteGpu,
        fallback_lane: ComputeLane::CpuVector,
        memory_mode: DeviceMemoryMode::GpuResident,
        adapter_hints: vec![RuntimeAdapterHint::Cuda, RuntimeAdapterHint::Wgpu],
        max_parallel_chunks: 4,
        kv_prefetch_blocks: 4,
        hot_kv_precision_bits: 8,
        cold_kv_precision_bits: 4,
        allow_disk_spill: true,
    };
    let observations = vec![
        RuntimeAdapterObservation::new(
            RuntimeAdapterHint::Cuda,
            0.99,
            0.99,
            0.99,
            Some(0.10),
            Some(0.70),
            1,
        ),
        RuntimeAdapterObservation::new(
            RuntimeAdapterHint::Wgpu,
            0.55,
            0.55,
            0.55,
            Some(0.20),
            Some(0.20),
            2,
        ),
    ];
    let block = manifest
        .runtime_adapter_lifecycle_block_summary(RuntimeAdapterHint::Cuda)
        .expect("lifecycle block");

    assert!(manifest.validate().passed());
    assert!(block.contains("state=quarantined"));
    assert!(block.contains("source_digest=sha256:cuda-polluted-source"));
    assert_eq!(
        manifest.preferred_adapter_for(&execution),
        Some(RuntimeAdapterHint::Wgpu)
    );
    assert_eq!(
        manifest.preferred_adapter_with_observations(&execution, &observations),
        Some(RuntimeAdapterHint::Wgpu)
    );
}

#[test]
fn kv_policy_updates_runtime_metadata_capabilities() {
    let manifest = RuntimeManifest::from_metadata(RuntimeMetadata::new("model", "tok", 4096, 64))
        .with_kv_policy(RuntimeKvPolicy::import_export());

    assert!(manifest.metadata.supports_kv_import);
    assert!(manifest.metadata.supports_kv_export);
    assert!(manifest.runtime_metadata().supports_kv_import);
    assert!(manifest.runtime_metadata().supports_kv_export);
    assert!(manifest.validate().passed());
}

#[test]
fn manifest_inherits_kv_precision_from_runtime_metadata() {
    let manifest = RuntimeManifest::from_metadata(
        RuntimeMetadata::new("compact-model", "tok", 4096, 64).with_kv_precision(4, 4),
    );

    assert_eq!(manifest.quantization.hot_kv, QuantizationBits::Four);
    assert_eq!(manifest.quantization.cold_kv, QuantizationBits::Four);
    assert_eq!(manifest.runtime_metadata().hot_kv_precision_bits, 4);
    assert_eq!(manifest.runtime_metadata().cold_kv_precision_bits, 4);
}

#[test]
fn manifest_rejects_cold_kv_width_above_hot_kv_width() {
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128)
        .with_quantization(RuntimeQuantizationPolicy {
            hot_kv: QuantizationBits::Four,
            cold_kv: QuantizationBits::Eight,
            weights: None,
        });

    let validation = manifest.validate();

    assert!(!validation.passed());
    assert!(
        validation
            .errors
            .iter()
            .any(|error| { error.contains("cold_kv") && error.contains("hot_kv") })
    );
}

#[test]
fn production_manifest_requires_existing_model_assets() {
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128);

    let validation = manifest.validate_for_production();

    assert!(!validation.passed());
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("weights asset path is required"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("tokenizer asset path is required"))
    );
}

#[test]
fn production_manifest_accepts_existing_asset_files() {
    let asset_dir = temp_asset_dir("runtime-manifest-assets");
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    let config = asset_dir.join("config.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    File::create(&config).unwrap();
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128).with_assets(
        RuntimeAssetPaths::new()
            .with_weights(weights)
            .with_tokenizer(tokenizer)
            .with_config(config),
    );

    let validation = manifest.validate_for_production();

    assert!(validation.passed(), "{validation:?}");
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn production_manifest_rejects_directories_as_assets() {
    let asset_dir = temp_asset_dir("runtime-manifest-directory-assets");
    let weights = asset_dir.join("weights-dir");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    fs::create_dir_all(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let manifest = RuntimeManifest::self_developed("model", "tokenizer", 8_192, 128).with_assets(
        RuntimeAssetPaths::new()
            .with_weights(&weights)
            .with_tokenizer(&tokenizer),
    );

    let validation = manifest.validate_for_production();

    assert!(!validation.passed());
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("weights asset path is not a file"))
    );
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn invalid_manifest_reports_blocking_errors() {
    let manifest = RuntimeManifest {
        metadata: RuntimeMetadata::new("", "", 0, 0),
        architecture: TransformerRuntimeArchitecture::new(0, 130, 8, 16, 0),
        assets: RuntimeAssetPaths::default(),
        kv_policy: RuntimeKvPolicy::import_export(),
        quantization: RuntimeQuantizationPolicy::default(),
        supported_devices: Vec::new(),
        adapter_hints: Vec::new(),
        retired_adapter_hints: Vec::new(),
        adapter_lifecycle_records: Vec::new(),
    };

    let validation = manifest.validate();

    assert!(!validation.passed());
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("model_id"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("native_context_window"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("hidden_size"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("supported_devices"))
    );
}

fn temp_asset_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{name}-{unique}"))
}
