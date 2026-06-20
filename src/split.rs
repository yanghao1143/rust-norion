//! Namespaced access to the split Norion crates.
//!
//! The root crate keeps its existing monolith-facing API stable while the
//! modular crates mature. This module makes the split boundaries visible to
//! downstream callers without flattening every type into the root namespace.
//!
//! Use these namespaces when new code wants the smaller crate contracts
//! directly, but keep the existing root re-exports for monolith-compatible
//! callers while the migration is still in progress.

pub use norion_agent as agent;
pub use norion_cli as cli;
pub use norion_core as core;
pub use norion_eval as eval;
pub use norion_memory as memory;
pub use norion_service as service;

pub mod bridge {
    pub fn memory_reuse_dry_run_to_agent_evidence(
        summary: &super::memory::MemoryReuseDryRunSummary,
    ) -> super::agent::MemoryRecallDryRunEvidence {
        super::agent::MemoryRecallDryRunEvidence {
            source: "norion_memory_reuse_dry_run".to_owned(),
            read_only: summary.read_only,
            candidate_count: summary.candidate_count,
            long_term_match_count: summary.long_term_match_count,
            context_decision_count: summary.context_decision_count,
            accepted_context_count: summary.accepted_context_count,
            rejected_context_count: summary.rejected_context_count,
            used_tokens: summary.used_tokens,
            requested_kv_count: summary.requested_kv_count,
            kv_promote_count: summary.kv_promote_count,
            kv_missing_count: summary.kv_missing_count,
            kv_already_hot_count: summary.kv_already_hot_count,
            kv_duplicate_count: summary.kv_duplicate_count,
            kv_backend_available: summary.kv_backend_available,
            memory_store_write_allowed: summary.memory_store_write_allowed,
            kv_prefetch_apply_allowed: summary.kv_prefetch_apply_allowed,
            reason_codes: summary.reason_codes.clone(),
            detail_codes: summary.detail_codes.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_split_crates_under_stable_namespaces_with_public_contracts() {
        let budget = super::agent::AgentBudget::new(1024, 8, 3);
        assert_eq!(budget.tokens, 1024);
        assert_eq!(budget.steps, 8);
        assert_eq!(budget.messages, 3);

        let cli = super::cli::CliRuntimeConfig::default();
        assert_eq!(cli.startup_snapshot().banner, "norion-cli protocol shell");

        let runtime =
            super::core::RuntimeMetadata::new("gemma-12b", "gemma-tokenizer", 262_144, 3_072);
        assert_eq!(runtime.model_id, "gemma-12b");
        assert_eq!(runtime.native_context_window, 262_144);

        let ledger = super::eval::LedgerSummary::from_records(&[]);
        assert_eq!(ledger.total_rounds, 0);
        assert_eq!(ledger.unique_rounds, 0);

        let document = super::memory::MemoryDocumentInput::new("keep clean memory", vec![0.5]);
        assert_eq!(document.content, "keep clean memory");
        assert_eq!(document.embedding, vec![0.5]);

        let message = super::service::ChatMessage::user("hello split service");
        assert_eq!(message.role, super::service::ChatRole::User);
        assert_eq!(message.content, "hello split service");
    }

    #[test]
    fn bridges_memory_reuse_dry_run_into_agent_recall_sidecar() {
        let summary = super::memory::MemoryReuseDryRunSummary {
            read_only: true,
            candidate_count: 2,
            long_term_match_count: 2,
            context_decision_count: 2,
            accepted_context_count: 1,
            rejected_context_count: 1,
            used_tokens: 96,
            requested_kv_count: 3,
            kv_promote_count: 1,
            kv_missing_count: 1,
            kv_already_hot_count: 1,
            kv_duplicate_count: 0,
            kv_backend_available: true,
            memory_store_write_allowed: false,
            kv_prefetch_apply_allowed: false,
            reason_codes: vec!["read_only".to_owned(), "context_accepted".to_owned()],
            detail_codes: vec!["kv_prefetch:promote:636f6c64".to_owned()],
        };
        let task = super::agent::AgentTask::new(
            "runtime-recall",
            super::agent::AgentRole::Coder,
            "reuse a memory dry run without prompt injection",
            super::agent::AgentBudget::new(8, 1, 1),
        );

        let evidence = super::bridge::memory_reuse_dry_run_to_agent_evidence(&summary);
        let context = super::agent::MemoryRecallContextPlanner::new()
            .plan_from_dry_run_evidence(&task, &evidence);

        assert!(evidence.safe_for_recall_sidecar());
        assert!(context.read_only);
        assert_eq!(context.returned_records, 1);
        assert_eq!(context.accepted_count(), 1);
        assert_eq!(context.rejected_count(), 0);
        assert_eq!(
            context.accepted_items()[0].source,
            "norion_memory_reuse_dry_run"
        );
        assert!(
            context.context_lines()[0]
                .contains("source=norion_memory_reuse_dry_run read_only=true")
        );
        assert!(
            context
                .reason_codes()
                .contains(&"dry_run:context_accepted".to_owned())
        );
        assert!(
            context
                .telemetry
                .iter()
                .any(|line| { line == "agent_memory_recall_dry_run_evidence_kv_promote=1" })
        );
        assert!(context.summary_line().contains("admitted=1"));
    }
}
