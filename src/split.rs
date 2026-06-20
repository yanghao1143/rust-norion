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
}
