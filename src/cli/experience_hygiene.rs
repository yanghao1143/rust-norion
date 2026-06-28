use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rust_norion::{
    ExperienceHygieneFinding, ExperienceHygieneQuarantinePlan, ExperienceHygieneReport,
    ExperienceIndexFinding, ExperienceIndexReport, ExperienceStore,
    SelfEvolvingMemorySourceQuarantineReport, SelfEvolvingMemoryStore,
};

use crate::cli::state::ensure_runtime_state_write_window_clean;
use crate::path_utils::{ensure_parent_dir, timestamped_sidecar_path};
use crate::Args;

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

#[derive(Debug, Clone)]
pub(crate) struct SelfEvolvingMemoryQuarantineCommandReport {
    pub(crate) report: SelfEvolvingMemorySourceQuarantineReport,
    pub(crate) snapshot_path: PathBuf,
    pub(crate) backup_path: Option<PathBuf>,
    pub(crate) applied_to_disk: bool,
    pub(crate) snapshot_digest: Option<String>,
    pub(crate) disk_snapshot_digest: Option<String>,
}

pub(crate) fn run_experience_hygiene_report(
    args: &Args,
) -> io::Result<ExperienceHygieneCommandReport> {
    let store = ExperienceStore::load_from_disk_kv_read_only(&args.experience_path)?;
    Ok(ExperienceHygieneCommandReport {
        hygiene: store.hygiene_report(args.experience_hygiene_limit),
        index: store.index_report(args.experience_hygiene_limit),
    })
}

pub(crate) fn run_experience_hygiene_quarantine(
    args: &Args,
) -> io::Result<ExperienceHygieneQuarantineCommandReport> {
    if args.experience_hygiene_apply {
        ensure_runtime_state_write_window_clean(args)?;
    }
    let store = if args.experience_hygiene_apply {
        ExperienceStore::load_from_disk_kv(&args.experience_path)?
    } else {
        ExperienceStore::load_from_disk_kv_read_only(&args.experience_path)?
    };
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

pub(crate) fn run_self_evolving_memory_quarantine(
    args: &Args,
) -> io::Result<SelfEvolvingMemoryQuarantineCommandReport> {
    if args.self_evolving_memory_quarantine_apply {
        ensure_runtime_state_write_window_clean(args)?;
    }
    let snapshot_path = self_evolving_memory_store_path(&args.experience_path);
    let mut store = SelfEvolvingMemoryStore::load_snapshot(&snapshot_path)?;
    let source_case = args
        .self_evolving_memory_quarantine_source_case
        .as_deref()
        .unwrap_or_default();
    let report =
        store.quarantine_source_case(source_case, &args.self_evolving_memory_quarantine_reason);
    let mut command = SelfEvolvingMemoryQuarantineCommandReport {
        report,
        snapshot_path,
        backup_path: None,
        applied_to_disk: false,
        snapshot_digest: None,
        disk_snapshot_digest: None,
    };

    if !args.self_evolving_memory_quarantine_apply || command.report.action_count() == 0 {
        append_self_evolving_memory_quarantine_trace(args, &command)?;
        return Ok(command);
    }

    let backup_path = timestamped_sidecar_path(&command.snapshot_path, "backup");
    ensure_parent_dir(&backup_path)?;
    fs::copy(&command.snapshot_path, &backup_path)?;
    let snapshot_digest = store.snapshot_digest();
    store.save_snapshot(&command.snapshot_path)?;
    let disk_snapshot_digest =
        SelfEvolvingMemoryStore::load_snapshot(&command.snapshot_path)?.snapshot_digest();
    command.backup_path = Some(backup_path);
    command.applied_to_disk = true;
    command.snapshot_digest = Some(snapshot_digest);
    command.disk_snapshot_digest = Some(disk_snapshot_digest);
    append_self_evolving_memory_quarantine_trace(args, &command)?;
    Ok(command)
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
            println!("{}", hygiene_finding_line(finding));
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
            println!("{}", index_finding_line(finding));
        }
    }
}

fn hygiene_finding_line(finding: &ExperienceHygieneFinding) -> String {
    format!(
        "  id={} severity={} reason={} markers={}",
        finding.experience_id,
        finding.severity.as_str(),
        finding.reason,
        finding.markers.join(",")
    )
}

fn index_finding_line(finding: &ExperienceIndexFinding) -> String {
    format!(
        "  id={} reason={} compacted={} noise_penalty={:.6} duplicate_of={} prompt_chars={} lesson_chars={}",
        finding.experience_id,
        finding.reason,
        finding.compacted,
        finding.noise_penalty,
        finding
            .duplicate_of
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        finding.prompt_chars,
        finding.lesson_chars
    )
}

pub(crate) fn print_self_evolving_memory_quarantine_report(
    report: &SelfEvolvingMemoryQuarantineCommandReport,
) {
    println!("Noiron self-evolving memory quarantine");
    println!(
        "self_evolving_memory_file: {}",
        report.snapshot_path.display()
    );
    println!("{}", report.report.summary_line());
    println!("applied_to_disk: {}", report.applied_to_disk);
    match &report.backup_path {
        Some(path) => println!("backup_file: {}", path.display()),
        None => println!("backup_file: none"),
    }
}

fn self_evolving_memory_store_path(experience_path: &Path) -> PathBuf {
    experience_path.with_extension("self-evolving-memory.tsv")
}

fn append_self_evolving_memory_quarantine_trace(
    args: &Args,
    report: &SelfEvolvingMemoryQuarantineCommandReport,
) -> io::Result<()> {
    let line = report.report.json_line(
        report.applied_to_disk,
        report.snapshot_digest.as_deref(),
        report.disk_snapshot_digest.as_deref(),
    );
    if let Some(path) = &args.trace_path {
        append_trace_line(path, &line)?;
    }
    if let Some(path) = &args.trace_schema_gate_path
        && args.trace_path.as_ref() != Some(path)
    {
        append_trace_line(path, &line)?;
    }
    Ok(())
}

fn append_trace_line(path: &Path, line: &str) -> io::Result<()> {
    ensure_parent_dir(path)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")
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
        println!("{}", hygiene_finding_line(finding));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::ExperienceHygieneSeverity;

    #[test]
    fn hygiene_finding_line_does_not_expose_previews() {
        let line = hygiene_finding_line(&ExperienceHygieneFinding {
            experience_id: 7,
            severity: ExperienceHygieneSeverity::Watch,
            reason: "legacy_metadata_lesson".to_owned(),
            markers: vec!["legacy".to_owned()],
            prompt_preview: "raw prompt should stay out".to_owned(),
            lesson_preview: "raw lesson should stay out".to_owned(),
        });

        assert!(line.contains("id=7"));
        assert!(line.contains("markers=legacy"));
        assert!(!line.contains("raw prompt should stay out"));
        assert!(!line.contains("raw lesson should stay out"));
    }

    #[test]
    fn index_finding_line_does_not_expose_previews() {
        let line = index_finding_line(&ExperienceIndexFinding {
            experience_id: 9,
            reason: "duplicate_output".to_owned(),
            compacted: true,
            noise_penalty: 0.25,
            duplicate_of: Some(3),
            prompt_chars: 42,
            lesson_chars: 24,
            prompt_preview: "raw prompt should stay out".to_owned(),
            lesson_preview: "raw lesson should stay out".to_owned(),
        });

        assert!(line.contains("id=9"));
        assert!(line.contains("duplicate_of=3"));
        assert!(line.contains("prompt_chars=42"));
        assert!(line.contains("lesson_chars=24"));
        assert!(!line.contains("prompt should stay out"));
        assert!(!line.contains("lesson should stay out"));
    }
}
