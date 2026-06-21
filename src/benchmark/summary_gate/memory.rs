use super::super::BenchmarkGate;
use super::super::summary::BenchmarkSummary;
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
