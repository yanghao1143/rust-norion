use std::io;
use std::path::Path;

use crate::kv_cache::{MemoryResidencyDecisionRecord, MemoryResidencyPlan};

use super::json::string_array_json;
use super::writer::append_line;

pub fn memory_residency_trace_json_line(plan: &MemoryResidencyPlan) -> String {
    let summaries = plan
        .decisions
        .iter()
        .map(memory_residency_decision_summary)
        .collect::<Vec<_>>();
    let protected_rollback_anchor_digests = plan
        .decisions
        .iter()
        .filter(|decision| decision.protected_rollback_anchor)
        .map(|decision| decision.rollback_anchor_digest.clone())
        .collect::<Vec<_>>();

    format!(
        "{{\"schema\":\"rust-norion-memory-residency-plan-v1\",\"redacted\":true,\"report_only\":true,\"decision_count\":{},\"hot\":{},\"warm\":{},\"cold\":{},\"quarantined\":{},\"retired\":{},\"protected_rollback_anchors\":{},\"blocked_reasons\":{},\"token_estimate\":{},\"read_only\":{},\"write_allowed\":{},\"durable_write_allowed\":false,\"applied\":{},\"replay_digest\":\"{}\",\"protected_rollback_anchor_digests\":{},\"decision_summaries\":{}}}",
        plan.decisions.len(),
        plan.hot_count(),
        plan.warm_count(),
        plan.cold_count(),
        plan.quarantined_count(),
        plan.retired_count(),
        plan.protected_rollback_anchor_count(),
        plan.blocked_reason_count(),
        plan.total_token_estimate(),
        plan.read_only,
        plan.write_allowed,
        plan.applied,
        plan.replay_digest,
        string_array_json(&protected_rollback_anchor_digests),
        string_array_json(&summaries)
    )
}

pub fn append_memory_residency_trace_jsonl(
    path: impl AsRef<Path>,
    plan: &MemoryResidencyPlan,
) -> io::Result<()> {
    let line = memory_residency_trace_json_line(plan);
    append_line(path, &line)
}

fn memory_residency_decision_summary(decision: &MemoryResidencyDecisionRecord) -> String {
    format!(
        "id={}:state={}:score={:.6}:tenant={}:namespace={}:rollback={}:protected={}:blocked={}:tokens={}",
        decision.id,
        decision.target_state.as_str(),
        decision.score,
        decision.tenant_id_digest,
        decision.namespace_digest,
        decision.rollback_anchor_digest,
        decision.protected_rollback_anchor,
        decision.blocked_reasons.len(),
        decision.token_estimate,
    )
}
