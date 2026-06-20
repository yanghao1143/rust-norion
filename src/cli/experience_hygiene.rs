use std::fs;
use std::io;
use std::path::PathBuf;

use rust_norion::{
    ExperienceHygieneQuarantinePlan, ExperienceHygieneReport, ExperienceIndexReport,
    ExperienceStore,
};

use crate::Args;
use crate::path_utils::{ensure_parent_dir, timestamped_sidecar_path};

#[derive(Debug, Clone)]
pub(crate) struct ExperienceHygieneCommandReport {
    pub(crate) hygiene: ExperienceHygieneReport,
    pub(crate) index: ExperienceIndexReport,
}

#[derive(Debug, Clone)]
pub(crate) struct ExperienceHygieneQuarantineCommandReport {
    pub(crate) plan: ExperienceHygieneQuarantinePlan,
    pub(crate) applied: bool,
    pub(crate) experience_path: PathBuf,
    pub(crate) backup_path: Option<PathBuf>,
    pub(crate) quarantine_path: Option<PathBuf>,
}

pub(crate) fn run_experience_hygiene_report(
    args: &Args,
) -> io::Result<ExperienceHygieneCommandReport> {
    let store = ExperienceStore::load_from_disk_kv(&args.experience_path)?;
    Ok(ExperienceHygieneCommandReport {
        hygiene: store.hygiene_report(args.experience_hygiene_limit),
        index: store.index_report(args.experience_hygiene_limit),
    })
}

pub(crate) fn run_experience_hygiene_quarantine(
    args: &Args,
) -> io::Result<ExperienceHygieneQuarantineCommandReport> {
    let store = ExperienceStore::load_from_disk_kv(&args.experience_path)?;
    let (retained_store, quarantined_store, plan) =
        store.split_hygiene_quarantine(args.experience_hygiene_limit);
    let mut report = ExperienceHygieneQuarantineCommandReport {
        plan,
        applied: false,
        experience_path: args.experience_path.clone(),
        backup_path: None,
        quarantine_path: None,
    };

    if !args.experience_hygiene_apply || report.plan.is_empty() {
        return Ok(report);
    }

    let backup_path = args
        .experience_hygiene_backup_path
        .clone()
        .unwrap_or_else(|| timestamped_sidecar_path(&args.experience_path, "backup"));
    let quarantine_path = args
        .experience_hygiene_quarantine_path
        .clone()
        .unwrap_or_else(|| timestamped_sidecar_path(&args.experience_path, "quarantine"));

    ensure_parent_dir(&backup_path)?;
    ensure_parent_dir(&quarantine_path)?;
    fs::copy(&args.experience_path, &backup_path)?;
    quarantined_store.save_to_disk_kv(&quarantine_path)?;
    retained_store.save_to_disk_kv(&args.experience_path)?;

    report.applied = true;
    report.backup_path = Some(backup_path);
    report.quarantine_path = Some(quarantine_path);
    Ok(report)
}

pub(crate) fn print_experience_hygiene_report(
    args: &Args,
    report: &ExperienceHygieneCommandReport,
) {
    println!("Noiron experience hygiene report");
    println!("experience_file: {}", args.experience_path.display());
    println!(
        "experience_hygiene: total_records={} findings={} watch={} quarantine_candidates={} legacy_metadata_lessons={} legacy_metadata_without_clean_gist={} listed={}",
        report.hygiene.total_records,
        report.hygiene.finding_count,
        report.hygiene.watch_count,
        report.hygiene.quarantine_candidate_count,
        report.hygiene.legacy_metadata_lesson_count,
        report.hygiene.legacy_metadata_without_clean_gist_count,
        report.hygiene.findings.len()
    );
    if report.hygiene.findings.is_empty() {
        println!("findings: none");
    } else {
        println!("findings:");
        for finding in &report.hygiene.findings {
            println!(
                "  id={} severity={} reason={} markers={} prompt={} lesson={}",
                finding.experience_id,
                finding.severity.as_str(),
                finding.reason,
                finding.markers.join(","),
                finding.prompt_preview,
                finding.lesson_preview
            );
        }
    }

    println!(
        "experience_index: total_records={} compacted_records={} overlong_records={} overlong_without_clean_gist={} max_record_chars={} noisy_records={} duplicate_outputs={} max_noise_penalty={:.6} quality_score={:.6} retrieval_ready={} risk_level={} recommended_action={} listed={}",
        report.index.total_records,
        report.index.compacted_record_count,
        report.index.overlong_record_count,
        report.index.overlong_without_clean_gist_count,
        report.index.max_record_chars,
        report.index.noisy_record_count,
        report.index.duplicate_output_count,
        report.index.max_noise_penalty,
        report.index.quality_score,
        report.index.retrieval_ready,
        report.index.risk_level,
        report.index.recommended_action,
        report.index.findings.len()
    );
    if report.index.findings.is_empty() {
        println!("index_findings: none");
    } else {
        println!("index_findings:");
        for finding in &report.index.findings {
            println!(
                "  id={} reason={} compacted={} noise_penalty={:.6} duplicate_of={} prompt_chars={} lesson_chars={} prompt={} lesson={}",
                finding.experience_id,
                finding.reason,
                finding.compacted,
                finding.noise_penalty,
                finding
                    .duplicate_of
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "none".to_owned()),
                finding.prompt_chars,
                finding.lesson_chars,
                finding.prompt_preview,
                finding.lesson_preview
            );
        }
    }
}

pub(crate) fn print_experience_hygiene_quarantine_report(
    report: &ExperienceHygieneQuarantineCommandReport,
) {
    println!("Noiron experience hygiene quarantine");
    println!("experience_file: {}", report.experience_path.display());
    println!(
        "experience_hygiene_quarantine: applied={} total_records={} retained_records={} quarantine_candidates={} listed={}",
        report.applied,
        report.plan.total_records,
        report.plan.retained_records,
        report.plan.quarantine_candidate_count,
        report.plan.listed_findings.len()
    );
    match &report.backup_path {
        Some(path) => println!("backup_file: {}", path.display()),
        None => println!("backup_file: none"),
    }
    match &report.quarantine_path {
        Some(path) => println!("quarantine_file: {}", path.display()),
        None => println!("quarantine_file: none"),
    }
    if report.plan.listed_findings.is_empty() {
        println!("findings: none");
        return;
    }

    println!("findings:");
    for finding in &report.plan.listed_findings {
        println!(
            "  id={} severity={} reason={} markers={} prompt={} lesson={}",
            finding.experience_id,
            finding.severity.as_str(),
            finding.reason,
            finding.markers.join(","),
            finding.prompt_preview,
            finding.lesson_preview
        );
    }
}
