use super::*;
use crate::adaptive_state::EvolutionLedger;
use crate::benchmark::BenchmarkGateReport;
use crate::engine::{
    GenerationContext, HeuristicBackend, InferenceBackend, InferenceRequest, NoironEngine,
};
use crate::hierarchy::TaskProfile;
use crate::kv_cache::{MemoryUpdateAction, MemoryUpdateReport};
use crate::kv_exchange::RuntimeKvBlock;
use crate::process_reward::RewardAction;
use crate::reflection::{InferenceDraft, ReasoningStep, RuntimeDiagnostics};
use crate::router::{
    GenerationMetrics, NoironRouter, ProfileObservations, ProfileThresholds, RouterState,
    RouterThresholdAdjustmentPreviewPlanner,
};
use crate::rust_validation::RustSnippetCheckReport;
use crate::self_evolution::{
    SelfEvolutionAdmissionEvidence, SelfEvolutionAdmissionGate, SelfEvolutionAdmissionReport,
    SelfEvolutionExperimentLedger, SelfEvolutionOperatorApprovalEvidence,
    SelfEvolutionOperatorApprovalGate, SelfEvolutionOperatorApprovalReport,
    SelfEvolutionRollbackReplayApplyGate, SelfEvolutionRollbackReplayApplyReport,
    SelfEvolutionRollbackReplayGate, SelfEvolutionRollbackReplayPlan,
    SelfEvolutionValidationEvidence, SelfEvolutionValidationLane,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct RuntimePrecisionBackend;

impl InferenceBackend for RuntimePrecisionBackend {
    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        let diagnostics = RuntimeDiagnostics {
            model_id: Some("trace-runtime".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            forward_energy: Some(0.42),
            kv_influence: Some(0.37),
            ..RuntimeDiagnostics::default()
        }
        .with_device_execution(
            context.hardware_plan.device.as_str(),
            context.hardware_plan.execution.primary_lane.as_str(),
            context.hardware_plan.execution.fallback_lane.as_str(),
            context.hardware_plan.execution.memory_mode.as_str(),
        )
        .with_kv_precision(
            context.hardware_plan.execution.hot_kv_precision_bits,
            context.hardware_plan.execution.cold_kv_precision_bits,
        );

        InferenceDraft::new(
            "Runtime precision diagnostics expose the self-developed ABI.",
            vec![ReasoningStep::new(
                "runtime",
                "runtime reported device execution and KV precision",
                0.91,
            )],
        )
        .with_runtime_diagnostics(diagnostics)
    }
}

struct FastPathExportingBackend;

impl InferenceBackend for FastPathExportingBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        InferenceDraft::new(
                "Rust local KV cache route memory stores useful Noiron notes for replay and future routing.",
                vec![ReasoningStep::new(
                    "runtime",
                    "exported under fast path",
                    0.45,
                )],
            )
            .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                4,
                2,
                0,
                4,
                vec![0.2, 0.1],
                vec![0.4, 0.3],
            )])
    }
}

mod adapter;
mod admission;
mod core_line;
mod evolution_replay;
mod improvement_corpus;
mod jsonl_gate;
mod memory_runtime;
mod runtime_kv;
mod rust_business;

fn auto_replay_trace_line() -> String {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let _ = engine.infer(
        InferenceRequest::new("trace auto replay seed", TaskProfile::Coding),
        &mut backend,
    );
    let outcome = engine.infer(
        InferenceRequest::new("trace auto replay seed", TaskProfile::Coding),
        &mut backend,
    );

    assert!(outcome.auto_replay_report.is_some());
    trace_json_line("trace auto replay seed", TaskProfile::Coding, 5, &outcome)
}

fn business_contract_auto_replay_trace_line() -> String {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let _ = engine.infer(
        InferenceRequest::new("trace business replay seed", TaskProfile::Coding),
        &mut backend,
    );
    let mut outcome = engine.infer(
        InferenceRequest::new("trace business replay seed", TaskProfile::Coding),
        &mut backend,
    );
    let report = outcome
        .auto_replay_report
        .as_mut()
        .expect("auto replay report should exist");
    assert!(report.applied >= 1);
    report.business_contract_items = 1;
    report.business_contract_passed = 1;
    report.business_contract_failed = 0;
    report.business_contract_raw_passed = 0;
    report.business_contract_raw_failed = 1;
    report.business_contract_response_normalized = 1;
    report.business_contract_sanitized = 0;
    report.business_contract_canonical_fallbacks = 1;
    outcome.evolution_ledger.replay_business_contract_items = 1;
    outcome.evolution_ledger.replay_business_contract_passed = 1;
    outcome.evolution_ledger.replay_business_contract_failed = 0;
    outcome.evolution_ledger.replay_business_contract_raw_passed = 0;
    outcome.evolution_ledger.replay_business_contract_raw_failed = 1;
    outcome
        .evolution_ledger
        .replay_business_contract_response_normalized = 1;
    outcome.evolution_ledger.replay_business_contract_sanitized = 0;
    outcome
        .evolution_ledger
        .replay_business_contract_canonical_fallbacks = 1;

    trace_json_line(
        "trace business replay seed",
        TaskProfile::Coding,
        5,
        &outcome,
    )
}

fn rollback_trace_line() -> String {
    struct BadBackend;

    impl InferenceBackend for BadBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new("", vec![ReasoningStep::new("runtime", "empty", 0.0)])
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = BadBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace rollback consistency", TaskProfile::Coding),
        &mut backend,
    );

    assert!(outcome.drift_report.rollback_adaptive);
    trace_json_line(
        "trace rollback consistency",
        TaskProfile::Coding,
        5,
        &outcome,
    )
}

fn runtime_kv_trace_line() -> String {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace runtime kv consistency", TaskProfile::Coding),
        &mut backend,
    );
    trace_json_line(
        "trace runtime kv consistency",
        TaskProfile::Coding,
        5,
        &outcome,
    )
}

fn fast_path_watch_trace_line() -> String {
    let mut engine = NoironEngine::new();
    engine.router.restore_state(RouterState {
        threshold: 0.88,
        observations: 0,
        profile_thresholds: ProfileThresholds::from_single(0.88),
        profile_observations: ProfileObservations::default(),
    });
    let mut backend = FastPathExportingBackend;
    let outcome = engine.infer(
        InferenceRequest::new("Rust local KV cache route memory", TaskProfile::Coding),
        &mut backend,
    );

    assert!(outcome.route_budget.attention_fraction < 0.10);
    assert!(outcome.drift_report.allow_memory_write);
    assert!(!outcome.drift_report.allow_runtime_kv_write);
    assert!(outcome.stored_memory_id.is_some());
    assert_eq!(outcome.exported_runtime_kv_blocks, 1);
    assert!(outcome.stored_runtime_kv_memory_ids.is_empty());
    assert!(
        outcome
            .drift_report
            .notes
            .iter()
            .any(|note| note == "route:fast_path_watch")
    );

    trace_json_line(
        "trace fast path runtime kv hold",
        TaskProfile::Coding,
        5,
        &outcome,
    )
}

fn replace_in_trace_object(line: &str, object: &str, from: &str, to: &str) -> String {
    let marker = format!("\"{object}\":{{");
    let object_start = line.find(&marker).expect("trace object should exist");
    let field_start = object_start + marker.len() - 1;
    let rest = &line[field_start..];
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in rest.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let object_end = field_start + offset + ch.len_utf8();
                    let mut out = String::new();
                    out.push_str(&line[..field_start]);
                    out.push_str(&line[field_start..object_end].replacen(from, to, 1));
                    out.push_str(&line[object_end..]);
                    return out;
                }
            }
            _ => {}
        }
    }

    panic!("trace object should close");
}

fn increment_trace_object_usize(line: &str, object: &str, field: &str) -> String {
    let object_json = json_object_after_field(line, object).expect("trace object should exist");
    let value =
        extract_json_usize_field(object_json, field).expect("trace usize field should exist");
    replace_in_trace_object(
        line,
        object,
        &format!("\"{field}\":{value}"),
        &format!("\"{field}\":{}", value.saturating_add(1)),
    )
}

fn replace_trace_object_usize(line: &str, object: &str, field: &str, value: usize) -> String {
    let object_json = json_object_after_field(line, object).expect("trace object should exist");
    let old = extract_json_usize_field(object_json, field).expect("trace usize field should exist");
    replace_in_trace_object(
        line,
        object,
        &format!("\"{field}\":{old}"),
        &format!("\"{field}\":{value}"),
    )
}

fn replace_trace_object_f32(line: &str, object: &str, field: &str, value: f32) -> String {
    let object_json = json_object_after_field(line, object).expect("trace object should exist");
    let old = extract_json_f32_field(object_json, field).expect("trace f32 field should exist");
    replace_in_trace_object(
        line,
        object,
        &format!("\"{field}\":{old:.6}"),
        &format!("\"{field}\":{value:.6}"),
    )
}

fn temp_path(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rust-norion-{label}-{}-{nanos}.jsonl",
        std::process::id()
    ))
}

fn cleanup(path: std::path::PathBuf) {
    let _ = fs::remove_file(path);
}
