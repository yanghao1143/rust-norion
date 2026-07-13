use crate::adaptive_state::GenomeEvolutionApplyReceipt;
use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::stable_redaction_digest;
use crate::reasoning_genome::{
    DnaEvolutionApplyDecision, DnaEvolutionController, DnaEvolutionValidationEvidence,
    GeneScissorsIntent, GeneScissorsOperatorDecision, GeneScissorsTransactionJournal,
    GeneValidationStatus, MutationPlan,
};
use crate::tenant_scope::TenantScope;
use crate::writer_gate::{
    UnifiedWriterGate, UnifiedWriterGateCandidate, UnifiedWriterGateDecision,
    UnifiedWriterGatePolicy,
};

use super::NoironEngine;
use super::types::{GenomeEvolutionExplicitApplyReport, GenomeEvolutionPreview};

impl NoironEngine {
    pub fn apply_genome_evolution_preview(
        &mut self,
        preview: &GenomeEvolutionPreview,
        approval_ref: &str,
        scope: &TenantScope,
    ) -> Result<GenomeEvolutionExplicitApplyReport, GenomeEvolutionApplyReceipt> {
        if let Some(reason) = preview.eligibility_reason() {
            return Err(self.held_genome_receipt(preview.profile, reason));
        }
        if approval_ref.trim().is_empty() {
            return Err(self.held_genome_receipt(preview.profile, "approval_ref_missing"));
        }
        if self.genome_runtime_state.generation(preview.profile) != preview.generation_before {
            return Err(self.held_genome_receipt(preview.profile, "candidate_generation_stale"));
        }
        if self.genome_runtime_state.active(preview.profile).id != preview.source_genome_id {
            return Err(self.held_genome_receipt(preview.profile, "candidate_source_genome_stale"));
        }
        if preview.candidate.profile != preview.profile
            || preview.candidate.id != preview.source_genome_id
        {
            return Err(self.held_genome_receipt(preview.profile, "candidate_profile_mismatch"));
        }

        let mut plans = preview.plans.clone();
        for plan in &mut plans {
            if plan.validation_status == GeneValidationStatus::Failed {
                return Err(self
                    .held_genome_receipt(preview.profile, "mutation_candidate_validation_failed"));
            }
            plan.validation_status = GeneValidationStatus::Passed;
        }
        let mut journal = GeneScissorsTransactionJournal::from_mutation_plans(
            preview.profile,
            preview.candidate.stable_anchor_id.clone(),
            &plans,
        );
        for transaction in &mut journal.transactions {
            transaction.operator_decision = GeneScissorsOperatorDecision::Approved;
        }
        let validation = runtime_preview_validation(preview, approval_ref);
        let controller = DnaEvolutionController::default().preview_plans(
            preview.profile,
            preview.candidate.id.as_str(),
            preview.candidate.stable_anchor_id.as_str(),
            &plans,
            &validation,
            GeneScissorsOperatorDecision::Approved,
            Some(&journal),
        );
        let writer_gate = UnifiedWriterGate::new()
            .with_policy(UnifiedWriterGatePolicy {
                durable_writes_enabled: true,
                ..UnifiedWriterGatePolicy::default()
            })
            .evaluate([
                UnifiedWriterGateCandidate::genome_transaction_journal(&journal),
                UnifiedWriterGateCandidate::dna_evolution_controller_report(&controller),
            ]);
        let apply_plan = controller.explicit_apply_plan(&writer_gate);
        if writer_gate.decision != UnifiedWriterGateDecision::ReadyForExplicitApply {
            return Err(self.held_genome_receipt(preview.profile, "writer_gate_not_ready"));
        }
        if apply_plan.decision != DnaEvolutionApplyDecision::ReadyForExplicitApply {
            return Err(self.held_genome_receipt(preview.profile, "apply_plan_not_ready"));
        }

        let receipt = self.genome_runtime_state.apply_with_lineage(
            preview.profile,
            &preview.candidate,
            &plans,
            &journal.to_journal_lines(),
            approval_ref,
            &scope.lineage_tenant_scope(),
            &scope.session_id,
        );
        if !receipt.applied {
            return Err(receipt);
        }

        Ok(GenomeEvolutionExplicitApplyReport {
            candidate_digest: preview.candidate_digest.clone(),
            controller,
            writer_gate,
            apply_plan,
            receipt,
        })
    }

    pub fn rollback_genome_evolution(
        &mut self,
        profile: TaskProfile,
        expected_generation: u64,
        approval_ref: &str,
    ) -> GenomeEvolutionApplyReceipt {
        if approval_ref.trim().is_empty() {
            return self.held_genome_receipt(profile, "approval_ref_missing");
        }
        if self.genome_runtime_state.generation(profile) != expected_generation {
            return self.held_genome_receipt(profile, "rollback_generation_stale");
        }
        let Some(target) = self.genome_runtime_state.active(profile).genes.first() else {
            return self.held_genome_receipt(profile, "rollback_target_missing");
        };
        let plan = MutationPlan::preview(
            format!("mutation:{}:explicit-rollback", target.id),
            GeneScissorsIntent::Rollback,
            target.id.clone(),
            "explicit operator rollback",
            "restore the previous persisted genome snapshot",
            self.genome_runtime_state
                .active(profile)
                .stable_anchor_id
                .clone(),
        )
        .with_validation_status(GeneValidationStatus::Passed);
        let mut journal = GeneScissorsTransactionJournal::from_mutation_plans(
            profile,
            self.genome_runtime_state
                .active(profile)
                .stable_anchor_id
                .clone(),
            &[plan],
        );
        for transaction in &mut journal.transactions {
            transaction.operator_decision = GeneScissorsOperatorDecision::Approved;
        }
        self.genome_runtime_state
            .rollback(profile, &journal.to_journal_lines(), approval_ref)
    }

    fn held_genome_receipt(
        &self,
        profile: TaskProfile,
        reason: &'static str,
    ) -> GenomeEvolutionApplyReceipt {
        GenomeEvolutionApplyReceipt::held(
            profile,
            self.genome_runtime_state.generation(profile),
            self.genome_runtime_state.active(profile).id.clone(),
            reason,
        )
    }
}

fn runtime_preview_validation(
    preview: &GenomeEvolutionPreview,
    approval_ref: &str,
) -> DnaEvolutionValidationEvidence {
    let quality = preview.quality_milli.to_string();
    let reward = preview.process_reward_milli.to_string();
    DnaEvolutionValidationEvidence {
        compiler_passed: preview.expression_vm_executed,
        tests_passed: preview.reasoning_frame_valid
            && preview.output_integrity_passed
            && preview.critical_reflection_issues == 0
            && preview.contradiction_count == 0,
        benchmark_passed: preview.quality_milli >= 750 && preview.process_reward_milli >= 500,
        trace_gate_passed: preview.reasoning_frame_id.starts_with("redaction-digest:"),
        privacy_gate_passed: preview.critical_reflection_issues == 0
            && preview.contradiction_count == 0,
        canary_replay_passed: preview.transaction_replay_passed,
        rollback_replay_passed: preview
            .plans
            .iter()
            .all(|plan| !plan.rollback_anchor_id.trim().is_empty()),
        artifact_digests: vec![
            stable_redaction_digest([
                "runtime-genome-preview-vm",
                preview.reasoning_frame_id.as_str(),
            ]),
            stable_redaction_digest([
                "runtime-genome-preview-reflection",
                preview.candidate_digest.as_str(),
            ]),
            stable_redaction_digest([
                "runtime-genome-preview-benchmark",
                quality.as_str(),
                reward.as_str(),
            ]),
            stable_redaction_digest([
                "runtime-genome-preview-operator",
                approval_ref,
                preview.candidate_digest.as_str(),
            ]),
        ],
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::types::GenomeEvolutionPreview;
    use crate::engine::{HeuristicBackend, InferenceRequest};
    use crate::hierarchy::TaskProfile;
    use crate::reasoning_genome::{
        GeneScissorsIntent, GenomeExpressionBudget, GenomeExpressionEnvironment,
        GenomeExpressionVm, GenomeExpressionVmInput, MutationPlan,
    };

    use super::*;

    fn eligible_preview(engine: &NoironEngine) -> GenomeEvolutionPreview {
        let profile = TaskProfile::Coding;
        let candidate = engine.genome_runtime_state.active(profile).clone();
        let plan = MutationPlan::preview(
            "mutation:coding:relabel",
            GeneScissorsIntent::Relabel,
            candidate.genes[0].id.clone(),
            "bounded runtime improvement",
            "improve coding retrieval precision",
            candidate.stable_anchor_id.clone(),
        )
        .with_repair_payload(
            "validated coding retrieval",
            "prefer compiler-backed Rust evidence",
            ["rust", "compiler", "retrieval"],
        );
        let expression = candidate.express(crate::reasoning_genome::GenomeExpressionInput {
            profile,
            quality: 0.92,
            process_reward: 0.8,
            contradiction_count: 0,
            critical_reflection_issue_count: 0,
            revision_action_count: 0,
            used_memories: 0,
            memory_feedback_updates: 0,
            route_attention_fraction: 0.5,
            agent_team_collision_free: true,
            toolsmith_gate_passed: true,
            drift_memory_write_allowed: true,
            genome_mutation_allowed: true,
            drift_rollback: false,
            runtime_kv_hold: false,
        });
        let environment = GenomeExpressionEnvironment::preview("runtime evolution preview");
        let budget = GenomeExpressionBudget::preview(profile);
        let frame = GenomeExpressionVm.execute(GenomeExpressionVmInput::new(
            &expression,
            &environment,
            &budget,
        ));
        assert!(frame.validate_preview().is_ok());
        GenomeEvolutionPreview::new(
            profile,
            0,
            &frame,
            candidate,
            vec![plan],
            0.92,
            0.8,
            0,
            0,
            "A complete compiler-backed Rust diagnosis with bounded evidence and a minimal repair that preserves durable memory lineage.",
            true,
            true,
        )
    }

    #[test]
    fn explicit_preview_apply_commits_both_genome_chains() {
        let mut engine = NoironEngine::new();
        let preview = eligible_preview(&engine);
        let scope = TenantScope::new("local-console", "rust-norion", "apply-test");

        let report = engine
            .apply_genome_evolution_preview(&preview, "approval:apply-test", &scope)
            .unwrap();

        assert!(report.receipt.applied);
        assert!(report.receipt.dual_chain_committed);
        assert_eq!(report.receipt.generation_before, 0);
        assert_eq!(report.receipt.generation_after, 1);
        assert_eq!(report.receipt.mutation_count, 1);
    }

    #[test]
    fn explicit_preview_apply_rejects_stale_and_supports_rollback() {
        let mut engine = NoironEngine::new();
        let preview = eligible_preview(&engine);
        let scope = TenantScope::new("local-console", "rust-norion", "rollback-test");
        let applied = engine
            .apply_genome_evolution_preview(&preview, "approval:rollback-test", &scope)
            .unwrap();

        let stale = engine
            .apply_genome_evolution_preview(&preview, "approval:stale", &scope)
            .unwrap_err();
        assert_eq!(stale.reason, "candidate_generation_stale");

        let rollback = engine.rollback_genome_evolution(
            TaskProfile::Coding,
            applied.receipt.generation_after,
            "approval:rollback",
        );
        assert!(rollback.applied);
        assert!(rollback.rolled_back);
        assert!(rollback.dual_chain_committed);
        assert_eq!(rollback.generation_before, 1);
        assert_eq!(rollback.generation_after, 2);
    }

    #[test]
    fn explicit_preview_apply_rejects_tampered_or_low_quality_candidates() {
        let mut engine = NoironEngine::new();
        let scope = TenantScope::new("local-console", "rust-norion", "reject-test");
        let mut tampered = eligible_preview(&engine);
        tampered.plans[0].reason.push_str(" tampered");

        let tampered_receipt = engine
            .apply_genome_evolution_preview(&tampered, "approval:tampered", &scope)
            .unwrap_err();
        assert_eq!(tampered_receipt.reason, "candidate_digest_mismatch");

        let mut low_quality = eligible_preview(&engine);
        low_quality.quality_milli = 200;
        low_quality.candidate_digest = low_quality.recomputed_candidate_digest();
        let low_quality_receipt = engine
            .apply_genome_evolution_preview(&low_quality, "approval:low-quality", &scope)
            .unwrap_err();
        assert_eq!(low_quality_receipt.reason, "quality_below_evolution_gate");
        assert_eq!(
            engine.genome_runtime_state.generation(TaskProfile::Coding),
            0
        );
    }

    #[test]
    fn explicit_preview_apply_rejects_repeated_or_truncated_output() {
        let mut engine = NoironEngine::new();
        let scope = TenantScope::new("local-console", "rust-norion", "integrity-test");
        let mut preview = eligible_preview(&engine);
        preview.output_integrity_passed = false;
        preview.candidate_digest = preview.recomputed_candidate_digest();

        let receipt = engine
            .apply_genome_evolution_preview(&preview, "approval:integrity", &scope)
            .unwrap_err();

        assert_eq!(receipt.reason, "output_integrity_failed");
        assert_eq!(
            engine.genome_runtime_state.generation(TaskProfile::Coding),
            0
        );
    }

    #[test]
    fn evolution_output_integrity_detects_repetition_and_unclosed_code() {
        use crate::engine::types::evolution_output_integrity_passes;

        assert!(!evolution_output_integrity_passes(
            "导致写锁延迟，导致读锁延迟，导致写锁延迟，导致读锁延迟，导致写锁延迟，导致读锁延迟。这个输出随后截断在 `std::sync"
        ));
        assert!(!evolution_output_integrity_passes(
            "这是一个足够长的候选答案。重复的完整诊断段落，重复的完整诊断段落，重复的完整诊断段落，剩余内容用于满足最小长度并验证重复门禁。"
        ));
        assert!(!evolution_output_integrity_passes(
            "这段候选答案已经超过最小长度，也没有未闭合的代码标记，但它在令牌预算耗尽后停在了一个未完成的验证步骤"
        ));
        assert!(evolution_output_integrity_passes(
            "最可能原因是过期 KV 被高命中率放大。先核对命中条目的代际与质量分，再旁路旧代际做对照，最后清理低分条目。最小修复仅隔离旧代际，不删除有效记忆，并保留回滚快照。"
        ));
    }

    #[test]
    fn inference_produces_an_eligible_operator_preview_without_writing() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let scope = TenantScope::new("local-console", "rust-norion", "preview-test");

        let outcome = engine.infer(
            InferenceRequest::new(
                "分析一个 Rust 本地工具的延迟问题并给出最小修复",
                TaskProfile::Coding,
            )
            .with_tenant_scope(scope),
            &mut backend,
        );

        assert!(!outcome.dna_apply_receipt.applied);
        assert_eq!(
            outcome.dna_apply_receipt.reason,
            "explicit_authorization_missing"
        );
        assert!(
            outcome.genome_evolution_preview.is_eligible(),
            "{:?}",
            outcome.genome_evolution_preview.eligibility_reason()
        );
        assert!(outcome.genome_evolution_preview.candidate_count() > 0);
    }
}
