use std::io::{self, Write};

use super::provider::ChatProvider;

pub fn require_prompt_preflight(
    provider: &dyn ChatProvider,
    require_safe_device: bool,
) -> io::Result<String> {
    match provider.prompt_preflight(require_safe_device) {
        Ok(summary) => {
            record_provider_event(
                provider,
                "preflight",
                &format!("require_safe_device={require_safe_device} {summary}"),
            );
            Ok(summary)
        }
        Err(error) => {
            record_provider_event(
                provider,
                "preflight_error",
                &format!("require_safe_device={require_safe_device} {error}"),
            );
            Err(io::Error::other(format!(
                "backend preflight failed: {error}"
            )))
        }
    }
}

pub fn run_health_check(provider: &dyn ChatProvider) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_health_check_to(provider, &mut stdout)
}

pub fn run_experience_hygiene_check(provider: &dyn ChatProvider) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_experience_hygiene_check_to(provider, &mut stdout)
}

pub fn run_experience_hygiene_quarantine_dry_run(
    provider: &dyn ChatProvider,
    limit: usize,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_experience_hygiene_quarantine_dry_run_to(provider, limit, &mut stdout)
}

pub fn run_experience_repair_dry_run(provider: &dyn ChatProvider, limit: usize) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_experience_repair_dry_run_to(provider, limit, &mut stdout)
}

pub fn run_experience_cleanup_audit(provider: &dyn ChatProvider, limit: usize) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_experience_cleanup_audit_to(provider, limit, &mut stdout)
}

pub fn run_preflight_check(
    provider: &dyn ChatProvider,
    require_safe_device: bool,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_preflight_check_to(provider, require_safe_device, &mut stdout)
}

fn run_health_check_to<W: Write>(provider: &dyn ChatProvider, output: &mut W) -> io::Result<()> {
    match provider.health_check() {
        Ok(summary) => {
            record_provider_event(provider, "health_check", &summary);
            writeln!(output, "status: {summary}")?;
            output.flush()
        }
        Err(error) => {
            record_provider_event(provider, "health_check_error", &error);
            Err(io::Error::other(format!("health check failed: {error}")))
        }
    }
}

fn run_experience_hygiene_check_to<W: Write>(
    provider: &dyn ChatProvider,
    output: &mut W,
) -> io::Result<()> {
    match provider.experience_hygiene() {
        Ok(summary) => {
            record_provider_event(provider, "experience_hygiene", &summary);
            writeln!(output, "{summary}")?;
            output.flush()
        }
        Err(error) => {
            record_provider_event(provider, "experience_hygiene_error", &error);
            Err(io::Error::other(format!(
                "experience hygiene check failed: {error}"
            )))
        }
    }
}

fn run_experience_hygiene_quarantine_dry_run_to<W: Write>(
    provider: &dyn ChatProvider,
    limit: usize,
    output: &mut W,
) -> io::Result<()> {
    match provider.experience_hygiene_quarantine_dry_run(limit) {
        Ok(summary) => {
            record_provider_event(provider, "experience_hygiene_quarantine_dry_run", &summary);
            writeln!(output, "{summary}")?;
            output.flush()
        }
        Err(error) => {
            record_provider_event(provider, "experience_hygiene_quarantine_error", &error);
            Err(io::Error::other(format!(
                "experience hygiene quarantine dry-run failed: {error}"
            )))
        }
    }
}

fn run_experience_repair_dry_run_to<W: Write>(
    provider: &dyn ChatProvider,
    limit: usize,
    output: &mut W,
) -> io::Result<()> {
    match provider.experience_repair_dry_run(limit) {
        Ok(summary) => {
            record_provider_event(provider, "experience_repair_dry_run", &summary);
            writeln!(output, "{summary}")?;
            output.flush()
        }
        Err(error) => {
            record_provider_event(provider, "experience_repair_error", &error);
            Err(io::Error::other(format!(
                "experience repair dry-run failed: {error}"
            )))
        }
    }
}

fn run_experience_cleanup_audit_to<W: Write>(
    provider: &dyn ChatProvider,
    limit: usize,
    output: &mut W,
) -> io::Result<()> {
    match provider.experience_cleanup_audit(limit.max(1)) {
        Ok(summary) => {
            record_provider_event(provider, "experience_cleanup_audit", &summary);
            writeln!(output, "{summary}")?;
            output.flush()
        }
        Err(error) => {
            record_provider_event(provider, "experience_cleanup_audit_error", &error);
            Err(io::Error::other(format!(
                "experience cleanup audit failed: {error}"
            )))
        }
    }
}

fn run_preflight_check_to<W: Write>(
    provider: &dyn ChatProvider,
    require_safe_device: bool,
    output: &mut W,
) -> io::Result<()> {
    let summary = require_prompt_preflight(provider, require_safe_device)?;
    writeln!(output, "preflight: PASS {summary}")?;
    output.flush()
}

fn record_provider_event(provider: &dyn ChatProvider, kind: &str, content: &str) {
    let _ = provider.record_event(kind, content);
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    };

    use super::*;
    use crate::app::provider::ProviderEvent;

    #[derive(Clone, Default)]
    struct HealthyProvider;

    impl ChatProvider for HealthyProvider {
        fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
            let (_tx, rx) = mpsc::channel();
            rx
        }

        fn health_check(&self) -> Result<String, String> {
            Ok("mock provider ready".to_owned())
        }
    }

    #[derive(Clone, Default)]
    struct FailingProvider;

    impl ChatProvider for FailingProvider {
        fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
            let (_tx, rx) = mpsc::channel();
            rx
        }

        fn health_check(&self) -> Result<String, String> {
            Err("backend offline".to_owned())
        }
    }

    #[derive(Clone, Default)]
    struct NotReadyProvider;

    impl ChatProvider for NotReadyProvider {
        fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
            let (_tx, rx) = mpsc::channel();
            rx
        }

        fn health_check(&self) -> Result<String, String> {
            Ok("service=rust-norion ok=true runtime=gemma-http".to_owned())
        }

        fn readiness_check(&self) -> Result<String, String> {
            Err("Gemma runtime is not reachable".to_owned())
        }
    }

    #[derive(Clone, Default)]
    struct RecordingProvider {
        events: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl RecordingProvider {
        fn events(&self) -> Vec<(String, String)> {
            self.events.lock().unwrap().clone()
        }
    }

    impl ChatProvider for RecordingProvider {
        fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
            let (_tx, rx) = mpsc::channel();
            rx
        }

        fn health_check(&self) -> Result<String, String> {
            Ok("recording provider ready".to_owned())
        }

        fn experience_hygiene(&self) -> Result<String, String> {
            Ok("Noiron experience hygiene\nreport: clean=false".to_owned())
        }

        fn experience_hygiene_quarantine_dry_run(&self, limit: usize) -> Result<String, String> {
            Ok(format!(
                "Noiron experience hygiene quarantine dry-run\nlimit={limit}\napplied=false"
            ))
        }

        fn experience_repair_dry_run(&self, limit: usize) -> Result<String, String> {
            Ok(format!(
                "Noiron experience repair dry-run\nlimit={limit}\napplied=false"
            ))
        }

        fn experience_cleanup_audit(&self, limit: usize) -> Result<String, String> {
            Ok(format!(
                "Noiron experience cleanup audit\nwrites_experience_state=false\nsample_limit={limit}\n\n## Hygiene\nclean=false\n\n## Quarantine dry-run\napplied=false\n\n## Repair dry-run\napplied=false"
            ))
        }

        fn record_event(&self, kind: &str, content: &str) -> Result<(), String> {
            self.events
                .lock()
                .unwrap()
                .push((kind.to_owned(), content.to_owned()));
            Ok(())
        }
    }

    #[test]
    fn health_check_accepts_healthy_provider() {
        let mut output = Vec::new();

        assert!(run_health_check_to(&HealthyProvider, &mut output).is_ok());
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "status: mock provider ready\n"
        );
    }

    #[test]
    fn health_check_returns_backend_errors() {
        let mut output = Vec::new();
        let error = run_health_check_to(&FailingProvider, &mut output).unwrap_err();

        assert!(error.to_string().contains("backend offline"));
        assert!(output.is_empty());
    }

    #[test]
    fn prompt_preflight_returns_summary_without_printing() {
        assert_eq!(
            require_prompt_preflight(&HealthyProvider, false).unwrap(),
            "mock provider ready"
        );
    }

    #[test]
    fn prompt_preflight_rejects_unhealthy_provider() {
        let error = require_prompt_preflight(&FailingProvider, false).unwrap_err();

        assert!(error.to_string().contains("backend preflight failed"));
    }

    #[test]
    fn prompt_preflight_rejects_not_ready_provider() {
        let error = require_prompt_preflight(&NotReadyProvider, false).unwrap_err();

        assert!(error.to_string().contains("Gemma runtime is not reachable"));
    }

    #[test]
    fn prompt_preflight_can_reject_unsafe_device_summary() {
        #[derive(Clone, Default)]
        struct UnsafeProvider;

        impl ChatProvider for UnsafeProvider {
            fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
                let (_tx, rx) = mpsc::channel();
                rx
            }

            fn readiness_check(&self) -> Result<String, String> {
                Ok("runtime=gemma-command lane=cpu-vector warnings=gemma_12b_device".to_owned())
            }
        }

        let error = require_prompt_preflight(&UnsafeProvider, true).unwrap_err();

        assert!(error.to_string().contains("safe-device guard"));
    }

    #[test]
    fn preflight_check_prints_pass_without_dispatching() {
        let mut output = Vec::new();

        run_preflight_check_to(&HealthyProvider, false, &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "preflight: PASS mock provider ready\n"
        );
    }

    #[test]
    fn health_and_preflight_checks_record_provider_events() {
        let provider = RecordingProvider::default();
        let mut output = Vec::new();

        run_health_check_to(&provider, &mut output).unwrap();
        require_prompt_preflight(&provider, true).unwrap();

        let events = provider.events();
        assert_eq!(events[0].0, "health_check");
        assert!(events[0].1.contains("recording provider ready"));
        assert_eq!(events[1].0, "preflight");
        assert!(events[1].1.contains("require_safe_device=true"));
    }

    #[test]
    fn hygiene_check_prints_report_and_records_event() {
        let provider = RecordingProvider::default();
        let mut output = Vec::new();

        run_experience_hygiene_check_to(&provider, &mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("Noiron experience hygiene"));
        let events = provider.events();
        assert_eq!(events[0].0, "experience_hygiene");
    }

    #[test]
    fn hygiene_quarantine_dry_run_prints_report_and_records_event() {
        let provider = RecordingProvider::default();
        let mut output = Vec::new();

        run_experience_hygiene_quarantine_dry_run_to(&provider, 20, &mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("quarantine dry-run"));
        assert!(text.contains("applied=false"));
        let events = provider.events();
        assert_eq!(events[0].0, "experience_hygiene_quarantine_dry_run");
    }

    #[test]
    fn repair_dry_run_prints_report_and_records_event() {
        let provider = RecordingProvider::default();
        let mut output = Vec::new();

        run_experience_repair_dry_run_to(&provider, 20, &mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("repair dry-run"));
        assert!(text.contains("applied=false"));
        let events = provider.events();
        assert_eq!(events[0].0, "experience_repair_dry_run");
    }

    #[test]
    fn cleanup_audit_prints_combined_read_only_report_and_records_event() {
        let provider = RecordingProvider::default();
        let mut output = Vec::new();

        run_experience_cleanup_audit_to(&provider, 7, &mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("Noiron experience cleanup audit"));
        assert!(text.contains("writes_experience_state=false"));
        assert!(text.contains("sample_limit=7"));
        assert!(text.contains("## Quarantine dry-run"));
        let events = provider.events();
        assert_eq!(events[0].0, "experience_cleanup_audit");
    }
}
