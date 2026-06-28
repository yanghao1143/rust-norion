use super::*;

#[test]
fn state_inspection_matrix_gate_requires_every_explicit_device_to_pass() {
    let passing = StateInspectionGateReport {
        passed: true,
        failures: Vec::new(),
    };
    let failing = StateInspectionGateReport {
        passed: false,
        failures: vec!["runtime_kv_memory_count 0 below required 1".to_owned()],
    };

    let complete = StateInspectionMatrixGateReport::evaluate(
        DeviceClass::explicit_profiles()
            .iter()
            .copied()
            .map(|device| {
                StateInspectionDeviceGateReport::new(device, passing.clone())
                    .with_runtime_evidence(1, 1, 1, 1, 1, 1, 1, 1)
                    .with_reflection_evidence(1, 1, 1)
                    .with_live_memory_feedback_evidence(1, 2)
                    .with_live_memory_feedback_detail_evidence(1, 2, 0, 0, 0.20)
            })
            .collect(),
    );

    assert!(complete.passed(), "{:?}", complete.failures);
    assert_eq!(
        complete.covered_devices(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(complete.missing_devices().is_empty());
    assert!(complete.failed_devices().is_empty());
    assert!(complete
        .summary_line()
        .contains("state_inspection_matrix_gate: passed=true"));
    assert!(complete
        .summary_line()
        .contains("runtime_device_execution_device_profiles=12"));
    assert!(complete
        .summary_line()
        .contains("live_memory_feedback_device_profiles=12"));

    let incomplete = StateInspectionMatrixGateReport::evaluate(vec![
        StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing)
            .with_runtime_evidence(1, 1, 1, 1, 1, 1, 1, 1)
            .with_reflection_evidence(1, 1, 1)
            .with_live_memory_feedback_evidence(1, 2)
            .with_live_memory_feedback_detail_evidence(1, 2, 0, 0, 0.20),
        StateInspectionDeviceGateReport::new(DeviceClass::IntegratedGpu, failing),
    ]);

    assert!(!incomplete.passed());
    assert_eq!(incomplete.covered_devices(), 2);
    assert_eq!(
        incomplete.missing_devices().len(),
        DeviceClass::explicit_profiles().len() - 2
    );
    assert_eq!(
        incomplete.failed_devices(),
        vec![DeviceClass::IntegratedGpu]
    );
    assert!(incomplete
        .failures
        .iter()
        .any(|failure| failure.contains("missing=")));
    assert!(incomplete
        .failures
        .iter()
        .any(|failure| failure.contains("device integrated state inspection failed")));
}

#[test]
fn state_inspection_matrix_gate_can_require_reflection_evidence_per_device() {
    let passing = StateInspectionGateReport {
        passed: true,
        failures: Vec::new(),
    };
    let gate = StateInspectionMatrixGate {
        min_reflection_issue_device_profiles: Some(2),
        min_critical_reflection_issue_device_profiles: Some(1),
        min_revision_action_device_profiles: Some(2),
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
                        device_report = device_report.with_reflection_evidence(1, 1, 1);
                    }
                    DeviceClass::IntegratedGpu => {
                        device_report = device_report.with_reflection_evidence(1, 0, 1);
                    }
                    _ => {}
                }
                device_report
            })
            .collect(),
        &gate,
    );

    assert!(report.passed(), "{:?}", report.failures);
    assert_eq!(report.reflection_issue_device_profiles(), 2);
    assert_eq!(report.critical_reflection_issue_device_profiles(), 1);
    assert_eq!(report.revision_action_device_profiles(), 2);
    assert!(report
        .summary_line()
        .contains("reflection_issue_device_profiles=2"));

    let failing = StateInspectionMatrixGateReport::evaluate_with_gate(
        vec![
            StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing)
                .with_reflection_evidence(1, 0, 0),
        ],
        &gate,
    );

    assert!(!failing.passed());
    assert!(failing
        .failures
        .iter()
        .any(|failure| { failure == "reflection_issue_device_profiles 1 below required 2" }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "critical_reflection_issue_device_profiles 0 below required 1"
    }));
    assert!(failing
        .failures
        .iter()
        .any(|failure| { failure == "revision_action_device_profiles 0 below required 2" }));
}

#[test]
fn state_inspection_matrix_gate_can_require_live_memory_feedback_per_device() {
    let passing = StateInspectionGateReport {
        passed: true,
        failures: Vec::new(),
    };
    let gate = StateInspectionMatrixGate {
        min_live_memory_feedback_device_profiles: Some(2),
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
                        device_report = device_report
                            .with_live_memory_feedback_evidence(1, 2)
                            .with_live_memory_feedback_detail_evidence(1, 2, 0, 0, 0.20);
                    }
                    DeviceClass::IntegratedGpu => {
                        device_report = device_report
                            .with_live_memory_feedback_evidence(2, 4)
                            .with_live_memory_feedback_detail_evidence(2, 3, 1, 1, 0.40);
                    }
                    _ => {}
                }
                device_report
            })
            .collect(),
        &gate,
    );

    assert!(report.passed(), "{:?}", report.failures);
    assert_eq!(report.live_memory_feedback_device_profiles(), 2);
    assert!(report
        .summary_line()
        .contains("live_memory_feedback_device_profiles=2"));

    let failing = StateInspectionMatrixGateReport::evaluate_with_gate(
        vec![
            StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing)
                .with_live_memory_feedback_evidence(1, 0),
        ],
        &gate,
    );

    assert!(!failing.passed());
    assert!(failing
        .failures
        .iter()
        .any(|failure| { failure == "live_memory_feedback_device_profiles 0 below required 2" }));
}

#[test]
fn state_inspection_matrix_gate_can_require_evolution_evidence_per_device() {
    let passing = StateInspectionGateReport {
        passed: true,
        failures: Vec::new(),
    };
    let gate = StateInspectionMatrixGate {
        min_evolution_live_inference_device_profiles: Some(2),
        min_evolution_live_router_threshold_mutation_device_profiles: Some(1),
        min_evolution_live_hierarchy_weight_mutation_device_profiles: Some(1),
        min_evolution_live_online_reward_device_profiles: Some(2),
        min_evolution_live_online_reward_strength_device_profiles: Some(2),
        min_evolution_live_memory_update_device_profiles: Some(2),
        min_evolution_live_stored_memory_update_device_profiles: Some(2),
        min_evolution_live_reflection_issue_device_profiles: Some(2),
        min_evolution_live_critical_reflection_issue_device_profiles: Some(1),
        min_evolution_live_revision_action_device_profiles: Some(2),
        min_evolution_replay_run_device_profiles: Some(2),
        min_evolution_replay_item_device_profiles: Some(2),
        min_evolution_router_threshold_mutation_device_profiles: Some(1),
        min_evolution_hierarchy_weight_mutation_device_profiles: Some(1),
        min_evolution_memory_update_device_profiles: Some(2),
        min_evolution_replay_live_memory_feedback_device_profiles: Some(2),
        min_evolution_replay_live_memory_feedback_detail_device_profiles: Some(2),
        min_evolution_replay_live_evolution_device_profiles: Some(2),
        min_evolution_replay_live_evolution_online_reward_device_profiles: Some(2),
        min_evolution_replay_live_evolution_online_reward_strength_device_profiles: Some(2),
        min_evolution_replay_live_evolution_memory_update_device_profiles: Some(2),
        min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles: Some(1),
        min_evolution_replay_live_evolution_revision_action_device_profiles: Some(2),
        min_evolution_recursive_replay_device_profiles: Some(1),
        min_evolution_recursive_runtime_call_device_profiles: Some(1),
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
                        device_report = device_report
                            .with_live_evolution_evidence(1, 1, 1, 3, 2, 1, 1, 1)
                            .with_live_evolution_online_reward_evidence(2, 1, 1, 1.0, 0.6, 0.4)
                            .with_evolution_evidence(1, 2, 1, 1, 3, 2, 1, 1)
                            .with_evolution_replay_live_memory_feedback_detail_evidence(
                                1, 1, 0, 1, 0.2,
                            )
                            .with_evolution_replay_live_evolution_evidence(1, 2, 1, 1, 1, 1)
                            .with_evolution_replay_live_evolution_online_reward_evidence(
                                2, 1, 1, 0.8, 0.5, 0.3,
                            );
                    }
                    DeviceClass::IntegratedGpu => {
                        device_report = device_report
                            .with_live_evolution_evidence(1, 0, 0, 2, 1, 1, 0, 1)
                            .with_live_evolution_online_reward_evidence(1, 1, 0, 0.7, 0.7, 0.0)
                            .with_evolution_evidence(1, 1, 0, 0, 2, 1, 0, 0)
                            .with_evolution_replay_live_memory_feedback_detail_evidence(
                                1, 1, 0, 0, 0.1,
                            )
                            .with_evolution_replay_live_evolution_evidence(1, 1, 0, 0, 0, 1)
                            .with_evolution_replay_live_evolution_online_reward_evidence(
                                1, 0, 1, 0.4, 0.0, 0.4,
                            );
                    }
                    _ => {}
                }
                device_report
            })
            .collect(),
        &gate,
    );

    assert!(report.passed(), "{:?}", report.failures);
    assert_eq!(report.evolution_live_inference_device_profiles(), 2);
    assert_eq!(
        report.evolution_live_router_threshold_mutation_device_profiles(),
        1
    );
    assert_eq!(
        report.evolution_live_hierarchy_weight_mutation_device_profiles(),
        1
    );
    assert_eq!(report.evolution_live_online_reward_device_profiles(), 2);
    assert_eq!(
        report.evolution_live_online_reward_strength_device_profiles(),
        2
    );
    assert_eq!(report.evolution_live_memory_update_device_profiles(), 2);
    assert_eq!(
        report.evolution_live_stored_memory_update_device_profiles(),
        2
    );
    assert_eq!(report.evolution_live_reflection_issue_device_profiles(), 2);
    assert_eq!(
        report.evolution_live_critical_reflection_issue_device_profiles(),
        1
    );
    assert_eq!(report.evolution_live_revision_action_device_profiles(), 2);
    assert_eq!(report.evolution_replay_run_device_profiles(), 2);
    assert_eq!(report.evolution_replay_item_device_profiles(), 2);
    assert_eq!(
        report.evolution_router_threshold_mutation_device_profiles(),
        1
    );
    assert_eq!(
        report.evolution_hierarchy_weight_mutation_device_profiles(),
        1
    );
    assert_eq!(report.evolution_memory_update_device_profiles(), 2);
    assert_eq!(
        report.evolution_replay_live_memory_feedback_device_profiles(),
        2
    );
    assert_eq!(
        report.evolution_replay_live_memory_feedback_detail_device_profiles(),
        2
    );
    assert_eq!(report.evolution_replay_live_evolution_device_profiles(), 2);
    assert_eq!(
        report.evolution_replay_live_evolution_online_reward_device_profiles(),
        2
    );
    assert_eq!(
        report.evolution_replay_live_evolution_online_reward_strength_device_profiles(),
        2
    );
    assert_eq!(
        report.evolution_replay_live_evolution_memory_update_device_profiles(),
        2
    );
    assert_eq!(
        report.evolution_replay_live_evolution_critical_reflection_issue_device_profiles(),
        1
    );
    assert_eq!(
        report.evolution_replay_live_evolution_revision_action_device_profiles(),
        2
    );
    assert_eq!(report.evolution_recursive_replay_device_profiles(), 1);
    assert_eq!(report.evolution_recursive_runtime_call_device_profiles(), 1);
    assert!(report
        .summary_line()
        .contains("evolution_memory_update_device_profiles=2"));
    assert!(report
        .summary_line()
        .contains("evolution_live_inference_device_profiles=2"));
    assert!(report
        .summary_line()
        .contains("evolution_live_online_reward_strength_device_profiles=2"));
    assert!(report
        .summary_line()
        .contains("evolution_live_critical_reflection_issue_device_profiles=1"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_memory_feedback_device_profiles=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_memory_feedback_detail_device_profiles=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_device_profiles=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_online_reward_strength_device_profiles=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_memory_update_device_profiles=2"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_critical_reflection_issue_device_profiles=1"));
    assert!(report
        .summary_line()
        .contains("evolution_replay_live_evolution_revision_action_device_profiles=2"));

    let failing = StateInspectionMatrixGateReport::evaluate_with_gate(
        vec![
            StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing)
                .with_live_evolution_evidence(1, 0, 0, 0, 0, 0, 0, 0)
                .with_live_evolution_online_reward_evidence(1, 1, 0, 0.0, 0.0, 0.0)
                .with_evolution_evidence(1, 0, 0, 0, 0, 0, 0, 0),
        ],
        &gate,
    );

    assert!(!failing.passed());
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_live_inference_device_profiles 1 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_live_memory_update_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_live_online_reward_strength_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_live_stored_memory_update_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_live_reflection_issue_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_live_critical_reflection_issue_device_profiles 0 below required 1"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_live_revision_action_device_profiles 0 below required 2"
    }));
    assert!(failing
        .failures
        .iter()
        .any(|failure| { failure == "evolution_replay_run_device_profiles 1 below required 2" }));
    assert!(failing
        .failures
        .iter()
        .any(|failure| { failure == "evolution_replay_item_device_profiles 0 below required 2" }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_router_threshold_mutation_device_profiles 0 below required 1"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_hierarchy_weight_mutation_device_profiles 0 below required 1"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_memory_update_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_replay_live_memory_feedback_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_replay_live_memory_feedback_detail_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_replay_live_evolution_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure
            == "evolution_replay_live_evolution_online_reward_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
            failure
                == "evolution_replay_live_evolution_online_reward_strength_device_profiles 0 below required 2"
        }));
    assert!(failing.failures.iter().any(|failure| {
        failure
            == "evolution_replay_live_evolution_memory_update_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
            failure
                == "evolution_replay_live_evolution_critical_reflection_issue_device_profiles 0 below required 1"
        }));
    assert!(failing.failures.iter().any(|failure| {
        failure
            == "evolution_replay_live_evolution_revision_action_device_profiles 0 below required 2"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_recursive_replay_device_profiles 0 below required 1"
    }));
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_recursive_runtime_call_device_profiles 0 below required 1"
    }));
}
