use std::io;
use std::path::PathBuf;

use rust_norion::{
    ExperienceRepairPlan, ExperienceRepairSkippedItem, ExperienceStore, NoironEngine,
};

use crate::Args;
use crate::path_utils::{ensure_parent_dir, timestamped_sidecar_path};

#[derive(Debug, Clone)]
pub(crate) struct ExperienceRepairCommandReport {
    pub(crate) plan: ExperienceRepairPlan,
    pub(crate) applied: bool,
    pub(crate) experience_path: PathBuf,
    pub(crate) backup_path: Option<PathBuf>,
}

pub(crate) fn run_experience_repair(args: &Args) -> io::Result<ExperienceRepairCommandReport> {
    let mut engine = args
        .experience_repair_apply
        .then(|| {
            NoironEngine::load_full_state(
                &args.memory_path,
                &args.experience_path,
                &args.adaptive_path,
            )
        })
        .transpose()?;
    let store = match &engine {
        Some(engine) => engine.experience.clone(),
        None => {
            let (_, experience_read_path, _) = NoironEngine::full_state_read_paths(
                &args.memory_path,
                &args.experience_path,
                &args.adaptive_path,
            )?;
            ExperienceStore::load_from_disk_kv_read_only(experience_read_path)?
        }
    };
    let (repaired_store, plan) = store.repaired_legacy_metadata_store(args.experience_repair_limit);
    let mut report = ExperienceRepairCommandReport {
        plan,
        applied: false,
        experience_path: args.experience_path.clone(),
        backup_path: None,
    };

    if !args.experience_repair_apply || report.plan.is_empty() {
        return Ok(report);
    }

    let backup_path = args
        .experience_repair_backup_path
        .clone()
        .unwrap_or_else(|| timestamped_sidecar_path(&args.experience_path, "repair-backup"));

    ensure_parent_dir(&backup_path)?;
    let (_, experience_read_path, _) = NoironEngine::full_state_read_paths(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    std::fs::copy(experience_read_path, &backup_path)?;
    let engine = engine.as_mut().expect("apply mode loaded full state");
    engine.experience = repaired_store;
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;

    report.applied = true;
    report.backup_path = Some(backup_path);
    Ok(report)
}

pub(crate) fn print_experience_repair_report(report: &ExperienceRepairCommandReport) {
    println!("Noiron experience repair");
    println!("experience_file: {}", report.experience_path.display());
    println!("{}", experience_repair_summary_line(report));
    match &report.backup_path {
        Some(path) => println!("backup_file: {}", path.display()),
        None => println!("backup_file: none"),
    }
    println!(
        "projected_hygiene_after_repair: total_records={} findings={} watch={} quarantine_candidates={} legacy_metadata_lessons={} legacy_metadata_without_clean_gist={}",
        report.plan.projected_after_repair.total_records,
        report.plan.projected_after_repair.hygiene_finding_count,
        report.plan.projected_after_repair.hygiene_watch_count,
        report
            .plan
            .projected_after_repair
            .hygiene_quarantine_candidate_count,
        report
            .plan
            .projected_after_repair
            .legacy_metadata_lesson_count,
        report
            .plan
            .projected_after_repair
            .legacy_metadata_without_clean_gist_count,
    );
    println!(
        "projected_index_after_repair: quality_score={:.6} noisy_records={} duplicate_outputs={} retrieval_ready={} risk_level={}",
        report.plan.projected_after_repair.index_quality_score,
        report.plan.projected_after_repair.index_noisy_record_count,
        report
            .plan
            .projected_after_repair
            .index_duplicate_output_count,
        report.plan.projected_after_repair.index_retrieval_ready,
        report.plan.projected_after_repair.index_risk_level,
    );

    if report.plan.listed_repairs.is_empty() {
        println!("repairs: none");
    } else {
        println!("repairs:");
        for item in &report.plan.listed_repairs {
            println!(
                "  id={} action={} source={} old_lesson={} proposed_lesson={} source_gist={}",
                item.experience_id,
                item.action.as_str(),
                item.source,
                item.old_lesson_preview,
                item.proposed_lesson_preview,
                item.source_gist_preview
            );
        }
    }

    print_skipped_items(
        "skipped_quarantine_candidates",
        &report.plan.listed_skipped_quarantine_candidates,
    );
    print_skipped_items(
        "skipped_missing_clean_gist",
        &report.plan.listed_skipped_missing_clean_gist,
    );
}

fn experience_repair_summary_line(report: &ExperienceRepairCommandReport) -> String {
    format!(
        "experience_repair: applied={} total_records={} legacy_metadata_lessons={} repairable_legacy_metadata_lessons={} repairable_index_records={} index_noisy_records={} index_duplicate_outputs={} remaining_legacy_metadata_lessons_after_repair={} remaining_watch_after_repair={} remaining_quarantine_candidates_after_repair={} projected_index_noisy_records_after_repair={} projected_index_duplicate_outputs_after_repair={} skipped_quarantine_candidates={} skipped_missing_clean_gist={} listed={} listed_skipped_quarantine_candidates={} listed_skipped_missing_clean_gist={}",
        report.applied,
        report.plan.total_records,
        report.plan.legacy_metadata_lesson_count,
        report.plan.repairable_legacy_metadata_lesson_count,
        report.plan.repairable_index_record_count,
        report.plan.index_noisy_record_count,
        report.plan.index_duplicate_output_count,
        report
            .plan
            .remaining_legacy_metadata_lesson_count_after_repair(),
        report.plan.remaining_watch_count_after_repair(),
        report
            .plan
            .remaining_quarantine_candidate_count_after_repair(),
        report.plan.projected_after_repair.index_noisy_record_count,
        report
            .plan
            .projected_after_repair
            .index_duplicate_output_count,
        report.plan.skipped_quarantine_candidate_count,
        report.plan.skipped_missing_clean_gist_count,
        report.plan.listed_repairs.len(),
        report.plan.listed_skipped_quarantine_candidates.len(),
        report.plan.listed_skipped_missing_clean_gist.len()
    )
}

fn print_skipped_items(label: &str, items: &[ExperienceRepairSkippedItem]) {
    if items.is_empty() {
        println!("{label}: none");
        return;
    }

    println!("{label}:");
    for item in items {
        println!(
            "  id={} reason={} gist_count={} old_lesson={} prompt={}",
            item.experience_id,
            item.reason,
            item.gist_count,
            item.old_lesson_preview,
            item.prompt_preview
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::ExperienceRepairProjection;

    #[test]
    fn repair_summary_line_exposes_index_repairs() {
        let report = ExperienceRepairCommandReport {
            plan: ExperienceRepairPlan {
                total_records: 4,
                legacy_metadata_lesson_count: 1,
                repairable_legacy_metadata_lesson_count: 1,
                index_noisy_record_count: 2,
                index_duplicate_output_count: 1,
                repairable_index_record_count: 2,
                projected_after_repair: ExperienceRepairProjection {
                    total_records: 4,
                    index_noisy_record_count: 0,
                    index_duplicate_output_count: 0,
                    ..ExperienceRepairProjection::default()
                },
                ..ExperienceRepairPlan::default()
            },
            applied: false,
            experience_path: PathBuf::from("noiron-experience.ndkv"),
            backup_path: None,
        };

        let line = experience_repair_summary_line(&report);

        assert!(line.contains("repairable_legacy_metadata_lessons=1"));
        assert!(line.contains("repairable_index_records=2"));
        assert!(line.contains("index_noisy_records=2"));
        assert!(line.contains("index_duplicate_outputs=1"));
        assert!(line.contains("projected_index_noisy_records_after_repair=0"));
        assert!(line.contains("projected_index_duplicate_outputs_after_repair=0"));
    }
}
