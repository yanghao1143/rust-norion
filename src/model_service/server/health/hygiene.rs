use std::{fs, path::PathBuf};

use rust_norion::{ExperienceIndexReport, ExperienceRepairPlan, ExperienceStore};

use crate::Args;
use crate::model_service::json::service_json_string;

const MAX_INLINE_EXPERIENCE_HYGIENE_BYTES: u64 = 1_000_000;

#[derive(Debug, Clone)]
pub(crate) struct ExperienceHygieneHealthStatus {
    pub(crate) experience_file: PathBuf,
    pub(crate) checked: bool,
    pub(crate) clean: Option<bool>,
    pub(crate) findings: Option<usize>,
    pub(crate) watch: Option<usize>,
    pub(crate) quarantine_candidates: Option<usize>,
    pub(crate) legacy_metadata_lessons: Option<usize>,
    pub(crate) legacy_metadata_without_clean_gist: Option<usize>,
    pub(crate) repair: Option<ExperienceHygieneRepairHealthStatus>,
    pub(crate) index: Option<ExperienceIndexHealthStatus>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExperienceHygieneRepairHealthStatus {
    pub(crate) repairable_legacy_metadata_lessons: usize,
    pub(crate) repairable_index_records: usize,
    pub(crate) projected_findings_after_repair: usize,
    pub(crate) projected_watch_after_repair: usize,
    pub(crate) projected_quarantine_candidates_after_repair: usize,
    pub(crate) projected_legacy_metadata_lessons_after_repair: usize,
    pub(crate) projected_legacy_metadata_without_clean_gist_after_repair: usize,
    pub(crate) skipped_quarantine_candidates: usize,
    pub(crate) skipped_missing_clean_gist: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExperienceIndexHealthStatus {
    pub(crate) total_records: usize,
    pub(crate) compacted_records: usize,
    pub(crate) noisy_records: usize,
    pub(crate) duplicate_outputs: usize,
    pub(crate) max_noise_penalty: f32,
    pub(crate) quality_score: f32,
    pub(crate) retrieval_ready: bool,
    pub(crate) risk_level: String,
}

impl ExperienceIndexHealthStatus {
    pub(crate) fn from_report(report: &ExperienceIndexReport) -> Self {
        Self {
            total_records: report.total_records,
            compacted_records: report.compacted_record_count,
            noisy_records: report.noisy_record_count,
            duplicate_outputs: report.duplicate_output_count,
            max_noise_penalty: report.max_noise_penalty,
            quality_score: report.quality_score,
            retrieval_ready: report.retrieval_ready,
            risk_level: report.risk_level.clone(),
        }
    }

    fn json(&self) -> String {
        format!(
            "{{\"total_records\":{},\"compacted_records\":{},\"noisy_records\":{},\"duplicate_outputs\":{},\"max_noise_penalty\":{:.6},\"quality_score\":{:.6},\"retrieval_ready\":{},\"risk_level\":{}}}",
            self.total_records,
            self.compacted_records,
            self.noisy_records,
            self.duplicate_outputs,
            self.max_noise_penalty,
            self.quality_score,
            self.retrieval_ready,
            service_json_string(&self.risk_level)
        )
    }

    fn warning(&self) -> Option<String> {
        if !self.retrieval_ready {
            return Some(format!(
                "experience_index: retrieval blocked risk_level={} quality_score={:.3}; run --experience-cleanup-audit before chatting",
                self.risk_level, self.quality_score
            ));
        }
        if self.risk_level != "clean" {
            return Some(format!(
                "experience_index: risk_level={} quality_score={:.3} noisy_records={} duplicate_outputs={}",
                self.risk_level, self.quality_score, self.noisy_records, self.duplicate_outputs
            ));
        }
        None
    }
}

impl ExperienceHygieneRepairHealthStatus {
    pub(crate) fn from_plan(plan: &ExperienceRepairPlan) -> Self {
        Self {
            repairable_legacy_metadata_lessons: plan.repairable_legacy_metadata_lesson_count,
            repairable_index_records: plan.repairable_index_record_count,
            projected_findings_after_repair: plan.projected_after_repair.hygiene_finding_count,
            projected_watch_after_repair: plan.projected_after_repair.hygiene_watch_count,
            projected_quarantine_candidates_after_repair: plan
                .projected_after_repair
                .hygiene_quarantine_candidate_count,
            projected_legacy_metadata_lessons_after_repair: plan
                .projected_after_repair
                .legacy_metadata_lesson_count,
            projected_legacy_metadata_without_clean_gist_after_repair: plan
                .projected_after_repair
                .legacy_metadata_without_clean_gist_count,
            skipped_quarantine_candidates: plan.skipped_quarantine_candidate_count,
            skipped_missing_clean_gist: plan.skipped_missing_clean_gist_count,
        }
    }

    fn json(self) -> String {
        format!(
            "{{\"repairable_legacy_metadata_lessons\":{},\"repairable_index_records\":{},\"projected_findings_after_repair\":{},\"projected_watch_after_repair\":{},\"projected_quarantine_candidates_after_repair\":{},\"projected_legacy_metadata_lessons_after_repair\":{},\"projected_legacy_metadata_without_clean_gist_after_repair\":{},\"skipped_quarantine_candidates\":{},\"skipped_missing_clean_gist\":{}}}",
            self.repairable_legacy_metadata_lessons,
            self.repairable_index_records,
            self.projected_findings_after_repair,
            self.projected_watch_after_repair,
            self.projected_quarantine_candidates_after_repair,
            self.projected_legacy_metadata_lessons_after_repair,
            self.projected_legacy_metadata_without_clean_gist_after_repair,
            self.skipped_quarantine_candidates,
            self.skipped_missing_clean_gist
        )
    }
}

impl ExperienceHygieneHealthStatus {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"experience_file\":{},\"checked\":{},\"clean\":{},\"findings\":{},\"watch\":{},\"quarantine_candidates\":{},\"legacy_metadata_lessons\":{},\"legacy_metadata_without_clean_gist\":{},\"repair\":{},\"index\":{},\"error\":{}}}",
            service_json_string(&self.experience_file.display().to_string()),
            self.checked,
            option_bool_json(self.clean),
            option_usize_json(self.findings),
            option_usize_json(self.watch),
            option_usize_json(self.quarantine_candidates),
            option_usize_json(self.legacy_metadata_lessons),
            option_usize_json(self.legacy_metadata_without_clean_gist),
            option_repair_json(self.repair),
            option_index_json(self.index.as_ref()),
            option_string_json(self.error.as_deref())
        )
    }

    pub(crate) fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if let Some(count) = self.quarantine_candidates.filter(|count| *count > 0) {
            warnings.push(format!(
                "experience_hygiene: {count} quarantine candidates; run --experience-hygiene-quarantine before chatting"
            ));
        }
        if let Some(repair) = self
            .repair
            .filter(|repair| repair.repairable_legacy_metadata_lessons > 0)
        {
            warnings.push(format!(
                "experience_repair: {} legacy metadata lessons can be repaired; dry-run with --experience-repair before chatting",
                repair.repairable_legacy_metadata_lessons
            ));
        }
        if let Some(repair) = self
            .repair
            .filter(|repair| repair.repairable_index_records > 0)
        {
            warnings.push(format!(
                "experience_repair: {} index records can be repaired; dry-run with --experience-repair before chatting",
                repair.repairable_index_records
            ));
        }
        if let Some(count) = self
            .legacy_metadata_without_clean_gist
            .filter(|count| *count > 0)
        {
            warnings.push(format!(
                "experience_hygiene: {count} legacy metadata lessons have no clean gist; run --experience-hygiene"
            ));
        }
        if let Some(index_warning) = self
            .index
            .as_ref()
            .and_then(ExperienceIndexHealthStatus::warning)
        {
            warnings.push(index_warning);
        }
        if let Some(error) = &self.error {
            warnings.push(format!("experience_hygiene: {error}"));
        }
        warnings
    }
}

pub(crate) fn experience_hygiene_health_status(args: &Args) -> ExperienceHygieneHealthStatus {
    if !args.experience_path.exists() {
        return unchecked_experience_hygiene_status(args, "experience_file_missing".to_owned());
    }

    if let Ok(metadata) = fs::metadata(&args.experience_path) {
        let size_bytes = metadata.len();
        if size_bytes > MAX_INLINE_EXPERIENCE_HYGIENE_BYTES {
            return unchecked_experience_hygiene_status(
                args,
                format!(
                    "experience_hygiene_deferred_large_file: size_bytes={} max_inline_bytes={}; run CLI --experience-cleanup-audit for offline full audit",
                    size_bytes, MAX_INLINE_EXPERIENCE_HYGIENE_BYTES
                ),
            );
        }
    }

    match ExperienceStore::load_from_disk_kv_read_only(&args.experience_path) {
        Ok(store) => {
            let report = store.hygiene_report(1);
            let repair = ExperienceHygieneRepairHealthStatus::from_plan(
                &store.legacy_metadata_repair_plan(1),
            );
            let index = ExperienceIndexHealthStatus::from_report(&store.index_report(1));
            ExperienceHygieneHealthStatus {
                experience_file: args.experience_path.clone(),
                checked: true,
                clean: Some(report.finding_count == 0),
                findings: Some(report.finding_count),
                watch: Some(report.watch_count),
                quarantine_candidates: Some(report.quarantine_candidate_count),
                legacy_metadata_lessons: Some(report.legacy_metadata_lesson_count),
                legacy_metadata_without_clean_gist: Some(
                    report.legacy_metadata_without_clean_gist_count,
                ),
                repair: Some(repair),
                index: Some(index),
                error: None,
            }
        }
        Err(error) => unchecked_experience_hygiene_status(args, error.to_string()),
    }
}

fn unchecked_experience_hygiene_status(
    args: &Args,
    error: String,
) -> ExperienceHygieneHealthStatus {
    ExperienceHygieneHealthStatus {
        experience_file: args.experience_path.clone(),
        checked: false,
        clean: None,
        findings: None,
        watch: None,
        quarantine_candidates: None,
        legacy_metadata_lessons: None,
        legacy_metadata_without_clean_gist: None,
        repair: None,
        index: None,
        error: if error.is_empty() { None } else { Some(error) },
    }
}

fn option_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_usize_json(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_repair_json(value: Option<ExperienceHygieneRepairHealthStatus>) -> String {
    value
        .map(ExperienceHygieneRepairHealthStatus::json)
        .unwrap_or_else(|| "null".to_owned())
}

fn option_index_json(value: Option<&ExperienceIndexHealthStatus>) -> String {
    value
        .map(ExperienceIndexHealthStatus::json)
        .unwrap_or_else(|| "null".to_owned())
}

fn option_string_json(value: Option<&str>) -> String {
    value
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned())
}
