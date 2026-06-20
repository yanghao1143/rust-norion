use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, ChatProvider};

const DUPLICATE_CHAR_DEBOUNCE: Duration = Duration::from_millis(220);

pub(super) fn handle_event<P: ChatProvider>(
    app: &mut App<P>,
    event: Event,
    key_filter: &mut KeyInputFilter,
    now: Instant,
) {
    match event {
        Event::Key(key) if key_filter.should_handle(&key, now) => handle_key(app, key),
        Event::Paste(text) if key_filter.should_handle_paste(&text, now) => insert_text(app, &text),
        _ => {}
    }
}

fn should_handle_key_event(key: &KeyEvent) -> bool {
    key.kind == KeyEventKind::Press
}

#[derive(Default)]
pub(super) struct KeyInputFilter {
    last_char: Option<DebouncedChar>,
}

struct DebouncedChar {
    ch: char,
    modifiers: KeyModifiers,
    at: Instant,
}

impl KeyInputFilter {
    fn should_handle(&mut self, key: &KeyEvent, now: Instant) -> bool {
        if !should_handle_key_event(key) {
            return false;
        }

        let KeyCode::Char(ch) = key.code else {
            self.last_char = None;
            return true;
        };

        if !is_plain_text_char(key.modifiers) {
            self.last_char = None;
            return true;
        }

        self.should_handle_plain_char(ch, normalized_text_modifiers(key.modifiers), now)
    }

    fn should_handle_paste(&mut self, text: &str, now: Instant) -> bool {
        let mut chars = text.chars();
        let Some(ch) = chars.next() else {
            return false;
        };
        if chars.next().is_some() {
            self.last_char = None;
            return true;
        }
        self.should_handle_plain_char(ch, KeyModifiers::NONE, now)
    }

    fn should_handle_plain_char(
        &mut self,
        ch: char,
        modifiers: KeyModifiers,
        now: Instant,
    ) -> bool {
        let duplicate = self.is_duplicate_plain_char(ch, modifiers, now);
        self.last_char = Some(DebouncedChar {
            ch,
            modifiers,
            at: now,
        });
        !duplicate
    }

    fn is_duplicate_plain_char(&self, ch: char, modifiers: KeyModifiers, now: Instant) -> bool {
        self.last_char.as_ref().is_some_and(|last| {
            last.ch == ch
                && last.modifiers == modifiers
                && now.saturating_duration_since(last.at) <= DUPLICATE_CHAR_DEBOUNCE
        })
    }
}

fn is_plain_text_char(modifiers: KeyModifiers) -> bool {
    matches!(modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT)
}

fn normalized_text_modifiers(modifiers: KeyModifiers) -> KeyModifiers {
    if is_plain_text_char(modifiers) {
        KeyModifiers::NONE
    } else {
        modifiers
    }
}

fn handle_key<P: ChatProvider>(app: &mut App<P>, key: KeyEvent) {
    match (key.code, key.modifiers) {
        (KeyCode::Char('x'), KeyModifiers::CONTROL) if app.provider_busy => {
            app.cancel_current_stream();
        }
        (KeyCode::PageUp, _) => app.scroll_up(8),
        (KeyCode::PageDown, _) => app.scroll_down(8),
        (KeyCode::Home, _) => app.scroll_top(),
        (KeyCode::End, _) => app.scroll_to_bottom(),
        (KeyCode::Left, _) => app.move_input_cursor_left(),
        (KeyCode::Right, _) => app.move_input_cursor_right(),
        (KeyCode::Delete, _) => app.delete_input_char(),
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => app.move_input_cursor_start(),
        (KeyCode::Char('e'), KeyModifiers::CONTROL) => app.move_input_cursor_end(),
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => app.clear_input_before_cursor(),
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => app.delete_word_before_cursor(),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.should_quit = true,
        (KeyCode::Esc, _) => app.should_quit = true,
        (KeyCode::Enter, modifiers) if should_insert_newline(modifiers) => {
            app.push_char('\n');
        }
        (KeyCode::Enter, _) => {
            app.submit();
        }
        (KeyCode::Backspace, _) => app.backspace(),
        (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => app.push_char(ch),
        _ => {}
    }
}

fn insert_text<P: ChatProvider>(app: &mut App<P>, text: &str) {
    app.push_text(text);
}

fn should_insert_newline(modifiers: KeyModifiers) -> bool {
    modifiers.contains(KeyModifiers::ALT) || modifiers.contains(KeyModifiers::SHIFT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MockProvider;

    #[test]
    fn ignores_non_press_key_events() {
        let repeat = KeyEvent::new_with_kind(
            KeyCode::Char('你'),
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        );
        let release = KeyEvent::new_with_kind(
            KeyCode::Char('你'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );

        assert!(!should_handle_key_event(&repeat));
        assert!(!should_handle_key_event(&release));
    }

    #[test]
    fn handles_press_key_events() {
        let press =
            KeyEvent::new_with_kind(KeyCode::Char('你'), KeyModifiers::NONE, KeyEventKind::Press);

        assert!(should_handle_key_event(&press));
    }

    #[test]
    fn key_filter_suppresses_immediate_duplicate_chars() {
        let mut filter = KeyInputFilter::default();
        let now = Instant::now();
        let key =
            KeyEvent::new_with_kind(KeyCode::Char('你'), KeyModifiers::NONE, KeyEventKind::Press);

        assert!(filter.should_handle(&key, now));
        assert!(!filter.should_handle(&key, now + Duration::from_millis(50)));
        assert!(!filter.should_handle(&key, now + Duration::from_millis(180)));
        assert!(filter.should_handle(&key, now + Duration::from_millis(401)));
    }

    #[test]
    fn key_filter_debounces_only_plain_text_chars() {
        let mut filter = KeyInputFilter::default();
        let now = Instant::now();
        let cancel = KeyEvent::new_with_kind(
            KeyCode::Char('x'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );

        assert!(filter.should_handle(&cancel, now));
        assert!(filter.should_handle(&cancel, now + Duration::from_millis(50)));
    }

    #[test]
    fn key_filter_allows_different_chars_without_debounce() {
        let mut filter = KeyInputFilter::default();
        let now = Instant::now();
        let first =
            KeyEvent::new_with_kind(KeyCode::Char('你'), KeyModifiers::NONE, KeyEventKind::Press);
        let second =
            KeyEvent::new_with_kind(KeyCode::Char('好'), KeyModifiers::NONE, KeyEventKind::Press);

        assert!(filter.should_handle(&first, now));
        assert!(filter.should_handle(&second, now + Duration::from_millis(5)));
    }

    #[test]
    fn alt_enter_inserts_newline_without_submitting() {
        let mut app = App::new(MockProvider::default());
        app.input = "hello".to_owned();

        handle_key(
            &mut app,
            KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::ALT, KeyEventKind::Press),
        );

        assert_eq!(app.input, "hello\n");
        assert!(!app.provider_busy);
    }

    #[test]
    fn shift_enter_inserts_newline_without_submitting() {
        let mut app = App::new(MockProvider::default());
        app.input = "hello".to_owned();

        handle_key(
            &mut app,
            KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::SHIFT, KeyEventKind::Press),
        );

        assert_eq!(app.input, "hello\n");
        assert!(!app.provider_busy);
    }

    #[test]
    fn paste_event_inserts_text_without_debounce() {
        let mut app = App::new(MockProvider::default());
        let mut filter = KeyInputFilter::default();
        let now = Instant::now();

        handle_event(
            &mut app,
            Event::Paste("hello\nhello".to_owned()),
            &mut filter,
            now,
        );

        assert_eq!(app.input, "hello\nhello");
    }

    #[test]
    fn single_char_paste_and_key_cross_debounce() {
        let mut app = App::new(MockProvider::default());
        let mut filter = KeyInputFilter::default();
        let now = Instant::now();

        handle_event(&mut app, Event::Paste("你".to_owned()), &mut filter, now);
        handle_event(
            &mut app,
            Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('你'),
                KeyModifiers::NONE,
                KeyEventKind::Press,
            )),
            &mut filter,
            now + Duration::from_millis(20),
        );
        handle_event(
            &mut app,
            Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('你'),
                KeyModifiers::SHIFT,
                KeyEventKind::Press,
            )),
            &mut filter,
            now + Duration::from_millis(40),
        );

        assert_eq!(app.input, "你");
    }

    #[test]
    fn key_and_single_char_paste_cross_debounce() {
        let mut app = App::new(MockProvider::default());
        let mut filter = KeyInputFilter::default();
        let now = Instant::now();

        handle_event(
            &mut app,
            Event::Key(KeyEvent::new_with_kind(
                KeyCode::Char('好'),
                KeyModifiers::NONE,
                KeyEventKind::Press,
            )),
            &mut filter,
            now,
        );
        handle_event(
            &mut app,
            Event::Paste("好".to_owned()),
            &mut filter,
            now + Duration::from_millis(20),
        );

        assert_eq!(app.input, "好");
    }

    #[test]
    fn arrow_delete_and_control_keys_edit_input_cursor() {
        let mut app = App::new(MockProvider::default());
        app.push_text("你好 world");

        handle_key(
            &mut app,
            KeyEvent::new_with_kind(KeyCode::Left, KeyModifiers::NONE, KeyEventKind::Press),
        );
        handle_key(
            &mut app,
            KeyEvent::new_with_kind(KeyCode::Left, KeyModifiers::NONE, KeyEventKind::Press),
        );
        handle_key(
            &mut app,
            KeyEvent::new_with_kind(KeyCode::Delete, KeyModifiers::NONE, KeyEventKind::Press),
        );

        assert_eq!(app.input, "你好 word");

        handle_key(
            &mut app,
            KeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL,
                KeyEventKind::Press,
            ),
        );
        handle_key(
            &mut app,
            KeyEvent::new_with_kind(KeyCode::Right, KeyModifiers::NONE, KeyEventKind::Press),
        );
        handle_key(
            &mut app,
            KeyEvent::new_with_kind(
                KeyCode::Char('u'),
                KeyModifiers::CONTROL,
                KeyEventKind::Press,
            ),
        );

        assert_eq!(app.input, "好 word");

        handle_key(
            &mut app,
            KeyEvent::new_with_kind(
                KeyCode::Char('e'),
                KeyModifiers::CONTROL,
                KeyEventKind::Press,
            ),
        );
        handle_key(
            &mut app,
            KeyEvent::new_with_kind(
                KeyCode::Char('w'),
                KeyModifiers::CONTROL,
                KeyEventKind::Press,
            ),
        );

        assert_eq!(app.input, "好 ");
    }

    #[test]
    fn ctrl_x_cancels_busy_provider() {
        let mut app = App::new(MockProvider::default());
        app.provider_busy = true;

        handle_key(
            &mut app,
            KeyEvent::new_with_kind(
                KeyCode::Char('x'),
                KeyModifiers::CONTROL,
                KeyEventKind::Press,
            ),
        );

        assert!(!app.provider_busy);
        assert_eq!(app.status, "cancel requested");
    }

    #[test]
    fn plain_enter_submits_multiline_prompt() {
        let mut app = App::new(MockProvider::default());
        app.input = "hello\nforge".to_owned();

        handle_key(
            &mut app,
            KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Press),
        );

        assert!(app.provider_busy);
        assert!(app.input.is_empty());
        assert!(
            app.messages
                .iter()
                .any(|message| message.content == "hello\nforge")
        );
    }

    #[test]
    fn page_up_and_end_control_transcript_follow_mode() {
        let mut app = App::new(MockProvider::default());
        app.input = "one\ntwo\nthree\nfour".to_owned();
        app.submit();
        app.auto_scroll();
        assert!(app.auto_follow);
        let bottom = app.scroll;

        handle_key(
            &mut app,
            KeyEvent::new_with_kind(KeyCode::PageUp, KeyModifiers::NONE, KeyEventKind::Press),
        );

        assert!(!app.auto_follow);
        assert!(app.scroll < bottom);

        handle_key(
            &mut app,
            KeyEvent::new_with_kind(KeyCode::End, KeyModifiers::NONE, KeyEventKind::Press),
        );

        assert!(app.auto_follow);
        assert_eq!(app.scroll, bottom);
    }
}
