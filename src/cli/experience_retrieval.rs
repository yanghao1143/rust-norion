use std::io;
use std::path::Path;

use rust_norion::{
    ExperienceRecord, ExperienceRetrievalReport, ExperienceStore, KvFusionCache, TenantScope,
    render_experience_hint,
};

use crate::Args;

pub(crate) fn run_experience_retrieval_report(
    args: &Args,
) -> io::Result<ExperienceRetrievalReport> {
    let store = ExperienceStore::load_from_disk_kv_read_only(&args.experience_path)?;
    let scope = experience_retrieval_scope(args);
    let visible_memory_ids = scoped_memory_ids(&args.memory_path, &scope)?;
    Ok(store.retrieval_report_matching(
        &args.prompt,
        args.profile,
        args.experience_retrieval_limit,
        |record| record_has_visible_memory(record, &visible_memory_ids),
    ))
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

fn experience_retrieval_scope(args: &Args) -> TenantScope {
    TenantScope::new(
        &args.experience_retrieval_tenant,
        &args.experience_retrieval_workspace,
        &args.experience_retrieval_session,
    )
}

fn scoped_memory_ids(memory_path: &Path, scope: &TenantScope) -> io::Result<Vec<u64>> {
    let Some(cache) = KvFusionCache::load_from_disk_kv_read_only_existing(memory_path)? else {
        return Ok(Vec::new());
    };
    Ok(cache
        .entries_scoped(scope)
        .into_iter()
        .map(|entry| entry.id)
        .collect())
}

fn record_has_visible_memory(record: &ExperienceRecord, visible_memory_ids: &[u64]) -> bool {
    record
        .stored_memory_id
        .is_some_and(|id| visible_memory_ids.contains(&id))
        || record
            .used_memory_ids
            .iter()
            .chain(record.gist_memory_ids.iter())
            .chain(record.stored_runtime_kv_memory_ids.iter())
            .any(|id| visible_memory_ids.contains(id))
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
    use rust_norion::{
        ExperienceInput, ExperienceMatch, ExperienceRuntimeTokenMetrics, HierarchyWeights,
        KvFusionCache, LiveInferenceEvolution, ProcessRewardReport, RewardAction, RouteBudget,
        RuntimeDiagnostics, TaskProfile, TenantResourceLane, TenantScope,
    };
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
    fn cli_retrieval_filters_records_by_tenant_scoped_memory_ids() {
        let memory_path = temp_path("retrieval-scoped-memory");
        let experience_path = temp_path("retrieval-scoped-experience");
        let tenant_a = TenantScope::new("tenant-a", "workspace", "session-a");
        let tenant_b = TenantScope::new("tenant-b", "workspace", "session-b");
        let mut cache = KvFusionCache::new();
        let memory_a = cache.store_scoped_or_fuse(
            &tenant_a,
            TenantResourceLane::KvMemory,
            "retrieval-a",
            vec![1.0, 0.0],
            0.9,
        );
        let memory_b = cache.store_scoped_or_fuse(
            &tenant_b,
            TenantResourceLane::KvMemory,
            "retrieval-b",
            vec![0.0, 1.0],
            0.9,
        );
        cache.save_to_disk_kv(&memory_path).unwrap();

        let mut store = ExperienceStore::new();
        let visible_id = store.record(ExperienceInput {
            prompt: "Rust scoped retrieval tenant memory".to_owned(),
            lesson: "tenant A scoped retrieval lesson".to_owned(),
            stored_memory_id: Some(memory_a),
            used_memory_ids: vec![memory_a],
            stored_runtime_kv_memory_ids: vec![memory_a],
            ..input("tenant a scoped retrieval", 0.92)
        });
        let hidden_id = store.record(ExperienceInput {
            prompt: "Rust scoped retrieval tenant memory".to_owned(),
            lesson: "tenant B scoped retrieval lesson".to_owned(),
            stored_memory_id: Some(memory_b),
            used_memory_ids: vec![memory_b],
            stored_runtime_kv_memory_ids: vec![memory_b],
            ..input("tenant b scoped retrieval", 0.95)
        });
        store.save_to_disk_kv(&experience_path).unwrap();

        let args = Args::parse(vec![
            "--memory".to_owned(),
            memory_path.display().to_string(),
            "--experience".to_owned(),
            experience_path.display().to_string(),
            "--experience-retrieval".to_owned(),
            "--experience-retrieval-tenant".to_owned(),
            "tenant-a".to_owned(),
            "--experience-retrieval-workspace".to_owned(),
            "workspace".to_owned(),
            "--experience-retrieval-session".to_owned(),
            "session-a".to_owned(),
            "Rust scoped retrieval tenant memory".to_owned(),
        ]);

        let report = run_experience_retrieval_report(&args).unwrap();

        assert_eq!(report.total_records, 1);
        assert_eq!(report.match_count(), 1);
        assert_eq!(report.matches[0].id, visible_id);
        assert!(report.matches.iter().all(|item| item.id != hidden_id));

        cleanup(memory_path);
        cleanup(experience_path);
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

    fn cleanup(path: std::path::PathBuf) {
        let _ = fs::remove_file(path);
    }

    fn input(lesson: &str, quality: f32) -> ExperienceInput {
        ExperienceInput {
            prompt: "Rust scoped retrieval tenant memory".to_owned(),
            profile: TaskProfile::Coding,
            lesson: lesson.to_owned(),
            quality,
            contradictions: Vec::new(),
            reflection_issues: Vec::new(),
            revision_actions: Vec::new(),
            stored_memory_id: None,
            router_threshold_after: 0.55,
            stream_windows: 1,
            route_budget: RouteBudget {
                threshold: 0.55,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::default(),
            used_memory_ids: Vec::new(),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: RuntimeDiagnostics::default(),
            runtime_token_metrics: ExperienceRuntimeTokenMetrics::default(),
            process_reward: ProcessRewardReport::default(),
            live_evolution: LiveInferenceEvolution::default(),
        }
    }
}
