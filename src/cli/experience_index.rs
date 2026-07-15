use std::fs;
use std::io;
use std::path::PathBuf;

use rust_norion::{ExperienceIndexReport, GistLevel, GistRecord, NoironEngine};

use crate::Args;
use crate::path_utils::{ensure_parent_dir, timestamped_sidecar_path};

#[derive(Debug, Clone)]
pub(crate) struct ExperienceIndexCleanGistCommandReport {
    pub(crate) experience_path: PathBuf,
    pub(crate) record_id: u64,
    pub(crate) applied: bool,
    pub(crate) already_present: bool,
    pub(crate) backup_path: Option<PathBuf>,
    pub(crate) gist_title: String,
    pub(crate) gist_summary: String,
    pub(crate) before: ExperienceIndexReport,
    pub(crate) after: ExperienceIndexReport,
}

pub(crate) fn run_experience_index_add_clean_gist(
    args: &Args,
) -> io::Result<ExperienceIndexCleanGistCommandReport> {
    let record_id = args.experience_index_record_id.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "--experience-index-record-id is required",
        )
    })?;
    let gist_summary =
        normalized_gist(args.experience_index_clean_gist.as_deref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--experience-index-clean-gist is required",
            )
        })?)?;
    let gist_title = args
        .experience_index_clean_gist_title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Manual clean gist")
        .to_owned();

    let mut engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    let before = engine
        .experience
        .index_report(args.experience_cleanup_audit_limit);
    let record = engine.experience.record_mut(record_id).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("experience record {record_id} was not found"),
        )
    })?;

    let already_present = record
        .gist_records
        .iter()
        .any(|gist| gist.summary == gist_summary);
    let mut backup_path = None;
    let applied = if already_present {
        false
    } else {
        let path = args
            .experience_index_backup_path
            .clone()
            .unwrap_or_else(|| {
                timestamped_sidecar_path(&args.experience_path, "index-gist-backup")
            });
        ensure_parent_dir(&path)?;
        let (_, experience_read_path, _) = NoironEngine::full_state_read_paths(
            &args.memory_path,
            &args.experience_path,
            &args.adaptive_path,
        )?;
        fs::copy(experience_read_path, &path)?;
        record.gist_records.push(GistRecord {
            level: GistLevel::Document,
            title: gist_title.clone(),
            summary: gist_summary.clone(),
            source_tokens: approximate_record_tokens(record).max(1),
            importance: args.experience_index_clean_gist_importance.clamp(0.0, 1.0),
        });
        let note = format!("experience_index:manual_clean_gist:record_id={record_id}");
        if !record.process_reward.notes.iter().any(|item| item == &note) {
            record.process_reward.notes.push(note);
        }
        engine.save_full_state(
            &args.memory_path,
            &args.experience_path,
            &args.adaptive_path,
        )?;
        backup_path = Some(path);
        true
    };
    let after = engine
        .experience
        .index_report(args.experience_cleanup_audit_limit);

    Ok(ExperienceIndexCleanGistCommandReport {
        experience_path: args.experience_path.clone(),
        record_id,
        applied,
        already_present,
        backup_path,
        gist_title,
        gist_summary,
        before,
        after,
    })
}

pub(crate) fn print_experience_index_clean_gist_report(
    report: &ExperienceIndexCleanGistCommandReport,
) {
    println!("Noiron experience index clean gist");
    println!("experience_file: {}", report.experience_path.display());
    println!(
        "experience_index_clean_gist: applied={} already_present={} record_id={} title={} summary={}",
        report.applied,
        report.already_present,
        report.record_id,
        report.gist_title,
        report.gist_summary
    );
    match &report.backup_path {
        Some(path) => println!("backup_file: {}", path.display()),
        None => println!("backup_file: none"),
    }
    println!(
        "experience_index_before: overlong_without_clean_gist={} noisy_records={} duplicate_outputs={} max_noise_penalty={:.6} quality_score={:.6} retrieval_ready={} risk_level={}",
        report.before.overlong_without_clean_gist_count,
        report.before.noisy_record_count,
        report.before.duplicate_output_count,
        report.before.max_noise_penalty,
        report.before.quality_score,
        report.before.retrieval_ready,
        report.before.risk_level,
    );
    println!(
        "experience_index_after: overlong_without_clean_gist={} noisy_records={} duplicate_outputs={} max_noise_penalty={:.6} quality_score={:.6} retrieval_ready={} risk_level={}",
        report.after.overlong_without_clean_gist_count,
        report.after.noisy_record_count,
        report.after.duplicate_output_count,
        report.after.max_noise_penalty,
        report.after.quality_score,
        report.after.retrieval_ready,
        report.after.risk_level,
    );
}

fn normalized_gist(value: &str) -> io::Result<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--experience-index-clean-gist cannot be blank",
        ));
    }
    Ok(normalized)
}

fn approximate_record_tokens(record: &rust_norion::ExperienceRecord) -> usize {
    record
        .prompt
        .chars()
        .count()
        .saturating_add(record.lesson.chars().count())
        .div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::{
        HeuristicBackend, InferenceRequest, MemoryRetentionPolicy, NoironEngine, TaskProfile,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn normalized_gist_rejects_blank_text() {
        assert!(normalized_gist(" \n\t ").is_err());
    }

    #[test]
    fn normalized_gist_compacts_whitespace() {
        assert_eq!(
            normalized_gist(" route   helper\nfeedback ").unwrap(),
            "route helper feedback"
        );
    }

    #[test]
    fn clean_gist_apply_republishes_the_current_full_state_generation() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "rust-norion-index-generation-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let memory_path = root.join("memory.ndkv");
        let experience_path = root.join("experience.ndkv");
        let adaptive_path = root.join("adaptive.ndkv");
        let backup_path = root.join("experience.backup.ndkv");
        let mut engine = NoironEngine::new();
        engine.set_memory_retention_policy(MemoryRetentionPolicy {
            stale_after: 37,
            ..MemoryRetentionPolicy::default()
        });
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(
            InferenceRequest::new("Rust full-state clean gist apply", TaskProfile::Coding),
            &mut backend,
        );
        engine
            .save_full_state(&memory_path, &experience_path, &adaptive_path)
            .unwrap();
        engine
            .save_full_state(&memory_path, &experience_path, &adaptive_path)
            .unwrap();
        assert!(!experience_path.exists());
        let memory_count = engine.cache.len();

        let args = Args::parse(vec![
            "--memory".to_owned(),
            memory_path.display().to_string(),
            "--experience".to_owned(),
            experience_path.display().to_string(),
            "--adaptive".to_owned(),
            adaptive_path.display().to_string(),
            "--experience-index-add-clean-gist".to_owned(),
            "--experience-index-record-id".to_owned(),
            outcome.experience_id.to_string(),
            "--experience-index-clean-gist".to_owned(),
            "bounded current generation clean gist".to_owned(),
            "--experience-index-backup-path".to_owned(),
            backup_path.display().to_string(),
        ]);
        let report = run_experience_index_add_clean_gist(&args).unwrap();

        assert!(report.applied);
        assert!(backup_path.is_file());
        let restored =
            NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
        assert_eq!(restored.cache.len(), memory_count);
        assert_eq!(restored.memory_retention_policy.stale_after, 37);
        assert!(
            restored
                .experience
                .records()
                .iter()
                .find(|record| record.id == outcome.experience_id)
                .is_some_and(|record| record
                    .gist_records
                    .iter()
                    .any(|gist| gist.summary == "bounded current generation clean gist"))
        );

        fs::remove_dir_all(root).unwrap();
    }
}
