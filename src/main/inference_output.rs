use rust_norion::{
    ExperienceReplayReport, GistLevel, LocalTransformerRuntime, ModelRuntime, TierMigrationAction,
};

use crate::cli::args::Args;
use crate::cli::display::{count_gists, count_tier_migrations};
use crate::model_service::types::TimedOutcome;

pub(crate) fn print_inference_summary(
    args: &Args,
    timed_outcome: &TimedOutcome,
    replay_report: Option<&ExperienceReplayReport>,
) -> std::io::Result<()> {
    let outcome = &timed_outcome.outcome;
    println!("Noiron Rust prototype");
    println!("profile: {:?}", args.profile);
    println!("memory_file: {}", args.memory_path.display());
    println!("experience_file: {}", args.experience_path.display());
    println!("adaptive_file: {}", args.adaptive_path.display());
    println!("elapsed_ms: {}", timed_outcome.elapsed_ms);
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
    } else if args.gemma_12b_runtime {
        let gemma_runtime = args.gemma_runtime_config();
        println!("runtime: gemma4-12b-mistralrs");
        println!("runtime_command: {}", gemma_runtime.program.display());
        println!("runtime_model_id: {}", gemma_runtime.model_id);
        println!(
            "runtime_quantization_mode: {}",
            gemma_runtime.quantization_mode.as_str()
        );
        println!("runtime_quantization: {}", gemma_runtime.quantization);
        if let Some(token_source) = &gemma_runtime.token_source {
            println!("runtime_token_source: {}", token_source);
        }
        if let Some(hf_cache) = &gemma_runtime.hf_cache {
            println!("runtime_hf_cache: {}", hf_cache.display());
        }
        if let Some(paged_attn) = &gemma_runtime.paged_attn {
            println!("runtime_paged_attn: {}", paged_attn);
        }
        if let Some(thinking) = &gemma_runtime.thinking {
            println!("runtime_thinking: {}", thinking);
        }
        println!("runtime_metadata: {}", gemma_runtime.metadata().summary());
        println!(
            "runtime_architecture: {}",
            gemma_runtime.architecture().summary()
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
    if let Some(replay_report) = replay_report {
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

    Ok(())
}
