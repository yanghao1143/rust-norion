use std::path::PathBuf;

use rust_norion::{
    BenchmarkCase, BenchmarkSummary, DeviceClass, DiskKvStore, ExperienceInput, HierarchyWeights,
    LocalTransformerRuntime, NoironEngine, PersistentRoundtripDeviceReport,
    PersistentRoundtripInput, PersistentRoundtripMatrixReport, PersistentRoundtripReport,
    ProcessRewardComponents, ProcessRewardReport, ReflectionIssue, ReflectionSeverity,
    RewardAction, RouteBudget, RuntimeBackend, RuntimeDiagnostics, TaskProfile, TenantResourceLane,
    TenantScopedKey, append_self_evolution_admission_trace_jsonl, issue30_kvswap_boundary_verified,
    stable_redaction_digest,
};
use rust_norion::{ExperienceMatch, RuntimeAdapterHint};

use crate::Args;
use crate::cli::benchmark::benchmark_self_evolution_admission_report;
use crate::engine_config::configure_engine;
use crate::inference_runner::run_timed_inference_with_options;

const ROUNDTRIP_FIRST_CASE: &str = "issue30_roundtrip_first";
const ROUNDTRIP_SECOND_CASE: &str = "issue30_roundtrip_second";

fn issue30_approved_experience_reuse_digest(experience: &ExperienceMatch) -> String {
    let id = experience.id.to_string();
    let quality = format!("{:.3}", experience.quality);
    let score = format!("{:.3}", experience.score);
    let used_memory_count = experience.used_memory_count.to_string();
    let runtime_kv_count = experience.stored_runtime_kv_memory_ids.len().to_string();
    let adapter = experience
        .runtime_selected_adapter
        .as_deref()
        .unwrap_or("none");
    let reward_action = experience.reward_action.as_str();

    stable_redaction_digest([
        "issue-30-approved-experience-reuse",
        id.as_str(),
        quality.as_str(),
        score.as_str(),
        used_memory_count.as_str(),
        runtime_kv_count.as_str(),
        adapter,
        reward_action,
    ])
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
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.5,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Hold,
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });
}

fn is_runtime_kv_memory_key(key: &str) -> bool {
    key.starts_with("runtime_kv:")
        || TenantScopedKey::parse(key)
            .is_some_and(|scoped| scoped.lane == TenantResourceLane::RuntimeKv)
}

pub(crate) fn run_persistent_roundtrip(args: &Args) -> std::io::Result<PersistentRoundtripReport> {
    let trace_output_path = roundtrip_trace_output_path(args);
    let mut benchmark_summary = BenchmarkSummary::new();
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
    let first_timed = run_timed_inference_with_options(
        &mut first_engine,
        &mut first_backend,
        args.prompt.clone(),
        args.profile,
        None,
        trace_output_path,
        Some(ROUNDTRIP_FIRST_CASE),
    );
    let first_timed = first_timed?;
    let first = first_timed.outcome;
    benchmark_summary.record(
        &roundtrip_benchmark_case(ROUNDTRIP_FIRST_CASE, args),
        first_timed.elapsed_ms,
        &first,
    );
    let first_runtime_kv_memory_ids = first.stored_runtime_kv_memory_ids.clone();
    let first_runtime_kv_namespace_preserved = !first_runtime_kv_memory_ids.is_empty()
        && first_runtime_kv_memory_ids.iter().all(|id| {
            first_engine
                .cache
                .entries()
                .iter()
                .any(|entry| entry.id == *id && is_runtime_kv_memory_key(&entry.key))
        });
    first_engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let first_disk_kv_reopen_verified = [
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    ]
    .iter()
    .all(|path| disk_kv_reopens_read_only(path));

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
                .find(|entry| entry.id == *id && is_runtime_kv_memory_key(&entry.key))
                .map(|entry| entry.vector.clone())
        })
        .collect::<Vec<_>>();
    let second_runtime_kv_disk_rehydrated = !first_runtime_kv_memory_ids.is_empty()
        && restored_runtime_kv_vectors.len() == first_runtime_kv_memory_ids.len();
    let mut second_backend = RuntimeBackend::new(LocalTransformerRuntime::with_manifest(
        args.runtime_manifest(),
    ));
    let second_timed = run_timed_inference_with_options(
        &mut second_engine,
        &mut second_backend,
        args.prompt.clone(),
        args.profile,
        None,
        trace_output_path,
        Some(ROUNDTRIP_SECOND_CASE),
    );
    let second_timed = second_timed?;
    let second = second_timed.outcome;
    benchmark_summary.record(
        &roundtrip_benchmark_case(ROUNDTRIP_SECOND_CASE, args),
        second_timed.elapsed_ms,
        &second,
    );
    let second_used_runtime_kv_memory = second.used_memories.iter().any(|memory| {
        first_runtime_kv_memory_ids.contains(&memory.id) && is_runtime_kv_memory_key(&memory.key)
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
    if let Some(path) = trace_output_path {
        let gate_report = benchmark_summary.evaluate(&args.benchmark_gate());
        let admission = benchmark_self_evolution_admission_report(
            format!("roundtrip:{}", path.display()),
            &second_engine,
            &benchmark_summary,
            &gate_report,
            args.profile,
        );
        append_self_evolution_admission_trace_jsonl(path, &admission)?;
    }

    Ok(PersistentRoundtripReport::evaluate(
        PersistentRoundtripInput {
            first_stored_memory: first.stored_memory_id.is_some(),
            first_runtime_kv_stored: first.stored_runtime_kv_memory_ids.len(),
            first_runtime_kv_namespace_preserved,
            first_disk_kv_reopen_verified,
            second_used_memories: second.used_memories.len(),
            second_used_runtime_kv_memory,
            second_used_experiences: second.used_experiences.len(),
            second_approved_experience_reuse_digest: second
                .used_experiences
                .first()
                .map(issue30_approved_experience_reuse_digest),
            second_imported_runtime_kv_blocks,
            second_imported_runtime_kv_from_namespace,
            second_runtime_kv_disk_rehydrated,
            second_kvswap_boundary_verified: issue30_kvswap_boundary_verified(),
            second_runtime_adapter_observations: second.runtime_adapter_observations.len(),
            second_runtime_adapter_best_score: second
                .runtime_adapter_observations
                .first()
                .map(|observation| observation.score),
            second_runtime_adapter_best_adapter: second
                .runtime_adapter_observations
                .first()
                .map(|observation| observation.adapter.as_str().to_owned()),
            second_runtime_selected_adapter: second
                .runtime_diagnostics
                .selected_adapter
                .as_deref()
                .and_then(RuntimeAdapterHint::canonical_name)
                .map(str::to_owned),
            second_compute_budget_saved_tokens: second.compute_budget_schedule.saved_tokens,
            second_compute_budget_avoided_tokens: second
                .compute_budget_schedule
                .wasted_compute_avoided_tokens,
            second_compute_budget_kv_lookups_skipped: second
                .compute_budget_schedule
                .kv_lookups_skipped,
            second_compute_budget_anchor_count: second.compute_budget_schedule.anchor_count,
            second_compute_budget_anchors_preserved: second
                .compute_budget_schedule
                .anchors_preserved(),
            second_compute_budget_anchors_preserved_count: second
                .compute_budget_schedule
                .anchors_preserved,
            second_quality: second.report.quality,
            first_drift_severity: first.drift_report.severity,
            second_drift_severity: second.drift_report.severity,
        },
    ))
}

fn disk_kv_reopens_read_only(path: &PathBuf) -> bool {
    DiskKvStore::open_read_only_existing(path)
        .ok()
        .flatten()
        .is_some_and(|store| !store.is_empty())
}

fn roundtrip_trace_output_path(args: &Args) -> Option<&PathBuf> {
    args.trace_path
        .as_ref()
        .or(args.trace_schema_gate_path.as_ref())
}

fn roundtrip_benchmark_case(name: &str, args: &Args) -> BenchmarkCase {
    BenchmarkCase::new(name, args.profile, args.prompt.clone())
}

pub(crate) fn run_persistent_roundtrip_all_devices(
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

pub(crate) fn print_persistent_roundtrip_report(args: &Args, report: &PersistentRoundtripReport) {
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

pub(crate) fn print_persistent_roundtrip_matrix_report(
    args: &Args,
    report: &PersistentRoundtripMatrixReport,
) {
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
