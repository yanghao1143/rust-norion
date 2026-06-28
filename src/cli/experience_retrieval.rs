use std::io;
use std::path::Path;

use rust_norion::{render_experience_hint, ExperienceRetrievalReport, ExperienceStore};

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
        format!("prompt_chars: {}", report.prompt.chars().count()),
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
            "  id={} score={:.6} quality={:.3} reward={:.3} action={} runtime_model={} adapter={} device={} recursive_runtime_calls={} prompt_chars={} lesson_chars={} usable_hint_chars={} gist_hint_count={}",
            item.id,
            item.score,
            item.quality,
            item.process_reward,
            item.reward_action.as_str(),
            option_text(item.runtime_model_id.as_deref()),
            option_text(item.runtime_selected_adapter.as_deref()),
            option_text(item.runtime_device_profile.as_deref()),
            option_usize_text(item.recursive_runtime_calls),
            item.prompt.chars().count(),
            item.lesson.chars().count(),
            render_experience_hint(item).chars().count(),
            item.gist_hints.len()
        ));
        if !item.reflection_issue_codes.is_empty() {
            lines.push(format!(
                "    reflection_issues={}",
                item.reflection_issue_codes.join(",")
            ));
        }
    }
    lines
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
    use rust_norion::{ExperienceMatch, RewardAction, TaskProfile};
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
    fn retrieval_report_lines_do_not_expose_prompt_lesson_or_gist_text() {
        let report = ExperienceRetrievalReport {
            prompt: "raw retrieval prompt should stay out".to_owned(),
            profile: TaskProfile::Coding,
            total_records: 1,
            requested_limit: 1,
            skipped_cross_task_pollution: 0,
            retrieval_noise_penalized_candidates: 0,
            retrieval_noise_filtered_candidates: 0,
            suppressed_prompt_index_candidates: 0,
            max_retrieval_noise_penalty: 0.0,
            matches: vec![ExperienceMatch {
                id: 8,
                prompt: "raw matched prompt should stay out".to_owned(),
                lesson: "raw matched lesson should stay out".to_owned(),
                quality: 0.8,
                score: 0.7,
                gist_hints: vec!["raw gist should stay out".to_owned()],
                reflection_issue_codes: Vec::new(),
                revision_actions: Vec::new(),
                process_reward: 0.6,
                reward_action: RewardAction::Reinforce,
                runtime_model_id: None,
                runtime_selected_adapter: None,
                runtime_device_profile: None,
                runtime_primary_lane: None,
                runtime_fallback_lane: None,
                runtime_memory_mode: None,
                runtime_device_execution_source: None,
                runtime_forward_energy: None,
                runtime_kv_influence: None,
                runtime_uncertainty_perplexity: None,
                recursive_runtime_calls: None,
                runtime_imported_kv_blocks: 0,
                runtime_weak_kv_imports_skipped: 0,
                runtime_budget_limited_kv_imports_skipped: 0,
                runtime_exported_kv_blocks: 0,
                runtime_kv_segments_included: 0,
                runtime_kv_segments_skipped: 0,
                runtime_kv_segments_rejected: 0,
                live_memory_feedback_reinforced: 0,
                live_memory_feedback_penalized: 0,
                live_memory_feedback_applied: 0,
                live_memory_feedback_removed: 0,
                live_memory_feedback_missing: 0,
                live_memory_feedback_strength_delta: 0.0,
                critical_reflection_issues: 0,
            }],
        };

        let joined =
            experience_retrieval_report_lines(Path::new("experience.ndkv"), &report).join("\n");

        assert!(joined.contains("prompt_chars="));
        assert!(joined.contains("lesson_chars="));
        assert!(joined.contains("usable_hint_chars="));
        assert!(joined.contains("gist_hint_count=1"));
        assert!(!joined.contains("raw retrieval prompt should stay out"));
        assert!(!joined.contains("raw matched prompt should stay out"));
        assert!(!joined.contains("raw matched lesson should stay out"));
        assert!(!joined.contains("raw gist should stay out"));
        assert!(!joined.contains("usable_hint="));
        assert!(!joined.contains("lesson="));
        assert!(!joined.contains("prompt="));
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
