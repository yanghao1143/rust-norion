use std::path::{Path, PathBuf};

use rust_norion::{
    BenchmarkCase, BenchmarkGateReport, BenchmarkSummary, DeviceClass, ExperienceInput,
    GenerationMetrics, HardwareSnapshot, HierarchyAdjustmentPreviewPlanner, HierarchyWeights,
    InferenceBackend, NoironEngine, ProcessRewardComponents, ProcessRewardReport,
    ProductionKernelConformanceDeviceReport, ProductionKernelConformanceGate,
    ProductionKernelConformanceMatrixReport, ProductionKernelConformanceReport, ReflectionIssue,
    ReflectionSeverity, RewardAction, RouteBudget, RouterThresholdAdjustmentPreviewPlanner,
    RuntimeBackend, RuntimeDiagnostics, SelfEvolutionAdmissionEvidence, SelfEvolutionAdmissionGate,
    SelfEvolutionAdmissionReport, TaskProfile, default_benchmark_cases, split,
};

use crate::Args;
use crate::engine_config::configure_engine;
use crate::inference_runner::run_timed_inference;

pub(crate) fn run_benchmark<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    trace_path: &PathBuf,
) -> std::io::Result<BenchmarkSummary> {
    let mut summary = BenchmarkSummary::new();
    seed_sparse_benchmark_memories(engine);
    seed_auto_replay_benchmark_experience(engine);

    for case in default_benchmark_cases() {
        let timed = run_timed_inference(
            engine,
            backend,
            case.prompt.clone(),
            case.profile,
            Some(trace_path),
            Some(&case.name),
        )?;
        summary.record(&case, timed.elapsed_ms, &timed.outcome);
    }

    Ok(summary)
}

pub(crate) fn run_benchmark_for_args<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    args: &Args,
    trace_path: &PathBuf,
) -> std::io::Result<BenchmarkSummary> {
    if args.benchmark_all_devices {
        run_benchmark_all_devices(engine, backend, args, trace_path)
    } else {
        run_benchmark(engine, backend, trace_path)
    }
}

fn run_benchmark_all_devices<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    args: &Args,
    trace_path: &PathBuf,
) -> std::io::Result<BenchmarkSummary> {
    let mut summary = BenchmarkSummary::new();
    seed_sparse_benchmark_memories(engine);
    seed_auto_replay_benchmark_experience(engine);

    for device in DeviceClass::explicit_profiles() {
        engine.set_hardware_snapshot(HardwareSnapshot::new(
            *device,
            args.cpu_load,
            args.gpu_load,
            args.ram_load,
            args.disk_load,
        ));

        for case in default_benchmark_cases() {
            let case_name = format!("{}_{}", device.as_str(), case.name);
            let timed = run_timed_inference(
                engine,
                backend,
                case.prompt.clone(),
                case.profile,
                Some(trace_path),
                Some(&case_name),
            )?;
            let recorded_case = BenchmarkCase::new(case_name, case.profile, case.prompt);
            summary.record(&recorded_case, timed.elapsed_ms, &timed.outcome);
        }
    }

    configure_engine(engine, args);

    Ok(summary)
}

pub(crate) fn run_production_benchmark_all_devices(
    engine: &mut NoironEngine,
    args: &Args,
    trace_path: &PathBuf,
) -> std::io::Result<BenchmarkSummary> {
    let mut summary = BenchmarkSummary::new();
    seed_sparse_benchmark_memories(engine);
    seed_auto_replay_benchmark_experience(engine);

    for device in DeviceClass::explicit_profiles() {
        engine.set_hardware_snapshot(HardwareSnapshot::new(
            *device,
            args.cpu_load,
            args.gpu_load,
            args.ram_load,
            args.disk_load,
        ));

        for case in default_benchmark_cases() {
            let case_name = format!("{}_{}", device.as_str(), case.name);
            let runtime = args.production_runtime_for_case(*device, case.profile, &case.prompt)?;
            let mut backend = RuntimeBackend::new(runtime);
            let timed = run_timed_inference(
                engine,
                &mut backend,
                case.prompt.clone(),
                case.profile,
                Some(trace_path),
                Some(&case_name),
            )?;
            let recorded_case = BenchmarkCase::new(case_name, case.profile, case.prompt);
            summary.record(&recorded_case, timed.elapsed_ms, &timed.outcome);
        }
    }

    configure_engine(engine, args);

    Ok(summary)
}

pub(crate) fn run_production_kernel_conformance_all_devices(
    args: &Args,
) -> ProductionKernelConformanceMatrixReport {
    let manifest = args.runtime_manifest();
    let device_reports = DeviceClass::explicit_profiles()
        .iter()
        .copied()
        .map(|device| {
            let report = match args.production_runtime_for_case(device, args.profile, &args.prompt)
            {
                Ok(runtime) => {
                    runtime.conformance_report(ProductionKernelConformanceGate::default())
                }
                Err(error) => ProductionKernelConformanceReport::failed(
                    manifest.metadata.model_id.clone(),
                    "none",
                    false,
                    format!("production runtime construction failed: {error}"),
                ),
            };
            ProductionKernelConformanceDeviceReport { device, report }
        })
        .collect();

    ProductionKernelConformanceMatrixReport::evaluate(device_reports)
}

fn seed_sparse_benchmark_memories(engine: &mut NoironEngine) {
    if engine
        .cache
        .entries()
        .iter()
        .any(|entry| entry.key.starts_with("benchmark_sparse_seed:long_context:"))
    {
        return;
    }

    for index in 0..28 {
        let key = format!(
            "benchmark_sparse_seed:long_context:{index}: FHT-DKE Noiron disk KV sparse context memory routing compression"
        );
        let vector = sparse_benchmark_vector(index);
        let id = engine.cache.store_or_fuse(key, vector, 1.0);
        engine.cache.reinforce(id, 1.0);
    }
}

fn seed_auto_replay_benchmark_experience(engine: &mut NoironEngine) {
    const SEED_PREFIX: &str = "benchmark_auto_replay_seed:v3:control_plane:";

    if engine
        .experience
        .records()
        .iter()
        .any(|record| record.lesson.starts_with(SEED_PREFIX))
    {
        return;
    }

    let reinforce_memory_id = engine.cache.store_or_fuse(
        "benchmark_auto_replay_seed:control_plane: reinforced memory",
        vec![0.70, 0.20, 0.10, 0.40],
        0.82,
    );
    let penalize_memory_id = engine.cache.store_or_fuse(
        "benchmark_auto_replay_seed:control_plane: penalized memory",
        vec![0.10, 0.60, 0.30, 0.20],
        0.42,
    );

    engine.experience.record(ExperienceInput {
        prompt: "benchmark auto replay control plane".to_owned(),
        profile: TaskProfile::Coding,
        lesson:
            "benchmark_auto_replay_seed:v3:control_plane: reinforce router threshold and hierarchy"
                .to_owned(),
        quality: 1.0,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: Some(reinforce_memory_id),
        router_threshold_after: 0.52,
        stream_windows: 0,
        route_budget: RouteBudget {
            threshold: 0.52,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: vec![reinforce_memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics::default(),
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 1.0,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Reinforce,
            notes: vec![
                "memory_feedback:reinforced=2:penalized=0:reinforcement_amount=1.600000:penalty_amount=0.000000"
                    .to_owned(),
            ],
        },
        live_evolution: Default::default(),
    });
    engine.experience.record(ExperienceInput {
        prompt: "benchmark auto replay control plane drift".to_owned(),
        profile: TaskProfile::Coding,
        lesson:
            "benchmark_auto_replay_seed:v3:control_plane: penalize weak router hierarchy memory"
                .to_owned(),
        quality: 0.0,
        contradictions: vec!["benchmark contradiction".to_owned()],
        reflection_issues: vec![ReflectionIssue::new(
            "benchmark_seed_low_quality",
            ReflectionSeverity::Warning,
            "auto-replay seed records an inspected weak control path",
        )],
        revision_actions: vec!["regenerate".to_owned()],
        stored_memory_id: Some(penalize_memory_id),
        router_threshold_after: 0.52,
        stream_windows: 4,
        route_budget: RouteBudget {
            threshold: 0.52,
            attention_tokens: 1,
            fast_tokens: 3,
            attention_fraction: 0.25,
        },
        hierarchy: HierarchyWeights::new(0.24, 0.58, 0.18),
        used_memory_ids: vec![penalize_memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics::default(),
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.0,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Penalize,
            notes: vec![
                "memory_feedback:reinforced=0:penalized=1:reinforcement_amount=0.000000:penalty_amount=0.800000"
                    .to_owned(),
            ],
        },
        live_evolution: Default::default(),
    });
    engine.experience.record(ExperienceInput {
        prompt: "runtime adapter self-developed Transformer KV import export".to_owned(),
        profile: TaskProfile::Coding,
        lesson:
            "benchmark_auto_replay_seed:v3:control_plane: prefer portable runtime adapter from prior KV experience"
                .to_owned(),
        quality: 0.94,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: Some(reinforce_memory_id),
        router_threshold_after: 0.52,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.52,
            attention_tokens: 2,
            fast_tokens: 1,
            attention_fraction: 0.67,
        },
        hierarchy: HierarchyWeights::new(0.22, 0.58, 0.20),
        used_memory_ids: vec![reinforce_memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: None,
            selected_adapter: Some("portable-rust".to_owned()),
            layer_count: 6,
            hidden_size: 64,
            local_window_tokens: 32,
            forward_energy: Some(0.18),
            kv_influence: Some(0.72),
            imported_kv_blocks: 2,
            exported_kv_blocks: 1,
            ..RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.90,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Reinforce,
            notes: vec![
                "memory_feedback:reinforced=1:penalized=0:reinforcement_amount=0.700000:penalty_amount=0.000000"
                    .to_owned(),
            ],
        },
        live_evolution: Default::default(),
    });
}

fn sparse_benchmark_vector(index: usize) -> Vec<f32> {
    let mut vector = vec![0.0; 64];
    vector[index % 64] = 1.0;
    vector[(index * 7 + 3) % 64] = 0.45;
    vector[(index * 13 + 11) % 64] = 0.25;
    vector
}

pub(crate) fn print_benchmark_summary(
    args: &Args,
    benchmark_path: &Path,
    summary: &BenchmarkSummary,
    gate_report: Option<&BenchmarkGateReport>,
    self_evolution_admission_report: Option<&SelfEvolutionAdmissionReport>,
) {
    println!("Noiron Rust benchmark");
    println!("benchmark_file: {}", benchmark_path.display());
    println!("memory_file: {}", args.memory_path.display());
    println!("experience_file: {}", args.experience_path.display());
    println!("adaptive_file: {}", args.adaptive_path.display());
    println!("benchmark_all_devices: {}", args.benchmark_all_devices);
    println!("{}", summary.summary_line());
    let reflection = summary.reflection_evidence();
    println!(
        "reflection_evidence: issue_cases={} issues={} issue_device_profiles={} critical_issue_cases={} critical_issues={} critical_issue_device_profiles={} revision_action_cases={} revision_actions={} revision_action_device_profiles={}",
        reflection.issue_cases,
        reflection.total_issues,
        reflection.issue_device_profiles(),
        reflection.critical_issue_cases,
        reflection.total_critical_issues,
        reflection.critical_issue_device_profiles(),
        reflection.revision_action_cases,
        reflection.total_revision_actions,
        reflection.revision_action_device_profiles()
    );

    for result in summary.results() {
        println!(
            "case={} profile={:?} device={} elapsed_ms={} quality={:.3} reward={:.3} attention_fraction={:.2} requires_recursion={} chunks={} waves={} recursive_runtime_calls={} auto_replay_applied={} auto_replay_router_updates={} auto_replay_hierarchy_updates={} auto_replay_router_threshold_mutations={} auto_replay_hierarchy_weight_mutations={} auto_replay_router_threshold_delta={:.6} auto_replay_hierarchy_weight_delta={:.6} auto_replay_memory_reinforcements={} auto_replay_memory_penalties={} auto_replay_live_memory_feedback_items={} auto_replay_live_memory_feedback_updates={} auto_replay_live_memory_feedback_reinforcements={} auto_replay_live_memory_feedback_penalties={} auto_replay_live_memory_feedback_detail_items={} auto_replay_live_memory_feedback_applied={} auto_replay_live_memory_feedback_removed={} auto_replay_live_memory_feedback_missing={} auto_replay_live_memory_feedback_strength_delta={:.6} auto_replay_recursive_items={} auto_replay_recursive_runtime_calls={} auto_replay_avg_recursive_call_pressure={:.3} auto_replay_max_recursive_call_pressure={:.3} used_memories={} infini_local_window={} infini_global_memory={} sparse_skipped={} sparse_skipped_tokens={} stored_memories={} compacted_memories={} runtime_forward_signal={} runtime_forward_energy_signal={} runtime_kv_influence_signal={} runtime_token_count={} runtime_uncertainty_tokens={} runtime_uncertainty_signal={} runtime_kv_imported={} runtime_kv_exported={} runtime_kv_stored={} runtime_selected_adapter={} runtime_adapter_contract_ok={} runtime_adapter_contract_violations={} runtime_adapter_observations={} runtime_adapter_best_score={} runtime_adapter_best_adapter={} runtime_adapter_selection_mismatches={} drift={}",
            result.name,
            result.profile,
            result.device.as_str(),
            result.elapsed_ms,
            result.quality,
            result.process_reward,
            result.attention_fraction,
            result.requires_recursion,
            result.recursive_chunks,
            result.recursive_waves,
            result.recursive_runtime_calls,
            result.auto_replay_applied,
            result.auto_replay_router_updates,
            result.auto_replay_hierarchy_updates,
            result.auto_replay_router_threshold_mutations,
            result.auto_replay_hierarchy_weight_mutations,
            result.auto_replay_router_threshold_delta,
            result.auto_replay_hierarchy_weight_delta,
            result.auto_replay_memory_reinforcements,
            result.auto_replay_memory_penalties,
            result.auto_replay_live_memory_feedback_items,
            result.auto_replay_live_memory_feedback_updates,
            result.auto_replay_live_memory_feedback_reinforcements,
            result.auto_replay_live_memory_feedback_penalties,
            result.auto_replay_live_memory_feedback_detail_items,
            result.auto_replay_live_memory_feedback_applied,
            result.auto_replay_live_memory_feedback_removed,
            result.auto_replay_live_memory_feedback_missing,
            result.auto_replay_live_memory_feedback_strength_delta,
            result.auto_replay_recursive_runtime_items,
            result.auto_replay_recursive_runtime_calls,
            result.auto_replay_avg_recursive_call_pressure,
            result.auto_replay_max_recursive_call_pressure,
            result.used_memories,
            result.infini_local_window,
            result.infini_global_memory,
            result.sparse_skipped,
            result.sparse_skipped_tokens,
            result.stored_memories,
            result.compacted_memories,
            result.runtime_forward_signal,
            result.runtime_forward_energy_signal,
            result.runtime_kv_influence_signal,
            result.runtime_token_count,
            result.runtime_uncertainty_token_count,
            result.runtime_uncertainty_signal,
            result.runtime_kv_imported,
            result.runtime_kv_exported,
            result.runtime_kv_stored,
            option_text(result.runtime_selected_adapter.as_deref()),
            result.runtime_adapter_contract_ok,
            result.runtime_adapter_contract_violations,
            result.runtime_adapter_observations,
            option_f32_text(result.runtime_adapter_best_score),
            option_text(result.runtime_adapter_best_adapter.as_deref()),
            result.runtime_adapter_selection_mismatches,
            result.drift_severity.as_str()
        );
    }

    if let Some(report) = gate_report {
        println!("{}", report.summary_line());
        for failure in &report.failures {
            println!("benchmark_gate_failure: {failure}");
        }
    }

    if let Some(report) = self_evolution_admission_report {
        println!("{}", report.summary_line());
        for reason in &report.blocked_reasons {
            println!("self_evolution_admission_blocked_reason: {reason}");
        }
    }
}

pub(crate) fn benchmark_self_evolution_admission_report(
    candidate_id: impl Into<String>,
    engine: &NoironEngine,
    summary: &BenchmarkSummary,
    gate_report: &BenchmarkGateReport,
    fallback_profile: TaskProfile,
) -> SelfEvolutionAdmissionReport {
    let profile = benchmark_admission_profile(summary, fallback_profile);
    let metrics = benchmark_admission_generation_metrics(summary);
    let router_preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        engine.router.state(),
        profile,
        metrics,
    );
    let hierarchy_preview = HierarchyAdjustmentPreviewPlanner::new().preview(
        engine.hierarchy.state(),
        profile,
        metrics,
    );

    let mut evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        candidate_id,
        summary.evolution_ledger(),
        gate_report,
    )
    .with_router_threshold_preview_report(&router_preview)
    .with_hierarchy_adjustment_preview_report(&hierarchy_preview);

    if let Some(kv_policy_preview) = benchmark_kv_fusion_policy_preview(summary) {
        evidence = evidence.with_kv_fusion_policy_observation_preview_report(&kv_policy_preview);
    }

    SelfEvolutionAdmissionGate::new().evaluate(&evidence)
}

fn benchmark_kv_fusion_policy_preview(
    summary: &BenchmarkSummary,
) -> Option<split::bridge::KvFusionPolicyObservationDryRunReport> {
    let runtime_kv_exported = summary.total_runtime_kv_exported();
    let runtime_kv_stored = summary.total_runtime_kv_stored();
    let runtime_kv_imported = summary.total_runtime_kv_imported();
    let runtime_kv_held = summary.total_runtime_kv_held();
    let runtime_kv_total = runtime_kv_exported
        .saturating_add(runtime_kv_stored)
        .saturating_add(runtime_kv_imported)
        .saturating_add(runtime_kv_held);

    if runtime_kv_total == 0 {
        return None;
    }

    let quality = finite_clamped(summary.average_quality(), 0.0, 1.0);
    let reward = finite_clamped(summary.average_reward(), 0.0, 1.0);
    let accepted = quality >= 0.50 && reward >= 0.50;
    let amount = if accepted {
        ((quality + reward) * 0.5).clamp(0.05, 1.0)
    } else {
        (1.0 - ((quality + reward) * 0.5)).clamp(0.05, 1.0)
    };
    let action = if accepted {
        split::agent::AgentRecallOutcomeAttributionAction::Reinforce
    } else {
        split::agent::AgentRecallOutcomeAttributionAction::Penalize
    };
    let attribution = split::agent::AgentRecallOutcomeAttribution {
        task_id: "benchmark_runtime_kv".to_owned(),
        record_id: format!(
            "runtime_kv:benchmark:exported={runtime_kv_exported}:stored={runtime_kv_stored}:imported={runtime_kv_imported}:held={runtime_kv_held}"
        ),
        source: "runtime_kv".to_owned(),
        action,
        amount,
        reason_codes: vec![
            format!("runtime_kv_exported={runtime_kv_exported}"),
            format!("runtime_kv_stored={runtime_kv_stored}"),
            format!("runtime_kv_imported={runtime_kv_imported}"),
            format!("runtime_kv_held={runtime_kv_held}"),
            format!("benchmark_average_quality={quality:.3}"),
            format!("benchmark_average_reward={reward:.3}"),
        ],
    };
    let recall_report = split::agent::AgentRecallOutcomeAttributionReport {
        attributions: vec![attribution],
        reinforced_count: usize::from(accepted),
        penalized_count: usize::from(!accepted),
        skipped_rejected_recall_count: 0,
        skipped_missing_outcome_task_ids: Vec::new(),
        read_only: true,
        memory_store_write_allowed: false,
        telemetry: Vec::new(),
    };
    let reward_preview =
        split::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview(&recall_report);

    Some(split::bridge::kv_fusion_reward_policy_observation_dry_run(
        &reward_preview,
        split::core::ReinforcedKvFusionPolicy::default(),
    ))
}

fn benchmark_admission_generation_metrics(summary: &BenchmarkSummary) -> GenerationMetrics {
    let quality = finite_clamped(summary.average_quality(), 0.0, 1.0);
    let reward = finite_clamped(summary.average_reward(), 0.0, 1.0);
    let semantic_consistency = ((quality + reward) * 0.5).clamp(0.0, 1.0);
    let perplexity = (12.0 * (1.0 - semantic_consistency)).max(0.1);
    let reflection = summary.reflection_evidence();
    let contradiction_count = reflection.total_critical_issues;
    let token_count = summary.total_runtime_tokens().max(summary.len()).max(1);

    GenerationMetrics {
        perplexity,
        semantic_consistency,
        contradiction_count,
        token_count,
    }
}

fn benchmark_admission_profile(
    summary: &BenchmarkSummary,
    fallback_profile: TaskProfile,
) -> TaskProfile {
    let mut counts: Vec<(TaskProfile, usize)> = Vec::new();

    for result in summary.results() {
        if let Some((_, count)) = counts
            .iter_mut()
            .find(|(profile, _)| *profile == result.profile)
        {
            *count += 1;
        } else {
            counts.push((result.profile, 1));
        }
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(profile, _)| profile)
        .unwrap_or(fallback_profile)
}

fn finite_clamped(value: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}

pub(crate) fn print_production_kernel_conformance_report(
    report: &ProductionKernelConformanceReport,
) {
    println!("Noiron production kernel conformance gate");
    println!("{}", report.summary_line());
    for failure in &report.failures {
        println!("production_kernel_conformance_failure: {failure}");
    }
}

pub(crate) fn print_production_kernel_conformance_matrix_report(
    report: &ProductionKernelConformanceMatrixReport,
) {
    println!("Noiron production kernel conformance all-devices gate");
    println!("{}", report.summary_line());
    for device_report in &report.device_reports {
        println!(
            "production_kernel_conformance_device: device={} {}",
            device_report.device.as_str(),
            device_report.report.summary_line()
        );
        for failure in &device_report.report.failures {
            println!(
                "production_kernel_conformance_device_failure: {}: {}",
                device_report.device.as_str(),
                failure
            );
        }
    }
    for failure in &report.failures {
        println!("production_kernel_conformance_matrix_failure: {failure}");
    }
}

fn option_text(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn option_f32_text(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "none".to_owned())
}
