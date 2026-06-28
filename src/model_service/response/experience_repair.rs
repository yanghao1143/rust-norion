use std::path::PathBuf;

use rust_norion::{ExperienceRepairItem, ExperienceRepairPlan, ExperienceRepairSkippedItem};

use super::super::json::{option_path_service_json, service_json_string};

pub(crate) struct ModelServiceExperienceRepairView<'a> {
    pub(crate) request_id: usize,
    pub(crate) experience_path: &'a PathBuf,
    pub(crate) applied: bool,
    pub(crate) backup_path: Option<&'a PathBuf>,
    pub(crate) plan: &'a ExperienceRepairPlan,
}

pub(crate) fn model_service_experience_repair_response_json(
    view: ModelServiceExperienceRepairView<'_>,
) -> String {
    format!(
        "{{\"ok\":true,\"request_id\":{},\"experience_file\":{},\"applied\":{},\"backup_file\":{},\"plan\":{}}}",
        view.request_id,
        service_json_string(&view.experience_path.display().to_string()),
        view.applied,
        option_path_service_json(view.backup_path),
        repair_plan_json(view.plan)
    )
}

pub(super) fn repair_plan_json(plan: &ExperienceRepairPlan) -> String {
    format!(
        "{{\"total_records\":{},\"legacy_metadata_lessons\":{},\"repairable_legacy_metadata_lessons\":{},\"index_noisy_records\":{},\"index_duplicate_outputs\":{},\"repairable_index_records\":{},\"remaining_legacy_metadata_lessons_after_repair\":{},\"remaining_watch_after_repair\":{},\"remaining_quarantine_candidates_after_repair\":{},\"skipped_quarantine_candidates\":{},\"skipped_missing_clean_gist\":{},\"projected_hygiene_after_repair\":{},\"listed_repairs\":{},\"listed_skipped_quarantine_candidates\":{},\"listed_skipped_missing_clean_gist\":{}}}",
        plan.total_records,
        plan.legacy_metadata_lesson_count,
        plan.repairable_legacy_metadata_lesson_count,
        plan.index_noisy_record_count,
        plan.index_duplicate_output_count,
        plan.repairable_index_record_count,
        plan.remaining_legacy_metadata_lesson_count_after_repair(),
        plan.remaining_watch_count_after_repair(),
        plan.remaining_quarantine_candidate_count_after_repair(),
        plan.skipped_quarantine_candidate_count,
        plan.skipped_missing_clean_gist_count,
        repair_projection_json(plan),
        repair_items_json(&plan.listed_repairs),
        skipped_items_json(&plan.listed_skipped_quarantine_candidates),
        skipped_items_json(&plan.listed_skipped_missing_clean_gist)
    )
}

fn repair_projection_json(plan: &ExperienceRepairPlan) -> String {
    let projection = &plan.projected_after_repair;
    format!(
        "{{\"total_records\":{},\"findings\":{},\"watch\":{},\"quarantine_candidates\":{},\"legacy_metadata_lessons\":{},\"legacy_metadata_without_clean_gist\":{},\"index_quality_score\":{:.6},\"index_noisy_records\":{},\"index_duplicate_outputs\":{},\"index_retrieval_ready\":{},\"index_risk_level\":{}}}",
        projection.total_records,
        projection.hygiene_finding_count,
        projection.hygiene_watch_count,
        projection.hygiene_quarantine_candidate_count,
        projection.legacy_metadata_lesson_count,
        projection.legacy_metadata_without_clean_gist_count,
        projection.index_quality_score,
        projection.index_noisy_record_count,
        projection.index_duplicate_output_count,
        projection.index_retrieval_ready,
        service_json_string(&projection.index_risk_level)
    )
}

fn repair_items_json(items: &[ExperienceRepairItem]) -> String {
    let items = items
        .iter()
        .map(repair_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn repair_item_json(item: &ExperienceRepairItem) -> String {
    format!(
        "{{\"experience_id\":{},\"action\":\"{}\",\"source\":{}}}",
        item.experience_id,
        item.action.as_str(),
        service_json_string(&item.source)
    )
}

fn skipped_items_json(items: &[ExperienceRepairSkippedItem]) -> String {
    let items = items
        .iter()
        .map(skipped_item_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn skipped_item_json(item: &ExperienceRepairSkippedItem) -> String {
    format!(
        "{{\"experience_id\":{},\"reason\":{},\"gist_count\":{}}}",
        item.experience_id,
        service_json_string(&item.reason),
        item.gist_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::ExperienceRepairAction;

    #[test]
    fn repair_response_renders_dry_run_plan() {
        let path = PathBuf::from("experience.ndkv");
        let plan = ExperienceRepairPlan {
            total_records: 2,
            legacy_metadata_lesson_count: 2,
            repairable_legacy_metadata_lesson_count: 1,
            index_noisy_record_count: 1,
            index_duplicate_output_count: 1,
            repairable_index_record_count: 1,
            skipped_quarantine_candidate_count: 0,
            skipped_missing_clean_gist_count: 1,
            projected_after_repair: rust_norion::ExperienceRepairProjection {
                total_records: 2,
                hygiene_finding_count: 1,
                hygiene_watch_count: 1,
                hygiene_quarantine_candidate_count: 0,
                legacy_metadata_lesson_count: 1,
                legacy_metadata_without_clean_gist_count: 1,
                index_quality_score: 0.88,
                index_noisy_record_count: 0,
                index_duplicate_output_count: 0,
                index_retrieval_ready: true,
                index_risk_level: "watch".to_owned(),
            },
            listed_repairs: vec![ExperienceRepairItem {
                experience_id: 7,
                action: ExperienceRepairAction::ReuseResponse,
                source: "clean_gist".to_owned(),
                old_lesson_preview: "raw old lesson should stay out".to_owned(),
                proposed_lesson_preview: "raw proposed lesson should stay out".to_owned(),
                source_gist_preview: "raw gist should stay out".to_owned(),
            }],
            listed_skipped_quarantine_candidates: Vec::new(),
            listed_skipped_missing_clean_gist: vec![ExperienceRepairSkippedItem {
                experience_id: 8,
                reason: "missing_clean_gist".to_owned(),
                old_lesson_preview: "raw skipped lesson should stay out".to_owned(),
                prompt_preview: "raw skipped prompt should stay out".to_owned(),
                gist_count: 0,
            }],
        };

        let body =
            model_service_experience_repair_response_json(ModelServiceExperienceRepairView {
                request_id: 5,
                experience_path: &path,
                applied: false,
                backup_path: None,
                plan: &plan,
            });

        assert!(body.contains("\"applied\":false"));
        assert!(body.contains("\"backup_file\":null"));
        assert!(body.contains("\"repairable_legacy_metadata_lessons\":1"));
        assert!(body.contains("\"index_noisy_records\":1"));
        assert!(body.contains("\"index_duplicate_outputs\":1"));
        assert!(body.contains("\"repairable_index_records\":1"));
        assert!(body.contains("\"remaining_legacy_metadata_lessons_after_repair\":1"));
        assert!(body.contains("\"remaining_watch_after_repair\":1"));
        assert!(body.contains("\"remaining_quarantine_candidates_after_repair\":0"));
        assert!(body.contains("\"projected_hygiene_after_repair\":{"));
        assert!(body.contains("\"legacy_metadata_without_clean_gist\":1"));
        assert!(body.contains("\"index_quality_score\":0.880000"));
        assert!(body.contains("\"index_retrieval_ready\":true"));
        assert!(body.contains("\"index_risk_level\":\"watch\""));
        assert!(body.contains("\"skipped_missing_clean_gist\":1"));
        assert!(body.contains("\"listed_skipped_missing_clean_gist\":[{"));
        assert!(body.contains("\"reason\":\"missing_clean_gist\""));
        assert!(body.contains("\"gist_count\":0"));
        assert!(body.contains("\"action\":\"reuse_response\""));
        assert!(body.contains("\"source\":\"clean_gist\""));
        assert!(!body.contains("old_lesson_preview"));
        assert!(!body.contains("proposed_lesson_preview"));
        assert!(!body.contains("source_gist_preview"));
        assert!(!body.contains("prompt_preview"));
        assert!(!body.contains("raw old lesson should stay out"));
        assert!(!body.contains("raw proposed lesson should stay out"));
        assert!(!body.contains("raw gist should stay out"));
        assert!(!body.contains("raw skipped lesson should stay out"));
        assert!(!body.contains("raw skipped prompt should stay out"));
    }
}
