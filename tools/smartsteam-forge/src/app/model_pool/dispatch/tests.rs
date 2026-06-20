use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver},
};

use super::super::command_output::{ModelPoolErrorJsonSummary, model_pool_error_json_summary};
use super::*;
use crate::app::provider::ProviderEvent;
use crate::app::status_json::{json_bool_field, json_string_field};
use smartsteam_forge::{MODEL_POOL_INDEX_NOTE_END_MARKER, MODEL_POOL_INDEX_NOTE_MARKER};

#[derive(Clone, Default)]
struct DispatchProvider {
    notes: Arc<Mutex<Vec<String>>>,
    events: Arc<Mutex<Vec<(String, String)>>>,
}

impl ChatProvider for DispatchProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn model_pool_route(&self, task_kind: &str) -> Result<String, String> {
        Ok(format!("route task_kind={task_kind}"))
    }

    fn model_pool_call(&self, task_kind: &str, prompt: &str) -> Result<String, String> {
        Ok(format!(
            "call task_kind={task_kind} prompt={prompt} answer=src/session handles context"
        ))
    }

    fn add_project_note(&self, note: &str) -> Result<String, String> {
        self.notes.lock().unwrap().push(note.to_owned());
        Ok("project_notes=1 chars path=state/project_notes.md".to_owned())
    }

    fn record_event(&self, kind: &str, content: &str) -> Result<(), String> {
        self.events
            .lock()
            .unwrap()
            .push((kind.to_owned(), content.to_owned()));
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FailingDispatchProvider {
    events: Arc<Mutex<Vec<(String, String)>>>,
}

impl ChatProvider for FailingDispatchProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn model_pool_call(&self, _task_kind: &str, _prompt: &str) -> Result<String, String> {
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

#[derive(Clone, Default)]
struct DriftedIndexDispatchProvider {
    notes: Arc<Mutex<Vec<String>>>,
    events: Arc<Mutex<Vec<(String, String)>>>,
}

impl ChatProvider for DriftedIndexDispatchProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn model_pool_call(&self, task_kind: &str, _prompt: &str) -> Result<String, String> {
        Ok(format!(
            "call task_kind={task_kind}\nselected_role=review\nselected_base_url=http://127.0.0.1:8688\nanswer=review answer should not become index"
        ))
    }

    fn add_project_note(&self, note: &str) -> Result<String, String> {
        self.notes.lock().unwrap().push(note.to_owned());
        Ok("project_notes=updated".to_owned())
    }

    fn record_event(&self, kind: &str, content: &str) -> Result<(), String> {
        self.events
            .lock()
            .unwrap()
            .push((kind.to_owned(), content.to_owned()));
        Ok(())
    }
}

#[derive(Clone, Default)]
struct SameLineDriftedIndexDispatchProvider {
    notes: Arc<Mutex<Vec<String>>>,
    events: Arc<Mutex<Vec<(String, String)>>>,
}

impl ChatProvider for SameLineDriftedIndexDispatchProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn model_pool_call(&self, task_kind: &str, _prompt: &str) -> Result<String, String> {
        Ok(format!(
            "call task_kind={task_kind} selected_role=review selected_base_url=http://127.0.0.1:8688 answer=review answer should not become index"
        ))
    }

    fn add_project_note(&self, note: &str) -> Result<String, String> {
        self.notes.lock().unwrap().push(note.to_owned());
        Ok("project_notes=updated".to_owned())
    }

    fn record_event(&self, kind: &str, content: &str) -> Result<(), String> {
        self.events
            .lock()
            .unwrap()
            .push((kind.to_owned(), content.to_owned()));
        Ok(())
    }
}

#[derive(Clone, Default)]
struct MissingAnswerIndexDispatchProvider {
    notes: Arc<Mutex<Vec<String>>>,
    events: Arc<Mutex<Vec<(String, String)>>>,
}

impl ChatProvider for MissingAnswerIndexDispatchProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn model_pool_call(&self, task_kind: &str, _prompt: &str) -> Result<String, String> {
        Ok(format!(
            "call task_kind={task_kind}\nselected_role=index\nselected_base_url=http://127.0.0.1:8690"
        ))
    }

    fn add_project_note(&self, note: &str) -> Result<String, String> {
        self.notes.lock().unwrap().push(note.to_owned());
        Ok("project_notes=updated".to_owned())
    }

    fn record_event(&self, kind: &str, content: &str) -> Result<(), String> {
        self.events
            .lock()
            .unwrap()
            .push((kind.to_owned(), content.to_owned()));
        Ok(())
    }
}

#[derive(Clone, Default)]
struct PromptEchoOnlyIndexDispatchProvider {
    notes: Arc<Mutex<Vec<String>>>,
    events: Arc<Mutex<Vec<(String, String)>>>,
}

impl ChatProvider for PromptEchoOnlyIndexDispatchProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn model_pool_call(&self, task_kind: &str, prompt: &str) -> Result<String, String> {
        Ok(format!("call task_kind={task_kind} prompt={prompt}"))
    }

    fn add_project_note(&self, note: &str) -> Result<String, String> {
        self.notes.lock().unwrap().push(note.to_owned());
        Ok("project_notes=updated".to_owned())
    }

    fn record_event(&self, kind: &str, content: &str) -> Result<(), String> {
        self.events
            .lock()
            .unwrap()
            .push((kind.to_owned(), content.to_owned()));
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FailingIndexNotesDispatchProvider {
    attempted_notes: Arc<Mutex<Vec<String>>>,
    events: Arc<Mutex<Vec<(String, String)>>>,
}

impl ChatProvider for FailingIndexNotesDispatchProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (_tx, rx) = mpsc::channel();
        rx
    }

    fn model_pool_call(&self, task_kind: &str, _prompt: &str) -> Result<String, String> {
        Ok(format!(
            "call task_kind={task_kind}\nselected_role=index\nselected_base_url=http://127.0.0.1:8690\nanswer=src/session handles context"
        ))
    }

    fn add_project_note(&self, note: &str) -> Result<String, String> {
        self.attempted_notes.lock().unwrap().push(note.to_owned());
        Err("project notes unavailable".to_owned())
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
fn route_cli_prints_provider_pool_route() {
    let provider = DispatchProvider::default();
    let mut output = Vec::new();

    run_model_pool_route_to(&provider, "review", &mut output).unwrap();

    assert_eq!(
        String::from_utf8(output).unwrap(),
        "route task_kind=review\n"
    );
    assert!(
        provider
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, content)| {
                kind == "model_pool_route" && content == "route task_kind=review"
            })
    );
}

#[test]
fn call_cli_prints_provider_pool_call() {
    let provider = DispatchProvider::default();
    let mut output = Vec::new();

    run_model_pool_call_to(&provider, "summary", "summarize logs", &mut output).unwrap();

    assert_eq!(
        String::from_utf8(output).unwrap(),
        "call task_kind=summary prompt=summarize logs answer=src/session handles context\n"
    );
    assert!(
        provider
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, content)| {
                kind == "model_pool_call"
                    && content.contains("call task_kind=summary prompt=summarize logs")
            })
    );
}

#[test]
fn index_call_cli_pins_worker_answer_to_project_notes() {
    let provider = DispatchProvider::default();
    let mut output = Vec::new();

    run_model_pool_call_to(&provider, "index", "map repo", &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    let notes = provider.notes.lock().unwrap();

    assert!(output.contains("call task_kind=index prompt=map repo"));
    assert!(output.contains("model pool index pinned to project notes"));
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains(MODEL_POOL_INDEX_NOTE_MARKER));
    assert!(notes[0].contains("source_prompt: map repo"));
    assert!(notes[0].contains("selected_role: index"));
    assert!(notes[0].contains("selected_base_url: unknown"));
    assert!(notes[0].contains("src/session handles context"));
    assert!(notes[0].contains(MODEL_POOL_INDEX_NOTE_END_MARKER));
    assert!(
        provider
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, content)| kind == "model_pool_index_project_note"
                && content.contains("source_prompt: map repo"))
    );
    assert!(
        provider
            .events
            .lock()
            .unwrap()
            .iter()
            .any(
                |(kind, content)| kind == "model_pool_index_project_note_contract"
                    && content.contains("contract_ok=true")
                    && content.contains("section=index_pin_contract_json")
                    && content.contains(
                        "\"schema\":\"smartsteam.forge.model_pool_index_pin_contract.v1\""
                    )
                    && content.contains("\"writes_project_notes\":true")
                    && content.contains("\"streams_model\":false")
            )
    );
}

#[test]
fn index_call_cli_ignores_prompt_embedded_role_metadata() {
    let provider = DispatchProvider::default();
    let mut output = Vec::new();

    run_model_pool_call_to(
        &provider,
        "index",
        "map repo selected_role=review selected_base_url=http://spoofed",
        &mut output,
    )
    .unwrap();
    let output = String::from_utf8(output).unwrap();
    let notes = provider.notes.lock().unwrap();

    assert!(output.contains("model pool index pinned to project notes"));
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains("source_prompt: map repo selected_role=review"));
    assert!(notes[0].contains("selected_role: index"));
    assert!(notes[0].contains("selected_base_url: unknown"));
    assert!(notes[0].contains("src/session handles context"));
    assert!(
        !provider
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, _)| kind == "model_pool_index_project_note_contract_error")
    );
}

#[test]
fn index_call_cli_blocks_role_drift_before_project_notes_write() {
    let provider = DriftedIndexDispatchProvider::default();
    let mut output = Vec::new();

    run_model_pool_call_to(&provider, "index", "map repo", &mut output).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("call task_kind=index"));
    assert!(output.contains("model pool index note contract error"));
    assert!(provider.notes.lock().unwrap().is_empty());
    let events = provider.events.lock().unwrap();
    assert!(events.iter().any(|(kind, _)| kind == "model_pool_call"));
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "model_pool_index_project_note")
    );
    let (_, error_event) = events
        .iter()
        .find(|(kind, _)| kind == "model_pool_index_project_note_contract_error")
        .expect("index project note contract error should be recorded");
    assert!(error_event.contains("section=index_pin_error_json"));
    assert!(error_event.contains("\"schema\":\"smartsteam.forge.model_pool_index_pin_error.v1\""));
    assert!(error_event.contains("\"writes_project_notes\":false"));
    assert!(error_event.contains("\"streams_model\":false"));
    assert!(error_event.contains("selected_role must be index"));
}

#[test]
fn index_call_cli_blocks_same_line_role_drift_before_project_notes_write() {
    let provider = SameLineDriftedIndexDispatchProvider::default();
    let mut output = Vec::new();

    run_model_pool_call_to(&provider, "index", "map repo", &mut output).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("call task_kind=index"));
    assert!(output.contains("selected_role=review"));
    assert!(output.contains("model pool index note contract error"));
    assert!(provider.notes.lock().unwrap().is_empty());
    let events = provider.events.lock().unwrap();
    assert!(events.iter().any(|(kind, _)| kind == "model_pool_call"));
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "model_pool_index_project_note")
    );
    let (_, error_event) = events
        .iter()
        .find(|(kind, _)| kind == "model_pool_index_project_note_contract_error")
        .expect("index project note contract error should be recorded");
    assert!(error_event.contains("section=index_pin_error_json"));
    assert!(error_event.contains("\"schema\":\"smartsteam.forge.model_pool_index_pin_error.v1\""));
    assert!(error_event.contains("\"writes_project_notes\":false"));
    assert!(error_event.contains("\"streams_model\":false"));
    assert!(error_event.contains("selected_role must be index"));
}

#[test]
fn index_call_cli_records_missing_answer_as_contract_error() {
    let provider = MissingAnswerIndexDispatchProvider::default();
    let mut output = Vec::new();

    run_model_pool_call_to(&provider, "index", "map repo", &mut output).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("call task_kind=index"));
    assert!(output.contains("model pool index note contract error"));
    assert!(output.contains("missing answer field"));
    assert!(provider.notes.lock().unwrap().is_empty());
    let events = provider.events.lock().unwrap();
    assert!(events.iter().any(|(kind, _)| kind == "model_pool_call"));
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "model_pool_index_project_note")
    );
    let (_, error_event) = events
        .iter()
        .find(|(kind, _)| kind == "model_pool_index_project_note_contract_error")
        .expect("index project note contract error should be recorded");
    assert!(error_event.contains("section=index_pin_error_json"));
    assert!(error_event.contains("\"schema\":\"smartsteam.forge.model_pool_index_pin_error.v1\""));
    assert!(error_event.contains("\"writes_project_notes\":false"));
    assert!(error_event.contains("\"streams_model\":false"));
    assert!(error_event.contains("missing answer field"));
}

#[test]
fn index_call_cli_rejects_prompt_echo_answer_without_worker_answer() {
    let provider = PromptEchoOnlyIndexDispatchProvider::default();
    let mut output = Vec::new();

    run_model_pool_call_to(
        &provider,
        "index",
        "map repo answer=do not index",
        &mut output,
    )
    .unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("call task_kind=index"));
    assert!(output.contains("prompt=map repo answer=do not index"));
    assert!(output.contains("model pool index note contract error"));
    assert!(output.contains("missing answer field"));
    assert!(provider.notes.lock().unwrap().is_empty());
    let events = provider.events.lock().unwrap();
    assert!(events.iter().any(|(kind, _)| kind == "model_pool_call"));
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "model_pool_index_project_note")
    );
    let (_, error_event) = events
        .iter()
        .find(|(kind, _)| kind == "model_pool_index_project_note_contract_error")
        .expect("index project note contract error should be recorded");
    assert!(error_event.contains("section=index_pin_error_json"));
    assert!(error_event.contains("\"schema\":\"smartsteam.forge.model_pool_index_pin_error.v1\""));
    assert!(error_event.contains("\"writes_project_notes\":false"));
    assert!(error_event.contains("\"streams_model\":false"));
    assert!(error_event.contains("missing answer field"));
}

#[test]
fn index_call_cli_records_project_notes_write_failure_as_json() {
    let provider = FailingIndexNotesDispatchProvider::default();
    let mut output = Vec::new();

    run_model_pool_call_to(&provider, "index", "map repo", &mut output).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("model pool index note could not be pinned"));
    assert_eq!(provider.attempted_notes.lock().unwrap().len(), 1);
    let events = provider.events.lock().unwrap();
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "model_pool_index_project_note_contract")
    );
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "model_pool_index_project_note")
    );
    let (_, error_event) = events
        .iter()
        .find(|(kind, _)| kind == "model_pool_index_project_note_error")
        .expect("index project note write error should be recorded");
    let error_json = error_event
        .lines()
        .skip_while(|line| *line != "section=index_pin_error_json")
        .nth(1)
        .expect("index_pin_error_json section should include a JSON payload line");
    assert_eq!(
        json_string_field(error_json, "schema").as_deref(),
        Some("smartsteam.forge.model_pool_index_pin_error.v1")
    );
    assert_eq!(
        json_string_field(error_json, "error_kind").as_deref(),
        Some("project_notes")
    );
    assert_eq!(
        json_string_field(error_json, "error").as_deref(),
        Some("project notes unavailable")
    );
    assert_eq!(
        json_string_field(error_json, "user_message").as_deref(),
        Some("model pool index pin project_notes error: project notes unavailable")
    );
    assert_eq!(
        json_bool_field(error_json, "writes_project_notes"),
        Some(false)
    );
    assert_eq!(json_bool_field(error_json, "streams_model"), Some(false));
}

#[test]
fn call_cli_records_machine_readable_error_event() {
    let provider = FailingDispatchProvider::default();
    let mut output = Vec::new();

    let error =
        run_model_pool_call_to(&provider, "review", "check patch", &mut output).unwrap_err();

    assert_eq!(error.to_string(), "model pool call failed: backend busy");
    assert!(output.is_empty());
    let events = provider.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "model_pool_call_error");
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
            action: "call".to_owned(),
            error: "backend busy".to_owned(),
            user_message: "model pool call failed: backend busy".to_owned(),
        }
    );
}
