use std::env;
use std::path::PathBuf;

use rust_norion::{HeuristicBackend, InferenceRequest, NoironEngine, TaskProfile};

fn main() -> std::io::Result<()> {
    let args = Args::parse(env::args().skip(1).collect());
    let mut engine = NoironEngine::load_memory(&args.memory_path)?;
    let mut backend = HeuristicBackend;

    let outcome = engine.infer(
        InferenceRequest::new(args.prompt.clone(), args.profile),
        &mut backend,
    );
    engine.save_memory(&args.memory_path)?;

    println!("Noiron Rust prototype");
    println!("profile: {:?}", args.profile);
    println!("memory_file: {}", args.memory_path.display());
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
    println!(
        "memory: used={} stored={:?}",
        outcome.used_memories.len(),
        outcome.stored_memory_id
    );

    Ok(())
}

#[derive(Debug, Clone)]
struct Args {
    prompt: String,
    profile: TaskProfile,
    memory_path: PathBuf,
}

impl Args {
    fn parse(raw: Vec<String>) -> Self {
        let mut prompt_parts = Vec::new();
        let mut profile = None;
        let mut memory_path = PathBuf::from("noiron-memory.tsv");
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
    println!("Usage: rust-norion [--profile coding|writing|long|general] [--memory path] <prompt>");
    std::process::exit(0);
}
