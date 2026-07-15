use std::fmt::Write as _;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rust_norion::{
    ExperienceHygieneQuarantinePlan, ExperienceHygieneReport, ExperienceIndexFinding,
    ExperienceIndexReport, ExperienceRepairPlan, ExperienceRepairSkippedItem, ExperienceStore,
    NoironEngine,
};

use crate::Args;
use crate::path_utils::ensure_parent_dir;

#[derive(Debug, Clone)]
pub(crate) struct ExperienceCleanupAuditCommandReport {
    pub(crate) audit_path: PathBuf,
    pub(crate) memory_path: PathBuf,
    pub(crate) experience_path: PathBuf,
    pub(crate) adaptive_path: PathBuf,
    pub(crate) sample_limit: usize,
    pub(crate) hygiene: ExperienceHygieneReport,
    pub(crate) quarantine: ExperienceHygieneQuarantinePlan,
    pub(crate) repair: ExperienceRepairPlan,
    pub(crate) index: ExperienceIndexReport,
}

pub(crate) fn run_experience_cleanup_audit(
    args: &Args,
) -> io::Result<ExperienceCleanupAuditCommandReport> {
    let (_, experience_read_path, _) = NoironEngine::full_state_read_paths(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let store = ExperienceStore::load_from_disk_kv_read_only(experience_read_path)?;
    let sample_limit = args.experience_cleanup_audit_limit.max(1);
    let report = ExperienceCleanupAuditCommandReport {
        audit_path: args
            .experience_cleanup_audit_path
            .clone()
            .unwrap_or_else(|| default_audit_path(&args.experience_path)),
        memory_path: args.memory_path.clone(),
        experience_path: args.experience_path.clone(),
        adaptive_path: args.adaptive_path.clone(),
        sample_limit,
        hygiene: store.hygiene_report(sample_limit),
        quarantine: store.hygiene_quarantine_plan(sample_limit),
        repair: store.legacy_metadata_repair_plan(sample_limit),
        index: store.index_report(sample_limit),
    };

    ensure_parent_dir(&report.audit_path)?;
    std::fs::write(&report.audit_path, render_cleanup_audit_markdown(&report))?;
    Ok(report)
}

pub(crate) fn print_experience_cleanup_audit_report(report: &ExperienceCleanupAuditCommandReport) {
    println!("Noiron experience cleanup audit");
    println!("experience_file: {}", report.experience_path.display());
    println!("audit_file: {}", report.audit_path.display());
    println!(
        "experience_cleanup_audit: total_records={} quarantine_candidates={} repairable_legacy_metadata_lessons={} repairable_index_records={} projected_legacy_metadata_lessons_after_repair={} projected_index_noisy_records_after_repair={} projected_index_duplicate_outputs_after_repair={} skipped_missing_clean_gist={} index_overlong_records={} index_overlong_without_clean_gist={} index_max_record_chars={} index_noisy_records={} duplicate_outputs={} max_noise_penalty={:.6} index_quality_score={:.6} index_retrieval_ready={} index_risk_level={} index_recommended_action={} sample_limit={}",
        report.hygiene.total_records,
        report.quarantine.quarantine_candidate_count,
        report.repair.repairable_legacy_metadata_lesson_count,
        report.repair.repairable_index_record_count,
        report
            .repair
            .projected_after_repair
            .legacy_metadata_lesson_count,
        report
            .repair
            .projected_after_repair
            .index_noisy_record_count,
        report
            .repair
            .projected_after_repair
            .index_duplicate_output_count,
        report.repair.skipped_missing_clean_gist_count,
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
        report.sample_limit
    );
}

fn render_cleanup_audit_markdown(report: &ExperienceCleanupAuditCommandReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Noiron Experience Cleanup Audit");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- experience_file: `{}`",
        report.experience_path.display()
    );
    let _ = writeln!(out, "- sample_limit: `{}`", report.sample_limit);
    let _ = writeln!(out, "- writes_experience_state: `false`");
    let _ = writeln!(out);

    write_readiness_gate_section(&mut out, report);
    let _ = writeln!(out);

    let _ = writeln!(out, "## Summary");
    let _ = writeln!(out, "- total_records: `{}`", report.hygiene.total_records);
    let _ = writeln!(
        out,
        "- hygiene_findings: `{}` watch=`{}` quarantine_candidates=`{}`",
        report.hygiene.finding_count,
        report.hygiene.watch_count,
        report.hygiene.quarantine_candidate_count
    );
    let _ = writeln!(
        out,
        "- legacy_metadata_lessons: `{}` without_clean_gist=`{}`",
        report.hygiene.legacy_metadata_lesson_count,
        report.hygiene.legacy_metadata_without_clean_gist_count
    );
    let _ = writeln!(
        out,
        "- repairable_legacy_metadata_lessons: `{}`",
        report.repair.repairable_legacy_metadata_lesson_count
    );
    let _ = writeln!(
        out,
        "- repairable_index_records: `{}` noisy_records=`{}` duplicate_outputs=`{}`",
        report.repair.repairable_index_record_count,
        report.repair.index_noisy_record_count,
        report.repair.index_duplicate_output_count
    );
    let _ = writeln!(
        out,
        "- projected_after_repair: findings=`{}` watch=`{}` quarantine_candidates=`{}` legacy_metadata_lessons=`{}` without_clean_gist=`{}` index_noisy_records=`{}` duplicate_outputs=`{}`",
        report.repair.projected_after_repair.hygiene_finding_count,
        report.repair.projected_after_repair.hygiene_watch_count,
        report
            .repair
            .projected_after_repair
            .hygiene_quarantine_candidate_count,
        report
            .repair
            .projected_after_repair
            .legacy_metadata_lesson_count,
        report
            .repair
            .projected_after_repair
            .legacy_metadata_without_clean_gist_count,
        report
            .repair
            .projected_after_repair
            .index_noisy_record_count,
        report
            .repair
            .projected_after_repair
            .index_duplicate_output_count
    );
    let _ = writeln!(
        out,
        "- index: compacted_records=`{}` overlong_records=`{}` overlong_without_clean_gist=`{}` max_record_chars=`{}` noisy_records=`{}` duplicate_outputs=`{}` max_noise_penalty=`{:.6}` quality_score=`{:.6}` retrieval_ready=`{}` risk_level=`{}` recommended_action=`{}`",
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
        report.index.recommended_action
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## Recommended Order");
    let _ = writeln!(
        out,
        "1. Review `ready_to_chat`, blocking reasons, and candidate IDs before sending more prompts."
    );
    let _ = writeln!(
        out,
        "2. Run quarantine dry-run and inspect the candidate IDs."
    );
    let _ = writeln!(
        out,
        "3. Apply quarantine only after explicit approval and backup review."
    );
    let _ = writeln!(
        out,
        "4. Run experience repair dry-run on the retained store and confirm the projected hygiene/index remainder."
    );
    let _ = writeln!(
        out,
        "5. Apply repair only after quarantine, explicit approval, and backup review."
    );
    let _ = writeln!(
        out,
        "6. Re-run cleanup audit, then run the strict inspect gate until quarantine, repair, and index noise thresholds pass."
    );
    let _ = writeln!(out);

    write_quarantine_section(&mut out, report);
    write_repair_section(&mut out, report);
    write_index_section(&mut out, report);
    write_commands_section(&mut out, report);
    out
}

fn write_readiness_gate_section(out: &mut String, report: &ExperienceCleanupAuditCommandReport) {
    let dirty = report.quarantine.quarantine_candidate_count > 0
        || report.repair.repairable_legacy_metadata_lesson_count > 0
        || report.repair.repairable_index_record_count > 0
        || report.index.overlong_without_clean_gist_count > 0
        || report.index.noisy_record_count > 0
        || report.index.duplicate_output_count > 0
        || report.index.max_noise_penalty > 0.0;
    let _ = writeln!(out, "## Readiness Gate");
    let _ = writeln!(out, "- ready_to_chat: `{}`", !dirty);
    let _ = writeln!(
        out,
        "- blocking_reasons: quarantine_candidates=`{}` repairable_legacy_metadata_lessons=`{}` repairable_index_records=`{}` index_overlong_without_clean_gist=`{}` index_noisy_records=`{}` duplicate_outputs=`{}` max_noise_penalty=`{:.6}` index_retrieval_ready=`{}` index_risk_level=`{}` index_recommended_action=`{}`",
        report.quarantine.quarantine_candidate_count,
        report.repair.repairable_legacy_metadata_lesson_count,
        report.repair.repairable_index_record_count,
        report.index.overlong_without_clean_gist_count,
        report.index.noisy_record_count,
        report.index.duplicate_output_count,
        report.index.max_noise_penalty,
        report.index.retrieval_ready,
        report.index.risk_level,
        report.index.recommended_action
    );
    if !report.quarantine.candidate_ids.is_empty() {
        let ids = report
            .quarantine
            .candidate_ids
            .iter()
            .take(12)
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let suffix = if report.quarantine.candidate_ids.len() > 12 {
            ",..."
        } else {
            ""
        };
        let _ = writeln!(out, "- candidate_ids: `[{ids}{suffix}]`");
    }
    let _ = writeln!(
        out,
        "- recommended_order: review audit -> dry-run quarantine -> explicit apply after backup -> dry-run repair -> explicit apply after backup -> strict inspect gate -> Forge smoke/preflight"
    );
    let _ = writeln!(
        out,
        "- ready_to_chat_note: when `ready_to_chat` is `false`, pause normal chat and use only audit, dry-run, approved apply, and inspect-gate commands until this gate passes."
    );
    let _ = writeln!(
        out,
        "- strict_inspect_gate: all cleanup/index-noise thresholds must pass at `0`, index quality must stay above `0.92`, and retrieval must be ready before Forge smoke/preflight or normal chat."
    );
    let _ = writeln!(
        out,
        "- apply_guard: audit generation is read-only; no `.ndkv` changes happen from this command."
    );
}

fn write_quarantine_section(out: &mut String, report: &ExperienceCleanupAuditCommandReport) {
    let _ = writeln!(out, "## Quarantine Candidates");
    let _ = writeln!(
        out,
        "- count: `{}` retained_after_quarantine: `{}`",
        report.quarantine.quarantine_candidate_count, report.quarantine.retained_records
    );
    if report.quarantine.listed_findings.is_empty() {
        let _ = writeln!(out, "- samples: none");
    } else {
        for finding in &report.quarantine.listed_findings {
            let _ = writeln!(
                out,
                "- id=`{}` severity=`{}` reason=`{}` markers=`{}` prompt=`{}` lesson=`{}`",
                finding.experience_id,
                finding.severity.as_str(),
                md_inline(&finding.reason),
                md_inline(&finding.markers.join(",")),
                md_inline(&finding.prompt_preview),
                md_inline(&finding.lesson_preview)
            );
        }
    }
    let _ = writeln!(out);
}

fn write_repair_section(out: &mut String, report: &ExperienceCleanupAuditCommandReport) {
    let _ = writeln!(out, "## Experience Repair");
    let _ = writeln!(
        out,
        "- repairable: legacy_metadata=`{}` index_records=`{}` skipped_quarantine_candidates=`{}` skipped_missing_clean_gist=`{}`",
        report.repair.repairable_legacy_metadata_lesson_count,
        report.repair.repairable_index_record_count,
        report.repair.skipped_quarantine_candidate_count,
        report.repair.skipped_missing_clean_gist_count
    );
    if report.repair.listed_repairs.is_empty() {
        let _ = writeln!(out, "- repair_samples: none");
    } else {
        let _ = writeln!(out, "### Repair Samples");
        for item in &report.repair.listed_repairs {
            let _ = writeln!(
                out,
                "- id=`{}` action=`{}` source=`{}` old=`{}` proposed=`{}` gist=`{}`",
                item.experience_id,
                item.action.as_str(),
                md_inline(&item.source),
                md_inline(&item.old_lesson_preview),
                md_inline(&item.proposed_lesson_preview),
                md_inline(&item.source_gist_preview)
            );
        }
    }
    write_skipped_items(
        out,
        "Skipped Quarantine Candidate Samples",
        &report.repair.listed_skipped_quarantine_candidates,
    );
    write_skipped_items(
        out,
        "Skipped Missing Clean Gist Samples",
        &report.repair.listed_skipped_missing_clean_gist,
    );
    let _ = writeln!(out);
}

fn write_skipped_items(out: &mut String, title: &str, items: &[ExperienceRepairSkippedItem]) {
    let _ = writeln!(out, "### {title}");
    if items.is_empty() {
        let _ = writeln!(out, "- none");
        return;
    }
    for item in items {
        let _ = writeln!(
            out,
            "- id=`{}` reason=`{}` gist_count=`{}` old=`{}` prompt=`{}`",
            item.experience_id,
            md_inline(&item.reason),
            item.gist_count,
            md_inline(&item.old_lesson_preview),
            md_inline(&item.prompt_preview)
        );
    }
}

fn write_index_section(out: &mut String, report: &ExperienceCleanupAuditCommandReport) {
    let _ = writeln!(out, "## Index Noise");
    let _ = writeln!(
        out,
        "- compacted_records: `{}` overlong_records: `{}` overlong_without_clean_gist: `{}` max_record_chars: `{}` noisy_records: `{}` duplicate_outputs: `{}` max_noise_penalty: `{:.6}` quality_score: `{:.6}` retrieval_ready: `{}` risk_level: `{}`",
        report.index.compacted_record_count,
        report.index.overlong_record_count,
        report.index.overlong_without_clean_gist_count,
        report.index.max_record_chars,
        report.index.noisy_record_count,
        report.index.duplicate_output_count,
        report.index.max_noise_penalty,
        report.index.quality_score,
        report.index.retrieval_ready,
        report.index.risk_level
    );
    if report.index.findings.is_empty() {
        let _ = writeln!(out, "- samples: none");
    } else {
        for finding in &report.index.findings {
            write_index_finding(out, finding);
        }
    }
    let _ = writeln!(out);
}

fn write_index_finding(out: &mut String, finding: &ExperienceIndexFinding) {
    let _ = writeln!(
        out,
        "- id=`{}` reason=`{}` compacted=`{}` noise_penalty=`{:.6}` duplicate_of=`{}` prompt_chars=`{}` lesson_chars=`{}` prompt=`{}` lesson=`{}`",
        finding.experience_id,
        md_inline(&finding.reason),
        finding.compacted,
        finding.noise_penalty,
        finding
            .duplicate_of
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        finding.prompt_chars,
        finding.lesson_chars,
        md_inline(&finding.prompt_preview),
        md_inline(&finding.lesson_preview)
    );
}

fn write_commands_section(out: &mut String, report: &ExperienceCleanupAuditCommandReport) {
    let memory_path = report.memory_path.display();
    let experience_path = report.experience_path.display();
    let adaptive_path = report.adaptive_path.display();
    let quarantine_backup = followup_sidecar_path(report, "quarantine-backup");
    let quarantine_file = followup_sidecar_path(report, "quarantine");
    let repair_backup = followup_sidecar_path(report, "repair-backup");
    let _ = writeln!(out, "## Follow-up Commands");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "These commands are printed for review. Only run apply commands after explicit approval and backup review."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "Dry-run quarantine:");
    let _ = writeln!(out, "```powershell");
    let _ = writeln!(
        out,
        "cargo run -- --memory \"{}\" --experience \"{}\" --adaptive \"{}\" --experience-hygiene-quarantine --experience-hygiene-limit {}",
        memory_path, experience_path, adaptive_path, report.sample_limit
    );
    let _ = writeln!(out, "```");
    let _ = writeln!(out);
    let _ = writeln!(out, "Apply quarantine after explicit approval:");
    let _ = writeln!(out, "```powershell");
    let _ = writeln!(
        out,
        "cargo run -- --memory \"{}\" --experience \"{}\" --adaptive \"{}\" --experience-hygiene-apply --experience-hygiene-limit {} --experience-hygiene-backup-path \"{}\" --experience-hygiene-quarantine-path \"{}\"",
        memory_path,
        experience_path,
        adaptive_path,
        report.sample_limit,
        quarantine_backup.display(),
        quarantine_file.display()
    );
    let _ = writeln!(out, "```");
    let _ = writeln!(out);
    let _ = writeln!(out, "Dry-run repair:");
    let _ = writeln!(out, "```powershell");
    let _ = writeln!(
        out,
        "cargo run -- --memory \"{}\" --experience \"{}\" --adaptive \"{}\" --experience-repair --experience-repair-limit {}",
        memory_path, experience_path, adaptive_path, report.sample_limit
    );
    let _ = writeln!(out, "```");
    let _ = writeln!(out);
    let _ = writeln!(out, "Apply repair after quarantine and explicit approval:");
    let _ = writeln!(out, "```powershell");
    let _ = writeln!(
        out,
        "cargo run -- --memory \"{}\" --experience \"{}\" --adaptive \"{}\" --experience-repair-apply --experience-repair-limit {} --experience-repair-backup-path \"{}\"",
        memory_path,
        experience_path,
        adaptive_path,
        report.sample_limit,
        repair_backup.display()
    );
    let _ = writeln!(out, "```");
    let _ = writeln!(out);
    let _ = writeln!(out, "Re-run cleanup audit:");
    let _ = writeln!(out, "```powershell");
    let _ = writeln!(
        out,
        "cargo run -- --memory \"{}\" --experience \"{}\" --adaptive \"{}\" --experience-cleanup-audit --experience-cleanup-audit-limit {}",
        memory_path, experience_path, adaptive_path, report.sample_limit
    );
    let _ = writeln!(out, "```");
    let _ = writeln!(out);
    let _ = writeln!(out, "Strict inspect gate:");
    let _ = writeln!(out, "```powershell");
    let _ = writeln!(
        out,
        "cargo run -- --memory \"{}\" --experience \"{}\" --adaptive \"{}\" --inspect-state --inspect-gate --inspect-limit {} --inspect-max-experience-hygiene-quarantine-candidates 0 --inspect-max-experience-repairable-legacy-metadata-lessons 0 --inspect-max-experience-repairable-index-records 0 --inspect-max-experience-repair-projected-legacy-metadata-lessons 0 --inspect-max-experience-repair-skipped-missing-clean-gist 0 --inspect-max-experience-index-overlong-without-clean-gist 0 --inspect-max-experience-index-noisy-records 0 --inspect-max-experience-index-noise-penalty 0 --inspect-min-experience-index-quality-score 0.92 --inspect-require-experience-index-retrieval-ready",
        memory_path, experience_path, adaptive_path, report.sample_limit
    );
    let _ = writeln!(out, "```");
}

fn followup_sidecar_path(report: &ExperienceCleanupAuditCommandReport, label: &str) -> PathBuf {
    let file_name = report
        .experience_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("experience.ndkv");
    PathBuf::from("target")
        .join("experience-cleanup-audit")
        .join(format!("{file_name}.{label}.manual.ndkv"))
}

fn default_audit_path(experience_path: &std::path::Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let file_name = experience_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("experience.ndkv");
    PathBuf::from("target")
        .join("experience-cleanup-audit")
        .join(format!("{file_name}.cleanup-audit.{stamp}.md"))
}

fn md_inline(value: &str) -> String {
    let compacted = value
        .replace('`', "'")
        .replace(['\r', '\n'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    const MAX_CHARS: usize = 220;
    let mut preview = compacted.chars().take(MAX_CHARS).collect::<String>();
    if compacted.chars().count() > MAX_CHARS {
        preview.push_str("...");
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use rust_norion::{
        ExperienceHygieneFinding, ExperienceHygieneSeverity, ExperienceRepairProjection,
    };

    #[test]
    fn cleanup_audit_markdown_blocks_dirty_experience_with_review_commands() {
        let report = audit_report_with_dirty_experience();

        let markdown = render_cleanup_audit_markdown(&report);
        let memory_path = report.memory_path.display().to_string();
        let experience_path = report.experience_path.display().to_string();
        let adaptive_path = report.adaptive_path.display().to_string();

        assert!(markdown.contains("## Readiness Gate"));
        assert!(markdown.contains("- ready_to_chat: `false`"));
        assert!(markdown.contains("quarantine_candidates=`2`"));
        assert!(markdown.contains("repairable_legacy_metadata_lessons=`1`"));
        assert!(markdown.contains("repairable_index_records=`1`"));
        assert!(markdown.contains("index_overlong_without_clean_gist=`1`"));
        assert!(markdown.contains("index_noisy_records=`1`"));
        assert!(markdown.contains("duplicate_outputs=`1`"));
        assert!(
            markdown.contains(
                "- repairable_index_records: `1` noisy_records=`1` duplicate_outputs=`1`"
            )
        );
        assert!(markdown.contains(
            "projected_after_repair: findings=`0` watch=`0` quarantine_candidates=`0` legacy_metadata_lessons=`1` without_clean_gist=`0` index_noisy_records=`0` duplicate_outputs=`0`"
        ));
        assert!(markdown.contains("## Experience Repair"));
        assert!(markdown.contains("- repairable: legacy_metadata=`1` index_records=`1`"));
        assert!(markdown.contains("index_retrieval_ready=`true`"));
        assert!(markdown.contains("index_risk_level=`degraded`"));
        assert!(markdown.contains("index_recommended_action=`deduplicate_repeated_lessons`"));
        assert!(markdown.contains("quality_score=`0.603333`"));
        assert!(markdown.contains("- candidate_ids: `[863,862]`"));
        assert!(markdown.contains("- writes_experience_state: `false`"));
        assert!(markdown.contains("pause normal chat"));
        assert!(markdown.contains("- strict_inspect_gate: all cleanup/index-noise thresholds"));
        assert!(markdown.contains("Apply quarantine only after explicit approval"));
        assert!(markdown.contains("Apply repair only after quarantine"));
        assert!(markdown.contains("Dry-run quarantine:"));
        assert!(markdown.contains("Apply quarantine after explicit approval:"));
        assert!(markdown.contains("Apply repair after quarantine and explicit approval:"));
        assert!(markdown.contains("Strict inspect gate:"));
        assert!(markdown.contains(&format!(
            "cargo run -- --memory \"{}\" --experience \"{}\" --adaptive \"{}\" --inspect-state --inspect-gate",
            memory_path, experience_path, adaptive_path
        )));
        assert!(markdown.contains("--inspect-max-experience-hygiene-quarantine-candidates 0"));
        assert!(markdown.contains("--inspect-max-experience-repairable-legacy-metadata-lessons 0"));
        assert!(markdown.contains("--inspect-max-experience-repairable-index-records 0"));
        assert!(markdown.contains("--inspect-max-experience-index-overlong-without-clean-gist 0"));
        assert!(markdown.contains("--inspect-max-experience-index-noisy-records 0"));
        assert!(markdown.contains("--inspect-max-experience-index-noise-penalty 0"));
        assert!(markdown.contains("--inspect-min-experience-index-quality-score 0.92"));
        assert!(markdown.contains("--inspect-require-experience-index-retrieval-ready"));

        let quarantine_dry_run = markdown.find("Dry-run quarantine:").unwrap();
        let quarantine_apply = markdown
            .find("Apply quarantine after explicit approval:")
            .unwrap();
        let repair_dry_run = markdown.find("Dry-run repair:").unwrap();
        let repair_apply = markdown
            .find("Apply repair after quarantine and explicit approval:")
            .unwrap();
        let rerun_audit = markdown.find("Re-run cleanup audit:").unwrap();
        let strict_gate = markdown.find("Strict inspect gate:").unwrap();

        assert!(quarantine_dry_run < quarantine_apply);
        assert!(quarantine_apply < repair_dry_run);
        assert!(repair_dry_run < repair_apply);
        assert!(repair_apply < rerun_audit);
        assert!(rerun_audit < strict_gate);
    }

    #[test]
    fn cleanup_audit_markdown_allows_clean_experience() {
        let report = ExperienceCleanupAuditCommandReport {
            audit_path: PathBuf::from("target/experience-cleanup-audit/audit.md"),
            memory_path: PathBuf::from("noiron-memory.ndkv"),
            experience_path: PathBuf::from("noiron-experience.ndkv"),
            adaptive_path: PathBuf::from("noiron-adaptive.ndkv"),
            sample_limit: 3,
            hygiene: ExperienceHygieneReport {
                total_records: 2,
                ..ExperienceHygieneReport::default()
            },
            quarantine: ExperienceHygieneQuarantinePlan {
                total_records: 2,
                retained_records: 2,
                ..empty_quarantine_plan()
            },
            repair: ExperienceRepairPlan {
                total_records: 2,
                projected_after_repair: ExperienceRepairProjection {
                    total_records: 2,
                    ..ExperienceRepairProjection::default()
                },
                ..ExperienceRepairPlan::default()
            },
            index: ExperienceIndexReport {
                total_records: 2,
                compacted_record_count: 2,
                recommended_action: "ready_for_retrieval".to_owned(),
                ..ExperienceIndexReport::default()
            },
        };

        let markdown = render_cleanup_audit_markdown(&report);

        assert!(markdown.contains("- ready_to_chat: `true`"));
        assert!(markdown.contains("retrieval_ready=`true`"));
        assert!(markdown.contains("risk_level=`clean`"));
        assert!(markdown.contains("recommended_action=`ready_for_retrieval`"));
        assert!(!markdown.contains("- candidate_ids:"));
        assert!(markdown.contains("- apply_guard: audit generation is read-only"));
    }

    fn audit_report_with_dirty_experience() -> ExperienceCleanupAuditCommandReport {
        ExperienceCleanupAuditCommandReport {
            audit_path: PathBuf::from("target/experience-cleanup-audit/audit.md"),
            memory_path: PathBuf::from("noiron-memory.ndkv"),
            experience_path: PathBuf::from("noiron-experience.ndkv"),
            adaptive_path: PathBuf::from("noiron-adaptive.ndkv"),
            sample_limit: 3,
            hygiene: ExperienceHygieneReport {
                total_records: 3,
                finding_count: 2,
                quarantine_candidate_count: 2,
                legacy_metadata_lesson_count: 1,
                findings: Vec::new(),
                ..ExperienceHygieneReport::default()
            },
            quarantine: ExperienceHygieneQuarantinePlan {
                total_records: 3,
                retained_records: 1,
                quarantine_candidate_count: 2,
                listed_findings: vec![ExperienceHygieneFinding {
                    experience_id: 863,
                    severity: ExperienceHygieneSeverity::QuarantineCandidate,
                    reason: "cross task shell transcript".to_string(),
                    markers: vec!["powershell".to_string()],
                    prompt_preview: "status check".to_string(),
                    lesson_preview: "old transcript noise".to_string(),
                }],
                candidate_ids: vec![863, 862],
            },
            repair: ExperienceRepairPlan {
                total_records: 3,
                legacy_metadata_lesson_count: 1,
                repairable_legacy_metadata_lesson_count: 1,
                index_noisy_record_count: 1,
                index_duplicate_output_count: 1,
                repairable_index_record_count: 1,
                projected_after_repair: ExperienceRepairProjection {
                    total_records: 3,
                    legacy_metadata_lesson_count: 1,
                    index_noisy_record_count: 0,
                    index_duplicate_output_count: 0,
                    ..ExperienceRepairProjection::default()
                },
                ..ExperienceRepairPlan::default()
            },
            index: ExperienceIndexReport {
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
                findings: vec![ExperienceIndexFinding {
                    experience_id: 851,
                    reason: "long prompt".to_string(),
                    compacted: true,
                    noise_penalty: 0.18,
                    duplicate_of: Some(849),
                    prompt_chars: 4096,
                    lesson_chars: 1024,
                    prompt_preview: "long prompt".to_string(),
                    lesson_preview: "long lesson".to_string(),
                }],
            },
        }
    }

    fn empty_quarantine_plan() -> ExperienceHygieneQuarantinePlan {
        ExperienceHygieneQuarantinePlan {
            total_records: 0,
            retained_records: 0,
            quarantine_candidate_count: 0,
            listed_findings: Vec::new(),
            candidate_ids: Vec::new(),
        }
    }
}
