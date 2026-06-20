use std::path::PathBuf;

use rust_norion::{
    ExperienceHygieneQuarantinePlan, ExperienceHygieneReport, ExperienceIndexReport,
    ExperienceRepairPlan,
};

use super::super::json::service_json_string;
use super::experience_hygiene::{index_report_json, quarantine_plan_json, report_json};
use super::experience_repair::repair_plan_json;

pub(crate) struct ModelServiceExperienceCleanupAuditView<'a> {
    pub(crate) request_id: usize,
    pub(crate) experience_path: &'a PathBuf,
    pub(crate) sample_limit: usize,
    pub(crate) hygiene: Option<&'a ExperienceHygieneReport>,
    pub(crate) quarantine: Option<&'a ExperienceHygieneQuarantinePlan>,
    pub(crate) repair: Option<&'a ExperienceRepairPlan>,
    pub(crate) index: Option<&'a ExperienceIndexReport>,
    pub(crate) error: Option<&'a str>,
}

pub(crate) fn model_service_experience_cleanup_audit_response_json(
    view: ModelServiceExperienceCleanupAuditView<'_>,
) -> String {
    let next_step = view.index.map(cleanup_next_step).unwrap_or(
        "Review this audit, then apply quarantine or repair only after explicit confirmation and backup review.",
    );
    format!(
        "{{\"ok\":true,\"request_id\":{},\"experience_file\":{},\"checked\":{},\"writes_experience_state\":false,\"sample_limit\":{},\"error\":{},\"report\":{},\"index_report\":{},\"quarantine_plan\":{},\"repair_plan\":{},\"next_step\":{}}}",
        view.request_id,
        service_json_string(&view.experience_path.display().to_string()),
        view.hygiene.is_some(),
        view.sample_limit.max(1),
        view.error
            .map(service_json_string)
            .unwrap_or_else(|| "null".to_owned()),
        view.hygiene
            .map(report_json)
            .unwrap_or_else(|| "null".to_owned()),
        view.index
            .map(index_report_json)
            .unwrap_or_else(|| "null".to_owned()),
        view.quarantine
            .map(|plan| quarantine_plan_json(plan, false))
            .unwrap_or_else(|| "null".to_owned()),
        view.repair
            .map(repair_plan_json)
            .unwrap_or_else(|| "null".to_owned()),
        service_json_string(next_step)
    )
}

fn cleanup_next_step(index: &ExperienceIndexReport) -> &'static str {
    match index.recommended_action.as_str() {
        "ready_for_retrieval" | "seed_experience" => {
            "Index is ready; keep using smoke/preflight before normal chat."
        }
        "pause_chat_and_add_clean_gists" | "add_clean_gists_for_long_records" => {
            "Pause normal chat and add or repair clean gists for long records before more retrieval."
        }
        "pause_chat_and_deduplicate_outputs" | "deduplicate_repeated_lessons" => {
            "Pause normal chat and deduplicate repeated lessons before more retrieval."
        }
        "review_index_findings" => {
            "Review listed index findings, then rerun cleanup audit before normal chat."
        }
        _ => {
            "Review this audit, then apply quarantine or repair only after explicit confirmation and backup review."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_audit_response_is_read_only_and_combined() {
        let path = PathBuf::from("experience.ndkv");
        let hygiene = ExperienceHygieneReport {
            total_records: 3,
            quarantine_candidate_count: 1,
            ..ExperienceHygieneReport::default()
        };
        let quarantine = ExperienceHygieneQuarantinePlan {
            total_records: 3,
            retained_records: 2,
            quarantine_candidate_count: 1,
            candidate_ids: vec![7],
            listed_findings: Vec::new(),
        };
        let repair = ExperienceRepairPlan {
            total_records: 3,
            repairable_legacy_metadata_lesson_count: 2,
            repairable_index_record_count: 1,
            ..ExperienceRepairPlan::default()
        };
        let index = ExperienceIndexReport {
            total_records: 3,
            compacted_record_count: 1,
            overlong_record_count: 1,
            overlong_without_clean_gist_count: 1,
            max_record_chars: 5120,
            noisy_record_count: 1,
            duplicate_output_count: 1,
            max_noise_penalty: 0.18,
            quality_score: 0.603333,
            retrieval_ready: true,
            risk_level: "degraded".to_owned(),
            recommended_action: "deduplicate_repeated_lessons".to_owned(),
            findings: Vec::new(),
        };

        let body = model_service_experience_cleanup_audit_response_json(
            ModelServiceExperienceCleanupAuditView {
                request_id: 9,
                experience_path: &path,
                sample_limit: 7,
                hygiene: Some(&hygiene),
                quarantine: Some(&quarantine),
                repair: Some(&repair),
                index: Some(&index),
                error: None,
            },
        );

        assert!(body.contains("\"request_id\":9"));
        assert!(body.contains("\"writes_experience_state\":false"));
        assert!(body.contains("\"sample_limit\":7"));
        assert!(body.contains("\"quarantine_candidates\":1"));
        assert!(body.contains("\"repairable_legacy_metadata_lessons\":2"));
        assert!(body.contains("\"repairable_index_records\":1"));
        assert!(body.contains("\"index_report\":{"));
        assert!(body.contains("\"overlong_records\":1"));
        assert!(body.contains("\"overlong_without_clean_gist\":1"));
        assert!(body.contains("\"max_record_chars\":5120"));
        assert!(body.contains("\"duplicate_outputs\":1"));
        assert!(body.contains("\"max_noise_penalty\":0.180000"));
        assert!(body.contains("\"quality_score\":0.603333"));
        assert!(body.contains("\"retrieval_ready\":true"));
        assert!(body.contains("\"risk_level\":\"degraded\""));
        assert!(body.contains("\"recommended_action\":\"deduplicate_repeated_lessons\""));
        assert!(body.contains(
            "\"next_step\":\"Pause normal chat and deduplicate repeated lessons before more retrieval.\""
        ));
    }
}
