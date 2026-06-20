use super::event::StreamEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SseFrame {
    pub event: String,
    pub data: String,
}

pub(crate) fn parse_frame(frame: &[u8]) -> Result<Option<SseFrame>, String> {
    let frame =
        std::str::from_utf8(frame).map_err(|error| format!("SSE frame was not UTF-8: {error}"))?;
    let mut event = "message";
    let mut data = Vec::new();
    let normalized = frame.replace("\r\n", "\n").replace('\r', "\n");

    for line in normalized.split('\n') {
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        if let Some(value) = line.strip_prefix("event:") {
            event = sse_field_value(value);
        } else if line == "event" {
            event = "";
        } else if let Some(value) = line.strip_prefix("data:") {
            data.push(sse_field_value(value));
        } else if line == "data" {
            data.push("");
        }
    }

    if event.is_empty() {
        event = "message";
    }
    if data.is_empty() && event == "message" {
        return Ok(None);
    }
    Ok(Some(SseFrame {
        event: event.to_owned(),
        data: data.join("\n"),
    }))
}

fn sse_field_value(value: &str) -> &str {
    value.strip_prefix(' ').unwrap_or(value)
}

pub(crate) fn drain_events(buffer: &mut Vec<u8>) -> Result<Vec<StreamEvent>, String> {
    let mut events = Vec::new();
    while let Some((frame_end, boundary_len)) = frame_boundary(buffer) {
        let frame = buffer[..frame_end].to_vec();
        buffer.drain(..frame_end + boundary_len);
        if let Some(frame) = parse_frame(&frame)? {
            events.push(StreamEvent::from_sse(&frame.event, frame.data));
        }
    }
    Ok(events)
}

pub(crate) fn frame_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
    [
        bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| (index, 4)),
        bytes
            .windows(2)
            .position(|window| window == b"\n\n")
            .map(|index| (index, 2)),
        bytes
            .windows(2)
            .position(|window| window == b"\r\r")
            .map(|index| (index, 2)),
    ]
    .into_iter()
    .flatten()
    .min_by_key(|(index, _)| *index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_delta_event() {
        let frame = parse_frame(b"event: delta\ndata: hello\n\n")
            .unwrap()
            .unwrap();

        assert_eq!(frame.event, "delta");
        assert_eq!(frame.data, "hello");
    }

    #[test]
    fn joins_multiline_data() {
        let frame = parse_frame(b"event: meta\r\ndata: one\r\ndata: two\r\n\r\n")
            .unwrap()
            .unwrap();

        assert_eq!(frame.event, "meta");
        assert_eq!(frame.data, "one\ntwo");
    }

    #[test]
    fn joins_cr_only_multiline_data() {
        let frame = parse_frame(b"event: meta\rdata: one\rdata: two")
            .unwrap()
            .unwrap();

        assert_eq!(frame.event, "meta");
        assert_eq!(frame.data, "one\ntwo");
    }

    #[test]
    fn preserves_data_indentation_after_optional_space() {
        let frame = parse_frame(b"data:   indented\n\n").unwrap().unwrap();

        assert_eq!(frame.event, "message");
        assert_eq!(frame.data, "  indented");
    }

    #[test]
    fn treats_sse_fields_without_colon_as_empty_values() {
        let frame = parse_frame(b"event\ndata\n\n").unwrap().unwrap();

        assert_eq!(frame.event, "message");
        assert_eq!(frame.data, "");
    }

    #[test]
    fn empty_event_field_falls_back_to_message() {
        let frame = parse_frame(b"event:\ndata: hello\n\n").unwrap().unwrap();

        assert_eq!(frame.event, "message");
        assert_eq!(frame.data, "hello");
    }

    #[test]
    fn ignores_comment_only_frame() {
        assert_eq!(parse_frame(b": keep-alive\n\n").unwrap(), None);
    }

    #[test]
    fn terminal_event_can_omit_data() {
        let mut buffer = b"event: done\n\n".to_vec();
        let events = drain_events(&mut buffer).unwrap();

        assert_eq!(events, vec![StreamEvent::Done]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn frame_boundary_reports_separator_length() {
        assert_eq!(
            frame_boundary(b"event: done\r\ndata: [DONE]\r\n\r\nnext"),
            Some((25, 4))
        );
        assert_eq!(
            frame_boundary(b"event: done\ndata: [DONE]\n\nnext"),
            Some((24, 2))
        );
        assert_eq!(
            frame_boundary(b"event: done\rdata: [DONE]\r\rnext"),
            Some((24, 2))
        );
    }

    #[test]
    fn frame_boundary_prefers_earliest_separator() {
        assert_eq!(frame_boundary(b"a\n\nb\r\n\r\nc"), Some((1, 2)));
        assert_eq!(frame_boundary(b"a\r\n\r\nb\n\nc"), Some((1, 4)));
        assert_eq!(frame_boundary(b"a\r\rb\n\nc"), Some((1, 2)));
    }

    #[test]
    fn drains_complete_events_and_keeps_partial_tail() {
        let mut buffer = b"event: stage\ndata: boot\n\nevent: delta\ndata: he".to_vec();
        let events = drain_events(&mut buffer).unwrap();

        assert_eq!(events, vec![StreamEvent::Stage("boot".to_owned())]);
        assert_eq!(buffer, b"event: delta\ndata: he");
    }

    #[test]
    fn drains_crlf_events_and_keeps_partial_tail() {
        let mut buffer = b"event: delta\r\ndata: hello\r\n\r\nevent: delta\r\ndata: wor".to_vec();
        let events = drain_events(&mut buffer).unwrap();

        assert_eq!(events, vec![StreamEvent::Delta("hello".to_owned())]);
        assert_eq!(buffer, b"event: delta\r\ndata: wor");
    }
}
