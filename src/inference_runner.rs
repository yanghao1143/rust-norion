use std::path::PathBuf;
use std::time::Instant;

use rust_norion::{
    append_self_evolving_memory_writeback_trace_jsonl, append_trace_jsonl,
    append_trace_jsonl_with_case, DraftToken, InferenceBackend, InferenceOutcome, InferenceRequest,
    NoironEngine, RuntimeError, SelfEvolvingEpisodeInput, SelfEvolvingHeuristicInput,
    SelfEvolvingMemoryApproval, SelfEvolvingMemoryMaintenancePolicy, SelfEvolvingMemoryQuery,
    SelfEvolvingMemoryRuntimeWritebackReport, SelfEvolvingMemoryStore, TaskProfile,
    ToolReliabilityObservationInput,
};

use crate::model_service::types::TimedOutcome;
use crate::Args;

pub(crate) fn self_evolving_experience_hints_for_args(
    args: &Args,
    prompt: &str,
    profile: TaskProfile,
) -> std::io::Result<Vec<String>> {
    SelfEvolvingMemoryStore::load_snapshot(
        args.experience_path
            .with_extension("self-evolving-memory.tsv"),
    )
    .map(|store| {
        store
            .retrieve_context(&SelfEvolvingMemoryQuery {
                prompt: prompt.to_owned(),
                profile,
                tags: Vec::new(),
                record_limit: 4,
                token_budget: 160,
            })
            .experience_hints()
    })
}

pub(crate) fn self_evolving_runtime_key_insights(
    profile: TaskProfile,
    outcome: &InferenceOutcome,
) -> Vec<String> {
    vec![
        format!("profile={profile:?}"),
        format!(
            "kv_fusion_saved={}",
            outcome.memory_admission.fusion_plan.saved_tokens
        ),
        format!(
            "reflection_issues={} critical={} revision_actions={}",
            outcome.report.issues.len(),
            outcome.report.critical_issue_count(),
            outcome.report.revision_actions.len()
        ),
        format!(
            "reasoning_genes={} splice_segments={} splice_exons={}",
            outcome.reasoning_genome.active_gene_ids.len(),
            outcome.reasoning_genome_splice.segments.len(),
            outcome.reasoning_genome_splice.exon_count()
        ),
    ]
}

pub(crate) fn record_self_evolving_experience_for_args(
    args: &Args,
    prompt: &str,
    profile: TaskProfile,
    outcome: &InferenceOutcome,
    tool_name: &str,
) -> std::io::Result<SelfEvolvingMemoryRuntimeWritebackReport> {
    let store_path = args
        .experience_path
        .with_extension("self-evolving-memory.tsv");
    let mut store = SelfEvolvingMemoryStore::load_snapshot(&store_path)?;
    let records_before = store.record_count();
    let snapshot_before_digest = store.snapshot_digest();
    let source_case_id = format!("runtime-inference:{tool_name}:{}", outcome.experience_id);
    let tags = vec![
        "runtime".to_owned(),
        "fht-dke".to_owned(),
        "noiron".to_owned(),
        format!("profile:{profile:?}"),
    ];
    let approval = SelfEvolvingMemoryApproval::approved(
        format!("rollback:runtime-inference:{}", outcome.experience_id),
        vec![
            format!("experience_id:{}", outcome.experience_id),
            format!("process_reward:{:.3}", outcome.process_reward.total),
            format!("tool:{tool_name}"),
        ],
    );

    let episode_write = store.append_episode(
        SelfEvolvingEpisodeInput {
            problem: prompt.to_owned(),
            solution_path: format!(
                "used_experiences={} imported_kv_blocks={} adaptive_saved={}",
                outcome.used_experiences.len(),
                outcome.runtime_diagnostics.imported_kv_blocks,
                outcome.adaptive_route_plan.saved_tokens
            ),
            outcome: format!(
                "reward={:.3} stored_runtime_kv={} compute_saved={}",
                outcome.process_reward.total,
                outcome.stored_runtime_kv_memory_ids.len(),
                outcome.compute_budget_schedule.saved_tokens
            ),
            key_insights: self_evolving_runtime_key_insights(profile, outcome),
            tags: tags.clone(),
            profile,
            quality: outcome.report.quality,
            token_estimate: outcome.runtime_token_metrics.token_count.max(1),
            source_case_id: source_case_id.clone(),
        },
        &approval,
    );
    let heuristic_write = store.append_heuristic(
        SelfEvolvingHeuristicInput {
            rule: "reuse positive runtime SEM episodes before spending fresh KV compute".to_owned(),
            tags,
            profile,
            priority: outcome.process_reward.total.clamp(0.0, 1.0),
            confidence: outcome.report.quality.clamp(0.0, 1.0),
            source_case_id: source_case_id.clone(),
            updated_step: outcome.experience_id,
        },
        &approval,
    );
    let tool_write = store.observe_tool(
        ToolReliabilityObservationInput {
            tool_name: tool_name.to_owned(),
            profile,
            success: outcome.process_reward.total > 0.0,
            quality: outcome.report.quality.clamp(0.0, 1.0),
            source_case_id: source_case_id.clone(),
            observed_step: outcome.experience_id,
        },
        &approval,
    );
    let maintenance = store.maintain(&SelfEvolvingMemoryMaintenancePolicy {
        current_step: outcome.experience_id,
        ..SelfEvolvingMemoryMaintenancePolicy::default()
    });
    store.save_snapshot(&store_path)?;
    let disk_snapshot_digest =
        SelfEvolvingMemoryStore::load_snapshot(&store_path)?.snapshot_digest();
    Ok(SelfEvolvingMemoryRuntimeWritebackReport::from_store(
        tool_name,
        profile,
        outcome.experience_id,
        &source_case_id,
        records_before,
        snapshot_before_digest,
        disk_snapshot_digest,
        &store,
        &[episode_write, heuristic_write, tool_write],
        &maintenance,
    ))
}

pub(crate) fn record_self_evolving_experience_trace_for_args(
    args: &Args,
    report: &SelfEvolvingMemoryRuntimeWritebackReport,
) -> std::io::Result<()> {
    if let Some(trace_path) = args.trace_path.as_ref() {
        append_self_evolving_memory_writeback_trace_jsonl(trace_path, report)?;
    }
    if let Some(trace_schema_gate_path) = args.trace_schema_gate_path.as_ref()
        && args.trace_path.as_ref() != Some(trace_schema_gate_path)
    {
        append_self_evolving_memory_writeback_trace_jsonl(trace_schema_gate_path, report)?;
    }
    Ok(())
}

pub(crate) fn persist_self_evolving_writeback_note_for_args(
    engine: &mut NoironEngine,
    args: &Args,
    report: &SelfEvolvingMemoryRuntimeWritebackReport,
) -> std::io::Result<bool> {
    let Some(record) = engine.experience.record_mut(report.experience_id) else {
        return Ok(false);
    };

    let note = self_evolving_writeback_experience_note(report);
    if record.process_reward.notes.iter().any(|item| item == &note) {
        return Ok(false);
    }

    record.process_reward.notes.push(note);
    engine.save_experience(&args.experience_path)?;
    Ok(true)
}

fn self_evolving_writeback_experience_note(
    report: &SelfEvolvingMemoryRuntimeWritebackReport,
) -> String {
    format!(
        "self_evolving_memory_writeback:attempted_records={}:accepted_records={}:records_before={}:records_after={}:tool_reliability_after={}:tool_observations_after={}:maintenance_actions={}:merged_duplicate_episodes={}:write_allowed={}:durable_write_allowed={}:applied={}:applied_to_disk={}:snapshot_changes={}",
        report.attempted_records,
        report.accepted_records,
        report.records_before,
        report.records_after,
        report.tool_reliability_after,
        report.tool_observations_after,
        report.maintenance_actions,
        report.merged_duplicate_episodes,
        report.write_allowed,
        report.durable_write_allowed,
        report.applied,
        report.applied_to_disk,
        usize::from(report.snapshot_before_digest != report.snapshot_digest),
    )
}

pub(crate) fn inference_trace_output_paths_for_args(args: &Args) -> [Option<&PathBuf>; 2] {
    [
        args.trace_path.as_ref(),
        args.trace_schema_gate_path
            .as_ref()
            .filter(|gate_path| args.trace_path.as_ref() != Some(*gate_path)),
    ]
}

pub(crate) fn run_timed_inference<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_options(
        engine, backend, prompt, profile, None, trace_path, case_name,
    )
}

pub(crate) fn run_timed_inference_with_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_external_experience_hints(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        Vec::new(),
        trace_path,
        case_name,
    )
}

pub(crate) fn run_timed_inference_with_external_experience_hints<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    external_experience_hints: Vec<String>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_with_external_experience_hints_to_trace_paths(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        external_experience_hints,
        [trace_path, None],
        case_name,
    )
}

pub(crate) fn run_timed_inference_with_external_experience_hints_to_trace_paths<
    B: InferenceBackend,
>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    external_experience_hints: Vec<String>,
    trace_paths: [Option<&PathBuf>; 2],
    case_name: Option<&str>,
) -> std::io::Result<TimedOutcome> {
    let started = Instant::now();
    let request = InferenceRequest::new(prompt.clone(), profile)
        .with_max_tokens(max_tokens)
        .with_external_experience_hints(external_experience_hints);
    let outcome = engine.infer(request, backend);
    let elapsed_ms = started.elapsed().as_millis();

    append_inference_trace_jsonl_to_paths(
        trace_paths,
        case_name,
        &prompt,
        profile,
        elapsed_ms,
        &outcome,
    )?;

    Ok(TimedOutcome {
        outcome,
        elapsed_ms,
    })
}

#[allow(dead_code)]
pub(crate) fn run_timed_inference_stream<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken),
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_with_options(
        engine, backend, prompt, profile, None, trace_path, case_name, on_token,
    )
}

pub(crate) fn run_timed_inference_stream_with_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken),
) -> std::io::Result<TimedOutcome> {
    let mut checked = |token: &DraftToken| {
        on_token(token);
        Ok(())
    };
    run_timed_inference_stream_checked_with_options(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        trace_path,
        case_name,
        &mut checked,
    )
}

pub(crate) fn run_timed_inference_stream_checked_with_options<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_checked_with_external_experience_hints(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        Vec::new(),
        trace_path,
        case_name,
        on_token,
    )
}

pub(crate) fn run_timed_inference_stream_checked_with_external_experience_hints<
    B: InferenceBackend,
>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    external_experience_hints: Vec<String>,
    trace_path: Option<&PathBuf>,
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
) -> std::io::Result<TimedOutcome> {
    run_timed_inference_stream_checked_with_external_experience_hints_to_trace_paths(
        engine,
        backend,
        prompt,
        profile,
        max_tokens,
        external_experience_hints,
        [trace_path, None],
        case_name,
        on_token,
    )
}

pub(crate) fn run_timed_inference_stream_checked_with_external_experience_hints_to_trace_paths<
    B: InferenceBackend,
>(
    engine: &mut NoironEngine,
    backend: &mut B,
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    external_experience_hints: Vec<String>,
    trace_paths: [Option<&PathBuf>; 2],
    case_name: Option<&str>,
    on_token: &mut dyn FnMut(&DraftToken) -> std::io::Result<()>,
) -> std::io::Result<TimedOutcome> {
    let started = Instant::now();
    let request = InferenceRequest::new(prompt.clone(), profile)
        .with_max_tokens(max_tokens)
        .with_external_experience_hints(external_experience_hints);
    let mut observer_error = None;
    let mut outcome = {
        let mut checked = |token: &DraftToken| match on_token(token) {
            Ok(()) => Ok(()),
            Err(error) => {
                let message = error.to_string();
                observer_error = Some(error);
                Err(RuntimeError::new(format!(
                    "stream observer failed: {message}"
                )))
            }
        };
        engine.infer_stream_checked(request, backend, &mut checked)
    };
    if let Some(error) = observer_error.as_ref() {
        let message = format!("stream observer failed: {error}");
        let timeout = matches!(
            error.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ) || message.to_ascii_lowercase().contains("timed out")
            || message.to_ascii_lowercase().contains("timeout");
        let note = format!(
            "runtime_error:label=runtime_stream_observer_error:timeout={timeout}:message_chars={}",
            message.chars().count()
        );
        if !outcome
            .process_reward
            .notes
            .iter()
            .any(|item| item == &note)
        {
            outcome.process_reward.notes.push(note);
        }
    }
    let elapsed_ms = started.elapsed().as_millis();

    let trace_result = append_inference_trace_jsonl_to_paths(
        trace_paths,
        case_name,
        &prompt,
        profile,
        elapsed_ms,
        &outcome,
    );

    if let Some(error) = observer_error {
        let _ = trace_result;
        return Err(error);
    }
    trace_result?;

    Ok(TimedOutcome {
        outcome,
        elapsed_ms,
    })
}

fn append_inference_trace_jsonl_to_paths(
    trace_paths: [Option<&PathBuf>; 2],
    case_name: Option<&str>,
    prompt: &str,
    profile: TaskProfile,
    elapsed_ms: u128,
    outcome: &InferenceOutcome,
) -> std::io::Result<()> {
    for trace_path in trace_paths.into_iter().flatten() {
        if let Some(case_name) = case_name {
            append_trace_jsonl_with_case(
                trace_path, case_name, prompt, profile, elapsed_ms, outcome,
            )?;
        } else {
            append_trace_jsonl(trace_path, prompt, profile, elapsed_ms, outcome)?;
        }
    }
    Ok(())
}
