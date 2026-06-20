use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver},
};

use super::super::command_output::{ModelPoolErrorJsonSummary, model_pool_error_json_summary};
use super::*;
use crate::app::provider::ProviderEvent;

#[derive(Clone, Default)]
struct ReadOnlyProvider {
    events: Arc<Mutex<Vec<(String, String)>>>,
}

#[derive(Clone, Default)]
struct FailingReadOnlyProvider {
    events: Arc<Mutex<Vec<(String, String)>>>,
}

impl ChatProvider for ReadOnlyProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn model_pool_status(&self) -> Result<String, String> {
        Ok("pool ok".to_owned())
    }

    fn model_pool_manifest(&self) -> Result<String, String> {
        Ok("manifest ok".to_owned())
    }

    fn record_event(&self, kind: &str, content: &str) -> Result<(), String> {
        self.events
            .lock()
            .unwrap()
            .push((kind.to_owned(), content.to_owned()));
        Ok(())
    }
}

impl ChatProvider for FailingReadOnlyProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn model_pool_status(&self) -> Result<String, String> {
        Err("backend busy".to_owned())
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
fn status_cli_prints_provider_pool_status() {
    let provider = ReadOnlyProvider::default();
    let mut output = Vec::new();

    run_model_pool_status_to(&provider, &mut output).unwrap();

    assert_eq!(String::from_utf8(output).unwrap(), "pool ok\n");
    assert!(
        provider
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, content)| { kind == "model_pool_status" && content == "pool ok" })
    );
}

#[test]
fn manifest_cli_prints_provider_pool_manifest() {
    let provider = ReadOnlyProvider::default();
    let mut output = Vec::new();

    run_model_pool_manifest_to(&provider, &mut output).unwrap();

    assert_eq!(String::from_utf8(output).unwrap(), "manifest ok\n");
    assert!(
        provider
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, content)| { kind == "model_pool_manifest" && content == "manifest ok" })
    );
}

#[test]
fn advice_cli_prints_pool_advice() {
    let provider = ReadOnlyProvider::default();
    let mut output = Vec::new();

    run_model_pool_advice_to(&provider, &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();

    assert!(output.contains("SmartSteam Apple model pool advice"));
    assert!(output.contains("read_only=true"));
    assert!(output.contains("sends_prompt=false"));
    assert!(
        provider
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, content)| {
                kind == "model_pool_advice"
                    && content.contains("SmartSteam Apple model pool advice")
            })
    );
}

#[test]
fn status_cli_records_machine_readable_error_event() {
    let provider = FailingReadOnlyProvider::default();
    let mut output = Vec::new();

    let error = run_model_pool_status_to(&provider, &mut output).unwrap_err();

    assert_eq!(error.to_string(), "model pool status failed: backend busy");
    assert!(output.is_empty());
    let events = provider.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "model_pool_status_error");
    assert!(events[0].1.contains("section=error_json"));
    let error_json = events[0]
        .1
        .lines()
        .skip_while(|line| *line != "section=error_json")
        .nth(1)
        .expect("error_json section should include a JSON payload line");

    assert_eq!(
        model_pool_error_json_summary(error_json).unwrap(),
        ModelPoolErrorJsonSummary {
            action: "status".to_owned(),
            error: "backend busy".to_owned(),
            user_message: "model pool status failed: backend busy".to_owned(),
        }
    );
}
