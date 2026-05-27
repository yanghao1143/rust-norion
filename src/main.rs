use std::env;
use std::path::PathBuf;

use rust_norion::{
    CommandPromptMode, CommandRuntime, HeuristicBackend, InferenceRequest, NoironEngine,
    RuntimeBackend, TaskProfile,
};

fn main() -> std::io::Result<()> {
    let args = Args::parse(env::args().skip(1).collect());
    let mut engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;

    let outcome = if let Some(runtime_command) = args.runtime_command.clone() {
        let runtime = CommandRuntime::new(runtime_command)
            .args(args.runtime_args.clone())
            .prompt_mode(args.runtime_prompt_mode);
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
    }
    println!();
    println!("{}", outcome.answer);
    println!();
    println!(
        "quality={:.3} perplexity={:.2} threshold_after={:.3}",
        outcome.report.quality, outcome.metrics.perplexity, outcome.router_threshold_after
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

    Ok(())
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
        }
    }
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
        "Usage: rust-norion [--profile coding|writing|long|general] [--memory path] [--experience path] [--adaptive path] [--runtime-command path] [--runtime-arg arg] [--runtime-prompt-mode stdin|args] <prompt>"
    );
    std::process::exit(0);
}
