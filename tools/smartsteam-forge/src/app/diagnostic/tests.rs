use std::sync::mpsc::{self, Receiver};

use super::*;
use crate::app::provider::ProviderEvent;

#[derive(Clone, Default)]
struct ReadyProvider;

impl ChatProvider for ReadyProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn diagnostic_target(&self) -> String {
        "mock".to_owned()
    }

    fn health_check(&self) -> Result<String, String> {
        Ok("service=mock ok=true".to_owned())
    }
}

#[derive(Clone, Default)]
struct OfflineProvider;

impl ChatProvider for OfflineProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn diagnostic_target(&self) -> String {
        "127.0.0.1:7878".to_owned()
    }

    fn health_check(&self) -> Result<String, String> {
        Err("connect backend failed: connection timed out".to_owned())
    }
}

#[derive(Clone, Default)]
struct UnreachableGemmaProvider;

impl ChatProvider for UnreachableGemmaProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn health_check(&self) -> Result<String, String> {
        Ok("service=rust-norion ok=true runtime=gemma-http gemma_reachable=false".to_owned())
    }

    fn readiness_check(&self) -> Result<String, String> {
        Err("Gemma runtime is not reachable server=http://127.0.0.1:8686".to_owned())
    }
}

#[derive(Clone, Default)]
struct CpuFirstGemmaProvider;

impl ChatProvider for CpuFirstGemmaProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn health_check(&self) -> Result<String, String> {
        Ok("service=rust-norion ok=true runtime=gemma-command device=cpu lane=cpu-vector warnings=gemma_12b_device: selected plan is CPU/disk-first".to_owned())
    }
}

#[derive(Clone, Default)]
struct BusyProvider;

impl ChatProvider for BusyProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn health_check(&self) -> Result<String, String> {
        Ok("service=rust-norion ok=true busy=true active_request=#42:chat-stream:1234ms active_prompt=\"联调 prompt\"".to_owned())
    }

    fn readiness_check(&self) -> Result<String, String> {
        Err("后端正在忙 (backend engine is busy): request_id=42 endpoint=chat-stream elapsed_ms=1234 prompt_preview=\"联调 prompt\"".to_owned())
    }
}

#[derive(Clone, Default)]
struct DirtyExperienceProvider;

impl ChatProvider for DirtyExperienceProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn health_check(&self) -> Result<String, String> {
        Ok("service=rust-norion ok=true experience_hygiene_clean=false experience_hygiene_quarantine_candidates=4".to_owned())
    }

    fn readiness_check(&self) -> Result<String, String> {
        Err(
            "backend experience hygiene failed: experience_hygiene quarantine_candidates=4"
                .to_owned(),
        )
    }
}

#[derive(Clone, Default)]
struct RepairableExperienceProvider;

impl ChatProvider for RepairableExperienceProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn health_check(&self) -> Result<String, String> {
        Ok("service=rust-norion ok=true experience_hygiene_clean=false repairable_legacy_metadata_lessons=828".to_owned())
    }

    fn readiness_check(&self) -> Result<String, String> {
        Err(
            "backend experience hygiene failed: experience_repair repairable_legacy_metadata_lessons=828 projected_findings_after_repair=32"
                .to_owned(),
        )
    }
}

#[derive(Clone, Default)]
struct RepairableIndexExperienceProvider;

impl ChatProvider for RepairableIndexExperienceProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn health_check(&self) -> Result<String, String> {
        Ok(
            "service=rust-norion ok=true experience_hygiene_clean=false repairable_index_records=1"
                .to_owned(),
        )
    }

    fn readiness_check(&self) -> Result<String, String> {
        Err(
            "backend experience hygiene failed: experience_repair repairable_index_records=1 projected_findings_after_repair=0"
                .to_owned(),
        )
    }
}

#[derive(Clone, Default)]
struct BlockedExperienceIndexProvider;

impl ChatProvider for BlockedExperienceIndexProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn health_check(&self) -> Result<String, String> {
        Ok("service=rust-norion ok=true experience_hygiene_clean=true experience_index_retrieval_ready=false experience_index_risk_level=blocked experience_index_quality_score=0.340000".to_owned())
    }

    fn readiness_check(&self) -> Result<String, String> {
        Err(
            "backend experience hygiene failed: experience_index retrieval_ready=false risk_level=blocked quality_score=0.340000"
                .to_owned(),
        )
    }
}

#[test]
fn diagnostic_reports_ready_backend() {
    let report = build_diagnostic_report(&ReadyProvider);

    assert!(report.contains("SmartSteam Forge doctor"));
    assert!(report.contains("health: PASS"));
    assert!(report.contains("readiness: PASS"));
    assert!(report.contains("safe-device: PASS"));
    assert!(report.contains("后端已 ready"));
}

#[test]
fn diagnostic_reports_offline_backend_with_start_hint() {
    let report = build_diagnostic_report(&OfflineProvider);

    assert!(report.contains("health: FAIL"));
    assert!(report.contains("readiness: FAIL"));
    assert!(report.contains("safe-device: FAIL"));
    assert!(report.contains("cargo run -- --serve --serve-bind 127.0.0.1:7878"));
    assert!(report.contains("--connect-timeout-ms 500"));
    assert!(report.contains("--read-timeout-ms 是单次 read 轮询/heartbeat 间隔"));
    assert!(report.contains("真实 Gemma 流式总等待窗口用 --timeout-secs"));
}

#[test]
fn diagnostic_reports_unreachable_gemma_hint() {
    let report = build_diagnostic_report(&UnreachableGemmaProvider);

    assert!(report.contains("health: PASS"));
    assert!(report.contains("readiness: FAIL"));
    assert!(report.contains("safe-device: FAIL"));
    assert!(report.contains("--gemma-runtime-server"));
}

#[test]
fn diagnostic_warns_when_ready_backend_is_cpu_first_for_gemma() {
    let report = build_diagnostic_report(&CpuFirstGemmaProvider);

    assert!(report.contains("health: PASS"));
    assert!(report.contains("readiness: PASS"));
    assert!(report.contains("safe-device: FAIL"));
    assert!(report.contains("CPU/disk-first"));
    assert!(report.contains("GPU-backed"));
}

#[test]
fn diagnostic_reports_busy_backend_with_active_request_hint() {
    let report = build_diagnostic_report(&BusyProvider);

    assert!(report.contains("readiness: FAIL"));
    assert!(report.contains("request_id=42"));
    assert!(report.contains("prompt_preview"));
    assert!(report.contains("active_requests"));
    assert!(report.contains("curl.exe -s http://127.0.0.1:7878/health"));
}

#[test]
fn diagnostic_reports_experience_hygiene_hint() {
    let report = build_diagnostic_report(&DirtyExperienceProvider);

    assert!(report.contains("readiness: FAIL"));
    assert!(report.contains("/v1/experience-hygiene"));
    assert!(report.contains("dry-run quarantine"));
}

#[test]
fn diagnostic_reports_experience_repair_hint() {
    let report = build_diagnostic_report(&RepairableExperienceProvider);

    assert!(report.contains("readiness: FAIL"));
    assert!(report.contains("repairable_legacy_metadata_lessons=828"));
    assert!(report.contains("--experience-repair"));
}

#[test]
fn diagnostic_reports_repairable_index_records_hint() {
    let report = build_diagnostic_report(&RepairableIndexExperienceProvider);

    assert!(report.contains("readiness: FAIL"));
    assert!(report.contains("repairable_index_records=1"));
    assert!(report.contains("--experience-repair"));
    assert!(report.contains("--experience-cleanup-audit"));
}

#[test]
fn diagnostic_reports_blocked_experience_index_hint() {
    let report = build_diagnostic_report(&BlockedExperienceIndexProvider);

    assert!(report.contains("readiness: FAIL"));
    assert!(report.contains("experience_index"));
    assert!(report.contains("retrieval_ready=false"));
    assert!(report.contains("risk_level=blocked"));
    assert!(report.contains("--experience-cleanup-audit"));
}
