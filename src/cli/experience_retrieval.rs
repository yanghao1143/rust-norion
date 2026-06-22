use std::io;
use std::path::Path;

use rust_norion::{ExperienceRetrievalReport, ExperienceStore, render_experience_hint};

use crate::Args;

pub(crate) fn run_experience_retrieval_report(
    args: &Args,
) -> io::Result<ExperienceRetrievalReport> {
    let store = ExperienceStore::load_from_disk_kv_read_only(&args.experience_path)?;
    Ok(store.retrieval_report(&args.prompt, args.profile, args.experience_retrieval_limit))
}

pub(crate) fn print_experience_retrieval_report(args: &Args, report: &ExperienceRetrievalReport) {
    for line in experience_retrieval_report_lines(&args.experience_path, report) {
        println!("{line}");
    }
}

fn experience_retrieval_report_lines(
    experience_path: &Path,
    report: &ExperienceRetrievalReport,
) -> Vec<String> {
    let mut lines = vec![
        "Noiron experience retrieval preview".to_owned(),
        format!("experience_file: {}", experience_path.display()),
        format!("profile: {:?}", report.profile),
        format!("prompt: {}", compact_preview(&report.prompt, 220)),
        format!(
            "experience_retrieval: total_records={} requested_limit={} matches={} skipped_cross_task_pollution={} retrieval_noise_penalized_candidates={} retrieval_noise_filtered_candidates={} suppressed_prompt_index_candidates={} max_retrieval_noise_penalty={} max_score={}",
            report.total_records,
            report.requested_limit,
            report.match_count(),
            report.skipped_cross_task_pollution,
            report.retrieval_noise_penalized_candidates,
            report.retrieval_noise_filtered_candidates,
            report.suppressed_prompt_index_candidates,
            option_f32_text(Some(report.max_retrieval_noise_penalty)),
            option_f32_text(report.max_score())
        ),
    ];
    if report.matches.is_empty() {
        lines.push("matches: none".to_owned());
        return lines;
    }

    lines.push("matches:".to_owned());
    for item in &report.matches {
        lines.push(format!(
            "  id={} score={:.6} quality={:.3} reward={:.3} action={} runtime_model={} adapter={} device={} recursive_runtime_calls={} usable_hint={} lesson={} prompt={}",
            item.id,
            item.score,
            item.quality,
            item.process_reward,
            item.reward_action.as_str(),
            option_text(item.runtime_model_id.as_deref()),
            option_text(item.runtime_selected_adapter.as_deref()),
            option_text(item.runtime_device_profile.as_deref()),
            option_usize_text(item.recursive_runtime_calls),
            compact_preview(&render_experience_hint(item), 260),
            compact_preview(&item.lesson, 220),
            compact_preview(&item.prompt, 180)
        ));
        if !item.gist_hints.is_empty() {
            lines.push(format!("    gist_hints={}", item.gist_hints.join(" | ")));
        }
        if !item.reflection_issue_codes.is_empty() {
            lines.push(format!(
                "    reflection_issues={}",
                item.reflection_issue_codes.join(",")
            ));
        }
    }
    lines
}

fn compact_preview(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in value.chars().take(max_chars) {
        if ch.is_whitespace() {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn option_text(value: Option<&str>) -> &str {
    value.filter(|item| !item.is_empty()).unwrap_or("none")
}

fn option_f32_text(value: Option<f32>) -> String {
    value
        .filter(|item| item.is_finite())
        .map(|item| format!("{item:.6}"))
        .unwrap_or_else(|| "none".to_owned())
}

fn option_usize_text(value: Option<usize>) -> String {
    value
        .map(|item| item.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Args;
    use rust_norion::TaskProfile;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn retrieval_report_lines_include_noise_counters() {
        let report = ExperienceRetrievalReport {
            prompt: "accepted_pattern quality overlap max_severity".to_owned(),
            profile: TaskProfile::Coding,
            total_records: 3,
            requested_limit: 5,
            skipped_cross_task_pollution: 1,
            retrieval_noise_penalized_candidates: 2,
            retrieval_noise_filtered_candidates: 1,
            suppressed_prompt_index_candidates: 2,
            max_retrieval_noise_penalty: 0.44,
            matches: Vec::new(),
        };

        let lines = experience_retrieval_report_lines(Path::new("experience.ndkv"), &report);
        let summary = lines
            .iter()
            .find(|line| line.starts_with("experience_retrieval:"))
            .unwrap();

        assert!(summary.contains("retrieval_noise_penalized_candidates=2"));
        assert!(summary.contains("retrieval_noise_filtered_candidates=1"));
        assert!(summary.contains("suppressed_prompt_index_candidates=2"));
        assert!(summary.contains("max_retrieval_noise_penalty=0.440000"));
        assert!(lines.contains(&"matches: none".to_owned()));
    }

    #[test]
    fn retrieval_report_missing_experience_is_read_only() {
        let path = temp_path("retrieval-missing-read-only");
        let args = Args::parse(vec![
            "--experience".to_owned(),
            path.display().to_string(),
            "--experience-retrieval".to_owned(),
            "missing experience retrieval prompt".to_owned(),
        ]);

        let report = run_experience_retrieval_report(&args).unwrap();

        assert_eq!(report.total_records, 0);
        assert!(report.matches.is_empty());
        assert!(!path.exists());
        let _ = fs::remove_file(path);
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-{label}-{}-{nanos}.ndkv",
            std::process::id()
        ))
    }
}
