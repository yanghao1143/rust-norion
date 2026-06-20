use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver},
};

use super::format::{WatchErrorJsonSummary, WatchIterationJsonSummary};
use super::*;
use crate::app::provider::ProviderEvent;

#[derive(Clone, Default)]
struct WatchProvider {
    events: Arc<Mutex<Vec<(String, String)>>>,
    fail_status: bool,
}

impl ChatProvider for WatchProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn model_pool_status(&self) -> Result<String, String> {
        if self.fail_status {
            return Err("backend busy".to_owned());
        }
        Ok("pool ok".to_owned())
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
fn watch_cli_repeats_provider_pool_status_for_limited_iterations() {
    let provider = WatchProvider::default();
    let mut output = Vec::new();

    run_model_pool_watch_to(&provider, Duration::ZERO, Some(2), &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    let events = provider.events.lock().unwrap();

    assert!(output.contains("model_pool_watch iteration=1"));
    assert!(output.contains("model_pool_watch iteration=2"));
    assert_eq!(output.matches("section=watch_iteration_json").count(), 2);
    let iteration_json = section_json_after(&output, "section=watch_iteration_json")
        .expect("watch_iteration_json section should include a JSON payload line");
    assert_eq!(
        super::format::watch_iteration_json_summary(iteration_json).unwrap(),
        WatchIterationJsonSummary {
            iteration: "1".to_owned()
        }
    );
    assert_eq!(output.matches("pool ok").count(), 2);
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|(kind, _)| kind == "model_pool_watch"));
}

#[test]
fn watch_cli_records_and_prints_status_errors_without_stopping_limited_watch() {
    let provider = WatchProvider {
        fail_status: true,
        ..WatchProvider::default()
    };
    let mut output = Vec::new();

    run_model_pool_watch_to(&provider, Duration::ZERO, Some(1), &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    let events = provider.events.lock().unwrap();

    assert!(output.contains("model_pool_watch iteration=1"));
    assert!(output.contains("section=watch_iteration_json"));
    assert!(output.contains("model_pool_watch_error iteration=1 error=backend busy"));
    assert!(output.contains("section=watch_error_json"));
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "model_pool_watch_error");
    assert!(events[0].1.contains("model_pool_watch_error iteration=1"));
    let error_json = events[0]
        .1
        .lines()
        .skip_while(|line| *line != "section=watch_error_json")
        .nth(1)
        .expect("watch_error_json section should include a JSON payload line");
    assert_eq!(
        super::format::watch_error_json_summary(error_json).unwrap(),
        WatchErrorJsonSummary {
            iteration: "1".to_owned(),
            error: "backend busy".to_owned(),
        }
    );
}

fn section_json_after<'a>(text: &'a str, section: &str) -> Option<&'a str> {
    text.lines().skip_while(|line| *line != section).nth(1)
}
