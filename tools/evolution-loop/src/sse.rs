pub(crate) fn relay_sse_frame(
    frame: &[u8],
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), String>,
) -> Result<(), String> {
    let frame = std::str::from_utf8(frame)
        .map_err(|error| format!("backend stream frame was not UTF-8: {error}"))?;
    let mut event = "message";
    let mut data = Vec::new();
    for line in frame.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(value) = line.strip_prefix("event:") {
            event = value.trim();
        } else if let Some(value) = line.strip_prefix("data:") {
            data.push(value.trim_start());
        }
    }
    on_event(event, &data.join("\n"))
}

pub(crate) fn frame_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
    let lf = bytes.windows(2).position(|window| window == b"\n\n");
    let crlf = bytes.windows(4).position(|window| window == b"\r\n\r\n");
    match (lf, crlf) {
        (Some(lf), Some(crlf)) if crlf < lf => Some((crlf, 4)),
        (Some(lf), _) => Some((lf, 2)),
        (None, Some(crlf)) => Some((crlf, 4)),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relays_multiline_sse_data() {
        let mut events = Vec::new();
        relay_sse_frame(
            b"event: final\r\ndata: one\r\ndata: two\r\n",
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(events, vec![("final".to_owned(), "one\ntwo".to_owned())]);
    }

    #[test]
    fn finds_lf_and_crlf_boundaries() {
        assert_eq!(frame_boundary(b"a\n\nb"), Some((1, 2)));
        assert_eq!(frame_boundary(b"a\r\n\r\nb"), Some((1, 4)));
    }
}
