use super::planner::ToolsmithInput;
use super::types::{ToolBlueprint, ToolBuildStatus, ToolIntent};
use super::util::compact;

pub(super) fn build_blueprint(
    intent: ToolIntent,
    input: ToolsmithInput<'_>,
    rust_only_gate: bool,
) -> ToolBlueprint {
    let (id, name, trigger, entrypoint, outline) = match intent {
        ToolIntent::Discovery => (
            "rust_toolsmith_probe",
            "Rust Toolsmith Probe",
            "discover missing local capabilities before editing code",
            "src/bin/noiron_toolsmith_probe.rs",
            vec![
                "parse task, profile, device, memory and experience hints from stdin",
                "rank missing capabilities by local evidence and expected reuse",
                "emit a ToolBlueprint JSONL row without creating files",
            ],
        ),
        ToolIntent::TraceAnalysis => (
            "rust_trace_lens",
            "Rust Trace Lens",
            "inspect JSONL traces and explain regressions",
            "src/bin/noiron_trace_lens.rs",
            vec![
                "stream JSONL records with std::io::BufRead",
                "count required control-plane fields and drift/reward failures",
                "emit compact text or JSONL diagnostics",
            ],
        ),
        ToolIntent::StateInspection => (
            "rust_state_lens",
            "Rust State Lens",
            "summarize local memory, experience, and adaptive state",
            "src/bin/noiron_state_lens.rs",
            vec![
                "open DiskKvStore paths in read-only inspection mode",
                "bucket memory vector dimensions and reward actions",
                "report stale or incompatible records without mutating state",
            ],
        ),
        ToolIntent::BenchmarkGate => (
            "rust_gate_runner",
            "Rust Gate Runner",
            "run focused benchmark gates for a proposed capability",
            "src/bin/noiron_gate_runner.rs",
            vec![
                "parse gate thresholds from args",
                "run existing benchmark, trace-schema, and kv-quant gates",
                "return nonzero only when a declared gate fails",
            ],
        ),
        ToolIntent::RuntimeAdapter => (
            "rust_runtime_adapter_probe",
            "Rust Runtime Adapter Probe",
            "probe self-developed runtime adapter fit without vendor lock-in",
            "src/bin/noiron_runtime_adapter_probe.rs",
            vec![
                "read RuntimeManifest and HardwarePlan summaries",
                "intersect manifest adapters with device adapter hints",
                "emit selected portable Rust fallback and rejected adapters",
            ],
        ),
        ToolIntent::MemoryMaintenance => (
            "rust_kv_maintainer",
            "Rust KV Maintainer",
            "inspect retention and KV compaction candidates",
            "src/bin/noiron_kv_maintainer.rs",
            vec![
                "scan persistent KV metadata with bounded candidate windows",
                "simulate retention and compaction before any mutation",
                "print protected ids and proposed merge pairs",
            ],
        ),
        ToolIntent::Generic => (
            "rust_task_tool",
            "Rust Task Tool",
            "create a small task-specific local Rust CLI",
            "src/bin/noiron_task_tool.rs",
            vec![
                "define a typed input struct and deterministic output report",
                "use std-only parsing unless the main crate already depends on a parser",
                "add a unit test and one CLI smoke path before promotion",
            ],
        ),
    };

    let status = if rust_only_gate {
        match intent {
            ToolIntent::RuntimeAdapter => ToolBuildStatus::Held,
            _ => ToolBuildStatus::Ready,
        }
    } else {
        ToolBuildStatus::Rejected
    };
    let mut gate_notes = vec![
        "rust_source_only".to_owned(),
        "no_dynamic_shell".to_owned(),
        "no_network_default".to_owned(),
    ];
    if status == ToolBuildStatus::Held {
        gate_notes.push("requires_runtime_contract_review".to_owned());
    }
    if status == ToolBuildStatus::Rejected {
        gate_notes.push("blocked_by_non_rust_request".to_owned());
    }

    ToolBlueprint {
        id: id.to_owned(),
        name: name.to_owned(),
        intent,
        trigger: format!("{}; prompt={}", trigger, compact(input.prompt, 80)),
        rust_crate: "rust".to_owned(),
        entrypoint: entrypoint.to_owned(),
        allowed_io: vec![
            "stdin".to_owned(),
            "stdout".to_owned(),
            "explicit-local-files".to_owned(),
        ],
        denied_capabilities: vec![
            "network".to_owned(),
            "arbitrary-shell".to_owned(),
            "non-rust-runtime".to_owned(),
            "implicit-state-mutation".to_owned(),
        ],
        build_steps: vec![
            "cargo fmt".to_owned(),
            "cargo test".to_owned(),
            "cargo run -- --trace-schema-gate <trace>".to_owned(),
        ],
        validation_steps: vec![
            "unit_test_blueprint_contract".to_owned(),
            "cli_smoke_with_sample_input".to_owned(),
            "trace_schema_gate_if_trace_output".to_owned(),
        ],
        source_outline: outline.into_iter().map(ToOwned::to_owned).collect(),
        provenance: format!("toolsmith-planner:v1:intent={}", intent.as_str()),
        status,
        gate_notes,
    }
}
