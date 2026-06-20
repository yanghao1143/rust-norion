use std::collections::HashSet;

use super::hygiene;
use super::hygiene::{ExperienceHygieneFinding, ExperienceHygieneSeverity};
use super::{ExperienceRecord, ExperienceStore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceHygieneQuarantinePlan {
    pub total_records: usize,
    pub retained_records: usize,
    pub quarantine_candidate_count: usize,
    pub listed_findings: Vec<ExperienceHygieneFinding>,
    pub candidate_ids: Vec<u64>,
}

impl ExperienceHygieneQuarantinePlan {
    pub fn is_empty(&self) -> bool {
        self.quarantine_candidate_count == 0
    }
}

pub(crate) fn hygiene_quarantine_candidate_ids(records: &[ExperienceRecord]) -> HashSet<u64> {
    hygiene_quarantine_candidate_findings(records)
        .into_iter()
        .map(|finding| finding.experience_id)
        .collect()
}

fn hygiene_quarantine_candidate_findings(
    records: &[ExperienceRecord],
) -> Vec<ExperienceHygieneFinding> {
    let full_report = hygiene::inspect_records(records, records.len().max(1));
    full_report
        .findings
        .into_iter()
        .filter(|finding| finding.severity == ExperienceHygieneSeverity::QuarantineCandidate)
        .collect()
}

impl ExperienceStore {
    pub fn hygiene_quarantine_plan(&self, listed_limit: usize) -> ExperienceHygieneQuarantinePlan {
        let candidate_findings = hygiene_quarantine_candidate_findings(&self.records);
        let candidate_ids = candidate_findings
            .iter()
            .map(|finding| finding.experience_id)
            .collect::<Vec<_>>();
        let listed_findings = candidate_findings
            .iter()
            .take(listed_limit)
            .cloned()
            .collect::<Vec<_>>();

        ExperienceHygieneQuarantinePlan {
            total_records: self.records.len(),
            retained_records: self.records.len().saturating_sub(candidate_ids.len()),
            quarantine_candidate_count: candidate_ids.len(),
            listed_findings,
            candidate_ids,
        }
    }

    pub fn split_hygiene_quarantine(
        &self,
        listed_limit: usize,
    ) -> (
        ExperienceStore,
        ExperienceStore,
        ExperienceHygieneQuarantinePlan,
    ) {
        let plan = self.hygiene_quarantine_plan(listed_limit);
        let candidate_ids = plan.candidate_ids.iter().copied().collect::<HashSet<_>>();
        let (quarantined_records, retained_records): (
            Vec<ExperienceRecord>,
            Vec<ExperienceRecord>,
        ) = self
            .records
            .iter()
            .cloned()
            .partition(|record| candidate_ids.contains(&record.id));

        (
            ExperienceStore {
                records: retained_records,
                next_id: self.next_id,
            },
            ExperienceStore {
                records: quarantined_records,
                next_id: self.next_id,
            },
            plan,
        )
    }
}
