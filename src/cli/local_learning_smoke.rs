use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rust_norion::{
    append_trace_jsonl_with_case, evaluate_trace_schema_jsonl, EvolutionLedger,
    ExperienceRuntimeTokenMetrics, InferenceOutcome, LiveInferenceEvolution,
    LocalTransformerRuntime, MemoryConsolidationEvidenceClass, MemoryConsolidationRecord,
    NoironEngine, RewardAction, RuntimeBackend, SelfEvolvingEpisodeInput,
    SelfEvolvingHeuristicInput, SelfEvolvingMemoryApproval, SelfEvolvingMemoryConsolidationPolicy,
    SelfEvolvingMemoryConsolidationWorker, SelfEvolvingMemoryMaintenancePolicy,
    SelfEvolvingMemoryQuery, SelfEvolvingMemoryRuntimeWritebackReport, SelfEvolvingMemoryStore,
    ToolReliabilityObservationInput, TraceSchemaGateReport,
};

use crate::cli::state::ensure_runtime_state_write_window_clean;
use crate::cli::trace_schema::print_trace_schema_gate_report;
use crate::engine_config::configure_engine;
use crate::inference_runner::{
    run_timed_inference_with_external_experience_hints, self_evolving_runtime_key_insights,
};
use crate::model_service::types::TimedOutcome;
use crate::Args;

const LOCAL_LEARNING_SMOKE_CASE: &str = "local_learning_smoke";
const DEFAULT_LOCAL_LEARNING_SMOKE_MAX_TOKENS: usize = 32;

pub(crate) fn run_local_learning_smoke_cli(args: &Args) -> io::Result<bool> {
    ensure_runtime_state_write_window_clean(args)?;
    let mut engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    configure_engine(&mut engine, args);
    let ledger_before = engine.evolution_ledger;
    let replay_expected = !engine.experience.is_empty();
    let replay_live_evolution_expected = engine
        .experience
        .records()
        .iter()
        .any(|record| record.live_evolution.has_evidence());
    let replay_live_reward_expected = engine.experience.records().iter().any(|record| {
        record.live_evolution.online_reward_feedbacks > 0
            || record.live_evolution.online_reward_strength > 0.000001
    });
    let runtime_kv_available_before = engine
        .experience
        .records()
        .iter()
        .any(|record| !record.stored_runtime_kv_memory_ids.is_empty());
    let runtime_kv_import_expected = replay_expected
        && runtime_kv_available_before
        && args.runtime_metadata.supports_kv_import
        && args.runtime_metadata.supports_kv_export;

    let runtime = LocalTransformerRuntime::with_manifest(args.runtime_manifest());
    let max_tokens = args
        .max_tokens
        .unwrap_or(DEFAULT_LOCAL_LEARNING_SMOKE_MAX_TOKENS);
    let mut backend = RuntimeBackend::new(runtime).with_max_tokens(max_tokens);
    let trace_output_path = local_learning_smoke_trace_output_path(args);
    let external_experience_hints = local_learning_self_evolving_experience_hints(args)?;
    let external_experience_hint_count = external_experience_hints.len();
    let timed = run_timed_inference_with_external_experience_hints(
        &mut engine,
        &mut backend,
        args.prompt.clone(),
        args.profile,
        Some(max_tokens),
        external_experience_hints,
        trace_output_path,
        Some(LOCAL_LEARNING_SMOKE_CASE),
    )?;
    if let Some(path) = local_learning_smoke_separate_trace_gate_path(args) {
        append_trace_jsonl_with_case(
            path,
            LOCAL_LEARNING_SMOKE_CASE,
            &args.prompt,
            args.profile,
            timed.elapsed_ms,
            &timed.outcome,
        )?;
    }

    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let persisted_state_matches = local_learning_smoke_persisted_state_matches(
        args,
        &timed.outcome,
        replay_expected,
        runtime_kv_import_expected,
    );
    record_local_learning_self_evolving_memory(trace_output_path, args, &timed.outcome)?;

    let trace_gate_report = if let Some(path) = &args.trace_schema_gate_path {
        let report = evaluate_trace_schema_jsonl(path)?;
        print_trace_schema_gate_report(path, &report);
        Some(report)
    } else {
        None
    };
    let ledger_delta_matches = live_evolution_matches_ledger_delta(ledger_before, &timed.outcome);
    let passed = local_learning_smoke_passed(
        &timed,
        trace_gate_report.as_ref(),
        ledger_delta_matches,
        persisted_state_matches,
        replay_expected,
        replay_live_evolution_expected,
        replay_live_reward_expected,
        runtime_kv_import_expected,
        external_experience_hint_count,
    );
    print_local_learning_smoke_summary(
        args,
        &timed,
        trace_gate_report.as_ref(),
        ledger_delta_matches,
        persisted_state_matches,
        replay_expected,
        replay_live_evolution_expected,
        replay_live_reward_expected,
        runtime_kv_import_expected,
        external_experience_hint_count,
        passed,
    );
    Ok(passed)
}

fn local_learning_smoke_trace_output_path(args: &Args) -> Option<&PathBuf> {
    args.trace_path
        .as_ref()
        .or(args.trace_schema_gate_path.as_ref())
}

fn local_learning_smoke_separate_trace_gate_path(args: &Args) -> Option<&PathBuf> {
    match (&args.trace_path, &args.trace_schema_gate_path) {
        (Some(trace_path), Some(trace_schema_gate_path))
            if trace_path != trace_schema_gate_path =>
        {
            Some(trace_schema_gate_path)
        }
        _ => None,
    }
}

fn local_learning_smoke_passed(
    timed: &TimedOutcome,
    trace_gate_report: Option<&TraceSchemaGateReport>,
    ledger_delta_matches: bool,
    persisted_state_matches: bool,
    replay_expected: bool,
    replay_live_evolution_expected: bool,
    replay_live_reward_expected: bool,
    runtime_kv_import_expected: bool,
    external_experience_hint_count: usize,
) -> bool {
    let compute_budget_saved = timed.outcome.compute_budget_schedule.saved_tokens;
    let compute_budget_sem_fusion_saved = timed
        .outcome
        .compute_budget_schedule
        .self_evolving_memory_fusion_saved_tokens;
    let compute_budget_avoided = timed
        .outcome
        .compute_budget_schedule
        .wasted_compute_avoided_tokens;
    let compute_budget_threshold_delta_milli =
        local_learning_smoke_milli(timed.outcome.compute_budget_schedule.threshold_delta);
    let kv_fusion_saved = timed.outcome.memory_admission.fusion_plan.saved_tokens;
    let adaptive_routing_saved = timed.outcome.adaptive_route_plan.saved_tokens;
    let task_hierarchy_mutations = timed.outcome.task_hierarchy_plan.mutation_count();
    let task_hierarchy_route_pressure = timed.outcome.task_hierarchy_plan.route_pressure;
    let task_hierarchy_compute_reduction = timed.outcome.task_hierarchy_plan.compute_reduction;
    let process_reward_total = timed.outcome.process_reward.total;
    let reasoning_active_genes = timed.outcome.reasoning_genome.active_gene_ids.len();
    let reasoning_splice_segments = timed.outcome.reasoning_genome_splice.segments.len();
    let reasoning_splice_exons = timed.outcome.reasoning_genome_splice.exon_count();
    let live_router_threshold_delta_milli =
        local_learning_smoke_milli(timed.outcome.live_evolution.router_threshold_delta);
    let live_hierarchy_weight_delta_milli =
        local_learning_smoke_milli(timed.outcome.live_evolution.hierarchy_weight_delta);
    let expected_runtime_experience_hints = timed
        .outcome
        .used_experiences
        .len()
        .saturating_add(external_experience_hint_count);
    timed.outcome.runtime_token_metrics.token_count > 0
        && has_fht_dke_budget_evidence(&timed.outcome)
        && timed.outcome.experience_id > 0
        && timed.outcome.raw_answer.contains(&format!(
            "{expected_runtime_experience_hints} experience hints"
        ))
        && process_reward_total > 0.0
        && timed.outcome.process_reward.action == RewardAction::Reinforce
        && timed.outcome.live_evolution.online_reward_feedbacks > 0
        && live_router_threshold_delta_milli > 0
        && live_hierarchy_weight_delta_milli > 0
        && reasoning_active_genes > 0
        && reasoning_splice_segments > 0
        && reasoning_splice_exons > 0
        && adaptive_routing_saved > 0
        && task_hierarchy_mutations > 0
        && task_hierarchy_route_pressure > 0.0
        && task_hierarchy_compute_reduction > 0.0
        && compute_budget_saved > 0
        && compute_budget_avoided > 0
        && compute_budget_threshold_delta_milli > 0
        && kv_fusion_saved > 0
        && ledger_delta_matches
        && persisted_state_matches
        && (!replay_expected
            || timed
                .outcome
                .auto_replay_report
                .as_ref()
                .is_some_and(|report| report.applied > 0)
                && !timed.outcome.used_experiences.is_empty())
        && (!replay_live_evolution_expected
            || timed
                .outcome
                .auto_replay_report
                .as_ref()
                .is_some_and(|report| report.live_evolution_items > 0))
        && (!replay_live_reward_expected
            || timed
                .outcome
                .auto_replay_report
                .as_ref()
                .is_some_and(|report| {
                    report.live_evolution_online_reward_feedbacks > 0
                        && report.live_evolution_online_reward_strength > 0.0
                }))
        && (!runtime_kv_import_expected || timed.outcome.runtime_diagnostics.imported_kv_blocks > 0)
        && trace_gate_report
            .map(|report| {
                report.passed
                    && report.checked_lines > 0
                    && (!replay_expected
                        || report.used_experiences >= timed.outcome.used_experiences.len())
                    && (!runtime_kv_import_expected
                        || report.imported_kv_blocks
                            >= timed.outcome.runtime_diagnostics.imported_kv_blocks)
                    && report.adaptive_routing_saved_tokens >= adaptive_routing_saved
                    && report.task_hierarchy_events > 0
                    && report.task_hierarchy_mutation_records >= task_hierarchy_mutations
                    && report.task_hierarchy_route_pressure_milli > 0
                    && report.task_hierarchy_compute_reduction_milli > 0
                    && report.fht_dke_events > 0
                    && report.fht_dke_enabled > 0
                    && report.fht_dke_total_tokens >= timed.outcome.fht_dke_budget.total_tokens
                    && report.fht_dke_routed_tokens >= timed.outcome.fht_dke_budget.routed_tokens
                    && report.fht_dke_token_split_invalid == 0
                    && report.compute_budget_saved_tokens >= compute_budget_saved
                    && report.compute_budget_self_evolving_memory_fusion_saved_tokens
                        >= compute_budget_sem_fusion_saved
                    && report.compute_budget_avoided_tokens >= compute_budget_avoided
                    && report.compute_budget_threshold_delta_milli
                        >= compute_budget_threshold_delta_milli
                    && report.kv_fusion_saved_tokens >= kv_fusion_saved
                    && report.process_reward_events > 0
                    && report.process_reward_positive > 0
                    && report.process_reward_reinforce > 0
                    && report.process_reward_total_milli > 0
                    && report.live_evolution_events > 0
                    && report.live_router_threshold_delta_milli >= live_router_threshold_delta_milli
                    && report.live_hierarchy_weight_delta_milli >= live_hierarchy_weight_delta_milli
                    && report.live_online_reward_feedbacks
                        >= timed.outcome.live_evolution.online_reward_feedbacks
                    && report.live_stored_memory_updates
                        >= timed.outcome.live_evolution.stored_memory_updates()
                    && report.live_memory_updates >= timed.outcome.live_evolution.memory_updates()
                    && report.reasoning_genome_events > 0
                    && report.reasoning_genome_active_genes >= reasoning_active_genes
                    && report.reasoning_genome_splice_segments >= reasoning_splice_segments
                    && report.reasoning_genome_splice_exons >= reasoning_splice_exons
                    && report.self_evolving_memory_store_events >= 4
                    && report.self_evolving_memory_store_retrieval_events > 0
                    && report.self_evolving_memory_store_maintenance_events > 0
                    && report.self_evolving_memory_store_admission_preview_events > 0
                    && report.self_evolving_memory_store_consolidation_events > 0
                    && report.self_evolving_memory_store_consolidation_actions > 0
                    && report.self_evolving_memory_store_merge_previews > 0
                    && report.self_evolving_memory_store_decay_previews > 0
                    && report.self_evolving_memory_store_tombstone_previews > 0
                    && report.self_evolving_memory_store_contexts > 0
                    && report.self_evolving_memory_store_saved_tokens > 0
                    && report.self_evolving_memory_store_maintenance_actions > 0
                    && report.self_evolving_memory_store_admission_candidates > 0
                    && report.self_evolving_memory_store_write_allowed == 0
                    && report.self_evolving_memory_store_durable_write_allowed == 0
                    && report.self_evolving_memory_store_applied == 0
                    && report.self_evolving_memory_store_applied_to_disk == 0
                    && report.self_evolving_memory_writeback_events > 0
                    && report.self_evolving_memory_writeback_attempted_records >= 3
                    && report.self_evolving_memory_writeback_accepted_records >= 3
                    && report.self_evolving_memory_writeback_records_after >= 4
                    && report.self_evolving_memory_writeback_write_allowed > 0
                    && report.self_evolving_memory_writeback_durable_write_allowed > 0
                    && report.self_evolving_memory_writeback_applied > 0
                    && report.self_evolving_memory_writeback_applied_to_disk > 0
            })
            .unwrap_or(true)
}

fn has_fht_dke_budget_evidence(outcome: &InferenceOutcome) -> bool {
    outcome.fht_dke_budget.enabled
        && outcome.fht_dke_budget.total_tokens > 0
        && outcome.fht_dke_budget.routed_tokens > 0
        && outcome.fht_dke_budget.token_split_is_valid
        && outcome
            .process_reward
            .notes
            .iter()
            .any(|note| note.starts_with("fht_dke_budget:"))
}

fn local_learning_smoke_milli(value: f32) -> usize {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as usize
    } else {
        0
    }
}

fn local_learning_self_evolving_experience_hints(args: &Args) -> io::Result<Vec<String>> {
    let store_path = local_learning_self_evolving_memory_store_path(&args.experience_path);
    let store = SelfEvolvingMemoryStore::load_snapshot(&store_path)?;
    Ok(store
        .retrieve_context(&SelfEvolvingMemoryQuery {
            prompt: args.prompt.clone(),
            profile: args.profile,
            tags: local_learning_self_evolving_tags(),
            record_limit: 4,
            token_budget: 160,
        })
        .experience_hints())
}

fn record_local_learning_self_evolving_memory(
    trace_path: Option<&PathBuf>,
    args: &Args,
    outcome: &InferenceOutcome,
) -> io::Result<()> {
    let store_path = local_learning_self_evolving_memory_store_path(&args.experience_path);
    let mut store = SelfEvolvingMemoryStore::load_snapshot(&store_path)?;
    let records_before = store.record_count();
    let snapshot_before_digest = store.snapshot_digest();
    let source_case_id = format!("{LOCAL_LEARNING_SMOKE_CASE}:{}", outcome.experience_id);
    let tags = local_learning_self_evolving_tags();
    let approval = SelfEvolvingMemoryApproval::approved(
        format!(
            "rollback:{LOCAL_LEARNING_SMOKE_CASE}:{}",
            outcome.experience_id
        ),
        vec![
            format!("experience_id:{}", outcome.experience_id),
            format!("process_reward:{:.3}", outcome.process_reward.total),
            format!(
                "live_online_reward_feedbacks:{}",
                outcome.live_evolution.online_reward_feedbacks
            ),
        ],
    );

    let episode_write = store.append_episode(
        SelfEvolvingEpisodeInput {
            problem: args.prompt.clone(),
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
            key_insights: self_evolving_runtime_key_insights(args.profile, outcome),
            tags: tags.clone(),
            profile: args.profile,
            quality: outcome.report.quality,
            token_estimate: outcome.runtime_token_metrics.token_count.max(1),
            source_case_id: source_case_id.clone(),
        },
        &approval,
    );
    let heuristic_write = store.append_heuristic(
        SelfEvolvingHeuristicInput {
            rule: "reuse positive local-learning episodes before spending fresh KV compute"
                .to_owned(),
            tags: tags.clone(),
            profile: args.profile,
            priority: outcome.process_reward.total.clamp(0.0, 1.0),
            confidence: outcome.report.quality.clamp(0.0, 1.0),
            source_case_id: source_case_id.clone(),
            updated_step: outcome.experience_id,
        },
        &approval,
    );
    let tool_write = store.observe_tool(
        ToolReliabilityObservationInput {
            tool_name: "local_transformer_runtime".to_owned(),
            profile: args.profile,
            success: outcome.process_reward.total > 0.0,
            quality: outcome.report.quality.clamp(0.0, 1.0),
            source_case_id: source_case_id.clone(),
            observed_step: outcome.experience_id,
        },
        &approval,
    );
    let writes = [episode_write, heuristic_write, tool_write];

    let retrieval = store.retrieve_context(&SelfEvolvingMemoryQuery {
        prompt: args.prompt.clone(),
        profile: args.profile,
        tags,
        record_limit: 4,
        token_budget: 32,
    });
    let maintenance = store.maintain(&SelfEvolvingMemoryMaintenancePolicy {
        current_step: outcome.experience_id.saturating_add(8),
        stale_after_steps: 1,
        heuristic_decay: 0.90,
        tool_reliability_decay: 0.95,
        quarantine_below_confidence: 0.05,
        merge_duplicate_episodes: true,
    });
    let admission = store.preview_from_memory_admission(&outcome.memory_admission);
    let consolidation =
        SelfEvolvingMemoryConsolidationWorker::new(SelfEvolvingMemoryConsolidationPolicy {
            current_step: outcome.experience_id.saturating_add(8),
            stale_after_steps: 1,
            ..SelfEvolvingMemoryConsolidationPolicy::default()
        })
        .plan(&local_learning_consolidation_records(&store, args, outcome));

    store.save_snapshot(&store_path)?;
    let disk_snapshot_digest =
        SelfEvolvingMemoryStore::load_snapshot(&store_path)?.snapshot_digest();
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport::from_store(
        LOCAL_LEARNING_SMOKE_CASE,
        args.profile,
        outcome.experience_id,
        &source_case_id,
        records_before,
        snapshot_before_digest,
        disk_snapshot_digest,
        &store,
        &writes,
        &maintenance,
    );

    let trace_lines = [
        retrieval.json_line(),
        maintenance.json_line(),
        admission.json_line(),
        consolidation.json_line(),
        writeback.json_line(),
    ];
    if let Some(path) = trace_path {
        append_self_evolving_memory_trace_lines(path, &trace_lines)?;
    }
    if let Some(path) = local_learning_smoke_separate_trace_gate_path(args) {
        append_self_evolving_memory_trace_lines(path, &trace_lines)?;
    }
    Ok(())
}

fn local_learning_self_evolving_memory_store_path(experience_path: &Path) -> PathBuf {
    experience_path.with_extension("self-evolving-memory.tsv")
}

fn local_learning_self_evolving_tags() -> Vec<String> {
    vec![
        LOCAL_LEARNING_SMOKE_CASE.to_owned(),
        "fht-dke".to_owned(),
        "noiron".to_owned(),
        "runtime".to_owned(),
    ]
}

fn local_learning_consolidation_records(
    store: &SelfEvolvingMemoryStore,
    args: &Args,
    outcome: &InferenceOutcome,
) -> Vec<MemoryConsolidationRecord> {
    let mut records = store.consolidation_snapshot("tenant:local-learning", outcome.experience_id);
    let tenant = "tenant:local-learning";
    let source = format!("local-learning:{}", outcome.experience_id);
    let content = format!("{}:{:.3}", args.prompt, outcome.process_reward.total);
    let duplicate_tokens = outcome.runtime_token_metrics.token_count.max(1).min(512);
    records.extend([
        MemoryConsolidationRecord::new(
            format!("episode:{}:primary", outcome.experience_id),
            tenant,
            MemoryConsolidationEvidenceClass::RetrospectiveEpisode,
            source.clone(),
            content.clone(),
            args.profile,
        )
        .with_scores(
            outcome.process_reward.total.clamp(0.0, 1.0),
            outcome.report.quality.clamp(0.0, 1.0),
        )
        .with_last_touched_step(outcome.experience_id)
        .with_token_estimate(duplicate_tokens)
        .with_validation_evidence_count(3),
        MemoryConsolidationRecord::new(
            format!("episode:{}:duplicate", outcome.experience_id),
            tenant,
            MemoryConsolidationEvidenceClass::RetrospectiveEpisode,
            source,
            content,
            args.profile,
        )
        .with_scores(
            (outcome.process_reward.total * 0.95).clamp(0.0, 1.0),
            (outcome.report.quality * 0.95).clamp(0.0, 1.0),
        )
        .with_last_touched_step(outcome.experience_id)
        .with_token_estimate(duplicate_tokens)
        .with_validation_evidence_count(3),
        MemoryConsolidationRecord::new(
            format!("heuristic:{}:stale-low-quality", outcome.experience_id),
            tenant,
            MemoryConsolidationEvidenceClass::ProceduralHeuristic,
            format!("local-learning-stale:{}", outcome.experience_id),
            "reuse-local-learning-low-confidence",
            args.profile,
        )
        .with_scores(0.04, 0.04)
        .with_last_touched_step(0)
        .with_token_estimate(1)
        .with_validation_evidence_count(1),
    ]);
    records
}

fn append_self_evolving_memory_trace_lines(path: &PathBuf, lines: &[String]) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    Ok(())
}

fn live_evolution_matches_ledger_delta(
    before: EvolutionLedger,
    outcome: &InferenceOutcome,
) -> bool {
    let mut expected = before;
    if let Some(report) = outcome.auto_replay_report.as_ref() {
        expected.record_replay(report);
    }
    expected.record_live_inference(outcome.live_evolution);
    expected == outcome.evolution_ledger
}

fn local_learning_smoke_persisted_state_matches(
    args: &Args,
    outcome: &InferenceOutcome,
    replay_expected: bool,
    runtime_kv_import_expected: bool,
) -> bool {
    let Ok(saved) = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    ) else {
        return false;
    };
    let Some(record) = saved
        .experience
        .records()
        .iter()
        .find(|record| record.id == outcome.experience_id)
    else {
        return false;
    };
    let cache_entries = saved.cache.entries();
    let stored_memory_matches = outcome
        .stored_memory_id
        .map(|id| cache_entries.iter().any(|entry| entry.id == id))
        .unwrap_or(true);
    let stored_runtime_kv_matches = outcome.stored_runtime_kv_memory_ids.iter().all(|id| {
        cache_entries
            .iter()
            .any(|entry| entry.id == *id && entry.key.starts_with("runtime_kv:"))
    });
    let replay_matches = !replay_expected
        || saved.evolution_ledger.replay_runs == outcome.evolution_ledger.replay_runs;
    let runtime_kv_import_matches =
        !runtime_kv_import_expected || record.runtime_diagnostics.imported_kv_blocks > 0;
    let runtime_token_metrics_matches = runtime_token_metrics_match(
        record.runtime_token_metrics,
        ExperienceRuntimeTokenMetrics::from(outcome.runtime_token_metrics),
    );
    let process_reward_matches = record.process_reward.action == outcome.process_reward.action
        && close_f32(record.process_reward.total, outcome.process_reward.total);
    let adaptive = saved.adaptive_state();

    record.profile == args.profile
        && record.stored_memory_id == outcome.stored_memory_id
        && record.stored_runtime_kv_memory_ids == outcome.stored_runtime_kv_memory_ids
        && record.gist_memory_ids == outcome.stored_gist_memory_ids
        && runtime_token_metrics_matches
        && record.runtime_diagnostics.imported_kv_blocks
            == outcome.runtime_diagnostics.imported_kv_blocks
        && process_reward_matches
        && live_evolution_match(record.live_evolution, outcome.live_evolution)
        && stored_memory_matches
        && stored_runtime_kv_matches
        && replay_matches
        && runtime_kv_import_matches
        && adaptive.router.observations >= outcome.evolution_ledger.live_inference_runs
        && adaptive.router.profile_observations.get(args.profile) > 0
        && adaptive.hierarchy.profile_observations.get(args.profile) > 0
        && close_f32(adaptive.router.threshold, record.router_threshold_after)
        && hierarchy_matches(adaptive.hierarchy.current, record.hierarchy)
        && adaptive.evolution_ledger.live_inference_runs
            == outcome.evolution_ledger.live_inference_runs
        && adaptive.evolution_ledger.live_stored_runtime_kv_memories
            == outcome.evolution_ledger.live_stored_runtime_kv_memories
}

fn hierarchy_matches(
    left: rust_norion::HierarchyWeights,
    right: rust_norion::HierarchyWeights,
) -> bool {
    close_f32(left.global, right.global)
        && close_f32(left.local, right.local)
        && close_f32(left.convolution, right.convolution)
}

fn close_f32(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.0001
}

fn close_optional_f32(left: Option<f32>, right: Option<f32>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => close_f32(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn runtime_token_metrics_match(
    left: ExperienceRuntimeTokenMetrics,
    right: ExperienceRuntimeTokenMetrics,
) -> bool {
    left.token_count == right.token_count
        && left.entropy_count == right.entropy_count
        && left.logprob_count == right.logprob_count
        && close_optional_f32(left.average_entropy, right.average_entropy)
        && close_optional_f32(left.average_neg_logprob, right.average_neg_logprob)
        && close_optional_f32(left.uncertainty_perplexity, right.uncertainty_perplexity)
}

fn live_evolution_match(left: LiveInferenceEvolution, right: LiveInferenceEvolution) -> bool {
    close_f32(left.router_threshold_delta, right.router_threshold_delta)
        && close_f32(left.hierarchy_weight_delta, right.hierarchy_weight_delta)
        && left.online_reward_feedbacks == right.online_reward_feedbacks
        && left.online_reward_reinforcements == right.online_reward_reinforcements
        && left.online_reward_penalties == right.online_reward_penalties
        && close_f32(left.online_reward_strength, right.online_reward_strength)
        && close_f32(
            left.online_reward_reinforcement_strength,
            right.online_reward_reinforcement_strength,
        )
        && close_f32(
            left.online_reward_penalty_strength,
            right.online_reward_penalty_strength,
        )
        && left.memory_reinforcements == right.memory_reinforcements
        && left.memory_penalties == right.memory_penalties
        && left.stored_memory == right.stored_memory
        && left.stored_gist_memories == right.stored_gist_memories
        && left.stored_runtime_kv_memories == right.stored_runtime_kv_memories
        && left.reflection_issues == right.reflection_issues
        && left.critical_reflection_issues == right.critical_reflection_issues
        && left.revision_actions == right.revision_actions
}

fn print_local_learning_smoke_summary(
    args: &Args,
    timed: &TimedOutcome,
    trace_gate_report: Option<&TraceSchemaGateReport>,
    ledger_delta_matches: bool,
    persisted_state_matches: bool,
    replay_expected: bool,
    replay_live_evolution_expected: bool,
    replay_live_reward_expected: bool,
    runtime_kv_import_expected: bool,
    external_experience_hint_count: usize,
    passed: bool,
) {
    let outcome = &timed.outcome;
    println!("Noiron local learning smoke");
    println!(
        "local_learning_smoke: passed={} elapsed_ms={} trace_gate_passed={} ledger_delta_match={} persisted_state_match={} replay_expected={} replay_live_evolution_expected={} replay_live_reward_expected={} replay_applied={} experience_reused={} runtime_kv_import_expected={} sem_runtime_hints={}",
        passed,
        timed.elapsed_ms,
        trace_gate_report
            .map(|report| report.passed)
            .unwrap_or(true),
        ledger_delta_matches,
        persisted_state_matches,
        replay_expected,
        replay_live_evolution_expected,
        replay_live_reward_expected,
        outcome
            .auto_replay_report
            .as_ref()
            .map(|report| report.applied)
            .unwrap_or(0),
        !outcome.used_experiences.is_empty(),
        runtime_kv_import_expected,
        external_experience_hint_count
    );
    println!("memory_file: {}", args.memory_path.display());
    println!("experience_file: {}", args.experience_path.display());
    println!("adaptive_file: {}", args.adaptive_path.display());
    println!(
        "self_evolving_memory_file: {}",
        local_learning_self_evolving_memory_store_path(&args.experience_path).display()
    );
    if let Some(path) = local_learning_smoke_trace_output_path(args) {
        println!("trace_file: {}", path.display());
    }
    println!(
        "runtime: tokens={} imported_kv={} exported_kv={} stored_runtime_kv={}",
        outcome.runtime_token_metrics.token_count,
        outcome.runtime_diagnostics.imported_kv_blocks,
        outcome.exported_runtime_kv_blocks,
        outcome.stored_runtime_kv_memory_ids.len()
    );
    println!(
        "memory: stored={} gist_stored={} experience_id={} used_memories={} used_experiences={}",
        outcome.stored_memory_id.is_some(),
        outcome.stored_gist_memory_ids.len(),
        outcome.experience_id,
        outcome.used_memories.len(),
        outcome.used_experiences.len()
    );
    println!(
        "compute: threshold_delta_milli={} budget_saved={} sem_fusion_saved={} budget_avoided={} kv_fusion_saved={}",
        local_learning_smoke_milli(outcome.compute_budget_schedule.threshold_delta),
        outcome.compute_budget_schedule.saved_tokens,
        outcome
            .compute_budget_schedule
            .self_evolving_memory_fusion_saved_tokens,
        outcome
            .compute_budget_schedule
            .wasted_compute_avoided_tokens,
        outcome.memory_admission.fusion_plan.saved_tokens
    );
    println!(
        "routing: adaptive_saved={} task_hierarchy_mutations={} route_pressure={:.3} compute_reduction={:.3}",
        outcome.adaptive_route_plan.saved_tokens,
        outcome.task_hierarchy_plan.mutation_count(),
        outcome.task_hierarchy_plan.route_pressure,
        outcome.task_hierarchy_plan.compute_reduction
    );
    println!(
        "reasoning: reward_total={:.3} reward_action={} online_feedbacks={} active_genes={} splice_segments={} splice_exons={}",
        outcome.process_reward.total,
        outcome.process_reward.action.as_str(),
        outcome.live_evolution.online_reward_feedbacks,
        outcome.reasoning_genome.active_gene_ids.len(),
        outcome.reasoning_genome_splice.segments.len(),
        outcome.reasoning_genome_splice.exon_count()
    );
    println!(
        "live_evolution: router_delta_milli={} hierarchy_delta_milli={} memory_updates={} stored_updates={} reflection_issues={} revision_actions={}",
        local_learning_smoke_milli(outcome.live_evolution.router_threshold_delta),
        local_learning_smoke_milli(outcome.live_evolution.hierarchy_weight_delta),
        outcome.live_evolution.memory_updates(),
        outcome.live_evolution.stored_memory_updates(),
        outcome.live_evolution.reflection_issues,
        outcome.live_evolution.revision_actions
    );
    println!(
        "evolution_ledger: live_runs={} live_memory_updates={} live_stored_runtime_kv={} replay_runs={}",
        outcome.evolution_ledger.live_inference_runs,
        outcome.evolution_ledger.live_memory_updates(),
        outcome.evolution_ledger.live_stored_runtime_kv_memories,
        outcome.evolution_ledger.replay_runs
    );
    if let Some(report) = trace_gate_report {
        println!(
            "self_evolving_memory_store: events={} retrieval={} maintenance={} admission_preview={} consolidation={} consolidation_actions={} merge_preview={} decay_preview={} tombstone_preview={} saved_tokens={} write_allowed={} applied_to_disk={}",
            report.self_evolving_memory_store_events,
            report.self_evolving_memory_store_retrieval_events,
            report.self_evolving_memory_store_maintenance_events,
            report.self_evolving_memory_store_admission_preview_events,
            report.self_evolving_memory_store_consolidation_events,
            report.self_evolving_memory_store_consolidation_actions,
            report.self_evolving_memory_store_merge_previews,
            report.self_evolving_memory_store_decay_previews,
            report.self_evolving_memory_store_tombstone_previews,
            report.self_evolving_memory_store_saved_tokens,
            report.self_evolving_memory_store_write_allowed,
            report.self_evolving_memory_store_applied_to_disk
        );
        println!(
            "self_evolving_memory_writeback: events={} attempted={} accepted={} applied={} applied_to_disk={}",
            report.self_evolving_memory_writeback_events,
            report.self_evolving_memory_writeback_attempted_records,
            report.self_evolving_memory_writeback_accepted_records,
            report.self_evolving_memory_writeback_applied,
            report.self_evolving_memory_writeback_applied_to_disk
        );
    }
}
