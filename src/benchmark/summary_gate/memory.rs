use super::super::summary::BenchmarkSummary;
use super::super::BenchmarkGate;
use super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(max_memory_governance_failures) = gate.max_memory_governance_failures {
        let memory_governance_failures = summary.memory_governance_evidence.failures.len();
        if memory_governance_failures > max_memory_governance_failures {
            failures.push(format!(
                "memory_governance_failures {} above maximum {}: {}",
                memory_governance_failures,
                max_memory_governance_failures,
                summary.memory_governance_evidence.failures.join("; ")
            ));
        }
    }

    if let Some(max_memory_feedback_evidence_failures) = gate.max_memory_feedback_evidence_failures
    {
        let memory_feedback_evidence_failures = summary.total_memory_feedback_evidence_failures();
        if memory_feedback_evidence_failures > max_memory_feedback_evidence_failures {
            failures.push(format!(
                "memory_feedback_evidence_failures {} above maximum {}: {}",
                memory_feedback_evidence_failures,
                max_memory_feedback_evidence_failures,
                summary
                    .reflection_evidence
                    .memory_feedback_failures
                    .join("; ")
            ));
        }
    }

    if let Some(min_memory_governance_cases) = gate.min_memory_governance_cases {
        let memory_governance_cases = summary.memory_governance_cases();
        if memory_governance_cases < min_memory_governance_cases {
            failures.push(format!(
                "memory_governance_cases {} below minimum {}",
                memory_governance_cases, min_memory_governance_cases
            ));
        }
    }

    if let Some(min_memory_governance_device_profiles) = gate.min_memory_governance_device_profiles
    {
        let memory_governance_device_profiles = summary.memory_governance_device_profiles();
        if memory_governance_device_profiles < min_memory_governance_device_profiles {
            failures.push(format!(
                "memory_governance_device_profiles {} below minimum {}",
                memory_governance_device_profiles, min_memory_governance_device_profiles
            ));
        }
    }

    if let Some(min_kv_fusion_cases) = gate.min_kv_fusion_cases {
        let observed = summary.kv_fusion_cases();
        if observed < min_kv_fusion_cases {
            failures.push(format!(
                "kv_fusion_cases {} below minimum {}",
                observed, min_kv_fusion_cases
            ));
        }
    }

    if let Some(min_kv_fusion_candidates) = gate.min_kv_fusion_candidates {
        let observed = summary.total_kv_fusion_candidates();
        if observed < min_kv_fusion_candidates {
            failures.push(format!(
                "kv_fusion_candidates {} below minimum {}",
                observed, min_kv_fusion_candidates
            ));
        }
    }

    if let Some(min_kv_fusion_saved_tokens) = gate.min_kv_fusion_saved_tokens {
        let observed = summary.total_kv_fusion_saved_tokens();
        if observed < min_kv_fusion_saved_tokens {
            failures.push(format!(
                "kv_fusion_saved_tokens {} below minimum {}",
                observed, min_kv_fusion_saved_tokens
            ));
        }
    }

    if let Some(max_kv_fusion_skipped) = gate.max_kv_fusion_skipped {
        let observed = summary.total_kv_fusion_skipped();
        if observed > max_kv_fusion_skipped {
            failures.push(format!(
                "kv_fusion_skipped {} above maximum {}",
                observed, max_kv_fusion_skipped
            ));
        }
    }

    if let Some(max_kv_fusion_held) = gate.max_kv_fusion_held {
        let observed = summary.total_kv_fusion_held();
        if observed > max_kv_fusion_held {
            failures.push(format!(
                "kv_fusion_held {} above maximum {}",
                observed, max_kv_fusion_held
            ));
        }
    }

    if let Some(max_kv_fusion_rejected) = gate.max_kv_fusion_rejected {
        let observed = summary.total_kv_fusion_rejected();
        if observed > max_kv_fusion_rejected {
            failures.push(format!(
                "kv_fusion_rejected {} above maximum {}",
                observed, max_kv_fusion_rejected
            ));
        }
    }

    if let Some(max_kv_fusion_approval_blocked) = gate.max_kv_fusion_approval_blocked {
        let observed = summary.total_kv_fusion_approval_blocked();
        if observed > max_kv_fusion_approval_blocked {
            failures.push(format!(
                "kv_fusion_approval_blocked {} above maximum {}",
                observed, max_kv_fusion_approval_blocked
            ));
        }
    }

    if let Some(min_memory_retention_activity_cases) = gate.min_memory_retention_activity_cases {
        let observed = summary.memory_governance_evidence.retention_activity_cases;
        if observed < min_memory_retention_activity_cases {
            failures.push(format!(
                "memory_retention_activity_cases {} below minimum {}",
                observed, min_memory_retention_activity_cases
            ));
        }
    }

    if let Some(min_memory_compaction_activity_cases) = gate.min_memory_compaction_activity_cases {
        let observed = summary.memory_governance_evidence.compaction_activity_cases;
        if observed < min_memory_compaction_activity_cases {
            failures.push(format!(
                "memory_compaction_activity_cases {} below minimum {}",
                observed, min_memory_compaction_activity_cases
            ));
        }
    }

    if let Some(min_memory_storage_benchmark_samples) = gate.min_memory_storage_benchmark_samples {
        let observed = summary.memory_storage_benchmark_samples();
        if observed < min_memory_storage_benchmark_samples {
            failures.push(format!(
                "memory_storage_benchmark_samples {} below minimum {}",
                observed, min_memory_storage_benchmark_samples
            ));
        }
    }

    if let Some(min_memory_storage_removed_entries) = gate.min_memory_storage_removed_entries {
        let observed = summary.total_memory_storage_entries_removed();
        if observed < min_memory_storage_removed_entries {
            failures.push(format!(
                "memory_storage_removed_entries {} below minimum {}",
                observed, min_memory_storage_removed_entries
            ));
        }
    }

    if let Some(min_memory_retrieval_latency_samples) = gate.min_memory_retrieval_latency_samples {
        let observed = summary.memory_retrieval_latency_samples();
        if observed < min_memory_retrieval_latency_samples {
            failures.push(format!(
                "memory_retrieval_latency_samples {} below minimum {}",
                observed, min_memory_retrieval_latency_samples
            ));
        }
    }

    if let Some(max_memory_retrieval_latency_avg_ms) = gate.max_memory_retrieval_latency_avg_ms {
        let observed = summary.average_memory_retrieval_latency_ms();
        if observed > max_memory_retrieval_latency_avg_ms {
            failures.push(format!(
                "memory_retrieval_latency_avg_ms {} above maximum {}",
                observed, max_memory_retrieval_latency_avg_ms
            ));
        }
    }

    if let Some(min_memory_retained_usefulness_abs_delta_milli) =
        gate.min_memory_retained_usefulness_abs_delta_milli
    {
        let observed = summary.memory_retained_usefulness_abs_delta_milli();
        if observed < min_memory_retained_usefulness_abs_delta_milli {
            failures.push(format!(
                "memory_retained_usefulness_abs_delta_milli {} below minimum {}",
                observed, min_memory_retained_usefulness_abs_delta_milli
            ));
        }
    }

    if let Some(min_reflection_issue_cases) = gate.min_reflection_issue_cases {
        let observed = summary.reflection_evidence.issue_cases;
        if observed < min_reflection_issue_cases {
            failures.push(format!(
                "reflection_issue_cases {} below minimum {}",
                observed, min_reflection_issue_cases
            ));
        }
    }

    if let Some(min_reflection_issues) = gate.min_reflection_issues {
        let observed = summary.reflection_evidence.total_issues;
        if observed < min_reflection_issues {
            failures.push(format!(
                "reflection_issues {} below minimum {}",
                observed, min_reflection_issues
            ));
        }
    }

    if let Some(min_critical_reflection_issue_cases) = gate.min_critical_reflection_issue_cases {
        let observed = summary.reflection_evidence.critical_issue_cases;
        if observed < min_critical_reflection_issue_cases {
            failures.push(format!(
                "critical_reflection_issue_cases {} below minimum {}",
                observed, min_critical_reflection_issue_cases
            ));
        }
    }

    if let Some(min_critical_reflection_issues) = gate.min_critical_reflection_issues {
        let observed = summary.reflection_evidence.total_critical_issues;
        if observed < min_critical_reflection_issues {
            failures.push(format!(
                "critical_reflection_issues {} below minimum {}",
                observed, min_critical_reflection_issues
            ));
        }
    }

    if let Some(min_revision_action_cases) = gate.min_revision_action_cases {
        let observed = summary.reflection_evidence.revision_action_cases;
        if observed < min_revision_action_cases {
            failures.push(format!(
                "revision_action_cases {} below minimum {}",
                observed, min_revision_action_cases
            ));
        }
    }

    if let Some(min_revision_actions) = gate.min_revision_actions {
        let observed = summary.reflection_evidence.total_revision_actions;
        if observed < min_revision_actions {
            failures.push(format!(
                "revision_actions {} below minimum {}",
                observed, min_revision_actions
            ));
        }
    }

    if let Some(min_reflection_issue_device_profiles) = gate.min_reflection_issue_device_profiles {
        let observed = summary.reflection_evidence.issue_device_profiles();
        if observed < min_reflection_issue_device_profiles {
            failures.push(format!(
                "reflection_issue_device_profiles {} below minimum {}",
                observed, min_reflection_issue_device_profiles
            ));
        }
    }

    if let Some(min_critical_reflection_issue_device_profiles) =
        gate.min_critical_reflection_issue_device_profiles
    {
        let observed = summary.reflection_evidence.critical_issue_device_profiles();
        if observed < min_critical_reflection_issue_device_profiles {
            failures.push(format!(
                "critical_reflection_issue_device_profiles {} below minimum {}",
                observed, min_critical_reflection_issue_device_profiles
            ));
        }
    }

    if let Some(min_revision_action_device_profiles) = gate.min_revision_action_device_profiles {
        let observed = summary
            .reflection_evidence
            .revision_action_device_profiles();
        if observed < min_revision_action_device_profiles {
            failures.push(format!(
                "revision_action_device_profiles {} below minimum {}",
                observed, min_revision_action_device_profiles
            ));
        }
    }
}
