use std::env;
use std::path::PathBuf;

use rust_norion::{
    CommandPromptMode, CommandRuntime, DeviceClass, GistLevel, HardwareSnapshot, HeuristicBackend,
    InferenceRequest, NoironEngine, RecursiveScheduler, RuntimeBackend, RuntimeMetadata,
    TaskProfile, TierMigrationAction,
};

fn main() -> std::io::Result<()> {
    let args = Args::parse(env::args().skip(1).collect());
    let mut engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    engine.recursive_scheduler = RecursiveScheduler::new(
        args.native_window_tokens,
        args.chunk_tokens,
        args.chunk_overlap_tokens,
        args.merge_fan_in,
    );
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        args.device,
        args.cpu_load,
        args.gpu_load,
        args.ram_load,
        args.disk_load,
    ));
    let replay_report = if args.replay_limit > 0 {
        Some(engine.replay_experience(args.replay_limit))
    } else {
        None
    };

    let outcome = if let Some(runtime_command) = args.runtime_command.clone() {
        let runtime = CommandRuntime::new(runtime_command)
            .args(args.runtime_args.clone())
            .prompt_mode(args.runtime_prompt_mode)
            .with_metadata(args.runtime_metadata.clone());
        let mut backend = RuntimeBackend::new(runtime);
        engine.infer(
            InferenceRequest::new(args.prompt.clone(), args.profile),
            &mut backend,
        )
    } else {
        let mut backend = HeuristicBackend;
        engine.infer(
            InferenceRequest::new(args.prompt.clone(), args.profile),
            &mut backend,
        )
    };
    engine.save_memory(&args.memory_path)?;
    engine.save_experience(&args.experience_path)?;
    engine.save_adaptive_state(&args.adaptive_path)?;

    println!("Noiron Rust prototype");
    println!("profile: {:?}", args.profile);
    println!("memory_file: {}", args.memory_path.display());
    println!("experience_file: {}", args.experience_path.display());
    println!("adaptive_file: {}", args.adaptive_path.display());
    if let Some(runtime_command) = &args.runtime_command {
        println!("runtime_command: {}", runtime_command.display());
        println!("runtime_metadata: {}", args.runtime_metadata.summary());
    }
    if let Some(replay_report) = &replay_report {
        println!("experience_replay: {}", replay_report.summary());
    }
    println!();
    println!("{}", outcome.answer);
    println!();
    println!(
        "quality={:.3} perplexity={:.2} threshold_after={:.3}",
        outcome.report.quality, outcome.metrics.perplexity, outcome.router_threshold_after
    );
    println!("process_reward: {}", outcome.process_reward.summary());
    println!("hardware: {}", outcome.hardware_plan.summary());
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
        "recursive: required={} chunks={} merge_rounds={} prompt_tokens={} native_window={} chunk_tokens={} overlap_tokens={}",
        outcome.recursive_schedule.requires_recursion,
        outcome.recursive_schedule.chunk_count(),
        outcome.recursive_schedule.merge_round_count(),
        outcome.recursive_schedule.prompt_tokens,
        outcome.recursive_schedule.native_window_tokens,
        outcome.recursive_schedule.chunk_tokens,
        outcome.recursive_schedule.overlap_tokens
    );
    let transformer_counts = outcome.transformer_plan.counts();
    println!(
        "transformer: global={} local={} convolution={}",
        transformer_counts.global, transformer_counts.local, transformer_counts.convolution
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
        "retention: before={} after={} decayed={} removed={}",
        outcome.retention_report.before,
        outcome.retention_report.after,
        outcome.retention_report.decayed,
        outcome.retention_report.removed.len()
    );

    Ok(())
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
    runtime_command: Option<PathBuf>,
    runtime_args: Vec<String>,
    runtime_prompt_mode: CommandPromptMode,
    runtime_metadata: RuntimeMetadata,
    native_window_tokens: usize,
    chunk_tokens: usize,
    chunk_overlap_tokens: usize,
    merge_fan_in: usize,
    replay_limit: usize,
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
        let mut memory_path = PathBuf::from("noiron-memory.tsv");
        let mut experience_path = PathBuf::from("noiron-experience.ndkv");
        let mut adaptive_path = PathBuf::from("noiron-adaptive.ndkv");
        let mut runtime_command = None;
        let mut runtime_args = Vec::new();
        let mut runtime_prompt_mode = CommandPromptMode::Stdin;
        let mut runtime_metadata = RuntimeMetadata::default();
        let default_scheduler = RecursiveScheduler::default();
        let mut native_window_tokens = default_scheduler.native_window_tokens();
        let mut chunk_tokens = default_scheduler.chunk_tokens();
        let mut chunk_overlap_tokens = default_scheduler.overlap_tokens();
        let mut merge_fan_in = default_scheduler.merge_fan_in();
        let mut replay_limit = 0;
        let default_hardware = HardwareSnapshot::default();
        let mut device = default_hardware.device;
        let mut cpu_load = default_hardware.cpu_load;
        let mut gpu_load = default_hardware.gpu_load;
        let mut ram_load = default_hardware.ram_load;
        let mut disk_load = default_hardware.disk_load;
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
                "--device" if index + 1 < raw.len() => {
                    device = raw[index + 1].parse::<DeviceClass>().unwrap_or(device);
                    index += 2;
                }
                "--cpu-load" if index + 1 < raw.len() => {
                    cpu_load = parse_f32(&raw[index + 1], cpu_load);
                    index += 2;
                }
                "--gpu-load" if index + 1 < raw.len() => {
                    gpu_load = parse_f32(&raw[index + 1], gpu_load);
                    index += 2;
                }
                "--ram-load" if index + 1 < raw.len() => {
                    ram_load = parse_f32(&raw[index + 1], ram_load);
                    index += 2;
                }
                "--disk-load" if index + 1 < raw.len() => {
                    disk_load = parse_f32(&raw[index + 1], disk_load);
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

        Self {
            prompt,
            profile,
            memory_path,
            experience_path,
            adaptive_path,
            runtime_command,
            runtime_args,
            runtime_prompt_mode,
            runtime_metadata,
            native_window_tokens,
            chunk_tokens,
            chunk_overlap_tokens,
            merge_fan_in,
            replay_limit,
            device,
            cpu_load,
            gpu_load,
            ram_load,
            disk_load,
        }
    }
}

fn parse_usize(value: &str, fallback: usize) -> usize {
    value.parse::<usize>().unwrap_or(fallback)
}

fn parse_f32(value: &str, fallback: f32) -> f32 {
    value.parse::<f32>().unwrap_or(fallback)
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
        "Usage: rust-norion [--profile coding|writing|long|general] [--memory path] [--experience path] [--adaptive path] [--runtime-command path] [--runtime-arg arg] [--runtime-prompt-mode stdin|args] [--runtime-model-id id] [--runtime-tokenizer name] [--runtime-native-window n] [--runtime-embedding-dims n] [--runtime-kv-import] [--runtime-kv-export] [--runtime-kv-exchange] [--native-window n] [--chunk-tokens n] [--chunk-overlap n] [--merge-fan-in n] [--replay n] [--device auto|cpu|integrated|discrete|uma|mobile|embedded|npu|multi-gpu|edge|server] [--cpu-load f] [--gpu-load f] [--ram-load f] [--disk-load f] <prompt>"
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
            "--runtime-model-id".to_owned(),
            "dev-transformer".to_owned(),
            "--runtime-tokenizer".to_owned(),
            "dev-bpe".to_owned(),
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
        assert_eq!(args.runtime_metadata.model_id, "dev-transformer");
        assert_eq!(args.runtime_metadata.tokenizer, "dev-bpe");
        assert_eq!(args.runtime_metadata.native_context_window, 4096);
        assert_eq!(args.runtime_metadata.embedding_dimensions, 128);
        assert!(args.runtime_metadata.supports_kv_import);
        assert!(args.runtime_metadata.supports_kv_export);
        assert_eq!(args.device, DeviceClass::CpuOnly);
        assert_eq!(args.cpu_load, 75.0);
        assert_eq!(args.ram_load, 0.5);
        assert!(args.prompt.contains("nine"));
    }
}
