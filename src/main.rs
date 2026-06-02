use std::env;
use std::path::PathBuf;
use std::time::Instant;

use rust_norion::{
    BenchmarkCase, BenchmarkGate, BenchmarkGateReport, BenchmarkSummary, CommandPromptMode,
    CommandRuntime, CommandWireFormat, DeviceClass, DevicePlanGateReport, ExperienceInput,
    GistLevel, HardwareAllocator, HardwarePlan, HardwareSnapshot, HeuristicBackend,
    HierarchyWeights, InferenceBackend, InferenceOutcome, InferenceRequest, KvQuantBenchmarkGate,
    KvQuantBenchmarkGateReport, KvQuantBenchmarkSummary, LocalTransformerRuntime,
    MemoryCompactionPolicy, MemoryRetentionPolicy, ModelRuntime, ModelRuntimeForwardKernel,
    NoironEngine, PersistentRoundtripDeviceReport, PersistentRoundtripInput,
    PersistentRoundtripMatrixReport, PersistentRoundtripReport, ProcessRewardComponents,
    ProcessRewardReport, ProductionKernelConformanceDeviceReport, ProductionKernelConformanceGate,
    ProductionKernelConformanceMatrixReport, ProductionKernelConformanceReport, RecursiveScheduler,
    ReferenceProductionForwardKernel, ReflectionIssue, ReflectionSeverity, RewardAction,
    RouteBudget, RuntimeAssetPaths, RuntimeBackend, RuntimeDiagnostics, RuntimeError,
    RuntimeManifest, RuntimeManifestDeviceGateReport, RuntimeManifestValidation, RuntimeMetadata,
    StateInspectionDeviceGateReport, StateInspectionGate, StateInspectionGateReport,
    StateInspectionMatrixGate, StateInspectionMatrixGateReport, StateInspectionReport, TaskProfile,
    TierMigrationAction, TraceSchemaGateReport, TransformerRuntimeArchitecture, append_trace_jsonl,
    append_trace_jsonl_with_case, default_benchmark_cases, evaluate_trace_schema_jsonl,
};

fn main() -> std::io::Result<()> {
    let args = Args::parse(env::args().skip(1).collect());
    if args.list_devices {
        print_device_matrix_and_exit();
    }
    if args.device_gate {
        let report = DevicePlanGateReport::evaluate();
        print_device_gate_report(&report);
        if !report.passed() {
            std::process::exit(2);
        }
        return Ok(());
    }
    if args.kv_quant_gate {
        let summary = KvQuantBenchmarkSummary::run_default();
        let gate_report = summary.evaluate(&args.kv_quant_gate());
        print_kv_quant_gate_report(&summary, &gate_report);
        if !gate_report.passed {
            std::process::exit(2);
        }
        return Ok(());
    }
    if args.runtime_manifest_gate {
        let manifest = args.runtime_manifest();
        let validation = manifest.validate_for_production();
        let device_gate = RuntimeManifestDeviceGateReport::evaluate(
            &manifest,
            &args.runtime_manifest_device_plan(),
        );
        let all_devices_gate = if args.runtime_manifest_all_devices_gate {
            Some(DevicePlanGateReport::evaluate_runtime_manifest(&manifest))
        } else {
            None
        };
        print_runtime_manifest_gate_report(
            &manifest,
            &validation,
            &device_gate,
            all_devices_gate.as_ref(),
        );
        if !validation.passed()
            || !device_gate.passed()
            || all_devices_gate
                .as_ref()
                .map(|report| !report.passed())
                .unwrap_or(false)
        {
            std::process::exit(2);
        }
        return Ok(());
    }
    if args.production_kernel_conformance_all_devices_gate {
        let report = run_production_kernel_conformance_all_devices(&args);
        print_production_kernel_conformance_matrix_report(&report);
        if !report.passed {
            std::process::exit(2);
        }
        return Ok(());
    }
    if args.production_kernel_conformance_gate {
        let runtime = args.production_runtime()?;
        let report = runtime.conformance_report(ProductionKernelConformanceGate::default());
        print_production_kernel_conformance_report(&report);
        if !report.passed {
            std::process::exit(2);
        }
        return Ok(());
    }
    if args.trace_schema_gate_path.is_some()
        && args.trace_path.is_none()
        && args.benchmark_path.is_none()
    {
        let path = args.trace_schema_gate_path.as_ref().unwrap();
        let report = evaluate_trace_schema_jsonl(path)?;
        print_trace_schema_gate_report(path, &report);
        if !report.passed {
            std::process::exit(2);
        }
        return Ok(());
    }

    if args.benchmark_roundtrip && args.inspect_state {
        if args.benchmark_all_devices {
            let roundtrip_report = run_persistent_roundtrip_all_devices(&args)?;
            print_persistent_roundtrip_matrix_report(&args, &roundtrip_report);
            let inspect_report = run_state_inspection_all_devices(&args)?;
            print_state_inspection_matrix_gate_report(&args, &inspect_report);
            if !roundtrip_report.passed || !inspect_report.passed() {
                std::process::exit(2);
            }
        } else {
            let roundtrip_report = run_persistent_roundtrip(&args)?;
            print_persistent_roundtrip_report(&args, &roundtrip_report);
            let inspect_report = run_state_inspection(&args)?;
            print_state_inspection_report(&args, &inspect_report);
            let inspect_passed = if args.inspect_gate {
                let gate_report = inspect_report.evaluate(&args.state_inspection_gate());
                print_state_inspection_gate_report(&gate_report);
                gate_report.passed()
            } else {
                true
            };
            if !roundtrip_report.passed || !inspect_passed {
                std::process::exit(2);
            }
        }
        return Ok(());
    }

    if args.inspect_state {
        if args.benchmark_all_devices {
            let report = run_state_inspection_all_devices(&args)?;
            print_state_inspection_matrix_gate_report(&args, &report);
            if !report.passed() {
                std::process::exit(2);
            }
        } else {
            let report = run_state_inspection(&args)?;
            print_state_inspection_report(&args, &report);
            if args.inspect_gate {
                let gate_report = report.evaluate(&args.state_inspection_gate());
                print_state_inspection_gate_report(&gate_report);
                if !gate_report.passed() {
                    std::process::exit(2);
                }
            }
        }
        return Ok(());
    }

    if args.benchmark_roundtrip {
        if args.benchmark_all_devices {
            let report = run_persistent_roundtrip_all_devices(&args)?;
            print_persistent_roundtrip_matrix_report(&args, &report);
            if !report.passed {
                std::process::exit(2);
            }
        } else {
            let report = run_persistent_roundtrip(&args)?;
            print_persistent_roundtrip_report(&args, &report);
            if !report.passed {
                std::process::exit(2);
            }
        }
        return Ok(());
    }

    let mut engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    configure_engine(&mut engine, &args);
    let replay_report = if args.replay_limit > 0 {
        Some(engine.replay_experience(args.replay_limit))
    } else {
        None
    };

    if let Some(benchmark_path) = args.benchmark_path.clone() {
        let summary = if args.production_runtime {
            if args.benchmark_all_devices {
                run_production_benchmark_all_devices(&mut engine, &args, &benchmark_path)?
            } else {
                let runtime = args.production_runtime()?;
                let mut backend = RuntimeBackend::new(runtime);
                run_benchmark(&mut engine, &mut backend, &benchmark_path)?
            }
        } else if let Some(runtime_command) = args.runtime_command.clone() {
            let runtime = CommandRuntime::new(runtime_command)
                .args(args.runtime_args.clone())
                .prompt_mode(args.runtime_prompt_mode)
                .wire_format(args.runtime_wire_format)
                .with_metadata(args.runtime_metadata.clone())
                .with_architecture(args.runtime_manifest().architecture);
            let mut backend = RuntimeBackend::new(runtime);
            run_benchmark_for_args(&mut engine, &mut backend, &args, &benchmark_path)?
        } else if args.local_runtime {
            let runtime = LocalTransformerRuntime::with_manifest(args.runtime_manifest());
            let mut backend = RuntimeBackend::new(runtime);
            run_benchmark_for_args(&mut engine, &mut backend, &args, &benchmark_path)?
        } else {
            let mut backend = HeuristicBackend;
            run_benchmark_for_args(&mut engine, &mut backend, &args, &benchmark_path)?
        };
        engine.save_full_state(
            &args.memory_path,
            &args.experience_path,
            &args.adaptive_path,
        )?;
        let gate_report = if args.benchmark_gate_enabled {
            Some(summary.evaluate(&args.benchmark_gate()))
        } else {
            None
        };
        print_benchmark_summary(&args, &benchmark_path, &summary, gate_report.as_ref());
        if let Some(trace_schema_gate_path) = &args.trace_schema_gate_path {
            let report = evaluate_trace_schema_jsonl(trace_schema_gate_path)?;
            print_trace_schema_gate_report(trace_schema_gate_path, &report);
            if !report.passed {
                std::process::exit(2);
            }
        }
        if let Some(report) = gate_report {
            if !report.passed {
                std::process::exit(2);
            }
        }
        return Ok(());
    }

    let timed_outcome = if args.production_runtime {
        let runtime = args.production_runtime()?;
        let mut backend = RuntimeBackend::new(runtime);
        run_timed_inference(
            &mut engine,
            &mut backend,
            args.prompt.clone(),
            args.profile,
            args.trace_path.as_ref(),
            None,
        )?
    } else if let Some(runtime_command) = args.runtime_command.clone() {
        let runtime = CommandRuntime::new(runtime_command)
            .args(args.runtime_args.clone())
            .prompt_mode(args.runtime_prompt_mode)
            .wire_format(args.runtime_wire_format)
            .with_metadata(args.runtime_metadata.clone())
            .with_architecture(args.runtime_manifest().architecture);
        let mut backend = RuntimeBackend::new(runtime);
        run_timed_inference(
            &mut engine,
            &mut backend,
            args.prompt.clone(),
            args.profile,
            args.trace_path.as_ref(),
            None,
        )?
    } else if args.local_runtime {
        let runtime = LocalTransformerRuntime::with_manifest(args.runtime_manifest());
        let mut backend = RuntimeBackend::new(runtime);
        run_timed_inference(
            &mut engine,
            &mut backend,
            args.prompt.clone(),
            args.profile,
            args.trace_path.as_ref(),
            None,
        )?
    } else {
        let mut backend = HeuristicBackend;
        run_timed_inference(
            &mut engine,
            &mut backend,
            args.prompt.clone(),
            args.profile,
            args.trace_path.as_ref(),
            None,
        )?
    };
    let outcome = timed_outcome.outcome;
    let elapsed_ms = timed_outcome.elapsed_ms;
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;

    println!("Noiron Rust prototype");
    println!("profile: {:?}", args.profile);
    println!("memory_file: {}", args.memory_path.display());
    println!("experience_file: {}", args.experience_path.display());
    println!("adaptive_file: {}", args.adaptive_path.display());
    println!("elapsed_ms: {}", elapsed_ms);
    if let Some(trace_path) = &args.trace_path {
        println!("trace_file: {}", trace_path.display());
    }
    if args.production_runtime {
        let runtime = args.production_runtime()?;
        println!("runtime: production-transformer-boundary");
        println!("runtime_metadata: {}", runtime.metadata().summary());
        println!("runtime_architecture: {}", runtime.architecture().summary());
        println!(
            "runtime_device_contract: {}",
            runtime.runtime_device_contract()
        );
        println!(
            "runtime_adapter: {}",
            runtime.device_gate().runtime_adapter_name()
        );
        println!("runtime_assets: {}", runtime.assets().summary_line());
        println!(
            "production_reference_kernel: {}",
            args.production_reference_kernel
        );
    } else if args.local_runtime {
        println!("runtime: local-transformer");
        println!(
            "runtime_metadata: {}",
            LocalTransformerRuntime::with_manifest(args.runtime_manifest())
                .metadata()
                .summary()
        );
        println!(
            "runtime_architecture: {}",
            args.runtime_manifest().architecture.summary()
        );
    } else if let Some(runtime_command) = &args.runtime_command {
        println!("runtime_command: {}", runtime_command.display());
        println!("runtime_metadata: {}", args.runtime_metadata.summary());
        println!(
            "runtime_architecture: {}",
            args.runtime_manifest().architecture.summary()
        );
        println!("runtime_wire_format: {}", args.runtime_wire_format.as_str());
    }
    if let Some(replay_report) = &replay_report {
        println!("experience_replay: {}", replay_report.summary());
    }
    if let Some(auto_replay_report) = &outcome.auto_replay_report {
        println!("auto_experience_replay: {}", auto_replay_report.summary());
    }
    println!();
    println!("{}", outcome.answer);
    println!();
    println!(
        "quality={:.3} perplexity={:.2} threshold_after={:.3} revision_passes={}",
        outcome.report.quality,
        outcome.metrics.perplexity,
        outcome.router_threshold_after,
        outcome.report.revision_passes
    );
    println!("process_reward: {}", outcome.process_reward.summary());
    println!("drift: {}", outcome.drift_report.summary());
    println!("hardware: {}", outcome.hardware_plan.summary());
    println!(
        "device_execution: {}",
        outcome.hardware_plan.execution.summary()
    );
    println!(
        "route: attention={} fast={} attention_fraction={:.2}",
        outcome.route_budget.attention_tokens,
        outcome.route_budget.fast_tokens,
        outcome.route_budget.attention_fraction
    );
    println!(
        "hierarchy: global={:.2} local={:.2} conv={:.2}",
        outcome.hierarchy.global, outcome.hierarchy.local, outcome.hierarchy.convolution
    );
    let tier_counts = outcome.tier_plan.counts();
    println!(
        "tiers: hot_gpu={} warm_ram={} cold_disk={}",
        tier_counts.hot_gpu, tier_counts.warm_ram, tier_counts.cold_disk
    );
    println!(
        "tier_migrations: new={} promote={} demote={} retain={} evict={}",
        count_tier_migrations(&outcome.tier_migrations, TierMigrationAction::New),
        count_tier_migrations(&outcome.tier_migrations, TierMigrationAction::Promote),
        count_tier_migrations(&outcome.tier_migrations, TierMigrationAction::Demote),
        count_tier_migrations(&outcome.tier_migrations, TierMigrationAction::Retain),
        count_tier_migrations(&outcome.tier_migrations, TierMigrationAction::Evict)
    );
    let infini_counts = outcome.infini_memory_plan.counts();
    println!(
        "infini_memory: local_window={} global_memory={} sparse_skipped={} local_tokens={} global_tokens={} skipped_tokens={}",
        infini_counts.local_window,
        infini_counts.global_memory,
        infini_counts.skipped,
        infini_counts.local_tokens,
        infini_counts.global_tokens,
        infini_counts.skipped_tokens
    );
    println!(
        "recursive: required={} chunks={} merge_rounds={} execution_waves={} max_parallel_chunks={} prompt_tokens={} native_window={} chunk_tokens={} overlap_tokens={}",
        outcome.recursive_schedule.requires_recursion,
        outcome.recursive_schedule.chunk_count(),
        outcome.recursive_schedule.merge_round_count(),
        outcome.recursive_schedule.execution_wave_count(),
        outcome.recursive_schedule.max_parallel_chunks,
        outcome.recursive_schedule.prompt_tokens,
        outcome.recursive_schedule.native_window_tokens,
        outcome.recursive_schedule.chunk_tokens,
        outcome.recursive_schedule.overlap_tokens
    );
    let transformer_counts = outcome.transformer_plan.counts();
    println!(
        "transformer: template={} global={} local={} convolution={}",
        outcome.transformer_plan.template_name(),
        transformer_counts.global,
        transformer_counts.local,
        transformer_counts.convolution
    );
    println!("agent_team: {}", outcome.agent_team_plan.summary());
    println!("stream_windows={}", outcome.stream_reports.len());
    println!(
        "memory: used={} stored={:?} feedback_reinforced={} feedback_penalized={} feedback_reinforcement_amount={:.3} feedback_penalty_amount={:.3} experience_used={} experience={}",
        outcome.used_memories.len(),
        outcome.stored_memory_id,
        outcome.memory_feedback.reinforced,
        outcome.memory_feedback.penalized,
        outcome.memory_feedback.reinforcement_amount,
        outcome.memory_feedback.penalty_amount,
        outcome.used_experiences.len(),
        outcome.experience_id
    );
    println!(
        "gist_memory: records={} document={} section={} paragraph={} stored_ids={}",
        outcome.gist_records.len(),
        count_gists(&outcome.gist_records, GistLevel::Document),
        count_gists(&outcome.gist_records, GistLevel::Section),
        count_gists(&outcome.gist_records, GistLevel::Paragraph),
        outcome.stored_gist_memory_ids.len()
    );
    println!(
        "runtime_kv: exported={} stored_ids={}",
        outcome.exported_runtime_kv_blocks,
        outcome.stored_runtime_kv_memory_ids.len()
    );
    println!(
        "retention: before={} after={} decayed={} removed={}",
        outcome.retention_report.before,
        outcome.retention_report.after,
        outcome.retention_report.decayed,
        outcome.retention_report.removed.len()
    );
    println!(
        "memory_compaction: before={} after={} merged={} removed={}",
        outcome.memory_compaction_report.before,
        outcome.memory_compaction_report.after,
        outcome.memory_compaction_report.merged.len(),
        outcome.memory_compaction_report.removed.len()
    );
    if let Some(trace_schema_gate_path) = &args.trace_schema_gate_path {
        let report = evaluate_trace_schema_jsonl(trace_schema_gate_path)?;
        print_trace_schema_gate_report(trace_schema_gate_path, &report);
        if !report.passed {
            std::process::exit(2);
        }
    }

    Ok(())
}

#[derive(Debug)]
struct TimedOutcome {
    outcome: InferenceOutcome,
    elapsed_ms: u128,
}

fn run_timed_inference<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    let started = Instant::now();
    let outcome = engine.infer(InferenceRequest::new(prompt.clone(), profile), backend);
    let elapsed_ms = started.elapsed().as_millis();

    if let Some(trace_path) = trace_path {
        if let Some(case_name) = case_name {
            append_trace_jsonl_with_case(
                trace_path, case_name, &prompt, profile, elapsed_ms, &outcome,
            )?;
        } else {
            append_trace_jsonl(trace_path, &prompt, profile, elapsed_ms, &outcome)?;
        }
    }

    Ok(TimedOutcome {
        outcome,
        elapsed_ms,
    })
}

fn run_benchmark<B: InferenceBackend>(
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

fn run_benchmark_for_args<B: InferenceBackend>(
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

fn run_production_benchmark_all_devices(
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

fn run_production_kernel_conformance_all_devices(
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
        process_reward: ProcessRewardReport {
            total: 1.0,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Reinforce,
            notes: vec![
                "memory_feedback:reinforced=2:penalized=0:reinforcement_amount=1.600000:penalty_amount=0.000000"
                    .to_owned(),
            ],
        },
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
        process_reward: ProcessRewardReport {
            total: 0.0,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Penalize,
            notes: vec![
                "memory_feedback:reinforced=0:penalized=1:reinforcement_amount=0.000000:penalty_amount=0.800000"
                    .to_owned(),
            ],
        },
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
        process_reward: ProcessRewardReport {
            total: 0.90,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Reinforce,
            notes: vec![
                "memory_feedback:reinforced=1:penalized=0:reinforcement_amount=0.700000:penalty_amount=0.000000"
                    .to_owned(),
            ],
        },
    });
}

fn seed_roundtrip_reflection_evidence(engine: &mut NoironEngine, profile: TaskProfile) {
    const SEED_PREFIX: &str = "roundtrip_reflection_seed:v1:device_state:";

    if engine
        .experience
        .records()
        .iter()
        .any(|record| record.lesson.starts_with(SEED_PREFIX))
    {
        return;
    }

    engine.experience.record(ExperienceInput {
        prompt: "persistent roundtrip reflection evidence".to_owned(),
        profile,
        lesson:
            "roundtrip_reflection_seed:v1:device_state: persist inspected reflection and revision evidence"
                .to_owned(),
        quality: 0.55,
        contradictions: Vec::new(),
        reflection_issues: vec![ReflectionIssue::new(
            "roundtrip_reflection_seed",
            ReflectionSeverity::Warning,
            "roundtrip state records a durable reflection issue for device inspection",
        )],
        revision_actions: vec!["roundtrip_review_seed".to_owned()],
        stored_memory_id: None,
        router_threshold_after: 0.52,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.52,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics::default(),
        process_reward: ProcessRewardReport {
            total: 0.5,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Hold,
            notes: Vec::new(),
        },
    });
}

fn sparse_benchmark_vector(index: usize) -> Vec<f32> {
    let mut vector = vec![0.0; 64];
    vector[index % 64] = 1.0;
    vector[(index * 7 + 3) % 64] = 0.45;
    vector[(index * 13 + 11) % 64] = 0.25;
    vector
}

fn run_persistent_roundtrip(args: &Args) -> std::io::Result<PersistentRoundtripReport> {
    let mut first_engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    configure_engine(&mut first_engine, args);
    if args.replay_limit > 0 {
        first_engine.replay_experience(args.replay_limit);
    }
    seed_roundtrip_reflection_evidence(&mut first_engine, args.profile);
    let mut first_backend = RuntimeBackend::new(LocalTransformerRuntime::with_manifest(
        args.runtime_manifest(),
    ));
    let first = first_engine.infer(
        InferenceRequest::new(args.prompt.clone(), args.profile),
        &mut first_backend,
    );
    let first_runtime_kv_memory_ids = first.stored_runtime_kv_memory_ids.clone();
    let first_runtime_kv_namespace_preserved = !first_runtime_kv_memory_ids.is_empty()
        && first_runtime_kv_memory_ids.iter().all(|id| {
            first_engine
                .cache
                .entries()
                .iter()
                .any(|entry| entry.id == *id && entry.key.starts_with("runtime_kv:"))
        });
    first_engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;

    let mut second_engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    configure_engine(&mut second_engine, args);
    let restored_runtime_kv_vectors = first_runtime_kv_memory_ids
        .iter()
        .filter_map(|id| {
            second_engine
                .cache
                .entries()
                .iter()
                .find(|entry| entry.id == *id && entry.key.starts_with("runtime_kv:"))
                .map(|entry| entry.vector.clone())
        })
        .collect::<Vec<_>>();
    let mut second_backend = RuntimeBackend::new(LocalTransformerRuntime::with_manifest(
        args.runtime_manifest(),
    ));
    let second = second_engine.infer(
        InferenceRequest::new(args.prompt.clone(), args.profile),
        &mut second_backend,
    );
    let second_used_runtime_kv_memory = second.used_memories.iter().any(|memory| {
        first_runtime_kv_memory_ids.contains(&memory.id) && memory.key.starts_with("runtime_kv:")
    });
    let imported_runtime_kv_blocks = second_backend.runtime().imported_kv_blocks();
    let second_imported_runtime_kv_blocks = imported_runtime_kv_blocks.len();
    let second_imported_runtime_kv_from_namespace =
        imported_runtime_kv_blocks.iter().any(|block| {
            !block.key.is_empty()
                && restored_runtime_kv_vectors
                    .iter()
                    .any(|vector| vector.starts_with(&block.key))
        });
    second_engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;

    Ok(PersistentRoundtripReport::evaluate(
        PersistentRoundtripInput {
            first_stored_memory: first.stored_memory_id.is_some(),
            first_runtime_kv_stored: first.stored_runtime_kv_memory_ids.len(),
            first_runtime_kv_namespace_preserved,
            second_used_memories: second.used_memories.len(),
            second_used_runtime_kv_memory,
            second_used_experiences: second.used_experiences.len(),
            second_imported_runtime_kv_blocks,
            second_imported_runtime_kv_from_namespace,
            second_runtime_adapter_observations: second.runtime_adapter_observations.len(),
            second_runtime_adapter_best_score: second
                .runtime_adapter_observations
                .first()
                .map(|observation| observation.score),
            second_runtime_adapter_best_adapter: second
                .runtime_adapter_observations
                .first()
                .map(|observation| observation.adapter.as_str().to_owned()),
            second_runtime_selected_adapter: second.runtime_diagnostics.selected_adapter.clone(),
            second_quality: second.report.quality,
            first_drift_severity: first.drift_report.severity,
            second_drift_severity: second.drift_report.severity,
        },
    ))
}

fn run_persistent_roundtrip_all_devices(
    args: &Args,
) -> std::io::Result<PersistentRoundtripMatrixReport> {
    let mut device_reports = Vec::new();

    for device in DeviceClass::explicit_profiles() {
        let device_args = args.for_roundtrip_device(*device);
        let report = run_persistent_roundtrip(&device_args)?;
        device_reports.push(PersistentRoundtripDeviceReport {
            device: *device,
            report,
        });
    }

    Ok(PersistentRoundtripMatrixReport::evaluate(device_reports))
}

fn run_state_inspection(args: &Args) -> std::io::Result<StateInspectionReport> {
    let mut engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    configure_engine(&mut engine, args);
    Ok(StateInspectionReport::from_engine(
        &engine,
        args.inspect_limit,
    ))
}

fn run_state_inspection_all_devices(
    args: &Args,
) -> std::io::Result<StateInspectionMatrixGateReport> {
    let mut device_reports = Vec::new();
    let gate = args.state_inspection_gate();
    let matrix_gate = args.state_inspection_matrix_gate();

    for device in DeviceClass::explicit_profiles() {
        let device_args = args.for_inspect_device(*device);
        let mut state_file_failures = Vec::new();
        if !device_args.memory_path.exists() {
            state_file_failures.push(format!(
                "memory file missing: {}",
                device_args.memory_path.display()
            ));
        }
        if !device_args.experience_path.exists() {
            state_file_failures.push(format!(
                "experience file missing: {}",
                device_args.experience_path.display()
            ));
        }
        if !device_args.adaptive_path.exists() {
            state_file_failures.push(format!(
                "adaptive file missing: {}",
                device_args.adaptive_path.display()
            ));
        }
        let mut engine = NoironEngine::load_full_state(
            &device_args.memory_path,
            &device_args.experience_path,
            &device_args.adaptive_path,
        )?;
        configure_engine(&mut engine, &device_args);
        let report = StateInspectionReport::from_engine(&engine, device_args.inspect_limit);
        let mut gate_report = report.evaluate(&gate);
        gate_report.failures.extend(state_file_failures);
        gate_report.passed = gate_report.failures.is_empty();
        device_reports.push(StateInspectionDeviceGateReport::from_report(
            *device,
            &report,
            gate_report,
        ));
    }

    Ok(StateInspectionMatrixGateReport::evaluate_with_gate(
        device_reports,
        &matrix_gate,
    ))
}

fn device_scoped_path(path: &std::path::Path, device: DeviceClass) -> PathBuf {
    let parent = path.parent();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("state");
    let extension = path.extension().and_then(|value| value.to_str());
    let file_name = match extension {
        Some(extension) if !extension.is_empty() => {
            format!("{}.{}.{}", stem, device.as_str(), extension)
        }
        _ => format!("{}.{}", stem, device.as_str()),
    };

    parent
        .map(|parent| parent.join(&file_name))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

fn configure_engine(engine: &mut NoironEngine, args: &Args) {
    let hardware_snapshot = HardwareSnapshot::new(
        args.device,
        args.cpu_load,
        args.gpu_load,
        args.ram_load,
        args.disk_load,
    );
    engine.recursive_scheduler = RecursiveScheduler::new(
        args.native_window_tokens,
        args.chunk_tokens,
        args.chunk_overlap_tokens,
        args.merge_fan_in,
    );
    engine.set_auto_replay_limit(args.auto_replay_limit);
    engine.set_hardware_snapshot(hardware_snapshot);
    let governance_plan = engine.hardware_allocator.memory_governance_plan(
        hardware_snapshot,
        engine.memory_retention_policy,
        engine.memory_compaction_policy.clone(),
    );
    engine.set_memory_retention_policy(memory_retention_policy_from_args(
        governance_plan.retention_policy,
        args,
    ));
    engine.set_memory_compaction_policy(memory_compaction_policy_from_args(
        governance_plan.compaction_policy,
        args,
    ));
}

fn memory_retention_policy_from_args(
    mut policy: MemoryRetentionPolicy,
    args: &Args,
) -> MemoryRetentionPolicy {
    if let Some(value) = args.retention_stale_after {
        policy.stale_after = value.max(1);
    }
    if let Some(value) = args.retention_decay_rate {
        policy.decay_rate = value.clamp(0.0, 0.95);
    }
    if let Some(value) = args.retention_remove_below {
        policy.remove_below_strength = value.clamp(0.0, 3.0);
    }
    if let Some(value) = args.retention_remove_after_failures {
        policy.remove_after_failures = value.max(1);
    }

    policy
}

fn memory_compaction_policy_from_args(
    mut policy: MemoryCompactionPolicy,
    args: &Args,
) -> MemoryCompactionPolicy {
    if let Some(value) = args.compaction_similarity_threshold {
        policy.similarity_threshold = value.clamp(0.10, 0.999);
    }
    if let Some(value) = args.compaction_max_candidates {
        policy.max_candidates = value.max(2);
    }
    if let Some(value) = args.compaction_max_merges {
        policy.max_merges = value;
    }

    policy
}

fn print_benchmark_summary(
    args: &Args,
    benchmark_path: &PathBuf,
    summary: &BenchmarkSummary,
    gate_report: Option<&BenchmarkGateReport>,
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
            "case={} profile={:?} device={} elapsed_ms={} quality={:.3} reward={:.3} attention_fraction={:.2} requires_recursion={} chunks={} waves={} recursive_runtime_calls={} auto_replay_applied={} auto_replay_router_updates={} auto_replay_hierarchy_updates={} auto_replay_router_threshold_mutations={} auto_replay_hierarchy_weight_mutations={} auto_replay_router_threshold_delta={:.6} auto_replay_hierarchy_weight_delta={:.6} auto_replay_memory_reinforcements={} auto_replay_memory_penalties={} auto_replay_live_memory_feedback_items={} auto_replay_live_memory_feedback_updates={} auto_replay_live_memory_feedback_reinforcements={} auto_replay_live_memory_feedback_penalties={} auto_replay_recursive_items={} auto_replay_recursive_runtime_calls={} auto_replay_avg_recursive_call_pressure={:.3} auto_replay_max_recursive_call_pressure={:.3} used_memories={} infini_local_window={} infini_global_memory={} sparse_skipped={} sparse_skipped_tokens={} stored_memories={} compacted_memories={} runtime_forward_signal={} runtime_forward_energy_signal={} runtime_kv_influence_signal={} runtime_token_count={} runtime_uncertainty_tokens={} runtime_uncertainty_signal={} runtime_kv_imported={} runtime_kv_exported={} runtime_kv_stored={} runtime_selected_adapter={} runtime_adapter_contract_ok={} runtime_adapter_contract_violations={} runtime_adapter_observations={} runtime_adapter_best_score={} drift={}",
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
            result.drift_severity.as_str()
        );
    }

    if let Some(report) = gate_report {
        println!("{}", report.summary_line());
        for failure in &report.failures {
            println!("benchmark_gate_failure: {failure}");
        }
    }
}

fn print_persistent_roundtrip_report(args: &Args, report: &PersistentRoundtripReport) {
    println!("Noiron persistent roundtrip benchmark");
    println!("memory_file: {}", args.memory_path.display());
    println!("experience_file: {}", args.experience_path.display());
    println!("adaptive_file: {}", args.adaptive_path.display());
    println!("runtime: local-transformer");
    println!("prompt_profile: {:?}", args.profile);
    println!("{}", report.summary_line());
    for failure in &report.failures {
        println!("persistent_roundtrip_failure: {failure}");
    }
}

fn print_persistent_roundtrip_matrix_report(args: &Args, report: &PersistentRoundtripMatrixReport) {
    println!("Noiron persistent roundtrip all-device benchmark");
    println!("memory_file_pattern: {}", args.memory_path.display());
    println!(
        "experience_file_pattern: {}",
        args.experience_path.display()
    );
    println!("adaptive_file_pattern: {}", args.adaptive_path.display());
    println!("runtime: local-transformer");
    println!("prompt_profile: {:?}", args.profile);
    println!("{}", report.summary_line());
    for device_report in &report.device_reports {
        println!(
            "device={} {}",
            device_report.device.as_str(),
            device_report.report.summary_line()
        );
        for failure in &device_report.report.failures {
            println!(
                "persistent_roundtrip_device_failure: {}: {}",
                device_report.device.as_str(),
                failure
            );
        }
    }
    for failure in &report.failures {
        println!("persistent_roundtrip_matrix_failure: {failure}");
    }
}

fn print_trace_schema_gate_report(path: &PathBuf, report: &TraceSchemaGateReport) {
    println!("Noiron trace schema gate");
    println!("trace_file: {}", path.display());
    println!("{}", report.summary_line());
    for failure in &report.failures {
        println!("trace_schema_gate_failure: {failure}");
    }
}

fn print_state_inspection_report(args: &Args, report: &StateInspectionReport) {
    println!("Noiron state inspection");
    println!("memory_file: {}", args.memory_path.display());
    println!("experience_file: {}", args.experience_path.display());
    println!("adaptive_file: {}", args.adaptive_path.display());
    println!("{}", report.summary_line());
    println!(
        "profile_observations: general={} coding={} writing={} long={}",
        report.profile_observations.general,
        report.profile_observations.coding,
        report.profile_observations.writing,
        report.profile_observations.long_document
    );
    println!(
        "profile_hierarchy_observations: general={} coding={} writing={} long={}",
        report.profile_hierarchy_observations.general,
        report.profile_hierarchy_observations.coding,
        report.profile_hierarchy_observations.writing,
        report.profile_hierarchy_observations.long_document
    );
    println!(
        "memory_retention_policy: stale_after={} decay_rate={:.3} remove_below_strength={:.3} remove_after_failures={}",
        report.memory_retention_policy.stale_after,
        report.memory_retention_policy.decay_rate,
        report.memory_retention_policy.remove_below_strength,
        report.memory_retention_policy.remove_after_failures
    );
    println!(
        "memory_compaction_policy: similarity_threshold={:.3} max_candidates={} max_merges={}",
        report.memory_compaction_policy.similarity_threshold,
        report.memory_compaction_policy.max_candidates,
        report.memory_compaction_policy.max_merges
    );
    if report.memory_vector_dimensions.is_empty() {
        println!("memory_vector_dimensions: none");
    } else {
        let dimensions = report
            .memory_vector_dimensions
            .iter()
            .map(|bucket| format!("{}:{}", bucket.dimensions, bucket.count))
            .collect::<Vec<_>>()
            .join(" ");
        println!("memory_vector_dimensions: {dimensions}");
    }
    if report.runtime_kv_vector_dimensions.is_empty() {
        println!("runtime_kv_vector_dimensions: none");
    } else {
        let dimensions = report
            .runtime_kv_vector_dimensions
            .iter()
            .map(|bucket| format!("{}:{}", bucket.dimensions, bucket.count))
            .collect::<Vec<_>>()
            .join(" ");
        println!("runtime_kv_vector_dimensions: {dimensions}");
    }

    println!("top_memories:");
    if report.top_memories.is_empty() {
        println!("  none");
    } else {
        for memory in &report.top_memories {
            println!(
                "  id={} dims={} strength={:.3} hits={} failures={} last_score={:.3} key={}",
                memory.id,
                memory.vector_dimensions,
                memory.strength,
                memory.hits,
                memory.failures,
                memory.last_score,
                memory.key
            );
        }
    }

    println!("top_runtime_kv_memories:");
    if report.top_runtime_kv_memories.is_empty() {
        println!("  none");
    } else {
        for memory in &report.top_runtime_kv_memories {
            println!(
                "  id={} dims={} strength={:.3} hits={} failures={} last_score={:.3} key={}",
                memory.id,
                memory.vector_dimensions,
                memory.strength,
                memory.hits,
                memory.failures,
                memory.last_score,
                memory.key
            );
        }
    }

    println!("top_experiences:");
    if report.top_experiences.is_empty() {
        println!("  none");
    } else {
        for experience in &report.top_experiences {
            println!(
                "  id={} profile={:?} quality={:.3} reward={:.3} action={} runtime_model={} adapter={} layers={} hidden={} local_window={} forward_energy={} kv_influence={} runtime_kv_imported={} runtime_kv_exported={} recursive_runtime_calls={} live_memory_feedback_updates={} live_memory_feedback_reinforced={} live_memory_feedback_penalized={} reflection_issues={} critical={} revision_actions={} lesson={}",
                experience.id,
                experience.profile,
                experience.quality,
                experience.process_reward,
                experience.reward_action.as_str(),
                option_text(experience.runtime_model_id.as_deref()),
                option_text(experience.runtime_selected_adapter.as_deref()),
                experience.runtime_layer_count,
                experience.runtime_hidden_size,
                experience.runtime_local_window_tokens,
                option_f32_text(experience.runtime_forward_energy),
                option_f32_text(experience.runtime_kv_influence),
                experience.runtime_imported_kv_blocks,
                experience.runtime_exported_kv_blocks,
                option_usize_text(experience.recursive_runtime_calls),
                experience.live_memory_feedback_updates,
                experience.live_memory_feedback_reinforced,
                experience.live_memory_feedback_penalized,
                experience.reflection_issues,
                experience.critical_reflection_issues,
                experience.revision_actions,
                experience.lesson
            );
        }
    }
}

fn print_state_inspection_gate_report(report: &StateInspectionGateReport) {
    println!("{}", report.summary_line());
    for failure in &report.failures {
        println!("state_inspection_gate_failure: {failure}");
    }
}

fn print_state_inspection_matrix_gate_report(
    args: &Args,
    report: &StateInspectionMatrixGateReport,
) {
    println!("Noiron state inspection all-device gate");
    println!("memory_file_pattern: {}", args.memory_path.display());
    println!(
        "experience_file_pattern: {}",
        args.experience_path.display()
    );
    println!("adaptive_file_pattern: {}", args.adaptive_path.display());
    println!("{}", report.summary_line());
    for device_report in &report.device_reports {
        println!(
            "device={} {} runtime_kv_memories={} runtime_model_experiences={} runtime_adapter_experiences={} runtime_forward_energy_experiences={} runtime_kv_influence_experiences={} runtime_kv_import_experiences={} runtime_kv_export_experiences={} reflection_issue_experiences={} critical_reflection_issue_experiences={} revision_action_experiences={} live_memory_feedback_experiences={} live_memory_feedback_updates={} evolution_live_inference_runs={} evolution_live_router_threshold_mutations={} evolution_live_hierarchy_weight_mutations={} evolution_live_memory_updates={} evolution_live_stored_memory_updates={} evolution_live_reflection_issues={} evolution_live_critical_reflection_issues={} evolution_live_revision_actions={} evolution_replay_runs={} evolution_replay_items={} evolution_router_threshold_mutations={} evolution_hierarchy_weight_mutations={} evolution_memory_updates={} evolution_replay_live_memory_feedback_updates={} evolution_recursive_replay_items={} evolution_recursive_runtime_calls={}",
            device_report.device.as_str(),
            device_report.report.summary_line(),
            device_report.runtime_kv_memories,
            device_report.runtime_model_experiences,
            device_report.runtime_adapter_experiences,
            device_report.runtime_forward_energy_experiences,
            device_report.runtime_kv_influence_experiences,
            device_report.runtime_kv_import_experiences,
            device_report.runtime_kv_export_experiences,
            device_report.reflection_issue_experiences,
            device_report.critical_reflection_issue_experiences,
            device_report.revision_action_experiences,
            device_report.live_memory_feedback_experiences,
            device_report.live_memory_feedback_updates,
            device_report.evolution_live_inference_runs,
            device_report.evolution_live_router_threshold_mutations,
            device_report.evolution_live_hierarchy_weight_mutations,
            device_report.evolution_live_memory_updates,
            device_report.evolution_live_stored_memory_updates,
            device_report.evolution_live_reflection_issues,
            device_report.evolution_live_critical_reflection_issues,
            device_report.evolution_live_revision_actions,
            device_report.evolution_replay_runs,
            device_report.evolution_replay_items,
            device_report.evolution_router_threshold_mutations,
            device_report.evolution_hierarchy_weight_mutations,
            device_report.evolution_memory_updates,
            device_report.evolution_replay_live_memory_feedback_updates,
            device_report.evolution_recursive_replay_items,
            device_report.evolution_recursive_runtime_calls
        );
        for failure in &device_report.report.failures {
            println!(
                "state_inspection_matrix_gate_failure: device={} {}",
                device_report.device.as_str(),
                failure
            );
        }
    }
    for failure in &report.failures {
        println!("state_inspection_matrix_gate_failure: {failure}");
    }
}

fn option_text(value: Option<&str>) -> &str {
    value.filter(|item| !item.is_empty()).unwrap_or("none")
}

fn option_f32_text(value: Option<f32>) -> String {
    value
        .filter(|item| item.is_finite())
        .map(|item| format!("{item:.3}"))
        .unwrap_or_else(|| "none".to_owned())
}

fn option_usize_text(value: Option<usize>) -> String {
    value
        .map(|item| item.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn print_device_matrix_and_exit() -> ! {
    let allocator = HardwareAllocator::new();
    let base_hierarchy = HierarchyWeights::default();

    println!("Noiron device matrix");
    println!(
        "profile,tier,scope,aliases,primary_lane,fallback_lane,memory_mode,adapters,parallel_chunks,kv_prefetch,kv_bits,disk_spill,local_kv_tokens,global_kv_tokens,latency_budget_ms,retention_stale_after,retention_decay_rate,retention_remove_below,retention_remove_after_failures,compaction_threshold,compaction_max_candidates,compaction_max_merges"
    );

    for device in DeviceClass::explicit_profiles() {
        let descriptor = device.descriptor();
        let snapshot = HardwareSnapshot::new(*device, 0.35, 0.30, 0.45, 0.20);
        let plan = allocator.plan(snapshot, TaskProfile::General, 4096, base_hierarchy);
        let governance = allocator.memory_governance_plan(
            snapshot,
            MemoryRetentionPolicy::default(),
            MemoryCompactionPolicy::default(),
        );
        let adapters = plan
            .execution
            .adapter_hints
            .iter()
            .map(|adapter| adapter.as_str())
            .collect::<Vec<_>>()
            .join("+");
        println!(
            "{},{},{},{},{},{},{},{},{},{},{}/{},{},{},{},{},{},{:.3},{:.3},{},{:.3},{},{}",
            device.as_str(),
            plan.tier.as_str(),
            descriptor.scope,
            descriptor.aliases_csv(),
            plan.execution.primary_lane.as_str(),
            plan.execution.fallback_lane.as_str(),
            plan.execution.memory_mode.as_str(),
            adapters,
            plan.execution.max_parallel_chunks,
            plan.execution.kv_prefetch_blocks,
            plan.execution.hot_kv_precision_bits,
            plan.execution.cold_kv_precision_bits,
            plan.execution.allow_disk_spill,
            plan.local_kv_token_budget,
            plan.global_kv_token_budget,
            plan.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            governance.retention_policy.stale_after,
            governance.retention_policy.decay_rate,
            governance.retention_policy.remove_below_strength,
            governance.retention_policy.remove_after_failures,
            governance.compaction_policy.similarity_threshold,
            governance.compaction_policy.max_candidates,
            governance.compaction_policy.max_merges
        );
    }

    std::process::exit(0);
}

fn print_device_gate_report(report: &DevicePlanGateReport) {
    println!("Noiron device compatibility gate");
    println!("{}", report.summary_line());
    println!(
        "profile,tier,scope,aliases,primary_lane,fallback_lane,memory_mode,adapters,runtime_adapter,parallel_chunks,kv_prefetch,kv_bits,disk_spill,runtime_kv_import,runtime_kv_export,runtime_max_import,runtime_max_export,runtime_kv_bits,local_kv_tokens,global_kv_tokens,latency_budget_ms,runtime_device_contract,retention_stale_after,retention_decay_rate,retention_remove_below,retention_remove_after_failures,compaction_threshold,compaction_max_candidates,compaction_max_merges,passed"
    );

    for row in &report.rows {
        let fields = vec![
            row.device.as_str().to_owned(),
            row.tier.as_str().to_owned(),
            row.scope.to_owned(),
            row.aliases_csv(),
            row.primary_lane.as_str().to_owned(),
            row.fallback_lane.as_str().to_owned(),
            row.memory_mode.as_str().to_owned(),
            row.adapters_csv(),
            row.runtime_adapter_name().to_owned(),
            row.max_parallel_chunks.to_string(),
            row.kv_prefetch_blocks.to_string(),
            format!(
                "{}/{}",
                row.hot_kv_precision_bits, row.cold_kv_precision_bits
            ),
            row.allow_disk_spill.to_string(),
            row.runtime_kv_import_enabled.to_string(),
            row.runtime_kv_export_enabled.to_string(),
            row.runtime_max_import_blocks.to_string(),
            row.runtime_max_export_blocks.to_string(),
            format!(
                "{}/{}",
                row.runtime_hot_kv_precision_bits, row.runtime_cold_kv_precision_bits
            ),
            row.local_kv_token_budget.to_string(),
            row.global_kv_token_budget.to_string(),
            row.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            row.runtime_device_contract.clone(),
            row.memory_governance
                .retention_policy
                .stale_after
                .to_string(),
            format!("{:.3}", row.memory_governance.retention_policy.decay_rate),
            format!(
                "{:.3}",
                row.memory_governance.retention_policy.remove_below_strength
            ),
            row.memory_governance
                .retention_policy
                .remove_after_failures
                .to_string(),
            format!(
                "{:.3}",
                row.memory_governance.compaction_policy.similarity_threshold
            ),
            row.memory_governance
                .compaction_policy
                .max_candidates
                .to_string(),
            row.memory_governance
                .compaction_policy
                .max_merges
                .to_string(),
            row.passed().to_string(),
        ];
        println!("{}", fields.join(","));

        for failure in &row.failures {
            println!("device_gate_failure: {}: {}", row.device.as_str(), failure);
        }
    }
}

fn print_kv_quant_gate_report(
    summary: &KvQuantBenchmarkSummary,
    report: &KvQuantBenchmarkGateReport,
) {
    println!("Noiron KV quantization benchmark");
    println!("{}", summary.summary_line());
    println!("{}", report.summary_line());
    println!("case,bits,len,max_abs_error,mean_abs_error,compression_ratio,elapsed_us");

    for result in summary.results() {
        println!(
            "{},q{},{},{:.6},{:.6},{:.3},{}",
            result.name,
            result.bits.width(),
            result.len,
            result.max_abs_error,
            result.mean_abs_error,
            result.compression_ratio,
            result.elapsed_us
        );
    }

    for failure in &report.failures {
        println!("kv_quant_gate_failure: {failure}");
    }
}

fn print_runtime_manifest_gate_report(
    manifest: &RuntimeManifest,
    validation: &RuntimeManifestValidation,
    device_gate: &RuntimeManifestDeviceGateReport,
    all_devices_gate: Option<&DevicePlanGateReport>,
) {
    let all_device_failures = all_devices_gate
        .map(DevicePlanGateReport::failure_count)
        .unwrap_or(0);
    println!("Noiron runtime manifest gate");
    println!(
        "runtime_manifest_gate: passed={} errors={} warnings={} device_failures={} all_device_failures={}",
        validation.passed()
            && device_gate.passed()
            && all_devices_gate
                .map(DevicePlanGateReport::passed)
                .unwrap_or(true),
        validation.errors.len() + device_gate.failures.len() + all_device_failures,
        validation.warnings.len(),
        device_gate.failures.len(),
        all_device_failures
    );
    println!("{}", device_gate.summary_line());
    println!(
        "runtime_metadata: {}",
        manifest.runtime_metadata().summary()
    );
    println!(
        "runtime_assets: weights={} tokenizer={} config={}",
        option_path_display(manifest.assets.weights.as_ref()),
        option_path_display(manifest.assets.tokenizer.as_ref()),
        option_path_display(manifest.assets.config.as_ref())
    );
    println!(
        "runtime_architecture: layers={} hidden={} attention_heads={} kv_heads={} local_window={}",
        manifest.architecture.layer_count,
        manifest.architecture.hidden_size,
        manifest.architecture.attention_heads,
        manifest.architecture.kv_heads,
        manifest.architecture.local_window_tokens
    );
    println!(
        "runtime_kv_policy: import={} export={} max_import={} max_export={} kv_bits={}/{}",
        manifest.kv_policy.import_enabled,
        manifest.kv_policy.export_enabled,
        manifest.kv_policy.max_import_blocks,
        manifest.kv_policy.max_export_blocks,
        manifest.quantization.hot_kv.width(),
        manifest.quantization.cold_kv.width()
    );
    println!(
        "runtime_device: device={} tier={} primary={} fallback={} memory={} adapters={} runtime_adapter={} parallel_chunks={} kv_prefetch={} kv_bits={}/{} disk_spill={} local_kv_tokens={} global_kv_tokens={} latency_budget_ms={}",
        device_gate.device.as_str(),
        device_gate.tier.as_str(),
        device_gate.primary_lane.as_str(),
        device_gate.fallback_lane.as_str(),
        device_gate.memory_mode.as_str(),
        device_gate.adapters_csv(),
        device_gate.runtime_adapter_name(),
        device_gate.max_parallel_chunks,
        device_gate.kv_prefetch_blocks,
        device_gate.hot_kv_precision_bits,
        device_gate.cold_kv_precision_bits,
        device_gate.allow_disk_spill,
        device_gate.local_kv_token_budget,
        device_gate.global_kv_token_budget,
        device_gate
            .latency_budget_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned())
    );
    println!(
        "runtime_device_contract: {}",
        device_gate.runtime_device_contract
    );

    for warning in &validation.warnings {
        println!("runtime_manifest_warning: {warning}");
    }
    for error in &validation.errors {
        println!("runtime_manifest_error: {error}");
    }
    for failure in &device_gate.failures {
        println!("runtime_manifest_device_failure: {failure}");
    }
    if let Some(report) = all_devices_gate {
        print_runtime_manifest_all_devices_gate_report(report);
    }
}

fn print_runtime_manifest_all_devices_gate_report(report: &DevicePlanGateReport) {
    println!("{}", report.summary_line());
    println!(
        "runtime_manifest_all_devices,profile,tier,scope,aliases,primary_lane,fallback_lane,memory_mode,adapters,runtime_adapter,parallel_chunks,kv_prefetch,kv_bits,disk_spill,runtime_kv_import,runtime_kv_export,runtime_max_import,runtime_max_export,runtime_kv_bits,local_kv_tokens,global_kv_tokens,latency_budget_ms,runtime_device_contract,passed"
    );

    for row in &report.rows {
        let fields = vec![
            "runtime_manifest_all_devices".to_owned(),
            row.device.as_str().to_owned(),
            row.tier.as_str().to_owned(),
            row.scope.to_owned(),
            row.aliases_csv(),
            row.primary_lane.as_str().to_owned(),
            row.fallback_lane.as_str().to_owned(),
            row.memory_mode.as_str().to_owned(),
            row.adapters_csv(),
            row.runtime_adapter_name().to_owned(),
            row.max_parallel_chunks.to_string(),
            row.kv_prefetch_blocks.to_string(),
            format!(
                "{}/{}",
                row.hot_kv_precision_bits, row.cold_kv_precision_bits
            ),
            row.allow_disk_spill.to_string(),
            row.runtime_kv_import_enabled.to_string(),
            row.runtime_kv_export_enabled.to_string(),
            row.runtime_max_import_blocks.to_string(),
            row.runtime_max_export_blocks.to_string(),
            format!(
                "{}/{}",
                row.runtime_hot_kv_precision_bits, row.runtime_cold_kv_precision_bits
            ),
            row.local_kv_token_budget.to_string(),
            row.global_kv_token_budget.to_string(),
            row.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            row.runtime_device_contract.clone(),
            row.passed().to_string(),
        ];
        println!("{}", fields.join(","));

        for failure in &row.failures {
            println!(
                "runtime_manifest_all_devices_failure: {}: {}",
                row.device.as_str(),
                failure
            );
        }
    }
}

fn print_production_kernel_conformance_report(report: &ProductionKernelConformanceReport) {
    println!("Noiron production kernel conformance gate");
    println!("{}", report.summary_line());
    for failure in &report.failures {
        println!("production_kernel_conformance_failure: {failure}");
    }
}

fn print_production_kernel_conformance_matrix_report(
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

fn option_path_display(path: Option<&PathBuf>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn count_gists(records: &[rust_norion::GistRecord], level: GistLevel) -> usize {
    records
        .iter()
        .filter(|record| record.level == level)
        .count()
}

fn count_tier_migrations(
    migrations: &[rust_norion::TierMigration],
    action: TierMigrationAction,
) -> usize {
    migrations
        .iter()
        .filter(|migration| migration.action == action)
        .count()
}

#[derive(Debug, Clone)]
struct Args {
    prompt: String,
    profile: TaskProfile,
    memory_path: PathBuf,
    experience_path: PathBuf,
    adaptive_path: PathBuf,
    trace_path: Option<PathBuf>,
    trace_schema_gate_path: Option<PathBuf>,
    benchmark_path: Option<PathBuf>,
    benchmark_all_devices: bool,
    benchmark_gate_enabled: bool,
    benchmark_min_quality: Option<f32>,
    benchmark_min_reward: Option<f32>,
    benchmark_max_total_ms: Option<u128>,
    benchmark_max_recursive_chunks: Option<usize>,
    benchmark_min_recursive_cases: Option<usize>,
    benchmark_min_recursive_runtime_calls: Option<usize>,
    benchmark_min_auto_replay_router_updates: Option<usize>,
    benchmark_min_auto_replay_hierarchy_updates: Option<usize>,
    benchmark_min_auto_replay_router_threshold_mutations: Option<usize>,
    benchmark_min_auto_replay_hierarchy_weight_mutations: Option<usize>,
    benchmark_min_auto_replay_router_threshold_delta: Option<f32>,
    benchmark_min_auto_replay_hierarchy_weight_delta: Option<f32>,
    benchmark_min_auto_replay_memory_updates: Option<usize>,
    benchmark_min_live_memory_feedback_updates: Option<usize>,
    benchmark_min_auto_replay_live_memory_feedback_updates: Option<usize>,
    benchmark_min_auto_replay_recursive_items: Option<usize>,
    benchmark_min_auto_replay_recursive_call_pressure: Option<f32>,
    benchmark_max_auto_replay_recursive_call_pressure: Option<f32>,
    benchmark_min_evolution_live_inference_runs: Option<u64>,
    benchmark_min_evolution_live_router_threshold_mutations: Option<u64>,
    benchmark_min_evolution_live_hierarchy_weight_mutations: Option<u64>,
    benchmark_min_evolution_live_router_threshold_delta: Option<f32>,
    benchmark_min_evolution_live_hierarchy_weight_delta: Option<f32>,
    benchmark_min_evolution_live_memory_updates: Option<u64>,
    benchmark_min_evolution_live_stored_memory_updates: Option<u64>,
    benchmark_min_evolution_live_reflection_issues: Option<u64>,
    benchmark_min_evolution_live_critical_reflection_issues: Option<u64>,
    benchmark_min_evolution_live_revision_actions: Option<u64>,
    benchmark_min_evolution_live_inference_device_profiles: Option<usize>,
    benchmark_min_evolution_live_router_threshold_mutation_device_profiles: Option<usize>,
    benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles: Option<usize>,
    benchmark_min_evolution_live_memory_update_device_profiles: Option<usize>,
    benchmark_min_evolution_live_stored_memory_update_device_profiles: Option<usize>,
    benchmark_min_evolution_live_reflection_issue_device_profiles: Option<usize>,
    benchmark_min_evolution_live_critical_reflection_issue_device_profiles: Option<usize>,
    benchmark_min_evolution_live_revision_action_device_profiles: Option<usize>,
    benchmark_min_evolution_replay_runs: Option<u64>,
    benchmark_min_evolution_replay_items: Option<u64>,
    benchmark_min_evolution_router_threshold_mutations: Option<u64>,
    benchmark_min_evolution_hierarchy_weight_mutations: Option<u64>,
    benchmark_min_evolution_router_threshold_delta: Option<f32>,
    benchmark_min_evolution_hierarchy_weight_delta: Option<f32>,
    benchmark_min_evolution_memory_updates: Option<u64>,
    benchmark_min_evolution_replay_live_memory_feedback_updates: Option<u64>,
    benchmark_min_evolution_recursive_replay_items: Option<u64>,
    benchmark_min_evolution_recursive_runtime_calls: Option<u64>,
    benchmark_max_evolution_drift_rollbacks: Option<u64>,
    benchmark_max_evolution_rollback_router_threshold_delta: Option<f32>,
    benchmark_max_evolution_rollback_hierarchy_weight_delta: Option<f32>,
    benchmark_min_sparse_skipped_cases: Option<usize>,
    benchmark_min_sparse_skipped_tokens: Option<usize>,
    benchmark_min_runtime_forward_cases: Option<usize>,
    benchmark_min_runtime_forward_energy_cases: Option<usize>,
    benchmark_min_runtime_kv_influence_cases: Option<usize>,
    benchmark_min_runtime_layer_mode_cases: Option<usize>,
    benchmark_min_runtime_all_layer_mode_cases: Option<usize>,
    benchmark_min_runtime_global_layers: Option<usize>,
    benchmark_min_runtime_local_window_layers: Option<usize>,
    benchmark_min_runtime_convolutional_fusion_layers: Option<usize>,
    benchmark_min_runtime_uncertainty_cases: Option<usize>,
    benchmark_min_runtime_uncertainty_tokens: Option<usize>,
    benchmark_min_runtime_kv_import_cases: Option<usize>,
    benchmark_min_runtime_kv_imported: Option<usize>,
    benchmark_min_runtime_kv_exported: Option<usize>,
    benchmark_min_runtime_kv_stored: Option<usize>,
    benchmark_min_runtime_adapter_contract_cases: Option<usize>,
    benchmark_min_runtime_adapter_kinds: Option<usize>,
    benchmark_min_runtime_adapter_observations: Option<usize>,
    benchmark_min_runtime_adapter_best_score: Option<f32>,
    benchmark_max_runtime_adapter_contract_violations: Option<usize>,
    benchmark_max_memory_governance_failures: Option<usize>,
    benchmark_min_memory_governance_cases: Option<usize>,
    benchmark_min_memory_governance_device_profiles: Option<usize>,
    benchmark_min_memory_retention_activity_cases: Option<usize>,
    benchmark_min_memory_compaction_activity_cases: Option<usize>,
    benchmark_min_reflection_issue_cases: Option<usize>,
    benchmark_min_reflection_issues: Option<usize>,
    benchmark_min_critical_reflection_issue_cases: Option<usize>,
    benchmark_min_critical_reflection_issues: Option<usize>,
    benchmark_min_revision_action_cases: Option<usize>,
    benchmark_min_revision_actions: Option<usize>,
    benchmark_min_reflection_issue_device_profiles: Option<usize>,
    benchmark_min_critical_reflection_issue_device_profiles: Option<usize>,
    benchmark_min_revision_action_device_profiles: Option<usize>,
    benchmark_min_device_profiles: Option<usize>,
    benchmark_min_recursive_device_profiles: Option<usize>,
    benchmark_max_drift_blocks: Option<usize>,
    benchmark_max_drift_rollbacks: Option<usize>,
    benchmark_roundtrip: bool,
    list_devices: bool,
    device_gate: bool,
    kv_quant_gate: bool,
    kv_quant_max_total_us: Option<u128>,
    runtime_manifest_gate: bool,
    runtime_manifest_all_devices_gate: bool,
    runtime_weights_path: Option<PathBuf>,
    runtime_tokenizer_path: Option<PathBuf>,
    runtime_config_path: Option<PathBuf>,
    runtime_layer_count: Option<usize>,
    runtime_hidden_size: Option<usize>,
    runtime_attention_heads: Option<usize>,
    runtime_kv_heads: Option<usize>,
    runtime_local_window_tokens: Option<usize>,
    inspect_state: bool,
    inspect_limit: usize,
    inspect_gate: bool,
    inspect_min_memories: Option<usize>,
    inspect_min_runtime_kv_memories: Option<usize>,
    inspect_min_experiences: Option<usize>,
    inspect_min_runtime_model_experiences: Option<usize>,
    inspect_min_runtime_adapter_experiences: Option<usize>,
    inspect_min_runtime_forward_energy_experiences: Option<usize>,
    inspect_min_runtime_kv_influence_experiences: Option<usize>,
    inspect_min_runtime_layer_mode_experiences: Option<usize>,
    inspect_min_runtime_all_layer_mode_experiences: Option<usize>,
    inspect_min_runtime_global_layers: Option<usize>,
    inspect_min_runtime_local_window_layers: Option<usize>,
    inspect_min_runtime_convolutional_fusion_layers: Option<usize>,
    inspect_min_runtime_kv_import_experiences: Option<usize>,
    inspect_min_runtime_kv_export_experiences: Option<usize>,
    inspect_min_runtime_kv_memory_device_profiles: Option<usize>,
    inspect_min_runtime_model_device_profiles: Option<usize>,
    inspect_min_runtime_adapter_device_profiles: Option<usize>,
    inspect_min_runtime_forward_energy_device_profiles: Option<usize>,
    inspect_min_runtime_kv_influence_device_profiles: Option<usize>,
    inspect_min_runtime_layer_mode_device_profiles: Option<usize>,
    inspect_min_runtime_all_layer_mode_device_profiles: Option<usize>,
    inspect_min_runtime_kv_import_device_profiles: Option<usize>,
    inspect_min_runtime_kv_export_device_profiles: Option<usize>,
    inspect_min_reflection_issue_experiences: Option<usize>,
    inspect_min_critical_reflection_issue_experiences: Option<usize>,
    inspect_min_revision_action_experiences: Option<usize>,
    inspect_min_live_memory_feedback_experiences: Option<usize>,
    inspect_min_live_memory_feedback_updates: Option<usize>,
    inspect_min_reflection_issue_device_profiles: Option<usize>,
    inspect_min_critical_reflection_issue_device_profiles: Option<usize>,
    inspect_min_revision_action_device_profiles: Option<usize>,
    inspect_min_live_memory_feedback_device_profiles: Option<usize>,
    inspect_min_evolution_live_inference_device_profiles: Option<usize>,
    inspect_min_evolution_live_router_threshold_mutation_device_profiles: Option<usize>,
    inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles: Option<usize>,
    inspect_min_evolution_live_memory_update_device_profiles: Option<usize>,
    inspect_min_evolution_live_stored_memory_update_device_profiles: Option<usize>,
    inspect_min_evolution_live_reflection_issue_device_profiles: Option<usize>,
    inspect_min_evolution_live_critical_reflection_issue_device_profiles: Option<usize>,
    inspect_min_evolution_live_revision_action_device_profiles: Option<usize>,
    inspect_min_evolution_replay_run_device_profiles: Option<usize>,
    inspect_min_evolution_replay_item_device_profiles: Option<usize>,
    inspect_min_evolution_router_threshold_mutation_device_profiles: Option<usize>,
    inspect_min_evolution_hierarchy_weight_mutation_device_profiles: Option<usize>,
    inspect_min_evolution_memory_update_device_profiles: Option<usize>,
    inspect_min_evolution_replay_live_memory_feedback_device_profiles: Option<usize>,
    inspect_min_evolution_recursive_replay_device_profiles: Option<usize>,
    inspect_min_evolution_recursive_runtime_call_device_profiles: Option<usize>,
    inspect_min_router_observations: Option<u64>,
    inspect_min_evolution_live_inference_runs: Option<u64>,
    inspect_min_evolution_live_router_threshold_mutations: Option<u64>,
    inspect_min_evolution_live_hierarchy_weight_mutations: Option<u64>,
    inspect_min_evolution_live_router_threshold_delta: Option<f32>,
    inspect_min_evolution_live_hierarchy_weight_delta: Option<f32>,
    inspect_min_evolution_live_memory_updates: Option<u64>,
    inspect_min_evolution_live_stored_memory_updates: Option<u64>,
    inspect_min_evolution_live_reflection_issues: Option<u64>,
    inspect_min_evolution_live_critical_reflection_issues: Option<u64>,
    inspect_min_evolution_live_revision_actions: Option<u64>,
    inspect_min_evolution_replay_runs: Option<u64>,
    inspect_min_evolution_replay_items: Option<u64>,
    inspect_min_evolution_router_threshold_mutations: Option<u64>,
    inspect_min_evolution_hierarchy_weight_mutations: Option<u64>,
    inspect_min_evolution_router_threshold_delta: Option<f32>,
    inspect_min_evolution_hierarchy_weight_delta: Option<f32>,
    inspect_min_evolution_memory_updates: Option<u64>,
    inspect_min_evolution_replay_live_memory_feedback_updates: Option<u64>,
    inspect_min_evolution_recursive_replay_items: Option<u64>,
    inspect_min_evolution_recursive_runtime_calls: Option<u64>,
    inspect_max_evolution_drift_rollbacks: Option<u64>,
    inspect_max_evolution_rollback_router_threshold_delta: Option<f32>,
    inspect_max_evolution_rollback_hierarchy_weight_delta: Option<f32>,
    inspect_require_runtime_kv_dimensions: bool,
    local_runtime: bool,
    production_runtime: bool,
    production_reference_kernel: bool,
    production_local_kernel: bool,
    production_kernel_conformance_gate: bool,
    production_kernel_conformance_all_devices_gate: bool,
    runtime_command: Option<PathBuf>,
    runtime_args: Vec<String>,
    runtime_prompt_mode: CommandPromptMode,
    runtime_wire_format: CommandWireFormat,
    runtime_metadata: RuntimeMetadata,
    native_window_tokens: usize,
    chunk_tokens: usize,
    chunk_overlap_tokens: usize,
    merge_fan_in: usize,
    replay_limit: usize,
    auto_replay_limit: usize,
    retention_stale_after: Option<u64>,
    retention_decay_rate: Option<f32>,
    retention_remove_below: Option<f32>,
    retention_remove_after_failures: Option<u64>,
    compaction_similarity_threshold: Option<f32>,
    compaction_max_candidates: Option<usize>,
    compaction_max_merges: Option<usize>,
    device: DeviceClass,
    cpu_load: f32,
    gpu_load: f32,
    ram_load: f32,
    disk_load: f32,
}

impl Args {
    fn for_inspect_device(&self, device: DeviceClass) -> Self {
        let mut args = self.clone();
        args.device = device;
        args.memory_path = device_scoped_path(&self.memory_path, device);
        args.experience_path = device_scoped_path(&self.experience_path, device);
        args.adaptive_path = device_scoped_path(&self.adaptive_path, device);
        args
    }

    fn for_roundtrip_device(&self, device: DeviceClass) -> Self {
        let mut args = self.clone();
        args.device = device;
        args.memory_path = device_scoped_path(&self.memory_path, device);
        args.experience_path = device_scoped_path(&self.experience_path, device);
        args.adaptive_path = device_scoped_path(&self.adaptive_path, device);
        args
    }

    fn parse(raw: Vec<String>) -> Self {
        let mut prompt_parts = Vec::new();
        let mut profile = None;
        let mut memory_path = PathBuf::from("noiron-memory.ndkv");
        let mut experience_path = PathBuf::from("noiron-experience.ndkv");
        let mut adaptive_path = PathBuf::from("noiron-adaptive.ndkv");
        let mut trace_path = None;
        let mut trace_schema_gate_path = None;
        let mut benchmark_path = None;
        let mut benchmark_all_devices = false;
        let mut benchmark_gate_enabled = false;
        let mut benchmark_min_quality = None;
        let mut benchmark_min_reward = None;
        let mut benchmark_max_total_ms = None;
        let mut benchmark_max_recursive_chunks = None;
        let mut benchmark_min_recursive_cases = None;
        let mut benchmark_min_recursive_runtime_calls = None;
        let mut benchmark_min_auto_replay_router_updates = None;
        let mut benchmark_min_auto_replay_hierarchy_updates = None;
        let mut benchmark_min_auto_replay_router_threshold_mutations = None;
        let mut benchmark_min_auto_replay_hierarchy_weight_mutations = None;
        let mut benchmark_min_auto_replay_router_threshold_delta = None;
        let mut benchmark_min_auto_replay_hierarchy_weight_delta = None;
        let mut benchmark_min_auto_replay_memory_updates = None;
        let mut benchmark_min_live_memory_feedback_updates = None;
        let mut benchmark_min_auto_replay_live_memory_feedback_updates = None;
        let mut benchmark_min_auto_replay_recursive_items = None;
        let mut benchmark_min_auto_replay_recursive_call_pressure = None;
        let mut benchmark_max_auto_replay_recursive_call_pressure = None;
        let mut benchmark_min_evolution_live_inference_runs = None;
        let mut benchmark_min_evolution_live_router_threshold_mutations = None;
        let mut benchmark_min_evolution_live_hierarchy_weight_mutations = None;
        let mut benchmark_min_evolution_live_router_threshold_delta = None;
        let mut benchmark_min_evolution_live_hierarchy_weight_delta = None;
        let mut benchmark_min_evolution_live_memory_updates = None;
        let mut benchmark_min_evolution_live_stored_memory_updates = None;
        let mut benchmark_min_evolution_live_reflection_issues = None;
        let mut benchmark_min_evolution_live_critical_reflection_issues = None;
        let mut benchmark_min_evolution_live_revision_actions = None;
        let mut benchmark_min_evolution_live_inference_device_profiles = None;
        let mut benchmark_min_evolution_live_router_threshold_mutation_device_profiles = None;
        let mut benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles = None;
        let mut benchmark_min_evolution_live_memory_update_device_profiles = None;
        let mut benchmark_min_evolution_live_stored_memory_update_device_profiles = None;
        let mut benchmark_min_evolution_live_reflection_issue_device_profiles = None;
        let mut benchmark_min_evolution_live_critical_reflection_issue_device_profiles = None;
        let mut benchmark_min_evolution_live_revision_action_device_profiles = None;
        let mut benchmark_min_evolution_replay_runs = None;
        let mut benchmark_min_evolution_replay_items = None;
        let mut benchmark_min_evolution_router_threshold_mutations = None;
        let mut benchmark_min_evolution_hierarchy_weight_mutations = None;
        let mut benchmark_min_evolution_router_threshold_delta = None;
        let mut benchmark_min_evolution_hierarchy_weight_delta = None;
        let mut benchmark_min_evolution_memory_updates = None;
        let mut benchmark_min_evolution_replay_live_memory_feedback_updates = None;
        let mut benchmark_min_evolution_recursive_replay_items = None;
        let mut benchmark_min_evolution_recursive_runtime_calls = None;
        let mut benchmark_max_evolution_drift_rollbacks = None;
        let mut benchmark_max_evolution_rollback_router_threshold_delta = None;
        let mut benchmark_max_evolution_rollback_hierarchy_weight_delta = None;
        let mut benchmark_min_sparse_skipped_cases = None;
        let mut benchmark_min_sparse_skipped_tokens = None;
        let mut benchmark_min_runtime_forward_cases = None;
        let mut benchmark_min_runtime_forward_energy_cases = None;
        let mut benchmark_min_runtime_kv_influence_cases = None;
        let mut benchmark_min_runtime_layer_mode_cases = None;
        let mut benchmark_min_runtime_all_layer_mode_cases = None;
        let mut benchmark_min_runtime_global_layers = None;
        let mut benchmark_min_runtime_local_window_layers = None;
        let mut benchmark_min_runtime_convolutional_fusion_layers = None;
        let mut benchmark_min_runtime_uncertainty_cases = None;
        let mut benchmark_min_runtime_uncertainty_tokens = None;
        let mut benchmark_min_runtime_kv_import_cases = None;
        let mut benchmark_min_runtime_kv_imported = None;
        let mut benchmark_min_runtime_kv_exported = None;
        let mut benchmark_min_runtime_kv_stored = None;
        let mut benchmark_min_runtime_adapter_contract_cases = None;
        let mut benchmark_min_runtime_adapter_kinds = None;
        let mut benchmark_min_runtime_adapter_observations = None;
        let mut benchmark_min_runtime_adapter_best_score = None;
        let mut benchmark_max_runtime_adapter_contract_violations = None;
        let mut benchmark_max_memory_governance_failures = None;
        let mut benchmark_min_memory_governance_cases = None;
        let mut benchmark_min_memory_governance_device_profiles = None;
        let mut benchmark_min_memory_retention_activity_cases = None;
        let mut benchmark_min_memory_compaction_activity_cases = None;
        let mut benchmark_min_reflection_issue_cases = None;
        let mut benchmark_min_reflection_issues = None;
        let mut benchmark_min_critical_reflection_issue_cases = None;
        let mut benchmark_min_critical_reflection_issues = None;
        let mut benchmark_min_revision_action_cases = None;
        let mut benchmark_min_revision_actions = None;
        let mut benchmark_min_reflection_issue_device_profiles = None;
        let mut benchmark_min_critical_reflection_issue_device_profiles = None;
        let mut benchmark_min_revision_action_device_profiles = None;
        let mut benchmark_min_device_profiles = None;
        let mut benchmark_min_recursive_device_profiles = None;
        let mut benchmark_max_drift_blocks = None;
        let mut benchmark_max_drift_rollbacks = None;
        let mut benchmark_roundtrip = false;
        let mut list_devices = false;
        let mut device_gate = false;
        let mut kv_quant_gate = false;
        let mut kv_quant_max_total_us = None;
        let mut runtime_manifest_gate = false;
        let mut runtime_manifest_all_devices_gate = false;
        let mut runtime_weights_path = None;
        let mut runtime_tokenizer_path = None;
        let mut runtime_config_path = None;
        let mut runtime_layer_count = None;
        let mut runtime_hidden_size = None;
        let mut runtime_attention_heads = None;
        let mut runtime_kv_heads = None;
        let mut runtime_local_window_tokens = None;
        let mut inspect_state = false;
        let mut inspect_limit = 5;
        let mut inspect_gate = false;
        let mut inspect_min_memories = None;
        let mut inspect_min_runtime_kv_memories = None;
        let mut inspect_min_experiences = None;
        let mut inspect_min_runtime_model_experiences = None;
        let mut inspect_min_runtime_adapter_experiences = None;
        let mut inspect_min_runtime_forward_energy_experiences = None;
        let mut inspect_min_runtime_kv_influence_experiences = None;
        let mut inspect_min_runtime_layer_mode_experiences = None;
        let mut inspect_min_runtime_all_layer_mode_experiences = None;
        let mut inspect_min_runtime_global_layers = None;
        let mut inspect_min_runtime_local_window_layers = None;
        let mut inspect_min_runtime_convolutional_fusion_layers = None;
        let mut inspect_min_runtime_kv_import_experiences = None;
        let mut inspect_min_runtime_kv_export_experiences = None;
        let mut inspect_min_runtime_kv_memory_device_profiles = None;
        let mut inspect_min_runtime_model_device_profiles = None;
        let mut inspect_min_runtime_adapter_device_profiles = None;
        let mut inspect_min_runtime_forward_energy_device_profiles = None;
        let mut inspect_min_runtime_kv_influence_device_profiles = None;
        let mut inspect_min_runtime_layer_mode_device_profiles = None;
        let mut inspect_min_runtime_all_layer_mode_device_profiles = None;
        let mut inspect_min_runtime_kv_import_device_profiles = None;
        let mut inspect_min_runtime_kv_export_device_profiles = None;
        let mut inspect_min_reflection_issue_experiences = None;
        let mut inspect_min_critical_reflection_issue_experiences = None;
        let mut inspect_min_revision_action_experiences = None;
        let mut inspect_min_live_memory_feedback_experiences = None;
        let mut inspect_min_live_memory_feedback_updates = None;
        let mut inspect_min_reflection_issue_device_profiles = None;
        let mut inspect_min_critical_reflection_issue_device_profiles = None;
        let mut inspect_min_revision_action_device_profiles = None;
        let mut inspect_min_live_memory_feedback_device_profiles = None;
        let mut inspect_min_evolution_live_inference_device_profiles = None;
        let mut inspect_min_evolution_live_router_threshold_mutation_device_profiles = None;
        let mut inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles = None;
        let mut inspect_min_evolution_live_memory_update_device_profiles = None;
        let mut inspect_min_evolution_live_stored_memory_update_device_profiles = None;
        let mut inspect_min_evolution_live_reflection_issue_device_profiles = None;
        let mut inspect_min_evolution_live_critical_reflection_issue_device_profiles = None;
        let mut inspect_min_evolution_live_revision_action_device_profiles = None;
        let mut inspect_min_evolution_replay_run_device_profiles = None;
        let mut inspect_min_evolution_replay_item_device_profiles = None;
        let mut inspect_min_evolution_router_threshold_mutation_device_profiles = None;
        let mut inspect_min_evolution_hierarchy_weight_mutation_device_profiles = None;
        let mut inspect_min_evolution_memory_update_device_profiles = None;
        let mut inspect_min_evolution_replay_live_memory_feedback_device_profiles = None;
        let mut inspect_min_evolution_recursive_replay_device_profiles = None;
        let mut inspect_min_evolution_recursive_runtime_call_device_profiles = None;
        let mut inspect_min_router_observations = None;
        let mut inspect_min_evolution_live_inference_runs = None;
        let mut inspect_min_evolution_live_router_threshold_mutations = None;
        let mut inspect_min_evolution_live_hierarchy_weight_mutations = None;
        let mut inspect_min_evolution_live_router_threshold_delta = None;
        let mut inspect_min_evolution_live_hierarchy_weight_delta = None;
        let mut inspect_min_evolution_live_memory_updates = None;
        let mut inspect_min_evolution_live_stored_memory_updates = None;
        let mut inspect_min_evolution_live_reflection_issues = None;
        let mut inspect_min_evolution_live_critical_reflection_issues = None;
        let mut inspect_min_evolution_live_revision_actions = None;
        let mut inspect_min_evolution_replay_runs = None;
        let mut inspect_min_evolution_replay_items = None;
        let mut inspect_min_evolution_router_threshold_mutations = None;
        let mut inspect_min_evolution_hierarchy_weight_mutations = None;
        let mut inspect_min_evolution_router_threshold_delta = None;
        let mut inspect_min_evolution_hierarchy_weight_delta = None;
        let mut inspect_min_evolution_memory_updates = None;
        let mut inspect_min_evolution_replay_live_memory_feedback_updates = None;
        let mut inspect_min_evolution_recursive_replay_items = None;
        let mut inspect_min_evolution_recursive_runtime_calls = None;
        let mut inspect_max_evolution_drift_rollbacks = None;
        let mut inspect_max_evolution_rollback_router_threshold_delta = None;
        let mut inspect_max_evolution_rollback_hierarchy_weight_delta = None;
        let mut inspect_require_runtime_kv_dimensions = false;
        let mut local_runtime = false;
        let mut production_runtime = false;
        let mut production_reference_kernel = false;
        let mut production_local_kernel = false;
        let mut production_kernel_conformance_gate = false;
        let mut production_kernel_conformance_all_devices_gate = false;
        let mut runtime_command = None;
        let mut runtime_args = Vec::new();
        let mut runtime_prompt_mode = CommandPromptMode::Stdin;
        let mut runtime_wire_format = CommandWireFormat::Text;
        let mut runtime_metadata = RuntimeMetadata::default();
        let default_scheduler = RecursiveScheduler::default();
        let mut native_window_tokens = default_scheduler.native_window_tokens();
        let mut chunk_tokens = default_scheduler.chunk_tokens();
        let mut chunk_overlap_tokens = default_scheduler.overlap_tokens();
        let mut merge_fan_in = default_scheduler.merge_fan_in();
        let mut replay_limit = 0;
        let mut auto_replay_limit = 2;
        let mut retention_stale_after = None;
        let mut retention_decay_rate = None;
        let mut retention_remove_below = None;
        let mut retention_remove_after_failures = None;
        let mut compaction_similarity_threshold = None;
        let mut compaction_max_candidates = None;
        let mut compaction_max_merges = None;
        let default_hardware = HardwareSnapshot::default();
        let mut device = default_hardware.device;
        let mut cpu_load = default_hardware.cpu_load;
        let mut gpu_load = default_hardware.gpu_load;
        let mut ram_load = default_hardware.ram_load;
        let mut disk_load = default_hardware.disk_load;
        let mut cpu_load_set = false;
        let mut gpu_load_set = false;
        let mut ram_load_set = false;
        let mut disk_load_set = false;
        let mut index = 0;

        while index < raw.len() {
            match raw[index].as_str() {
                "--profile" | "-p" if index + 1 < raw.len() => {
                    profile = raw[index + 1].parse::<TaskProfile>().ok();
                    index += 2;
                }
                "--memory" | "-m" if index + 1 < raw.len() => {
                    memory_path = PathBuf::from(&raw[index + 1]);
                    index += 2;
                }
                "--experience" | "-e" if index + 1 < raw.len() => {
                    experience_path = PathBuf::from(&raw[index + 1]);
                    index += 2;
                }
                "--adaptive" | "-a" if index + 1 < raw.len() => {
                    adaptive_path = PathBuf::from(&raw[index + 1]);
                    index += 2;
                }
                "--trace" if index + 1 < raw.len() => {
                    trace_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--trace-schema-gate" | "--trace-gate" if index + 1 < raw.len() => {
                    trace_schema_gate_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--benchmark" if index + 1 < raw.len() => {
                    benchmark_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--benchmark-gate" => {
                    benchmark_gate_enabled = true;
                    index += 1;
                }
                "--benchmark-all-devices" => {
                    benchmark_all_devices = true;
                    if inspect_state {
                        inspect_gate = true;
                    }
                    index += 1;
                }
                "--benchmark-min-quality" if index + 1 < raw.len() => {
                    benchmark_min_quality = Some(parse_f32(&raw[index + 1], 0.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-reward" if index + 1 < raw.len() => {
                    benchmark_min_reward = Some(parse_f32(&raw[index + 1], 0.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-max-total-ms" if index + 1 < raw.len() => {
                    benchmark_max_total_ms = Some(parse_u128(&raw[index + 1], u128::MAX));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-max-recursive-chunks" if index + 1 < raw.len() => {
                    benchmark_max_recursive_chunks = Some(parse_usize(&raw[index + 1], usize::MAX));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-recursive-cases" if index + 1 < raw.len() => {
                    benchmark_min_recursive_cases = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-recursive-runtime-calls" if index + 1 < raw.len() => {
                    benchmark_min_recursive_runtime_calls = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-auto-replay-router-updates" if index + 1 < raw.len() => {
                    benchmark_min_auto_replay_router_updates =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-auto-replay-hierarchy-updates" if index + 1 < raw.len() => {
                    benchmark_min_auto_replay_hierarchy_updates =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-auto-replay-router-threshold-mutations"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_auto_replay_router_threshold_mutations =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-auto-replay-hierarchy-weight-mutations"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_auto_replay_hierarchy_weight_mutations =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-auto-replay-router-threshold-delta" if index + 1 < raw.len() => {
                    benchmark_min_auto_replay_router_threshold_delta =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-auto-replay-hierarchy-weight-delta" if index + 1 < raw.len() => {
                    benchmark_min_auto_replay_hierarchy_weight_delta =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-auto-replay-memory-updates" if index + 1 < raw.len() => {
                    benchmark_min_auto_replay_memory_updates =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-live-memory-feedback-updates" if index + 1 < raw.len() => {
                    benchmark_min_live_memory_feedback_updates =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-auto-replay-live-memory-feedback-updates"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_auto_replay_live_memory_feedback_updates =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-auto-replay-recursive-items" if index + 1 < raw.len() => {
                    benchmark_min_auto_replay_recursive_items =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-auto-replay-recursive-call-pressure" if index + 1 < raw.len() => {
                    benchmark_min_auto_replay_recursive_call_pressure =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-max-auto-replay-recursive-call-pressure" if index + 1 < raw.len() => {
                    benchmark_max_auto_replay_recursive_call_pressure =
                        Some(parse_f32(&raw[index + 1], 1.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-inference-runs" if index + 1 < raw.len() => {
                    benchmark_min_evolution_live_inference_runs =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-router-threshold-mutations"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_router_threshold_mutations =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-hierarchy-weight-mutations"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_hierarchy_weight_mutations =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-router-threshold-delta"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_router_threshold_delta =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-hierarchy-weight-delta"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_hierarchy_weight_delta =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-memory-updates" if index + 1 < raw.len() => {
                    benchmark_min_evolution_live_memory_updates =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-stored-memory-updates" if index + 1 < raw.len() => {
                    benchmark_min_evolution_live_stored_memory_updates =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-reflection-issues" if index + 1 < raw.len() => {
                    benchmark_min_evolution_live_reflection_issues =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-critical-reflection-issues"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_critical_reflection_issues =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-revision-actions" if index + 1 < raw.len() => {
                    benchmark_min_evolution_live_revision_actions =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-inference-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_inference_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-router-threshold-mutation-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_router_threshold_mutation_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-hierarchy-weight-mutation-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-memory-update-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_memory_update_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-stored-memory-update-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_stored_memory_update_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-reflection-issue-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_reflection_issue_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-critical-reflection-issue-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_critical_reflection_issue_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--benchmark-min-evolution-live-revision-action-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_live_revision_action_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--benchmark-min-evolution-replay-runs" if index + 1 < raw.len() => {
                    benchmark_min_evolution_replay_runs = Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-replay-items" if index + 1 < raw.len() => {
                    benchmark_min_evolution_replay_items = Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-router-threshold-mutations" if index + 1 < raw.len() => {
                    benchmark_min_evolution_router_threshold_mutations =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-hierarchy-weight-mutations" if index + 1 < raw.len() => {
                    benchmark_min_evolution_hierarchy_weight_mutations =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-router-threshold-delta" if index + 1 < raw.len() => {
                    benchmark_min_evolution_router_threshold_delta =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-hierarchy-weight-delta" if index + 1 < raw.len() => {
                    benchmark_min_evolution_hierarchy_weight_delta =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-memory-updates" if index + 1 < raw.len() => {
                    benchmark_min_evolution_memory_updates = Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-replay-live-memory-feedback-updates"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_evolution_replay_live_memory_feedback_updates =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-recursive-replay-items" if index + 1 < raw.len() => {
                    benchmark_min_evolution_recursive_replay_items =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-evolution-recursive-runtime-calls" if index + 1 < raw.len() => {
                    benchmark_min_evolution_recursive_runtime_calls =
                        Some(parse_u64(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-max-evolution-drift-rollbacks" if index + 1 < raw.len() => {
                    benchmark_max_evolution_drift_rollbacks =
                        Some(parse_u64(&raw[index + 1], u64::MAX));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-max-evolution-rollback-router-threshold-delta"
                    if index + 1 < raw.len() =>
                {
                    benchmark_max_evolution_rollback_router_threshold_delta =
                        Some(parse_f32(&raw[index + 1], f32::MAX));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-max-evolution-rollback-hierarchy-weight-delta"
                    if index + 1 < raw.len() =>
                {
                    benchmark_max_evolution_rollback_hierarchy_weight_delta =
                        Some(parse_f32(&raw[index + 1], f32::MAX));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-sparse-skipped-cases" if index + 1 < raw.len() => {
                    benchmark_min_sparse_skipped_cases = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-sparse-skipped-tokens" if index + 1 < raw.len() => {
                    benchmark_min_sparse_skipped_tokens = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-forward-cases" if index + 1 < raw.len() => {
                    benchmark_min_runtime_forward_cases = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-forward-energy-cases" if index + 1 < raw.len() => {
                    benchmark_min_runtime_forward_energy_cases =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-kv-influence-cases" if index + 1 < raw.len() => {
                    benchmark_min_runtime_kv_influence_cases =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-layer-mode-cases" if index + 1 < raw.len() => {
                    benchmark_min_runtime_layer_mode_cases = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-all-layer-mode-cases" if index + 1 < raw.len() => {
                    benchmark_min_runtime_all_layer_mode_cases =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-global-layers" if index + 1 < raw.len() => {
                    benchmark_min_runtime_global_layers = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-local-window-layers" if index + 1 < raw.len() => {
                    benchmark_min_runtime_local_window_layers =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-convolutional-fusion-layers" if index + 1 < raw.len() => {
                    benchmark_min_runtime_convolutional_fusion_layers =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-uncertainty-cases" if index + 1 < raw.len() => {
                    benchmark_min_runtime_uncertainty_cases = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-uncertainty-tokens" if index + 1 < raw.len() => {
                    benchmark_min_runtime_uncertainty_tokens =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-kv-import-cases" if index + 1 < raw.len() => {
                    benchmark_min_runtime_kv_import_cases = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-kv-imported" if index + 1 < raw.len() => {
                    benchmark_min_runtime_kv_imported = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-kv-exported" if index + 1 < raw.len() => {
                    benchmark_min_runtime_kv_exported = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-kv-stored" if index + 1 < raw.len() => {
                    benchmark_min_runtime_kv_stored = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-adapter-contract-cases" if index + 1 < raw.len() => {
                    benchmark_min_runtime_adapter_contract_cases =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-adapter-kinds" if index + 1 < raw.len() => {
                    benchmark_min_runtime_adapter_kinds = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-adapter-observations" if index + 1 < raw.len() => {
                    benchmark_min_runtime_adapter_observations =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-runtime-adapter-best-score" if index + 1 < raw.len() => {
                    benchmark_min_runtime_adapter_best_score =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-max-runtime-adapter-contract-violations" if index + 1 < raw.len() => {
                    benchmark_max_runtime_adapter_contract_violations =
                        Some(parse_usize(&raw[index + 1], usize::MAX));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-max-memory-governance-failures" if index + 1 < raw.len() => {
                    benchmark_max_memory_governance_failures =
                        Some(parse_usize(&raw[index + 1], usize::MAX));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-memory-governance-cases" if index + 1 < raw.len() => {
                    benchmark_min_memory_governance_cases = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-memory-governance-device-profiles" if index + 1 < raw.len() => {
                    benchmark_min_memory_governance_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--benchmark-min-memory-retention-activity-cases" if index + 1 < raw.len() => {
                    benchmark_min_memory_retention_activity_cases =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-memory-compaction-activity-cases" if index + 1 < raw.len() => {
                    benchmark_min_memory_compaction_activity_cases =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-reflection-issue-cases" if index + 1 < raw.len() => {
                    benchmark_min_reflection_issue_cases = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-reflection-issues" if index + 1 < raw.len() => {
                    benchmark_min_reflection_issues = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-critical-reflection-issue-cases" if index + 1 < raw.len() => {
                    benchmark_min_critical_reflection_issue_cases =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-critical-reflection-issues" if index + 1 < raw.len() => {
                    benchmark_min_critical_reflection_issues =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-revision-action-cases" if index + 1 < raw.len() => {
                    benchmark_min_revision_action_cases = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-revision-actions" if index + 1 < raw.len() => {
                    benchmark_min_revision_actions = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-reflection-issue-device-profiles" if index + 1 < raw.len() => {
                    benchmark_min_reflection_issue_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-critical-reflection-issue-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    benchmark_min_critical_reflection_issue_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-revision-action-device-profiles" if index + 1 < raw.len() => {
                    benchmark_min_revision_action_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-device-profiles" if index + 1 < raw.len() => {
                    benchmark_min_device_profiles = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-min-recursive-device-profiles" if index + 1 < raw.len() => {
                    benchmark_min_recursive_device_profiles = Some(parse_usize(&raw[index + 1], 0));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-max-drift-blocks" if index + 1 < raw.len() => {
                    benchmark_max_drift_blocks = Some(parse_usize(&raw[index + 1], usize::MAX));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-max-drift-rollbacks" if index + 1 < raw.len() => {
                    benchmark_max_drift_rollbacks = Some(parse_usize(&raw[index + 1], usize::MAX));
                    benchmark_gate_enabled = true;
                    index += 2;
                }
                "--benchmark-roundtrip" | "--roundtrip-gate" => {
                    benchmark_roundtrip = true;
                    index += 1;
                }
                "--list-devices" => {
                    list_devices = true;
                    index += 1;
                }
                "--device-gate" => {
                    device_gate = true;
                    index += 1;
                }
                "--kv-quant-gate" => {
                    kv_quant_gate = true;
                    index += 1;
                }
                "--kv-quant-max-total-us" if index + 1 < raw.len() => {
                    kv_quant_max_total_us = Some(parse_u128(&raw[index + 1], u128::MAX));
                    kv_quant_gate = true;
                    index += 2;
                }
                "--runtime-manifest-gate" => {
                    runtime_manifest_gate = true;
                    index += 1;
                }
                "--runtime-manifest-all-devices-gate" => {
                    runtime_manifest_gate = true;
                    runtime_manifest_all_devices_gate = true;
                    index += 1;
                }
                "--runtime-weights" if index + 1 < raw.len() => {
                    runtime_weights_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--runtime-tokenizer-path" if index + 1 < raw.len() => {
                    runtime_tokenizer_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--runtime-config" if index + 1 < raw.len() => {
                    runtime_config_path = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--runtime-layers" if index + 1 < raw.len() => {
                    runtime_layer_count = Some(parse_usize(&raw[index + 1], 0));
                    index += 2;
                }
                "--runtime-hidden-size" if index + 1 < raw.len() => {
                    runtime_hidden_size = Some(parse_usize(&raw[index + 1], 0));
                    index += 2;
                }
                "--runtime-attention-heads" if index + 1 < raw.len() => {
                    runtime_attention_heads = Some(parse_usize(&raw[index + 1], 0));
                    index += 2;
                }
                "--runtime-kv-heads" if index + 1 < raw.len() => {
                    runtime_kv_heads = Some(parse_usize(&raw[index + 1], 0));
                    index += 2;
                }
                "--runtime-local-window" if index + 1 < raw.len() => {
                    runtime_local_window_tokens = Some(parse_usize(&raw[index + 1], 0));
                    index += 2;
                }
                "--inspect-state" => {
                    inspect_state = true;
                    if benchmark_all_devices {
                        inspect_gate = true;
                    }
                    index += 1;
                }
                "--inspect-gate" => {
                    inspect_state = true;
                    inspect_gate = true;
                    index += 1;
                }
                "--inspect-limit" if index + 1 < raw.len() => {
                    inspect_limit = parse_usize(&raw[index + 1], inspect_limit).max(1);
                    inspect_state = true;
                    index += 2;
                }
                "--inspect-min-memories" if index + 1 < raw.len() => {
                    inspect_min_memories = Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-kv-memories" if index + 1 < raw.len() => {
                    inspect_min_runtime_kv_memories = Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-experiences" if index + 1 < raw.len() => {
                    inspect_min_experiences = Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-model-experiences" if index + 1 < raw.len() => {
                    inspect_min_runtime_model_experiences = Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-adapter-experiences" if index + 1 < raw.len() => {
                    inspect_min_runtime_adapter_experiences = Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-forward-energy-experiences" if index + 1 < raw.len() => {
                    inspect_min_runtime_forward_energy_experiences =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-kv-influence-experiences" if index + 1 < raw.len() => {
                    inspect_min_runtime_kv_influence_experiences =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-layer-mode-experiences" if index + 1 < raw.len() => {
                    inspect_min_runtime_layer_mode_experiences =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-all-layer-mode-experiences" if index + 1 < raw.len() => {
                    inspect_min_runtime_all_layer_mode_experiences =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-global-layers" if index + 1 < raw.len() => {
                    inspect_min_runtime_global_layers = Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-local-window-layers" if index + 1 < raw.len() => {
                    inspect_min_runtime_local_window_layers = Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-convolutional-fusion-layers" if index + 1 < raw.len() => {
                    inspect_min_runtime_convolutional_fusion_layers =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-kv-import-experiences" if index + 1 < raw.len() => {
                    inspect_min_runtime_kv_import_experiences =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-kv-export-experiences" if index + 1 < raw.len() => {
                    inspect_min_runtime_kv_export_experiences =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-runtime-kv-memory-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_runtime_kv_memory_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-runtime-model-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_runtime_model_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-runtime-adapter-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_runtime_adapter_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-runtime-forward-energy-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_runtime_forward_energy_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-runtime-kv-influence-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_runtime_kv_influence_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-runtime-layer-mode-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_runtime_layer_mode_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-runtime-all-layer-mode-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_runtime_all_layer_mode_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-runtime-kv-import-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_runtime_kv_import_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-runtime-kv-export-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_runtime_kv_export_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-reflection-issue-experiences" if index + 1 < raw.len() => {
                    inspect_min_reflection_issue_experiences =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-critical-reflection-issue-experiences" if index + 1 < raw.len() => {
                    inspect_min_critical_reflection_issue_experiences =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-revision-action-experiences" if index + 1 < raw.len() => {
                    inspect_min_revision_action_experiences = Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-live-memory-feedback-experiences" if index + 1 < raw.len() => {
                    inspect_min_live_memory_feedback_experiences =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-live-memory-feedback-updates" if index + 1 < raw.len() => {
                    inspect_min_live_memory_feedback_updates =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-reflection-issue-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_reflection_issue_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-critical-reflection-issue-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_critical_reflection_issue_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-revision-action-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_revision_action_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-live-memory-feedback-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_live_memory_feedback_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-inference-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_inference_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-router-threshold-mutation-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_router_threshold_mutation_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-hierarchy-weight-mutation-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-memory-update-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_memory_update_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-stored-memory-update-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_stored_memory_update_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-reflection-issue-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_reflection_issue_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-critical-reflection-issue-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_critical_reflection_issue_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-revision-action-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_revision_action_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-replay-run-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_evolution_replay_run_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-replay-item-device-profiles" if index + 1 < raw.len() => {
                    inspect_min_evolution_replay_item_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-router-threshold-mutation-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_router_threshold_mutation_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-hierarchy-weight-mutation-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_hierarchy_weight_mutation_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-memory-update-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_memory_update_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-replay-live-memory-feedback-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_replay_live_memory_feedback_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-recursive-replay-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_recursive_replay_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-evolution-recursive-runtime-call-device-profiles"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_recursive_runtime_call_device_profiles =
                        Some(parse_usize(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    benchmark_all_devices = true;
                    index += 2;
                }
                "--inspect-min-router-observations" if index + 1 < raw.len() => {
                    inspect_min_router_observations = Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-inference-runs" if index + 1 < raw.len() => {
                    inspect_min_evolution_live_inference_runs = Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-router-threshold-mutations"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_router_threshold_mutations =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-hierarchy-weight-mutations"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_hierarchy_weight_mutations =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-router-threshold-delta" if index + 1 < raw.len() => {
                    inspect_min_evolution_live_router_threshold_delta =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-hierarchy-weight-delta" if index + 1 < raw.len() => {
                    inspect_min_evolution_live_hierarchy_weight_delta =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-memory-updates" if index + 1 < raw.len() => {
                    inspect_min_evolution_live_memory_updates = Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-stored-memory-updates" if index + 1 < raw.len() => {
                    inspect_min_evolution_live_stored_memory_updates =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-reflection-issues" if index + 1 < raw.len() => {
                    inspect_min_evolution_live_reflection_issues =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-critical-reflection-issues"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_live_critical_reflection_issues =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-live-revision-actions" if index + 1 < raw.len() => {
                    inspect_min_evolution_live_revision_actions =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-replay-runs" if index + 1 < raw.len() => {
                    inspect_min_evolution_replay_runs = Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-replay-items" if index + 1 < raw.len() => {
                    inspect_min_evolution_replay_items = Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-router-threshold-mutations" if index + 1 < raw.len() => {
                    inspect_min_evolution_router_threshold_mutations =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-hierarchy-weight-mutations" if index + 1 < raw.len() => {
                    inspect_min_evolution_hierarchy_weight_mutations =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-router-threshold-delta" if index + 1 < raw.len() => {
                    inspect_min_evolution_router_threshold_delta =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-hierarchy-weight-delta" if index + 1 < raw.len() => {
                    inspect_min_evolution_hierarchy_weight_delta =
                        Some(parse_f32(&raw[index + 1], 0.0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-memory-updates" if index + 1 < raw.len() => {
                    inspect_min_evolution_memory_updates = Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-replay-live-memory-feedback-updates"
                    if index + 1 < raw.len() =>
                {
                    inspect_min_evolution_replay_live_memory_feedback_updates =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-recursive-replay-items" if index + 1 < raw.len() => {
                    inspect_min_evolution_recursive_replay_items =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-min-evolution-recursive-runtime-calls" if index + 1 < raw.len() => {
                    inspect_min_evolution_recursive_runtime_calls =
                        Some(parse_u64(&raw[index + 1], 0));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-max-evolution-drift-rollbacks" if index + 1 < raw.len() => {
                    inspect_max_evolution_drift_rollbacks =
                        Some(parse_u64(&raw[index + 1], u64::MAX));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-max-evolution-rollback-router-threshold-delta"
                    if index + 1 < raw.len() =>
                {
                    inspect_max_evolution_rollback_router_threshold_delta =
                        Some(parse_f32(&raw[index + 1], f32::MAX));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-max-evolution-rollback-hierarchy-weight-delta"
                    if index + 1 < raw.len() =>
                {
                    inspect_max_evolution_rollback_hierarchy_weight_delta =
                        Some(parse_f32(&raw[index + 1], f32::MAX));
                    inspect_state = true;
                    inspect_gate = true;
                    index += 2;
                }
                "--inspect-require-runtime-kv-dimensions" => {
                    inspect_require_runtime_kv_dimensions = true;
                    inspect_state = true;
                    inspect_gate = true;
                    index += 1;
                }
                "--local-runtime" => {
                    local_runtime = true;
                    runtime_metadata.supports_kv_import = true;
                    runtime_metadata.supports_kv_export = true;
                    index += 1;
                }
                "--production-runtime" => {
                    production_runtime = true;
                    runtime_metadata.supports_kv_import = true;
                    runtime_metadata.supports_kv_export = true;
                    index += 1;
                }
                "--production-reference-kernel" => {
                    production_runtime = true;
                    production_reference_kernel = true;
                    runtime_metadata.supports_kv_import = true;
                    runtime_metadata.supports_kv_export = true;
                    index += 1;
                }
                "--production-local-kernel" => {
                    production_runtime = true;
                    production_local_kernel = true;
                    runtime_metadata.supports_kv_import = true;
                    runtime_metadata.supports_kv_export = true;
                    index += 1;
                }
                "--production-kernel-conformance-gate" => {
                    production_runtime = true;
                    production_kernel_conformance_gate = true;
                    runtime_metadata.supports_kv_import = true;
                    runtime_metadata.supports_kv_export = true;
                    index += 1;
                }
                "--production-kernel-conformance-all-devices-gate" => {
                    production_runtime = true;
                    production_kernel_conformance_gate = true;
                    production_kernel_conformance_all_devices_gate = true;
                    runtime_metadata.supports_kv_import = true;
                    runtime_metadata.supports_kv_export = true;
                    index += 1;
                }
                "--runtime-command" if index + 1 < raw.len() => {
                    runtime_command = Some(PathBuf::from(&raw[index + 1]));
                    index += 2;
                }
                "--runtime-arg" if index + 1 < raw.len() => {
                    runtime_args.push(raw[index + 1].clone());
                    index += 2;
                }
                "--runtime-prompt-mode" if index + 1 < raw.len() => {
                    runtime_prompt_mode = match raw[index + 1].as_str() {
                        "args" => CommandPromptMode::Args,
                        _ => CommandPromptMode::Stdin,
                    };
                    index += 2;
                }
                "--runtime-wire-format" if index + 1 < raw.len() => {
                    runtime_wire_format = match raw[index + 1].as_str() {
                        "json" => CommandWireFormat::Json,
                        _ => CommandWireFormat::Text,
                    };
                    index += 2;
                }
                "--runtime-json" => {
                    runtime_wire_format = CommandWireFormat::Json;
                    index += 1;
                }
                "--runtime-model-id" if index + 1 < raw.len() => {
                    runtime_metadata.model_id = raw[index + 1].clone();
                    index += 2;
                }
                "--runtime-tokenizer" if index + 1 < raw.len() => {
                    runtime_metadata.tokenizer = raw[index + 1].clone();
                    index += 2;
                }
                "--runtime-native-window" if index + 1 < raw.len() => {
                    runtime_metadata.native_context_window =
                        parse_usize(&raw[index + 1], runtime_metadata.native_context_window);
                    index += 2;
                }
                "--runtime-embedding-dims" if index + 1 < raw.len() => {
                    runtime_metadata.embedding_dimensions =
                        parse_usize(&raw[index + 1], runtime_metadata.embedding_dimensions);
                    index += 2;
                }
                "--runtime-kv-import" => {
                    runtime_metadata.supports_kv_import = true;
                    index += 1;
                }
                "--runtime-kv-export" => {
                    runtime_metadata.supports_kv_export = true;
                    index += 1;
                }
                "--runtime-kv-exchange" => {
                    runtime_metadata.supports_kv_import = true;
                    runtime_metadata.supports_kv_export = true;
                    index += 1;
                }
                "--native-window" if index + 1 < raw.len() => {
                    native_window_tokens = parse_usize(&raw[index + 1], native_window_tokens);
                    index += 2;
                }
                "--chunk-tokens" if index + 1 < raw.len() => {
                    chunk_tokens = parse_usize(&raw[index + 1], chunk_tokens);
                    index += 2;
                }
                "--chunk-overlap" if index + 1 < raw.len() => {
                    chunk_overlap_tokens = parse_usize(&raw[index + 1], chunk_overlap_tokens);
                    index += 2;
                }
                "--merge-fan-in" if index + 1 < raw.len() => {
                    merge_fan_in = parse_usize(&raw[index + 1], merge_fan_in);
                    index += 2;
                }
                "--replay" if index + 1 < raw.len() => {
                    replay_limit = parse_usize(&raw[index + 1], replay_limit);
                    index += 2;
                }
                "--auto-replay" if index + 1 < raw.len() => {
                    auto_replay_limit = parse_usize(&raw[index + 1], auto_replay_limit);
                    index += 2;
                }
                "--retention-stale-after" if index + 1 < raw.len() => {
                    retention_stale_after = Some(parse_u64(&raw[index + 1], 64));
                    index += 2;
                }
                "--retention-decay-rate" if index + 1 < raw.len() => {
                    retention_decay_rate = Some(parse_f32(&raw[index + 1], 0.04));
                    index += 2;
                }
                "--retention-remove-below" if index + 1 < raw.len() => {
                    retention_remove_below = Some(parse_f32(&raw[index + 1], 0.04));
                    index += 2;
                }
                "--retention-remove-after-failures" if index + 1 < raw.len() => {
                    retention_remove_after_failures = Some(parse_u64(&raw[index + 1], 4));
                    index += 2;
                }
                "--compaction-threshold" if index + 1 < raw.len() => {
                    compaction_similarity_threshold = Some(parse_f32(&raw[index + 1], 0.92));
                    index += 2;
                }
                "--compaction-max-candidates" if index + 1 < raw.len() => {
                    compaction_max_candidates = Some(parse_usize(&raw[index + 1], 512));
                    index += 2;
                }
                "--compaction-max-merges" if index + 1 < raw.len() => {
                    compaction_max_merges = Some(parse_usize(&raw[index + 1], 32));
                    index += 2;
                }
                "--device" if index + 1 < raw.len() => {
                    device = parse_device_or_generic(&raw[index + 1]);
                    index += 2;
                }
                "--cpu-load" if index + 1 < raw.len() => {
                    cpu_load = parse_f32(&raw[index + 1], cpu_load);
                    cpu_load_set = true;
                    index += 2;
                }
                "--gpu-load" if index + 1 < raw.len() => {
                    gpu_load = parse_f32(&raw[index + 1], gpu_load);
                    gpu_load_set = true;
                    index += 2;
                }
                "--ram-load" if index + 1 < raw.len() => {
                    ram_load = parse_f32(&raw[index + 1], ram_load);
                    ram_load_set = true;
                    index += 2;
                }
                "--disk-load" if index + 1 < raw.len() => {
                    disk_load = parse_f32(&raw[index + 1], disk_load);
                    disk_load_set = true;
                    index += 2;
                }
                "--help" | "-h" => {
                    print_help_and_exit();
                }
                value => {
                    prompt_parts.push(value.to_owned());
                    index += 1;
                }
            }
        }

        let prompt = if prompt_parts.is_empty() {
            "Design a Rust Noiron prototype with adaptive routing, KV fusion, hierarchy control, and reflection."
                .to_owned()
        } else {
            prompt_parts.join(" ")
        };
        runtime_metadata = normalize_runtime_metadata(runtime_metadata);
        let profile = profile.unwrap_or_else(|| detect_profile(&prompt));
        if inspect_state && benchmark_all_devices {
            inspect_gate = true;
        }
        if device == DeviceClass::Auto {
            let detected = HardwareSnapshot::auto_detect();
            device = detected.device;
            if !cpu_load_set {
                cpu_load = detected.cpu_load;
            }
            if !gpu_load_set {
                gpu_load = detected.gpu_load;
            }
            if !ram_load_set {
                ram_load = detected.ram_load;
            }
            if !disk_load_set {
                disk_load = detected.disk_load;
            }
        }

        Self {
            prompt,
            profile,
            memory_path,
            experience_path,
            adaptive_path,
            trace_path,
            trace_schema_gate_path,
            benchmark_path,
            benchmark_all_devices,
            benchmark_gate_enabled,
            benchmark_min_quality,
            benchmark_min_reward,
            benchmark_max_total_ms,
            benchmark_max_recursive_chunks,
            benchmark_min_recursive_cases,
            benchmark_min_recursive_runtime_calls,
            benchmark_min_auto_replay_router_updates,
            benchmark_min_auto_replay_hierarchy_updates,
            benchmark_min_auto_replay_router_threshold_mutations,
            benchmark_min_auto_replay_hierarchy_weight_mutations,
            benchmark_min_auto_replay_router_threshold_delta,
            benchmark_min_auto_replay_hierarchy_weight_delta,
            benchmark_min_auto_replay_memory_updates,
            benchmark_min_live_memory_feedback_updates,
            benchmark_min_auto_replay_live_memory_feedback_updates,
            benchmark_min_auto_replay_recursive_items,
            benchmark_min_auto_replay_recursive_call_pressure,
            benchmark_max_auto_replay_recursive_call_pressure,
            benchmark_min_evolution_live_inference_runs,
            benchmark_min_evolution_live_router_threshold_mutations,
            benchmark_min_evolution_live_hierarchy_weight_mutations,
            benchmark_min_evolution_live_router_threshold_delta,
            benchmark_min_evolution_live_hierarchy_weight_delta,
            benchmark_min_evolution_live_memory_updates,
            benchmark_min_evolution_live_stored_memory_updates,
            benchmark_min_evolution_live_reflection_issues,
            benchmark_min_evolution_live_critical_reflection_issues,
            benchmark_min_evolution_live_revision_actions,
            benchmark_min_evolution_live_inference_device_profiles,
            benchmark_min_evolution_live_router_threshold_mutation_device_profiles,
            benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles,
            benchmark_min_evolution_live_memory_update_device_profiles,
            benchmark_min_evolution_live_stored_memory_update_device_profiles,
            benchmark_min_evolution_live_reflection_issue_device_profiles,
            benchmark_min_evolution_live_critical_reflection_issue_device_profiles,
            benchmark_min_evolution_live_revision_action_device_profiles,
            benchmark_min_evolution_replay_runs,
            benchmark_min_evolution_replay_items,
            benchmark_min_evolution_router_threshold_mutations,
            benchmark_min_evolution_hierarchy_weight_mutations,
            benchmark_min_evolution_router_threshold_delta,
            benchmark_min_evolution_hierarchy_weight_delta,
            benchmark_min_evolution_memory_updates,
            benchmark_min_evolution_replay_live_memory_feedback_updates,
            benchmark_min_evolution_recursive_replay_items,
            benchmark_min_evolution_recursive_runtime_calls,
            benchmark_max_evolution_drift_rollbacks,
            benchmark_max_evolution_rollback_router_threshold_delta,
            benchmark_max_evolution_rollback_hierarchy_weight_delta,
            benchmark_min_sparse_skipped_cases,
            benchmark_min_sparse_skipped_tokens,
            benchmark_min_runtime_forward_cases,
            benchmark_min_runtime_forward_energy_cases,
            benchmark_min_runtime_kv_influence_cases,
            benchmark_min_runtime_layer_mode_cases,
            benchmark_min_runtime_all_layer_mode_cases,
            benchmark_min_runtime_global_layers,
            benchmark_min_runtime_local_window_layers,
            benchmark_min_runtime_convolutional_fusion_layers,
            benchmark_min_runtime_uncertainty_cases,
            benchmark_min_runtime_uncertainty_tokens,
            benchmark_min_runtime_kv_import_cases,
            benchmark_min_runtime_kv_imported,
            benchmark_min_runtime_kv_exported,
            benchmark_min_runtime_kv_stored,
            benchmark_min_runtime_adapter_contract_cases,
            benchmark_min_runtime_adapter_kinds,
            benchmark_min_runtime_adapter_observations,
            benchmark_min_runtime_adapter_best_score,
            benchmark_max_runtime_adapter_contract_violations,
            benchmark_max_memory_governance_failures,
            benchmark_min_memory_governance_cases,
            benchmark_min_memory_governance_device_profiles,
            benchmark_min_memory_retention_activity_cases,
            benchmark_min_memory_compaction_activity_cases,
            benchmark_min_reflection_issue_cases,
            benchmark_min_reflection_issues,
            benchmark_min_critical_reflection_issue_cases,
            benchmark_min_critical_reflection_issues,
            benchmark_min_revision_action_cases,
            benchmark_min_revision_actions,
            benchmark_min_reflection_issue_device_profiles,
            benchmark_min_critical_reflection_issue_device_profiles,
            benchmark_min_revision_action_device_profiles,
            benchmark_min_device_profiles,
            benchmark_min_recursive_device_profiles,
            benchmark_max_drift_blocks,
            benchmark_max_drift_rollbacks,
            benchmark_roundtrip,
            list_devices,
            device_gate,
            kv_quant_gate,
            kv_quant_max_total_us,
            runtime_manifest_gate,
            runtime_manifest_all_devices_gate,
            runtime_weights_path,
            runtime_tokenizer_path,
            runtime_config_path,
            runtime_layer_count,
            runtime_hidden_size,
            runtime_attention_heads,
            runtime_kv_heads,
            runtime_local_window_tokens,
            inspect_state,
            inspect_limit,
            inspect_gate,
            inspect_min_memories,
            inspect_min_runtime_kv_memories,
            inspect_min_experiences,
            inspect_min_runtime_model_experiences,
            inspect_min_runtime_adapter_experiences,
            inspect_min_runtime_forward_energy_experiences,
            inspect_min_runtime_kv_influence_experiences,
            inspect_min_runtime_layer_mode_experiences,
            inspect_min_runtime_all_layer_mode_experiences,
            inspect_min_runtime_global_layers,
            inspect_min_runtime_local_window_layers,
            inspect_min_runtime_convolutional_fusion_layers,
            inspect_min_runtime_kv_import_experiences,
            inspect_min_runtime_kv_export_experiences,
            inspect_min_runtime_kv_memory_device_profiles,
            inspect_min_runtime_model_device_profiles,
            inspect_min_runtime_adapter_device_profiles,
            inspect_min_runtime_forward_energy_device_profiles,
            inspect_min_runtime_kv_influence_device_profiles,
            inspect_min_runtime_layer_mode_device_profiles,
            inspect_min_runtime_all_layer_mode_device_profiles,
            inspect_min_runtime_kv_import_device_profiles,
            inspect_min_runtime_kv_export_device_profiles,
            inspect_min_reflection_issue_experiences,
            inspect_min_critical_reflection_issue_experiences,
            inspect_min_revision_action_experiences,
            inspect_min_live_memory_feedback_experiences,
            inspect_min_live_memory_feedback_updates,
            inspect_min_reflection_issue_device_profiles,
            inspect_min_critical_reflection_issue_device_profiles,
            inspect_min_revision_action_device_profiles,
            inspect_min_live_memory_feedback_device_profiles,
            inspect_min_evolution_live_inference_device_profiles,
            inspect_min_evolution_live_router_threshold_mutation_device_profiles,
            inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles,
            inspect_min_evolution_live_memory_update_device_profiles,
            inspect_min_evolution_live_stored_memory_update_device_profiles,
            inspect_min_evolution_live_reflection_issue_device_profiles,
            inspect_min_evolution_live_critical_reflection_issue_device_profiles,
            inspect_min_evolution_live_revision_action_device_profiles,
            inspect_min_evolution_replay_run_device_profiles,
            inspect_min_evolution_replay_item_device_profiles,
            inspect_min_evolution_router_threshold_mutation_device_profiles,
            inspect_min_evolution_hierarchy_weight_mutation_device_profiles,
            inspect_min_evolution_memory_update_device_profiles,
            inspect_min_evolution_replay_live_memory_feedback_device_profiles,
            inspect_min_evolution_recursive_replay_device_profiles,
            inspect_min_evolution_recursive_runtime_call_device_profiles,
            inspect_min_router_observations,
            inspect_min_evolution_live_inference_runs,
            inspect_min_evolution_live_router_threshold_mutations,
            inspect_min_evolution_live_hierarchy_weight_mutations,
            inspect_min_evolution_live_router_threshold_delta,
            inspect_min_evolution_live_hierarchy_weight_delta,
            inspect_min_evolution_live_memory_updates,
            inspect_min_evolution_live_stored_memory_updates,
            inspect_min_evolution_live_reflection_issues,
            inspect_min_evolution_live_critical_reflection_issues,
            inspect_min_evolution_live_revision_actions,
            inspect_min_evolution_replay_runs,
            inspect_min_evolution_replay_items,
            inspect_min_evolution_router_threshold_mutations,
            inspect_min_evolution_hierarchy_weight_mutations,
            inspect_min_evolution_router_threshold_delta,
            inspect_min_evolution_hierarchy_weight_delta,
            inspect_min_evolution_memory_updates,
            inspect_min_evolution_replay_live_memory_feedback_updates,
            inspect_min_evolution_recursive_replay_items,
            inspect_min_evolution_recursive_runtime_calls,
            inspect_max_evolution_drift_rollbacks,
            inspect_max_evolution_rollback_router_threshold_delta,
            inspect_max_evolution_rollback_hierarchy_weight_delta,
            inspect_require_runtime_kv_dimensions,
            local_runtime,
            production_runtime,
            production_reference_kernel,
            production_local_kernel,
            production_kernel_conformance_gate,
            production_kernel_conformance_all_devices_gate,
            runtime_command,
            runtime_args,
            runtime_prompt_mode,
            runtime_wire_format,
            runtime_metadata,
            native_window_tokens,
            chunk_tokens,
            chunk_overlap_tokens,
            merge_fan_in,
            replay_limit,
            auto_replay_limit,
            retention_stale_after,
            retention_decay_rate,
            retention_remove_below,
            retention_remove_after_failures,
            compaction_similarity_threshold,
            compaction_max_candidates,
            compaction_max_merges,
            device,
            cpu_load,
            gpu_load,
            ram_load,
            disk_load,
        }
    }

    fn benchmark_gate(&self) -> BenchmarkGate {
        let mut gate = BenchmarkGate::default();

        if let Some(value) = self.benchmark_min_quality {
            gate.min_average_quality = value;
        }
        if let Some(value) = self.benchmark_min_reward {
            gate.min_average_reward = value;
        }
        if let Some(value) = self.benchmark_max_total_ms {
            gate.max_total_elapsed_ms = Some(value);
        }
        if let Some(value) = self.benchmark_max_recursive_chunks {
            gate.max_case_recursive_chunks = Some(value);
        }
        if let Some(value) = self.benchmark_min_recursive_cases {
            gate.min_recursive_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_recursive_runtime_calls {
            gate.min_recursive_runtime_calls = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_router_updates {
            gate.min_auto_replay_router_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_hierarchy_updates {
            gate.min_auto_replay_hierarchy_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_router_threshold_mutations {
            gate.min_auto_replay_router_threshold_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_hierarchy_weight_mutations {
            gate.min_auto_replay_hierarchy_weight_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_router_threshold_delta {
            gate.min_auto_replay_router_threshold_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_auto_replay_hierarchy_weight_delta {
            gate.min_auto_replay_hierarchy_weight_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_auto_replay_memory_updates {
            gate.min_auto_replay_memory_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_live_memory_feedback_updates {
            gate.min_live_memory_feedback_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_live_memory_feedback_updates {
            gate.min_auto_replay_live_memory_feedback_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_recursive_items {
            gate.min_auto_replay_recursive_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_auto_replay_recursive_call_pressure {
            gate.min_auto_replay_recursive_call_pressure = Some(value.clamp(0.0, 1.0));
        }
        if let Some(value) = self.benchmark_max_auto_replay_recursive_call_pressure {
            gate.max_auto_replay_recursive_call_pressure = Some(value.clamp(0.0, 1.0));
        }
        if let Some(value) = self.benchmark_min_evolution_live_inference_runs {
            gate.min_evolution_live_inference_runs = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_router_threshold_mutations {
            gate.min_evolution_live_router_threshold_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_hierarchy_weight_mutations {
            gate.min_evolution_live_hierarchy_weight_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_router_threshold_delta {
            gate.min_evolution_live_router_threshold_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_live_hierarchy_weight_delta {
            gate.min_evolution_live_hierarchy_weight_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_live_memory_updates {
            gate.min_evolution_live_memory_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_stored_memory_updates {
            gate.min_evolution_live_stored_memory_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_reflection_issues {
            gate.min_evolution_live_reflection_issues = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_critical_reflection_issues {
            gate.min_evolution_live_critical_reflection_issues = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_revision_actions {
            gate.min_evolution_live_revision_actions = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_inference_device_profiles {
            gate.min_evolution_live_inference_device_profiles = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_live_router_threshold_mutation_device_profiles
        {
            gate.min_evolution_live_router_threshold_mutation_device_profiles = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles
        {
            gate.min_evolution_live_hierarchy_weight_mutation_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_memory_update_device_profiles {
            gate.min_evolution_live_memory_update_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_stored_memory_update_device_profiles
        {
            gate.min_evolution_live_stored_memory_update_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_reflection_issue_device_profiles {
            gate.min_evolution_live_reflection_issue_device_profiles = Some(value);
        }
        if let Some(value) =
            self.benchmark_min_evolution_live_critical_reflection_issue_device_profiles
        {
            gate.min_evolution_live_critical_reflection_issue_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_live_revision_action_device_profiles {
            gate.min_evolution_live_revision_action_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_runs {
            gate.min_evolution_replay_runs = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_items {
            gate.min_evolution_replay_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_router_threshold_mutations {
            gate.min_evolution_router_threshold_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_hierarchy_weight_mutations {
            gate.min_evolution_hierarchy_weight_mutations = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_router_threshold_delta {
            gate.min_evolution_router_threshold_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_hierarchy_weight_delta {
            gate.min_evolution_hierarchy_weight_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_evolution_memory_updates {
            gate.min_evolution_memory_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_replay_live_memory_feedback_updates {
            gate.min_evolution_replay_live_memory_feedback_updates = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_recursive_replay_items {
            gate.min_evolution_recursive_replay_items = Some(value);
        }
        if let Some(value) = self.benchmark_min_evolution_recursive_runtime_calls {
            gate.min_evolution_recursive_runtime_calls = Some(value);
        }
        if let Some(value) = self.benchmark_max_evolution_drift_rollbacks {
            gate.max_evolution_drift_rollbacks = Some(value);
        }
        if let Some(value) = self.benchmark_max_evolution_rollback_router_threshold_delta {
            gate.max_evolution_rollback_router_threshold_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_max_evolution_rollback_hierarchy_weight_delta {
            gate.max_evolution_rollback_hierarchy_weight_delta = Some(value.max(0.0));
        }
        if let Some(value) = self.benchmark_min_sparse_skipped_cases {
            gate.min_sparse_skipped_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_sparse_skipped_tokens {
            gate.min_sparse_skipped_tokens = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_forward_cases {
            gate.min_runtime_forward_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_forward_energy_cases {
            gate.min_runtime_forward_energy_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_influence_cases {
            gate.min_runtime_kv_influence_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_layer_mode_cases {
            gate.min_runtime_layer_mode_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_all_layer_mode_cases {
            gate.min_runtime_all_layer_mode_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_global_layers {
            gate.min_runtime_global_layers = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_local_window_layers {
            gate.min_runtime_local_window_layers = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_convolutional_fusion_layers {
            gate.min_runtime_convolutional_fusion_layers = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_uncertainty_cases {
            gate.min_runtime_uncertainty_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_uncertainty_tokens {
            gate.min_runtime_uncertainty_tokens = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_import_cases {
            gate.min_runtime_kv_import_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_imported {
            gate.min_runtime_kv_imported = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_exported {
            gate.min_runtime_kv_exported = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_kv_stored {
            gate.min_runtime_kv_stored = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_contract_cases {
            gate.min_runtime_adapter_contract_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_kinds {
            gate.min_runtime_adapter_kinds = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_observations {
            gate.min_runtime_adapter_observations = Some(value);
        }
        if let Some(value) = self.benchmark_min_runtime_adapter_best_score {
            gate.min_runtime_adapter_best_score = Some(value.clamp(0.0, 1.0));
        }
        if let Some(value) = self.benchmark_max_runtime_adapter_contract_violations {
            gate.max_runtime_adapter_contract_violations = Some(value);
        }
        if let Some(value) = self.benchmark_max_memory_governance_failures {
            gate.max_memory_governance_failures = Some(value);
        }
        if let Some(value) = self.benchmark_min_memory_governance_cases {
            gate.min_memory_governance_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_memory_governance_device_profiles {
            gate.min_memory_governance_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_memory_retention_activity_cases {
            gate.min_memory_retention_activity_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_memory_compaction_activity_cases {
            gate.min_memory_compaction_activity_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_reflection_issue_cases {
            gate.min_reflection_issue_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_reflection_issues {
            gate.min_reflection_issues = Some(value);
        }
        if let Some(value) = self.benchmark_min_critical_reflection_issue_cases {
            gate.min_critical_reflection_issue_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_critical_reflection_issues {
            gate.min_critical_reflection_issues = Some(value);
        }
        if let Some(value) = self.benchmark_min_revision_action_cases {
            gate.min_revision_action_cases = Some(value);
        }
        if let Some(value) = self.benchmark_min_revision_actions {
            gate.min_revision_actions = Some(value);
        }
        if let Some(value) = self.benchmark_min_reflection_issue_device_profiles {
            gate.min_reflection_issue_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_critical_reflection_issue_device_profiles {
            gate.min_critical_reflection_issue_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_revision_action_device_profiles {
            gate.min_revision_action_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_device_profiles {
            gate.min_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_min_recursive_device_profiles {
            gate.min_recursive_device_profiles = Some(value);
        }
        if let Some(value) = self.benchmark_max_drift_blocks {
            gate.max_drift_blocks = Some(value);
        }
        if let Some(value) = self.benchmark_max_drift_rollbacks {
            gate.max_drift_rollbacks = Some(value);
        }

        gate
    }

    fn state_inspection_gate(&self) -> StateInspectionGate {
        StateInspectionGate {
            min_memories: self.inspect_min_memories,
            min_runtime_kv_memories: self.inspect_min_runtime_kv_memories,
            min_experiences: self.inspect_min_experiences,
            min_runtime_model_experiences: self.inspect_min_runtime_model_experiences,
            min_runtime_adapter_experiences: self.inspect_min_runtime_adapter_experiences,
            min_runtime_forward_energy_experiences: self
                .inspect_min_runtime_forward_energy_experiences,
            min_runtime_kv_influence_experiences: self.inspect_min_runtime_kv_influence_experiences,
            min_runtime_layer_mode_experiences: self.inspect_min_runtime_layer_mode_experiences,
            min_runtime_all_layer_mode_experiences: self
                .inspect_min_runtime_all_layer_mode_experiences,
            min_runtime_global_layers: self.inspect_min_runtime_global_layers,
            min_runtime_local_window_layers: self.inspect_min_runtime_local_window_layers,
            min_runtime_convolutional_fusion_layers: self
                .inspect_min_runtime_convolutional_fusion_layers,
            min_runtime_kv_import_experiences: self.inspect_min_runtime_kv_import_experiences,
            min_runtime_kv_export_experiences: self.inspect_min_runtime_kv_export_experiences,
            min_reflection_issue_experiences: self.inspect_min_reflection_issue_experiences,
            min_critical_reflection_issue_experiences: self
                .inspect_min_critical_reflection_issue_experiences,
            min_revision_action_experiences: self.inspect_min_revision_action_experiences,
            min_live_memory_feedback_experiences: self.inspect_min_live_memory_feedback_experiences,
            min_live_memory_feedback_updates: self.inspect_min_live_memory_feedback_updates,
            min_router_observations: self.inspect_min_router_observations,
            min_evolution_live_inference_runs: self.inspect_min_evolution_live_inference_runs,
            min_evolution_live_router_threshold_mutations: self
                .inspect_min_evolution_live_router_threshold_mutations,
            min_evolution_live_hierarchy_weight_mutations: self
                .inspect_min_evolution_live_hierarchy_weight_mutations,
            min_evolution_live_router_threshold_delta: self
                .inspect_min_evolution_live_router_threshold_delta
                .map(|value| value.max(0.0)),
            min_evolution_live_hierarchy_weight_delta: self
                .inspect_min_evolution_live_hierarchy_weight_delta
                .map(|value| value.max(0.0)),
            min_evolution_live_memory_updates: self.inspect_min_evolution_live_memory_updates,
            min_evolution_live_stored_memory_updates: self
                .inspect_min_evolution_live_stored_memory_updates,
            min_evolution_live_reflection_issues: self.inspect_min_evolution_live_reflection_issues,
            min_evolution_live_critical_reflection_issues: self
                .inspect_min_evolution_live_critical_reflection_issues,
            min_evolution_live_revision_actions: self.inspect_min_evolution_live_revision_actions,
            min_evolution_replay_runs: self.inspect_min_evolution_replay_runs,
            min_evolution_replay_items: self.inspect_min_evolution_replay_items,
            min_evolution_router_threshold_mutations: self
                .inspect_min_evolution_router_threshold_mutations,
            min_evolution_hierarchy_weight_mutations: self
                .inspect_min_evolution_hierarchy_weight_mutations,
            min_evolution_router_threshold_delta: self
                .inspect_min_evolution_router_threshold_delta
                .map(|value| value.max(0.0)),
            min_evolution_hierarchy_weight_delta: self
                .inspect_min_evolution_hierarchy_weight_delta
                .map(|value| value.max(0.0)),
            min_evolution_memory_updates: self.inspect_min_evolution_memory_updates,
            min_evolution_replay_live_memory_feedback_updates: self
                .inspect_min_evolution_replay_live_memory_feedback_updates,
            min_evolution_recursive_replay_items: self.inspect_min_evolution_recursive_replay_items,
            min_evolution_recursive_runtime_calls: self
                .inspect_min_evolution_recursive_runtime_calls,
            max_evolution_drift_rollbacks: self.inspect_max_evolution_drift_rollbacks,
            max_evolution_rollback_router_threshold_delta: self
                .inspect_max_evolution_rollback_router_threshold_delta
                .map(|value| value.max(0.0)),
            max_evolution_rollback_hierarchy_weight_delta: self
                .inspect_max_evolution_rollback_hierarchy_weight_delta
                .map(|value| value.max(0.0)),
            require_runtime_kv_dimensions: self.inspect_require_runtime_kv_dimensions,
        }
    }

    fn state_inspection_matrix_gate(&self) -> StateInspectionMatrixGate {
        StateInspectionMatrixGate {
            min_runtime_kv_memory_device_profiles: self
                .inspect_min_runtime_kv_memory_device_profiles,
            min_runtime_model_device_profiles: self.inspect_min_runtime_model_device_profiles,
            min_runtime_adapter_device_profiles: self.inspect_min_runtime_adapter_device_profiles,
            min_runtime_forward_energy_device_profiles: self
                .inspect_min_runtime_forward_energy_device_profiles,
            min_runtime_kv_influence_device_profiles: self
                .inspect_min_runtime_kv_influence_device_profiles,
            min_runtime_layer_mode_device_profiles: self
                .inspect_min_runtime_layer_mode_device_profiles,
            min_runtime_all_layer_mode_device_profiles: self
                .inspect_min_runtime_all_layer_mode_device_profiles,
            min_runtime_kv_import_device_profiles: self
                .inspect_min_runtime_kv_import_device_profiles,
            min_runtime_kv_export_device_profiles: self
                .inspect_min_runtime_kv_export_device_profiles,
            min_reflection_issue_device_profiles: self.inspect_min_reflection_issue_device_profiles,
            min_critical_reflection_issue_device_profiles: self
                .inspect_min_critical_reflection_issue_device_profiles,
            min_revision_action_device_profiles: self.inspect_min_revision_action_device_profiles,
            min_live_memory_feedback_device_profiles: self
                .inspect_min_live_memory_feedback_device_profiles,
            min_evolution_live_inference_device_profiles: self
                .inspect_min_evolution_live_inference_device_profiles,
            min_evolution_live_router_threshold_mutation_device_profiles: self
                .inspect_min_evolution_live_router_threshold_mutation_device_profiles,
            min_evolution_live_hierarchy_weight_mutation_device_profiles: self
                .inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles,
            min_evolution_live_memory_update_device_profiles: self
                .inspect_min_evolution_live_memory_update_device_profiles,
            min_evolution_live_stored_memory_update_device_profiles: self
                .inspect_min_evolution_live_stored_memory_update_device_profiles,
            min_evolution_live_reflection_issue_device_profiles: self
                .inspect_min_evolution_live_reflection_issue_device_profiles,
            min_evolution_live_critical_reflection_issue_device_profiles: self
                .inspect_min_evolution_live_critical_reflection_issue_device_profiles,
            min_evolution_live_revision_action_device_profiles: self
                .inspect_min_evolution_live_revision_action_device_profiles,
            min_evolution_replay_run_device_profiles: self
                .inspect_min_evolution_replay_run_device_profiles,
            min_evolution_replay_item_device_profiles: self
                .inspect_min_evolution_replay_item_device_profiles,
            min_evolution_router_threshold_mutation_device_profiles: self
                .inspect_min_evolution_router_threshold_mutation_device_profiles,
            min_evolution_hierarchy_weight_mutation_device_profiles: self
                .inspect_min_evolution_hierarchy_weight_mutation_device_profiles,
            min_evolution_memory_update_device_profiles: self
                .inspect_min_evolution_memory_update_device_profiles,
            min_evolution_replay_live_memory_feedback_device_profiles: self
                .inspect_min_evolution_replay_live_memory_feedback_device_profiles,
            min_evolution_recursive_replay_device_profiles: self
                .inspect_min_evolution_recursive_replay_device_profiles,
            min_evolution_recursive_runtime_call_device_profiles: self
                .inspect_min_evolution_recursive_runtime_call_device_profiles,
        }
    }

    fn runtime_manifest(&self) -> RuntimeManifest {
        let mut assets = RuntimeAssetPaths::new();
        if let Some(path) = &self.runtime_weights_path {
            assets = assets.with_weights(path.clone());
        }
        if let Some(path) = &self.runtime_tokenizer_path {
            assets = assets.with_tokenizer(path.clone());
        }
        if let Some(path) = &self.runtime_config_path {
            assets = assets.with_config(path.clone());
        }

        let manifest = RuntimeManifest::from_metadata(self.runtime_metadata.clone());
        let defaults = manifest.architecture;
        let architecture = TransformerRuntimeArchitecture::new(
            self.runtime_layer_count.unwrap_or(defaults.layer_count),
            self.runtime_hidden_size.unwrap_or(defaults.hidden_size),
            self.runtime_attention_heads
                .unwrap_or(defaults.attention_heads),
            self.runtime_kv_heads.unwrap_or(defaults.kv_heads),
            self.runtime_local_window_tokens
                .unwrap_or(defaults.local_window_tokens),
        );

        manifest.with_assets(assets).with_architecture(architecture)
    }

    fn runtime_manifest_device_plan(&self) -> HardwarePlan {
        self.runtime_manifest_device_plan_for(self.device, self.profile, &self.prompt)
    }

    fn runtime_manifest_device_plan_for(
        &self,
        device: DeviceClass,
        profile: TaskProfile,
        prompt: &str,
    ) -> HardwarePlan {
        let snapshot = HardwareSnapshot::new(
            device,
            self.cpu_load,
            self.gpu_load,
            self.ram_load,
            self.disk_load,
        );
        let prompt_tokens = RecursiveScheduler::new(
            self.native_window_tokens,
            self.chunk_tokens,
            self.chunk_overlap_tokens,
            self.merge_fan_in,
        )
        .plan(prompt)
        .prompt_tokens;

        HardwareAllocator::new().plan(
            snapshot,
            profile,
            prompt_tokens,
            HierarchyWeights::default(),
        )
    }

    fn production_runtime(&self) -> std::io::Result<rust_norion::ProductionTransformerRuntime> {
        self.production_runtime_for_case(self.device, self.profile, &self.prompt)
    }

    fn production_runtime_for_case(
        &self,
        device: DeviceClass,
        profile: TaskProfile,
        prompt: &str,
    ) -> std::io::Result<rust_norion::ProductionTransformerRuntime> {
        let runtime = rust_norion::ProductionTransformerRuntime::from_manifest_for_plan(
            self.runtime_manifest(),
            &self.runtime_manifest_device_plan_for(device, profile, prompt),
        )
        .map_err(runtime_error_to_io)?;

        if self.production_local_kernel {
            let local = LocalTransformerRuntime::with_manifest(self.runtime_manifest());
            Ok(runtime.with_kernel(ModelRuntimeForwardKernel::new(local)))
        } else if self.production_reference_kernel {
            Ok(runtime.with_kernel(ReferenceProductionForwardKernel::new()))
        } else {
            Ok(runtime)
        }
    }

    fn kv_quant_gate(&self) -> KvQuantBenchmarkGate {
        let mut gate = KvQuantBenchmarkGate::default();

        if let Some(value) = self.kv_quant_max_total_us {
            gate.max_total_elapsed_us = Some(value);
        }

        gate
    }
}

fn runtime_error_to_io(error: RuntimeError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, error.message().to_owned())
}

fn normalize_runtime_metadata(metadata: RuntimeMetadata) -> RuntimeMetadata {
    let hot_bits = metadata.hot_kv_precision_bits;
    let cold_bits = metadata.cold_kv_precision_bits;
    let max_import_blocks = if metadata.supports_kv_import {
        metadata.max_kv_import_blocks.max(8)
    } else {
        0
    };
    let max_export_blocks = if metadata.supports_kv_export {
        metadata.max_kv_export_blocks.max(4)
    } else {
        0
    };

    metadata
        .with_kv_limits(max_import_blocks, max_export_blocks)
        .with_kv_precision(hot_bits, cold_bits)
}

fn parse_usize(value: &str, fallback: usize) -> usize {
    value.parse::<usize>().unwrap_or(fallback)
}

fn parse_u128(value: &str, fallback: u128) -> u128 {
    value.parse::<u128>().unwrap_or(fallback)
}

fn parse_u64(value: &str, fallback: u64) -> u64 {
    value.parse::<u64>().unwrap_or(fallback)
}

fn parse_f32(value: &str, fallback: f32) -> f32 {
    value.parse::<f32>().unwrap_or(fallback)
}

fn parse_device_or_generic(value: &str) -> DeviceClass {
    value.parse::<DeviceClass>().unwrap_or(DeviceClass::CpuOnly)
}

fn detect_profile(prompt: &str) -> TaskProfile {
    let lower = prompt.to_ascii_lowercase();

    if lower.contains("rust")
        || lower.contains("code")
        || lower.contains("api")
        || lower.contains("struct")
        || lower.contains("trait")
    {
        TaskProfile::Coding
    } else if lower.contains("novel") || lower.contains("story") || lower.contains("writing") {
        TaskProfile::Writing
    } else if lower.contains("document")
        || lower.contains("context")
        || lower.contains("million token")
    {
        TaskProfile::LongDocument
    } else {
        TaskProfile::General
    }
}

fn print_help_and_exit() -> ! {
    let usage = concat!(
        "Usage: rust-norion [options] <prompt>\n",
        "\n",
        "Core: --profile coding|writing|long|general --memory path --experience path --adaptive path\n",
        "Benchmark: --benchmark path --benchmark-gate --benchmark-all-devices --benchmark-roundtrip --benchmark-min-live-memory-feedback-updates n --benchmark-min-auto-replay-live-memory-feedback-updates n --benchmark-min-evolution-replay-live-memory-feedback-updates n\n",
        "Benchmark live evolution: --benchmark-min-evolution-live-inference-runs n --benchmark-min-evolution-live-router-threshold-mutations n --benchmark-min-evolution-live-hierarchy-weight-mutations n --benchmark-min-evolution-live-router-threshold-delta f --benchmark-min-evolution-live-hierarchy-weight-delta f --benchmark-min-evolution-live-memory-updates n --benchmark-min-evolution-live-stored-memory-updates n --benchmark-min-evolution-live-reflection-issues n --benchmark-min-evolution-live-critical-reflection-issues n --benchmark-min-evolution-live-revision-actions n\n",
        "Benchmark all-device live evolution: --benchmark-min-evolution-live-inference-device-profiles n --benchmark-min-evolution-live-router-threshold-mutation-device-profiles n --benchmark-min-evolution-live-hierarchy-weight-mutation-device-profiles n --benchmark-min-evolution-live-memory-update-device-profiles n --benchmark-min-evolution-live-stored-memory-update-device-profiles n --benchmark-min-evolution-live-reflection-issue-device-profiles n --benchmark-min-evolution-live-critical-reflection-issue-device-profiles n --benchmark-min-evolution-live-revision-action-device-profiles n\n",
        "Benchmark memory governance: --benchmark-max-memory-governance-failures n --benchmark-min-memory-governance-cases n --benchmark-min-memory-governance-device-profiles n --benchmark-min-memory-retention-activity-cases n --benchmark-min-memory-compaction-activity-cases n\n",
        "Benchmark reflection evidence: --benchmark-min-reflection-issue-cases n --benchmark-min-reflection-issues n --benchmark-min-critical-reflection-issue-cases n --benchmark-min-critical-reflection-issues n --benchmark-min-revision-action-cases n --benchmark-min-revision-actions n --benchmark-min-reflection-issue-device-profiles n --benchmark-min-critical-reflection-issue-device-profiles n --benchmark-min-revision-action-device-profiles n\n",
        "Runtime: --local-runtime --production-runtime --runtime-command path --runtime-json --runtime-kv-exchange\n",
        "Manifest: --runtime-manifest-gate --runtime-manifest-all-devices-gate --runtime-weights path --runtime-tokenizer-path path --runtime-config path\n",
        "Inspect: --inspect-state --inspect-limit n --inspect-gate --inspect-min-memories n --inspect-min-runtime-kv-memories n --inspect-min-experiences n\n",
        "Inspect runtime evidence: --inspect-min-runtime-model-experiences n --inspect-min-runtime-adapter-experiences n --inspect-min-runtime-forward-energy-experiences n --inspect-min-runtime-kv-influence-experiences n --inspect-min-runtime-layer-mode-experiences n --inspect-min-runtime-all-layer-mode-experiences n --inspect-min-runtime-global-layers n --inspect-min-runtime-local-window-layers n --inspect-min-runtime-convolutional-fusion-layers n --inspect-min-runtime-kv-import-experiences n --inspect-min-runtime-kv-export-experiences n --inspect-min-runtime-layer-mode-device-profiles n --inspect-min-runtime-all-layer-mode-device-profiles n\n",
        "Inspect reflection evidence: --inspect-min-reflection-issue-experiences n --inspect-min-critical-reflection-issue-experiences n --inspect-min-revision-action-experiences n --inspect-min-live-memory-feedback-experiences n --inspect-min-live-memory-feedback-updates n --inspect-min-live-memory-feedback-device-profiles n\n",
        "Inspect evolution: --inspect-min-router-observations n --inspect-min-evolution-router-threshold-delta f --inspect-min-evolution-hierarchy-weight-delta f --inspect-min-evolution-memory-updates n --inspect-min-evolution-replay-live-memory-feedback-updates n --inspect-min-evolution-replay-live-memory-feedback-device-profiles n --inspect-min-evolution-recursive-replay-items n --inspect-max-evolution-rollback-router-threshold-delta f --inspect-max-evolution-rollback-hierarchy-weight-delta f --inspect-require-runtime-kv-dimensions\n",
        "Inspect live evolution: --inspect-min-evolution-live-inference-runs n --inspect-min-evolution-live-router-threshold-mutations n --inspect-min-evolution-live-hierarchy-weight-mutations n --inspect-min-evolution-live-router-threshold-delta f --inspect-min-evolution-live-hierarchy-weight-delta f --inspect-min-evolution-live-memory-updates n --inspect-min-evolution-live-stored-memory-updates n --inspect-min-evolution-live-reflection-issues n --inspect-min-evolution-live-critical-reflection-issues n --inspect-min-evolution-live-revision-actions n\n",
        "Inspect all-device live evolution: --inspect-min-evolution-live-inference-device-profiles n --inspect-min-evolution-live-router-threshold-mutation-device-profiles n --inspect-min-evolution-live-hierarchy-weight-mutation-device-profiles n --inspect-min-evolution-live-memory-update-device-profiles n --inspect-min-evolution-live-stored-memory-update-device-profiles n --inspect-min-evolution-live-reflection-issue-device-profiles n --inspect-min-evolution-live-critical-reflection-issue-device-profiles n --inspect-min-evolution-live-revision-action-device-profiles n\n",
        "Device: --list-devices --device-gate --device auto|cpu|integrated|discrete|uma|mobile|embedded|browser-wasm|microcontroller|npu|multi-gpu|edge|server --cpu-load f --gpu-load f --ram-load f --disk-load f"
    );
    println!("{usage}");
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_recursive_scheduler_flags() {
        let args = Args::parse(vec![
            "--profile".to_owned(),
            "long".to_owned(),
            "--native-window".to_owned(),
            "8".to_owned(),
            "--chunk-tokens".to_owned(),
            "6".to_owned(),
            "--chunk-overlap".to_owned(),
            "2".to_owned(),
            "--merge-fan-in".to_owned(),
            "2".to_owned(),
            "--replay".to_owned(),
            "3".to_owned(),
            "--auto-replay".to_owned(),
            "4".to_owned(),
            "--retention-stale-after".to_owned(),
            "12".to_owned(),
            "--retention-decay-rate".to_owned(),
            "0.25".to_owned(),
            "--retention-remove-below".to_owned(),
            "0.08".to_owned(),
            "--retention-remove-after-failures".to_owned(),
            "7".to_owned(),
            "--compaction-threshold".to_owned(),
            "0.88".to_owned(),
            "--compaction-max-candidates".to_owned(),
            "64".to_owned(),
            "--compaction-max-merges".to_owned(),
            "5".to_owned(),
            "--trace".to_owned(),
            "trace.jsonl".to_owned(),
            "--trace-schema-gate".to_owned(),
            "trace-schema.jsonl".to_owned(),
            "--benchmark".to_owned(),
            "benchmark.jsonl".to_owned(),
            "--benchmark-all-devices".to_owned(),
            "--benchmark-min-quality".to_owned(),
            "0.6".to_owned(),
            "--benchmark-min-reward".to_owned(),
            "0.5".to_owned(),
            "--benchmark-max-total-ms".to_owned(),
            "10000".to_owned(),
            "--benchmark-max-recursive-chunks".to_owned(),
            "8".to_owned(),
            "--benchmark-min-recursive-cases".to_owned(),
            "1".to_owned(),
            "--benchmark-min-recursive-runtime-calls".to_owned(),
            "4".to_owned(),
            "--benchmark-min-auto-replay-router-updates".to_owned(),
            "1".to_owned(),
            "--benchmark-min-auto-replay-hierarchy-updates".to_owned(),
            "1".to_owned(),
            "--benchmark-min-auto-replay-router-threshold-mutations".to_owned(),
            "1".to_owned(),
            "--benchmark-min-auto-replay-hierarchy-weight-mutations".to_owned(),
            "1".to_owned(),
            "--benchmark-min-auto-replay-router-threshold-delta".to_owned(),
            "0.01".to_owned(),
            "--benchmark-min-auto-replay-hierarchy-weight-delta".to_owned(),
            "0.01".to_owned(),
            "--benchmark-min-auto-replay-memory-updates".to_owned(),
            "1".to_owned(),
            "--benchmark-min-live-memory-feedback-updates".to_owned(),
            "2".to_owned(),
            "--benchmark-min-auto-replay-live-memory-feedback-updates".to_owned(),
            "3".to_owned(),
            "--benchmark-min-auto-replay-recursive-items".to_owned(),
            "1".to_owned(),
            "--benchmark-min-auto-replay-recursive-call-pressure".to_owned(),
            "0.05".to_owned(),
            "--benchmark-max-auto-replay-recursive-call-pressure".to_owned(),
            "0.25".to_owned(),
            "--benchmark-min-evolution-live-inference-runs".to_owned(),
            "8".to_owned(),
            "--benchmark-min-evolution-live-router-threshold-mutations".to_owned(),
            "2".to_owned(),
            "--benchmark-min-evolution-live-hierarchy-weight-mutations".to_owned(),
            "1".to_owned(),
            "--benchmark-min-evolution-live-router-threshold-delta".to_owned(),
            "0.04".to_owned(),
            "--benchmark-min-evolution-live-hierarchy-weight-delta".to_owned(),
            "0.03".to_owned(),
            "--benchmark-min-evolution-live-memory-updates".to_owned(),
            "5".to_owned(),
            "--benchmark-min-evolution-live-stored-memory-updates".to_owned(),
            "4".to_owned(),
            "--benchmark-min-evolution-live-reflection-issues".to_owned(),
            "3".to_owned(),
            "--benchmark-min-evolution-live-critical-reflection-issues".to_owned(),
            "1".to_owned(),
            "--benchmark-min-evolution-live-revision-actions".to_owned(),
            "2".to_owned(),
            "--benchmark-min-evolution-live-inference-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-evolution-live-router-threshold-mutation-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-evolution-live-hierarchy-weight-mutation-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-evolution-live-memory-update-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-evolution-live-stored-memory-update-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-evolution-live-reflection-issue-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-evolution-live-critical-reflection-issue-device-profiles".to_owned(),
            "6".to_owned(),
            "--benchmark-min-evolution-live-revision-action-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-evolution-replay-runs".to_owned(),
            "1".to_owned(),
            "--benchmark-min-evolution-replay-items".to_owned(),
            "2".to_owned(),
            "--benchmark-min-evolution-router-threshold-mutations".to_owned(),
            "3".to_owned(),
            "--benchmark-min-evolution-hierarchy-weight-mutations".to_owned(),
            "4".to_owned(),
            "--benchmark-min-evolution-router-threshold-delta".to_owned(),
            "0.02".to_owned(),
            "--benchmark-min-evolution-hierarchy-weight-delta".to_owned(),
            "0.03".to_owned(),
            "--benchmark-min-evolution-memory-updates".to_owned(),
            "5".to_owned(),
            "--benchmark-min-evolution-replay-live-memory-feedback-updates".to_owned(),
            "6".to_owned(),
            "--benchmark-min-evolution-recursive-replay-items".to_owned(),
            "6".to_owned(),
            "--benchmark-min-evolution-recursive-runtime-calls".to_owned(),
            "7".to_owned(),
            "--benchmark-max-evolution-drift-rollbacks".to_owned(),
            "0".to_owned(),
            "--benchmark-max-evolution-rollback-router-threshold-delta".to_owned(),
            "0.0".to_owned(),
            "--benchmark-max-evolution-rollback-hierarchy-weight-delta".to_owned(),
            "0.0".to_owned(),
            "--benchmark-min-sparse-skipped-cases".to_owned(),
            "1".to_owned(),
            "--benchmark-min-sparse-skipped-tokens".to_owned(),
            "3".to_owned(),
            "--benchmark-min-runtime-forward-cases".to_owned(),
            "4".to_owned(),
            "--benchmark-min-runtime-forward-energy-cases".to_owned(),
            "4".to_owned(),
            "--benchmark-min-runtime-kv-influence-cases".to_owned(),
            "4".to_owned(),
            "--benchmark-min-runtime-uncertainty-cases".to_owned(),
            "4".to_owned(),
            "--benchmark-min-runtime-uncertainty-tokens".to_owned(),
            "4".to_owned(),
            "--benchmark-min-runtime-kv-import-cases".to_owned(),
            "4".to_owned(),
            "--benchmark-min-runtime-kv-imported".to_owned(),
            "4".to_owned(),
            "--benchmark-min-runtime-kv-exported".to_owned(),
            "4".to_owned(),
            "--benchmark-min-runtime-kv-stored".to_owned(),
            "2".to_owned(),
            "--benchmark-min-runtime-adapter-contract-cases".to_owned(),
            "4".to_owned(),
            "--benchmark-min-runtime-adapter-kinds".to_owned(),
            "3".to_owned(),
            "--benchmark-min-runtime-adapter-observations".to_owned(),
            "2".to_owned(),
            "--benchmark-min-runtime-adapter-best-score".to_owned(),
            "0.25".to_owned(),
            "--benchmark-max-runtime-adapter-contract-violations".to_owned(),
            "0".to_owned(),
            "--benchmark-max-memory-governance-failures".to_owned(),
            "0".to_owned(),
            "--benchmark-min-memory-governance-cases".to_owned(),
            "4".to_owned(),
            "--benchmark-min-memory-governance-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-memory-retention-activity-cases".to_owned(),
            "1".to_owned(),
            "--benchmark-min-memory-compaction-activity-cases".to_owned(),
            "1".to_owned(),
            "--benchmark-min-reflection-issue-cases".to_owned(),
            "2".to_owned(),
            "--benchmark-min-reflection-issues".to_owned(),
            "3".to_owned(),
            "--benchmark-min-critical-reflection-issue-cases".to_owned(),
            "1".to_owned(),
            "--benchmark-min-critical-reflection-issues".to_owned(),
            "1".to_owned(),
            "--benchmark-min-revision-action-cases".to_owned(),
            "2".to_owned(),
            "--benchmark-min-revision-actions".to_owned(),
            "4".to_owned(),
            "--benchmark-min-reflection-issue-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-critical-reflection-issue-device-profiles".to_owned(),
            "6".to_owned(),
            "--benchmark-min-revision-action-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-min-recursive-device-profiles".to_owned(),
            "12".to_owned(),
            "--benchmark-max-drift-blocks".to_owned(),
            "0".to_owned(),
            "--benchmark-max-drift-rollbacks".to_owned(),
            "0".to_owned(),
            "--benchmark-roundtrip".to_owned(),
            "--list-devices".to_owned(),
            "--device-gate".to_owned(),
            "--kv-quant-max-total-us".to_owned(),
            "100000".to_owned(),
            "--runtime-manifest-gate".to_owned(),
            "--runtime-manifest-all-devices-gate".to_owned(),
            "--runtime-weights".to_owned(),
            "weights.noiron".to_owned(),
            "--runtime-tokenizer-path".to_owned(),
            "tokenizer.noiron".to_owned(),
            "--runtime-config".to_owned(),
            "config.noiron".to_owned(),
            "--runtime-layers".to_owned(),
            "18".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "128".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "8".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "4".to_owned(),
            "--runtime-local-window".to_owned(),
            "2048".to_owned(),
            "--inspect-state".to_owned(),
            "--inspect-limit".to_owned(),
            "2".to_owned(),
            "--inspect-gate".to_owned(),
            "--inspect-min-memories".to_owned(),
            "3".to_owned(),
            "--inspect-min-runtime-kv-memories".to_owned(),
            "1".to_owned(),
            "--inspect-min-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-runtime-model-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-runtime-adapter-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-runtime-forward-energy-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-runtime-kv-influence-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-runtime-layer-mode-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-runtime-all-layer-mode-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-runtime-global-layers".to_owned(),
            "4".to_owned(),
            "--inspect-min-runtime-local-window-layers".to_owned(),
            "6".to_owned(),
            "--inspect-min-runtime-convolutional-fusion-layers".to_owned(),
            "2".to_owned(),
            "--inspect-min-runtime-kv-import-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-runtime-kv-export-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-runtime-kv-memory-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-runtime-model-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-runtime-adapter-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-runtime-forward-energy-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-runtime-kv-influence-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-runtime-layer-mode-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-runtime-all-layer-mode-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-runtime-kv-import-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-runtime-kv-export-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-reflection-issue-experiences".to_owned(),
            "3".to_owned(),
            "--inspect-min-critical-reflection-issue-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-revision-action-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-live-memory-feedback-experiences".to_owned(),
            "2".to_owned(),
            "--inspect-min-live-memory-feedback-updates".to_owned(),
            "5".to_owned(),
            "--inspect-min-reflection-issue-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-critical-reflection-issue-device-profiles".to_owned(),
            "6".to_owned(),
            "--inspect-min-revision-action-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-live-memory-feedback-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-live-inference-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-live-router-threshold-mutation-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-live-hierarchy-weight-mutation-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-live-memory-update-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-live-stored-memory-update-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-live-reflection-issue-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-live-critical-reflection-issue-device-profiles".to_owned(),
            "6".to_owned(),
            "--inspect-min-evolution-live-revision-action-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-replay-run-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-replay-item-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-router-threshold-mutation-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-hierarchy-weight-mutation-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-memory-update-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-replay-live-memory-feedback-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-recursive-replay-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-evolution-recursive-runtime-call-device-profiles".to_owned(),
            "12".to_owned(),
            "--inspect-min-router-observations".to_owned(),
            "4".to_owned(),
            "--inspect-min-evolution-live-inference-runs".to_owned(),
            "4".to_owned(),
            "--inspect-min-evolution-live-router-threshold-mutations".to_owned(),
            "5".to_owned(),
            "--inspect-min-evolution-live-hierarchy-weight-mutations".to_owned(),
            "6".to_owned(),
            "--inspect-min-evolution-live-router-threshold-delta".to_owned(),
            "0.06".to_owned(),
            "--inspect-min-evolution-live-hierarchy-weight-delta".to_owned(),
            "0.07".to_owned(),
            "--inspect-min-evolution-live-memory-updates".to_owned(),
            "8".to_owned(),
            "--inspect-min-evolution-live-stored-memory-updates".to_owned(),
            "7".to_owned(),
            "--inspect-min-evolution-live-reflection-issues".to_owned(),
            "6".to_owned(),
            "--inspect-min-evolution-live-critical-reflection-issues".to_owned(),
            "2".to_owned(),
            "--inspect-min-evolution-live-revision-actions".to_owned(),
            "5".to_owned(),
            "--inspect-min-evolution-replay-runs".to_owned(),
            "5".to_owned(),
            "--inspect-min-evolution-replay-items".to_owned(),
            "6".to_owned(),
            "--inspect-min-evolution-router-threshold-mutations".to_owned(),
            "7".to_owned(),
            "--inspect-min-evolution-hierarchy-weight-mutations".to_owned(),
            "8".to_owned(),
            "--inspect-min-evolution-router-threshold-delta".to_owned(),
            "0.04".to_owned(),
            "--inspect-min-evolution-hierarchy-weight-delta".to_owned(),
            "0.05".to_owned(),
            "--inspect-min-evolution-memory-updates".to_owned(),
            "9".to_owned(),
            "--inspect-min-evolution-replay-live-memory-feedback-updates".to_owned(),
            "10".to_owned(),
            "--inspect-min-evolution-recursive-replay-items".to_owned(),
            "10".to_owned(),
            "--inspect-min-evolution-recursive-runtime-calls".to_owned(),
            "11".to_owned(),
            "--inspect-max-evolution-drift-rollbacks".to_owned(),
            "0".to_owned(),
            "--inspect-max-evolution-rollback-router-threshold-delta".to_owned(),
            "0.0".to_owned(),
            "--inspect-max-evolution-rollback-hierarchy-weight-delta".to_owned(),
            "0.0".to_owned(),
            "--inspect-require-runtime-kv-dimensions".to_owned(),
            "--local-runtime".to_owned(),
            "--production-runtime".to_owned(),
            "--production-reference-kernel".to_owned(),
            "--production-local-kernel".to_owned(),
            "--production-kernel-conformance-gate".to_owned(),
            "--production-kernel-conformance-all-devices-gate".to_owned(),
            "--runtime-model-id".to_owned(),
            "dev-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "dev-bpe".to_owned(),
            "--runtime-wire-format".to_owned(),
            "json".to_owned(),
            "--runtime-native-window".to_owned(),
            "4096".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "128".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--device".to_owned(),
            "cpu".to_owned(),
            "--cpu-load".to_owned(),
            "75".to_owned(),
            "--ram-load".to_owned(),
            "0.5".to_owned(),
            "one two three four five six seven eight nine".to_owned(),
        ]);

        assert_eq!(args.profile, TaskProfile::LongDocument);
        assert_eq!(args.native_window_tokens, 8);
        assert_eq!(args.chunk_tokens, 6);
        assert_eq!(args.chunk_overlap_tokens, 2);
        assert_eq!(args.merge_fan_in, 2);
        assert_eq!(args.replay_limit, 3);
        assert_eq!(args.auto_replay_limit, 4);
        assert_eq!(args.retention_stale_after, Some(12));
        assert_eq!(args.retention_decay_rate, Some(0.25));
        assert_eq!(args.retention_remove_below, Some(0.08));
        assert_eq!(args.retention_remove_after_failures, Some(7));
        assert_eq!(args.compaction_similarity_threshold, Some(0.88));
        assert_eq!(args.compaction_max_candidates, Some(64));
        assert_eq!(args.compaction_max_merges, Some(5));
        assert_eq!(
            args.trace_path.as_ref().unwrap(),
            &PathBuf::from("trace.jsonl")
        );
        assert_eq!(
            args.trace_schema_gate_path.as_ref().unwrap(),
            &PathBuf::from("trace-schema.jsonl")
        );
        assert_eq!(
            args.benchmark_path.as_ref().unwrap(),
            &PathBuf::from("benchmark.jsonl")
        );
        assert!(args.benchmark_all_devices);
        assert!(args.benchmark_gate_enabled);
        assert_eq!(args.benchmark_min_quality, Some(0.6));
        assert_eq!(args.benchmark_min_reward, Some(0.5));
        assert_eq!(args.benchmark_max_total_ms, Some(10000));
        assert_eq!(args.benchmark_max_recursive_chunks, Some(8));
        assert_eq!(args.benchmark_min_recursive_cases, Some(1));
        assert_eq!(args.benchmark_min_recursive_runtime_calls, Some(4));
        assert_eq!(args.benchmark_min_auto_replay_router_updates, Some(1));
        assert_eq!(args.benchmark_min_auto_replay_hierarchy_updates, Some(1));
        assert_eq!(
            args.benchmark_min_auto_replay_router_threshold_mutations,
            Some(1)
        );
        assert_eq!(
            args.benchmark_min_auto_replay_hierarchy_weight_mutations,
            Some(1)
        );
        assert_eq!(
            args.benchmark_min_auto_replay_router_threshold_delta,
            Some(0.01)
        );
        assert_eq!(
            args.benchmark_min_auto_replay_hierarchy_weight_delta,
            Some(0.01)
        );
        assert_eq!(args.benchmark_min_auto_replay_memory_updates, Some(1));
        assert_eq!(args.benchmark_min_live_memory_feedback_updates, Some(2));
        assert_eq!(
            args.benchmark_min_auto_replay_live_memory_feedback_updates,
            Some(3)
        );
        assert_eq!(
            args.benchmark_gate().min_auto_replay_router_updates,
            Some(1)
        );
        assert_eq!(
            args.benchmark_gate().min_auto_replay_hierarchy_updates,
            Some(1)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_auto_replay_router_threshold_mutations,
            Some(1)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_auto_replay_hierarchy_weight_mutations,
            Some(1)
        );
        assert_eq!(
            args.benchmark_gate().min_auto_replay_router_threshold_delta,
            Some(0.01)
        );
        assert_eq!(
            args.benchmark_gate().min_auto_replay_hierarchy_weight_delta,
            Some(0.01)
        );
        assert_eq!(
            args.benchmark_gate().min_auto_replay_memory_updates,
            Some(1)
        );
        assert_eq!(
            args.benchmark_gate().min_live_memory_feedback_updates,
            Some(2)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_auto_replay_live_memory_feedback_updates,
            Some(3)
        );
        assert_eq!(args.benchmark_min_auto_replay_recursive_items, Some(1));
        assert_eq!(
            args.benchmark_min_auto_replay_recursive_call_pressure,
            Some(0.05)
        );
        assert_eq!(
            args.benchmark_max_auto_replay_recursive_call_pressure,
            Some(0.25)
        );
        assert_eq!(
            args.benchmark_gate().min_auto_replay_recursive_items,
            Some(1)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_auto_replay_recursive_call_pressure,
            Some(0.05)
        );
        assert_eq!(
            args.benchmark_gate()
                .max_auto_replay_recursive_call_pressure,
            Some(0.25)
        );
        assert_eq!(args.benchmark_min_evolution_live_inference_runs, Some(8));
        assert_eq!(
            args.benchmark_min_evolution_live_router_threshold_mutations,
            Some(2)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_hierarchy_weight_mutations,
            Some(1)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_router_threshold_delta,
            Some(0.04)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_hierarchy_weight_delta,
            Some(0.03)
        );
        assert_eq!(args.benchmark_min_evolution_live_memory_updates, Some(5));
        assert_eq!(
            args.benchmark_min_evolution_live_stored_memory_updates,
            Some(4)
        );
        assert_eq!(args.benchmark_min_evolution_live_reflection_issues, Some(3));
        assert_eq!(
            args.benchmark_min_evolution_live_critical_reflection_issues,
            Some(1)
        );
        assert_eq!(args.benchmark_min_evolution_live_revision_actions, Some(2));
        assert_eq!(
            args.benchmark_gate().min_evolution_live_inference_runs,
            Some(8)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_router_threshold_mutations,
            Some(2)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_hierarchy_weight_mutations,
            Some(1)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_router_threshold_delta,
            Some(0.04)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_hierarchy_weight_delta,
            Some(0.03)
        );
        assert_eq!(
            args.benchmark_gate().min_evolution_live_memory_updates,
            Some(5)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_stored_memory_updates,
            Some(4)
        );
        assert_eq!(
            args.benchmark_gate().min_evolution_live_reflection_issues,
            Some(3)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_critical_reflection_issues,
            Some(1)
        );
        assert_eq!(
            args.benchmark_gate().min_evolution_live_revision_actions,
            Some(2)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_inference_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_router_threshold_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_hierarchy_weight_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_memory_update_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_stored_memory_update_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_reflection_issue_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_critical_reflection_issue_device_profiles,
            Some(6)
        );
        assert_eq!(
            args.benchmark_min_evolution_live_revision_action_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_inference_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_router_threshold_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_hierarchy_weight_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_memory_update_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_stored_memory_update_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_reflection_issue_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_critical_reflection_issue_device_profiles,
            Some(6)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_live_revision_action_device_profiles,
            Some(12)
        );
        assert_eq!(args.benchmark_min_evolution_replay_runs, Some(1));
        assert_eq!(args.benchmark_min_evolution_replay_items, Some(2));
        assert_eq!(
            args.benchmark_min_evolution_router_threshold_mutations,
            Some(3)
        );
        assert_eq!(
            args.benchmark_min_evolution_hierarchy_weight_mutations,
            Some(4)
        );
        assert_eq!(
            args.benchmark_min_evolution_router_threshold_delta,
            Some(0.02)
        );
        assert_eq!(
            args.benchmark_min_evolution_hierarchy_weight_delta,
            Some(0.03)
        );
        assert_eq!(args.benchmark_min_evolution_memory_updates, Some(5));
        assert_eq!(
            args.benchmark_min_evolution_replay_live_memory_feedback_updates,
            Some(6)
        );
        assert_eq!(args.benchmark_min_evolution_recursive_replay_items, Some(6));
        assert_eq!(
            args.benchmark_min_evolution_recursive_runtime_calls,
            Some(7)
        );
        assert_eq!(args.benchmark_max_evolution_drift_rollbacks, Some(0));
        assert_eq!(
            args.benchmark_max_evolution_rollback_router_threshold_delta,
            Some(0.0)
        );
        assert_eq!(
            args.benchmark_max_evolution_rollback_hierarchy_weight_delta,
            Some(0.0)
        );
        assert_eq!(args.benchmark_gate().min_evolution_replay_runs, Some(1));
        assert_eq!(args.benchmark_gate().min_evolution_replay_items, Some(2));
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_router_threshold_mutations,
            Some(3)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_hierarchy_weight_mutations,
            Some(4)
        );
        assert_eq!(
            args.benchmark_gate().min_evolution_router_threshold_delta,
            Some(0.02)
        );
        assert_eq!(
            args.benchmark_gate().min_evolution_hierarchy_weight_delta,
            Some(0.03)
        );
        assert_eq!(args.benchmark_gate().min_evolution_memory_updates, Some(5));
        assert_eq!(
            args.benchmark_gate()
                .min_evolution_replay_live_memory_feedback_updates,
            Some(6)
        );
        assert_eq!(
            args.benchmark_gate().min_evolution_recursive_replay_items,
            Some(6)
        );
        assert_eq!(
            args.benchmark_gate().min_evolution_recursive_runtime_calls,
            Some(7)
        );
        assert_eq!(args.benchmark_gate().max_evolution_drift_rollbacks, Some(0));
        assert_eq!(
            args.benchmark_gate()
                .max_evolution_rollback_router_threshold_delta,
            Some(0.0)
        );
        assert_eq!(
            args.benchmark_gate()
                .max_evolution_rollback_hierarchy_weight_delta,
            Some(0.0)
        );
        assert_eq!(args.benchmark_min_sparse_skipped_cases, Some(1));
        assert_eq!(args.benchmark_min_sparse_skipped_tokens, Some(3));
        assert_eq!(args.benchmark_gate().min_sparse_skipped_cases, Some(1));
        assert_eq!(args.benchmark_gate().min_sparse_skipped_tokens, Some(3));
        assert_eq!(args.benchmark_min_runtime_forward_cases, Some(4));
        assert_eq!(args.benchmark_min_runtime_forward_energy_cases, Some(4));
        assert_eq!(args.benchmark_min_runtime_kv_influence_cases, Some(4));
        assert_eq!(args.benchmark_min_runtime_uncertainty_cases, Some(4));
        assert_eq!(args.benchmark_min_runtime_uncertainty_tokens, Some(4));
        assert_eq!(args.benchmark_min_runtime_kv_import_cases, Some(4));
        assert_eq!(args.benchmark_min_runtime_kv_imported, Some(4));
        assert_eq!(args.benchmark_min_runtime_kv_exported, Some(4));
        assert_eq!(args.benchmark_min_runtime_kv_stored, Some(2));
        assert_eq!(args.benchmark_min_runtime_adapter_contract_cases, Some(4));
        assert_eq!(args.benchmark_min_runtime_adapter_kinds, Some(3));
        assert_eq!(args.benchmark_min_runtime_adapter_observations, Some(2));
        assert_eq!(args.benchmark_min_runtime_adapter_best_score, Some(0.25));
        assert_eq!(
            args.benchmark_max_runtime_adapter_contract_violations,
            Some(0)
        );
        assert_eq!(args.benchmark_max_memory_governance_failures, Some(0));
        assert_eq!(args.benchmark_min_memory_governance_cases, Some(4));
        assert_eq!(
            args.benchmark_min_memory_governance_device_profiles,
            Some(12)
        );
        assert_eq!(args.benchmark_min_memory_retention_activity_cases, Some(1));
        assert_eq!(args.benchmark_min_memory_compaction_activity_cases, Some(1));
        assert_eq!(args.benchmark_min_reflection_issue_cases, Some(2));
        assert_eq!(args.benchmark_min_reflection_issues, Some(3));
        assert_eq!(args.benchmark_min_critical_reflection_issue_cases, Some(1));
        assert_eq!(args.benchmark_min_critical_reflection_issues, Some(1));
        assert_eq!(args.benchmark_min_revision_action_cases, Some(2));
        assert_eq!(args.benchmark_min_revision_actions, Some(4));
        assert_eq!(
            args.benchmark_min_reflection_issue_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_min_critical_reflection_issue_device_profiles,
            Some(6)
        );
        assert_eq!(args.benchmark_min_revision_action_device_profiles, Some(12));
        assert_eq!(args.benchmark_min_device_profiles, Some(12));
        assert_eq!(args.benchmark_min_recursive_device_profiles, Some(12));
        assert_eq!(args.benchmark_gate().min_runtime_forward_cases, Some(4));
        assert_eq!(
            args.benchmark_gate().min_runtime_forward_energy_cases,
            Some(4)
        );
        assert_eq!(
            args.benchmark_gate().min_runtime_kv_influence_cases,
            Some(4)
        );
        assert_eq!(args.benchmark_gate().min_runtime_uncertainty_cases, Some(4));
        assert_eq!(
            args.benchmark_gate().min_runtime_uncertainty_tokens,
            Some(4)
        );
        assert_eq!(args.benchmark_gate().min_runtime_kv_import_cases, Some(4));
        assert_eq!(args.benchmark_gate().min_runtime_kv_imported, Some(4));
        assert_eq!(args.benchmark_gate().min_runtime_kv_exported, Some(4));
        assert_eq!(args.benchmark_gate().min_runtime_kv_stored, Some(2));
        assert_eq!(
            args.benchmark_gate().min_runtime_adapter_contract_cases,
            Some(4)
        );
        assert_eq!(args.benchmark_gate().min_runtime_adapter_kinds, Some(3));
        assert_eq!(
            args.benchmark_gate().min_runtime_adapter_observations,
            Some(2)
        );
        assert_eq!(
            args.benchmark_gate().min_runtime_adapter_best_score,
            Some(0.25)
        );
        assert_eq!(
            args.benchmark_gate()
                .max_runtime_adapter_contract_violations,
            Some(0)
        );
        assert_eq!(
            args.benchmark_gate().max_memory_governance_failures,
            Some(0)
        );
        assert_eq!(args.benchmark_gate().min_memory_governance_cases, Some(4));
        assert_eq!(
            args.benchmark_gate().min_memory_governance_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_gate().min_memory_retention_activity_cases,
            Some(1)
        );
        assert_eq!(
            args.benchmark_gate().min_memory_compaction_activity_cases,
            Some(1)
        );
        assert_eq!(args.benchmark_gate().min_reflection_issue_cases, Some(2));
        assert_eq!(args.benchmark_gate().min_reflection_issues, Some(3));
        assert_eq!(
            args.benchmark_gate().min_critical_reflection_issue_cases,
            Some(1)
        );
        assert_eq!(
            args.benchmark_gate().min_critical_reflection_issues,
            Some(1)
        );
        assert_eq!(args.benchmark_gate().min_revision_action_cases, Some(2));
        assert_eq!(args.benchmark_gate().min_revision_actions, Some(4));
        assert_eq!(
            args.benchmark_gate().min_reflection_issue_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.benchmark_gate()
                .min_critical_reflection_issue_device_profiles,
            Some(6)
        );
        assert_eq!(
            args.benchmark_gate().min_revision_action_device_profiles,
            Some(12)
        );
        assert_eq!(args.benchmark_gate().min_device_profiles, Some(12));
        assert_eq!(
            args.benchmark_gate().min_recursive_device_profiles,
            Some(12)
        );
        assert_eq!(args.benchmark_max_drift_blocks, Some(0));
        assert_eq!(args.benchmark_max_drift_rollbacks, Some(0));
        assert!(args.benchmark_roundtrip);
        assert!(args.list_devices);
        assert!(args.device_gate);
        assert!(args.kv_quant_gate);
        assert_eq!(args.kv_quant_max_total_us, Some(100000));
        assert!(args.runtime_manifest_gate);
        assert!(args.runtime_manifest_all_devices_gate);
        assert_eq!(
            args.runtime_weights_path.as_ref().unwrap(),
            &PathBuf::from("weights.noiron")
        );
        assert_eq!(
            args.runtime_tokenizer_path.as_ref().unwrap(),
            &PathBuf::from("tokenizer.noiron")
        );
        assert_eq!(
            args.runtime_config_path.as_ref().unwrap(),
            &PathBuf::from("config.noiron")
        );
        assert_eq!(args.runtime_layer_count, Some(18));
        assert_eq!(args.runtime_hidden_size, Some(128));
        assert_eq!(args.runtime_attention_heads, Some(8));
        assert_eq!(args.runtime_kv_heads, Some(4));
        assert_eq!(args.runtime_local_window_tokens, Some(2048));
        assert!(args.inspect_state);
        assert_eq!(args.inspect_limit, 2);
        assert!(args.inspect_gate);
        assert_eq!(args.inspect_min_memories, Some(3));
        assert_eq!(args.inspect_min_runtime_kv_memories, Some(1));
        assert_eq!(args.inspect_min_experiences, Some(2));
        assert_eq!(args.inspect_min_runtime_model_experiences, Some(2));
        assert_eq!(args.inspect_min_runtime_adapter_experiences, Some(2));
        assert_eq!(args.inspect_min_runtime_forward_energy_experiences, Some(2));
        assert_eq!(args.inspect_min_runtime_kv_influence_experiences, Some(2));
        assert_eq!(args.inspect_min_runtime_layer_mode_experiences, Some(2));
        assert_eq!(args.inspect_min_runtime_all_layer_mode_experiences, Some(2));
        assert_eq!(args.inspect_min_runtime_global_layers, Some(4));
        assert_eq!(args.inspect_min_runtime_local_window_layers, Some(6));
        assert_eq!(
            args.inspect_min_runtime_convolutional_fusion_layers,
            Some(2)
        );
        assert_eq!(args.inspect_min_runtime_kv_import_experiences, Some(2));
        assert_eq!(args.inspect_min_runtime_kv_export_experiences, Some(2));
        assert_eq!(args.inspect_min_runtime_kv_memory_device_profiles, Some(12));
        assert_eq!(args.inspect_min_runtime_model_device_profiles, Some(12));
        assert_eq!(args.inspect_min_runtime_adapter_device_profiles, Some(12));
        assert_eq!(
            args.inspect_min_runtime_forward_energy_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_runtime_kv_influence_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_runtime_layer_mode_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_runtime_all_layer_mode_device_profiles,
            Some(12)
        );
        assert_eq!(args.inspect_min_runtime_kv_import_device_profiles, Some(12));
        assert_eq!(args.inspect_min_runtime_kv_export_device_profiles, Some(12));
        assert_eq!(args.inspect_min_reflection_issue_experiences, Some(3));
        assert_eq!(
            args.inspect_min_critical_reflection_issue_experiences,
            Some(1)
        );
        assert_eq!(args.inspect_min_revision_action_experiences, Some(2));
        assert_eq!(args.inspect_min_live_memory_feedback_experiences, Some(2));
        assert_eq!(args.inspect_min_live_memory_feedback_updates, Some(5));
        assert_eq!(args.inspect_min_reflection_issue_device_profiles, Some(12));
        assert_eq!(
            args.inspect_min_critical_reflection_issue_device_profiles,
            Some(6)
        );
        assert_eq!(args.inspect_min_revision_action_device_profiles, Some(12));
        assert_eq!(
            args.inspect_min_live_memory_feedback_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_live_inference_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_live_router_threshold_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_live_hierarchy_weight_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_live_memory_update_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_live_stored_memory_update_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_live_reflection_issue_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_live_critical_reflection_issue_device_profiles,
            Some(6)
        );
        assert_eq!(
            args.inspect_min_evolution_live_revision_action_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_replay_run_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_replay_item_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_router_threshold_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_hierarchy_weight_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_memory_update_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_replay_live_memory_feedback_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_recursive_replay_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.inspect_min_evolution_recursive_runtime_call_device_profiles,
            Some(12)
        );
        assert_eq!(args.inspect_min_router_observations, Some(4));
        assert_eq!(args.inspect_min_evolution_live_inference_runs, Some(4));
        assert_eq!(
            args.inspect_min_evolution_live_router_threshold_mutations,
            Some(5)
        );
        assert_eq!(
            args.inspect_min_evolution_live_hierarchy_weight_mutations,
            Some(6)
        );
        assert_eq!(
            args.inspect_min_evolution_live_router_threshold_delta,
            Some(0.06)
        );
        assert_eq!(
            args.inspect_min_evolution_live_hierarchy_weight_delta,
            Some(0.07)
        );
        assert_eq!(args.inspect_min_evolution_live_memory_updates, Some(8));
        assert_eq!(
            args.inspect_min_evolution_live_stored_memory_updates,
            Some(7)
        );
        assert_eq!(args.inspect_min_evolution_live_reflection_issues, Some(6));
        assert_eq!(
            args.inspect_min_evolution_live_critical_reflection_issues,
            Some(2)
        );
        assert_eq!(args.inspect_min_evolution_live_revision_actions, Some(5));
        assert_eq!(args.inspect_min_evolution_replay_runs, Some(5));
        assert_eq!(args.inspect_min_evolution_replay_items, Some(6));
        assert_eq!(
            args.inspect_min_evolution_router_threshold_mutations,
            Some(7)
        );
        assert_eq!(
            args.inspect_min_evolution_hierarchy_weight_mutations,
            Some(8)
        );
        assert_eq!(
            args.inspect_min_evolution_router_threshold_delta,
            Some(0.04)
        );
        assert_eq!(
            args.inspect_min_evolution_hierarchy_weight_delta,
            Some(0.05)
        );
        assert_eq!(args.inspect_min_evolution_memory_updates, Some(9));
        assert_eq!(
            args.inspect_min_evolution_replay_live_memory_feedback_updates,
            Some(10)
        );
        assert_eq!(args.inspect_min_evolution_recursive_replay_items, Some(10));
        assert_eq!(args.inspect_min_evolution_recursive_runtime_calls, Some(11));
        assert_eq!(args.inspect_max_evolution_drift_rollbacks, Some(0));
        assert_eq!(
            args.inspect_max_evolution_rollback_router_threshold_delta,
            Some(0.0)
        );
        assert_eq!(
            args.inspect_max_evolution_rollback_hierarchy_weight_delta,
            Some(0.0)
        );
        assert!(args.inspect_require_runtime_kv_dimensions);
        assert_eq!(args.state_inspection_gate().min_memories, Some(3));
        assert_eq!(
            args.state_inspection_gate().min_runtime_kv_memories,
            Some(1)
        );
        assert_eq!(args.state_inspection_gate().min_experiences, Some(2));
        assert_eq!(
            args.state_inspection_gate().min_runtime_model_experiences,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate().min_runtime_adapter_experiences,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_runtime_forward_energy_experiences,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_runtime_kv_influence_experiences,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_runtime_layer_mode_experiences,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_runtime_all_layer_mode_experiences,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate().min_runtime_global_layers,
            Some(4)
        );
        assert_eq!(
            args.state_inspection_gate().min_runtime_local_window_layers,
            Some(6)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_runtime_convolutional_fusion_layers,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_runtime_kv_import_experiences,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_runtime_kv_export_experiences,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_reflection_issue_experiences,
            Some(3)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_critical_reflection_issue_experiences,
            Some(1)
        );
        assert_eq!(
            args.state_inspection_gate().min_revision_action_experiences,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_live_memory_feedback_experiences,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_live_memory_feedback_updates,
            Some(5)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_live_inference_runs,
            Some(4)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_live_router_threshold_mutations,
            Some(5)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_live_hierarchy_weight_mutations,
            Some(6)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_live_router_threshold_delta,
            Some(0.06)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_live_hierarchy_weight_delta,
            Some(0.07)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_live_memory_updates,
            Some(8)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_live_stored_memory_updates,
            Some(7)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_live_reflection_issues,
            Some(6)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_live_critical_reflection_issues,
            Some(2)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_live_revision_actions,
            Some(5)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_runtime_kv_memory_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_runtime_model_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_runtime_adapter_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_runtime_forward_energy_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_runtime_kv_influence_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_runtime_layer_mode_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_runtime_all_layer_mode_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_runtime_kv_import_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_runtime_kv_export_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_reflection_issue_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_critical_reflection_issue_device_profiles,
            Some(6)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_revision_action_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_live_memory_feedback_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_live_inference_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_live_router_threshold_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_live_hierarchy_weight_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_live_memory_update_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_live_stored_memory_update_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_live_reflection_issue_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_live_critical_reflection_issue_device_profiles,
            Some(6)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_live_revision_action_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_replay_run_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_replay_item_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_router_threshold_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_hierarchy_weight_mutation_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_memory_update_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_replay_live_memory_feedback_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_recursive_replay_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_matrix_gate()
                .min_evolution_recursive_runtime_call_device_profiles,
            Some(12)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_router_threshold_delta,
            Some(0.04)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_hierarchy_weight_delta,
            Some(0.05)
        );
        assert_eq!(
            args.state_inspection_gate().min_evolution_memory_updates,
            Some(9)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_replay_live_memory_feedback_updates,
            Some(10)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_recursive_replay_items,
            Some(10)
        );
        assert_eq!(
            args.state_inspection_gate()
                .min_evolution_recursive_runtime_calls,
            Some(11)
        );
        assert_eq!(
            args.state_inspection_gate()
                .max_evolution_rollback_router_threshold_delta,
            Some(0.0)
        );
        assert_eq!(
            args.state_inspection_gate()
                .max_evolution_rollback_hierarchy_weight_delta,
            Some(0.0)
        );
        assert!(args.state_inspection_gate().require_runtime_kv_dimensions);
        assert!(args.local_runtime);
        assert!(args.production_runtime);
        assert!(args.production_reference_kernel);
        assert!(args.production_local_kernel);
        assert!(args.production_kernel_conformance_gate);
        assert!(args.production_kernel_conformance_all_devices_gate);
        assert_eq!(args.runtime_metadata.model_id, "dev-transformer");
        assert_eq!(args.runtime_metadata.tokenizer, "dev-bpe");
        assert_eq!(args.runtime_wire_format, CommandWireFormat::Json);
        assert_eq!(args.runtime_metadata.native_context_window, 4096);
        assert_eq!(args.runtime_metadata.embedding_dimensions, 128);
        assert!(args.runtime_metadata.supports_kv_import);
        assert!(args.runtime_metadata.supports_kv_export);
        assert_eq!(args.runtime_metadata.max_kv_import_blocks, 8);
        assert_eq!(args.runtime_metadata.max_kv_export_blocks, 4);
        assert_eq!(args.runtime_metadata.hot_kv_precision_bits, 8);
        assert_eq!(args.runtime_metadata.cold_kv_precision_bits, 4);
        assert_eq!(args.runtime_manifest().architecture.layer_count, 18);
        assert_eq!(args.runtime_manifest().architecture.hidden_size, 128);
        assert_eq!(args.runtime_manifest().architecture.attention_heads, 8);
        assert_eq!(args.runtime_manifest().architecture.kv_heads, 4);
        assert_eq!(
            args.runtime_manifest().architecture.local_window_tokens,
            2048
        );
        assert_eq!(args.device, DeviceClass::CpuOnly);
        assert_eq!(args.cpu_load, 75.0);
        assert_eq!(args.ram_load, 0.5);
        assert!(args.prompt.contains("nine"));
    }

    #[test]
    fn inspect_threshold_flags_imply_state_gate() {
        let args = Args::parse(vec![
            "--inspect-min-runtime-kv-memories".to_owned(),
            "1".to_owned(),
            "--inspect-min-experiences".to_owned(),
            "1".to_owned(),
        ]);

        assert!(args.inspect_state);
        assert!(args.inspect_gate);
        assert_eq!(args.inspect_min_runtime_kv_memories, Some(1));
        assert_eq!(args.inspect_min_experiences, Some(1));
    }

    #[test]
    fn inspect_all_devices_flag_implies_state_gate() {
        let args = Args::parse(vec![
            "--inspect-state".to_owned(),
            "--benchmark-all-devices".to_owned(),
        ]);

        assert!(args.inspect_state);
        assert!(args.inspect_gate);
        assert!(args.benchmark_all_devices);
    }

    #[test]
    fn runtime_manifest_gate_builds_production_manifest_from_cli_assets() {
        let asset_dir = temp_asset_dir("runtime-manifest-cli-assets");
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        let config = asset_dir.join("config.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        File::create(&config).unwrap();
        let args = Args::parse(vec![
            "--runtime-manifest-gate".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "65536".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "256".to_owned(),
            "--runtime-layers".to_owned(),
            "32".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "256".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "8".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "4".to_owned(),
            "--runtime-local-window".to_owned(),
            "8192".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
            "--runtime-config".to_owned(),
            config.display().to_string(),
            "--device".to_owned(),
            "cpu".to_owned(),
        ]);

        let manifest = args.runtime_manifest();
        let validation = manifest.validate_for_production();
        let device_gate = RuntimeManifestDeviceGateReport::evaluate(
            &manifest,
            &args.runtime_manifest_device_plan(),
        );
        let all_devices_gate = DevicePlanGateReport::evaluate_runtime_manifest(&manifest);

        assert!(args.runtime_manifest_gate);
        assert!(!args.runtime_manifest_all_devices_gate);
        assert_eq!(manifest.metadata.model_id, "self-owned-transformer");
        assert_eq!(manifest.metadata.tokenizer, "self-bpe");
        assert_eq!(manifest.metadata.native_context_window, 65_536);
        assert_eq!(manifest.metadata.embedding_dimensions, 256);
        assert_eq!(manifest.architecture.layer_count, 32);
        assert_eq!(manifest.architecture.hidden_size, 256);
        assert_eq!(manifest.architecture.attention_heads, 8);
        assert_eq!(manifest.architecture.kv_heads, 4);
        assert_eq!(manifest.architecture.local_window_tokens, 8_192);
        assert!(manifest.metadata.supports_kv_import);
        assert!(manifest.metadata.supports_kv_export);
        assert!(validation.passed(), "{validation:?}");
        assert!(device_gate.passed(), "{device_gate:?}");
        assert!(all_devices_gate.passed(), "{all_devices_gate:?}");
        assert_eq!(
            all_devices_gate.rows.len(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(device_gate.device, DeviceClass::CpuOnly);
        assert_eq!(device_gate.runtime_adapter_name(), "portable-rust");
        assert!(device_gate.runtime_device_contract.contains("device=cpu"));
        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn runtime_manifest_all_devices_gate_cli_flag_enables_manifest_gate() {
        let args = Args::parse(vec![
            "--runtime-manifest-all-devices-gate".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-model-id".to_owned(),
            "all-device-transformer".to_owned(),
            "--runtime-native-window".to_owned(),
            "65536".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "256".to_owned(),
        ]);

        let report = DevicePlanGateReport::evaluate_runtime_manifest(&args.runtime_manifest());

        assert!(args.runtime_manifest_gate);
        assert!(args.runtime_manifest_all_devices_gate);
        assert!(report.passed(), "{report:?}");
        assert_eq!(report.rows.len(), DeviceClass::explicit_profiles().len());
    }

    #[test]
    fn production_runtime_cli_builds_manifest_backed_boundary() {
        let asset_dir = temp_asset_dir("production-runtime-cli-assets");
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let args = Args::parse(vec![
            "--production-runtime".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "4096".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "64".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "1024".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
            "--device".to_owned(),
            "cpu".to_owned(),
            "connect production runtime".to_owned(),
        ]);

        let runtime = args.production_runtime().unwrap();

        assert!(args.production_runtime);
        assert_eq!(runtime.metadata().model_id, "self-owned-transformer");
        assert_eq!(runtime.architecture().layer_count, 6);
        assert!(runtime.device_gate().passed());
        assert_eq!(
            runtime.device_gate().runtime_adapter_name(),
            "portable-rust"
        );
        assert!(runtime.assets().summary_line().contains("weights_bytes=0"));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_kernel_conformance_gate_cli_passes_reference_kernel() {
        let asset_dir = temp_asset_dir("production-runtime-cli-conformance-reference");
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let args = Args::parse(vec![
            "--production-reference-kernel".to_owned(),
            "--production-kernel-conformance-gate".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "4096".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "64".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "1024".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
            "--device".to_owned(),
            "cpu".to_owned(),
        ]);

        let runtime = args.production_runtime().unwrap();
        let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

        assert!(args.production_runtime);
        assert!(args.production_reference_kernel);
        assert!(args.production_kernel_conformance_gate);
        assert!(report.passed, "{report:?}");
        assert_eq!(report.model_id, "self-owned-transformer");
        assert_eq!(report.selected_adapter, "portable-rust");
        assert!(report.exported_kv_blocks > 0);

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_local_kernel_cli_passes_conformance_gate() {
        let asset_dir = temp_asset_dir("production-runtime-cli-local-kernel");
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let args = Args::parse(vec![
            "--production-local-kernel".to_owned(),
            "--production-kernel-conformance-gate".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "4096".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "64".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "1024".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
            "--device".to_owned(),
            "cpu".to_owned(),
        ]);

        let runtime = args.production_runtime().unwrap();
        let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

        assert!(args.production_runtime);
        assert!(args.production_local_kernel);
        assert!(args.production_kernel_conformance_gate);
        assert!(report.passed, "{report:?}");
        assert_eq!(report.model_id, "self-owned-transformer");
        assert!(report.exported_kv_blocks > 0);

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_kernel_conformance_gate_cli_fails_without_kernel() {
        let asset_dir = temp_asset_dir("production-runtime-cli-conformance-missing");
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let args = Args::parse(vec![
            "--production-kernel-conformance-gate".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "4096".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "64".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "1024".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
            "--device".to_owned(),
            "cpu".to_owned(),
        ]);

        let runtime = args.production_runtime().unwrap();
        let report = runtime.conformance_report(ProductionKernelConformanceGate::default());

        assert!(args.production_runtime);
        assert!(!args.production_reference_kernel);
        assert!(args.production_kernel_conformance_gate);
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
    fn production_reference_kernel_conformance_all_devices_cli_passes() {
        assert_production_kernel_conformance_all_devices_cli_passes(
            "--production-reference-kernel",
            "production-runtime-cli-conformance-reference-matrix",
        );
    }

    #[test]
    fn production_local_kernel_conformance_all_devices_cli_passes() {
        assert_production_kernel_conformance_all_devices_cli_passes(
            "--production-local-kernel",
            "production-runtime-cli-conformance-local-matrix",
        );
    }

    #[test]
    fn production_kernel_conformance_all_devices_cli_fails_without_kernel() {
        let asset_dir = temp_asset_dir("production-runtime-cli-conformance-missing-matrix");
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let args = Args::parse(production_conformance_all_devices_args(
            "--production-kernel-conformance-all-devices-gate",
            &weights,
            &tokenizer,
        ));

        let report = run_production_kernel_conformance_all_devices(&args);

        assert!(args.production_runtime);
        assert!(args.production_kernel_conformance_gate);
        assert!(args.production_kernel_conformance_all_devices_gate);
        assert!(!report.passed);
        assert_eq!(
            report.covered_devices(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.failed_devices().len(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(report.device_reports.iter().all(|device_report| {
            !device_report.report.kernel_connected
                && device_report
                    .report
                    .failures
                    .iter()
                    .any(|failure| failure.contains("production forward kernel is not connected"))
        }));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_cli_generate_reports_kernel_boundary_error() {
        let asset_dir = temp_asset_dir("production-runtime-cli-generate");
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let args = Args::parse(vec![
            "--production-runtime".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "4096".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "64".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "1024".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
            "--device".to_owned(),
            "cpu".to_owned(),
            "connect production runtime".to_owned(),
        ]);
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &args);
        let mut backend = RuntimeBackend::new(args.production_runtime().unwrap());

        let outcome = engine.infer(
            InferenceRequest::new(args.prompt.clone(), args.profile),
            &mut backend,
        );

        assert!(outcome.answer.contains("kernel is not connected"));
        assert!(
            backend
                .last_error()
                .unwrap()
                .message()
                .contains("kernel is not connected")
        );
        assert!(outcome.report.quality < 0.5);

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_runtime_cli_can_run_reference_kernel_end_to_end() {
        let asset_dir = temp_asset_dir("production-runtime-cli-reference");
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let args = Args::parse(vec![
            "--production-reference-kernel".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "4096".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "64".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "1024".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
            "--device".to_owned(),
            "cpu".to_owned(),
            "connect production reference kernel with Noiron memory".to_owned(),
        ]);
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &args);
        let mut backend = RuntimeBackend::new(args.production_runtime().unwrap());

        let outcome = engine.infer(
            InferenceRequest::new(args.prompt.clone(), args.profile),
            &mut backend,
        );

        assert!(args.production_runtime);
        assert!(args.production_reference_kernel);
        assert!(backend.last_error().is_none());
        assert!(
            outcome
                .answer
                .contains("Reference production Transformer kernel result")
        );
        assert!(outcome.runtime_diagnostics.has_forward_signal());
        assert_eq!(
            outcome.runtime_diagnostics.model_id.as_deref(),
            Some("self-owned-transformer")
        );
        assert_eq!(
            outcome.runtime_diagnostics.selected_adapter.as_deref(),
            Some("portable-rust")
        );
        assert_eq!(outcome.runtime_diagnostics.layer_count, 6);
        assert!(outcome.exported_runtime_kv_blocks > 0);
        assert!(outcome.report.quality > 0.46);

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_reference_kernel_benchmark_passes_runtime_and_trace_gates() {
        let asset_dir = temp_asset_dir("production-reference-benchmark");
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let trace_path = asset_dir.join("benchmark.jsonl");
        let args = Args::parse(vec![
            "--production-reference-kernel".to_owned(),
            "--benchmark".to_owned(),
            trace_path.display().to_string(),
            "--benchmark-gate".to_owned(),
            "--benchmark-min-quality".to_owned(),
            "0.45".to_owned(),
            "--benchmark-min-reward".to_owned(),
            "0.35".to_owned(),
            "--benchmark-min-runtime-forward-cases".to_owned(),
            "4".to_owned(),
            "--benchmark-min-runtime-kv-exported".to_owned(),
            "4".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "4096".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "64".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "1024".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
            "--device".to_owned(),
            "cpu".to_owned(),
        ]);
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &args);
        let runtime = args.production_runtime().unwrap();
        let mut backend = RuntimeBackend::new(runtime);

        let summary = run_benchmark(&mut engine, &mut backend, &trace_path).unwrap();
        let gate_report = summary.evaluate(&args.benchmark_gate());
        let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

        assert_eq!(summary.len(), 4);
        assert_eq!(summary.runtime_forward_cases(), 4);
        assert!(summary.total_runtime_kv_exported() >= 4);
        assert!(summary.summary_line().contains("runtime_forward_cases=4"));
        assert!(summary.summary_line().contains("runtime_kv_exported="));
        assert!(gate_report.passed, "{:?}", gate_report.failures);
        assert!(trace_report.passed, "{:?}", trace_report.failures);
        assert_eq!(trace_report.checked_lines, 4);
        assert!(backend.last_error().is_none());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn benchmark_all_devices_runs_every_explicit_profile() {
        let asset_dir = temp_asset_dir("all-device-benchmark");
        fs::create_dir_all(&asset_dir).unwrap();
        let trace_path = asset_dir.join("benchmark.jsonl");
        let args = Args::parse(vec![
            "--benchmark".to_owned(),
            trace_path.display().to_string(),
            "--benchmark-all-devices".to_owned(),
            "--benchmark-gate".to_owned(),
            "--benchmark-min-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--benchmark-max-drift-blocks".to_owned(),
            "0".to_owned(),
            "--benchmark-max-drift-rollbacks".to_owned(),
            "0".to_owned(),
            "--device".to_owned(),
            "cpu".to_owned(),
        ]);
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &args);
        let mut backend = HeuristicBackend;

        let summary =
            run_benchmark_for_args(&mut engine, &mut backend, &args, &trace_path).unwrap();
        let gate_report = summary.evaluate(&args.benchmark_gate());
        let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

        assert!(args.benchmark_all_devices);
        assert_eq!(
            summary.len(),
            DeviceClass::explicit_profiles().len() * default_benchmark_cases().len()
        );
        assert_eq!(
            summary.explicit_device_profiles_covered(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(summary.missing_explicit_device_profiles().is_empty());
        assert!(summary.results().iter().any(|result| {
            result.device == DeviceClass::Microcontroller
                && result.name.starts_with("microcontroller_")
        }));
        assert!(summary.summary_line().contains("device_profiles=12"));
        assert!(gate_report.passed, "{:?}", gate_report.failures);
        assert!(trace_report.passed, "{:?}", trace_report.failures);
        assert_eq!(trace_report.checked_lines, summary.len());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn benchmark_all_devices_can_gate_recursive_coverage_per_profile() {
        let asset_dir = temp_asset_dir("all-device-recursive-benchmark");
        fs::create_dir_all(&asset_dir).unwrap();
        let trace_path = asset_dir.join("benchmark.jsonl");
        let args = Args::parse(vec![
            "--benchmark".to_owned(),
            trace_path.display().to_string(),
            "--benchmark-all-devices".to_owned(),
            "--benchmark-gate".to_owned(),
            "--benchmark-min-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--benchmark-min-recursive-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--benchmark-min-recursive-cases".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--benchmark-max-drift-blocks".to_owned(),
            "0".to_owned(),
            "--benchmark-max-drift-rollbacks".to_owned(),
            "0".to_owned(),
            "--native-window".to_owned(),
            "64".to_owned(),
            "--chunk-tokens".to_owned(),
            "32".to_owned(),
            "--chunk-overlap".to_owned(),
            "8".to_owned(),
            "--device".to_owned(),
            "cpu".to_owned(),
        ]);
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &args);
        let mut backend = HeuristicBackend;

        let summary =
            run_benchmark_for_args(&mut engine, &mut backend, &args, &trace_path).unwrap();
        let gate_report = summary.evaluate(&args.benchmark_gate());
        let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

        assert_eq!(
            summary.explicit_device_profiles_covered(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            summary.recursive_device_profiles_covered(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            summary.recursive_cases(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(
            summary
                .summary_line()
                .contains("recursive_device_profiles=12")
        );
        assert!(gate_report.passed, "{:?}", gate_report.failures);
        assert!(trace_report.passed, "{:?}", trace_report.failures);
        assert_eq!(trace_report.checked_lines, summary.len());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn production_reference_kernel_all_devices_gates_recursive_runtime_coverage() {
        assert_production_kernel_all_devices_gate_recursive_runtime_coverage(
            "--production-reference-kernel",
            "production-reference-all-device-recursive",
        );
    }

    #[test]
    fn production_local_kernel_all_devices_gates_recursive_runtime_coverage() {
        assert_production_kernel_all_devices_gate_recursive_runtime_coverage(
            "--production-local-kernel",
            "production-local-all-device-recursive",
        );
    }

    #[test]
    fn persistent_roundtrip_all_devices_verifies_runtime_kv_namespace_reuse() {
        let asset_dir = temp_asset_dir("roundtrip-all-devices");
        fs::create_dir_all(&asset_dir).unwrap();
        let args = Args::parse(vec![
            "--benchmark-roundtrip".to_owned(),
            "--benchmark-all-devices".to_owned(),
            "--memory".to_owned(),
            asset_dir.join("memory.ndkv").display().to_string(),
            "--experience".to_owned(),
            asset_dir.join("experience.ndkv").display().to_string(),
            "--adaptive".to_owned(),
            asset_dir.join("adaptive.ndkv").display().to_string(),
            "--profile".to_owned(),
            "coding".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "32".to_owned(),
            "Verify persistent runtime KV reuse across every supported device".to_owned(),
        ]);

        let report = run_persistent_roundtrip_all_devices(&args).unwrap();

        assert!(args.benchmark_roundtrip);
        assert!(args.benchmark_all_devices);
        assert!(report.passed, "{:?}", report.failures);
        assert_eq!(
            report.covered_devices(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.device_reports.len(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(report.missing_devices().is_empty());
        assert!(report.failed_devices().is_empty());
        assert!(report.summary_line().contains("devices=12"));
        assert!(report.device_reports.iter().all(|device_report| {
            device_report.report.first_runtime_kv_namespace_preserved
                && device_report.report.second_used_runtime_kv_memory
                && device_report
                    .report
                    .second_imported_runtime_kv_from_namespace
                && device_report.report.second_runtime_adapter_best_adapter
                    == device_report.report.second_runtime_selected_adapter
        }));
        assert!(
            device_scoped_path(&args.memory_path, DeviceClass::CpuOnly)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("memory.cpu.ndkv")
        );
        assert!(device_scoped_path(&args.memory_path, DeviceClass::CpuOnly).exists());
        assert!(device_scoped_path(&args.memory_path, DeviceClass::Mobile).exists());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn state_inspection_all_devices_gates_roundtrip_state_files() {
        let asset_dir = temp_asset_dir("inspect-all-devices");
        fs::create_dir_all(&asset_dir).unwrap();
        let roundtrip_args = Args::parse(vec![
            "--benchmark-roundtrip".to_owned(),
            "--benchmark-all-devices".to_owned(),
            "--memory".to_owned(),
            asset_dir.join("memory.ndkv").display().to_string(),
            "--experience".to_owned(),
            asset_dir.join("experience.ndkv").display().to_string(),
            "--adaptive".to_owned(),
            asset_dir.join("adaptive.ndkv").display().to_string(),
            "--profile".to_owned(),
            "coding".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "32".to_owned(),
            "Create inspectable runtime KV state for every supported device".to_owned(),
        ]);
        let roundtrip = run_persistent_roundtrip_all_devices(&roundtrip_args).unwrap();
        assert!(roundtrip.passed, "{:?}", roundtrip.failures);

        let inspect_args = Args::parse(vec![
            "--inspect-state".to_owned(),
            "--benchmark-all-devices".to_owned(),
            "--memory".to_owned(),
            asset_dir.join("memory.ndkv").display().to_string(),
            "--experience".to_owned(),
            asset_dir.join("experience.ndkv").display().to_string(),
            "--adaptive".to_owned(),
            asset_dir.join("adaptive.ndkv").display().to_string(),
            "--inspect-min-memories".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-memories".to_owned(),
            "1".to_owned(),
            "--inspect-min-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-model-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-adapter-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-forward-energy-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-influence-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-layer-mode-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-all-layer-mode-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-global-layers".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-local-window-layers".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-convolutional-fusion-layers".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-import-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-export-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-memory-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-model-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-adapter-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-forward-energy-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-kv-influence-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-layer-mode-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-all-layer-mode-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-kv-import-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-kv-export-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-reflection-issue-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-revision-action-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-live-memory-feedback-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-evolution-replay-run-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-evolution-replay-item-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-evolution-memory-update-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-evolution-memory-updates".to_owned(),
            "1".to_owned(),
            "--inspect-require-runtime-kv-dimensions".to_owned(),
        ]);
        let report = run_state_inspection_all_devices(&inspect_args).unwrap();

        assert!(inspect_args.inspect_state);
        assert!(inspect_args.inspect_gate);
        assert!(inspect_args.benchmark_all_devices);
        assert!(report.passed(), "{:?}", report.failures);
        assert_eq!(
            report.covered_devices(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.runtime_kv_memory_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.runtime_model_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.runtime_adapter_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.runtime_forward_energy_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.runtime_kv_influence_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.runtime_layer_mode_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.runtime_all_layer_mode_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.runtime_kv_import_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.runtime_kv_export_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.reflection_issue_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.revision_action_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.live_memory_feedback_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.evolution_replay_run_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.evolution_replay_item_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.evolution_memory_update_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(report.missing_devices().is_empty());
        assert!(report.failed_devices().is_empty());
        assert!(
            report
                .summary_line()
                .contains("state_inspection_matrix_gate: passed=true")
        );
        assert!(report.device_reports.iter().all(|device_report| {
            device_report.report.passed()
                && device_report.report.summary_line().contains("passed=true")
        }));
        assert!(device_scoped_path(&inspect_args.memory_path, DeviceClass::Server).exists());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn roundtrip_and_inspect_state_can_chain_single_device_gate() {
        let asset_dir = temp_asset_dir("roundtrip-inspect-single");
        fs::create_dir_all(&asset_dir).unwrap();
        let args = Args::parse(vec![
            "--benchmark-roundtrip".to_owned(),
            "--inspect-state".to_owned(),
            "--inspect-gate".to_owned(),
            "--memory".to_owned(),
            asset_dir.join("memory.ndkv").display().to_string(),
            "--experience".to_owned(),
            asset_dir.join("experience.ndkv").display().to_string(),
            "--adaptive".to_owned(),
            asset_dir.join("adaptive.ndkv").display().to_string(),
            "--profile".to_owned(),
            "coding".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "32".to_owned(),
            "--inspect-min-runtime-kv-memories".to_owned(),
            "1".to_owned(),
            "--inspect-min-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-model-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-adapter-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-forward-energy-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-influence-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-import-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-export-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-live-memory-feedback-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-live-memory-feedback-updates".to_owned(),
            "1".to_owned(),
            "--inspect-min-reflection-issue-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-revision-action-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-live-memory-feedback-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-evolution-memory-updates".to_owned(),
            "1".to_owned(),
            "--inspect-require-runtime-kv-dimensions".to_owned(),
            "Chain roundtrip into inspect gate for self-owned runtime state".to_owned(),
        ]);

        let roundtrip = run_persistent_roundtrip(&args).unwrap();
        let inspect = run_state_inspection(&args).unwrap();
        let gate = inspect.evaluate(&args.state_inspection_gate());

        assert!(args.benchmark_roundtrip);
        assert!(args.inspect_state);
        assert!(args.inspect_gate);
        assert!(roundtrip.passed, "{:?}", roundtrip.failures);
        assert!(gate.passed(), "{:?}", gate.failures);
        assert!(args.memory_path.exists());
        assert!(args.experience_path.exists());
        assert!(args.adaptive_path.exists());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn roundtrip_and_inspect_state_can_chain_all_device_gate() {
        let asset_dir = temp_asset_dir("roundtrip-inspect-all-devices");
        fs::create_dir_all(&asset_dir).unwrap();
        let args = Args::parse(vec![
            "--benchmark-roundtrip".to_owned(),
            "--inspect-state".to_owned(),
            "--benchmark-all-devices".to_owned(),
            "--memory".to_owned(),
            asset_dir.join("memory.ndkv").display().to_string(),
            "--experience".to_owned(),
            asset_dir.join("experience.ndkv").display().to_string(),
            "--adaptive".to_owned(),
            asset_dir.join("adaptive.ndkv").display().to_string(),
            "--profile".to_owned(),
            "coding".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "32".to_owned(),
            "--inspect-min-runtime-kv-memories".to_owned(),
            "1".to_owned(),
            "--inspect-min-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-model-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-adapter-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-forward-energy-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-influence-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-import-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-export-experiences".to_owned(),
            "1".to_owned(),
            "--inspect-min-runtime-kv-memory-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-model-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-adapter-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-forward-energy-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-kv-influence-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-kv-import-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-runtime-kv-export-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-live-memory-feedback-device-profiles".to_owned(),
            DeviceClass::explicit_profiles().len().to_string(),
            "--inspect-min-evolution-memory-updates".to_owned(),
            "1".to_owned(),
            "--inspect-require-runtime-kv-dimensions".to_owned(),
            "Chain all-device roundtrip into inspect gate".to_owned(),
        ]);

        let roundtrip = run_persistent_roundtrip_all_devices(&args).unwrap();
        let inspect = run_state_inspection_all_devices(&args).unwrap();

        assert!(args.benchmark_roundtrip);
        assert!(args.inspect_state);
        assert!(args.inspect_gate);
        assert!(args.benchmark_all_devices);
        assert!(roundtrip.passed, "{:?}", roundtrip.failures);
        assert!(inspect.passed(), "{:?}", inspect.failures);
        assert_eq!(
            inspect.covered_devices(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            inspect.runtime_kv_memory_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            inspect.runtime_model_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            inspect.runtime_adapter_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            inspect.runtime_forward_energy_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            inspect.runtime_kv_influence_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            inspect.runtime_kv_import_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            inspect.runtime_kv_export_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            inspect.reflection_issue_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            inspect.revision_action_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            inspect.live_memory_feedback_device_profiles(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(device_scoped_path(&args.memory_path, DeviceClass::CpuOnly).exists());
        assert!(device_scoped_path(&args.memory_path, DeviceClass::Server).exists());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn state_inspection_all_devices_fails_missing_scoped_state_files() {
        let asset_dir = temp_asset_dir("inspect-missing-all-devices");
        fs::create_dir_all(&asset_dir).unwrap();
        let inspect_args = Args::parse(vec![
            "--inspect-state".to_owned(),
            "--benchmark-all-devices".to_owned(),
            "--memory".to_owned(),
            asset_dir.join("memory.ndkv").display().to_string(),
            "--experience".to_owned(),
            asset_dir.join("experience.ndkv").display().to_string(),
            "--adaptive".to_owned(),
            asset_dir.join("adaptive.ndkv").display().to_string(),
        ]);

        let report = run_state_inspection_all_devices(&inspect_args).unwrap();

        assert!(!report.passed());
        assert_eq!(
            report.covered_devices(),
            DeviceClass::explicit_profiles().len()
        );
        assert_eq!(
            report.failed_devices().len(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(report.device_reports.iter().all(|device_report| {
            device_report
                .report
                .failures
                .iter()
                .any(|failure| failure.contains("memory file missing"))
        }));
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("device cpu state inspection failed"))
        );

        fs::remove_dir_all(asset_dir).unwrap();
    }

    fn assert_production_kernel_all_devices_gate_recursive_runtime_coverage(
        kernel_flag: &str,
        asset_name: &str,
    ) {
        let asset_dir = temp_asset_dir(asset_name);
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let trace_path = asset_dir.join("benchmark.jsonl");
        let device_count = DeviceClass::explicit_profiles().len();
        let case_count = default_benchmark_cases().len();
        let min_adapter_kinds = if kernel_flag == "--production-reference-kernel" {
            6
        } else {
            1
        };
        let args = Args::parse(vec![
            kernel_flag.to_owned(),
            "--benchmark".to_owned(),
            trace_path.display().to_string(),
            "--benchmark-all-devices".to_owned(),
            "--benchmark-gate".to_owned(),
            "--benchmark-min-quality".to_owned(),
            "0.45".to_owned(),
            "--benchmark-min-reward".to_owned(),
            "0.30".to_owned(),
            "--benchmark-min-device-profiles".to_owned(),
            device_count.to_string(),
            "--benchmark-min-recursive-device-profiles".to_owned(),
            device_count.to_string(),
            "--benchmark-min-recursive-cases".to_owned(),
            device_count.to_string(),
            "--benchmark-min-runtime-forward-cases".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-forward-energy-cases".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-kv-influence-cases".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-layer-mode-cases".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-all-layer-mode-cases".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-global-layers".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-local-window-layers".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-convolutional-fusion-layers".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-uncertainty-cases".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-uncertainty-tokens".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-kv-import-cases".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-kv-imported".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-kv-exported".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-kv-stored".to_owned(),
            "1".to_owned(),
            "--benchmark-min-runtime-adapter-contract-cases".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-runtime-adapter-kinds".to_owned(),
            min_adapter_kinds.to_string(),
            "--benchmark-min-runtime-adapter-observations".to_owned(),
            "1".to_owned(),
            "--benchmark-min-runtime-adapter-best-score".to_owned(),
            "0.05".to_owned(),
            "--benchmark-max-runtime-adapter-contract-violations".to_owned(),
            "0".to_owned(),
            "--benchmark-max-memory-governance-failures".to_owned(),
            "0".to_owned(),
            "--benchmark-min-memory-governance-cases".to_owned(),
            (device_count * case_count).to_string(),
            "--benchmark-min-memory-governance-device-profiles".to_owned(),
            device_count.to_string(),
            "--benchmark-min-auto-replay-router-threshold-mutations".to_owned(),
            "1".to_owned(),
            "--benchmark-min-auto-replay-hierarchy-weight-mutations".to_owned(),
            "1".to_owned(),
            "--benchmark-min-auto-replay-router-threshold-delta".to_owned(),
            "0.001".to_owned(),
            "--benchmark-min-auto-replay-hierarchy-weight-delta".to_owned(),
            "0.001".to_owned(),
            "--benchmark-min-auto-replay-memory-updates".to_owned(),
            "1".to_owned(),
            "--benchmark-min-evolution-replay-runs".to_owned(),
            "1".to_owned(),
            "--benchmark-min-evolution-replay-items".to_owned(),
            "1".to_owned(),
            "--benchmark-min-evolution-router-threshold-mutations".to_owned(),
            "1".to_owned(),
            "--benchmark-min-evolution-hierarchy-weight-mutations".to_owned(),
            "1".to_owned(),
            "--benchmark-min-evolution-router-threshold-delta".to_owned(),
            "0.001".to_owned(),
            "--benchmark-min-evolution-hierarchy-weight-delta".to_owned(),
            "0.001".to_owned(),
            "--benchmark-min-evolution-memory-updates".to_owned(),
            "1".to_owned(),
            "--benchmark-min-evolution-recursive-replay-items".to_owned(),
            "1".to_owned(),
            "--benchmark-min-evolution-recursive-runtime-calls".to_owned(),
            "1".to_owned(),
            "--benchmark-max-drift-blocks".to_owned(),
            "0".to_owned(),
            "--benchmark-max-drift-rollbacks".to_owned(),
            "0".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "64".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "64".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "32".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
            "--chunk-tokens".to_owned(),
            "32".to_owned(),
            "--chunk-overlap".to_owned(),
            "8".to_owned(),
            "--device".to_owned(),
            "cpu".to_owned(),
        ]);
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &args);

        let summary =
            run_production_benchmark_all_devices(&mut engine, &args, &trace_path).unwrap();
        let gate_report = summary.evaluate(&args.benchmark_gate());
        let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

        assert!(args.production_runtime);
        match kernel_flag {
            "--production-reference-kernel" => assert!(args.production_reference_kernel),
            "--production-local-kernel" => assert!(args.production_local_kernel),
            _ => panic!("unexpected production kernel flag {kernel_flag}"),
        }
        assert_eq!(summary.len(), device_count * case_count);
        assert_eq!(summary.explicit_device_profiles_covered(), device_count);
        assert_eq!(summary.recursive_device_profiles_covered(), device_count);
        assert_eq!(summary.recursive_cases(), device_count);
        assert_eq!(summary.runtime_forward_cases(), device_count * case_count);
        assert_eq!(
            summary.runtime_forward_energy_cases(),
            device_count * case_count
        );
        assert_eq!(
            summary.runtime_kv_influence_cases(),
            device_count * case_count
        );
        assert_eq!(
            summary.runtime_layer_mode_cases(),
            device_count * case_count
        );
        assert_eq!(
            summary.runtime_all_layer_mode_cases(),
            device_count * case_count
        );
        assert!(summary.total_runtime_global_layers() >= device_count * case_count);
        assert!(summary.total_runtime_local_window_layers() >= device_count * case_count);
        assert!(summary.total_runtime_convolutional_fusion_layers() >= device_count * case_count);
        assert_eq!(
            summary.runtime_uncertainty_cases(),
            device_count * case_count
        );
        assert!(summary.total_runtime_uncertainty_tokens() >= device_count * case_count);
        assert_eq!(summary.runtime_kv_import_cases(), device_count * case_count);
        assert!(summary.total_runtime_kv_imported() >= device_count * case_count);
        assert_eq!(
            summary.runtime_adapter_contract_cases(),
            device_count * case_count
        );
        assert!(summary.runtime_adapter_kinds() >= min_adapter_kinds);
        assert!(summary.total_runtime_adapter_observations() >= 1);
        assert!(summary.max_runtime_adapter_score().unwrap_or(0.0) >= 0.05);
        assert_eq!(summary.total_runtime_adapter_contract_violations(), 0);
        assert_eq!(summary.memory_governance_cases(), device_count * case_count);
        assert_eq!(summary.memory_governance_device_profiles(), device_count);
        assert_eq!(summary.memory_governance_evidence().failures.len(), 0);
        assert!(summary.total_runtime_kv_exported() >= device_count * case_count);
        assert!(summary.total_runtime_kv_stored() >= 1);
        assert!(summary.total_auto_replay_router_threshold_mutations() >= 1);
        assert!(summary.total_auto_replay_hierarchy_weight_mutations() >= 1);
        assert!(summary.total_auto_replay_router_threshold_delta() >= 0.001);
        assert!(summary.total_auto_replay_hierarchy_weight_delta() >= 0.001);
        assert!(summary.total_auto_replay_memory_updates() >= 1);
        assert!(summary.evolution_ledger().replay_runs >= 1);
        assert!(summary.evolution_ledger().replay_items >= 1);
        assert!(summary.evolution_ledger().router_threshold_mutations >= 1);
        assert!(summary.evolution_ledger().hierarchy_weight_mutations >= 1);
        assert!(summary.evolution_ledger().router_threshold_delta >= 0.001);
        assert!(summary.evolution_ledger().hierarchy_weight_delta >= 0.001);
        assert!(summary.evolution_ledger().memory_updates() >= 1);
        assert!(summary.evolution_ledger().recursive_replay_items >= 1);
        assert!(summary.evolution_ledger().recursive_runtime_calls >= 1);
        assert!(summary.results().iter().any(|result| {
            result.device == DeviceClass::Microcontroller
                && result.name.starts_with("microcontroller_")
                && result.runtime_forward_signal
                && result.runtime_selected_adapter.as_deref() == Some("portable-rust")
                && result.runtime_adapter_contract_ok
        }));
        let trace = fs::read_to_string(&trace_path).unwrap();
        let microcontroller_line = trace
            .lines()
            .find(|line| line.contains("\"case\":\"microcontroller_long_context_scheduler\""))
            .unwrap();
        assert!(
            microcontroller_line.contains("\"runtime_device_contract\":\"device=microcontroller")
        );
        assert!(microcontroller_line.contains("\"selected_adapter\":\"portable-rust\""));
        assert!(microcontroller_line.contains("\"max_parallel_chunks\":1"));

        let discrete_line = trace
            .lines()
            .find(|line| line.contains("\"case\":\"discrete_long_context_scheduler\""))
            .unwrap();
        assert!(discrete_line.contains("\"runtime_device_contract\":\"device=discrete"));
        if kernel_flag == "--production-reference-kernel" {
            assert!(discrete_line.contains("\"selected_adapter\":\"cuda\""));
        } else {
            assert_trace_selected_adapter_allowed(
                discrete_line,
                &[
                    "cuda",
                    "rocm",
                    "vulkan",
                    "wgpu",
                    "oneapi",
                    "directml",
                    "portable-rust",
                ],
            );
        }
        assert!(discrete_line.contains("\"execution_waves\":23"));

        let multi_gpu_line = trace
            .lines()
            .find(|line| line.contains("\"case\":\"multi-gpu_long_context_scheduler\""))
            .unwrap();
        assert!(multi_gpu_line.contains("\"runtime_device_contract\":\"device=multi-gpu"));
        if kernel_flag == "--production-reference-kernel" {
            assert!(multi_gpu_line.contains("\"selected_adapter\":\"multi-device\""));
        } else {
            assert_trace_selected_adapter_allowed(
                multi_gpu_line,
                &[
                    "multi-device",
                    "cuda",
                    "rocm",
                    "oneapi",
                    "vulkan",
                    "wgpu",
                    "custom-accelerator",
                    "portable-rust",
                ],
            );
        }
        assert!(multi_gpu_line.contains("\"execution_waves\":12"));
        assert!(
            summary
                .summary_line()
                .contains("recursive_device_profiles=12")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_forward_energy_cases=48")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_kv_influence_cases=48")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_layer_mode_cases=48")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_all_layer_mode_cases=48")
        );
        assert!(summary.summary_line().contains("runtime_global_layers="));
        assert!(
            summary
                .summary_line()
                .contains("runtime_local_window_layers=")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_convolutional_fusion_layers=")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_adapter_contract_cases=48")
        );
        assert!(summary.summary_line().contains("runtime_adapter_kinds="));
        assert!(
            summary
                .summary_line()
                .contains("runtime_adapter_observations=")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_adapter_best_score=")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_uncertainty_cases=48")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_kv_import_cases=48")
        );
        assert!(summary.summary_line().contains("runtime_kv_stored="));
        assert!(
            summary
                .summary_line()
                .contains("auto_replay_router_threshold_mutations=")
        );
        assert!(
            summary
                .summary_line()
                .contains("auto_replay_hierarchy_weight_mutations=")
        );
        assert!(
            summary
                .summary_line()
                .contains("auto_replay_router_threshold_delta=")
        );
        assert!(
            summary
                .summary_line()
                .contains("auto_replay_hierarchy_weight_delta=")
        );
        assert!(
            summary
                .summary_line()
                .contains("runtime_adapter_contract_violations=0")
        );
        assert!(
            summary
                .summary_line()
                .contains("memory_governance_cases=48")
        );
        assert!(
            summary
                .summary_line()
                .contains("memory_governance_device_profiles=12")
        );
        assert!(
            summary
                .summary_line()
                .contains("memory_governance_failures=0")
        );
        assert!(gate_report.passed, "{:?}", gate_report.failures);
        assert!(trace_report.passed, "{:?}", trace_report.failures);
        assert_eq!(trace_report.checked_lines, summary.len());

        fs::remove_dir_all(asset_dir).unwrap();
    }

    fn assert_production_kernel_conformance_all_devices_cli_passes(
        kernel_flag: &str,
        asset_name: &str,
    ) {
        let asset_dir = temp_asset_dir(asset_name);
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let args = Args::parse(production_conformance_all_devices_args(
            kernel_flag,
            &weights,
            &tokenizer,
        ));

        let report = run_production_kernel_conformance_all_devices(&args);

        assert!(args.production_runtime);
        assert!(args.production_kernel_conformance_gate);
        assert!(args.production_kernel_conformance_all_devices_gate);
        match kernel_flag {
            "--production-reference-kernel" => assert!(args.production_reference_kernel),
            "--production-local-kernel" => assert!(args.production_local_kernel),
            _ => panic!("unexpected production kernel flag {kernel_flag}"),
        }
        assert!(report.passed, "{report:?}");
        assert_eq!(
            report.covered_devices(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(report.missing_devices().is_empty());
        assert!(report.failed_devices().is_empty());
        assert!(report.device_reports.iter().all(|device_report| {
            device_report.report.kernel_connected
                && device_report.report.token_count > 0
                && device_report.report.trace_steps > 0
                && device_report.report.imported_kv_blocks > 0
                && device_report.report.exported_kv_blocks > 0
        }));
        assert!(report.summary_line().contains("devices=12"));

        fs::remove_dir_all(asset_dir).unwrap();
    }

    fn production_conformance_all_devices_args(
        kernel_flag: &str,
        weights: &Path,
        tokenizer: &Path,
    ) -> Vec<String> {
        vec![
            kernel_flag.to_owned(),
            "--production-kernel-conformance-all-devices-gate".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "4096".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "64".to_owned(),
            "--runtime-layers".to_owned(),
            "6".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "64".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "4".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "2".to_owned(),
            "--runtime-local-window".to_owned(),
            "1024".to_owned(),
            "--runtime-kv-exchange".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
            "--device".to_owned(),
            "cpu".to_owned(),
        ]
    }

    fn assert_trace_selected_adapter_allowed(line: &str, allowed: &[&str]) {
        assert!(
            allowed
                .iter()
                .any(|adapter| line.contains(&format!("\"selected_adapter\":\"{adapter}\""))),
            "selected adapter was outside allowed set {:?}: {}",
            allowed,
            line
        );
    }

    #[test]
    fn runtime_manifest_gate_rejects_invalid_cli_architecture() {
        let asset_dir = temp_asset_dir("runtime-manifest-invalid-architecture");
        fs::create_dir_all(&asset_dir).unwrap();
        let weights = asset_dir.join("weights.noiron");
        let tokenizer = asset_dir.join("tokenizer.noiron");
        File::create(&weights).unwrap();
        File::create(&tokenizer).unwrap();
        let args = Args::parse(vec![
            "--runtime-manifest-gate".to_owned(),
            "--runtime-model-id".to_owned(),
            "self-owned-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "self-bpe".to_owned(),
            "--runtime-native-window".to_owned(),
            "4096".to_owned(),
            "--runtime-embedding-dims".to_owned(),
            "130".to_owned(),
            "--runtime-layers".to_owned(),
            "12".to_owned(),
            "--runtime-hidden-size".to_owned(),
            "130".to_owned(),
            "--runtime-attention-heads".to_owned(),
            "8".to_owned(),
            "--runtime-kv-heads".to_owned(),
            "16".to_owned(),
            "--runtime-local-window".to_owned(),
            "8192".to_owned(),
            "--runtime-weights".to_owned(),
            weights.display().to_string(),
            "--runtime-tokenizer-path".to_owned(),
            tokenizer.display().to_string(),
        ]);

        let validation = args.runtime_manifest().validate_for_production();

        assert!(!validation.passed());
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("hidden_size must be divisible"))
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("kv_heads must not exceed"))
        );
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("local_window_tokens must not exceed"))
        );
        fs::remove_dir_all(asset_dir).unwrap();
    }

    #[test]
    fn configure_engine_applies_memory_policy_flags() {
        let args = Args::parse(vec![
            "--device".to_owned(),
            "microcontroller".to_owned(),
            "--retention-stale-after".to_owned(),
            "9".to_owned(),
            "--retention-decay-rate".to_owned(),
            "1.5".to_owned(),
            "--retention-remove-below".to_owned(),
            "0.12".to_owned(),
            "--retention-remove-after-failures".to_owned(),
            "0".to_owned(),
            "--compaction-threshold".to_owned(),
            "0.05".to_owned(),
            "--compaction-max-candidates".to_owned(),
            "1".to_owned(),
            "--compaction-max-merges".to_owned(),
            "0".to_owned(),
        ]);
        let mut engine = NoironEngine::new();

        configure_engine(&mut engine, &args);

        assert_eq!(engine.memory_retention_policy.stale_after, 9);
        assert_eq!(engine.memory_retention_policy.decay_rate, 0.95);
        assert_eq!(engine.memory_retention_policy.remove_below_strength, 0.12);
        assert_eq!(engine.memory_retention_policy.remove_after_failures, 1);
        assert_eq!(engine.memory_compaction_policy.similarity_threshold, 0.10);
        assert_eq!(engine.memory_compaction_policy.max_candidates, 2);
        assert_eq!(engine.memory_compaction_policy.max_merges, 0);
    }

    #[test]
    fn configure_engine_applies_device_memory_governance_defaults() {
        let tiny_args = Args::parse(vec![
            "--device".to_owned(),
            "microcontroller".to_owned(),
            "--cpu-load".to_owned(),
            "30".to_owned(),
            "--ram-load".to_owned(),
            "35".to_owned(),
        ]);
        let server_args = Args::parse(vec![
            "--device".to_owned(),
            "server".to_owned(),
            "--cpu-load".to_owned(),
            "10".to_owned(),
            "--gpu-load".to_owned(),
            "15".to_owned(),
            "--ram-load".to_owned(),
            "20".to_owned(),
        ]);
        let mut tiny_engine = NoironEngine::new();
        let mut server_engine = NoironEngine::new();

        configure_engine(&mut tiny_engine, &tiny_args);
        configure_engine(&mut server_engine, &server_args);

        assert!(tiny_engine.memory_retention_policy.stale_after < 64);
        assert!(tiny_engine.memory_retention_policy.decay_rate > 0.04);
        assert!(tiny_engine.memory_compaction_policy.max_candidates < 512);
        assert!(tiny_engine.memory_compaction_policy.similarity_threshold > 0.92);
        assert!(server_engine.memory_retention_policy.stale_after > 64);
        assert!(server_engine.memory_retention_policy.decay_rate < 0.04);
        assert!(server_engine.memory_compaction_policy.max_candidates > 512);
    }

    #[test]
    fn auto_device_preserves_manual_load_overrides() {
        let args = Args::parse(vec![
            "--device".to_owned(),
            "auto".to_owned(),
            "--cpu-load".to_owned(),
            "91".to_owned(),
            "--gpu-load".to_owned(),
            "12".to_owned(),
            "--ram-load".to_owned(),
            "61".to_owned(),
            "--disk-load".to_owned(),
            "7".to_owned(),
            "probe defaults should not replace explicit loads".to_owned(),
        ]);

        assert_eq!(args.cpu_load, 91.0);
        assert_eq!(args.gpu_load, 12.0);
        assert_eq!(args.ram_load, 61.0);
        assert_eq!(args.disk_load, 7.0);
    }

    #[test]
    fn unknown_manual_device_uses_portable_cpu_fallback() {
        let args = Args::parse(vec![
            "--device".to_owned(),
            "future-device-sku".to_owned(),
            "unknown devices should still get a portable execution plan".to_owned(),
        ]);

        assert_eq!(args.device, DeviceClass::CpuOnly);
    }

    fn temp_asset_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{unique}"))
    }
}
