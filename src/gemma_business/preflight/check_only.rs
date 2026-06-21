use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use rust_norion::{ExperienceStore, GemmaRuntimeFitSummary, HardwareAllocator, HierarchyWeights};

use crate::Args;
use crate::gemma_business::paths::gemma_smoke_base_dir;

#[derive(Debug, Clone)]
pub(crate) struct GemmaSmokeCheckOnlyReport {
    lines: Vec<String>,
    failures: Vec<String>,
}

impl GemmaSmokeCheckOnlyReport {
    pub(crate) fn passed(&self) -> bool {
        self.failures.is_empty()
    }

    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }
}

pub(crate) fn gemma_smoke_check_only_report(
    args: &Args,
    preflight_failures: &[String],
) -> GemmaSmokeCheckOnlyReport {
    let probe = args.effective_probe_report();
    let plan = HardwareAllocator::new().plan(
        probe.snapshot(),
        args.profile,
        args.prompt_token_estimate(),
        HierarchyWeights::default(),
    );
    let state_scope = state_scope(args);
    let experience = experience_safety(args, state_scope);
    let mut failures = preflight_failures.to_vec();
    failures.extend(experience.failures.clone());

    let mut lines = vec![
        "Noiron Gemma smoke check-only gate".to_owned(),
        format!(
            "gemma_smoke_check_only: passed={} failures={} starts_model=false writes_ndkv=false",
            failures.is_empty(),
            failures.len()
        ),
        format!("state_dir: {}", state_dir_summary(args)),
        format!(
            "state_files: memory={} experience={} adaptive={} trace={}",
            args.memory_path.display(),
            args.experience_path.display(),
            args.adaptive_path.display(),
            args.trace_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_owned())
        ),
        format!("state_scope: {state_scope}"),
        format!("serve_bind: {}", args.serve_bind),
        format!(
            "gemma_runtime_server: {}",
            args.gemma_runtime_server
                .as_deref()
                .unwrap_or("none")
        ),
        backend_health_line(args),
        format!(
            "hardware_probe: device={} os={} arch={} cpus={} accelerators={} pressure={:.6}",
            probe.device.as_str(),
            probe.os,
            probe.arch,
            probe.cpu_count,
            probe.accelerator_count,
            plan.pressure
        ),
        ram_vram_line(args, &probe),
        experience.summary,
        "safety_note: check-only reads configuration and existing state only; it does not start Gemma, start the model service, or write .ndkv state.".to_owned(),
    ];
    for note in experience.notes {
        lines.push(format!("safety_note: {note}"));
    }
    for failure in &failures {
        lines.push(format!("gemma_smoke_check_only_failure: {failure}"));
    }

    GemmaSmokeCheckOnlyReport { lines, failures }
}

pub(crate) fn print_gemma_smoke_check_only_report(report: &GemmaSmokeCheckOnlyReport) {
    for line in report.lines() {
        println!("{line}");
    }
}

fn backend_health_line(args: &Args) -> String {
    if let Some(server) = args.gemma_runtime_server.as_deref() {
        let reachable = gemma_runtime_reachable(server)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        return format!("backend_health: mode=gemma-http reachable={reachable} endpoint={server}");
    }

    if args.gemma_12b_runtime {
        return "backend_health: mode=gemma-command reachable=not_started_check_only endpoint=local-command".to_owned();
    }

    "backend_health: mode=built-in reachable=not_applicable endpoint=none".to_owned()
}

fn gemma_runtime_reachable(server: &str) -> Option<bool> {
    parse_http_authority_socket_addr(server)
        .map(|address| TcpStream::connect_timeout(&address, Duration::from_millis(120)).is_ok())
}

fn parse_http_authority_socket_addr(server: &str) -> Option<SocketAddr> {
    let trimmed = server.trim().trim_end_matches('/');
    let without_scheme = trimmed.strip_prefix("http://").unwrap_or(trimmed);
    if without_scheme.starts_with("https://") {
        return None;
    }
    let authority = without_scheme
        .split_once('/')
        .map(|(authority, _)| authority)
        .unwrap_or(without_scheme);
    authority.to_socket_addrs().ok()?.next()
}

fn ram_vram_line(args: &Args, probe: &rust_norion::HardwareProbeReport) -> String {
    let fit = GemmaRuntimeFitSummary::for_vram(0);
    format!(
        "ram_vram: ram_load={:.2} vram_load={:.2} vram_mib=unknown bf16_weights_mib={} q4_weights_mib={} recommended_quant={} configured_quant={}",
        probe.ram_load,
        probe.gpu_load,
        fit.bf16_weights_mib,
        fit.q4_weights_mib,
        fit.recommended_quantization,
        args.gemma_runtime_quantization
    )
}

#[derive(Debug, Clone)]
struct ExperienceSafety {
    summary: String,
    notes: Vec<String>,
    failures: Vec<String>,
}

fn experience_safety(args: &Args, state_scope: &str) -> ExperienceSafety {
    if !args.experience_path.exists() {
        return ExperienceSafety {
            summary: "experience_safety: checked=false experience_dirty=unknown error=experience_file_missing".to_owned(),
            notes: Vec::new(),
            failures: Vec::new(),
        };
    }

    match ExperienceStore::load_from_disk_kv_read_only(&args.experience_path) {
        Ok(store) => {
            let hygiene = store.hygiene_report(args.inspect_limit);
            let quarantine = store.hygiene_quarantine_plan(args.inspect_limit);
            let repair = store.legacy_metadata_repair_plan(args.inspect_limit);
            let index = store.index_report(args.inspect_limit);
            experience_safety_from_counts(
                state_scope,
                hygiene.finding_count,
                quarantine.quarantine_candidate_count,
                repair.repairable_legacy_metadata_lesson_count,
                repair.repairable_index_record_count,
                index.noisy_record_count,
                index.max_noise_penalty,
            )
        }
        Err(error) => ExperienceSafety {
            summary: format!(
                "experience_safety: checked=false experience_dirty=unknown error={error}"
            ),
            notes: Vec::new(),
            failures: vec![format!(
                "experience_state: could not read {}: {error}",
                args.experience_path.display()
            )],
        },
    }
}

fn experience_safety_from_counts(
    state_scope: &str,
    hygiene_findings: usize,
    quarantine_candidates: usize,
    repairable_legacy_metadata_lessons: usize,
    repairable_index_records: usize,
    index_noisy_records: usize,
    index_max_noise_penalty: f32,
) -> ExperienceSafety {
    let dirty = quarantine_candidates > 0
        || repairable_legacy_metadata_lessons > 0
        || repairable_index_records > 0
        || index_noisy_records > 0
        || index_max_noise_penalty > 0.0;
    let summary = format!(
        "experience_safety: checked=true experience_dirty={dirty} hygiene_findings={hygiene_findings} quarantine_candidates={quarantine_candidates} repairable_legacy_metadata_lessons={repairable_legacy_metadata_lessons} repairable_index_records={repairable_index_records} index_noisy_records={index_noisy_records} index_max_noise_penalty={index_max_noise_penalty:.6}"
    );
    let requires_isolation = dirty && state_scope != "isolated_smoke_state";
    let notes = requires_isolation.then(|| {
        "project experience is dirty; use isolated smoke state or get explicit authorization before cleanup/apply.".to_owned()
    }).into_iter().collect::<Vec<_>>();
    let failures = requires_isolation
        .then(|| {
            "experience_dirty: project experience is dirty; use isolated smoke state or explicit cleanup authorization before real Gemma".to_owned()
        })
        .into_iter()
        .collect::<Vec<_>>();

    ExperienceSafety {
        summary,
        notes,
        failures,
    }
}

fn state_dir_summary(args: &Args) -> String {
    let memory_parent = args.memory_path.parent();
    let experience_parent = args.experience_path.parent();
    let adaptive_parent = args.adaptive_path.parent();
    if memory_parent == experience_parent && experience_parent == adaptive_parent {
        return memory_parent
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| ".".to_owned());
    }
    format!(
        "mixed(memory={},experience={},adaptive={})",
        parent_display(&args.memory_path),
        parent_display(&args.experience_path),
        parent_display(&args.adaptive_path)
    )
}

fn state_scope(args: &Args) -> &'static str {
    let Some(base_dir) = gemma_smoke_base_dir(args) else {
        return "non_smoke_state";
    };
    let Some(parent) = args.experience_path.parent() else {
        return "explicit_or_mixed_state";
    };
    let parent = parent.display().to_string().replace('\\', "/");
    if parent.starts_with(&format!("{base_dir}-")) {
        "isolated_smoke_state"
    } else {
        "explicit_or_mixed_state"
    }
}

fn parent_display(path: &Path) -> String {
    path.parent()
        .map(|parent| parent.display().to_string())
        .unwrap_or_else(|| ".".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Args;

    #[test]
    fn check_only_report_lists_state_ports_hardware_backend_and_no_write_contract() {
        let args = Args::parse(vec![
            "--gemma-model-service-smoke".to_owned(),
            "--gemma-smoke-check-only".to_owned(),
            "--gemma-runtime-server".to_owned(),
            "http://127.0.0.1:9".to_owned(),
            "--memory".to_owned(),
            "target/check-only/memory.ndkv".to_owned(),
            "--experience".to_owned(),
            "target/check-only/experience.ndkv".to_owned(),
            "--adaptive".to_owned(),
            "target/check-only/adaptive.ndkv".to_owned(),
            "--ram-load".to_owned(),
            "0.61".to_owned(),
            "--gpu-load".to_owned(),
            "0.27".to_owned(),
        ]);
        let report = gemma_smoke_check_only_report(&args, &["preflight missing".to_owned()]);
        let text = report.lines().join("\n");

        assert!(!report.passed());
        assert!(text.contains("gemma_smoke_check_only: passed=false"));
        assert!(text.contains("starts_model=false writes_ndkv=false"));
        assert!(text.contains("state_dir: target/check-only"));
        assert!(text.contains("state_scope: explicit_or_mixed_state"));
        assert!(text.contains("serve_bind: 127.0.0.1:7878"));
        assert!(text.contains("gemma_runtime_server: http://127.0.0.1:9"));
        assert!(text.contains("backend_health: mode=gemma-http reachable=false"));
        assert!(text.contains("ram_vram: ram_load=0.61 vram_load=0.27"));
        assert!(text.contains("vram_mib=unknown"));
        assert!(text.contains("experience_safety: checked=false"));
        assert!(text.contains("gemma_smoke_check_only_failure: preflight missing"));
        assert!(text.contains("does not start Gemma"));
    }

    #[test]
    fn dirty_explicit_experience_requires_isolation_or_cleanup_authorization() {
        let safety = experience_safety_from_counts("explicit_or_mixed_state", 4, 2, 1, 1, 1, 0.18);

        assert!(safety.summary.contains("experience_dirty=true"));
        assert!(safety.summary.contains("repairable_index_records=1"));
        assert!(
            safety
                .notes
                .iter()
                .any(|note| note.contains("use isolated smoke state"))
        );
        assert!(
            safety
                .failures
                .iter()
                .any(|failure| failure.contains("explicit cleanup authorization"))
        );
    }

    #[test]
    fn isolated_dirty_smoke_state_is_reported_without_project_cleanup_failure() {
        let safety = experience_safety_from_counts("isolated_smoke_state", 4, 2, 1, 1, 1, 0.18);

        assert!(safety.summary.contains("experience_dirty=true"));
        assert!(safety.notes.is_empty());
        assert!(safety.failures.is_empty());
    }

    #[test]
    fn repairable_index_records_make_project_experience_dirty() {
        let safety = experience_safety_from_counts("explicit_or_mixed_state", 0, 0, 0, 1, 0, 0.0);

        assert!(safety.summary.contains("experience_dirty=true"));
        assert!(safety.summary.contains("repairable_index_records=1"));
        assert!(!safety.failures.is_empty());
    }

    #[test]
    fn default_smoke_run_directory_is_reported_as_isolated_state() {
        let args = Args::parse(vec![
            "--gemma-model-service-smoke".to_owned(),
            "--gemma-smoke-check-only".to_owned(),
        ]);

        assert_eq!(state_scope(&args), "isolated_smoke_state");
    }
}
