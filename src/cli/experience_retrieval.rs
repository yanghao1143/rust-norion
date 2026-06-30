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
            "experience_retrieval: total_records={} requested_limit={} matches={} skipped_cross_task_pollution={} development_evidence_surface_blocked_candidates={} retrieval_noise_penalized_candidates={} retrieval_noise_filtered_candidates={} suppressed_prompt_index_candidates={} max_retrieval_noise_penalty={} max_score={}",
            report.total_records,
            report.requested_limit,
            report.match_count(),
            report.skipped_cross_task_pollution,
            report.development_evidence_surface_blocked_candidates,
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
            "  id={} score={:.6} quality={:.3} reward={:.3} action={} runtime_model={} adapter={} device={} stored_runtime_kv_memory_ids={} recursive_runtime_calls={} usable_hint={} lesson={} prompt={}",
            item.id,
            item.score,
            item.quality,
            item.process_reward,
            item.reward_action.as_str(),
            option_text(item.runtime_model_id.as_deref()),
            option_text(item.runtime_selected_adapter.as_deref()),
            option_text(item.runtime_device_profile.as_deref()),
            u64_list_text(&item.stored_runtime_kv_memory_ids),
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

fn u64_list_text(values: &[u64]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
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
            development_evidence_surface_blocked_candidates: 1,
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
        assert!(summary.contains("development_evidence_surface_blocked_candidates=1"));
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

    #[test]
    fn retrieval_report_lines_include_runtime_kv_memory_ids() {
        let report = ExperienceRetrievalReport {
            prompt: "runtime kv retrieval".to_owned(),
            profile: TaskProfile::Coding,
            total_records: 1,
            requested_limit: 1,
            skipped_cross_task_pollution: 0,
            development_evidence_surface_blocked_candidates: 0,
            retrieval_noise_penalized_candidates: 0,
            retrieval_noise_filtered_candidates: 0,
            suppressed_prompt_index_candidates: 0,
            max_retrieval_noise_penalty: 0.0,
            matches: vec![ExperienceMatch {
                id: 7,
                prompt: "runtime kv prompt".to_owned(),
                lesson: "reuse runtime kv memory".to_owned(),
                quality: 0.9,
                score: 0.8,
                gist_hints: Vec::new(),
                reflection_issue_codes: Vec::new(),
                revision_actions: Vec::new(),
                process_reward: 0.7,
                reward_action: RewardAction::Reinforce,
                used_memory_count: 1,
                stored_runtime_kv_memory_ids: vec![11, 13],
                route_threshold: 0.5,
                route_attention_tokens: 1,
                route_fast_tokens: 1,
                route_attention_fraction: 0.5,
                runtime_model_id: Some("noiron-runtime".to_owned()),
                runtime_selected_adapter: Some("portable-rust".to_owned()),
                runtime_device_profile: Some("cpu".to_owned()),
                runtime_primary_lane: None,
                runtime_fallback_lane: None,
                runtime_memory_mode: Some("kv".to_owned()),
                runtime_device_execution_source: None,
                runtime_forward_energy: None,
                runtime_kv_influence: Some(0.42),
                runtime_uncertainty_perplexity: None,
                recursive_runtime_calls: Some(2),
            }],
        };

        let lines = experience_retrieval_report_lines(Path::new("experience.ndkv"), &report);
        let match_line = lines.iter().find(|line| line.contains("id=7")).unwrap();

        assert!(match_line.contains("stored_runtime_kv_memory_ids=11,13"));
        assert!(match_line.contains("runtime_model=noiron-runtime"));
        assert!(match_line.contains("recursive_runtime_calls=2"));
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
