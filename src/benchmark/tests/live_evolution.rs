use super::*;
use crate::engine::{HeuristicBackend, InferenceRequest, NoironEngine};

#[test]
fn gate_reports_missing_live_evolution_device_profile_coverage() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.35,
        0.00,
        0.45,
        0.20,
    ));
    let mut backend = HeuristicBackend;
    let case = BenchmarkCase::new(
        "live_evolution_cpu",
        TaskProfile::General,
        "Explain why live Noiron inference should update local memory and reflection state.",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();
    summary.record(&case, 3, &outcome);

    let mut gate = BenchmarkGate::default();
    gate.min_evolution_live_inference_device_profiles = Some(2);

    let failing = summary.evaluate(&gate);

    assert!(!failing.passed);
    assert_eq!(
        summary
            .live_evolution_evidence()
            .inference_device_profiles(),
        1
    );
    assert!(failing.failures.iter().any(|failure| {
        failure == "evolution_live_inference_device_profiles 1 below minimum 2"
    }));

    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::IntegratedGpu,
        0.35,
        0.30,
        0.45,
        0.20,
    ));
    let gpu_case = BenchmarkCase::new(
        "live_evolution_integrated",
        TaskProfile::General,
        "Explain why live Noiron inference should preserve reflection and memory evidence.",
    );
    let gpu_outcome = engine.infer(
        InferenceRequest::new(gpu_case.prompt.clone(), gpu_case.profile),
        &mut backend,
    );
    summary.record(&gpu_case, 4, &gpu_outcome);
    gate.min_evolution_live_inference_device_profiles = Some(2);

    let passing = summary.evaluate(&gate);

    assert!(passing.passed, "{:?}", passing.failures);
    assert_eq!(
        summary
            .live_evolution_evidence()
            .inference_device_profiles(),
        2
    );
    assert!(
        summary
            .summary_line()
            .contains("evolution_live_inference_device_profiles=2")
    );
}

#[test]
fn gate_reports_missing_live_evolution_detail_device_profile_coverage() {
    let base_result =
        baseline_benchmark_result("live_detail", TaskProfile::General, DeviceClass::CpuOnly);
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence {
            inference_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
            router_threshold_mutation_devices: vec![DeviceClass::CpuOnly],
            hierarchy_weight_mutation_devices: vec![DeviceClass::CpuOnly],
            online_reward_devices: vec![DeviceClass::CpuOnly],
            online_reward_strength_devices: Vec::new(),
            memory_update_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
            stored_memory_update_devices: vec![DeviceClass::CpuOnly],
            reflection_issue_devices: vec![DeviceClass::CpuOnly],
            critical_reflection_issue_devices: vec![DeviceClass::CpuOnly],
            revision_action_devices: vec![DeviceClass::CpuOnly],
            replay_live_evolution_devices: Vec::new(),
            replay_live_evolution_online_reward_devices: Vec::new(),
            replay_live_evolution_online_reward_strength_devices: Vec::new(),
            replay_live_evolution_memory_update_devices: Vec::new(),
            replay_live_evolution_critical_reflection_issue_devices: Vec::new(),
            replay_live_evolution_revision_action_devices: Vec::new(),
        },
        routing_evidence: BenchmarkRoutingEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![base_result],
    };
    let gate = BenchmarkGate {
        min_average_quality: 0.5,
        min_average_reward: 0.45,
        min_evolution_live_inference_device_profiles: Some(2),
        min_evolution_live_router_threshold_mutation_device_profiles: Some(2),
        min_evolution_live_hierarchy_weight_mutation_device_profiles: Some(2),
        min_evolution_live_online_reward_device_profiles: Some(1),
        min_evolution_live_online_reward_strength_device_profiles: Some(2),
        min_evolution_live_memory_update_device_profiles: Some(2),
        min_evolution_live_stored_memory_update_device_profiles: Some(2),
        min_evolution_live_reflection_issue_device_profiles: Some(2),
        min_evolution_live_critical_reflection_issue_device_profiles: Some(2),
        min_evolution_live_revision_action_device_profiles: Some(2),
        ..BenchmarkGate::default()
    };

    let failing = summary.evaluate(&gate);

    assert!(!failing.passed);
    for marker in [
        "evolution_live_router_threshold_mutation_device_profiles",
        "evolution_live_hierarchy_weight_mutation_device_profiles",
        "evolution_live_online_reward_strength_device_profiles",
        "evolution_live_stored_memory_update_device_profiles",
        "evolution_live_reflection_issue_device_profiles",
        "evolution_live_critical_reflection_issue_device_profiles",
        "evolution_live_revision_action_device_profiles",
    ] {
        assert!(
            failing
                .failures
                .iter()
                .any(|failure| failure.contains(marker)),
            "missing failure marker {marker}: {:?}",
            failing.failures
        );
    }
    assert!(
        failing
            .failures
            .iter()
            .all(|failure| !failure.contains("evolution_live_inference_device_profiles"))
    );
    assert!(
        failing
            .failures
            .iter()
            .all(|failure| !failure.contains("evolution_live_memory_update_device_profiles"))
    );

    let passing = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence {
            router_threshold_mutation_devices: vec![
                DeviceClass::CpuOnly,
                DeviceClass::IntegratedGpu,
            ],
            hierarchy_weight_mutation_devices: vec![
                DeviceClass::CpuOnly,
                DeviceClass::IntegratedGpu,
            ],
            online_reward_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
            online_reward_strength_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
            stored_memory_update_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
            reflection_issue_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
            critical_reflection_issue_devices: vec![
                DeviceClass::CpuOnly,
                DeviceClass::IntegratedGpu,
            ],
            revision_action_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
            ..summary.live_evolution_evidence.clone()
        },
        ..summary
    };

    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert!(
        passing
            .summary_line()
            .contains("evolution_live_online_reward_device_profiles=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_live_online_reward_strength_device_profiles=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_live_revision_action_device_profiles=2")
    );
}

#[test]
fn gate_reports_missing_replay_live_evolution_device_profile_coverage() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let case = BenchmarkCase::new(
        "replay_live_evolution_device_coverage",
        TaskProfile::Coding,
        "Rust Noiron replay live evolution device coverage",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();
    summary.record(&case, 1, &outcome);
    summary.live_evolution_evidence = BenchmarkLiveEvolutionEvidence {
        replay_live_evolution_devices: vec![DeviceClass::CpuOnly],
        replay_live_evolution_online_reward_devices: vec![DeviceClass::CpuOnly],
        replay_live_evolution_online_reward_strength_devices: Vec::new(),
        replay_live_evolution_memory_update_devices: vec![DeviceClass::CpuOnly],
        ..BenchmarkLiveEvolutionEvidence::default()
    };
    let gate = BenchmarkGate {
        min_average_quality: 0.0,
        min_average_reward: 0.0,
        min_evolution_replay_live_evolution_device_profiles: Some(2),
        min_evolution_replay_live_evolution_online_reward_device_profiles: Some(1),
        min_evolution_replay_live_evolution_online_reward_strength_device_profiles: Some(2),
        min_evolution_replay_live_evolution_memory_update_device_profiles: Some(2),
        min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles: Some(2),
        min_evolution_replay_live_evolution_revision_action_device_profiles: Some(2),
        ..BenchmarkGate::default()
    };

    let failing = summary.evaluate(&gate);

    assert!(!failing.passed);
    for marker in [
        "evolution_replay_live_evolution_device_profiles",
        "evolution_replay_live_evolution_online_reward_strength_device_profiles",
        "evolution_replay_live_evolution_memory_update_device_profiles",
        "evolution_replay_live_evolution_critical_reflection_issue_device_profiles",
        "evolution_replay_live_evolution_revision_action_device_profiles",
    ] {
        assert!(
            failing
                .failures
                .iter()
                .any(|failure| failure.contains(marker)),
            "missing failure marker {marker}: {:?}",
            failing.failures
        );
    }

    let mut passing = summary.clone();
    passing.live_evolution_evidence = BenchmarkLiveEvolutionEvidence {
        replay_live_evolution_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
        replay_live_evolution_online_reward_devices: vec![
            DeviceClass::CpuOnly,
            DeviceClass::IntegratedGpu,
        ],
        replay_live_evolution_online_reward_strength_devices: vec![
            DeviceClass::CpuOnly,
            DeviceClass::IntegratedGpu,
        ],
        replay_live_evolution_memory_update_devices: vec![
            DeviceClass::CpuOnly,
            DeviceClass::IntegratedGpu,
        ],
        replay_live_evolution_critical_reflection_issue_devices: vec![
            DeviceClass::CpuOnly,
            DeviceClass::IntegratedGpu,
        ],
        replay_live_evolution_revision_action_devices: vec![
            DeviceClass::CpuOnly,
            DeviceClass::IntegratedGpu,
        ],
        ..BenchmarkLiveEvolutionEvidence::default()
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_device_profiles=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_online_reward_device_profiles=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_online_reward_strength_device_profiles=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_memory_update_device_profiles=2")
    );
    assert!(
        passing.summary_line().contains(
            "evolution_replay_live_evolution_critical_reflection_issue_device_profiles=2"
        )
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_revision_action_device_profiles=2")
    );
}
