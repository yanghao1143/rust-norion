use std::env;
use std::path::PathBuf;
use std::time::Instant;

use rust_norion::{
    BenchmarkGate, BenchmarkGateReport, BenchmarkSummary, CommandPromptMode, CommandRuntime,
    CommandWireFormat, DeviceClass, DevicePlanGateReport, GistLevel, HardwareAllocator,
    HardwareSnapshot, HeuristicBackend, HierarchyWeights, InferenceBackend, InferenceOutcome,
    InferenceRequest, KvQuantBenchmarkGate, KvQuantBenchmarkGateReport, KvQuantBenchmarkSummary,
    LocalTransformerRuntime, MemoryCompactionPolicy, MemoryRetentionPolicy, ModelRuntime,
    NoironEngine, PersistentRoundtripInput, PersistentRoundtripReport, RecursiveScheduler,
    RuntimeBackend, RuntimeMetadata, StateInspectionReport, TaskProfile, TierMigrationAction,
    TraceSchemaGateReport, append_trace_jsonl, append_trace_jsonl_with_case,
    default_benchmark_cases, evaluate_trace_schema_jsonl,
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

    if args.inspect_state {
        let mut engine = NoironEngine::load_full_state(
            &args.memory_path,
            &args.experience_path,
            &args.adaptive_path,
        )?;
        configure_engine(&mut engine, &args);
        let report = StateInspectionReport::from_engine(&engine, args.inspect_limit);
        print_state_inspection_report(&args, &report);
        return Ok(());
    }

    if args.benchmark_roundtrip {
        let report = run_persistent_roundtrip(&args)?;
        print_persistent_roundtrip_report(&args, &report);
        if !report.passed {
            std::process::exit(2);
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
        let summary = if let Some(runtime_command) = args.runtime_command.clone() {
            let runtime = CommandRuntime::new(runtime_command)
                .args(args.runtime_args.clone())
                .prompt_mode(args.runtime_prompt_mode)
                .wire_format(args.runtime_wire_format)
                .with_metadata(args.runtime_metadata.clone());
            let mut backend = RuntimeBackend::new(runtime);
            run_benchmark(&mut engine, &mut backend, &benchmark_path)?
        } else if args.local_runtime {
            let runtime = LocalTransformerRuntime::with_metadata(args.runtime_metadata.clone());
            let mut backend = RuntimeBackend::new(runtime);
            run_benchmark(&mut engine, &mut backend, &benchmark_path)?
        } else {
            let mut backend = HeuristicBackend;
            run_benchmark(&mut engine, &mut backend, &benchmark_path)?
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

    let timed_outcome = if let Some(runtime_command) = args.runtime_command.clone() {
        let runtime = CommandRuntime::new(runtime_command)
            .args(args.runtime_args.clone())
            .prompt_mode(args.runtime_prompt_mode)
            .wire_format(args.runtime_wire_format)
            .with_metadata(args.runtime_metadata.clone());
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
        let runtime = LocalTransformerRuntime::with_metadata(args.runtime_metadata.clone());
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
    if args.local_runtime {
        println!("runtime: local-transformer");
        println!(
            "runtime_metadata: {}",
            LocalTransformerRuntime::with_metadata(args.runtime_metadata.clone())
                .metadata()
                .summary()
        );
    } else if let Some(runtime_command) = &args.runtime_command {
        println!("runtime_command: {}", runtime_command.display());
        println!("runtime_metadata: {}", args.runtime_metadata.summary());
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
    println!("stream_windows={}", outcome.stream_reports.len());
    println!(
        "memory: used={} stored={:?} experience_used={} experience={}",
        outcome.used_memories.len(),
        outcome.stored_memory_id,
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
    let mut first_backend = RuntimeBackend::new(LocalTransformerRuntime::with_metadata(
        args.runtime_metadata.clone(),
    ));
    let first = first_engine.infer(
        InferenceRequest::new(args.prompt.clone(), args.profile),
        &mut first_backend,
    );
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
    let mut second_backend = RuntimeBackend::new(LocalTransformerRuntime::with_metadata(
        args.runtime_metadata.clone(),
    ));
    let second = second_engine.infer(
        InferenceRequest::new(args.prompt.clone(), args.profile),
        &mut second_backend,
    );
    let second_imported_runtime_kv_blocks = second_backend.runtime().imported_kv_blocks().len();
    second_engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;

    Ok(PersistentRoundtripReport::evaluate(
        PersistentRoundtripInput {
            first_stored_memory: first.stored_memory_id.is_some(),
            first_runtime_kv_stored: first.stored_runtime_kv_memory_ids.len(),
            second_used_memories: second.used_memories.len(),
            second_used_experiences: second.used_experiences.len(),
            second_imported_runtime_kv_blocks,
            second_quality: second.report.quality,
            first_drift_severity: first.drift_report.severity,
            second_drift_severity: second.drift_report.severity,
        },
    ))
}

fn configure_engine(engine: &mut NoironEngine, args: &Args) {
    engine.recursive_scheduler = RecursiveScheduler::new(
        args.native_window_tokens,
        args.chunk_tokens,
        args.chunk_overlap_tokens,
        args.merge_fan_in,
    );
    engine.set_auto_replay_limit(args.auto_replay_limit);
    engine.set_memory_retention_policy(memory_retention_policy_from_args(engine, args));
    engine.set_memory_compaction_policy(memory_compaction_policy_from_args(engine, args));
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        args.device,
        args.cpu_load,
        args.gpu_load,
        args.ram_load,
        args.disk_load,
    ));
}

fn memory_retention_policy_from_args(engine: &NoironEngine, args: &Args) -> MemoryRetentionPolicy {
    let mut policy = engine.memory_retention_policy;

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
    engine: &NoironEngine,
    args: &Args,
) -> MemoryCompactionPolicy {
    let mut policy = engine.memory_compaction_policy.clone();

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
    println!("{}", summary.summary_line());

    for result in summary.results() {
        println!(
            "case={} profile={:?} elapsed_ms={} quality={:.3} reward={:.3} attention_fraction={:.2} requires_recursion={} chunks={} waves={} used_memories={} stored_memories={} compacted_memories={} runtime_kv_exported={} runtime_kv_stored={} drift={}",
            result.name,
            result.profile,
            result.elapsed_ms,
            result.quality,
            result.process_reward,
            result.attention_fraction,
            result.requires_recursion,
            result.recursive_chunks,
            result.recursive_waves,
            result.used_memories,
            result.stored_memories,
            result.compacted_memories,
            result.runtime_kv_exported,
            result.runtime_kv_stored,
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

    println!("top_experiences:");
    if report.top_experiences.is_empty() {
        println!("  none");
    } else {
        for experience in &report.top_experiences {
            println!(
                "  id={} profile={:?} quality={:.3} reward={:.3} action={} reflection_issues={} critical={} revision_actions={} lesson={}",
                experience.id,
                experience.profile,
                experience.quality,
                experience.process_reward,
                experience.reward_action.as_str(),
                experience.reflection_issues,
                experience.critical_reflection_issues,
                experience.revision_actions,
                experience.lesson
            );
        }
    }
}

fn print_device_matrix_and_exit() -> ! {
    let allocator = HardwareAllocator::new();
    let base_hierarchy = HierarchyWeights::default();

    println!("Noiron device matrix");
    println!(
        "profile,tier,scope,aliases,primary_lane,fallback_lane,memory_mode,adapters,parallel_chunks,kv_prefetch,kv_bits,disk_spill"
    );

    for device in DeviceClass::explicit_profiles() {
        let descriptor = device.descriptor();
        let plan = allocator.plan(
            HardwareSnapshot::new(*device, 0.35, 0.30, 0.45, 0.20),
            TaskProfile::General,
            4096,
            base_hierarchy,
        );
        let adapters = plan
            .execution
            .adapter_hints
            .iter()
            .map(|adapter| adapter.as_str())
            .collect::<Vec<_>>()
            .join("+");
        println!(
            "{},{},{},{},{},{},{},{},{},{},{}/{},{}",
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
            plan.execution.allow_disk_spill
        );
    }

    std::process::exit(0);
}

fn print_device_gate_report(report: &DevicePlanGateReport) {
    println!("Noiron device compatibility gate");
    println!("{}", report.summary_line());
    println!(
        "profile,tier,scope,aliases,primary_lane,fallback_lane,memory_mode,adapters,parallel_chunks,kv_prefetch,kv_bits,disk_spill,local_kv_tokens,global_kv_tokens,latency_budget_ms,passed"
    );

    for row in &report.rows {
        println!(
            "{},{},{},{},{},{},{},{},{},{},{}/{},{},{},{},{},{}",
            row.device.as_str(),
            row.tier.as_str(),
            row.scope,
            row.aliases_csv(),
            row.primary_lane.as_str(),
            row.fallback_lane.as_str(),
            row.memory_mode.as_str(),
            row.adapters_csv(),
            row.max_parallel_chunks,
            row.kv_prefetch_blocks,
            row.hot_kv_precision_bits,
            row.cold_kv_precision_bits,
            row.allow_disk_spill,
            row.local_kv_token_budget,
            row.global_kv_token_budget,
            row.latency_budget_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            row.passed()
        );

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
    benchmark_gate_enabled: bool,
    benchmark_min_quality: Option<f32>,
    benchmark_min_reward: Option<f32>,
    benchmark_max_total_ms: Option<u128>,
    benchmark_max_recursive_chunks: Option<usize>,
    benchmark_min_recursive_cases: Option<usize>,
    benchmark_max_drift_blocks: Option<usize>,
    benchmark_max_drift_rollbacks: Option<usize>,
    benchmark_roundtrip: bool,
    list_devices: bool,
    device_gate: bool,
    kv_quant_gate: bool,
    kv_quant_max_total_us: Option<u128>,
    inspect_state: bool,
    inspect_limit: usize,
    local_runtime: bool,
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
    fn parse(raw: Vec<String>) -> Self {
        let mut prompt_parts = Vec::new();
        let mut profile = None;
        let mut memory_path = PathBuf::from("noiron-memory.ndkv");
        let mut experience_path = PathBuf::from("noiron-experience.ndkv");
        let mut adaptive_path = PathBuf::from("noiron-adaptive.ndkv");
        let mut trace_path = None;
        let mut trace_schema_gate_path = None;
        let mut benchmark_path = None;
        let mut benchmark_gate_enabled = false;
        let mut benchmark_min_quality = None;
        let mut benchmark_min_reward = None;
        let mut benchmark_max_total_ms = None;
        let mut benchmark_max_recursive_chunks = None;
        let mut benchmark_min_recursive_cases = None;
        let mut benchmark_max_drift_blocks = None;
        let mut benchmark_max_drift_rollbacks = None;
        let mut benchmark_roundtrip = false;
        let mut list_devices = false;
        let mut device_gate = false;
        let mut kv_quant_gate = false;
        let mut kv_quant_max_total_us = None;
        let mut inspect_state = false;
        let mut inspect_limit = 5;
        let mut local_runtime = false;
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
                "--inspect-state" => {
                    inspect_state = true;
                    index += 1;
                }
                "--inspect-limit" if index + 1 < raw.len() => {
                    inspect_limit = parse_usize(&raw[index + 1], inspect_limit).max(1);
                    inspect_state = true;
                    index += 2;
                }
                "--local-runtime" => {
                    local_runtime = true;
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
        let profile = profile.unwrap_or_else(|| detect_profile(&prompt));
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
            benchmark_gate_enabled,
            benchmark_min_quality,
            benchmark_min_reward,
            benchmark_max_total_ms,
            benchmark_max_recursive_chunks,
            benchmark_min_recursive_cases,
            benchmark_max_drift_blocks,
            benchmark_max_drift_rollbacks,
            benchmark_roundtrip,
            list_devices,
            device_gate,
            kv_quant_gate,
            kv_quant_max_total_us,
            inspect_state,
            inspect_limit,
            local_runtime,
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
        if let Some(value) = self.benchmark_max_drift_blocks {
            gate.max_drift_blocks = Some(value);
        }
        if let Some(value) = self.benchmark_max_drift_rollbacks {
            gate.max_drift_rollbacks = Some(value);
        }

        gate
    }

    fn kv_quant_gate(&self) -> KvQuantBenchmarkGate {
        let mut gate = KvQuantBenchmarkGate::default();

        if let Some(value) = self.kv_quant_max_total_us {
            gate.max_total_elapsed_us = Some(value);
        }

        gate
    }
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
    println!(
        "Usage: rust-norion [--profile coding|writing|long|general] [--memory path] [--experience path] [--adaptive path] [--trace path] [--trace-schema-gate path] [--benchmark path] [--benchmark-gate] [--benchmark-roundtrip] [--benchmark-min-quality f] [--benchmark-min-reward f] [--benchmark-max-total-ms n] [--benchmark-max-recursive-chunks n] [--benchmark-min-recursive-cases n] [--benchmark-max-drift-blocks n] [--benchmark-max-drift-rollbacks n] [--list-devices] [--device-gate] [--kv-quant-gate] [--kv-quant-max-total-us n] [--inspect-state] [--inspect-limit n] [--local-runtime] [--runtime-command path] [--runtime-arg arg] [--runtime-prompt-mode stdin|args] [--runtime-wire-format text|json] [--runtime-json] [--runtime-model-id id] [--runtime-tokenizer name] [--runtime-native-window n] [--runtime-embedding-dims n] [--runtime-kv-import] [--runtime-kv-export] [--runtime-kv-exchange] [--native-window n] [--chunk-tokens n] [--chunk-overlap n] [--merge-fan-in n] [--replay n] [--auto-replay n] [--retention-stale-after n] [--retention-decay-rate f] [--retention-remove-below f] [--retention-remove-after-failures n] [--compaction-threshold f] [--compaction-max-candidates n] [--compaction-max-merges n] [--device auto|cpu|integrated|discrete|uma|mobile|embedded|npu|multi-gpu|edge|server] [--cpu-load f] [--gpu-load f] [--ram-load f] [--disk-load f] <prompt>"
    );
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "--benchmark-max-drift-blocks".to_owned(),
            "0".to_owned(),
            "--benchmark-max-drift-rollbacks".to_owned(),
            "0".to_owned(),
            "--benchmark-roundtrip".to_owned(),
            "--list-devices".to_owned(),
            "--device-gate".to_owned(),
            "--kv-quant-max-total-us".to_owned(),
            "100000".to_owned(),
            "--inspect-state".to_owned(),
            "--inspect-limit".to_owned(),
            "2".to_owned(),
            "--local-runtime".to_owned(),
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
        assert_eq!(args.trace_path.unwrap(), PathBuf::from("trace.jsonl"));
        assert_eq!(
            args.trace_schema_gate_path.unwrap(),
            PathBuf::from("trace-schema.jsonl")
        );
        assert_eq!(
            args.benchmark_path.unwrap(),
            PathBuf::from("benchmark.jsonl")
        );
        assert!(args.benchmark_gate_enabled);
        assert_eq!(args.benchmark_min_quality, Some(0.6));
        assert_eq!(args.benchmark_min_reward, Some(0.5));
        assert_eq!(args.benchmark_max_total_ms, Some(10000));
        assert_eq!(args.benchmark_max_recursive_chunks, Some(8));
        assert_eq!(args.benchmark_min_recursive_cases, Some(1));
        assert_eq!(args.benchmark_max_drift_blocks, Some(0));
        assert_eq!(args.benchmark_max_drift_rollbacks, Some(0));
        assert!(args.benchmark_roundtrip);
        assert!(args.list_devices);
        assert!(args.device_gate);
        assert!(args.kv_quant_gate);
        assert_eq!(args.kv_quant_max_total_us, Some(100000));
        assert!(args.inspect_state);
        assert_eq!(args.inspect_limit, 2);
        assert!(args.local_runtime);
        assert_eq!(args.runtime_metadata.model_id, "dev-transformer");
        assert_eq!(args.runtime_metadata.tokenizer, "dev-bpe");
        assert_eq!(args.runtime_wire_format, CommandWireFormat::Json);
        assert_eq!(args.runtime_metadata.native_context_window, 4096);
        assert_eq!(args.runtime_metadata.embedding_dimensions, 128);
        assert!(args.runtime_metadata.supports_kv_import);
        assert!(args.runtime_metadata.supports_kv_export);
        assert_eq!(args.device, DeviceClass::CpuOnly);
        assert_eq!(args.cpu_load, 75.0);
        assert_eq!(args.ram_load, 0.5);
        assert!(args.prompt.contains("nine"));
    }

    #[test]
    fn configure_engine_applies_memory_policy_flags() {
        let args = Args::parse(vec![
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
}
