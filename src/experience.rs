mod codec;
pub(crate) mod evidence;
mod hint;
mod hygiene;
mod index;
mod model;
mod noise;
mod persistence;
mod quarantine;
mod relevance;
mod repair;
mod retrieval;
mod text_normalize;

use crate::hierarchy::TaskProfile;

#[cfg(test)]
use codec::{
    deserialize_live_evolution, deserialize_record, deserialize_runtime_diagnostics, escape_field,
    serialize_record, serialize_runtime_diagnostics,
};
pub use hint::render_experience_hint;
pub use hygiene::{ExperienceHygieneFinding, ExperienceHygieneReport, ExperienceHygieneSeverity};
pub use index::{ExperienceIndexFinding, ExperienceIndexReport};
pub use model::{
    ExperienceInput, ExperienceMatch, ExperienceRecord, ExperienceRetrievalReport,
    ExperienceRuntimeTokenMetrics,
};
pub use quarantine::ExperienceHygieneQuarantinePlan;
pub(crate) use quarantine::hygiene_quarantine_candidate_ids;
pub use repair::{
    ExperienceRepairAction, ExperienceRepairItem, ExperienceRepairPlan, ExperienceRepairProjection,
    ExperienceRepairSkippedItem,
};
pub use retrieval::recursive_runtime_calls_from_notes;

#[derive(Debug, Clone)]
pub struct ExperienceStore {
    records: Vec<ExperienceRecord>,
    next_id: u64,
}

impl Default for ExperienceStore {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
        }
    }
}

impl ExperienceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn records(&self) -> &[ExperienceRecord] {
        &self.records
    }

    pub fn record_mut(&mut self, id: u64) -> Option<&mut ExperienceRecord> {
        self.records.iter_mut().find(|record| record.id == id)
    }

    pub fn hygiene_report(&self, limit: usize) -> ExperienceHygieneReport {
        hygiene::inspect_records(&self.records, limit)
    }

    pub fn index_report(&self, limit: usize) -> ExperienceIndexReport {
        index::inspect_records(&self.records, limit)
    }

    pub fn record(&mut self, input: ExperienceInput) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut record = ExperienceRecord {
            id,
            prompt: input.prompt,
            profile: input.profile,
            lesson: input.lesson,
            quality: input.quality.clamp(0.0, 1.0),
            contradictions: input.contradictions,
            reflection_issues: input.reflection_issues,
            revision_actions: input.revision_actions,
            stored_memory_id: input.stored_memory_id,
            router_threshold_after: input.router_threshold_after,
            stream_windows: input.stream_windows,
            route_budget: input.route_budget,
            hierarchy: input.hierarchy,
            used_memory_ids: input.used_memory_ids,
            gist_records: input.gist_records,
            gist_memory_ids: input.gist_memory_ids,
            stored_runtime_kv_memory_ids: input.stored_runtime_kv_memory_ids,
            runtime_diagnostics: input.runtime_diagnostics,
            runtime_token_metrics: input.runtime_token_metrics,
            process_reward: input.process_reward,
            live_evolution: input.live_evolution,
        };
        hygiene::apply_admission_hygiene(&mut record);
        index::apply_runtime_backend_error_clean_gist(&mut record);
        index::apply_generated_response_clean_gist(&mut record);
        index::apply_admission_duplicate_guard(&mut record, &self.records);
        index::apply_admission_index_note(&mut record);
        self.records.push(record);
        id
    }

    pub fn recent(&self, limit: usize) -> Vec<&ExperienceRecord> {
        self.records.iter().rev().take(limit).collect()
    }

    pub fn top_lessons(&self, min_quality: f32, limit: usize) -> Vec<&ExperienceRecord> {
        let mut records = self
            .records
            .iter()
            .filter(|record| record.quality >= min_quality)
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            right
                .quality
                .partial_cmp(&left.quality)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        records.truncate(limit);
        records
    }

    pub fn retrieve_lessons(
        &self,
        prompt: &str,
        profile: TaskProfile,
        limit: usize,
    ) -> Vec<ExperienceMatch> {
        retrieval::retrieve_lessons(&self.records, prompt, profile, limit)
    }

    pub fn retrieval_report(
        &self,
        prompt: &str,
        profile: TaskProfile,
        limit: usize,
    ) -> ExperienceRetrievalReport {
        retrieval::retrieve_report(&self.records, prompt, profile, limit)
    }
}

#[cfg(test)]
mod tests;
