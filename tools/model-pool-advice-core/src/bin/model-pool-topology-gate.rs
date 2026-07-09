use std::fs;

use model_pool_advice_core::{
    evaluate_model_pool_topology_placement, model_pool_evidence_is_sanitized,
    sample_model_pool_topology_snapshot,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let summary = evaluate_model_pool_topology_placement(&sample_model_pool_topology_snapshot());
    let evidence = summary.evidence_line();
    let json = summary.json_line();
    if !model_pool_evidence_is_sanitized(&evidence)
        || !model_pool_evidence_is_sanitized(&json)
        || evidence.contains("rack-secret-node")
        || json.contains("rack-secret-node")
    {
        return Err("refusing to write unsanitized topology evidence".to_owned());
    }

    fs::create_dir_all("target").map_err(|error| format!("create target failed: {error}"))?;
    let artifact = "target/model-pool-topology-preview.jsonl";
    fs::write(
        artifact,
        format!("{json}\n{{\"evidence\":\"{evidence}\"}}\n"),
    )
    .map_err(|error| format!("write {artifact} failed: {error}"))?;
    println!("{evidence}");
    println!("artifact={artifact}");
    if summary.passed {
        Ok(())
    } else {
        Err(format!("topology gate failed: {:?}", summary.failures))
    }
}
