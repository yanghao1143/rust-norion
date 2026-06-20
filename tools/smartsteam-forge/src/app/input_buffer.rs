#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct InputCursor {
    byte_index: Option<usize>,
}

impl InputCursor {
    pub(super) fn byte_index(&self, input: &str) -> usize {
        self.byte_index
            .map(|index| previous_boundary(input, index.min(input.len())))
            .unwrap_or(input.len())
    }

    pub(super) fn insert_char(&mut self, input: &mut String, ch: char) {
        let index = self.byte_index(input);
        input.insert(index, ch);
        if self.byte_index.is_some() {
            self.byte_index = Some(index.saturating_add(ch.len_utf8()));
        }
    }

    pub(super) fn insert_text(&mut self, input: &mut String, text: &str) {
        for ch in text.chars() {
            self.insert_char(input, ch);
        }
    }

    pub(super) fn backspace(&mut self, input: &mut String) {
        let index = self.byte_index(input);
        if index == 0 {
            return;
        }
        let previous = previous_char_start(input, index);
        input.replace_range(previous..index, "");
        if self.byte_index.is_some() {
            self.byte_index = Some(previous);
        }
    }

    pub(super) fn delete(&mut self, input: &mut String) {
        let index = self.byte_index(input);
        if index >= input.len() {
            return;
        }
        let next = next_char_end(input, index);
        input.replace_range(index..next, "");
        if self.byte_index.is_some() {
            self.byte_index = Some(index);
        }
    }

    pub(super) fn move_left(&mut self, input: &str) {
        let index = self.byte_index(input);
        if index == 0 {
            self.byte_index = Some(0);
            return;
        }
        self.byte_index = Some(previous_char_start(input, index));
    }

    pub(super) fn move_right(&mut self, input: &str) {
        let index = self.byte_index(input);
        if index >= input.len() {
            self.byte_index = None;
            return;
        }
        let next = next_char_end(input, index);
        self.byte_index = if next >= input.len() {
            None
        } else {
            Some(next)
        };
    }

    pub(super) fn move_start(&mut self) {
        self.byte_index = Some(0);
    }

    pub(super) fn move_end(&mut self) {
        self.byte_index = None;
    }

    pub(super) fn clear_before(&mut self, input: &mut String) {
        let index = self.byte_index(input);
        if index == 0 {
            return;
        }
        input.replace_range(..index, "");
        if self.byte_index.is_some() {
            self.byte_index = Some(0);
        }
    }

    pub(super) fn delete_word_before(&mut self, input: &mut String) {
        let index = self.byte_index(input);
        if index == 0 {
            return;
        }

        let before_cursor = &input[..index];
        let word_end = trim_trailing_whitespace_index(before_cursor);
        let word_start = previous_word_start(before_cursor, word_end);
        input.replace_range(word_start..index, "");
        if self.byte_index.is_some() {
            self.byte_index = Some(word_start);
        }
    }

    pub(super) fn reset_to_end(&mut self) {
        self.byte_index = None;
    }
}

fn previous_boundary(input: &str, mut index: usize) -> usize {
    while index > 0 && !input.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn previous_char_start(input: &str, index: usize) -> usize {
    input[..index]
        .char_indices()
        .next_back()
        .map(|(start, _)| start)
        .unwrap_or(0)
}

fn next_char_end(input: &str, index: usize) -> usize {
    input[index..]
        .chars()
        .next()
        .map(|ch| index + ch.len_utf8())
        .unwrap_or(index)
}

fn trim_trailing_whitespace_index(text: &str) -> usize {
    text.char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0)
}

fn previous_word_start(text: &str, word_end: usize) -> usize {
    text[..word_end]
        .char_indices()
        .rev()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0)
}
