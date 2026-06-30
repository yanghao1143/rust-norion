use std::sync::mpsc::{self, Receiver};

use super::super::provider::{ChatProvider, ProviderEvent};
use super::{App, MessageRole};

#[derive(Clone, Copy)]
struct TestProvider;

impl ChatProvider for TestProvider {
    fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
        let (tx, rx) = mpsc::channel();
        tx.send(ProviderEvent::Delta("ok".to_owned())).unwrap();
        tx.send(ProviderEvent::Done).unwrap();
        rx
    }

    fn status(&self) -> String {
        "test status".to_owned()
    }
}

#[test]
fn app_state_ticks_provider_stream() {
    let mut app = App::new(TestProvider);
    app.push_text("hello");

    app.submit();
    app.tick();

    assert!(!app.provider_busy);
    assert!(
        app.messages
            .iter()
            .any(|message| { message.role == MessageRole::Assistant && message.content == "ok" })
    );
}
