use std::path::Path;

use rust_norion::TraceSchemaGateReport;

pub(crate) fn print_trace_schema_gate_report(path: &Path, report: &TraceSchemaGateReport) {
    println!("Noiron trace schema gate");
    println!("trace_file: {}", path.display());
    println!("{}", report.summary_line());
    println!("{}", runtime_closed_loop_summary_line(report));
    for failure in &report.failures {
        println!("trace_schema_gate_failure: {failure}");
    }
}

fn runtime_closed_loop_summary_line(report: &TraceSchemaGateReport) -> String {
    format!(
        "runtime_closed_loop_counters adaptive_routing_events={} adaptive_routing_saved_tokens={} task_hierarchy_events={} task_hierarchy_compute_reduction_milli={} compute_budget_events={} compute_budget_avoided_tokens={} compute_budget_kv_lookups_skipped={} memory_admission_events={} memory_admission_ledger_records={} memory_admission_ledger_preview_only={} self_evolving_memory_store_events={} self_evolving_memory_store_retrieval_events={} self_evolving_memory_store_maintenance_events={} self_evolving_memory_store_admission_preview_events={} self_evolving_memory_store_contexts={} self_evolving_memory_store_maintenance_actions={} self_evolving_memory_store_admission_candidates={} self_evolving_memory_store_write_allowed={} self_evolving_memory_store_durable_write_allowed={} self_evolving_memory_store_applied={} self_evolving_memory_store_applied_to_disk={} memory_residency_events={} memory_residency_decisions={} memory_residency_hot={} memory_residency_warm={} memory_residency_cold={} memory_residency_quarantined={} memory_residency_retired={} memory_residency_protected_rollback_anchors={} memory_residency_blocked_reasons={} memory_residency_token_estimate={} memory_residency_write_allowed={} memory_residency_durable_write_allowed={} memory_residency_applied={} kv_fusion_events={} kv_fusion_saved_tokens={} self_evolution_experiment_events={} self_evolution_experiment_rollback={} self_evolution_rollback_replay_events={} self_evolution_rollback_replay_items={} self_evolution_rollback_replay_apply_ready={} self_evolution_promotion_preflight_ready={} reasoning_genome_events={} reasoning_genome_genes={} reasoning_genome_gene_scissors_proposals={} reasoning_genome_repair_payloads={} reasoning_genome_regeneration_payloads={} reasoning_genome_splice_quarantined={} reasoning_genome_mutation_applied={}",
        report.adaptive_routing_events,
        report.adaptive_routing_saved_tokens,
        report.task_hierarchy_events,
        report.task_hierarchy_compute_reduction_milli,
        report.compute_budget_events,
        report.compute_budget_avoided_tokens,
        report.compute_budget_kv_lookups_skipped,
        report.memory_admission_events,
        report.memory_admission_ledger_records,
        report.memory_admission_ledger_preview_only,
        report.self_evolving_memory_store_events,
        report.self_evolving_memory_store_retrieval_events,
        report.self_evolving_memory_store_maintenance_events,
        report.self_evolving_memory_store_admission_preview_events,
        report.self_evolving_memory_store_contexts,
        report.self_evolving_memory_store_maintenance_actions,
        report.self_evolving_memory_store_admission_candidates,
        report.self_evolving_memory_store_write_allowed,
        report.self_evolving_memory_store_durable_write_allowed,
        report.self_evolving_memory_store_applied,
        report.self_evolving_memory_store_applied_to_disk,
        report.memory_residency_events,
        report.memory_residency_decisions,
        report.memory_residency_hot,
        report.memory_residency_warm,
        report.memory_residency_cold,
        report.memory_residency_quarantined,
        report.memory_residency_retired,
        report.memory_residency_protected_rollback_anchors,
        report.memory_residency_blocked_reasons,
        report.memory_residency_token_estimate,
        report.memory_residency_write_allowed,
        report.memory_residency_durable_write_allowed,
        report.memory_residency_applied,
        report.kv_fusion_events,
        report.kv_fusion_saved_tokens,
        report.self_evolution_experiment_events,
        report.self_evolution_experiment_rollback,
        report.self_evolution_rollback_replay_events,
        report.self_evolution_rollback_replay_items,
        report.self_evolution_rollback_replay_apply_ready,
        report.self_evolution_promotion_preflight_ready,
        report.reasoning_genome_events,
        report.reasoning_genome_genes,
        report.reasoning_genome_gene_scissors_proposals,
        report.reasoning_genome_repair_payloads,
        report.reasoning_genome_regeneration_payloads,
        report.reasoning_genome_splice_quarantined,
        report.reasoning_genome_mutation_applied,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_closed_loop_summary_line_groups_trace_gate_counters() {
        let report = TraceSchemaGateReport {
            adaptive_routing_events: 2,
            adaptive_routing_saved_tokens: 192,
            task_hierarchy_events: 1,
            task_hierarchy_compute_reduction_milli: 280,
            compute_budget_events: 3,
            compute_budget_avoided_tokens: 233,
            compute_budget_kv_lookups_skipped: 4,
            memory_admission_events: 1,
            memory_admission_ledger_records: 3,
            memory_admission_ledger_preview_only: 1,
            self_evolving_memory_store_events: 3,
            self_evolving_memory_store_retrieval_events: 1,
            self_evolving_memory_store_maintenance_events: 1,
            self_evolving_memory_store_admission_preview_events: 1,
            self_evolving_memory_store_contexts: 4,
            self_evolving_memory_store_maintenance_actions: 2,
            self_evolving_memory_store_admission_candidates: 2,
            self_evolving_memory_store_write_allowed: 0,
            self_evolving_memory_store_durable_write_allowed: 0,
            self_evolving_memory_store_applied: 0,
            self_evolving_memory_store_applied_to_disk: 0,
            memory_residency_events: 1,
            memory_residency_decisions: 4,
            memory_residency_hot: 1,
            memory_residency_warm: 1,
            memory_residency_cold: 1,
            memory_residency_quarantined: 1,
            memory_residency_retired: 0,
            memory_residency_protected_rollback_anchors: 2,
            memory_residency_blocked_reasons: 1,
            memory_residency_token_estimate: 20,
            memory_residency_write_allowed: 0,
            memory_residency_durable_write_allowed: 0,
            memory_residency_applied: 0,
            kv_fusion_events: 1,
            kv_fusion_saved_tokens: 100,
            self_evolution_experiment_events: 4,
            self_evolution_experiment_rollback: 1,
            self_evolution_rollback_replay_events: 2,
            self_evolution_rollback_replay_items: 5,
            self_evolution_rollback_replay_apply_ready: 1,
            self_evolution_promotion_preflight_ready: 1,
            reasoning_genome_events: 1,
            reasoning_genome_genes: 8,
            reasoning_genome_gene_scissors_proposals: 2,
            reasoning_genome_repair_payloads: 2,
            reasoning_genome_regeneration_payloads: 1,
            reasoning_genome_splice_quarantined: 1,
            reasoning_genome_mutation_applied: 0,
            ..TraceSchemaGateReport::default()
        };

        let line = runtime_closed_loop_summary_line(&report);

        assert!(line.starts_with("runtime_closed_loop_counters "));
        assert!(line.contains("adaptive_routing_events=2"));
        assert!(line.contains("compute_budget_avoided_tokens=233"));
        assert!(line.contains("compute_budget_kv_lookups_skipped=4"));
        assert!(line.contains("memory_admission_ledger_records=3"));
        assert!(line.contains("self_evolving_memory_store_events=3"));
        assert!(line.contains("self_evolving_memory_store_admission_candidates=2"));
        assert!(line.contains("memory_residency_decisions=4"));
        assert!(line.contains("memory_residency_quarantined=1"));
        assert!(line.contains("memory_residency_token_estimate=20"));
        assert!(line.contains("kv_fusion_saved_tokens=100"));
        assert!(line.contains("self_evolution_experiment_events=4"));
        assert!(line.contains("self_evolution_rollback_replay_items=5"));
        assert!(line.contains("self_evolution_promotion_preflight_ready=1"));
        assert!(line.contains("reasoning_genome_events=1"));
        assert!(line.contains("reasoning_genome_gene_scissors_proposals=2"));
        assert!(line.contains("reasoning_genome_mutation_applied=0"));
    }
}
