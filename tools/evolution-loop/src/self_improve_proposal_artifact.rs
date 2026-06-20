use std::collections::BTreeSet;

use norion_eval::{
    SelfImproveMemoryAdmissionCandidate, SelfImproveProposalAcceptanceGate,
    SelfImproveProposalAcceptanceReport, SelfImproveProposalAcceptanceSummaryReport,
    SelfImproveProposalActionAssignment, SelfImproveProposalActionAssignmentFirstTargetDigest,
    SelfImproveProposalActionAssignmentTarget, SelfImproveProposalActionClosureEvidence,
    SelfImproveProposalActionClosureItem, SelfImproveProposalActionClosureReport,
    SelfImproveProposalActionPlan, SelfImproveProposalEvidence,
    SelfImproveProposalMemoryAdmissionCommitApprovalDecisionItem,
    SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport,
    SelfImproveProposalMemoryAdmissionCommitApprovalRequestItem,
    SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport,
    SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketItem,
    SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketReport,
    SelfImproveProposalMemoryAdmissionCommitRecordStageItem,
    SelfImproveProposalMemoryAdmissionCommitRecordStageReport,
    SelfImproveProposalMemoryAdmissionDecisionReport,
    SelfImproveProposalMemoryAdmissionOperatorApprovalTokenIntakePreviewItem,
    SelfImproveProposalMemoryAdmissionOperatorApprovalTokenIntakePreviewReport,
    SelfImproveProposalMemoryAdmissionReadinessItem,
    SelfImproveProposalMemoryAdmissionReadinessReport,
    SelfImproveProposalMemoryAdmissionRequestItem, SelfImproveProposalMemoryAdmissionRequestReport,
    SelfImproveProposalMemoryAdmissionWriterDryRunItem,
    SelfImproveProposalMemoryAdmissionWriterDryRunReceiptItem,
    SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport,
    SelfImproveProposalMemoryAdmissionWriterDryRunReport,
    SelfImproveProposalMemoryAdmissionWriterPlanItem,
    SelfImproveProposalMemoryAdmissionWriterPlanReport,
    SelfImproveProposalMemoryReflectionDedupeClusterItem,
    SelfImproveProposalMemoryReflectionDedupeClusterReport,
    SelfImproveProposalMemoryReflectionReusePlanItem,
    SelfImproveProposalMemoryReflectionReusePlanReport,
    SelfImproveProposalMemoryReflectionReusePreflightItem,
    SelfImproveProposalMemoryReflectionReusePreflightReport,
    SelfImproveProposalMemoryReflectionUsefulnessItem,
    SelfImproveProposalMemoryReflectionUsefulnessReport, SelfImproveProposalPromptGuidance,
};

use crate::helper_feedback;
use crate::json::{
    json_array_field, json_bool_field, json_object_field, json_string, json_string_array,
    json_string_field, json_u64_field, parse_json_object_array,
};
use crate::validation;

const MAX_PROJECTED_PROPOSALS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfImproveProposalArtifact {
    pub(crate) total_candidate_count: usize,
    pub(crate) proposals: Vec<SelfImproveProposal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfImproveProposal {
    pub(crate) proposal_id: String,
    pub(crate) source_round: Option<u64>,
    pub(crate) evidence_id: String,
    pub(crate) suggested_action: String,
    pub(crate) validation_command: Option<String>,
    pub(crate) validation_source: Option<String>,
    pub(crate) validation_command_safety: String,
    pub(crate) validation_checked: bool,
    pub(crate) validation_passed: bool,
    pub(crate) admission_status: String,
    pub(crate) source_admission_status: Option<String>,
}

pub(crate) fn from_ledger_text(text: &str) -> SelfImproveProposalArtifact {
    let mut proposals = Vec::new();
    let mut seen = BTreeSet::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        for proposal in proposals_from_ledger_line(line) {
            let key = format!(
                "{}:{}:{}",
                proposal.proposal_id,
                proposal.source_round.unwrap_or_default(),
                proposal.evidence_id
            );
            if seen.insert(key) {
                proposals.push(proposal);
            }
        }
    }

    let total_candidate_count = proposals.len();
    let keep_from = proposals.len().saturating_sub(MAX_PROJECTED_PROPOSALS);
    proposals.drain(..keep_from);

    SelfImproveProposalArtifact {
        total_candidate_count,
        proposals,
    }
}

pub(crate) fn option_artifact_json(artifact: Option<&SelfImproveProposalArtifact>) -> String {
    match artifact {
        Some(artifact) => artifact.report_json(),
        None => format!(
            "{{\"schema\":\"self_improve_proposal_artifact_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_candidate\",\"read_only\":true,\"candidate_only\":true,\"artifact_loaded\":false,\"source\":\"ledger_round_final_json_projection\",\"total_candidate_count\":0,\"projected_candidate_count\":0,\"proposals\":[],\"side_effects\":{}}}",
            side_effects_json()
        ),
    }
}

pub(crate) fn option_acceptance_summary_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => acceptance_summary_json(
            &artifact.acceptance_summary_report(),
            artifact.total_candidate_count,
            true,
            &artifact.acceptance_action_assignment(),
        ),
        None => acceptance_summary_json(
            &SelfImproveProposalAcceptanceSummaryReport::from_reports(&[]),
            0,
            false,
            &empty_acceptance_action_assignment(),
        ),
    }
}

pub(crate) fn option_action_assignment_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => action_assignment_report_json(
            &artifact.acceptance_action_assignment(),
            artifact.total_candidate_count,
            true,
        ),
        None => action_assignment_report_json(&empty_acceptance_action_assignment(), 0, false),
    }
}

pub(crate) fn option_action_closure_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => action_closure_report_json(&artifact.action_closure_report(), true),
        None => action_closure_report_json(&empty_action_closure_report(), false),
    }
}

pub(crate) fn option_memory_admission_readiness_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_admission_readiness_report_json(
            &artifact.memory_admission_readiness_report(),
            true,
        ),
        None => memory_admission_readiness_report_json(
            &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                &empty_action_closure_report(),
            ),
            false,
        ),
    }
}

pub(crate) fn option_memory_admission_request_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => {
            memory_admission_request_report_json(&artifact.memory_admission_request_report(), true)
        }
        None => memory_admission_request_report_json(
            &SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &empty_action_closure_report(),
                ),
            ),
            false,
        ),
    }
}

pub(crate) fn option_memory_admission_decision_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_admission_decision_report_json(
            &artifact.memory_admission_decision_report(),
            true,
        ),
        None => memory_admission_decision_report_json(
            &SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                &SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                    &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                        &empty_action_closure_report(),
                    ),
                ),
            ),
            false,
        ),
    }
}

pub(crate) fn option_memory_admission_writer_plan_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => {
            memory_admission_writer_plan_report_json(&artifact.memory_admission_writer_plan(), true)
        }
        None => {
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &empty_action_closure_report(),
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            memory_admission_writer_plan_report_json(
                &SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                ),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_admission_writer_dry_run_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_admission_writer_dry_run_report_json(
            &artifact.memory_admission_writer_dry_run(),
            true,
        ),
        None => {
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &empty_action_closure_report(),
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            memory_admission_writer_dry_run_report_json(
                &SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                    &writer_plan,
                ),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_admission_writer_dry_run_receipt_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_admission_writer_dry_run_receipt_report_json(
            &artifact.memory_admission_writer_dry_run_receipt(),
            true,
        ),
        None => {
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &empty_action_closure_report(),
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            let dry_run = SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                &writer_plan,
            );
            memory_admission_writer_dry_run_receipt_report_json(
                &SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(
                    &dry_run,
                ),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_admission_commit_record_stage_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_admission_commit_record_stage_report_json(
            &artifact.memory_admission_commit_record_stage(),
            true,
        ),
        None => {
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &empty_action_closure_report(),
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            let dry_run = SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                &writer_plan,
            );
            let receipt =
                SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(&dry_run);
            memory_admission_commit_record_stage_report_json(
                &SelfImproveProposalMemoryAdmissionCommitRecordStageReport::from_dry_run_receipt(
                    &receipt,
                ),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_admission_commit_approval_request_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_admission_commit_approval_request_report_json(
            &artifact.memory_admission_commit_approval_request(),
            true,
        ),
        None => {
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &empty_action_closure_report(),
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            let dry_run = SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                &writer_plan,
            );
            let receipt =
                SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(&dry_run);
            let stage =
                SelfImproveProposalMemoryAdmissionCommitRecordStageReport::from_dry_run_receipt(
                    &receipt,
                );
            memory_admission_commit_approval_request_report_json(
                &SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport::from_commit_record_stage(
                    &stage,
                ),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_admission_commit_approval_decision_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_admission_commit_approval_decision_report_json(
            &artifact.memory_admission_commit_approval_decision(),
            true,
        ),
        None => {
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &empty_action_closure_report(),
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            let dry_run = SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                &writer_plan,
            );
            let receipt =
                SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(&dry_run);
            let stage =
                SelfImproveProposalMemoryAdmissionCommitRecordStageReport::from_dry_run_receipt(
                    &receipt,
                );
            let approval_request =
                SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport::from_commit_record_stage(
                    &stage,
                );
            memory_admission_commit_approval_decision_report_json(
                &SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport::from_commit_approval_request(
                    &approval_request,
                ),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_admission_commit_approval_review_packet_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_admission_commit_approval_review_packet_report_json(
            &artifact.memory_admission_commit_approval_review_packet(),
            true,
        ),
        None => {
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &empty_action_closure_report(),
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            let dry_run = SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                &writer_plan,
            );
            let receipt =
                SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(&dry_run);
            let stage =
                SelfImproveProposalMemoryAdmissionCommitRecordStageReport::from_dry_run_receipt(
                    &receipt,
                );
            let approval_request =
                SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport::from_commit_record_stage(
                    &stage,
                );
            let approval_decision =
                SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport::from_commit_approval_request(
                    &approval_request,
                );
            memory_admission_commit_approval_review_packet_report_json(
                &SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketReport::from_commit_approval_decision(
                    &approval_decision,
                ),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_reflection_usefulness_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => {
            memory_reflection_usefulness_report_json(&artifact.memory_reflection_usefulness(), true)
        }
        None => {
            let acceptance = SelfImproveProposalAcceptanceSummaryReport::from_reports(&[]);
            let closure = empty_action_closure_report();
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &closure,
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            let dry_run = SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                &writer_plan,
            );
            let receipt =
                SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(&dry_run);
            let stage =
                SelfImproveProposalMemoryAdmissionCommitRecordStageReport::from_dry_run_receipt(
                    &receipt,
                );
            let approval_request =
                SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport::from_commit_record_stage(
                    &stage,
                );
            let approval_decision =
                SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport::from_commit_approval_request(
                    &approval_request,
                );
            let review =
                SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketReport::from_commit_approval_decision(
                    &approval_decision,
                );
            memory_reflection_usefulness_report_json(
                &SelfImproveProposalMemoryReflectionUsefulnessReport::from_acceptance_closure_and_review_packet(
                    &acceptance,
                    &closure,
                    &review,
                ),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_reflection_dedupe_cluster_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_reflection_dedupe_cluster_report_json(
            &artifact.memory_reflection_dedupe_cluster(),
            true,
        ),
        None => {
            let acceptance = SelfImproveProposalAcceptanceSummaryReport::from_reports(&[]);
            let closure = empty_action_closure_report();
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &closure,
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            let dry_run = SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                &writer_plan,
            );
            let receipt =
                SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(&dry_run);
            let stage =
                SelfImproveProposalMemoryAdmissionCommitRecordStageReport::from_dry_run_receipt(
                    &receipt,
                );
            let approval_request =
                SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport::from_commit_record_stage(
                    &stage,
                );
            let approval_decision =
                SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport::from_commit_approval_request(
                    &approval_request,
                );
            let review =
                SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketReport::from_commit_approval_decision(
                    &approval_decision,
                );
            let usefulness =
                SelfImproveProposalMemoryReflectionUsefulnessReport::from_acceptance_closure_and_review_packet(
                    &acceptance,
                    &closure,
                    &review,
                );
            memory_reflection_dedupe_cluster_report_json(
                &SelfImproveProposalMemoryReflectionDedupeClusterReport::from_reflection_usefulness(
                    &usefulness,
                ),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_reflection_reuse_plan_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => {
            memory_reflection_reuse_plan_report_json(&artifact.memory_reflection_reuse_plan(), true)
        }
        None => {
            let acceptance = SelfImproveProposalAcceptanceSummaryReport::from_reports(&[]);
            let closure = empty_action_closure_report();
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &closure,
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            let dry_run = SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                &writer_plan,
            );
            let receipt =
                SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(&dry_run);
            let stage =
                SelfImproveProposalMemoryAdmissionCommitRecordStageReport::from_dry_run_receipt(
                    &receipt,
                );
            let approval_request =
                SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport::from_commit_record_stage(
                    &stage,
                );
            let approval_decision =
                SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport::from_commit_approval_request(
                    &approval_request,
                );
            let review =
                SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketReport::from_commit_approval_decision(
                    &approval_decision,
                );
            let usefulness =
                SelfImproveProposalMemoryReflectionUsefulnessReport::from_acceptance_closure_and_review_packet(
                    &acceptance,
                    &closure,
                    &review,
                );
            let dedupe =
                SelfImproveProposalMemoryReflectionDedupeClusterReport::from_reflection_usefulness(
                    &usefulness,
                );
            memory_reflection_reuse_plan_report_json(
                &SelfImproveProposalMemoryReflectionReusePlanReport::from_dedupe_cluster(&dedupe),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_reflection_reuse_preflight_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_reflection_reuse_preflight_report_json(
            &artifact.memory_reflection_reuse_preflight(),
            true,
        ),
        None => {
            let acceptance = SelfImproveProposalAcceptanceSummaryReport::from_reports(&[]);
            let closure = empty_action_closure_report();
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &closure,
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            let dry_run = SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                &writer_plan,
            );
            let receipt =
                SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(&dry_run);
            let stage =
                SelfImproveProposalMemoryAdmissionCommitRecordStageReport::from_dry_run_receipt(
                    &receipt,
                );
            let approval_request =
                SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport::from_commit_record_stage(
                    &stage,
                );
            let approval_decision =
                SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport::from_commit_approval_request(
                    &approval_request,
                );
            let review =
                SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketReport::from_commit_approval_decision(
                    &approval_decision,
                );
            let usefulness =
                SelfImproveProposalMemoryReflectionUsefulnessReport::from_acceptance_closure_and_review_packet(
                    &acceptance,
                    &closure,
                    &review,
                );
            let dedupe =
                SelfImproveProposalMemoryReflectionDedupeClusterReport::from_reflection_usefulness(
                    &usefulness,
                );
            let reuse =
                SelfImproveProposalMemoryReflectionReusePlanReport::from_dedupe_cluster(&dedupe);
            memory_reflection_reuse_preflight_report_json(
                &SelfImproveProposalMemoryReflectionReusePreflightReport::from_reuse_plan(&reuse),
                false,
            )
        }
    }
}

pub(crate) fn option_memory_approval_token_intake_preview_report_json(
    artifact: Option<&SelfImproveProposalArtifact>,
) -> String {
    match artifact {
        Some(artifact) => memory_approval_token_intake_preview_report_json(
            &artifact.memory_approval_token_intake_preview(),
            true,
        ),
        None => {
            let acceptance = SelfImproveProposalAcceptanceSummaryReport::from_reports(&[]);
            let closure = empty_action_closure_report();
            let request = SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
                &SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
                    &closure,
                ),
            );
            let decision =
                SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
                    &request,
                );
            let writer_plan =
                SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
                    &request, &decision,
                );
            let dry_run = SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
                &writer_plan,
            );
            let receipt =
                SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(&dry_run);
            let stage =
                SelfImproveProposalMemoryAdmissionCommitRecordStageReport::from_dry_run_receipt(
                    &receipt,
                );
            let approval_request =
                SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport::from_commit_record_stage(
                    &stage,
                );
            let approval_decision =
                SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport::from_commit_approval_request(
                    &approval_request,
                );
            let review =
                SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketReport::from_commit_approval_decision(
                    &approval_decision,
                );
            let usefulness =
                SelfImproveProposalMemoryReflectionUsefulnessReport::from_acceptance_closure_and_review_packet(
                    &acceptance,
                    &closure,
                    &review,
                );
            memory_approval_token_intake_preview_report_json(
                &SelfImproveProposalMemoryAdmissionOperatorApprovalTokenIntakePreviewReport::from_review_packet_and_reflection_usefulness(
                    &review,
                    &usefulness,
                ),
                false,
            )
        }
    }
}

impl SelfImproveProposalArtifact {
    pub(crate) fn report_json(&self) -> String {
        let proposals = self
            .proposals
            .iter()
            .map(SelfImproveProposal::report_json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"schema\":\"self_improve_proposal_artifact_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_candidate\",\"read_only\":true,\"candidate_only\":true,\"artifact_loaded\":true,\"source\":\"ledger_round_final_json_projection\",\"total_candidate_count\":{},\"projected_candidate_count\":{},\"proposals\":[{}],\"side_effects\":{}}}",
            self.total_candidate_count,
            self.proposals.len(),
            proposals,
            side_effects_json()
        )
    }

    pub(crate) fn acceptance_summary_report(&self) -> SelfImproveProposalAcceptanceSummaryReport {
        let reports = self.acceptance_reports();
        SelfImproveProposalAcceptanceSummaryReport::from_reports(&reports)
    }

    pub(crate) fn acceptance_action_assignment(&self) -> SelfImproveProposalActionAssignment {
        let reports = self.acceptance_reports();
        let summary = SelfImproveProposalAcceptanceSummaryReport::from_reports(&reports);
        let plan =
            SelfImproveProposalActionPlan::from_summary(self.total_candidate_count, &summary);
        SelfImproveProposalActionAssignment::from_reports_and_plan(&reports, &plan)
    }

    pub(crate) fn action_closure_report(&self) -> SelfImproveProposalActionClosureReport {
        let assignment = self.acceptance_action_assignment();
        let closure_evidence = self.action_closure_evidence(&assignment);
        SelfImproveProposalActionClosureReport::from_assignment_and_evidence(
            &assignment,
            &closure_evidence,
        )
    }

    pub(crate) fn memory_admission_readiness_report(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionReadinessReport {
        SelfImproveProposalMemoryAdmissionReadinessReport::from_action_closure_report(
            &self.action_closure_report(),
        )
    }

    pub(crate) fn memory_admission_request_report(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionRequestReport {
        SelfImproveProposalMemoryAdmissionRequestReport::from_readiness_report(
            &self.memory_admission_readiness_report(),
        )
    }

    pub(crate) fn memory_admission_decision_report(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionDecisionReport {
        SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(
            &self.memory_admission_request_report(),
        )
    }

    pub(crate) fn memory_admission_writer_plan(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionWriterPlanReport {
        let request = self.memory_admission_request_report();
        let decision =
            SelfImproveProposalMemoryAdmissionDecisionReport::strict_from_request_report(&request);
        SelfImproveProposalMemoryAdmissionWriterPlanReport::from_request_and_decision(
            &request, &decision,
        )
    }

    pub(crate) fn memory_admission_writer_dry_run(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionWriterDryRunReport {
        SelfImproveProposalMemoryAdmissionWriterDryRunReport::from_writer_plan(
            &self.memory_admission_writer_plan(),
        )
    }

    pub(crate) fn memory_admission_writer_dry_run_receipt(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport {
        SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport::from_dry_run(
            &self.memory_admission_writer_dry_run(),
        )
    }

    pub(crate) fn memory_admission_commit_record_stage(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionCommitRecordStageReport {
        SelfImproveProposalMemoryAdmissionCommitRecordStageReport::from_dry_run_receipt(
            &self.memory_admission_writer_dry_run_receipt(),
        )
    }

    pub(crate) fn memory_admission_commit_approval_request(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport {
        SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport::from_commit_record_stage(
            &self.memory_admission_commit_record_stage(),
        )
    }

    pub(crate) fn memory_admission_commit_approval_decision(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport {
        SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport::from_commit_approval_request(
            &self.memory_admission_commit_approval_request(),
        )
    }

    pub(crate) fn memory_admission_commit_approval_review_packet(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketReport {
        SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketReport::from_commit_approval_decision(
            &self.memory_admission_commit_approval_decision(),
        )
    }

    pub(crate) fn memory_reflection_usefulness(
        &self,
    ) -> SelfImproveProposalMemoryReflectionUsefulnessReport {
        SelfImproveProposalMemoryReflectionUsefulnessReport::from_acceptance_closure_and_review_packet(
            &self.acceptance_summary_report(),
            &self.action_closure_report(),
            &self.memory_admission_commit_approval_review_packet(),
        )
    }

    pub(crate) fn memory_reflection_dedupe_cluster(
        &self,
    ) -> SelfImproveProposalMemoryReflectionDedupeClusterReport {
        SelfImproveProposalMemoryReflectionDedupeClusterReport::from_reflection_usefulness(
            &self.memory_reflection_usefulness(),
        )
    }

    pub(crate) fn memory_reflection_reuse_plan(
        &self,
    ) -> SelfImproveProposalMemoryReflectionReusePlanReport {
        SelfImproveProposalMemoryReflectionReusePlanReport::from_dedupe_cluster(
            &self.memory_reflection_dedupe_cluster(),
        )
    }

    pub(crate) fn memory_reflection_reuse_preflight(
        &self,
    ) -> SelfImproveProposalMemoryReflectionReusePreflightReport {
        SelfImproveProposalMemoryReflectionReusePreflightReport::from_reuse_plan(
            &self.memory_reflection_reuse_plan(),
        )
    }

    pub(crate) fn memory_approval_token_intake_preview(
        &self,
    ) -> SelfImproveProposalMemoryAdmissionOperatorApprovalTokenIntakePreviewReport {
        SelfImproveProposalMemoryAdmissionOperatorApprovalTokenIntakePreviewReport::from_review_packet_and_reflection_usefulness(
            &self.memory_admission_commit_approval_review_packet(),
            &self.memory_reflection_usefulness(),
        )
    }

    fn acceptance_reports(&self) -> Vec<SelfImproveProposalAcceptanceReport> {
        self.proposals
            .iter()
            .map(SelfImproveProposal::acceptance_report)
            .collect::<Vec<_>>()
    }

    fn action_closure_evidence(
        &self,
        assignment: &SelfImproveProposalActionAssignment,
    ) -> Vec<SelfImproveProposalActionClosureEvidence> {
        assignment
            .targets
            .iter()
            .filter_map(|target| {
                self.proposals
                    .iter()
                    .find(|proposal| proposal.proposal_id == target.proposal_id)
                    .and_then(SelfImproveProposal::action_closure_evidence)
            })
            .collect::<Vec<_>>()
    }
}

impl SelfImproveProposal {
    fn report_json(&self) -> String {
        let acceptance_report = self.acceptance_report();
        format!(
            "{{\"proposal_id\":{},\"source_round\":{},\"evidence_id\":{},\"suggested_action\":{},\"validation\":{{\"command\":{},\"source\":{},\"command_safety\":{},\"checked\":{},\"passed\":{}}},\"safety_flags\":{{\"safe_validation_command\":{},\"side_effects_allowed\":false,\"mutates_code\":false,\"starts_daemon\":false,\"stops_daemon\":false,\"touches_remote\":false,\"downloads_model\":false,\"starts_forge\":false,\"starts_web_lab\":false,\"sends_prompt\":false,\"starts_stream\":false}},\"admission\":{{\"status\":{},\"source_status\":{},\"candidate_only\":true,\"auto_apply\":false,\"requires_human_or_main_window_acceptance\":true}},\"business_improvement_acceptance\":{}}}",
            json_string(&self.proposal_id),
            option_u64_json(self.source_round),
            json_string(&self.evidence_id),
            json_string(&self.suggested_action),
            option_string_json(self.validation_command.as_deref()),
            option_string_json(self.validation_source.as_deref()),
            json_string(&self.validation_command_safety),
            self.validation_checked,
            self.validation_passed,
            validation_command_is_safe_for_evidence(&self.validation_command_safety),
            json_string(&self.admission_status),
            option_string_json(self.source_admission_status.as_deref()),
            acceptance_report_json(&acceptance_report)
        )
    }

    fn acceptance_report(&self) -> SelfImproveProposalAcceptanceReport {
        let source = self
            .validation_source
            .as_deref()
            .or(self.validation_command.as_deref())
            .unwrap_or("ledger.self_improve_proposal_artifact");
        let candidate = if source_status_is_accepted(self.source_admission_status.as_deref()) {
            SelfImproveMemoryAdmissionCandidate::accepted(
                self.proposal_id.clone(),
                vec!["source admission explicitly accepted this proposal".to_owned()],
            )
        } else {
            SelfImproveMemoryAdmissionCandidate::quarantined(
                self.proposal_id.clone(),
                vec!["proposal remains advisory until accepted by main-window evidence".to_owned()],
            )
        };
        let evidence = SelfImproveProposalEvidence::clean_candidate(
            self.source_round.unwrap_or_default(),
            [self.evidence_id.clone()],
            source,
            self.suggested_action.clone(),
            candidate,
        )
        .with_source_round(self.source_round)
        .with_validation(self.validation_checked, self.validation_passed)
        .with_validation_command_source(
            source,
            validation_command_is_safe_for_evidence(&self.validation_command_safety),
        );
        SelfImproveProposalAcceptanceReport::from_gate_and_evidence(
            &SelfImproveProposalAcceptanceGate::strict(),
            &evidence,
        )
    }

    fn action_closure_evidence(&self) -> Option<SelfImproveProposalActionClosureEvidence> {
        if !proposal_requests_no_fail_fast_validation_command(&self.suggested_action) {
            return None;
        }

        let pool_default = crate::pool_stage_call::DEFAULT_TEST_GATE_VALIDATION_COMMAND;
        let repair_default = crate::helper_stage_repair::VALIDATION_COMMAND;
        let pool_default_safe = validation::test_gate_validation_command_safety(Some(pool_default));
        let repair_default_safe =
            validation::test_gate_validation_command_safety(Some(repair_default));
        let code_evidence_present = pool_default.contains("--no-fail-fast")
            && repair_default.contains("--no-fail-fast")
            && pool_default_safe == "safe"
            && repair_default_safe == "safe";
        let code_evidence_ids = if code_evidence_present {
            vec![
                "tools/evolution-loop/src/pool_stage_call.rs::DEFAULT_TEST_GATE_VALIDATION_COMMAND"
                    .to_owned(),
                "tools/evolution-loop/src/helper_stage_repair.rs::VALIDATION_COMMAND".to_owned(),
                "tools/evolution-loop/src/validation.rs::test_gate_validation_command_safety"
                    .to_owned(),
            ]
        } else {
            Vec::new()
        };
        let validation_evidence_present = self.validation_checked;
        let validation_evidence_ids = if validation_evidence_present {
            vec![
                self.evidence_id.clone(),
                "ledger.self_improve_proposal.validation_checked".to_owned(),
            ]
        } else {
            Vec::new()
        };

        Some(SelfImproveProposalActionClosureEvidence {
            proposal_id: self.proposal_id.clone(),
            source_round: self.source_round,
            closure_kind: "test_gate_no_fail_fast".to_owned(),
            code_evidence_present,
            code_evidence_ids,
            validation_evidence_present,
            validation_evidence_ids,
            validation_passed: self.validation_checked && self.validation_passed,
            memory_store_write_attempted: false,
            ndkv_write_attempted: false,
            notes: vec![
                "test-gate and helper repair validation commands include --no-fail-fast".to_owned(),
                "closure is report-only and does not auto-admit memory".to_owned(),
            ],
        })
    }
}

fn proposals_from_ledger_line(line: &str) -> Vec<SelfImproveProposal> {
    let round = json_u64_field(line, "round");
    let final_json = json_string_field(line, "final_preview").unwrap_or_default();
    let mut proposals = proposal_objects_from_json(&final_json)
        .into_iter()
        .filter_map(|object| proposal_from_object(&object, line, round))
        .collect::<Vec<_>>();
    if proposals.is_empty()
        && let Some(proposal) = proposal_from_helper_contract(line, &final_json, round)
    {
        proposals.push(proposal);
    }
    proposals
}

fn proposal_objects_from_json(final_json: &str) -> Vec<String> {
    let mut objects = Vec::new();
    for field in [
        "self_improve_proposals",
        "self_improvement_proposals",
        "self_improve_candidates",
        "proposals",
    ] {
        if let Some(array) = json_array_field(final_json, field) {
            objects.extend(parse_json_object_array(&array));
        }
    }
    for field in [
        "self_improve_proposal",
        "self_improvement_proposal",
        "self_improve_candidate",
    ] {
        if let Some(object) = json_object_field(final_json, field) {
            objects.push(object);
        }
    }
    objects
}

fn proposal_from_object(
    object: &str,
    ledger_line: &str,
    ledger_round: Option<u64>,
) -> Option<SelfImproveProposal> {
    let suggested_action = first_string_field(
        object,
        &["suggested_action", "change_request", "action", "proposal"],
    )?;
    if !suggested_action_is_actionable(&suggested_action) {
        return None;
    }
    let source_round = json_u64_field(object, "source_round").or(ledger_round);
    let proposal_id = first_string_field(object, &["proposal_id", "id"])
        .unwrap_or_else(|| derived_proposal_id(source_round, &suggested_action, "final_json"));
    let evidence_id = first_string_field(object, &["evidence_id", "evidence", "result_id"])
        .unwrap_or_else(|| derived_evidence_id(source_round, "final_json.self_improve_proposal"));
    let validation_command = first_string_field(
        object,
        &[
            "validation_command",
            "validation",
            "validation_command_preview",
        ],
    )
    .or_else(|| json_string_field(ledger_line, "validation_command_preview"));
    let validation_source = first_string_field(object, &["validation_source", "source"])
        .or_else(|| json_string_field(ledger_line, "validation_command_source"));
    let validation_command_safety = first_string_field(
        object,
        &["validation_command_safety", "validation_safety", "safety"],
    )
    .or_else(|| json_string_field(ledger_line, "validation_command_safety"))
    .unwrap_or_else(|| {
        validation::test_gate_validation_command_safety(validation_command.as_deref()).to_owned()
    });
    let source_admission_status =
        first_string_field(object, &["admission_status", "status", "admission"]);
    let validation_checked = json_bool_field(object, "validation_checked")
        .or_else(|| json_bool_field(ledger_line, "validation_checked"))
        .unwrap_or(false);
    let validation_passed = json_bool_field(object, "validation_passed")
        .or_else(|| json_bool_field(ledger_line, "validation_passed"))
        .unwrap_or(false);

    Some(SelfImproveProposal {
        proposal_id,
        source_round,
        evidence_id,
        suggested_action,
        validation_command,
        validation_source,
        validation_command_safety,
        validation_checked,
        validation_passed,
        admission_status: "candidate_report_only".to_owned(),
        source_admission_status,
    })
}

fn proposal_from_helper_contract(
    ledger_line: &str,
    final_json: &str,
    round: Option<u64>,
) -> Option<SelfImproveProposal> {
    let contract = json_object_field(ledger_line, "helper_stage_contract_by_role")
        .or_else(|| json_object_field(final_json, "helper_stage_contract_by_role"))?;
    let fields_by_role = helper_feedback::contract_fields_by_role_from_json(&contract);
    let review = fields_by_role.get("review")?;
    let suggested_action = clean_field(review.get("change_request")?)?;
    if !suggested_action_is_actionable(&suggested_action) {
        return None;
    }
    let test_gate = fields_by_role.get("test-gate");
    let validation_command = test_gate
        .and_then(|fields| fields.get("validation_command"))
        .and_then(|value| clean_field(value))
        .or_else(|| json_string_field(ledger_line, "validation_command_preview"))
        .or_else(|| {
            review
                .get("verification")
                .and_then(|value| clean_field(value))
        });
    let validation_source = if test_gate
        .and_then(|fields| fields.get("validation_command"))
        .and_then(|value| clean_field(value))
        .is_some()
    {
        Some("helper_stage_contract.test-gate.validation_command".to_owned())
    } else if json_string_field(ledger_line, "validation_command_preview").is_some() {
        json_string_field(ledger_line, "validation_command_source")
    } else {
        Some("helper_stage_contract.review.verification".to_owned())
    };
    let validation_command_safety = json_string_field(ledger_line, "validation_command_safety")
        .unwrap_or_else(|| {
            validation::test_gate_validation_command_safety(validation_command.as_deref())
                .to_owned()
        });
    let validation_checked = json_bool_field(ledger_line, "validation_checked").unwrap_or(false);
    let validation_passed = json_bool_field(ledger_line, "validation_passed").unwrap_or(false);
    let evidence_id = derived_evidence_id(round, "helper_stage_contract.review.change_request");

    Some(SelfImproveProposal {
        proposal_id: derived_proposal_id(round, &suggested_action, "helper_contract"),
        source_round: round,
        evidence_id,
        suggested_action,
        validation_command,
        validation_source,
        validation_command_safety,
        validation_checked,
        validation_passed,
        admission_status: "candidate_report_only".to_owned(),
        source_admission_status: json_bool_field(ledger_line, "self_improve_passed")
            .map(|passed| if passed { "passed" } else { "failed" }.to_owned()),
    })
}

fn first_string_field(body: &str, fields: &[&str]) -> Option<String> {
    fields
        .iter()
        .find_map(|field| json_string_field(body, field))
        .and_then(|value| clean_field(&value))
}

fn clean_field(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(value.to_owned())
    }
}

fn suggested_action_is_actionable(value: &str) -> bool {
    let normalized = normalize_candidate_action_for_matching(value);
    if normalized.is_empty() {
        return false;
    }
    let normalized = strip_leading_candidate_action_label(&normalized);
    !suggested_action_is_explicit_noop(normalized)
        && !suggested_action_is_generic_placeholder(normalized)
        && !suggested_action_is_warning_suppression_noise(normalized)
        && !suggested_action_is_dynamic_buffer_config_noise(normalized)
}

fn normalize_candidate_action_for_matching(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_leading_candidate_action_label(mut value: &str) -> &str {
    loop {
        let mut stripped = false;
        for label in [
            "change request",
            "suggested action",
            "action",
            "proposal",
            "review change request",
        ] {
            if value == label {
                return "";
            }
            if let Some(rest) = value.strip_prefix(&format!("{label} ")) {
                value = rest.trim_start();
                stripped = true;
                break;
            }
        }
        if !stripped {
            return value;
        }
    }
}

fn suggested_action_is_explicit_noop(value: &str) -> bool {
    if matches!(
        value,
        "none"
            | "n/a"
            | "na"
            | "noop"
            | "no-op"
            | "no op"
            | "no change"
            | "no changes"
            | "no changes needed"
            | "no changes required"
            | "nothing"
            | "nothing to change"
            | "nothing to do"
            | "no specific small next change"
    ) {
        return true;
    }

    for prefix in [
        "none as",
        "none because",
        "no change",
        "no changes",
        "no changes required",
        "no changes needed",
        "no change suggested in the primary answer",
        "no small next change is grounded in the same evidence",
        "no specific small next change",
    ] {
        if value.starts_with(prefix) {
            return true;
        }
    }

    false
}

fn suggested_action_is_generic_placeholder(value: &str) -> bool {
    value.starts_with("small next change grounded in the same evidence")
        || value.starts_with("small next change is grounded in the same evidence")
}

fn suggested_action_is_warning_suppression_noise(value: &str) -> bool {
    if value.contains("wno unused function") {
        return true;
    }

    let mentions_unused_warning =
        value.contains("unused function") || value.contains("unused functions");
    let points_at_validation_noise =
        value.contains("validation stderr tail") || value.contains("compiler warning");
    let only_suppresses_warning = value.contains("suppress")
        || value.contains("build flags")
        || value.contains("warning flag")
        || value.contains("address the warnings");

    mentions_unused_warning && points_at_validation_noise && only_suppresses_warning
}

fn suggested_action_is_dynamic_buffer_config_noise(value: &str) -> bool {
    let mentions_policy = value.contains("test gate dynamic upstream buffer v1");
    let asks_enable_and_tune = value.contains("enable") && value.contains("tune");
    let echoes_review_config_request = value.contains("test gate stage configuration")
        || value.contains("configuration file")
        || value.contains("primary answer");
    let only_restates_enabled_policy = !value.contains("missing")
        && !value.contains("disabled")
        && !value.contains("failing")
        && !value.contains("regression");

    mentions_policy
        && asks_enable_and_tune
        && echoes_review_config_request
        && only_restates_enabled_policy
}

fn derived_proposal_id(round: Option<u64>, action: &str, source: &str) -> String {
    let key = action
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_lowercase())
        .take(24)
        .collect::<String>();
    format!(
        "self-improve-r{}-{}-{}",
        round.unwrap_or_default(),
        source,
        if key.is_empty() { "candidate" } else { &key }
    )
}

fn derived_evidence_id(round: Option<u64>, field: &str) -> String {
    format!("ledger.round.{}.{}", round.unwrap_or_default(), field)
}

fn source_status_is_accepted(status: Option<&str>) -> bool {
    let Some(status) = status else {
        return false;
    };
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "accepted" | "admitted" | "memory_accepted" | "memory-admission-accepted"
    )
}

fn validation_command_is_safe_for_evidence(safety: &str) -> bool {
    matches!(
        safety.trim().to_ascii_lowercase().as_str(),
        "safe" | "explicit"
    )
}

fn proposal_requests_no_fail_fast_validation_command(suggested_action: &str) -> bool {
    let lower = suggested_action.to_ascii_lowercase();
    lower.contains("--no-fail-fast")
        && (lower.contains("validation_command") || lower.contains("validation command"))
}

fn acceptance_report_json(report: &SelfImproveProposalAcceptanceReport) -> String {
    format!(
        "{{\"schema\":\"self_improve_proposal_acceptance_v1\",\"validation_checked\":{},\"validation_passed\":{},\"validation_passed_for_promotion\":{},\"memory_admission_decision\":{},\"memory_admission_accepted\":{},\"evidence_backed_business_improvement\":{},\"advisory_only\":{},\"allow_promotion\":{},\"require_repair\":{},\"failure_reasons\":{}}}",
        report.validation_checked,
        report.validation_passed,
        report.validation_passed_for_promotion,
        json_string(&report.memory_admission_decision),
        report.memory_admission_accepted,
        report.evidence_backed_business_improvement,
        report.advisory_only,
        report.allow_promotion,
        report.require_repair,
        json_string_array(&report.failure_reasons)
    )
}

fn acceptance_summary_json(
    summary: &SelfImproveProposalAcceptanceSummaryReport,
    total_candidate_count: usize,
    artifact_loaded: bool,
    action_assignment: &SelfImproveProposalActionAssignment,
) -> String {
    let guidance = SelfImproveProposalPromptGuidance::from_summary(total_candidate_count, summary);
    let action_plan = SelfImproveProposalActionPlan::from_guidance(&guidance);
    format!(
        "{{\"schema\":\"self_improve_proposal_acceptance_summary_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_candidate_summary\",\"read_only\":true,\"candidate_only\":true,\"artifact_loaded\":{},\"total_candidate_count\":{},\"projected_report_count\":{},\"memory_admission_accepted_count\":{},\"evidence_backed_business_improvement_count\":{},\"advisory_only_count\":{},\"allow_promotion_count\":{},\"require_repair_count\":{},\"accepted_without_business_evidence_count\":{},\"only_advisory_or_repair\":{},\"prompt_guidance\":{{\"should_convert_advisory_to_evidence_backed_business_improvement\":{},\"should_repair_unvalidated_or_unaccepted_proposals\":{},\"requires_checked_passed_validation_and_accepted_memory_admission\":{}}},\"action_plan\":{{\"read_only\":true,\"report_only\":true,\"candidate_only\":true,\"auto_apply\":false,\"action_required\":{},\"primary_action\":{},\"actions\":{},\"requires_checked_passed_validation_and_accepted_memory_admission\":{},\"side_effects\":{}}},\"action_assignment\":{},\"side_effects\":{}}}",
        artifact_loaded,
        total_candidate_count,
        summary.projected_report_count,
        summary.memory_admission_accepted_count,
        summary.evidence_backed_business_improvement_count,
        summary.advisory_only_count,
        summary.allow_promotion_count,
        summary.require_repair_count,
        summary.accepted_without_business_evidence_count,
        summary.only_advisory_or_repair(),
        guidance.should_convert_advisory_to_evidence_backed_business_improvement,
        guidance.should_repair_unvalidated_or_unaccepted_proposals,
        guidance.requires_checked_passed_validation_and_accepted_memory_admission,
        action_plan.action_required,
        json_string(&action_plan.primary_action),
        json_string_array(&action_plan.actions),
        action_plan.requires_checked_passed_validation_and_accepted_memory_admission,
        side_effects_json(),
        action_assignment_json(action_assignment),
        side_effects_json()
    )
}

fn empty_acceptance_action_assignment() -> SelfImproveProposalActionAssignment {
    let summary = SelfImproveProposalAcceptanceSummaryReport::from_reports(&[]);
    let plan = SelfImproveProposalActionPlan::from_summary(0, &summary);
    SelfImproveProposalActionAssignment::from_reports_and_plan(&[], &plan)
}

fn empty_action_closure_report() -> SelfImproveProposalActionClosureReport {
    let assignment = empty_acceptance_action_assignment();
    SelfImproveProposalActionClosureReport::from_assignment_and_evidence(&assignment, &[])
}

fn action_assignment_json(assignment: &SelfImproveProposalActionAssignment) -> String {
    let targets = action_assignment_targets_json(assignment);
    format!(
        "{{\"read_only\":true,\"report_only\":true,\"candidate_only\":true,\"auto_apply\":false,\"action_required\":{},\"primary_action\":{},\"actions\":{},\"target_count\":{},\"requires_checked_passed_validation_and_accepted_memory_admission\":{},\"targets\":[{}],\"side_effects\":{}}}",
        assignment.action_required,
        json_string(&assignment.primary_action),
        json_string_array(&assignment.actions),
        assignment.target_count,
        assignment.requires_checked_passed_validation_and_accepted_memory_admission,
        targets,
        side_effects_json()
    )
}

fn action_closure_report_json(
    report: &SelfImproveProposalActionClosureReport,
    artifact_loaded: bool,
) -> String {
    let closure_items = report
        .closure_items
        .iter()
        .map(action_closure_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_action_closure_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_action_closure\",\"read_only\":true,\"report_only\":true,\"candidate_only\":true,\"artifact_loaded\":{},\"auto_apply\":false,\"target_count\":{},\"closed_target_count\":{},\"open_target_count\":{},\"all_targets_closed\":{},\"first_target_id\":{},\"first_target_closed\":{},\"first_target_closure_kind\":{},\"first_target_still_requires_memory_admission\":{},\"closure_items\":[{}],\"side_effects\":{}}}",
        artifact_loaded,
        report.target_count,
        report.closed_target_count,
        report.open_target_count,
        report.all_targets_closed(),
        option_string_json(report.first_target_id.as_deref()),
        report.first_target_closed,
        option_string_json(report.first_target_closure_kind.as_deref()),
        report.first_target_still_requires_memory_admission,
        closure_items,
        side_effects_json()
    )
}

fn action_closure_item_json(item: &SelfImproveProposalActionClosureItem) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"closure_kind\":{},\"closed\":{},\"code_evidence_present\":{},\"code_evidence_ids\":{},\"validation_evidence_present\":{},\"validation_evidence_ids\":{},\"validation_passed\":{},\"still_requires_memory_admission\":{},\"missing_requirements\":{},\"notes\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        json_string(&item.closure_kind),
        item.closed,
        item.code_evidence_present,
        json_string_array(&item.code_evidence_ids),
        item.validation_evidence_present,
        json_string_array(&item.validation_evidence_ids),
        item.validation_passed,
        item.still_requires_memory_admission,
        json_string_array(&item.missing_requirements),
        json_string_array(&item.notes)
    )
}

fn memory_admission_readiness_report_json(
    report: &SelfImproveProposalMemoryAdmissionReadinessReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .readiness_items
        .iter()
        .map(memory_admission_readiness_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_readiness_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_readiness\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":false,\"target_count\":{},\"ready_count\":{},\"blocked_count\":{},\"first_target_id\":{},\"first_target_ready\":{},\"all_closed_targets_ready\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"readiness_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.target_count,
        report.ready_count,
        report.blocked_count,
        option_string_json(report.first_target_id.as_deref()),
        report.first_target_ready,
        report.all_closed_targets_ready,
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_admission_readiness_item_json(
    item: &SelfImproveProposalMemoryAdmissionReadinessItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"ready_for_memory_admission\":{},\"admission_candidate_decision\":{},\"evidence_ids\":{},\"reasons\":{},\"blocked_reasons\":{},\"report_only\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.ready_for_memory_admission,
        json_string(&item.admission_candidate_decision),
        json_string_array(&item.evidence_ids),
        json_string_array(&item.reasons),
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_admission_request_report_json(
    report: &SelfImproveProposalMemoryAdmissionRequestReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .request_items
        .iter()
        .map(memory_admission_request_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_request_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_request\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"request_count\":{},\"blocked_count\":{},\"first_candidate_id\":{},\"first_candidate_ready\":{},\"all_ready_targets_requested\":{},\"writer_required\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"request_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.request_count,
        report.blocked_count,
        option_string_json(report.first_candidate_id.as_deref()),
        report.first_candidate_ready,
        report.all_ready_targets_requested,
        report.writer_required,
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_admission_request_item_json(
    item: &SelfImproveProposalMemoryAdmissionRequestItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"ready_for_memory_admission\":{},\"requested_admission_action\":{},\"writer_required\":{},\"evidence_ids\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.ready_for_memory_admission,
        json_string(&item.requested_admission_action),
        item.writer_required,
        json_string_array(&item.evidence_ids),
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_admission_decision_report_json(
    report: &SelfImproveProposalMemoryAdmissionDecisionReport,
    artifact_loaded: bool,
) -> String {
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_decision_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_decision\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"request_count\":{},\"blocked_count\":{},\"first_candidate_id\":{},\"writer_required\":{},\"admission_writer_preflight_passed\":{},\"explicit_writer_invocation_required\":{},\"admission_write_authorized\":{},\"gate_blocked\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.request_count,
        report.blocked_count,
        option_string_json(report.first_candidate_id.as_deref()),
        report.writer_required,
        report.admission_writer_preflight_passed,
        report.explicit_writer_invocation_required,
        report.admission_write_authorized,
        report.gate_blocked,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        side_effects_json()
    )
}

fn memory_admission_writer_plan_report_json(
    report: &SelfImproveProposalMemoryAdmissionWriterPlanReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .plan_items
        .iter()
        .map(memory_admission_writer_plan_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_writer_plan_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_writer_plan\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"request_count\":{},\"writer_plan_item_count\":{},\"ready_plan_count\":{},\"blocked_count\":{},\"first_plan_item_id\":{},\"writer_plan_ready\":{},\"explicit_writer_invocation_required\":{},\"experiment_required\":{},\"rollback_required\":{},\"validation_required\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"plan_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.request_count,
        report.writer_plan_item_count,
        report.ready_plan_count,
        report.blocked_count,
        option_string_json(report.first_plan_item_id.as_deref()),
        report.writer_plan_ready,
        report.explicit_writer_invocation_required,
        report.experiment_required,
        report.rollback_required,
        report.validation_required,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_admission_writer_plan_item_json(
    item: &SelfImproveProposalMemoryAdmissionWriterPlanItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"ready_for_writer_invocation\":{},\"planned_admission_action\":{},\"evidence_ids\":{},\"experiment_id\":{},\"rollback_anchor_ids\":{},\"explicit_writer_invocation_required\":{},\"operator_or_main_window_review_required\":{},\"validation_required\":{},\"write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.ready_for_writer_invocation,
        json_string(&item.planned_admission_action),
        json_string_array(&item.evidence_ids),
        json_string(&item.experiment_id),
        json_string_array(&item.rollback_anchor_ids),
        item.explicit_writer_invocation_required,
        item.operator_or_main_window_review_required,
        item.validation_required,
        item.write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_admission_writer_dry_run_report_json(
    report: &SelfImproveProposalMemoryAdmissionWriterDryRunReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .dry_run_items
        .iter()
        .map(memory_admission_writer_dry_run_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_writer_dry_run_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_writer_dry_run\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"request_count\":{},\"writer_plan_item_count\":{},\"dry_run_item_count\":{},\"ready_dry_run_count\":{},\"blocked_count\":{},\"first_dry_run_item_id\":{},\"dry_run_ready\":{},\"explicit_writer_invocation_required\":{},\"dry_run_required\":{},\"experiment_required\":{},\"rollback_required\":{},\"validation_required\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"dry_run_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.request_count,
        report.writer_plan_item_count,
        report.dry_run_item_count,
        report.ready_dry_run_count,
        report.blocked_count,
        option_string_json(report.first_dry_run_item_id.as_deref()),
        report.dry_run_ready,
        report.explicit_writer_invocation_required,
        report.dry_run_required,
        report.experiment_required,
        report.rollback_required,
        report.validation_required,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_admission_writer_dry_run_item_json(
    item: &SelfImproveProposalMemoryAdmissionWriterDryRunItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"ready_for_dry_run\":{},\"planned_writer_action\":{},\"evidence_ids\":{},\"experiment_id\":{},\"rollback_anchor_ids\":{},\"explicit_writer_invocation_required\":{},\"validation_required\":{},\"dry_run_required\":{},\"write_authorized\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.ready_for_dry_run,
        json_string(&item.planned_writer_action),
        json_string_array(&item.evidence_ids),
        json_string(&item.experiment_id),
        json_string_array(&item.rollback_anchor_ids),
        item.explicit_writer_invocation_required,
        item.validation_required,
        item.dry_run_required,
        item.write_authorized,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_admission_writer_dry_run_receipt_report_json(
    report: &SelfImproveProposalMemoryAdmissionWriterDryRunReceiptReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .receipt_items
        .iter()
        .map(memory_admission_writer_dry_run_receipt_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_writer_dry_run_receipt_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_writer_dry_run_receipt\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"request_count\":{},\"dry_run_item_count\":{},\"receipt_item_count\":{},\"succeeded_receipt_count\":{},\"blocked_count\":{},\"first_receipt_item_id\":{},\"dry_run_receipt_ready\":{},\"explicit_writer_invocation_required\":{},\"commit_allowed\":{},\"validation_required\":{},\"rollback_required\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"receipt_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.request_count,
        report.dry_run_item_count,
        report.receipt_item_count,
        report.succeeded_receipt_count,
        report.blocked_count,
        option_string_json(report.first_receipt_item_id.as_deref()),
        report.dry_run_receipt_ready,
        report.explicit_writer_invocation_required,
        report.commit_allowed,
        report.validation_required,
        report.rollback_required,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_admission_writer_dry_run_receipt_item_json(
    item: &SelfImproveProposalMemoryAdmissionWriterDryRunReceiptItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"dry_run_succeeded\":{},\"planned_writer_action\":{},\"preview_memory_record_id\":{},\"idempotency_key\":{},\"content_digest\":{},\"evidence_ids\":{},\"experiment_id\":{},\"rollback_anchor_ids\":{},\"validation_required\":{},\"rollback_required\":{},\"write_authorized\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.dry_run_succeeded,
        json_string(&item.planned_writer_action),
        json_string(&item.preview_memory_record_id),
        json_string(&item.idempotency_key),
        json_string(&item.content_digest),
        json_string_array(&item.evidence_ids),
        json_string(&item.experiment_id),
        json_string_array(&item.rollback_anchor_ids),
        item.validation_required,
        item.rollback_required,
        item.write_authorized,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_admission_commit_record_stage_report_json(
    report: &SelfImproveProposalMemoryAdmissionCommitRecordStageReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .commit_record_items
        .iter()
        .map(memory_admission_commit_record_stage_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_commit_record_stage_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_record_stage\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"request_count\":{},\"receipt_item_count\":{},\"commit_record_item_count\":{},\"staged_commit_record_count\":{},\"blocked_count\":{},\"first_commit_record_item_id\":{},\"commit_record_stage_ready\":{},\"explicit_writer_invocation_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"commit_record_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.request_count,
        report.receipt_item_count,
        report.commit_record_item_count,
        report.staged_commit_record_count,
        report.blocked_count,
        option_string_json(report.first_commit_record_item_id.as_deref()),
        report.commit_record_stage_ready,
        report.explicit_writer_invocation_required,
        report.validation_required,
        report.rollback_required,
        report.commit_allowed,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_admission_commit_record_stage_item_json(
    item: &SelfImproveProposalMemoryAdmissionCommitRecordStageItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"commit_record_staged\":{},\"planned_writer_action\":{},\"preview_memory_record_id\":{},\"staged_commit_record_id\":{},\"idempotency_key\":{},\"content_digest\":{},\"evidence_ids\":{},\"experiment_id\":{},\"rollback_anchor_ids\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"write_authorized\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.commit_record_staged,
        json_string(&item.planned_writer_action),
        json_string(&item.preview_memory_record_id),
        json_string(&item.staged_commit_record_id),
        json_string(&item.idempotency_key),
        json_string(&item.content_digest),
        json_string_array(&item.evidence_ids),
        json_string(&item.experiment_id),
        json_string_array(&item.rollback_anchor_ids),
        item.validation_required,
        item.rollback_required,
        item.commit_allowed,
        item.write_authorized,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_admission_commit_approval_request_report_json(
    report: &SelfImproveProposalMemoryAdmissionCommitApprovalRequestReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .approval_request_items
        .iter()
        .map(memory_admission_commit_approval_request_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_commit_approval_request_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_approval_request\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"request_count\":{},\"commit_record_item_count\":{},\"approval_request_item_count\":{},\"requested_commit_approval_count\":{},\"blocked_count\":{},\"first_approval_request_item_id\":{},\"commit_approval_request_ready\":{},\"explicit_commit_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"approval_request_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.request_count,
        report.commit_record_item_count,
        report.approval_request_item_count,
        report.requested_commit_approval_count,
        report.blocked_count,
        option_string_json(report.first_approval_request_item_id.as_deref()),
        report.commit_approval_request_ready,
        report.explicit_commit_approval_required,
        report.validation_required,
        report.rollback_required,
        report.commit_allowed,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_admission_commit_approval_request_item_json(
    item: &SelfImproveProposalMemoryAdmissionCommitApprovalRequestItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"commit_approval_requested\":{},\"planned_writer_action\":{},\"preview_memory_record_id\":{},\"staged_commit_record_id\":{},\"approval_request_id\":{},\"idempotency_key\":{},\"content_digest\":{},\"evidence_ids\":{},\"experiment_id\":{},\"rollback_anchor_ids\":{},\"explicit_commit_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"write_authorized\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.commit_approval_requested,
        json_string(&item.planned_writer_action),
        json_string(&item.preview_memory_record_id),
        json_string(&item.staged_commit_record_id),
        json_string(&item.approval_request_id),
        json_string(&item.idempotency_key),
        json_string(&item.content_digest),
        json_string_array(&item.evidence_ids),
        json_string(&item.experiment_id),
        json_string_array(&item.rollback_anchor_ids),
        item.explicit_commit_approval_required,
        item.validation_required,
        item.rollback_required,
        item.commit_allowed,
        item.write_authorized,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_admission_commit_approval_decision_report_json(
    report: &SelfImproveProposalMemoryAdmissionCommitApprovalDecisionReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .approval_decision_items
        .iter()
        .map(memory_admission_commit_approval_decision_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_commit_approval_decision_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_approval_decision\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"request_count\":{},\"approval_request_item_count\":{},\"approval_decision_item_count\":{},\"recorded_approval_decision_count\":{},\"approved_commit_count\":{},\"pending_approval_count\":{},\"blocked_count\":{},\"first_approval_decision_item_id\":{},\"commit_approval_decision_ready\":{},\"explicit_commit_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"approval_decision_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.request_count,
        report.approval_request_item_count,
        report.approval_decision_item_count,
        report.recorded_approval_decision_count,
        report.approved_commit_count,
        report.pending_approval_count,
        report.blocked_count,
        option_string_json(report.first_approval_decision_item_id.as_deref()),
        report.commit_approval_decision_ready,
        report.explicit_commit_approval_required,
        report.validation_required,
        report.rollback_required,
        report.commit_allowed,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_admission_commit_approval_decision_item_json(
    item: &SelfImproveProposalMemoryAdmissionCommitApprovalDecisionItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"commit_approval_decision_recorded\":{},\"commit_approval_granted\":{},\"approval_decision\":{},\"planned_writer_action\":{},\"preview_memory_record_id\":{},\"staged_commit_record_id\":{},\"approval_request_id\":{},\"approval_decision_id\":{},\"idempotency_key\":{},\"content_digest\":{},\"evidence_ids\":{},\"experiment_id\":{},\"rollback_anchor_ids\":{},\"explicit_commit_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"write_authorized\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.commit_approval_decision_recorded,
        item.commit_approval_granted,
        json_string(&item.approval_decision),
        json_string(&item.planned_writer_action),
        json_string(&item.preview_memory_record_id),
        json_string(&item.staged_commit_record_id),
        json_string(&item.approval_request_id),
        json_string(&item.approval_decision_id),
        json_string(&item.idempotency_key),
        json_string(&item.content_digest),
        json_string_array(&item.evidence_ids),
        json_string(&item.experiment_id),
        json_string_array(&item.rollback_anchor_ids),
        item.explicit_commit_approval_required,
        item.validation_required,
        item.rollback_required,
        item.commit_allowed,
        item.write_authorized,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_admission_commit_approval_review_packet_report_json(
    report: &SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .review_packet_items
        .iter()
        .map(memory_admission_commit_approval_review_packet_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_approval_review_packet\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"request_count\":{},\"approval_request_item_count\":{},\"approval_decision_item_count\":{},\"review_packet_item_count\":{},\"ready_review_packet_count\":{},\"pending_approval_count\":{},\"blocked_count\":{},\"first_review_packet_item_id\":{},\"approval_review_packet_ready\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"review_packet_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.request_count,
        report.approval_request_item_count,
        report.approval_decision_item_count,
        report.review_packet_item_count,
        report.ready_review_packet_count,
        report.pending_approval_count,
        report.blocked_count,
        option_string_json(report.first_review_packet_item_id.as_deref()),
        report.approval_review_packet_ready,
        report.explicit_operator_approval_required,
        report.validation_required,
        report.rollback_required,
        report.commit_allowed,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_admission_commit_approval_review_packet_item_json(
    item: &SelfImproveProposalMemoryAdmissionCommitApprovalReviewPacketItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"approval_review_packet_ready\":{},\"approval_decision\":{},\"planned_operator_action\":{},\"preview_memory_record_id\":{},\"staged_commit_record_id\":{},\"approval_request_id\":{},\"approval_decision_id\":{},\"approval_review_packet_id\":{},\"idempotency_key\":{},\"content_digest\":{},\"evidence_ids\":{},\"experiment_id\":{},\"rollback_anchor_ids\":{},\"approval_token\":{},\"rejection_token\":{},\"operator_checklist\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"write_authorized\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.approval_review_packet_ready,
        json_string(&item.approval_decision),
        json_string(&item.planned_operator_action),
        json_string(&item.preview_memory_record_id),
        json_string(&item.staged_commit_record_id),
        json_string(&item.approval_request_id),
        json_string(&item.approval_decision_id),
        json_string(&item.approval_review_packet_id),
        json_string(&item.idempotency_key),
        json_string(&item.content_digest),
        json_string_array(&item.evidence_ids),
        json_string(&item.experiment_id),
        json_string_array(&item.rollback_anchor_ids),
        json_string(&item.approval_token),
        json_string(&item.rejection_token),
        json_string_array(&item.operator_checklist),
        item.explicit_operator_approval_required,
        item.validation_required,
        item.rollback_required,
        item.commit_allowed,
        item.write_authorized,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_reflection_usefulness_report_json(
    report: &SelfImproveProposalMemoryReflectionUsefulnessReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .reflection_items
        .iter()
        .map(memory_reflection_usefulness_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_reflection_usefulness_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_reflection_usefulness\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"projected_report_count\":{},\"accepted_memory_admission_count\":{},\"quarantined_candidate_count\":{},\"review_packet_item_count\":{},\"useful_reflection_item_count\":{},\"pending_operator_approval_count\":{},\"blocked_count\":{},\"wasted_compute_guard_count\":{},\"adapter_safe_count\":{},\"first_reflection_item_id\":{},\"reflection_usefulness_ready\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"reflection_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.projected_report_count,
        report.accepted_memory_admission_count,
        report.quarantined_candidate_count,
        report.review_packet_item_count,
        report.useful_reflection_item_count,
        report.pending_operator_approval_count,
        report.blocked_count,
        report.wasted_compute_guard_count,
        report.adapter_safe_count,
        option_string_json(report.first_reflection_item_id.as_deref()),
        report.reflection_usefulness_ready,
        report.explicit_operator_approval_required,
        report.validation_required,
        report.rollback_required,
        report.commit_allowed,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_reflection_usefulness_item_json(
    item: &SelfImproveProposalMemoryReflectionUsefulnessItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"reflection_usefulness_ready\":{},\"reflection_status\":{},\"reuse_recommendation\":{},\"wasted_compute_guard\":{},\"adapter_safety_status\":{},\"preview_memory_record_id\":{},\"approval_review_packet_id\":{},\"idempotency_key\":{},\"content_digest\":{},\"evidence_ids\":{},\"usefulness_evidence_ids\":{},\"experiment_id\":{},\"rollback_anchor_ids\":{},\"pending_operator_approval\":{},\"closed_action_confirmed\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.reflection_usefulness_ready,
        json_string(&item.reflection_status),
        json_string(&item.reuse_recommendation),
        item.wasted_compute_guard,
        json_string(&item.adapter_safety_status),
        json_string(&item.preview_memory_record_id),
        json_string(&item.approval_review_packet_id),
        json_string(&item.idempotency_key),
        json_string(&item.content_digest),
        json_string_array(&item.evidence_ids),
        json_string_array(&item.usefulness_evidence_ids),
        json_string(&item.experiment_id),
        json_string_array(&item.rollback_anchor_ids),
        item.pending_operator_approval,
        item.closed_action_confirmed,
        item.explicit_operator_approval_required,
        item.validation_required,
        item.rollback_required,
        item.commit_allowed,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_reflection_dedupe_cluster_report_json(
    report: &SelfImproveProposalMemoryReflectionDedupeClusterReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .cluster_items
        .iter()
        .map(memory_reflection_dedupe_cluster_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_reflection_dedupe_cluster_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_reflection_dedupe_cluster\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"useful_reflection_item_count\":{},\"reflection_cluster_count\":{},\"duplicate_cluster_count\":{},\"duplicate_reflection_item_count\":{},\"wasted_compute_guard_count\":{},\"pending_operator_approval_count\":{},\"adapter_safe_count\":{},\"first_cluster_id\":{},\"reflection_dedupe_ready\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"cluster_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.useful_reflection_item_count,
        report.reflection_cluster_count,
        report.duplicate_cluster_count,
        report.duplicate_reflection_item_count,
        report.wasted_compute_guard_count,
        report.pending_operator_approval_count,
        report.adapter_safe_count,
        option_string_json(report.first_cluster_id.as_deref()),
        report.reflection_dedupe_ready,
        report.explicit_operator_approval_required,
        report.validation_required,
        report.rollback_required,
        report.commit_allowed,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_reflection_dedupe_cluster_item_json(
    item: &SelfImproveProposalMemoryReflectionDedupeClusterItem,
) -> String {
    format!(
        "{{\"cluster_id\":{},\"cluster_key\":{},\"representative_proposal_id\":{},\"proposal_ids\":{},\"approval_review_packet_ids\":{},\"content_digests\":{},\"evidence_ids\":{},\"reflection_count\":{},\"duplicate_reflection_count\":{},\"wasted_compute_guard_count\":{},\"pending_operator_approval_count\":{},\"adapter_safe_count\":{},\"reuse_recommendation\":{},\"dedupe_recommendation\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.cluster_id),
        json_string(&item.cluster_key),
        json_string(&item.representative_proposal_id),
        json_string_array(&item.proposal_ids),
        json_string_array(&item.approval_review_packet_ids),
        json_string_array(&item.content_digests),
        json_string_array(&item.evidence_ids),
        item.reflection_count,
        item.duplicate_reflection_count,
        item.wasted_compute_guard_count,
        item.pending_operator_approval_count,
        item.adapter_safe_count,
        json_string(&item.reuse_recommendation),
        json_string(&item.dedupe_recommendation),
        item.explicit_operator_approval_required,
        item.validation_required,
        item.rollback_required,
        item.commit_allowed,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_reflection_reuse_plan_report_json(
    report: &SelfImproveProposalMemoryReflectionReusePlanReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .plan_items
        .iter()
        .map(memory_reflection_reuse_plan_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_reflection_reuse_plan_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_reflection_reuse_plan\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"reflection_cluster_count\":{},\"plan_item_count\":{},\"ready_reuse_plan_count\":{},\"duplicate_cluster_count\":{},\"duplicate_reflection_item_count\":{},\"projected_saved_reflection_count\":{},\"first_plan_item_id\":{},\"reflection_reuse_plan_ready\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"plan_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.reflection_cluster_count,
        report.plan_item_count,
        report.ready_reuse_plan_count,
        report.duplicate_cluster_count,
        report.duplicate_reflection_item_count,
        report.projected_saved_reflection_count,
        option_string_json(report.first_plan_item_id.as_deref()),
        report.reflection_reuse_plan_ready,
        report.explicit_operator_approval_required,
        report.validation_required,
        report.rollback_required,
        report.commit_allowed,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_reflection_reuse_plan_item_json(
    item: &SelfImproveProposalMemoryReflectionReusePlanItem,
) -> String {
    format!(
        "{{\"cluster_id\":{},\"representative_proposal_id\":{},\"duplicate_proposal_ids\":{},\"approval_review_packet_ids\":{},\"evidence_ids\":{},\"reflection_count\":{},\"duplicate_reflection_count\":{},\"projected_saved_reflection_count\":{},\"reuse_plan_ready\":{},\"planned_reuse_action\":{},\"dedupe_recommendation\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.cluster_id),
        json_string(&item.representative_proposal_id),
        json_string_array(&item.duplicate_proposal_ids),
        json_string_array(&item.approval_review_packet_ids),
        json_string_array(&item.evidence_ids),
        item.reflection_count,
        item.duplicate_reflection_count,
        item.projected_saved_reflection_count,
        item.reuse_plan_ready,
        json_string(&item.planned_reuse_action),
        json_string(&item.dedupe_recommendation),
        item.explicit_operator_approval_required,
        item.validation_required,
        item.rollback_required,
        item.commit_allowed,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_reflection_reuse_preflight_report_json(
    report: &SelfImproveProposalMemoryReflectionReusePreflightReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .preflight_items
        .iter()
        .map(memory_reflection_reuse_preflight_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_reflection_reuse_preflight_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_reflection_reuse_preflight\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"plan_item_count\":{},\"ready_reuse_plan_count\":{},\"preflight_item_count\":{},\"preflight_passed_item_count\":{},\"blocked_item_count\":{},\"duplicate_cluster_count\":{},\"duplicate_reflection_item_count\":{},\"projected_saved_reflection_count\":{},\"projected_model_call_skip_count\":{},\"first_preflight_item_id\":{},\"reuse_preflight_passed\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"model_call_skip_authorized\":{},\"reflection_reuse_execution_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"preflight_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.plan_item_count,
        report.ready_reuse_plan_count,
        report.preflight_item_count,
        report.preflight_passed_item_count,
        report.blocked_item_count,
        report.duplicate_cluster_count,
        report.duplicate_reflection_item_count,
        report.projected_saved_reflection_count,
        report.projected_model_call_skip_count,
        option_string_json(report.first_preflight_item_id.as_deref()),
        report.reuse_preflight_passed,
        report.explicit_operator_approval_required,
        report.validation_required,
        report.rollback_required,
        report.commit_allowed,
        report.admission_write_authorized,
        report.model_call_skip_authorized,
        report.reflection_reuse_execution_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_reflection_reuse_preflight_item_json(
    item: &SelfImproveProposalMemoryReflectionReusePreflightItem,
) -> String {
    format!(
        "{{\"cluster_id\":{},\"representative_proposal_id\":{},\"duplicate_proposal_ids\":{},\"evidence_ids\":{},\"projected_saved_reflection_count\":{},\"reuse_plan_ready\":{},\"reuse_preflight_passed\":{},\"planned_reuse_action\":{},\"required_operator_action\":{},\"model_call_skip_candidate\":{},\"model_call_skip_authorized\":{},\"reflection_reuse_execution_authorized\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.cluster_id),
        json_string(&item.representative_proposal_id),
        json_string_array(&item.duplicate_proposal_ids),
        json_string_array(&item.evidence_ids),
        item.projected_saved_reflection_count,
        item.reuse_plan_ready,
        item.reuse_preflight_passed,
        json_string(&item.planned_reuse_action),
        json_string(&item.required_operator_action),
        item.model_call_skip_candidate,
        item.model_call_skip_authorized,
        item.reflection_reuse_execution_authorized,
        item.explicit_operator_approval_required,
        item.validation_required,
        item.rollback_required,
        item.commit_allowed,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn memory_approval_token_intake_preview_report_json(
    report: &SelfImproveProposalMemoryAdmissionOperatorApprovalTokenIntakePreviewReport,
    artifact_loaded: bool,
) -> String {
    let items = report
        .intake_items
        .iter()
        .map(memory_approval_token_intake_preview_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":\"self_improve_proposal_memory_admission_operator_approval_token_intake_preview_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_operator_approval_token_intake_preview\",\"read_only\":true,\"report_only\":{},\"candidate_only\":{},\"artifact_loaded\":{},\"auto_apply\":{},\"target_count\":{},\"review_packet_item_count\":{},\"useful_reflection_item_count\":{},\"intake_item_count\":{},\"ready_intake_count\":{},\"pending_operator_token_count\":{},\"blocked_count\":{},\"approval_token_present_count\":{},\"rejection_token_present_count\":{},\"first_intake_item_id\":{},\"approval_token_intake_ready\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"admission_write_authorized\":{},\"failure_reasons\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{},\"intake_items\":[{}],\"side_effects\":{}}}",
        report.report_only,
        report.candidate_only,
        artifact_loaded,
        report.auto_apply,
        report.target_count,
        report.review_packet_item_count,
        report.useful_reflection_item_count,
        report.intake_item_count,
        report.ready_intake_count,
        report.pending_operator_token_count,
        report.blocked_count,
        report.approval_token_present_count,
        report.rejection_token_present_count,
        option_string_json(report.first_intake_item_id.as_deref()),
        report.approval_token_intake_ready,
        report.explicit_operator_approval_required,
        report.validation_required,
        report.rollback_required,
        report.commit_allowed,
        report.admission_write_authorized,
        json_string_array(&report.failure_reasons),
        report.memory_store_write_allowed,
        report.ndkv_write_allowed,
        items,
        side_effects_json()
    )
}

fn memory_approval_token_intake_preview_item_json(
    item: &SelfImproveProposalMemoryAdmissionOperatorApprovalTokenIntakePreviewItem,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"token_intake_ready\":{},\"intake_status\":{},\"planned_operator_action\":{},\"approval_token\":{},\"rejection_token\":{},\"preview_memory_record_id\":{},\"approval_review_packet_id\":{},\"idempotency_key\":{},\"content_digest\":{},\"evidence_ids\":{},\"usefulness_evidence_ids\":{},\"experiment_id\":{},\"rollback_anchor_ids\":{},\"pending_operator_approval\":{},\"useful_reflection_confirmed\":{},\"approval_review_packet_ready\":{},\"explicit_operator_approval_required\":{},\"validation_required\":{},\"rollback_required\":{},\"commit_allowed\":{},\"write_authorized\":{},\"admission_write_authorized\":{},\"blocked_reasons\":{},\"report_only\":{},\"candidate_only\":{},\"auto_apply\":{},\"memory_store_write_allowed\":{},\"ndkv_write_allowed\":{}}}",
        json_string(&item.proposal_id),
        option_u64_json(item.source_round),
        item.token_intake_ready,
        json_string(&item.intake_status),
        json_string(&item.planned_operator_action),
        json_string(&item.approval_token),
        json_string(&item.rejection_token),
        json_string(&item.preview_memory_record_id),
        json_string(&item.approval_review_packet_id),
        json_string(&item.idempotency_key),
        json_string(&item.content_digest),
        json_string_array(&item.evidence_ids),
        json_string_array(&item.usefulness_evidence_ids),
        json_string(&item.experiment_id),
        json_string_array(&item.rollback_anchor_ids),
        item.pending_operator_approval,
        item.useful_reflection_confirmed,
        item.approval_review_packet_ready,
        item.explicit_operator_approval_required,
        item.validation_required,
        item.rollback_required,
        item.commit_allowed,
        item.write_authorized,
        item.admission_write_authorized,
        json_string_array(&item.blocked_reasons),
        item.report_only,
        item.candidate_only,
        item.auto_apply,
        item.memory_store_write_allowed,
        item.ndkv_write_allowed
    )
}

fn action_assignment_report_json(
    assignment: &SelfImproveProposalActionAssignment,
    total_candidate_count: usize,
    artifact_loaded: bool,
) -> String {
    let targets = action_assignment_targets_json(assignment);
    let first_target = assignment
        .first_target_digest()
        .map(|digest| action_assignment_first_target_digest_json(&digest))
        .unwrap_or_else(|| "null".to_owned());
    format!(
        "{{\"schema\":\"self_improve_proposal_action_assignment_v1\",\"consumer_surface\":\"evolution_loop_report_only_self_improve_action_assignment\",\"read_only\":true,\"report_only\":true,\"candidate_only\":true,\"artifact_loaded\":{},\"total_candidate_count\":{},\"auto_apply\":false,\"action_required\":{},\"primary_action\":{},\"actions\":{},\"target_count\":{},\"requires_checked_passed_validation_and_accepted_memory_admission\":{},\"first_target\":{},\"targets\":[{}],\"assignment\":{},\"side_effects\":{}}}",
        artifact_loaded,
        total_candidate_count,
        assignment.action_required,
        json_string(&assignment.primary_action),
        json_string_array(&assignment.actions),
        assignment.target_count,
        assignment.requires_checked_passed_validation_and_accepted_memory_admission,
        first_target,
        targets,
        action_assignment_json(assignment),
        side_effects_json()
    )
}

fn action_assignment_targets_json(assignment: &SelfImproveProposalActionAssignment) -> String {
    let targets = assignment
        .targets
        .iter()
        .map(action_assignment_target_json)
        .collect::<Vec<_>>()
        .join(",");
    targets
}

fn action_assignment_first_target_digest_json(
    digest: &SelfImproveProposalActionAssignmentFirstTargetDigest,
) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"evidence_ids\":{},\"current_memory_admission_decision\":{},\"validation_checked\":{},\"validation_passed\":{},\"memory_admission_accepted\":{},\"evidence_backed_business_improvement\":{},\"advisory_only\":{},\"require_repair\":{},\"missing_requirements\":{}}}",
        json_string(&digest.proposal_id),
        option_u64_json(digest.source_round),
        json_string_array(&digest.evidence_ids),
        json_string(&digest.current_memory_admission_decision),
        digest.validation_checked,
        digest.validation_passed,
        digest.memory_admission_accepted,
        digest.evidence_backed_business_improvement,
        digest.advisory_only,
        digest.require_repair,
        json_string_array(&digest.missing_requirements)
    )
}

fn action_assignment_target_json(target: &SelfImproveProposalActionAssignmentTarget) -> String {
    format!(
        "{{\"proposal_id\":{},\"source_round\":{},\"evidence_ids\":{},\"current_memory_admission_decision\":{},\"advisory_only\":{},\"require_repair\":{},\"validation_checked\":{},\"validation_passed\":{},\"memory_admission_accepted\":{},\"evidence_backed_business_improvement\":{},\"missing_requirements\":{}}}",
        json_string(&target.proposal_id),
        option_u64_json(target.source_round),
        json_string_array(&target.evidence_ids),
        json_string(&target.current_memory_admission_decision),
        target.advisory_only,
        target.require_repair,
        target.validation_checked,
        target.validation_passed,
        target.memory_admission_accepted,
        target.evidence_backed_business_improvement,
        json_string_array(&target.missing_requirements)
    )
}

fn side_effects_json() -> &'static str {
    "{\"applies_code\":false,\"edits_files\":false,\"mutates_ledger\":false,\"mutates_memory_store\":false,\"writes_ndkv\":false,\"starts_daemon\":false,\"stops_daemon\":false,\"touches_remote\":false,\"downloads_model\":false,\"warms_model_cache\":false,\"starts_forge\":false,\"starts_web_lab\":false,\"sends_prompt\":false,\"starts_stream\":false,\"replays_prompt\":false,\"calls_model\":false}"
}

fn option_string_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_explicit_final_json_self_improve_proposal_as_candidate() {
        let text = "{\"round\":26,\"case\":\"proposal\",\"success\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"final_preview\":\"{\\\"self_improve_proposals\\\":[{\\\"proposal_id\\\":\\\"r26-proposal-1\\\",\\\"source_round\\\":26,\\\"evidence_id\\\":\\\"helper-review:r26\\\",\\\"suggested_action\\\":\\\"add typed proposal artifact\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"validation_source\\\":\\\"test-gate\\\",\\\"admission_status\\\":\\\"proposed\\\"}]}\"}\n";

        let artifact = from_ledger_text(text);
        let json = artifact.report_json();

        assert_eq!(artifact.total_candidate_count, 1);
        assert_eq!(artifact.proposals[0].proposal_id, "r26-proposal-1");
        assert_eq!(artifact.proposals[0].source_round, Some(26));
        assert_eq!(
            artifact.proposals[0].admission_status,
            "candidate_report_only"
        );
        assert_eq!(
            artifact.proposals[0].source_admission_status.as_deref(),
            Some("proposed")
        );
        assert!(json.contains("\"schema\":\"self_improve_proposal_artifact_v1\""));
        assert!(json.contains("\"candidate_only\":true"));
        assert!(json.contains("\"auto_apply\":false"));
        assert!(json.contains("\"validation_checked\":true"));
        assert!(json.contains("\"validation_passed\":true"));
        assert!(json.contains("\"memory_admission_decision\":\"quarantined\""));
        assert!(json.contains("\"evidence_backed_business_improvement\":false"));
        assert!(json.contains("\"advisory_only\":true"));
        assert!(json.contains("\"applies_code\":false"));
        assert!(json.contains("\"calls_model\":false"));
    }

    #[test]
    fn explicit_accepted_proposal_requires_validation_before_business_improvement_claim() {
        let accepted_text = "{\"round\":28,\"case\":\"accepted-proposal\",\"success\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r28-accepted\\\",\\\"source_round\\\":28,\\\"evidence_id\\\":\\\"validation:r28\\\",\\\"suggested_action\\\":\\\"accept validated improvement\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"accepted\\\"}}\"}\n";
        let accepted_artifact = from_ledger_text(accepted_text);
        let accepted_json = accepted_artifact.report_json();

        assert!(accepted_json.contains("\"memory_admission_decision\":\"accepted\""));
        assert!(accepted_json.contains("\"memory_admission_accepted\":true"));
        assert!(accepted_json.contains("\"evidence_backed_business_improvement\":true"));
        assert!(accepted_json.contains("\"advisory_only\":false"));
        assert!(accepted_json.contains("\"require_repair\":false"));

        let unvalidated_text = "{\"round\":29,\"case\":\"unvalidated-proposal\",\"success\":true,\"validation_checked\":false,\"validation_passed\":false,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r29-unvalidated\\\",\\\"source_round\\\":29,\\\"evidence_id\\\":\\\"suggestion:r29\\\",\\\"suggested_action\\\":\\\"accept unvalidated model suggestion\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"accepted\\\"}}\"}\n";
        let unvalidated_json = from_ledger_text(unvalidated_text).report_json();

        assert!(unvalidated_json.contains("\"memory_admission_accepted\":true"));
        assert!(unvalidated_json.contains("\"validation_passed_for_promotion\":false"));
        assert!(unvalidated_json.contains("\"evidence_backed_business_improvement\":false"));
        assert!(unvalidated_json.contains("\"advisory_only\":false"));
        assert!(unvalidated_json.contains("\"require_repair\":true"));
    }

    #[test]
    fn acceptance_summary_counts_business_advisory_and_repair_candidates() {
        let text = "{\"round\":30,\"case\":\"accepted-proposal\",\"success\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r30-accepted\\\",\\\"source_round\\\":30,\\\"evidence_id\\\":\\\"validation:r30\\\",\\\"suggested_action\\\":\\\"accept validated improvement\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"accepted\\\"}}\"}\n\
{\"round\":31,\"case\":\"advisory-proposal\",\"success\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r31-advisory\\\",\\\"source_round\\\":31,\\\"evidence_id\\\":\\\"review:r31\\\",\\\"suggested_action\\\":\\\"keep report-only suggestion\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"proposed\\\"}}\"}\n\
{\"round\":32,\"case\":\"repair-proposal\",\"success\":true,\"validation_checked\":false,\"validation_passed\":false,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r32-repair\\\",\\\"source_round\\\":32,\\\"evidence_id\\\":\\\"suggestion:r32\\\",\\\"suggested_action\\\":\\\"accepted-looking suggestion without validation\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"accepted\\\"}}\"}\n";
        let artifact = from_ledger_text(text);
        let summary = artifact.acceptance_summary_report();
        let json = option_acceptance_summary_json(Some(&artifact));
        let action_assignment_json = option_action_assignment_report_json(Some(&artifact));

        assert_eq!(summary.projected_report_count, 3);
        assert_eq!(summary.memory_admission_accepted_count, 2);
        assert_eq!(summary.evidence_backed_business_improvement_count, 1);
        assert_eq!(summary.advisory_only_count, 1);
        assert_eq!(summary.require_repair_count, 1);
        assert_eq!(summary.accepted_without_business_evidence_count, 1);
        assert!(json.contains("\"schema\":\"self_improve_proposal_acceptance_summary_v1\""));
        assert!(json.contains("\"total_candidate_count\":3"));
        assert!(json.contains("\"projected_report_count\":3"));
        assert!(json.contains("\"evidence_backed_business_improvement_count\":1"));
        assert!(json.contains("\"advisory_only_count\":1"));
        assert!(json.contains("\"require_repair_count\":1"));
        assert!(json.contains("\"accepted_without_business_evidence_count\":1"));
        assert!(json.contains("\"only_advisory_or_repair\":false"));
        assert!(json.contains(
            "\"prompt_guidance\":{\"should_convert_advisory_to_evidence_backed_business_improvement\":false"
        ));
        assert!(json.contains("\"should_repair_unvalidated_or_unaccepted_proposals\":true"));
        assert!(
            json.contains(
                "\"requires_checked_passed_validation_and_accepted_memory_admission\":true"
            )
        );
        assert!(json.contains("\"action_plan\":{\"read_only\":true"));
        assert!(json.contains("\"action_required\":true"));
        assert!(json.contains("\"primary_action\":\"repair_unvalidated_or_unaccepted_proposals\""));
        assert!(json.contains("\"actions\":[\"repair_unvalidated_or_unaccepted_proposals\",\"require_checked_passed_validation_and_accepted_memory_admission\"]"));
        assert!(json.contains("\"action_assignment\":{\"read_only\":true"));
        assert!(json.contains("\"target_count\":2"));
        assert!(json.contains("\"proposal_id\":\"r31-advisory\""));
        assert!(json.contains("\"missing_requirements\":[\"accepted_memory_admission\",\"evidence_backed_business_improvement\"]"));
        assert!(json.contains("\"proposal_id\":\"r32-repair\""));
        assert!(json.contains(
            "\"missing_requirements\":[\"checked_passed_validation\",\"evidence_backed_business_improvement\"]"
        ));
        assert!(json.contains("\"auto_apply\":false"));
        assert!(json.contains("\"calls_model\":false"));
        assert!(
            action_assignment_json
                .contains("\"schema\":\"self_improve_proposal_action_assignment_v1\"")
        );
        assert!(action_assignment_json.contains("\"artifact_loaded\":true"));
        assert!(action_assignment_json.contains("\"total_candidate_count\":3"));
        assert!(action_assignment_json.contains("\"target_count\":2"));
        assert!(
            action_assignment_json.contains("\"first_target\":{\"proposal_id\":\"r31-advisory\"")
        );
        assert!(action_assignment_json.contains("\"source_round\":31"));
        assert!(action_assignment_json.contains("\"evidence_ids\":[\"review:r31\"]"));
        assert!(action_assignment_json.contains(
            "\"missing_requirements\":[\"accepted_memory_admission\",\"evidence_backed_business_improvement\"]"
        ));
        assert!(action_assignment_json.contains("\"assignment\":{\"read_only\":true"));
        assert!(action_assignment_json.contains("\"auto_apply\":false"));
        assert!(action_assignment_json.contains("\"calls_model\":false"));
    }

    #[test]
    fn acceptance_summary_prompt_guidance_is_quiet_without_candidates() {
        let json = option_acceptance_summary_json(None);
        let action_assignment_json = option_action_assignment_report_json(None);

        assert!(json.contains("\"artifact_loaded\":false"));
        assert!(json.contains("\"total_candidate_count\":0"));
        assert!(json.contains(
            "\"prompt_guidance\":{\"should_convert_advisory_to_evidence_backed_business_improvement\":false"
        ));
        assert!(json.contains("\"should_repair_unvalidated_or_unaccepted_proposals\":false"));
        assert!(json.contains(
            "\"requires_checked_passed_validation_and_accepted_memory_admission\":false"
        ));
        assert!(json.contains("\"action_required\":false"));
        assert!(json.contains("\"primary_action\":\"none\""));
        assert!(json.contains("\"actions\":[]"));
        assert!(json.contains("\"action_assignment\":{\"read_only\":true"));
        assert!(json.contains("\"target_count\":0"));
        assert!(json.contains("\"targets\":[]"));
        assert!(
            action_assignment_json
                .contains("\"schema\":\"self_improve_proposal_action_assignment_v1\"")
        );
        assert!(action_assignment_json.contains("\"artifact_loaded\":false"));
        assert!(action_assignment_json.contains("\"action_required\":false"));
        assert!(action_assignment_json.contains("\"first_target\":null"));
        assert!(action_assignment_json.contains("\"targets\":[]"));
    }

    #[test]
    fn acceptance_summary_action_plan_converts_advisory_only_candidates() {
        let text = "{\"round\":31,\"case\":\"advisory-proposal\",\"success\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r31-advisory\\\",\\\"source_round\\\":31,\\\"evidence_id\\\":\\\"review:r31\\\",\\\"suggested_action\\\":\\\"convert advisory proposal to business evidence\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"proposed\\\"}}\"}\n";

        let artifact = from_ledger_text(text);
        let json = option_acceptance_summary_json(Some(&artifact));

        assert!(json.contains("\"advisory_only_count\":1"));
        assert!(json.contains("\"evidence_backed_business_improvement_count\":0"));
        assert!(
            json.contains(
                "\"should_convert_advisory_to_evidence_backed_business_improvement\":true"
            )
        );
        assert!(json.contains("\"action_required\":true"));
        assert!(json.contains(
            "\"primary_action\":\"convert_advisory_to_evidence_backed_business_improvement\""
        ));
        assert!(json.contains("\"actions\":[\"convert_advisory_to_evidence_backed_business_improvement\",\"require_checked_passed_validation_and_accepted_memory_admission\"]"));
        assert!(json.contains("\"action_assignment\":{\"read_only\":true"));
        assert!(json.contains("\"target_count\":1"));
        assert!(json.contains("\"proposal_id\":\"r31-advisory\""));
        assert!(json.contains("\"current_memory_admission_decision\":\"quarantined\""));
        assert!(json.contains("\"missing_requirements\":[\"accepted_memory_admission\",\"evidence_backed_business_improvement\"]"));
    }

    #[test]
    fn action_closure_report_marks_no_fail_fast_targets_closed_from_code_evidence() {
        let text = "{\"round\":392,\"case\":\"no-fail-fast-proposal\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"validation stops early\",\"change_request\":\"Update the `validation_command` to include `--no-fail-fast` to ensure comprehensive testing\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";

        let artifact = from_ledger_text(text);
        let report = artifact.action_closure_report();
        let json = option_action_closure_report_json(Some(&artifact));

        assert_eq!(report.target_count, 1);
        assert_eq!(report.closed_target_count, 1);
        assert_eq!(report.open_target_count, 0);
        assert!(report.first_target_closed);
        assert_eq!(
            report.first_target_closure_kind.as_deref(),
            Some("test_gate_no_fail_fast")
        );
        assert!(report.first_target_still_requires_memory_admission);
        assert!(json.contains("\"schema\":\"self_improve_proposal_action_closure_report_v1\""));
        assert!(json.contains("\"closed_target_count\":1"));
        assert!(json.contains("\"open_target_count\":0"));
        assert!(json.contains("\"closure_kind\":\"test_gate_no_fail_fast\""));
        assert!(json.contains("\"still_requires_memory_admission\":true"));
        assert!(json.contains("DEFAULT_TEST_GATE_VALIDATION_COMMAND"));
        assert!(json.contains("\"writes_ndkv\":false"));
    }

    #[test]
    fn memory_admission_readiness_marks_closed_actions_ready_without_writes() {
        let text = "{\"round\":392,\"case\":\"no-fail-fast-proposal\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"validation stops early\",\"change_request\":\"Update the `validation_command` to include `--no-fail-fast` to ensure comprehensive testing\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";

        let artifact = from_ledger_text(text);
        let report = artifact.memory_admission_readiness_report();
        let json = option_memory_admission_readiness_report_json(Some(&artifact));
        let request_report = artifact.memory_admission_request_report();
        let request_json = option_memory_admission_request_report_json(Some(&artifact));
        let decision_report = artifact.memory_admission_decision_report();
        let decision_json = option_memory_admission_decision_report_json(Some(&artifact));
        let writer_plan = artifact.memory_admission_writer_plan();
        let writer_plan_json = option_memory_admission_writer_plan_report_json(Some(&artifact));
        let writer_dry_run = artifact.memory_admission_writer_dry_run();
        let writer_dry_run_json =
            option_memory_admission_writer_dry_run_report_json(Some(&artifact));
        let writer_dry_run_receipt = artifact.memory_admission_writer_dry_run_receipt();
        let writer_dry_run_receipt_json =
            option_memory_admission_writer_dry_run_receipt_report_json(Some(&artifact));
        let commit_record_stage = artifact.memory_admission_commit_record_stage();
        let commit_record_stage_json =
            option_memory_admission_commit_record_stage_report_json(Some(&artifact));
        let commit_approval_request = artifact.memory_admission_commit_approval_request();
        let commit_approval_request_json =
            option_memory_admission_commit_approval_request_report_json(Some(&artifact));
        let commit_approval_decision = artifact.memory_admission_commit_approval_decision();
        let commit_approval_decision_json =
            option_memory_admission_commit_approval_decision_report_json(Some(&artifact));
        let commit_approval_review = artifact.memory_admission_commit_approval_review_packet();
        let commit_approval_review_json =
            option_memory_admission_commit_approval_review_packet_report_json(Some(&artifact));
        let memory_reflection_usefulness = artifact.memory_reflection_usefulness();
        let memory_reflection_usefulness_json =
            option_memory_reflection_usefulness_report_json(Some(&artifact));
        let memory_reflection_dedupe_cluster = artifact.memory_reflection_dedupe_cluster();
        let memory_reflection_dedupe_cluster_json =
            option_memory_reflection_dedupe_cluster_report_json(Some(&artifact));
        let memory_reflection_reuse_plan = artifact.memory_reflection_reuse_plan();
        let memory_reflection_reuse_plan_json =
            option_memory_reflection_reuse_plan_report_json(Some(&artifact));
        let memory_reflection_reuse_preflight = artifact.memory_reflection_reuse_preflight();
        let memory_reflection_reuse_preflight_json =
            option_memory_reflection_reuse_preflight_report_json(Some(&artifact));
        let memory_approval_token_intake_preview = artifact.memory_approval_token_intake_preview();
        let memory_approval_token_intake_preview_json =
            option_memory_approval_token_intake_preview_report_json(Some(&artifact));

        assert_eq!(report.target_count, 1);
        assert_eq!(report.ready_count, 1);
        assert_eq!(report.blocked_count, 0);
        assert!(report.first_target_ready);
        assert!(report.all_closed_targets_ready);
        assert!(!report.memory_store_write_allowed);
        assert!(!report.ndkv_write_allowed);
        assert!(
            json.contains(
                "\"schema\":\"self_improve_proposal_memory_admission_readiness_report_v1\""
            )
        );
        assert!(json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_readiness\""));
        assert!(json.contains("\"ready_count\":1"));
        assert!(json.contains("\"blocked_count\":0"));
        assert!(json.contains("\"admission_candidate_decision\":\"candidate_ready\""));
        assert!(json.contains("\"memory_store_write_allowed\":false"));
        assert!(json.contains("\"ndkv_write_allowed\":false"));
        assert!(json.contains("\"writes_ndkv\":false"));
        assert_eq!(request_report.target_count, 1);
        assert_eq!(request_report.request_count, 1);
        assert_eq!(request_report.blocked_count, 0);
        assert!(request_report.all_ready_targets_requested);
        assert!(request_report.writer_required);
        assert!(!request_report.auto_apply);
        assert!(
            request_json.contains(
                "\"schema\":\"self_improve_proposal_memory_admission_request_report_v1\""
            )
        );
        assert!(request_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_request\""));
        assert!(request_json.contains("\"request_count\":1"));
        assert!(request_json.contains(
            "\"requested_admission_action\":\"request_evidence_backed_memory_admission\""
        ));
        assert!(request_json.contains("\"writer_required\":true"));
        assert!(request_json.contains("\"memory_store_write_allowed\":false"));
        assert!(request_json.contains("\"ndkv_write_allowed\":false"));
        assert!(request_json.contains("\"writes_ndkv\":false"));
        assert_eq!(decision_report.request_count, 1);
        assert!(decision_report.admission_writer_preflight_passed);
        assert!(decision_report.explicit_writer_invocation_required);
        assert!(!decision_report.admission_write_authorized);
        assert!(!decision_report.gate_blocked);
        assert!(
            decision_json.contains(
                "\"schema\":\"self_improve_proposal_memory_admission_decision_report_v1\""
            )
        );
        assert!(decision_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_decision\""));
        assert!(decision_json.contains("\"admission_writer_preflight_passed\":true"));
        assert!(decision_json.contains("\"explicit_writer_invocation_required\":true"));
        assert!(decision_json.contains("\"admission_write_authorized\":false"));
        assert!(decision_json.contains("\"gate_blocked\":false"));
        assert!(decision_json.contains("\"failure_reasons\":[]"));
        assert!(decision_json.contains("\"memory_store_write_allowed\":false"));
        assert!(decision_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(writer_plan.target_count, 1);
        assert_eq!(writer_plan.request_count, 1);
        assert_eq!(writer_plan.ready_plan_count, 1);
        assert!(writer_plan.writer_plan_ready);
        assert!(writer_plan.explicit_writer_invocation_required);
        assert!(writer_plan.rollback_required);
        assert!(writer_plan.experiment_required);
        assert!(!writer_plan.admission_write_authorized);
        assert!(writer_plan_json.contains(
            "\"schema\":\"self_improve_proposal_memory_admission_writer_plan_report_v1\""
        ));
        assert!(writer_plan_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_writer_plan\""));
        assert!(writer_plan_json.contains("\"writer_plan_ready\":true"));
        assert!(writer_plan_json.contains("\"ready_plan_count\":1"));
        assert!(writer_plan_json.contains("\"admission_write_authorized\":false"));
        assert!(
            writer_plan_json
                .contains("\"planned_admission_action\":\"stage_evidence_backed_memory_write_for_explicit_writer\"")
        );
        assert!(writer_plan_json.contains("\"rollback_anchor_ids\""));
        assert!(writer_plan_json.contains("\"memory_store_write_allowed\":false"));
        assert!(writer_plan_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(writer_dry_run.target_count, 1);
        assert_eq!(writer_dry_run.request_count, 1);
        assert_eq!(writer_dry_run.dry_run_item_count, 1);
        assert_eq!(writer_dry_run.ready_dry_run_count, 1);
        assert!(writer_dry_run.dry_run_ready);
        assert!(writer_dry_run.explicit_writer_invocation_required);
        assert!(writer_dry_run.dry_run_required);
        assert!(writer_dry_run.rollback_required);
        assert!(writer_dry_run.experiment_required);
        assert!(!writer_dry_run.admission_write_authorized);
        assert!(writer_dry_run_json.contains(
            "\"schema\":\"self_improve_proposal_memory_admission_writer_dry_run_report_v1\""
        ));
        assert!(writer_dry_run_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_writer_dry_run\""));
        assert!(writer_dry_run_json.contains("\"dry_run_ready\":true"));
        assert!(writer_dry_run_json.contains("\"ready_dry_run_count\":1"));
        assert!(writer_dry_run_json.contains("\"dry_run_required\":true"));
        assert!(
            writer_dry_run_json
                .contains("\"planned_writer_action\":\"dry_run_explicit_memory_admission_writer\"")
        );
        assert!(writer_dry_run_json.contains("\"write_authorized\":false"));
        assert!(writer_dry_run_json.contains("\"admission_write_authorized\":false"));
        assert!(writer_dry_run_json.contains("\"memory_store_write_allowed\":false"));
        assert!(writer_dry_run_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(writer_dry_run_receipt.target_count, 1);
        assert_eq!(writer_dry_run_receipt.request_count, 1);
        assert_eq!(writer_dry_run_receipt.dry_run_item_count, 1);
        assert_eq!(writer_dry_run_receipt.receipt_item_count, 1);
        assert_eq!(writer_dry_run_receipt.succeeded_receipt_count, 1);
        assert!(writer_dry_run_receipt.dry_run_receipt_ready);
        assert!(writer_dry_run_receipt.explicit_writer_invocation_required);
        assert!(!writer_dry_run_receipt.commit_allowed);
        assert!(writer_dry_run_receipt.rollback_required);
        assert!(!writer_dry_run_receipt.admission_write_authorized);
        assert!(writer_dry_run_receipt_json.contains(
            "\"schema\":\"self_improve_proposal_memory_admission_writer_dry_run_receipt_report_v1\""
        ));
        assert!(writer_dry_run_receipt_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_writer_dry_run_receipt\""));
        assert!(writer_dry_run_receipt_json.contains("\"dry_run_receipt_ready\":true"));
        assert!(writer_dry_run_receipt_json.contains("\"succeeded_receipt_count\":1"));
        assert!(writer_dry_run_receipt_json.contains("\"commit_allowed\":false"));
        assert!(writer_dry_run_receipt_json.contains("\"dry_run_succeeded\":true"));
        assert!(writer_dry_run_receipt_json.contains("\"preview_memory_record_id\""));
        assert!(writer_dry_run_receipt_json.contains("\"idempotency_key\""));
        assert!(writer_dry_run_receipt_json.contains("\"content_digest\":\"fnv1a64:"));
        assert!(writer_dry_run_receipt_json.contains(
            "\"planned_writer_action\":\"record_explicit_memory_admission_writer_dry_run_receipt\""
        ));
        assert!(writer_dry_run_receipt_json.contains("\"write_authorized\":false"));
        assert!(writer_dry_run_receipt_json.contains("\"admission_write_authorized\":false"));
        assert!(writer_dry_run_receipt_json.contains("\"memory_store_write_allowed\":false"));
        assert!(writer_dry_run_receipt_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(commit_record_stage.target_count, 1);
        assert_eq!(commit_record_stage.request_count, 1);
        assert_eq!(commit_record_stage.receipt_item_count, 1);
        assert_eq!(commit_record_stage.commit_record_item_count, 1);
        assert_eq!(commit_record_stage.staged_commit_record_count, 1);
        assert!(commit_record_stage.commit_record_stage_ready);
        assert!(commit_record_stage.explicit_writer_invocation_required);
        assert!(!commit_record_stage.commit_allowed);
        assert!(!commit_record_stage.admission_write_authorized);
        assert!(commit_record_stage_json.contains(
            "\"schema\":\"self_improve_proposal_memory_admission_commit_record_stage_report_v1\""
        ));
        assert!(commit_record_stage_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_record_stage\""));
        assert!(commit_record_stage_json.contains("\"commit_record_stage_ready\":true"));
        assert!(commit_record_stage_json.contains("\"staged_commit_record_count\":1"));
        assert!(commit_record_stage_json.contains("\"commit_record_staged\":true"));
        assert!(commit_record_stage_json.contains("\"staged_commit_record_id\":\"memory-admission-commit-stage:selfimprover392helpercontractupd\""));
        assert!(commit_record_stage_json.contains(
            "\"planned_writer_action\":\"stage_memory_admission_commit_record_for_explicit_commit\""
        ));
        assert!(commit_record_stage_json.contains("\"commit_allowed\":false"));
        assert!(commit_record_stage_json.contains("\"write_authorized\":false"));
        assert!(commit_record_stage_json.contains("\"admission_write_authorized\":false"));
        assert!(commit_record_stage_json.contains("\"memory_store_write_allowed\":false"));
        assert!(commit_record_stage_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(commit_approval_request.target_count, 1);
        assert_eq!(commit_approval_request.request_count, 1);
        assert_eq!(commit_approval_request.commit_record_item_count, 1);
        assert_eq!(commit_approval_request.approval_request_item_count, 1);
        assert_eq!(commit_approval_request.requested_commit_approval_count, 1);
        assert_eq!(commit_approval_request.blocked_count, 0);
        assert!(commit_approval_request.commit_approval_request_ready);
        assert!(commit_approval_request.explicit_commit_approval_required);
        assert!(!commit_approval_request.commit_allowed);
        assert!(!commit_approval_request.admission_write_authorized);
        assert!(commit_approval_request_json.contains(
            "\"schema\":\"self_improve_proposal_memory_admission_commit_approval_request_report_v1\""
        ));
        assert!(commit_approval_request_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_approval_request\""));
        assert!(commit_approval_request_json.contains("\"commit_approval_request_ready\":true"));
        assert!(commit_approval_request_json.contains("\"requested_commit_approval_count\":1"));
        assert!(commit_approval_request_json.contains("\"commit_approval_requested\":true"));
        assert!(commit_approval_request_json.contains("\"approval_request_id\":\"memory-admission-commit-approval-request:selfimprover392helpercontractupd\""));
        assert!(commit_approval_request_json.contains(
            "\"planned_writer_action\":\"request_explicit_memory_admission_commit_approval\""
        ));
        assert!(commit_approval_request_json.contains("\"commit_allowed\":false"));
        assert!(commit_approval_request_json.contains("\"write_authorized\":false"));
        assert!(commit_approval_request_json.contains("\"admission_write_authorized\":false"));
        assert!(commit_approval_request_json.contains("\"memory_store_write_allowed\":false"));
        assert!(commit_approval_request_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(commit_approval_decision.target_count, 1);
        assert_eq!(commit_approval_decision.request_count, 1);
        assert_eq!(commit_approval_decision.approval_request_item_count, 1);
        assert_eq!(commit_approval_decision.approval_decision_item_count, 1);
        assert_eq!(commit_approval_decision.recorded_approval_decision_count, 1);
        assert_eq!(commit_approval_decision.approved_commit_count, 0);
        assert_eq!(commit_approval_decision.pending_approval_count, 1);
        assert_eq!(commit_approval_decision.blocked_count, 0);
        assert!(commit_approval_decision.commit_approval_decision_ready);
        assert!(commit_approval_decision.explicit_commit_approval_required);
        assert!(!commit_approval_decision.commit_allowed);
        assert!(!commit_approval_decision.admission_write_authorized);
        assert!(commit_approval_decision_json.contains(
            "\"schema\":\"self_improve_proposal_memory_admission_commit_approval_decision_report_v1\""
        ));
        assert!(commit_approval_decision_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_approval_decision\""));
        assert!(commit_approval_decision_json.contains("\"commit_approval_decision_ready\":true"));
        assert!(commit_approval_decision_json.contains("\"recorded_approval_decision_count\":1"));
        assert!(commit_approval_decision_json.contains("\"approved_commit_count\":0"));
        assert!(commit_approval_decision_json.contains("\"pending_approval_count\":1"));
        assert!(
            commit_approval_decision_json.contains("\"commit_approval_decision_recorded\":true")
        );
        assert!(commit_approval_decision_json.contains("\"commit_approval_granted\":false"));
        assert!(
            commit_approval_decision_json
                .contains("\"approval_decision\":\"pending_explicit_commit_approval\"")
        );
        assert!(commit_approval_decision_json.contains("\"approval_decision_id\":\"memory-admission-commit-approval-decision:selfimprover392helpercontractupd\""));
        assert!(commit_approval_decision_json.contains(
            "\"planned_writer_action\":\"await_explicit_memory_admission_commit_approval\""
        ));
        assert!(commit_approval_decision_json.contains("\"commit_allowed\":false"));
        assert!(commit_approval_decision_json.contains("\"write_authorized\":false"));
        assert!(commit_approval_decision_json.contains("\"admission_write_authorized\":false"));
        assert!(commit_approval_decision_json.contains("\"memory_store_write_allowed\":false"));
        assert!(commit_approval_decision_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(commit_approval_review.target_count, 1);
        assert_eq!(commit_approval_review.request_count, 1);
        assert_eq!(commit_approval_review.approval_request_item_count, 1);
        assert_eq!(commit_approval_review.approval_decision_item_count, 1);
        assert_eq!(commit_approval_review.review_packet_item_count, 1);
        assert_eq!(commit_approval_review.ready_review_packet_count, 1);
        assert_eq!(commit_approval_review.pending_approval_count, 1);
        assert_eq!(commit_approval_review.blocked_count, 0);
        assert!(commit_approval_review.approval_review_packet_ready);
        assert!(commit_approval_review.explicit_operator_approval_required);
        assert!(!commit_approval_review.commit_allowed);
        assert!(!commit_approval_review.admission_write_authorized);
        assert!(commit_approval_review_json.contains(
            "\"schema\":\"self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1\""
        ));
        assert!(commit_approval_review_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_commit_approval_review_packet\""));
        assert!(commit_approval_review_json.contains("\"approval_review_packet_ready\":true"));
        assert!(commit_approval_review_json.contains("\"ready_review_packet_count\":1"));
        assert!(
            commit_approval_review_json.contains(
                "\"approval_review_packet_id\":\"memory-admission-approval-review:selfimprover392helpercontractupd\""
            )
        );
        assert!(commit_approval_review_json
            .contains("\"approval_token\":\"approve-memory-admission:selfimprover392helpercontractupd:fnv1a64-"));
        assert!(commit_approval_review_json
            .contains("\"rejection_token\":\"reject-memory-admission:selfimprover392helpercontractupd:fnv1a64-"));
        assert!(
            commit_approval_review_json
                .contains("\"planned_operator_action\":\"review_pending_memory_admission_commit_before_explicit_approval\"")
        );
        assert!(commit_approval_review_json.contains(
            "\"operator_checklist\":[\"verify validation_required=true and rollback_required=true\""
        ));
        assert!(commit_approval_review_json.contains("\"commit_allowed\":false"));
        assert!(commit_approval_review_json.contains("\"write_authorized\":false"));
        assert!(commit_approval_review_json.contains("\"admission_write_authorized\":false"));
        assert!(commit_approval_review_json.contains("\"memory_store_write_allowed\":false"));
        assert!(commit_approval_review_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(memory_reflection_usefulness.target_count, 1);
        assert_eq!(memory_reflection_usefulness.projected_report_count, 1);
        assert_eq!(
            memory_reflection_usefulness.accepted_memory_admission_count,
            0
        );
        assert_eq!(memory_reflection_usefulness.quarantined_candidate_count, 1);
        assert_eq!(memory_reflection_usefulness.review_packet_item_count, 1);
        assert_eq!(memory_reflection_usefulness.useful_reflection_item_count, 1);
        assert_eq!(
            memory_reflection_usefulness.pending_operator_approval_count,
            1
        );
        assert_eq!(memory_reflection_usefulness.blocked_count, 0);
        assert_eq!(memory_reflection_usefulness.wasted_compute_guard_count, 1);
        assert_eq!(memory_reflection_usefulness.adapter_safe_count, 1);
        assert!(memory_reflection_usefulness.reflection_usefulness_ready);
        assert!(!memory_reflection_usefulness.commit_allowed);
        assert!(!memory_reflection_usefulness.admission_write_authorized);
        assert!(memory_reflection_usefulness_json.contains(
            "\"schema\":\"self_improve_proposal_memory_reflection_usefulness_report_v1\""
        ));
        assert!(memory_reflection_usefulness_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_reflection_usefulness\""));
        assert!(memory_reflection_usefulness_json.contains("\"reflection_usefulness_ready\":true"));
        assert!(memory_reflection_usefulness_json.contains("\"useful_reflection_item_count\":1"));
        assert!(memory_reflection_usefulness_json.contains("\"wasted_compute_guard_count\":1"));
        assert!(memory_reflection_usefulness_json.contains("\"adapter_safe_count\":1"));
        assert!(memory_reflection_usefulness_json.contains(
            "\"reflection_status\":\"useful_pending_operator_memory_admission_approval\""
        ));
        assert!(memory_reflection_usefulness_json.contains(
            "\"reuse_recommendation\":\"reuse_as_closed_action_reflection_after_operator_approval\""
        ));
        assert!(memory_reflection_usefulness_json.contains("\"wasted_compute_guard\":true"));
        assert!(
            memory_reflection_usefulness_json
                .contains("\"adapter_safety_status\":\"adapter_safe_no_writes\"")
        );
        assert!(memory_reflection_usefulness_json.contains("\"pending_operator_approval\":true"));
        assert!(memory_reflection_usefulness_json.contains("\"closed_action_confirmed\":true"));
        assert!(memory_reflection_usefulness_json.contains("\"commit_allowed\":false"));
        assert!(memory_reflection_usefulness_json.contains("\"admission_write_authorized\":false"));
        assert!(memory_reflection_usefulness_json.contains("\"memory_store_write_allowed\":false"));
        assert!(memory_reflection_usefulness_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(memory_reflection_dedupe_cluster.target_count, 1);
        assert_eq!(
            memory_reflection_dedupe_cluster.useful_reflection_item_count,
            1
        );
        assert_eq!(memory_reflection_dedupe_cluster.reflection_cluster_count, 1);
        assert_eq!(memory_reflection_dedupe_cluster.duplicate_cluster_count, 0);
        assert_eq!(
            memory_reflection_dedupe_cluster.duplicate_reflection_item_count,
            0
        );
        assert_eq!(
            memory_reflection_dedupe_cluster.wasted_compute_guard_count,
            1
        );
        assert_eq!(
            memory_reflection_dedupe_cluster.pending_operator_approval_count,
            1
        );
        assert_eq!(memory_reflection_dedupe_cluster.adapter_safe_count, 1);
        assert!(memory_reflection_dedupe_cluster.reflection_dedupe_ready);
        assert!(!memory_reflection_dedupe_cluster.commit_allowed);
        assert!(!memory_reflection_dedupe_cluster.admission_write_authorized);
        assert!(memory_reflection_dedupe_cluster_json.contains(
            "\"schema\":\"self_improve_proposal_memory_reflection_dedupe_cluster_report_v1\""
        ));
        assert!(memory_reflection_dedupe_cluster_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_reflection_dedupe_cluster\""));
        assert!(memory_reflection_dedupe_cluster_json.contains("\"reflection_dedupe_ready\":true"));
        assert!(memory_reflection_dedupe_cluster_json.contains("\"reflection_cluster_count\":1"));
        assert!(memory_reflection_dedupe_cluster_json.contains("\"duplicate_cluster_count\":0"));
        assert!(
            memory_reflection_dedupe_cluster_json.contains("\"duplicate_reflection_item_count\":0")
        );
        assert!(memory_reflection_dedupe_cluster_json.contains(
            "\"dedupe_recommendation\":\"keep_single_reflection_cluster_for_future_reuse\""
        ));
        assert!(memory_reflection_dedupe_cluster_json.contains("\"commit_allowed\":false"));
        assert!(
            memory_reflection_dedupe_cluster_json.contains("\"admission_write_authorized\":false")
        );
        assert!(
            memory_reflection_dedupe_cluster_json.contains("\"memory_store_write_allowed\":false")
        );
        assert!(memory_reflection_dedupe_cluster_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(memory_reflection_reuse_plan.target_count, 1);
        assert_eq!(memory_reflection_reuse_plan.reflection_cluster_count, 1);
        assert_eq!(memory_reflection_reuse_plan.plan_item_count, 1);
        assert_eq!(memory_reflection_reuse_plan.ready_reuse_plan_count, 1);
        assert_eq!(memory_reflection_reuse_plan.duplicate_cluster_count, 0);
        assert_eq!(
            memory_reflection_reuse_plan.duplicate_reflection_item_count,
            0
        );
        assert_eq!(
            memory_reflection_reuse_plan.projected_saved_reflection_count,
            0
        );
        assert!(memory_reflection_reuse_plan.reflection_reuse_plan_ready);
        assert!(!memory_reflection_reuse_plan.commit_allowed);
        assert!(!memory_reflection_reuse_plan.admission_write_authorized);
        assert!(memory_reflection_reuse_plan_json.contains(
            "\"schema\":\"self_improve_proposal_memory_reflection_reuse_plan_report_v1\""
        ));
        assert!(memory_reflection_reuse_plan_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_reflection_reuse_plan\""));
        assert!(memory_reflection_reuse_plan_json.contains("\"reflection_reuse_plan_ready\":true"));
        assert!(memory_reflection_reuse_plan_json.contains("\"plan_item_count\":1"));
        assert!(memory_reflection_reuse_plan_json.contains("\"ready_reuse_plan_count\":1"));
        assert!(
            memory_reflection_reuse_plan_json.contains("\"projected_saved_reflection_count\":0")
        );
        assert!(memory_reflection_reuse_plan_json.contains(
            "\"planned_reuse_action\":\"cache_representative_reflection_for_future_reuse_match\""
        ));
        assert!(memory_reflection_reuse_plan_json.contains("\"commit_allowed\":false"));
        assert!(memory_reflection_reuse_plan_json.contains("\"admission_write_authorized\":false"));
        assert!(memory_reflection_reuse_plan_json.contains("\"memory_store_write_allowed\":false"));
        assert!(memory_reflection_reuse_plan_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(memory_reflection_reuse_preflight.target_count, 1);
        assert_eq!(memory_reflection_reuse_preflight.plan_item_count, 1);
        assert_eq!(memory_reflection_reuse_preflight.ready_reuse_plan_count, 1);
        assert_eq!(memory_reflection_reuse_preflight.preflight_item_count, 1);
        assert_eq!(
            memory_reflection_reuse_preflight.preflight_passed_item_count,
            0
        );
        assert_eq!(memory_reflection_reuse_preflight.blocked_item_count, 1);
        assert_eq!(
            memory_reflection_reuse_preflight.projected_saved_reflection_count,
            0
        );
        assert_eq!(
            memory_reflection_reuse_preflight.projected_model_call_skip_count,
            0
        );
        assert!(!memory_reflection_reuse_preflight.reuse_preflight_passed);
        assert!(!memory_reflection_reuse_preflight.commit_allowed);
        assert!(!memory_reflection_reuse_preflight.admission_write_authorized);
        assert!(!memory_reflection_reuse_preflight.model_call_skip_authorized);
        assert!(!memory_reflection_reuse_preflight.reflection_reuse_execution_authorized);
        assert!(memory_reflection_reuse_preflight_json.contains(
            "\"schema\":\"self_improve_proposal_memory_reflection_reuse_preflight_report_v1\""
        ));
        assert!(memory_reflection_reuse_preflight_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_reflection_reuse_preflight\""));
        assert!(
            memory_reflection_reuse_preflight_json.contains("\"reuse_preflight_passed\":false")
        );
        assert!(memory_reflection_reuse_preflight_json.contains("\"preflight_item_count\":1"));
        assert!(
            memory_reflection_reuse_preflight_json.contains("\"preflight_passed_item_count\":0")
        );
        assert!(memory_reflection_reuse_preflight_json.contains("\"blocked_item_count\":1"));
        assert!(
            memory_reflection_reuse_preflight_json
                .contains("\"projected_model_call_skip_count\":0")
        );
        assert!(
            memory_reflection_reuse_preflight_json.contains("\"model_call_skip_authorized\":false")
        );
        assert!(
            memory_reflection_reuse_preflight_json
                .contains("\"reflection_reuse_execution_authorized\":false")
        );
        assert!(
            memory_reflection_reuse_preflight_json
                .contains("\"reflection reuse preflight has no projected model-call savings\"")
        );
        assert!(memory_reflection_reuse_preflight_json.contains("\"commit_allowed\":false"));
        assert!(
            memory_reflection_reuse_preflight_json.contains("\"admission_write_authorized\":false")
        );
        assert!(
            memory_reflection_reuse_preflight_json.contains("\"memory_store_write_allowed\":false")
        );
        assert!(memory_reflection_reuse_preflight_json.contains("\"ndkv_write_allowed\":false"));
        assert_eq!(memory_approval_token_intake_preview.target_count, 1);
        assert_eq!(
            memory_approval_token_intake_preview.review_packet_item_count,
            1
        );
        assert_eq!(
            memory_approval_token_intake_preview.useful_reflection_item_count,
            1
        );
        assert_eq!(memory_approval_token_intake_preview.intake_item_count, 1);
        assert_eq!(memory_approval_token_intake_preview.ready_intake_count, 1);
        assert_eq!(
            memory_approval_token_intake_preview.pending_operator_token_count,
            1
        );
        assert_eq!(memory_approval_token_intake_preview.blocked_count, 0);
        assert_eq!(
            memory_approval_token_intake_preview.approval_token_present_count,
            1
        );
        assert_eq!(
            memory_approval_token_intake_preview.rejection_token_present_count,
            1
        );
        assert!(memory_approval_token_intake_preview.approval_token_intake_ready);
        assert!(!memory_approval_token_intake_preview.commit_allowed);
        assert!(!memory_approval_token_intake_preview.admission_write_authorized);
        assert!(memory_approval_token_intake_preview_json.contains(
            "\"schema\":\"self_improve_proposal_memory_admission_operator_approval_token_intake_preview_report_v1\""
        ));
        assert!(memory_approval_token_intake_preview_json.contains("\"consumer_surface\":\"evolution_loop_report_only_self_improve_memory_admission_operator_approval_token_intake_preview\""));
        assert!(
            memory_approval_token_intake_preview_json
                .contains("\"approval_token_intake_ready\":true")
        );
        assert!(memory_approval_token_intake_preview_json.contains("\"ready_intake_count\":1"));
        assert!(
            memory_approval_token_intake_preview_json
                .contains("\"pending_operator_token_count\":1")
        );
        assert!(
            memory_approval_token_intake_preview_json
                .contains("\"approval_token_present_count\":1")
        );
        assert!(
            memory_approval_token_intake_preview_json
                .contains("\"rejection_token_present_count\":1")
        );
        assert!(memory_approval_token_intake_preview_json.contains("\"token_intake_ready\":true"));
        assert!(
            memory_approval_token_intake_preview_json
                .contains("\"intake_status\":\"ready_for_explicit_operator_token_review\"")
        );
        assert!(memory_approval_token_intake_preview_json.contains(
            "\"planned_operator_action\":\"operator_may_choose_approval_or_rejection_token_for_next_dry_run\""
        ));
        assert!(
            memory_approval_token_intake_preview_json
                .contains("\"useful_reflection_confirmed\":true")
        );
        assert!(
            memory_approval_token_intake_preview_json
                .contains("\"approval_review_packet_ready\":true")
        );
        assert!(memory_approval_token_intake_preview_json.contains("\"commit_allowed\":false"));
        assert!(memory_approval_token_intake_preview_json.contains("\"write_authorized\":false"));
        assert!(
            memory_approval_token_intake_preview_json
                .contains("\"admission_write_authorized\":false")
        );
        assert!(
            memory_approval_token_intake_preview_json
                .contains("\"memory_store_write_allowed\":false")
        );
        assert!(memory_approval_token_intake_preview_json.contains("\"ndkv_write_allowed\":false"));
    }

    #[test]
    fn action_closure_report_keeps_unimplemented_targets_open() {
        let text = "{\"round\":31,\"case\":\"advisory-proposal\",\"success\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r31-advisory\\\",\\\"source_round\\\":31,\\\"evidence_id\\\":\\\"review:r31\\\",\\\"suggested_action\\\":\\\"convert advisory proposal to business evidence\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"proposed\\\"}}\"}\n";

        let artifact = from_ledger_text(text);
        let report = artifact.action_closure_report();

        assert_eq!(report.target_count, 1);
        assert_eq!(report.closed_target_count, 0);
        assert_eq!(report.open_target_count, 1);
        assert!(!report.first_target_closed);
        assert_eq!(report.first_target_closure_kind, None);
    }

    #[test]
    fn projects_review_contract_change_request_when_final_proposal_is_absent() {
        let text = "{\"round\":27,\"case\":\"contract-proposal\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"project self-improve admission evidence\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";

        let artifact = from_ledger_text(text);
        let proposal = artifact.proposals.first().unwrap();
        let json = artifact.report_json();

        assert_eq!(proposal.source_round, Some(27));
        assert_eq!(
            proposal.suggested_action,
            "project self-improve admission evidence"
        );
        assert_eq!(proposal.validation_command_safety, "safe");
        assert_eq!(proposal.source_admission_status.as_deref(), Some("passed"));
        assert!(json.contains(
            "\"validation\":{\"command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\""
        ));
        assert!(json.contains("\"side_effects_allowed\":false"));
        assert!(json.contains("\"evidence_backed_business_improvement\":false"));
        assert!(json.contains("\"advisory_only\":true"));
    }

    #[test]
    fn filters_noop_review_change_request_after_successful_validation() {
        let text = "{\"round\":403,\"case\":\"noop-review\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"None\",\"change_request\":\"None, as the validation passed and no code changes are required\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";

        let artifact = from_ledger_text(text);
        let assignment = artifact.acceptance_action_assignment();
        let closure = artifact.action_closure_report();
        let readiness = artifact.memory_admission_readiness_report();
        let request = artifact.memory_admission_request_report();
        let action_assignment_json = option_action_assignment_report_json(Some(&artifact));

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(!assignment.action_required);
        assert_eq!(assignment.target_count, 0);
        assert_eq!(closure.open_target_count, 0);
        assert_eq!(readiness.blocked_count, 0);
        assert_eq!(request.blocked_count, 0);
        assert!(action_assignment_json.contains("\"target_count\":0"));
        assert!(action_assignment_json.contains("\"first_target\":null"));
        assert!(!action_assignment_json.contains("None, as the validation passed"));
    }

    #[test]
    fn filters_explicit_noop_final_json_change_request() {
        let text = "{\"round\":404,\"case\":\"noop-final-json\",\"success\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r404-noop\\\",\\\"source_round\\\":404,\\\"evidence_id\\\":\\\"review:r404\\\",\\\"change_request\\\":\\\"No changes required, validation passed\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"proposed\\\"}}\"}\n";

        let artifact = from_ledger_text(text);
        let json = artifact.report_json();

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(json.contains("\"projected_candidate_count\":0"));
        assert!(!json.contains("r404-noop"));
    }

    #[test]
    fn filters_primary_answer_noop_review_change_request() {
        let text = "{\"round\":411,\"case\":\"noop-review\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"No change suggested in the primary_answer for this round.\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";

        let artifact = from_ledger_text(text);
        let json = artifact.report_json();

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(json.contains("\"projected_candidate_count\":0"));
        assert!(!json.contains("nochangesuggestedinthepr"));
    }

    #[test]
    fn filters_primary_answer_noop_final_json_suggested_action() {
        let text = "{\"round\":411,\"case\":\"noop-final-json\",\"success\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r78-generic-noop\\\",\\\"source_round\\\":411,\\\"evidence_id\\\":\\\"review:r411\\\",\\\"suggested_action\\\":\\\"change_request: `No change suggested in the primary_answer for this round.`\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"proposed\\\"}}\"}\n";

        let artifact = from_ledger_text(text);
        let json = artifact.report_json();

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(json.contains("\"projected_candidate_count\":0"));
        assert!(!json.contains("r78-generic-noop"));
    }

    #[test]
    fn filters_generic_small_next_change_placeholder() {
        let text = "{\"round\":412,\"case\":\"generic-review\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"Small next change grounded in the same evidence\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";

        let artifact = from_ledger_text(text);
        let json = artifact.report_json();

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(json.contains("\"projected_candidate_count\":0"));
        assert!(!json.contains("smallnextchangegroundedi"));
    }

    #[test]
    fn filters_negative_small_next_change_placeholder() {
        let text = "{\"round\":414,\"case\":\"negative-generic-review\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"No small next change is grounded in the same evidence; the previous round already identified a change request regarding `--no-fail-fast`.\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";

        let artifact = from_ledger_text(text);
        let json = artifact.report_json();

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(json.contains("\"projected_candidate_count\":0"));
        assert!(!json.contains("nosmallnextchangeisgroun"));
    }

    #[test]
    fn filters_generic_placeholder_final_json_suggested_action() {
        let text = "{\"round\":412,\"case\":\"generic-final-json\",\"success\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"configured\",\"validation_command_safety\":\"explicit\",\"final_preview\":\"{\\\"self_improve_proposal\\\":{\\\"proposal_id\\\":\\\"r78-generic-placeholder\\\",\\\"source_round\\\":412,\\\"evidence_id\\\":\\\"review:r412\\\",\\\"suggested_action\\\":\\\"Suggested action - Small next change grounded in the same evidence.\\\",\\\"validation_command\\\":\\\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\\\",\\\"admission_status\\\":\\\"proposed\\\"}}\"}\n";

        let artifact = from_ledger_text(text);
        let json = artifact.report_json();

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(json.contains("\"projected_candidate_count\":0"));
        assert!(!json.contains("r78-generic-placeholder"));
    }

    #[test]
    fn suggested_action_filter_handles_labels_and_punctuation_variants() {
        for value in [
            "change_request: `No change suggested in the primary_answer for this round.`",
            "Suggested action - No change suggested in the primary answer.",
            "review.change_request = Small next change grounded in the same evidence.",
            "proposal: `Small next change grounded in the same evidence`",
            "change_request: No small next change is grounded in the same evidence.",
            "No specific small next change grounded in the same evidence (No explicit instructions given)",
            "Add the missing `-Wno-unused-function` flag to the build process or clean up the unused functions in `src\\self_improve_proposal_artifact.rs` to address the warnings seen in `validation_stderr_tail`.",
            "Update the build flags for the `evolution-loop` in the configuration to include `-Wno-unused-function` to suppress compiler warnings, as suggested by the primary_answer.",
            "Update the `test-gate` stage configuration in the evolution loop to explicitly enable and tune the `test_gate_dynamic_upstream_buffer_v1` policy by modifying the relevant configuration file (primary_answer).",
        ] {
            assert!(!suggested_action_is_actionable(value));
        }
        assert!(suggested_action_is_actionable(
            "Update the validation_command to include --no-fail-fast"
        ));
        assert!(suggested_action_is_actionable(
            "Fix a regression where test_gate_dynamic_upstream_buffer_v1 is disabled"
        ));
    }

    #[test]
    fn filters_warning_suppression_review_noise() {
        let text = "{\"round\":418,\"case\":\"warning-suppression-a\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"The validation command output contains multiple compiler warnings (`warning: function ... is never used`) which, while not failing the build, indicate unused code paths in `src\\\\self_improve_proposal_artifact.rs`.\",\"change_request\":\"Add the missing `-Wno-unused-function` flag to the build process or clean up the unused functions in `src\\\\self_improve_proposal_artifact.rs` to address the warnings seen in `validation_stderr_tail`.\",\"verification\":\"cargo build -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}}}}\n\
{\"round\":419,\"case\":\"warning-suppression-b\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"The primary_answer references a `review.change_request` from round 418, but the provided structured_facts and previews do not contain this specific change request to verify the need for adding `-Wno-unused-function`.\",\"change_request\":\"Update the build flags for the `evolution-loop` in the configuration to include `-Wno-unused-function` to suppress compiler warnings, as suggested by the primary_answer.\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}}}}\n";

        let artifact = from_ledger_text(text);
        let json = artifact.report_json();

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(json.contains("\"projected_candidate_count\":0"));
        assert!(!json.contains("Wno-unused-function"));
        assert!(!json.contains("warning-suppression"));
    }

    #[test]
    fn filters_dynamic_buffer_config_review_noise() {
        let text = "{\"round\":422,\"case\":\"dynamic-buffer-config-noise\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"The proposed policy change involves modifying configuration to enable `test_gate_dynamic_upstream_buffer_v1`.\",\"change_request\":\"Update the `test-gate` stage configuration in the evolution loop to explicitly enable and tune the `test_gate_dynamic_upstream_buffer_v1` policy by modifying the relevant configuration file (primary_answer).\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}}},\"allocation_evidence\":[\"pool_stage_route[test-gate] selected_context_buffer_policy:strategy:test_gate_dynamic_upstream_buffer_v1 selected_context_sufficient:true\"]}\n";

        let artifact = from_ledger_text(text);
        let json = artifact.report_json();

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(artifact.proposals.is_empty());
        assert!(json.contains("\"projected_candidate_count\":0"));
        assert!(!json.contains("dynamic-buffer-config-noise"));
        assert!(!json.contains("test_gate_dynamic_upstream_buffer_v1"));
    }

    #[test]
    fn filtered_noop_review_does_not_block_closed_action_memory_admission() {
        let text = "{\"round\":392,\"case\":\"no-fail-fast-proposal\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"validation stops early\",\"change_request\":\"Update the `validation_command` to include `--no-fail-fast` to ensure comprehensive testing\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n\
{\"round\":403,\"case\":\"noop-review\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"None\",\"change_request\":\"None, as the validation passed and no code changes are required\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";

        let artifact = from_ledger_text(text);
        let readiness = artifact.memory_admission_readiness_report();
        let request = artifact.memory_admission_request_report();
        let decision = artifact.memory_admission_decision_report();
        let request_json = option_memory_admission_request_report_json(Some(&artifact));
        let decision_json = option_memory_admission_decision_report_json(Some(&artifact));

        assert_eq!(artifact.total_candidate_count, 1);
        assert_eq!(artifact.proposals.len(), 1);
        assert_eq!(readiness.ready_count, 1);
        assert_eq!(readiness.blocked_count, 0);
        assert_eq!(request.request_count, 1);
        assert_eq!(request.blocked_count, 0);
        assert!(request.all_ready_targets_requested);
        assert!(decision.admission_writer_preflight_passed);
        assert!(!decision.gate_blocked);
        assert!(artifact.memory_admission_writer_plan().writer_plan_ready);
        assert!(request_json.contains("\"request_count\":1"));
        assert!(request_json.contains("\"blocked_count\":0"));
        assert!(decision_json.contains("\"admission_writer_preflight_passed\":true"));
        assert!(decision_json.contains("\"gate_blocked\":false"));
        assert!(!decision_json.contains("noop-review"));
    }

    #[test]
    fn generic_noop_tail_does_not_block_closed_action_memory_admission() {
        let text = "{\"round\":405,\"case\":\"closed-a\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"validation stops early\",\"change_request\":\"Update the `validation_command` to include `--no-fail-fast` to ensure comprehensive testing\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n\
{\"round\":406,\"case\":\"closed-b\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"validation stops early\",\"change_request\":\"Update the `validation_command` to include `--no-fail-fast` as previously requested\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n\
{\"round\":411,\"case\":\"noop-review\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"No change suggested in the primary_answer for this round.\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n\
{\"round\":412,\"case\":\"generic-review\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"none\",\"change_request\":\"Small next change grounded in the same evidence\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml\"}}}}\n";

        let artifact = from_ledger_text(text);
        let readiness = artifact.memory_admission_readiness_report();
        let request = artifact.memory_admission_request_report();
        let decision = artifact.memory_admission_decision_report();
        let writer_plan = artifact.memory_admission_writer_plan();
        let writer_dry_run = artifact.memory_admission_writer_dry_run();
        let writer_dry_run_receipt = artifact.memory_admission_writer_dry_run_receipt();

        assert_eq!(artifact.total_candidate_count, 2);
        assert_eq!(artifact.proposals.len(), 2);
        assert_eq!(readiness.ready_count, 2);
        assert_eq!(readiness.blocked_count, 0);
        assert_eq!(request.request_count, 2);
        assert_eq!(request.blocked_count, 0);
        assert!(decision.admission_writer_preflight_passed);
        assert!(!decision.gate_blocked);
        assert_eq!(writer_plan.ready_plan_count, 2);
        assert!(writer_plan.writer_plan_ready);
        assert_eq!(writer_dry_run.ready_dry_run_count, 2);
        assert!(writer_dry_run.dry_run_ready);
        assert_eq!(writer_dry_run_receipt.succeeded_receipt_count, 2);
        assert!(writer_dry_run_receipt.dry_run_receipt_ready);
    }

    #[test]
    fn warning_suppression_tail_does_not_block_closed_action_memory_admission() {
        let text = "{\"round\":417,\"case\":\"closed-no-fail-fast\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"validation stops early\",\"change_request\":\"Modify the `validation_command` in `tools/evolution-loop/Cargo.toml` to include `--no-fail-fast` to prevent premature test failure propagation, following the guidance in the `primary_answer`.\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}}}}\n\
{\"round\":418,\"case\":\"warning-suppression-tail-a\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"The validation command output contains multiple compiler warnings.\",\"change_request\":\"Add the missing `-Wno-unused-function` flag to the build process or clean up the unused functions in `src\\\\self_improve_proposal_artifact.rs` to address the warnings seen in `validation_stderr_tail`.\",\"verification\":\"cargo build -q --manifest-path tools/evolution-loop/Cargo.toml\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}}}}\n\
{\"round\":419,\"case\":\"warning-suppression-tail-b\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"The previous warning suppression request is not grounded by structured facts.\",\"change_request\":\"Update the build flags for the `evolution-loop` in the configuration to include `-Wno-unused-function` to suppress compiler warnings, as suggested by the primary_answer.\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}}}}\n\
{\"round\":422,\"case\":\"dynamic-buffer-config-tail\",\"success\":true,\"self_improve_passed\":true,\"validation_checked\":true,\"validation_passed\":true,\"validation_command_source\":\"test-gate\",\"validation_command_safety\":\"safe\",\"validation_command_preview\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\",\"helper_stage_contract_by_role\":{\"review\":{\"fields\":{\"risk\":\"The proposed policy change involves modifying configuration to enable `test_gate_dynamic_upstream_buffer_v1`.\",\"change_request\":\"Update the `test-gate` stage configuration in the evolution loop to explicitly enable and tune the `test_gate_dynamic_upstream_buffer_v1` policy by modifying the relevant configuration file (primary_answer).\",\"verification\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}},\"test-gate\":{\"fields\":{\"validation_command\":\"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\\\evolution-loop-daemon-check\"}}},\"allocation_evidence\":[\"pool_stage_route[test-gate] selected_context_buffer_policy:strategy:test_gate_dynamic_upstream_buffer_v1 selected_context_sufficient:true\"]}\n";

        let artifact = from_ledger_text(text);
        let closure = artifact.action_closure_report();
        let readiness = artifact.memory_admission_readiness_report();
        let request = artifact.memory_admission_request_report();
        let decision = artifact.memory_admission_decision_report();
        let writer_plan = artifact.memory_admission_writer_plan();
        let writer_dry_run = artifact.memory_admission_writer_dry_run();
        let writer_dry_run_receipt = artifact.memory_admission_writer_dry_run_receipt();
        let commit_record_stage = artifact.memory_admission_commit_record_stage();
        let approval_request = artifact.memory_admission_commit_approval_request();
        let approval_decision = artifact.memory_admission_commit_approval_decision();

        assert_eq!(artifact.total_candidate_count, 1);
        assert_eq!(artifact.proposals.len(), 1);
        assert_eq!(closure.closed_target_count, 1);
        assert_eq!(closure.open_target_count, 0);
        assert_eq!(readiness.ready_count, 1);
        assert_eq!(readiness.blocked_count, 0);
        assert_eq!(request.request_count, 1);
        assert_eq!(request.blocked_count, 0);
        assert!(decision.admission_writer_preflight_passed);
        assert!(!decision.gate_blocked);
        assert!(writer_plan.writer_plan_ready);
        assert!(writer_dry_run.dry_run_ready);
        assert!(writer_dry_run_receipt.dry_run_receipt_ready);
        assert!(commit_record_stage.commit_record_stage_ready);
        assert!(approval_request.commit_approval_request_ready);
        assert!(approval_decision.commit_approval_decision_ready);
        assert_eq!(approval_decision.recorded_approval_decision_count, 1);
        assert_eq!(approval_decision.approved_commit_count, 0);
        assert_eq!(approval_decision.pending_approval_count, 1);
        assert!(!approval_decision.commit_allowed);
        assert!(!approval_decision.admission_write_authorized);
        assert!(!approval_decision.memory_store_write_allowed);
        assert!(!approval_decision.ndkv_write_allowed);
    }

    #[test]
    fn missing_proposals_keep_stable_report_only_surface() {
        let artifact = from_ledger_text("{\"round\":1,\"success\":true}\n");
        let json = artifact.report_json();

        assert_eq!(artifact.total_candidate_count, 0);
        assert!(json.contains("\"projected_candidate_count\":0"));
        assert!(json.contains("\"proposals\":[]"));
        assert!(json.contains("\"sends_prompt\":false"));
    }

    #[test]
    fn option_artifact_json_can_emit_unloaded_surface() {
        let json = option_artifact_json(None);

        assert!(json.contains("\"artifact_loaded\":false"));
        assert!(json.contains("\"candidate_only\":true"));
        assert!(json.contains("\"edits_files\":false"));
    }
}
