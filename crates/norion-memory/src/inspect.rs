use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AdapterProjectionAudit, AdapterProjectionIssueSeverity, MemoryAdapter, MemoryAdapterCapability,
    MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryAdapterStatus, MemoryEvolutionAssessment,
    MemoryEvolutionLedger, MemoryReadinessReport, MemoryResult, RetentionMemoryEntry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryInspectionRiskSeverity {
    Info,
    Warning,
    Blocker,
}

impl MemoryInspectionRiskSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Blocker => "blocker",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryInspectionRisk {
    pub severity: MemoryInspectionRiskSeverity,
    pub code: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryVectorDimensions {
    pub dimensions: usize,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryInspectionSummary {
    pub id: String,
    pub key: String,
    pub vector_dimensions: usize,
    pub strength: f32,
    pub hits: u64,
    pub failures: u64,
    pub value_score: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemoryInspectionSnapshot {
    pub memory_count: usize,
    pub runtime_kv_memory_count: usize,
    pub experience_count: usize,
    pub kv_shard_count: usize,
    pub adapter_count: usize,
    pub unhealthy_adapter_count: usize,
    pub projection_issue_count: usize,
    pub projection_blocker_count: usize,
    pub readiness_missing_capability_count: usize,
    pub readiness_write_blocker_count: usize,
    pub evolution_blocker_count: usize,
    pub evolution_warning_count: usize,
    pub memory_vector_dimensions: Vec<MemoryVectorDimensions>,
    pub runtime_kv_vector_dimensions: Vec<MemoryVectorDimensions>,
    pub top_memories: Vec<MemoryInspectionSummary>,
    pub top_runtime_kv_memories: Vec<MemoryInspectionSummary>,
    pub risks: Vec<MemoryInspectionRisk>,
}

impl MemoryInspectionSnapshot {
    pub fn has_blockers(&self) -> bool {
        self.risks
            .iter()
            .any(|risk| risk.severity == MemoryInspectionRiskSeverity::Blocker)
    }

    pub fn risk_codes(&self) -> Vec<String> {
        self.risks
            .iter()
            .map(|risk| normalized_risk_code(&risk.code))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn risk_detail_codes(&self) -> Vec<String> {
        self.risks
            .iter()
            .map(|risk| {
                format!(
                    "{}:{}:{}",
                    risk.severity.as_str(),
                    normalized_risk_detail_code(&risk.code),
                    risk.count
                )
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn checklist_detail(&self) -> String {
        format!("inspection_risks={}", self.risks.len())
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_inspection memories={} runtime_kv_memories={} experiences={} kv_shards={} adapters={} unhealthy_adapters={} projection_issues={} projection_blockers={} readiness_missing_capabilities={} readiness_write_blockers={} evolution_blockers={} evolution_warnings={} vector_dimensions={} runtime_kv_vector_dimensions={} risks={} risk_codes={} detail_codes={}",
            self.memory_count,
            self.runtime_kv_memory_count,
            self.experience_count,
            self.kv_shard_count,
            self.adapter_count,
            self.unhealthy_adapter_count,
            self.projection_issue_count,
            self.projection_blocker_count,
            self.readiness_missing_capability_count,
            self.readiness_write_blocker_count,
            self.evolution_blocker_count,
            self.evolution_warning_count,
            format_vector_dimensions(&self.memory_vector_dimensions),
            format_vector_dimensions(&self.runtime_kv_vector_dimensions),
            format_risks(&self.risks),
            join_codes(self.risk_codes()),
            join_codes(self.risk_detail_codes()),
        )
    }
}

pub trait MemoryInspectionBuilder {
    #[allow(clippy::too_many_arguments)]
    fn build(
        &self,
        memory_entries: &[RetentionMemoryEntry],
        experience_count: usize,
        kv_shard_count: usize,
        adapters: &[MemoryAdapterStatus],
        projection_audit: Option<&AdapterProjectionAudit>,
        readiness: Option<&MemoryReadinessReport>,
        evolution_ledger: Option<&MemoryEvolutionLedger>,
        evolution_assessment: Option<&MemoryEvolutionAssessment>,
        limit: usize,
    ) -> MemoryInspectionSnapshot;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultMemoryInspectionBuilder;

impl MemoryAdapter for DefaultMemoryInspectionBuilder {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_memory_inspection_builder",
            vec![MemoryAdapterCapability::StateInspection],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl MemoryInspectionBuilder for DefaultMemoryInspectionBuilder {
    fn build(
        &self,
        memory_entries: &[RetentionMemoryEntry],
        experience_count: usize,
        kv_shard_count: usize,
        adapters: &[MemoryAdapterStatus],
        projection_audit: Option<&AdapterProjectionAudit>,
        readiness: Option<&MemoryReadinessReport>,
        evolution_ledger: Option<&MemoryEvolutionLedger>,
        evolution_assessment: Option<&MemoryEvolutionAssessment>,
        limit: usize,
    ) -> MemoryInspectionSnapshot {
        let limit = limit.max(1);
        let mut risks = Vec::new();
        let projection_issue_count = projection_audit
            .map(|audit| audit.issues.len())
            .unwrap_or_default();
        let projection_blocker_count = projection_audit
            .map(|audit| audit.blockers().len())
            .unwrap_or_default();
        if let Some(audit) = projection_audit {
            for issue in &audit.issues {
                push_risk(
                    &mut risks,
                    match issue.severity {
                        AdapterProjectionIssueSeverity::Warning => {
                            MemoryInspectionRiskSeverity::Warning
                        }
                        AdapterProjectionIssueSeverity::Blocker => {
                            MemoryInspectionRiskSeverity::Blocker
                        }
                    },
                    &format!("projection:{}", issue.code),
                );
            }
        }

        let readiness_missing_capability_count = readiness
            .map(|report| report.missing_capabilities.len())
            .unwrap_or_default();
        let readiness_write_blocker_count = readiness
            .map(|report| report.write_mode_blockers.len())
            .unwrap_or_default();
        if let Some(report) = readiness {
            for capability in &report.missing_capabilities {
                push_risk(
                    &mut risks,
                    MemoryInspectionRiskSeverity::Blocker,
                    &format!("readiness:missing:{}", capability.as_str()),
                );
            }
            for capability in &report.write_mode_blockers {
                push_risk(
                    &mut risks,
                    MemoryInspectionRiskSeverity::Blocker,
                    &format!("readiness:write_blocker:{}", capability.as_str()),
                );
            }
            for adapter in &report.unhealthy_adapters {
                push_risk(
                    &mut risks,
                    MemoryInspectionRiskSeverity::Warning,
                    &format!("readiness:unhealthy_adapter:{adapter}"),
                );
            }
        }

        let evolution_blocker_count = evolution_assessment
            .map(|assessment| assessment.blockers.len())
            .unwrap_or_default();
        let evolution_warning_count = evolution_assessment
            .map(|assessment| assessment.warnings.len())
            .unwrap_or_default();
        if let Some(assessment) = evolution_assessment {
            for blocker in &assessment.blockers {
                push_risk(
                    &mut risks,
                    MemoryInspectionRiskSeverity::Blocker,
                    &format!("evolution:{blocker}"),
                );
            }
            for warning in &assessment.warnings {
                push_risk(
                    &mut risks,
                    MemoryInspectionRiskSeverity::Warning,
                    &format!("evolution:{warning}"),
                );
            }
        }
        if let Some(ledger) = evolution_ledger
            && ledger.context_rot_items > 0
        {
            push_risk(
                &mut risks,
                MemoryInspectionRiskSeverity::Info,
                "evolution:context_rot_seen",
            );
        }
        risks.sort_by(|left, right| {
            right
                .severity
                .cmp(&left.severity)
                .then_with(|| left.code.cmp(&right.code))
        });

        MemoryInspectionSnapshot {
            memory_count: memory_entries.len(),
            runtime_kv_memory_count: memory_entries
                .iter()
                .filter(|entry| entry.key.starts_with("runtime_kv:"))
                .count(),
            experience_count,
            kv_shard_count,
            adapter_count: adapters.len(),
            unhealthy_adapter_count: adapters
                .iter()
                .filter(|adapter| !adapter.health.ready)
                .count(),
            projection_issue_count,
            projection_blocker_count,
            readiness_missing_capability_count,
            readiness_write_blocker_count,
            evolution_blocker_count,
            evolution_warning_count,
            memory_vector_dimensions: vector_dimensions(memory_entries.iter()),
            runtime_kv_vector_dimensions: vector_dimensions(
                memory_entries
                    .iter()
                    .filter(|entry| entry.key.starts_with("runtime_kv:")),
            ),
            top_memories: top_memory_summaries(
                memory_entries
                    .iter()
                    .filter(|entry| !entry.key.starts_with("runtime_kv:")),
                limit,
            ),
            top_runtime_kv_memories: top_memory_summaries(
                memory_entries
                    .iter()
                    .filter(|entry| entry.key.starts_with("runtime_kv:")),
                limit,
            ),
            risks,
        }
    }
}

fn vector_dimensions<'a>(
    entries: impl Iterator<Item = &'a RetentionMemoryEntry>,
) -> Vec<MemoryVectorDimensions> {
    let mut buckets = BTreeMap::<usize, usize>::new();
    for entry in entries {
        *buckets.entry(entry.vector.len()).or_insert(0) += 1;
    }
    buckets
        .into_iter()
        .map(|(dimensions, count)| MemoryVectorDimensions { dimensions, count })
        .collect()
}

fn top_memory_summaries<'a>(
    entries: impl Iterator<Item = &'a RetentionMemoryEntry>,
    limit: usize,
) -> Vec<MemoryInspectionSummary> {
    let mut scored = entries
        .map(|entry| (memory_value_score(entry), entry))
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.1.id.cmp(&right.1.id))
    });
    scored
        .into_iter()
        .take(limit)
        .map(|(value_score, entry)| MemoryInspectionSummary {
            id: entry.id.clone(),
            key: compact(&entry.key, 120),
            vector_dimensions: entry.vector.len(),
            strength: entry.strength,
            hits: entry.hits,
            failures: entry.failures,
            value_score,
        })
        .collect()
}

fn memory_value_score(entry: &RetentionMemoryEntry) -> f32 {
    entry.strength + entry.hits as f32 * 0.04 - entry.failures as f32 * 0.10
}

fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn push_risk(
    risks: &mut Vec<MemoryInspectionRisk>,
    severity: MemoryInspectionRiskSeverity,
    code: &str,
) {
    if let Some(existing) = risks.iter_mut().find(|risk| risk.code == code) {
        existing.count = existing.count.saturating_add(1);
        existing.severity = existing.severity.max(severity);
    } else {
        risks.push(MemoryInspectionRisk {
            severity,
            code: code.to_owned(),
            count: 1,
        });
    }
}

fn format_vector_dimensions(buckets: &[MemoryVectorDimensions]) -> String {
    if buckets.is_empty() {
        return "none".to_owned();
    }
    buckets
        .iter()
        .map(|bucket| format!("{}:{}", bucket.dimensions, bucket.count))
        .collect::<Vec<_>>()
        .join("|")
}

fn format_risks(risks: &[MemoryInspectionRisk]) -> String {
    if risks.is_empty() {
        return "none".to_owned();
    }
    risks
        .iter()
        .map(|risk| format!("{}:{}:{}", risk.severity.as_str(), risk.code, risk.count))
        .collect::<Vec<_>>()
        .join("|")
}

fn normalized_risk_code(code: &str) -> String {
    code.split_once('=')
        .map_or(code, |(prefix, _)| prefix)
        .to_owned()
}

fn normalized_risk_detail_code(code: &str) -> String {
    if let Some((prefix, value)) = code.split_once('=') {
        format!("{prefix}:{}", metric_value_code(value))
    } else {
        code.to_owned()
    }
}

fn metric_value_code(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
        .collect()
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AdapterProjectionAudit, AdapterProjectionIssue, AdapterWriteMode, MemoryAdapterDescriptor,
        MemoryConsumerProfile, MemoryEvolutionAssessment, MemoryEvolutionLedger,
        MemoryReadinessReport, MemoryServiceManifest, MemoryServiceRequirement,
    };

    fn entry(
        id: &str,
        key: &str,
        dims: usize,
        strength: f32,
        hits: u64,
        failures: u64,
    ) -> RetentionMemoryEntry {
        RetentionMemoryEntry::new(id, key, vec![0.1; dims], strength).with_feedback(hits, failures)
    }

    #[test]
    fn inspection_snapshot_summarizes_memory_shapes_and_top_entries() {
        let entries = vec![
            entry("semantic-low", "semantic:low", 2, 0.2, 0, 1),
            entry("semantic-high", "semantic:high", 4, 0.8, 3, 0),
            entry("runtime", "runtime_kv:block", 4, 0.7, 1, 0),
        ];

        let snapshot =
            DefaultMemoryInspectionBuilder.build(&entries, 7, 2, &[], None, None, None, None, 1);

        assert_eq!(snapshot.memory_count, 3);
        assert_eq!(snapshot.runtime_kv_memory_count, 1);
        assert_eq!(snapshot.experience_count, 7);
        assert_eq!(snapshot.kv_shard_count, 2);
        assert_eq!(
            snapshot.memory_vector_dimensions,
            vec![
                MemoryVectorDimensions {
                    dimensions: 2,
                    count: 1,
                },
                MemoryVectorDimensions {
                    dimensions: 4,
                    count: 2,
                },
            ]
        );
        assert_eq!(snapshot.top_memories[0].id, "semantic-high");
        assert_eq!(snapshot.top_runtime_kv_memories[0].id, "runtime");
    }

    #[test]
    fn inspection_snapshot_groups_projection_readiness_and_evolution_risks() {
        let projection = AdapterProjectionAudit {
            experience_count: 1,
            kv_shard_count: 0,
            issues: vec![
                AdapterProjectionIssue::warning(
                    Some("exp".to_owned()),
                    "missing_task_scope",
                    "missing task",
                ),
                AdapterProjectionIssue::blocker(
                    Some("exp".to_owned()),
                    "duplicate_experience_id",
                    "duplicate",
                ),
            ],
        };
        let readiness = MemoryReadinessReport {
            profile: MemoryConsumerProfile::Service,
            required_write_mode: AdapterWriteMode::IsolatedWrite,
            ready: false,
            adapter_statuses: Vec::new(),
            missing_capabilities: vec![MemoryAdapterCapability::KvSwap],
            write_mode_blockers: vec![MemoryAdapterCapability::DiskKvOffload],
            unhealthy_adapters: vec!["disk".to_owned()],
            warnings: Vec::new(),
            coverage: Vec::new(),
        };
        let evolution = MemoryEvolutionAssessment {
            allow_isolated_write: false,
            rollback_recommended: true,
            blockers: vec!["missing_replay_evidence".to_owned()],
            warnings: vec!["drift_rollbacks=1".to_owned()],
        };
        let ledger = MemoryEvolutionLedger {
            context_rot_items: 2,
            ..MemoryEvolutionLedger::default()
        };

        let snapshot = DefaultMemoryInspectionBuilder.build(
            &[],
            1,
            0,
            &[],
            Some(&projection),
            Some(&readiness),
            Some(&ledger),
            Some(&evolution),
            4,
        );

        assert!(snapshot.has_blockers());
        assert_eq!(snapshot.projection_issue_count, 2);
        assert_eq!(snapshot.projection_blocker_count, 1);
        assert_eq!(snapshot.readiness_missing_capability_count, 1);
        assert_eq!(snapshot.readiness_write_blocker_count, 1);
        assert_eq!(snapshot.evolution_blocker_count, 1);
        assert!(
            snapshot
                .risks
                .iter()
                .any(|risk| risk.code == "projection:duplicate_experience_id"
                    && risk.severity == MemoryInspectionRiskSeverity::Blocker)
        );
        assert!(
            snapshot
                .risks
                .iter()
                .any(|risk| risk.code == "evolution:context_rot_seen"
                    && risk.severity == MemoryInspectionRiskSeverity::Info)
        );
        let risk_codes = snapshot.risk_codes();
        assert!(risk_codes.contains(&"evolution:drift_rollbacks".to_owned()));
        assert!(risk_codes.contains(&"evolution:missing_replay_evidence".to_owned()));
        assert!(risk_codes.contains(&"projection:duplicate_experience_id".to_owned()));
        assert!(risk_codes.contains(&"readiness:missing:kv_swap".to_owned()));
        let risk_detail_codes = snapshot.risk_detail_codes();
        assert!(
            risk_detail_codes.contains(&"blocker:evolution:missing_replay_evidence:1".to_owned())
        );
        assert!(risk_detail_codes.contains(&"warning:evolution:drift_rollbacks:1:1".to_owned()));
        assert!(
            risk_detail_codes.contains(&"blocker:projection:duplicate_experience_id:1".to_owned())
        );
        assert!(
            snapshot
                .summary_line()
                .contains("risk_codes=evolution:context_rot_seen|evolution:drift_rollbacks")
        );
        assert!(
            snapshot
                .summary_line()
                .contains("detail_codes=blocker:evolution:missing_replay_evidence:1")
        );
    }

    #[test]
    fn inspection_snapshot_counts_unhealthy_adapters_and_formats_summary() {
        let adapters = vec![MemoryAdapterStatus::new(
            MemoryAdapterDescriptor::new("disk", vec![MemoryAdapterCapability::DiskKvOffload]),
            MemoryAdapterHealth {
                ready: false,
                record_count: Some(3),
                warnings: vec!["read_only_fixture".to_owned()],
            },
            AdapterWriteMode::ReadOnly,
        )];
        let readiness = MemoryServiceManifest::new(adapters.clone()).readiness(
            &MemoryServiceRequirement::for_profile(
                MemoryConsumerProfile::ShadowMigration,
                AdapterWriteMode::ReadOnly,
            )
            .with_capabilities(vec![MemoryAdapterCapability::DiskKvOffload]),
        );

        let snapshot = DefaultMemoryInspectionBuilder.build(
            &[],
            0,
            0,
            &adapters,
            None,
            Some(&readiness),
            None,
            None,
            4,
        );
        let summary = snapshot.summary_line();

        assert_eq!(snapshot.adapter_count, 1);
        assert_eq!(snapshot.unhealthy_adapter_count, 1);
        assert!(summary.contains("memory_inspection memories=0"));
        assert!(summary.contains("unhealthy_adapters=1"));
        assert!(summary.contains("readiness_missing_capabilities=1"));
        assert!(summary.contains("risk_codes=readiness:missing:disk_kv_offload"));
    }

    #[test]
    fn inspection_builder_reports_read_only_adapter_capability() {
        let builder = DefaultMemoryInspectionBuilder;
        let descriptor = builder.descriptor();

        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::StateInspection)
        );
        assert!(builder.health().unwrap().ready);
    }
}
