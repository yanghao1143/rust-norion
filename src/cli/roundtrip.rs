use rust_norion::{
    DeviceClass, ExperienceInput, HierarchyWeights, InferenceRequest, LocalTransformerRuntime,
    NoironEngine, PersistentRoundtripDeviceReport, PersistentRoundtripInput,
    PersistentRoundtripMatrixReport, PersistentRoundtripReport, ProcessRewardComponents,
    ProcessRewardReport, ReflectionIssue, ReflectionSeverity, RewardAction, RouteBudget,
    RuntimeBackend, RuntimeDiagnostics, TaskProfile,
};

use crate::Args;
use crate::engine_config::configure_engine;

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

pub(crate) fn run_persistent_roundtrip(args: &Args) -> std::io::Result<PersistentRoundtripReport> {
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
