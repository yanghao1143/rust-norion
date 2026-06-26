use super::*;

fn record_cpu_runtime_adapter_experience(
    engine: &mut NoironEngine,
    adapter: &str,
    quality: f32,
    reward: f32,
    forward_energy: f32,
) {
    engine.experience.record(ExperienceInput {
        prompt: format!("inspect runtime adapter {adapter}"),
        profile: TaskProfile::General,
        lesson: format!("reuse {adapter} only when persisted evidence still wins"),
        quality,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.50,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.50,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.34, 0.33, 0.33),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: crate::reflection::RuntimeDiagnostics {
            model_id: Some("inspect-runtime".to_owned()),
            selected_adapter: Some(adapter.to_owned()),
            device_profile: Some("cpu".to_owned()),
            primary_lane: Some("cpu-vector".to_owned()),
            fallback_lane: Some("cpu-portable".to_owned()),
            memory_mode: Some("tiered-disk".to_owned()),
            device_execution_source: Some(
                crate::reflection::RuntimeDiagnostics::runtime_reported_device_execution_source()
                    .to_owned(),
            ),
            layer_count: 8,
            global_layers: 2,
            local_window_layers: 4,
            convolutional_fusion_layers: 2,
            hidden_size: 96,
            local_window_tokens: 2048,
            forward_energy: Some(forward_energy),
            kv_influence: Some(0.42),
            imported_kv_blocks: 1,
            exported_kv_blocks: 1,
            hot_kv_precision_bits: Some(8),
            cold_kv_precision_bits: Some(4),
            ..crate::reflection::RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: reward,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });
}

#[test]
fn state_inspection_matrix_gate_can_require_runtime_evidence_per_device() {
    let passing = StateInspectionGateReport {
        passed: true,
        failures: Vec::new(),
    };
    let gate = StateInspectionMatrixGate {
        min_runtime_kv_memory_device_profiles: Some(2),
        min_runtime_model_device_profiles: Some(2),
        min_runtime_adapter_device_profiles: Some(2),
        min_runtime_forward_energy_device_profiles: Some(1),
        min_runtime_kv_influence_device_profiles: Some(1),
        min_runtime_uncertainty_device_profiles: Some(1),
        min_runtime_uncertainty_token_device_profiles: Some(1),
        min_runtime_kv_precision_device_profiles: Some(2),
        max_runtime_kv_precision_mismatches: Some(0),
        min_runtime_device_execution_device_profiles: Some(2),
        min_runtime_layer_mode_device_profiles: Some(2),
        min_runtime_all_layer_mode_device_profiles: Some(1),
        min_runtime_kv_import_device_profiles: Some(1),
        min_runtime_kv_weak_import_skip_device_profiles: Some(1),
        min_runtime_kv_export_device_profiles: Some(1),
        min_runtime_kv_segment_device_profiles: Some(1),
        ..StateInspectionMatrixGate::default()
    };

    let report = StateInspectionMatrixGateReport::evaluate_with_gate(
        DeviceClass::explicit_profiles()
            .iter()
            .copied()
            .map(|device| {
                let mut device_report =
                    StateInspectionDeviceGateReport::new(device, passing.clone());
                match device {
                    DeviceClass::CpuOnly => {
                        device_report = device_report.with_runtime_evidence(1, 1, 1, 1, 1, 1, 1, 1);
                        device_report = device_report.with_runtime_uncertainty_evidence(1, 7);
                        device_report = device_report.with_runtime_kv_precision_evidence(1);
                        device_report = device_report.with_runtime_layer_mode_evidence(1, 1);
                        device_report = device_report.with_runtime_kv_weak_skip_evidence(1, 3);
                        device_report = device_report.with_runtime_kv_segment_evidence(1, 2, 1, 0);
                    }
                    DeviceClass::IntegratedGpu => {
                        device_report = device_report.with_runtime_evidence(2, 1, 1, 0, 0, 1, 0, 0);
                        device_report = device_report.with_runtime_kv_precision_evidence(1);
                        device_report = device_report.with_runtime_layer_mode_evidence(1, 0);
                    }
                    _ => {}
                }
                device_report
            })
            .collect(),
        &gate,
    );

    assert!(report.passed(), "{:?}", report.failures);
    assert_eq!(report.runtime_kv_memory_device_profiles(), 2);
    assert_eq!(report.runtime_model_device_profiles(), 2);
    assert_eq!(report.runtime_adapter_device_profiles(), 2);
    assert_eq!(report.runtime_forward_energy_device_profiles(), 1);
    assert_eq!(report.runtime_kv_influence_device_profiles(), 1);
    assert_eq!(report.runtime_uncertainty_device_profiles(), 1);
    assert_eq!(report.runtime_uncertainty_token_device_profiles(), 1);
    assert_eq!(report.runtime_kv_precision_device_profiles(), 2);
    assert_eq!(report.runtime_kv_precision_mismatches(), 0);
    assert_eq!(report.runtime_device_execution_device_profiles(), 2);
    assert_eq!(report.runtime_layer_mode_device_profiles(), 2);
    assert_eq!(report.runtime_all_layer_mode_device_profiles(), 1);
    assert_eq!(report.runtime_kv_import_device_profiles(), 1);
    assert_eq!(report.runtime_kv_weak_import_skip_device_profiles(), 1);
    assert_eq!(report.runtime_kv_export_device_profiles(), 1);
    assert_eq!(report.runtime_kv_segment_device_profiles(), 1);
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_memory_device_profiles=2")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_model_device_profiles=2")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_device_execution_device_profiles=2")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_uncertainty_device_profiles=1")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_uncertainty_token_device_profiles=1")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_precision_device_profiles=2")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_precision_mismatches=0")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_layer_mode_device_profiles=2")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_all_layer_mode_device_profiles=1")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_weak_import_skip_device_profiles=1")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_segment_device_profiles=1")
    );

    let failing = StateInspectionMatrixGateReport::evaluate_with_gate(
        vec![
            StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing.clone())
                .with_runtime_evidence(1, 1, 0, 0, 0, 0, 0, 0),
        ],
        &gate,
    );

    assert!(!failing.passed());
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_kv_memory_device_profiles 1 below required 2" })
    );
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_adapter_device_profiles 0 below required 2" })
    );
    assert!(
        failing.failures.iter().any(|failure| {
            failure == "runtime_forward_energy_device_profiles 0 below required 1"
        })
    );
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_uncertainty_device_profiles 0 below required 1" })
    );
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_uncertainty_token_device_profiles 0 below required 1"
    }));
    assert!(
        failing.failures.iter().any(|failure| {
            failure == "runtime_kv_precision_device_profiles 0 below required 2"
        })
    );
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_device_execution_device_profiles 0 below required 2"
    }));
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_layer_mode_device_profiles 0 below required 2" })
    );
    assert!(
        failing.failures.iter().any(|failure| {
            failure == "runtime_all_layer_mode_device_profiles 0 below required 1"
        })
    );
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_kv_export_device_profiles 0 below required 1" })
    );
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_kv_weak_import_skip_device_profiles 0 below required 1"
    }));
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_kv_segment_device_profiles 0 below required 1" })
    );

    let mismatch = StateInspectionMatrixGateReport::evaluate_with_gate(
        vec![
            StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing.clone())
                .with_runtime_kv_precision_evidence(1)
                .with_runtime_kv_precision_mismatches(1),
            StateInspectionDeviceGateReport::new(DeviceClass::IntegratedGpu, passing)
                .with_runtime_kv_precision_evidence(1),
        ],
        &StateInspectionMatrixGate {
            min_runtime_kv_precision_device_profiles: Some(2),
            max_runtime_kv_precision_mismatches: Some(0),
            ..StateInspectionMatrixGate::default()
        },
    );

    assert_eq!(mismatch.runtime_kv_precision_device_profiles(), 2);
    assert_eq!(mismatch.runtime_kv_precision_mismatches(), 1);
    assert!(!mismatch.passed());
    assert!(
        mismatch
            .failures
            .iter()
            .any(|failure| { failure == "runtime_kv_precision_mismatches 1 above maximum 0" })
    );
    assert!(
        mismatch
            .summary_line()
            .contains("runtime_kv_precision_mismatches=1")
    );

    let adapter_mismatch_passing = StateInspectionGateReport {
        passed: true,
        failures: Vec::new(),
    };
    let adapter_mismatch = StateInspectionMatrixGateReport::evaluate_with_gate(
        vec![
            StateInspectionDeviceGateReport::new(
                DeviceClass::CpuOnly,
                adapter_mismatch_passing.clone(),
            )
            .with_runtime_adapter_selection_mismatches(1),
            StateInspectionDeviceGateReport::new(
                DeviceClass::IntegratedGpu,
                adapter_mismatch_passing,
            ),
        ],
        &StateInspectionMatrixGate {
            max_runtime_adapter_selection_mismatches: Some(0),
            ..StateInspectionMatrixGate::default()
        },
    );

    assert_eq!(adapter_mismatch.runtime_adapter_selection_mismatches(), 1);
    assert!(!adapter_mismatch.passed());
    assert!(
        adapter_mismatch
            .failures
            .iter()
            .any(|failure| { failure == "runtime_adapter_selection_mismatches 1 above maximum 0" })
    );
    assert!(
        adapter_mismatch
            .summary_line()
            .contains("runtime_adapter_selection_mismatches=1")
    );
}

#[test]
fn inspection_gate_rejects_runtime_kv_precision_execution_mismatch() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::Embedded,
        0.35,
        0.00,
        0.45,
        0.20,
    ));
    engine.experience.record(ExperienceInput {
        prompt: "inspect runtime kv precision mismatch".to_owned(),
        profile: TaskProfile::General,
        lesson: "persisted diagnostics must match the device execution precision".to_owned(),
        quality: 0.88,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.50,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.50,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.34, 0.33, 0.33),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: crate::reflection::RuntimeDiagnostics {
            model_id: Some("inspect-runtime".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            device_profile: Some("embedded".to_owned()),
            primary_lane: Some("disk-streaming".to_owned()),
            fallback_lane: Some("cpu-portable".to_owned()),
            memory_mode: Some("minimal-disk".to_owned()),
            device_execution_source: Some(
                crate::reflection::RuntimeDiagnostics::runtime_reported_device_execution_source()
                    .to_owned(),
            ),
            layer_count: 4,
            global_layers: 1,
            local_window_layers: 2,
            convolutional_fusion_layers: 1,
            hidden_size: 64,
            local_window_tokens: 512,
            forward_energy: Some(0.24),
            kv_influence: Some(0.36),
            imported_kv_blocks: 1,
            exported_kv_blocks: 1,
            hot_kv_precision_bits: Some(8),
            cold_kv_precision_bits: Some(4),
            ..crate::reflection::RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport::default(),
        live_evolution: Default::default(),
    });

    let report = StateInspectionReport::from_engine(&engine, 1);
    let gate_report = report.evaluate(&StateInspectionGate {
        min_runtime_kv_precision_experiences: Some(1),
        max_runtime_kv_precision_mismatches: Some(0),
        ..StateInspectionGate::default()
    });

    assert_eq!(report.runtime_kv_precision_experience_count, 1);
    assert_eq!(report.runtime_kv_precision_mismatch_count, 1);
    assert!(!gate_report.passed());
    assert!(
        gate_report
            .failures
            .iter()
            .any(|failure| { failure == "runtime_kv_precision_mismatch_count 1 above maximum 0" })
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_precision_mismatches=1")
    );
}

#[test]
fn inspection_gate_rejects_runtime_adapter_selection_mismatch() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.35,
        0.00,
        0.45,
        0.20,
    ));
    record_cpu_runtime_adapter_experience(&mut engine, "cpu-simd", 0.96, 0.92, 0.10);
    record_cpu_runtime_adapter_experience(&mut engine, "portable-rust", 0.56, 0.42, 0.35);

    let report = StateInspectionReport::from_engine(&engine, 2);
    let gate_report = report.evaluate(&StateInspectionGate {
        min_runtime_adapter_experiences: Some(2),
        max_runtime_adapter_selection_mismatches: Some(0),
        ..StateInspectionGate::default()
    });

    assert_eq!(report.runtime_adapter_experience_count, 2);
    assert_eq!(report.runtime_adapter_selection_mismatch_count, 1);
    assert!(!gate_report.passed());
    assert!(gate_report.failures.iter().any(|failure| {
        failure == "runtime_adapter_selection_mismatch_count 1 above maximum 0"
    }));
    assert!(
        report
            .summary_line()
            .contains("runtime_adapter_selection_mismatches=1")
    );
}

#[test]
fn inspection_gate_does_not_count_exported_only_runtime_kv_as_budget_pressure() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.35,
        0.00,
        0.45,
        0.20,
    ));
    record_cpu_runtime_adapter_experience(&mut engine, "cpu-simd", 0.96, 0.92, 0.10);

    let report = StateInspectionReport::from_engine(&engine, 1);

    assert_eq!(report.runtime_kv_export_experience_count, 1);
    assert_eq!(report.runtime_kv_budget_pressure_experience_count, 0);
    assert_eq!(report.runtime_kv_budget_pressure_avg, 0.0);
    let pressure = report.evaluate(&StateInspectionGate {
        min_runtime_kv_budget_pressure_experiences: Some(1),
        ..StateInspectionGate::default()
    });

    assert!(!pressure.passed());
    assert!(pressure.failures.iter().any(|failure| {
        failure == "runtime_kv_budget_pressure_experience_count 0 below required 1"
    }));
}

#[test]
fn inspection_gate_ignores_untrusted_runtime_selected_adapter() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.35,
        0.00,
        0.45,
        0.20,
    ));
    record_cpu_runtime_adapter_experience(&mut engine, "portable-rust", 0.96, 0.92, 0.10);
    let record_id = engine.experience.records()[0].id;
    engine
        .experience
        .record_mut(record_id)
        .expect("runtime adapter experience")
        .runtime_diagnostics
        .selected_adapter = Some("unknown-adapter secret=sk-inspect".to_owned());

    let report = StateInspectionReport::from_engine(&engine, 2);
    let gate_report = report.evaluate(&StateInspectionGate {
        min_runtime_adapter_experiences: Some(1),
        max_runtime_adapter_selection_mismatches: Some(0),
        ..StateInspectionGate::default()
    });

    assert_eq!(
        engine.experience.records()[0]
            .runtime_diagnostics
            .selected_adapter
            .as_deref(),
        Some("unknown-adapter secret=sk-inspect")
    );
    assert_eq!(report.runtime_adapter_experience_count, 0);
    assert_eq!(report.runtime_adapter_selection_mismatch_count, 0);
    assert!(!gate_report.passed());
    assert!(
        gate_report
            .failures
            .iter()
            .any(|failure| { failure == "runtime_adapter_experience_count 0 below required 1" })
    );
    assert!(
        !gate_report
            .failures
            .iter()
            .any(|failure| { failure.contains("runtime_adapter_selection_mismatch_count") })
    );
    for marker in ["unknown-adapter", "secret=", "sk-inspect"] {
        assert!(!report.summary_line().contains(marker));
    }
}

#[test]
fn inspection_gate_tracks_runtime_kv_hold_evidence() {
    let mut engine = NoironEngine::new();
    engine.experience.record(ExperienceInput {
        prompt: "fast path exported runtime kv should be held".to_owned(),
        profile: TaskProfile::General,
        lesson: "runtime kv export can be audited even when durable runtime kv write is held"
            .to_owned(),
        quality: 0.64,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.5,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 0,
            fast_tokens: 4,
            attention_fraction: 0.0,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: vec![41],
        runtime_diagnostics: crate::reflection::RuntimeDiagnostics {
            exported_kv_blocks: 3,
            ..crate::reflection::RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport::default(),
        live_evolution: Default::default(),
    });

    let report = StateInspectionReport::from_engine(&engine, 1);
    let gate_report = report.evaluate(&StateInspectionGate {
        min_runtime_kv_export_experiences: Some(1),
        min_runtime_kv_hold_experiences: Some(1),
        min_runtime_kv_held_blocks: Some(2),
        ..StateInspectionGate::default()
    });

    assert_eq!(report.runtime_kv_export_experience_count, 1);
    assert_eq!(report.runtime_kv_hold_experience_count, 1);
    assert_eq!(report.runtime_kv_held_blocks, 2);
    assert!(gate_report.passed(), "{:?}", gate_report.failures);
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_hold_experiences=1")
    );
    assert!(report.summary_line().contains("runtime_kv_held_blocks=2"));

    let failing = report.evaluate(&StateInspectionGate {
        min_runtime_kv_hold_experiences: Some(2),
        min_runtime_kv_held_blocks: Some(3),
        ..StateInspectionGate::default()
    });
    assert!(!failing.passed());
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_kv_hold_experience_count 1 below required 2" })
    );
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_kv_held_blocks 2 below required 3" })
    );

    let passing = StateInspectionGateReport {
        passed: true,
        failures: Vec::new(),
    };
    let matrix = StateInspectionMatrixGateReport::evaluate_with_gate(
        DeviceClass::explicit_profiles()
            .iter()
            .copied()
            .map(|device| {
                let report = StateInspectionDeviceGateReport::new(device, passing.clone());
                if device == DeviceClass::CpuOnly {
                    report.with_runtime_kv_hold_evidence(1, 2)
                } else {
                    report
                }
            })
            .collect(),
        &StateInspectionMatrixGate {
            min_runtime_kv_hold_device_profiles: Some(1),
            ..StateInspectionMatrixGate::default()
        },
    );

    assert!(matrix.passed(), "{:?}", matrix.failures);
    assert_eq!(matrix.runtime_kv_hold_device_profiles(), 1);
    assert!(
        matrix
            .summary_line()
            .contains("runtime_kv_hold_device_profiles=1")
    );
}

#[test]
fn inspection_gate_tracks_runtime_kv_activity_evidence() {
    let mut engine = NoironEngine::new();
    engine.experience.record(ExperienceInput {
        prompt: "weak runtime kv import should remain inspectable".to_owned(),
        profile: TaskProfile::General,
        lesson: "weak runtime kv skips and segment filtering are activity evidence, not imports"
            .to_owned(),
        quality: 0.71,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.5,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 3,
            attention_fraction: 0.25,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: crate::reflection::RuntimeDiagnostics {
            weak_runtime_kv_imports_skipped: 3,
            budget_limited_runtime_kv_imports_skipped: 4,
            runtime_kv_segments_included: 2,
            runtime_kv_segments_skipped: 1,
            runtime_kv_segments_rejected: 1,
            ..crate::reflection::RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport::default(),
        live_evolution: Default::default(),
    });

    let report = StateInspectionReport::from_engine(&engine, 1);
    let gate_report = report.evaluate(&StateInspectionGate {
        min_runtime_kv_weak_import_skip_experiences: Some(1),
        min_weak_runtime_kv_imports_skipped: Some(3),
        min_runtime_kv_weak_import_pressure_experiences: Some(1),
        min_runtime_kv_weak_import_pressure: Some(1.0),
        max_runtime_kv_weak_import_pressure: Some(1.0),
        min_runtime_kv_budget_import_skip_experiences: Some(1),
        min_budget_limited_runtime_kv_imports_skipped: Some(4),
        min_runtime_kv_budget_pressure_experiences: Some(1),
        min_runtime_kv_budget_pressure: Some(1.0),
        max_runtime_kv_budget_pressure: Some(1.0),
        min_runtime_kv_segment_experiences: Some(1),
        min_runtime_kv_segments_included: Some(2),
        max_runtime_kv_segments_rejected: Some(1),
        ..StateInspectionGate::default()
    });

    assert_eq!(report.runtime_kv_import_experience_count, 0);
    assert_eq!(report.runtime_kv_export_experience_count, 0);
    assert_eq!(report.runtime_kv_weak_import_skip_experience_count, 1);
    assert_eq!(report.weak_runtime_kv_imports_skipped, 3);
    assert_eq!(report.runtime_kv_weak_import_pressure_experience_count, 1);
    assert!((report.runtime_kv_weak_import_pressure_avg - 1.0).abs() < 0.0001);
    assert!((report.runtime_kv_weak_import_pressure_max - 1.0).abs() < 0.0001);
    assert_eq!(report.runtime_kv_budget_import_skip_experience_count, 1);
    assert_eq!(report.budget_limited_runtime_kv_imports_skipped, 4);
    assert_eq!(report.runtime_kv_budget_pressure_experience_count, 1);
    assert!((report.runtime_kv_budget_pressure_avg - 1.0).abs() < 0.0001);
    assert!((report.runtime_kv_budget_pressure_max - 1.0).abs() < 0.0001);
    assert_eq!(report.runtime_kv_segment_experience_count, 1);
    assert_eq!(report.runtime_kv_segments_included, 2);
    assert_eq!(report.runtime_kv_segments_skipped, 1);
    assert_eq!(report.runtime_kv_segments_rejected, 1);
    assert!(gate_report.passed(), "{:?}", gate_report.failures);
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_weak_import_skip_experiences=1")
    );
    assert!(
        report
            .summary_line()
            .contains("weak_runtime_kv_imports_skipped=3")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_weak_import_pressure_experiences=1")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_weak_import_pressure_avg=1.000")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_weak_import_pressure_max=1.000")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_budget_import_skip_experiences=1")
    );
    assert!(
        report
            .summary_line()
            .contains("budget_limited_runtime_kv_imports_skipped=4")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_budget_pressure_experiences=1")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_budget_pressure_avg=1.000")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_budget_pressure_max=1.000")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_segment_experiences=1")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_segments_included=2")
    );
    assert!(
        report
            .summary_line()
            .contains("runtime_kv_segments_rejected=1")
    );

    let top = &report.top_experiences[0];
    assert_eq!(top.runtime_imported_kv_blocks, 0);
    assert_eq!(top.runtime_exported_kv_blocks, 0);
    assert_eq!(top.runtime_weak_kv_imports_skipped, 3);
    assert_eq!(top.runtime_budget_limited_kv_imports_skipped, 4);
    assert_eq!(top.runtime_kv_segments_included, 2);
    assert_eq!(top.runtime_kv_segments_skipped, 1);
    assert_eq!(top.runtime_kv_segments_rejected, 1);

    let device_report =
        StateInspectionDeviceGateReport::from_report(DeviceClass::CpuOnly, &report, gate_report);
    assert_eq!(device_report.runtime_kv_weak_import_skip_experiences, 1);
    assert_eq!(device_report.weak_runtime_kv_imports_skipped, 3);
    assert_eq!(device_report.runtime_kv_weak_import_pressure_experiences, 1);
    assert_eq!(device_report.runtime_kv_budget_import_skip_experiences, 1);
    assert_eq!(device_report.budget_limited_runtime_kv_imports_skipped, 4);
    assert_eq!(device_report.runtime_kv_budget_pressure_experiences, 1);
    assert_eq!(device_report.runtime_kv_segment_experiences, 1);
    assert_eq!(device_report.runtime_kv_segments_included, 2);
    assert_eq!(device_report.runtime_kv_segments_skipped, 1);
    assert_eq!(device_report.runtime_kv_segments_rejected, 1);
    let matrix = StateInspectionMatrixGateReport::evaluate_with_gate(
        vec![device_report],
        &StateInspectionMatrixGate {
            min_runtime_kv_weak_import_skip_device_profiles: Some(1),
            min_runtime_kv_weak_import_pressure_device_profiles: Some(1),
            min_runtime_kv_budget_import_skip_device_profiles: Some(1),
            min_runtime_kv_budget_pressure_device_profiles: Some(1),
            min_runtime_kv_segment_device_profiles: Some(1),
            ..StateInspectionMatrixGate::default()
        },
    );
    assert_eq!(matrix.runtime_kv_weak_import_skip_device_profiles(), 1);
    assert_eq!(matrix.runtime_kv_weak_import_pressure_device_profiles(), 1);
    assert_eq!(matrix.runtime_kv_budget_import_skip_device_profiles(), 1);
    assert_eq!(matrix.runtime_kv_budget_pressure_device_profiles(), 1);
    assert_eq!(matrix.runtime_kv_segment_device_profiles(), 1);
    assert!(!matrix.passed());
    assert!(!matrix.failures.iter().any(|failure| {
        failure.contains("runtime_kv_weak_import_skip_device_profiles")
            || failure.contains("runtime_kv_weak_import_pressure_device_profiles")
            || failure.contains("runtime_kv_budget_import_skip_device_profiles")
            || failure.contains("runtime_kv_budget_pressure_device_profiles")
            || failure.contains("runtime_kv_segment_device_profiles")
    }));
    assert!(
        matrix
            .summary_line()
            .contains("runtime_kv_weak_import_skip_device_profiles=1")
    );
    assert!(
        matrix
            .summary_line()
            .contains("runtime_kv_weak_import_pressure_device_profiles=1")
    );
    assert!(
        matrix
            .summary_line()
            .contains("runtime_kv_budget_import_skip_device_profiles=1")
    );
    assert!(
        matrix
            .summary_line()
            .contains("runtime_kv_budget_pressure_device_profiles=1")
    );
    assert!(
        matrix
            .summary_line()
            .contains("runtime_kv_segment_device_profiles=1")
    );
    let missing_pressure_matrix = StateInspectionMatrixGateReport::evaluate_with_gate(
        vec![StateInspectionDeviceGateReport::new(
            DeviceClass::CpuOnly,
            StateInspectionGateReport {
                passed: true,
                failures: Vec::new(),
            },
        )],
        &StateInspectionMatrixGate {
            min_runtime_kv_budget_pressure_device_profiles: Some(1),
            ..StateInspectionMatrixGate::default()
        },
    );
    assert!(missing_pressure_matrix.failures.iter().any(|failure| {
        failure == "runtime_kv_budget_pressure_device_profiles 0 below required 1"
    }));
    let missing_weak_pressure_matrix = StateInspectionMatrixGateReport::evaluate_with_gate(
        vec![StateInspectionDeviceGateReport::new(
            DeviceClass::CpuOnly,
            StateInspectionGateReport {
                passed: true,
                failures: Vec::new(),
            },
        )],
        &StateInspectionMatrixGate {
            min_runtime_kv_weak_import_pressure_device_profiles: Some(1),
            ..StateInspectionMatrixGate::default()
        },
    );
    assert!(missing_weak_pressure_matrix.failures.iter().any(|failure| {
        failure == "runtime_kv_weak_import_pressure_device_profiles 0 below required 1"
    }));

    let failing = report.evaluate(&StateInspectionGate {
        min_runtime_kv_weak_import_skip_experiences: Some(2),
        min_weak_runtime_kv_imports_skipped: Some(4),
        min_runtime_kv_weak_import_pressure_experiences: Some(2),
        min_runtime_kv_weak_import_pressure: Some(1.1),
        max_runtime_kv_weak_import_pressure: Some(0.5),
        min_runtime_kv_budget_import_skip_experiences: Some(2),
        min_budget_limited_runtime_kv_imports_skipped: Some(5),
        min_runtime_kv_budget_pressure_experiences: Some(2),
        min_runtime_kv_budget_pressure: Some(1.1),
        max_runtime_kv_budget_pressure: Some(0.5),
        min_runtime_kv_segment_experiences: Some(2),
        min_runtime_kv_segments_included: Some(3),
        max_runtime_kv_segments_rejected: Some(0),
        ..StateInspectionGate::default()
    });

    assert!(!failing.passed());
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_kv_weak_import_skip_experience_count 1 below required 2"
    }));
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "weak_runtime_kv_imports_skipped 3 below required 4" })
    );
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_kv_weak_import_pressure_experience_count 1 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_kv_weak_import_pressure_avg 1.000000 below required 1.100000"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_kv_weak_import_pressure_max 1.000000 above maximum 0.500000"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_kv_budget_import_skip_experience_count 1 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "budget_limited_runtime_kv_imports_skipped 4 below required 5"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_kv_budget_pressure_experience_count 1 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_kv_budget_pressure_avg 1.000000 below required 1.100000"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "runtime_kv_budget_pressure_max 1.000000 above maximum 0.500000"
    }));
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_kv_segment_experience_count 1 below required 2" })
    );
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_kv_segments_included 2 below required 3" })
    );
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| { failure == "runtime_kv_segments_rejected 1 above maximum 0" })
    );
}

#[test]
fn inspection_gate_rejects_experiences_without_runtime_evidence() {
    let mut engine = NoironEngine::new();
    engine.experience.record(ExperienceInput {
        prompt: "plain heuristic answer".to_owned(),
        profile: TaskProfile::General,
        lesson: "experience without runtime diagnostics should not satisfy runtime gates"
            .to_owned(),
        quality: 0.72,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.5,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.34, 0.33, 0.33),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: crate::reflection::RuntimeDiagnostics::default(),
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.62,
            action: RewardAction::Hold,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });

    let report = StateInspectionReport::from_engine(&engine, 3);
    let gate = StateInspectionGate {
        min_experiences: Some(1),
        min_runtime_model_experiences: Some(1),
        min_runtime_adapter_experiences: Some(1),
        max_runtime_adapter_selection_mismatches: Some(0),
        min_runtime_forward_energy_experiences: Some(1),
        min_runtime_kv_influence_experiences: Some(1),
        min_runtime_tokens: Some(1),
        min_runtime_uncertainty_experiences: Some(1),
        min_runtime_uncertainty_tokens: Some(1),
        min_runtime_architecture_experiences: Some(1),
        min_runtime_kv_precision_experiences: Some(1),
        max_runtime_kv_precision_mismatches: Some(0),
        min_runtime_device_execution_experiences: Some(1),
        min_runtime_layer_mode_experiences: Some(1),
        min_runtime_all_layer_mode_experiences: Some(1),
        min_runtime_global_layers: Some(1),
        min_runtime_local_window_layers: Some(1),
        min_runtime_convolutional_fusion_layers: Some(1),
        min_runtime_kv_import_experiences: Some(1),
        min_runtime_kv_export_experiences: Some(1),
        require_runtime_kv_dimensions: false,
        ..StateInspectionGate::default()
    };

    let gate_report = report.evaluate(&gate);

    assert_eq!(report.experience_count, 1);
    assert_eq!(report.runtime_model_experience_count, 0);
    assert_eq!(report.runtime_adapter_experience_count, 0);
    assert_eq!(report.runtime_forward_energy_experience_count, 0);
    assert_eq!(report.runtime_kv_influence_experience_count, 0);
    assert_eq!(report.runtime_token_count, 0);
    assert_eq!(report.runtime_uncertainty_token_count, 0);
    assert_eq!(report.runtime_architecture_experience_count, 0);
    assert_eq!(report.runtime_kv_precision_experience_count, 0);
    assert_eq!(report.runtime_kv_precision_mismatch_count, 0);
    assert_eq!(report.runtime_device_execution_experience_count, 0);
    assert_eq!(report.runtime_layer_mode_experience_count, 0);
    assert_eq!(report.runtime_all_layer_mode_experience_count, 0);
    assert_eq!(report.runtime_global_layers, 0);
    assert_eq!(report.runtime_local_window_layers, 0);
    assert_eq!(report.runtime_convolutional_fusion_layers, 0);
    assert_eq!(report.runtime_kv_import_experience_count, 0);
    assert_eq!(report.runtime_kv_export_experience_count, 0);
    assert!(!gate_report.passed());
    assert!(
        gate_report
            .failures
            .contains(&"runtime_model_experience_count 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_token_count 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_uncertainty_token_count 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_architecture_experience_count 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_adapter_experience_count 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_kv_export_experience_count 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_device_execution_experience_count 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_layer_mode_experience_count 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_all_layer_mode_experience_count 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_global_layers 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_local_window_layers 0 below required 1".to_owned())
    );
    assert!(
        gate_report
            .failures
            .contains(&"runtime_convolutional_fusion_layers 0 below required 1".to_owned())
    );
}
