use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use crate::app::{App, ChatProvider, Message, MessageRole};
use crate::text_width::terminal_width;

const MIN_INPUT_HEIGHT: u16 = 3;
const MAX_INPUT_HEIGHT: u16 = 8;

pub(super) fn draw<P: ChatProvider>(frame: &mut ratatui::Frame<'_>, app: &mut App<P>) {
    let input_height = input_box_height(&app.input);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(input_height),
        ])
        .split(frame.area());
    app.set_transcript_viewport(chunks[1].height, chunks[1].width);

    let busy = if app.provider_busy {
        "streaming"
    } else {
        "idle"
    };
    let scroll = if app.auto_follow { "follow" } else { "manual" };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "SmartSteam Forge",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!(
                "status: {} | provider: {busy} | scroll: {scroll}",
                app.status
            ),
            Style::default().fg(Color::Gray),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    let messages = transcript_lines(&app.messages);

    let transcript = Paragraph::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Conversation"))
        .scroll((app.scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(transcript, chunks[1]);
    let mut scrollbar_state = ScrollbarState::new(app.transcript_content_lines())
        .position(app.scroll as usize)
        .viewport_content_length(chunks[1].height.saturating_sub(2).max(1) as usize);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .thumb_style(Style::default().fg(Color::Cyan))
        .track_style(Style::default().fg(Color::DarkGray));
    frame.render_stateful_widget(
        scrollbar,
        chunks[1].inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );

    let visible_input_lines = input_height.saturating_sub(2).max(1);
    let visible_input_columns = chunks[2].width.saturating_sub(2).max(1);
    let cursor = input_cursor(
        &app.input,
        app.input_cursor_byte_index(),
        visible_input_lines,
        visible_input_columns,
    );
    let input_scroll = input_scroll(
        &app.input,
        app.input_cursor_byte_index(),
        visible_input_lines,
    );
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .style(Style::default().fg(Color::White))
        .scroll((input_scroll, cursor.horizontal_scroll));
    frame.render_widget(input, chunks[2]);

    let cursor_x = chunks[2].x.saturating_add(1).saturating_add(cursor.column);
    let cursor_y = chunks[2].y.saturating_add(1).saturating_add(cursor.row);
    frame.set_cursor_position((cursor_x, cursor_y));
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct InputCursor {
    row: u16,
    column: u16,
    horizontal_scroll: u16,
}

fn input_box_height(input: &str) -> u16 {
    input_line_count(input)
        .saturating_add(2)
        .clamp(MIN_INPUT_HEIGHT, MAX_INPUT_HEIGHT)
}

fn input_scroll(input: &str, cursor_byte_index: usize, visible_lines: u16) -> u16 {
    let max_scroll = input_line_count(input).saturating_sub(visible_lines);
    input_cursor_line_index(input, cursor_byte_index)
        .saturating_sub(visible_lines.saturating_sub(1))
        .min(max_scroll)
}

fn input_cursor(
    input: &str,
    cursor_byte_index: usize,
    visible_lines: u16,
    visible_columns: u16,
) -> InputCursor {
    let cursor_byte_index = clamp_to_char_boundary(input, cursor_byte_index);
    let line_index = input_cursor_line_index(input, cursor_byte_index);
    let scroll = input_scroll(input, cursor_byte_index, visible_lines);
    let row = line_index
        .saturating_sub(scroll)
        .min(visible_lines.saturating_sub(1));
    let column_width = input_cursor_width(input, cursor_byte_index);
    let horizontal_scroll = input_horizontal_scroll(column_width, visible_columns);
    InputCursor {
        row,
        column: column_width.saturating_sub(horizontal_scroll),
        horizontal_scroll,
    }
}

fn input_line_count(input: &str) -> u16 {
    input.split('\n').count().max(1).min(u16::MAX as usize) as u16
}

fn input_cursor_width(input: &str, cursor_byte_index: usize) -> u16 {
    let cursor_byte_index = clamp_to_char_boundary(input, cursor_byte_index);
    let line_start = input[..cursor_byte_index]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    terminal_width(&input[line_start..cursor_byte_index]).min(u16::MAX as usize) as u16
}

fn input_cursor_line_index(input: &str, cursor_byte_index: usize) -> u16 {
    let cursor_byte_index = clamp_to_char_boundary(input, cursor_byte_index);
    input[..cursor_byte_index]
        .chars()
        .filter(|ch| *ch == '\n')
        .count()
        .min(u16::MAX as usize) as u16
}

fn input_horizontal_scroll(cursor_width: u16, visible_columns: u16) -> u16 {
    cursor_width.saturating_sub(visible_columns.saturating_sub(1).max(1))
}

fn clamp_to_char_boundary(input: &str, cursor_byte_index: usize) -> usize {
    let mut index = cursor_byte_index.min(input.len());
    while index > 0 && !input.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn transcript_lines(messages: &[Message]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for message in messages {
        lines.push(role_label_line(message.role));
        for line in message.content.split('\n') {
            lines.push(Line::from(line.to_owned()));
        }
    }
    lines
}

fn role_label_line(role: MessageRole) -> Line<'static> {
    let (label, color) = match role {
        MessageRole::System => ("system", Color::DarkGray),
        MessageRole::User => ("you", Color::Yellow),
        MessageRole::Assistant => ("forge", Color::Green),
    };
    Line::from(Span::styled(
        format!("{label}:"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MockProvider;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    #[test]
    fn cursor_width_counts_terminal_columns_for_wide_chars() {
        assert_eq!(input_cursor_width("你a", "你a".len()), 3);
        assert_eq!(input_cursor_width("你a", "你".len()), 2);
        assert_eq!("你a".len(), 4);
    }

    #[test]
    fn cursor_width_uses_last_input_line() {
        assert_eq!(input_cursor_width("first\n你a", "first\n你a".len()), 3);
    }

    #[test]
    fn transcript_lines_preserve_message_newlines() {
        let messages = vec![Message::new(MessageRole::Assistant, "one\ntwo\n")];
        let rendered = transcript_lines(&messages)
            .iter()
            .map(line_text)
            .collect::<Vec<_>>();

        assert_eq!(rendered, ["forge:", "one", "two", ""]);
    }

    #[test]
    fn input_box_grows_until_max_height() {
        assert_eq!(input_box_height(""), 3);
        assert_eq!(input_box_height("a\nb\nc"), 5);
        assert_eq!(input_box_height("1\n2\n3\n4\n5\n6\n7\n8\n9"), 8);
    }

    #[test]
    fn input_cursor_tracks_visible_multiline_tail() {
        assert_eq!(
            input_cursor("a\nb\ncd", "a\nb\ncd".len(), 3, 80),
            InputCursor {
                row: 2,
                column: 2,
                horizontal_scroll: 0
            }
        );
        assert_eq!(
            input_cursor("1\n2\n3\n4\nxy", "1\n2\n3\n4\nxy".len(), 3, 80),
            InputCursor {
                row: 2,
                column: 2,
                horizontal_scroll: 0
            }
        );
        assert_eq!(input_scroll("1\n2\n3\n4\nxy", "1\n2\n3\n4\nxy".len(), 3), 2);
    }

    #[test]
    fn input_cursor_tracks_middle_line_and_column() {
        let input = "one\n你a\nthree";
        let cursor = "one\n你".len();

        assert_eq!(
            input_cursor(input, cursor, 3, 80),
            InputCursor {
                row: 1,
                column: 2,
                horizontal_scroll: 0
            }
        );
        assert_eq!(input_scroll(input, cursor, 3), 0);
    }

    #[test]
    fn input_cursor_horizontal_scroll_keeps_long_line_visible() {
        assert_eq!(
            input_cursor("abcdef", "abcdef".len(), 1, 4),
            InputCursor {
                row: 0,
                column: 3,
                horizontal_scroll: 3
            }
        );
    }

    #[test]
    fn draw_keeps_latest_multiline_output_visible_when_following() {
        let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
        let mut app = App::new(MockProvider::default());
        app.messages.push(Message::new(
            MessageRole::Assistant,
            "one\ntwo\nthree\nfour\nvisible-tail",
        ));

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        let screen = buffer_text(terminal.backend().buffer());
        assert!(
            screen.contains("visible-tail"),
            "latest streamed line should stay visible at bottom, got:\n{screen}"
        );
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    fn buffer_text(buffer: &Buffer) -> String {
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }
}
