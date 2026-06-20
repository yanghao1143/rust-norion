use super::*;
use crate::engine::{GenerationContext, InferenceBackend, InferenceRequest, NoironEngine};
use crate::reflection::{InferenceDraft, ReasoningStep, RuntimeDiagnostics};

#[derive(Debug, Clone, Copy)]
enum RuntimeDeviceExecutionMode {
    Matching,
    Mismatching,
    Missing,
    MissingKvPrecision,
    StaticArchitecture,
}

struct RuntimeDeviceExecutionBackend {
    mode: RuntimeDeviceExecutionMode,
}

impl RuntimeDeviceExecutionBackend {
    fn matching() -> Self {
        Self {
            mode: RuntimeDeviceExecutionMode::Matching,
        }
    }

    fn mismatching() -> Self {
        Self {
            mode: RuntimeDeviceExecutionMode::Mismatching,
        }
    }

    fn missing() -> Self {
        Self {
            mode: RuntimeDeviceExecutionMode::Missing,
        }
    }

    fn missing_kv_precision() -> Self {
        Self {
            mode: RuntimeDeviceExecutionMode::MissingKvPrecision,
        }
    }

    fn static_architecture() -> Self {
        Self {
            mode: RuntimeDeviceExecutionMode::StaticArchitecture,
        }
    }
}

impl InferenceBackend for RuntimeDeviceExecutionBackend {
    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        let execution = &context.hardware_plan.execution;
        let selected_adapter = execution
            .adapter_hints
            .first()
            .map(|adapter| adapter.as_str().to_owned());
        let diagnostics = match self.mode {
            RuntimeDeviceExecutionMode::Matching => RuntimeDiagnostics {
                model_id: Some("runtime-device-execution-test".to_owned()),
                selected_adapter: selected_adapter.clone(),
                layer_count: 6,
                global_layers: 2,
                local_window_layers: 2,
                convolutional_fusion_layers: 2,
                hidden_size: 64,
                local_window_tokens: 128,
                forward_energy: Some(0.31),
                kv_influence: Some(0.22),
                ..RuntimeDiagnostics::default()
                    .with_device_execution(
                        context.hardware_plan.device.as_str(),
                        execution.primary_lane.as_str(),
                        execution.fallback_lane.as_str(),
                        execution.memory_mode.as_str(),
                    )
                    .with_kv_precision(
                        execution.hot_kv_precision_bits,
                        execution.cold_kv_precision_bits,
                    )
            },
            RuntimeDeviceExecutionMode::Mismatching => RuntimeDiagnostics {
                model_id: Some("runtime-device-execution-test".to_owned()),
                selected_adapter: selected_adapter.clone(),
                layer_count: 6,
                forward_energy: Some(0.31),
                kv_influence: Some(0.22),
                ..RuntimeDiagnostics::default()
                    .with_device_execution("server", "cuda", "cpu-simd", "gpu-resident")
                    .with_kv_precision(
                        execution.hot_kv_precision_bits,
                        execution.cold_kv_precision_bits,
                    )
            },
            RuntimeDeviceExecutionMode::Missing => RuntimeDiagnostics {
                model_id: Some("runtime-device-execution-test".to_owned()),
                selected_adapter,
                layer_count: 6,
                forward_energy: Some(0.31),
                kv_influence: Some(0.22),
                ..RuntimeDiagnostics::default()
            },
            RuntimeDeviceExecutionMode::MissingKvPrecision => RuntimeDiagnostics {
                model_id: Some("runtime-device-execution-test".to_owned()),
                selected_adapter,
                layer_count: 6,
                forward_energy: Some(0.31),
                kv_influence: Some(0.22),
                ..RuntimeDiagnostics::default().with_device_execution(
                    context.hardware_plan.device.as_str(),
                    execution.primary_lane.as_str(),
                    execution.fallback_lane.as_str(),
                    execution.memory_mode.as_str(),
                )
            },
            RuntimeDeviceExecutionMode::StaticArchitecture => RuntimeDiagnostics {
                model_id: Some("gemma-static-architecture-test".to_owned()),
                selected_adapter: None,
                device_profile: Some(context.hardware_plan.device.as_str().to_owned()),
                primary_lane: Some(execution.primary_lane.as_str().to_owned()),
                fallback_lane: Some(execution.fallback_lane.as_str().to_owned()),
                memory_mode: Some(execution.memory_mode.as_str().to_owned()),
                device_execution_source: Some(
                    RuntimeDiagnostics::control_plane_filled_device_execution_source().to_owned(),
                ),
                hot_kv_precision_bits: Some(execution.hot_kv_precision_bits),
                cold_kv_precision_bits: Some(execution.cold_kv_precision_bits),
                layer_count: 48,
                hidden_size: 3840,
                local_window_tokens: 1024,
                ..RuntimeDiagnostics::default()
            },
        };

        InferenceDraft::new(
            "Runtime device execution diagnostics are available for benchmark gating.",
            vec![ReasoningStep::new(
                "runtime-device-execution",
                "Attach execution lane and memory-mode evidence to the runtime result.",
                0.91,
            )],
        )
        .with_runtime_diagnostics(diagnostics)
    }
}

#[test]
fn summary_records_runtime_device_execution_evidence() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.35,
        0.00,
        0.45,
        0.20,
    ));
    let mut backend = RuntimeDeviceExecutionBackend::matching();
    let case = BenchmarkCase::new(
        "runtime_device_execution",
        TaskProfile::General,
        "prove runtime device execution diagnostics match the hardware plan",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);

    assert_eq!(summary.runtime_device_execution_cases(), 1);
    assert_eq!(summary.runtime_device_execution_matched_cases(), 1);
    assert_eq!(summary.runtime_device_execution_device_profiles(), 1);
    assert_eq!(summary.runtime_kv_precision_cases(), 1);
    assert_eq!(summary.runtime_kv_precision_device_profiles(), 1);
    assert_eq!(summary.total_runtime_device_execution_violations(), 0);
    let report = summary.evaluate(&BenchmarkGate {
        min_runtime_device_execution_cases: Some(1),
        min_runtime_device_execution_device_profiles: Some(1),
        min_runtime_kv_precision_cases: Some(1),
        min_runtime_kv_precision_device_profiles: Some(1),
        max_runtime_device_execution_violations: Some(0),
        ..BenchmarkGate::default()
    });
    assert!(report.passed, "{:?}", report.failures);
    assert!(
        summary
            .summary_line()
            .contains("runtime_device_execution_matched_cases=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_device_execution_devices=cpu")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_kv_precision_cases=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_kv_precision_device_profiles=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_kv_precision_devices=cpu")
    );
}

#[test]
fn summary_records_static_runtime_architecture_without_device_execution_violation() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::DiscreteGpu,
        0.35,
        0.20,
        0.45,
        0.20,
    ));
    let mut backend = RuntimeDeviceExecutionBackend::static_architecture();
    let case = BenchmarkCase::new(
        "runtime_architecture",
        TaskProfile::Coding,
        "prove static Gemma runtime architecture diagnostics are benchmark-gated",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);

    assert_eq!(summary.runtime_architecture_cases(), 1);
    assert_eq!(summary.runtime_architecture_device_profiles(), 1);
    assert_eq!(summary.runtime_device_execution_cases(), 0);
    assert_eq!(summary.runtime_device_execution_matched_cases(), 0);
    assert_eq!(summary.total_runtime_device_execution_violations(), 0);
    let report = summary.evaluate(&BenchmarkGate {
        min_runtime_architecture_cases: Some(1),
        min_runtime_architecture_device_profiles: Some(1),
        max_runtime_device_execution_violations: Some(0),
        ..BenchmarkGate::default()
    });
    assert!(report.passed, "{:?}", report.failures);
    assert!(
        summary
            .summary_line()
            .contains("runtime_architecture_cases=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_architecture_device_profiles=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_architecture_devices=discrete")
    );
}

#[test]
fn gate_reports_runtime_device_execution_mismatch() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.35,
        0.00,
        0.45,
        0.20,
    ));
    let mut backend = RuntimeDeviceExecutionBackend::mismatching();
    let case = BenchmarkCase::new(
        "runtime_device_execution_mismatch",
        TaskProfile::General,
        "catch runtime device execution diagnostics that drift from hardware",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);
    let report = summary.evaluate(&BenchmarkGate::default());

    assert!(!report.passed);
    assert_eq!(summary.runtime_device_execution_cases(), 1);
    assert_eq!(summary.runtime_device_execution_matched_cases(), 0);
    assert_eq!(summary.total_runtime_device_execution_violations(), 1);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_device_execution_violations")
            && failure.contains("device_profile actual=server expected=cpu")
    }));
}

#[test]
fn gate_reports_missing_runtime_device_execution_for_forward_signal() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.35,
        0.00,
        0.45,
        0.20,
    ));
    let mut backend = RuntimeDeviceExecutionBackend::missing();
    let case = BenchmarkCase::new(
        "runtime_device_execution_missing",
        TaskProfile::General,
        "catch runtime forward diagnostics that omit device execution evidence",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);
    let report = summary.evaluate(&BenchmarkGate::default());

    assert!(!report.passed);
    assert_eq!(summary.runtime_device_execution_cases(), 0);
    assert_eq!(summary.runtime_device_execution_matched_cases(), 0);
    assert_eq!(summary.total_runtime_device_execution_violations(), 1);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_device_execution_violations")
            && failure.contains("missing device execution diagnostics")
    }));
}

#[test]
fn gate_reports_missing_runtime_kv_precision_diagnostics() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.35,
        0.00,
        0.45,
        0.20,
    ));
    let mut backend = RuntimeDeviceExecutionBackend::missing_kv_precision();
    let case = BenchmarkCase::new(
        "runtime_kv_precision_missing",
        TaskProfile::General,
        "catch runtime device execution diagnostics that omit KV precision evidence",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);
    let report = summary.evaluate(&BenchmarkGate {
        min_runtime_kv_precision_cases: Some(1),
        min_runtime_kv_precision_device_profiles: Some(1),
        max_runtime_device_execution_violations: Some(0),
        ..BenchmarkGate::default()
    });

    assert!(!report.passed);
    assert_eq!(summary.runtime_device_execution_cases(), 1);
    assert_eq!(summary.runtime_device_execution_matched_cases(), 0);
    assert_eq!(summary.runtime_kv_precision_cases(), 0);
    assert_eq!(summary.runtime_kv_precision_device_profiles(), 0);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_precision_cases 0 below minimum 1"))
    );
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_kv_precision_device_profiles 0 below minimum 1")
    }));
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_device_execution_violations")
            && failure.contains("missing valid KV precision diagnostics")
    }));
}
