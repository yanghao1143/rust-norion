use std::io::{ErrorKind, Read};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use super::io::{BACKEND_STREAM_READ_POLL_INTERVAL, backend_io_error_message};
use crate::status::wait_status_message;

pub(super) fn stream_backend_events(
    stream: &mut TcpStream,
    response_timeout: Duration,
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), String>,
) -> Result<(), String> {
    let started = Instant::now();
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let mut saw_any_event = false;
    let (header_end, header_boundary_len) = loop {
        if stream_timeout_expired(started, response_timeout) {
            return Err(stream_timeout_message("headers", response_timeout));
        }
        set_next_read_timeout(stream, started, response_timeout).map_err(|error| {
            backend_io_error_message(
                "configure backend stream header read timeout",
                error,
                response_timeout,
            )
        })?;
        match stream.read(&mut chunk) {
            Ok(0) => return Err("backend stream closed before headers".to_owned()),
            Ok(read) => {
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(header_boundary) = http_header_boundary(&buffer) {
                    break header_boundary;
                }
            }
            Err(error) if is_timeout_error(&error) => {
                emit_waiting_heartbeat(started, on_event)?;
            }
            Err(error) => {
                return Err(backend_io_error_message(
                    "read backend stream headers",
                    error,
                    response_timeout,
                ));
            }
        }
    };

    let headers = String::from_utf8_lossy(&buffer[..header_end]);
    let content_length = http_content_length(&headers);
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .unwrap_or(0);
    let mut body = buffer
        .get(header_end + header_boundary_len..)
        .unwrap_or_default()
        .to_vec();
    if !(200..300).contains(&status) {
        read_http_error_body(stream, &mut body, content_length, started, response_timeout)
            .map_err(|error| {
                backend_io_error_message("read backend stream error body", error, response_timeout)
            })?;
        return Err(format!(
            "backend stream returned HTTP {status}: {}",
            String::from_utf8_lossy(&body).trim()
        ));
    }

    loop {
        while let Some((frame_end, boundary_len)) = sse_frame_boundary(&body) {
            let frame = body[..frame_end].to_vec();
            body.drain(..frame_end + boundary_len);
            match relay_sse_frame_result(&frame, on_event)? {
                RelayedSseFrame::Ignored => {}
                RelayedSseFrame::NonTerminal => saw_any_event = true,
                RelayedSseFrame::Terminal => {
                    return Ok(());
                }
            }
        }
        if stream_timeout_expired(started, response_timeout) {
            return Err(stream_timeout_message("body", response_timeout));
        }
        set_next_read_timeout(stream, started, response_timeout).map_err(|error| {
            backend_io_error_message(
                "configure backend stream body read timeout",
                error,
                response_timeout,
            )
        })?;
        match stream.read(&mut chunk) {
            Ok(0) => {
                if !body.is_empty() {
                    return Err(
                        "backend stream truncated: incomplete SSE frame before EOF".to_owned()
                    );
                }
                let reason = if saw_any_event {
                    "backend stream truncated: EOF after SSE events but before done or error terminal event"
                } else {
                    "backend stream truncated: EOF before done or error terminal event"
                };
                return Err(reason.to_owned());
            }
            Ok(read) => body.extend_from_slice(&chunk[..read]),
            Err(error) if is_timeout_error(&error) => {
                emit_waiting_heartbeat(started, on_event)?;
            }
            Err(error) => {
                return Err(backend_io_error_message(
                    "read backend stream body",
                    error,
                    response_timeout,
                ));
            }
        }
    }
}

fn emit_waiting_heartbeat(
    started: Instant,
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), String>,
) -> Result<(), String> {
    on_event(
        "heartbeat",
        &wait_status_message(started.elapsed().as_secs()),
    )
}

fn stream_timeout_expired(started: Instant, response_timeout: Duration) -> bool {
    started.elapsed() >= response_timeout
}

fn set_next_read_timeout(
    stream: &TcpStream,
    started: Instant,
    response_timeout: Duration,
) -> Result<(), std::io::Error> {
    let remaining = response_timeout
        .saturating_sub(started.elapsed())
        .max(Duration::from_millis(1));
    let poll_interval = match stream.read_timeout()? {
        Some(current) => current.min(BACKEND_STREAM_READ_POLL_INTERVAL),
        None => BACKEND_STREAM_READ_POLL_INTERVAL,
    };
    let timeout = poll_interval.min(remaining);
    stream.set_read_timeout(Some(timeout))
}

fn stream_timeout_message(stage: &str, response_timeout: Duration) -> String {
    format!(
        "backend stream timed out after {} while waiting for {stage}",
        format_duration(response_timeout)
    )
}

fn format_duration(duration: Duration) -> String {
    if duration.subsec_millis() == 0 && duration.as_secs() > 0 {
        format!("{}s", duration.as_secs())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

fn is_timeout_error(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)
}

fn http_header_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
    double_newline_boundary(bytes)
}

fn double_newline_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
    let lf = bytes.windows(2).position(|window| window == b"\n\n");
    let crlf = bytes.windows(4).position(|window| window == b"\r\n\r\n");
    match (lf, crlf) {
        (Some(lf), Some(crlf)) if crlf < lf => Some((crlf, 4)),
        (Some(lf), _) => Some((lf, 2)),
        (None, Some(crlf)) => Some((crlf, 4)),
        (None, None) => None,
    }
}

fn http_content_length(headers: &str) -> Option<usize> {
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    })
}

fn read_http_error_body(
    stream: &mut TcpStream,
    body: &mut Vec<u8>,
    content_length: Option<usize>,
    started: Instant,
    response_timeout: Duration,
) -> Result<(), std::io::Error> {
    let Some(content_length) = content_length else {
        return Ok(());
    };
    if body.len() > content_length {
        body.truncate(content_length);
        return Ok(());
    }
    let mut chunk = [0_u8; 4096];
    while body.len() < content_length {
        if stream_timeout_expired(started, response_timeout) {
            return Ok(());
        }
        set_next_read_timeout(stream, started, response_timeout)?;
        let remaining = content_length - body.len();
        match stream.read(&mut chunk[..remaining.min(4096)]) {
            Ok(0) => return Ok(()),
            Ok(read) => body.extend_from_slice(&chunk[..read]),
            Err(error) if is_timeout_error(&error) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn sse_frame_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
    let lf = bytes.windows(2).position(|window| window == b"\n\n");
    let cr = bytes.windows(2).position(|window| window == b"\r\r");
    let crlf = bytes.windows(4).position(|window| window == b"\r\n\r\n");
    [(lf, 2), (cr, 2), (crlf, 4)]
        .into_iter()
        .filter_map(|(index, len)| index.map(|index| (index, len)))
        .min_by_key(|(index, _)| *index)
}

pub(super) fn relay_sse_frame(
    frame: &[u8],
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), String>,
) -> Result<bool, String> {
    relay_sse_frame_result(frame, on_event).map(|result| result == RelayedSseFrame::Terminal)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelayedSseFrame {
    Ignored,
    NonTerminal,
    Terminal,
}

fn relay_sse_frame_result(
    frame: &[u8],
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), String>,
) -> Result<RelayedSseFrame, String> {
    let frame = std::str::from_utf8(frame)
        .map_err(|error| format!("backend stream frame was not UTF-8: {error}"))?;
    let mut event = "message";
    let mut data = Vec::new();
    let mut saw_sse_field = false;
    let normalized_frame = frame.replace("\r\n", "\n").replace('\r', "\n");
    for line in normalized_frame.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            saw_sse_field = true;
            event = sse_field_value(value);
        } else if line == "event" {
            saw_sse_field = true;
            event = "";
        } else if let Some(value) = line.strip_prefix("data:") {
            saw_sse_field = true;
            data.push(sse_field_value(value));
        } else if line == "data" {
            saw_sse_field = true;
            data.push("");
        }
    }
    if !saw_sse_field {
        return Ok(RelayedSseFrame::Ignored);
    }
    if event.is_empty() {
        event = "message";
    }
    let is_terminal = matches!(event, "done" | "error");
    on_event(event, &data.join("\n"))?;
    Ok(if is_terminal {
        RelayedSseFrame::Terminal
    } else {
        RelayedSseFrame::NonTerminal
    })
}

fn sse_field_value(value: &str) -> &str {
    value.strip_prefix(' ').unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;

    fn stream_response(body: &'static [u8]) -> Vec<(String, String)> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n")
                .unwrap();
            stream.write_all(body).unwrap();
        });

        let mut stream = TcpStream::connect(address).unwrap();
        let mut events = Vec::new();
        stream_backend_events(&mut stream, Duration::from_secs(30), &mut |event, data| {
            events.push((event.to_owned(), data.to_owned()));
            Ok(())
        })
        .unwrap();
        server.join().unwrap();
        events
    }

    fn stream_response_error(body: &'static [u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n")
                .unwrap();
            stream.write_all(body).unwrap();
        });

        let mut stream = TcpStream::connect(address).unwrap();
        let error = stream_backend_events(&mut stream, Duration::from_secs(30), &mut |_, _| Ok(()))
            .unwrap_err();
        server.join().unwrap();
        error
    }

    #[test]
    fn http_header_boundary_reports_separator_length() {
        assert_eq!(
            http_header_boundary(b"HTTP/1.1 200 OK\r\n\r\nevent: done\n\n"),
            Some((15, 4))
        );
        assert_eq!(
            http_header_boundary(b"HTTP/1.1 200 OK\n\nevent: done\n\n"),
            Some((15, 2))
        );
    }

    #[test]
    fn sse_frame_boundary_reports_separator_length() {
        assert_eq!(
            sse_frame_boundary(b"event: done\r\ndata: [DONE]\r\n\r\nnext"),
            Some((25, 4))
        );
        assert_eq!(
            sse_frame_boundary(b"event: done\ndata: [DONE]\n\nnext"),
            Some((24, 2))
        );
        assert_eq!(
            sse_frame_boundary(b"event: done\rdata: [DONE]\r\rnext"),
            Some((24, 2))
        );
    }

    #[test]
    fn double_newline_boundary_prefers_earliest_separator() {
        assert_eq!(double_newline_boundary(b"a\n\nb\r\n\r\nc"), Some((1, 2)));
        assert_eq!(double_newline_boundary(b"a\r\n\r\nb\n\nc"), Some((1, 4)));
    }

    #[test]
    fn http_content_length_is_case_insensitive_and_trims_value() {
        assert_eq!(
            http_content_length("HTTP/1.1 500 Error\r\nContent-Length:  14 \r\n"),
            Some(14)
        );
        assert_eq!(
            http_content_length("HTTP/1.1 500 Error\r\ncontent-length: 0\r\n"),
            Some(0)
        );
    }

    #[test]
    fn stream_timeout_message_formats_subsecond_durations() {
        assert_eq!(
            stream_timeout_message("body", Duration::from_millis(250)),
            "backend stream timed out after 250ms while waiting for body"
        );
        assert_eq!(
            stream_timeout_message("headers", Duration::from_secs(30)),
            "backend stream timed out after 30s while waiting for headers"
        );
        assert_eq!(
            stream_timeout_message("body", Duration::from_millis(1500)),
            "backend stream timed out after 1500ms while waiting for body"
        );
    }

    #[test]
    fn next_read_timeout_preserves_existing_short_poll_interval() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = thread::spawn(move || listener.accept().unwrap());
        let stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();

        set_next_read_timeout(&stream, Instant::now(), Duration::from_secs(30)).unwrap();

        assert_eq!(
            stream.read_timeout().unwrap(),
            Some(Duration::from_millis(20))
        );
        drop(stream);
        let _ = accepted.join().unwrap();
    }

    #[test]
    fn next_read_timeout_uses_short_poll_when_socket_has_no_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = thread::spawn(move || listener.accept().unwrap());
        let stream = TcpStream::connect(address).unwrap();
        assert_eq!(stream.read_timeout().unwrap(), None);

        set_next_read_timeout(&stream, Instant::now(), Duration::from_secs(30)).unwrap();

        assert_eq!(
            stream.read_timeout().unwrap(),
            Some(BACKEND_STREAM_READ_POLL_INTERVAL)
        );
        drop(stream);
        let _ = accepted.join().unwrap();
    }

    #[test]
    fn next_read_timeout_caps_existing_long_timeout_to_short_poll_interval() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = thread::spawn(move || listener.accept().unwrap());
        let stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();

        set_next_read_timeout(&stream, Instant::now(), Duration::from_secs(900)).unwrap();

        assert_eq!(
            stream.read_timeout().unwrap(),
            Some(BACKEND_STREAM_READ_POLL_INTERVAL)
        );
        drop(stream);
        let _ = accepted.join().unwrap();
    }

    #[test]
    fn next_read_timeout_caps_existing_poll_to_remaining_total_window() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = thread::spawn(move || listener.accept().unwrap());
        let stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let started = Instant::now() - Duration::from_millis(950);

        set_next_read_timeout(&stream, started, Duration::from_secs(1)).unwrap();

        let timeout = stream.read_timeout().unwrap().unwrap();
        assert!(timeout <= Duration::from_millis(50));
        assert!(timeout >= Duration::from_millis(1));
        drop(stream);
        let _ = accepted.join().unwrap();
    }

    #[test]
    fn stream_backend_events_heartbeats_while_waiting_for_slow_backend_headers() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(80));
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n\
event: done\ndata: [DONE]\n\n",
                )
                .unwrap();
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let mut events = Vec::new();
        stream_backend_events(&mut stream, Duration::from_secs(2), &mut |event, data| {
            events.push((event.to_owned(), data.to_owned()));
            Ok(())
        })
        .unwrap();
        server.join().unwrap();

        assert!(
            events
                .iter()
                .any(|(event, data)| event == "heartbeat" && data.contains("本地后端"))
        );
        assert_eq!(
            events.last(),
            Some(&("done".to_owned(), "[DONE]".to_owned()))
        );
    }

    #[test]
    fn stream_backend_events_times_out_waiting_for_headers_after_heartbeats() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(160));
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let mut events = Vec::new();
        let error = stream_backend_events(
            &mut stream,
            Duration::from_millis(70),
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap_err();
        server.join().unwrap();

        assert!(
            events
                .iter()
                .any(|(event, data)| event == "heartbeat" && data.contains("本地后端"))
        );
        assert!(error.contains("backend stream timed out after 70ms"));
        assert!(error.contains("waiting for headers"));
    }

    #[test]
    fn stream_backend_events_caps_header_wait_to_total_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(300));
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let started = Instant::now();
        let mut events = Vec::new();
        let error = stream_backend_events(
            &mut stream,
            Duration::from_millis(80),
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap_err();
        let elapsed = started.elapsed();
        server.join().unwrap();

        assert!(
            events
                .iter()
                .any(|(event, data)| event == "heartbeat" && data.contains("本地后端"))
        );
        assert!(error.contains("backend stream timed out after 80ms"));
        assert!(error.contains("waiting for headers"));
        assert!(
            elapsed < Duration::from_secs(1),
            "expected total timeout cap to return before the 2s socket timeout; elapsed={elapsed:?}"
        );
    }

    #[test]
    fn stream_backend_events_heartbeats_while_waiting_for_slow_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n")
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(80));
            stream.write_all(b"event: done\ndata: [DONE]\n\n").unwrap();
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let mut events = Vec::new();
        stream_backend_events(&mut stream, Duration::from_secs(2), &mut |event, data| {
            events.push((event.to_owned(), data.to_owned()));
            Ok(())
        })
        .unwrap();
        server.join().unwrap();

        assert!(
            events
                .iter()
                .any(|(event, data)| event == "heartbeat" && data.contains("本地后端"))
        );
        assert_eq!(
            events.last(),
            Some(&("done".to_owned(), "[DONE]".to_owned()))
        );
    }

    #[test]
    fn stream_backend_events_times_out_waiting_for_body_after_heartbeats() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n")
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(160));
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let mut events = Vec::new();
        let error = stream_backend_events(
            &mut stream,
            Duration::from_millis(70),
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap_err();
        server.join().unwrap();

        assert!(
            events
                .iter()
                .any(|(event, data)| event == "heartbeat" && data.contains("本地后端"))
        );
        assert!(error.contains("backend stream timed out after 70ms"));
        assert!(error.contains("waiting for body"));
    }

    #[test]
    fn stream_backend_events_accepts_done_terminal_event() {
        let events = stream_response(b"event: delta\ndata: hello\n\nevent: done\ndata: [DONE]\n\n");

        assert_eq!(
            events,
            vec![
                ("delta".to_owned(), "hello".to_owned()),
                ("done".to_owned(), "[DONE]".to_owned()),
            ]
        );
    }

    #[test]
    fn stream_backend_events_accepts_done_terminal_event_without_data() {
        let events = stream_response(b"event: done\n\n");

        assert_eq!(events, vec![("done".to_owned(), String::new())]);
    }

    #[test]
    fn stream_backend_events_accepts_lf_only_http_headers() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\ncontent-type: text/event-stream\n\n\
event: done\ndata: [DONE]\n\n",
                )
                .unwrap();
        });

        let mut stream = TcpStream::connect(address).unwrap();
        let mut events = Vec::new();
        stream_backend_events(&mut stream, Duration::from_secs(30), &mut |event, data| {
            events.push((event.to_owned(), data.to_owned()));
            Ok(())
        })
        .unwrap();
        server.join().unwrap();

        assert_eq!(events, vec![("done".to_owned(), "[DONE]".to_owned())]);
    }

    #[test]
    fn stream_backend_events_accepts_cr_only_sse_frames() {
        let events = stream_response(b"event: delta\rdata: hello\r\revent: done\rdata: [DONE]\r\r");

        assert_eq!(
            events,
            vec![
                ("delta".to_owned(), "hello".to_owned()),
                ("done".to_owned(), "[DONE]".to_owned()),
            ]
        );
    }

    #[test]
    fn stream_backend_events_returns_http_error_without_waiting_for_eof() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 14\r\n\r\nbackend failed",
                )
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(200));
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let error =
            stream_backend_events(&mut stream, Duration::from_millis(100), &mut |_, _| Ok(()))
                .unwrap_err();
        server.join().unwrap();

        assert!(error.contains("backend stream returned HTTP 500"));
        assert!(error.contains("backend failed"));
    }

    #[test]
    fn stream_backend_events_returns_lf_only_http_error_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(b"HTTP/1.1 502 Bad Gateway\ncontent-length: 14\n\nbackend failed")
                .unwrap();
        });

        let mut stream = TcpStream::connect(address).unwrap();
        let error = stream_backend_events(&mut stream, Duration::from_secs(30), &mut |_, _| Ok(()))
            .unwrap_err();
        server.join().unwrap();

        assert!(error.contains("backend stream returned HTTP 502"));
        assert!(error.contains("backend failed"));
    }

    #[test]
    fn stream_backend_events_returns_partial_http_error_body_on_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 100\r\n\r\npartial failure",
                )
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(200));
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let error =
            stream_backend_events(&mut stream, Duration::from_millis(100), &mut |_, _| Ok(()))
                .unwrap_err();
        server.join().unwrap();

        assert!(error.contains("backend stream returned HTTP 500"));
        assert!(error.contains("partial failure"));
    }

    #[test]
    fn stream_backend_events_caps_http_error_body_wait_to_total_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 100\r\n\r\npartial failure",
                )
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(300));
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let started = Instant::now();
        let error =
            stream_backend_events(&mut stream, Duration::from_millis(80), &mut |_, _| Ok(()))
                .unwrap_err();
        let elapsed = started.elapsed();
        server.join().unwrap();

        assert!(error.contains("backend stream returned HTTP 500"));
        assert!(error.contains("partial failure"));
        assert!(
            elapsed < Duration::from_secs(1),
            "expected total timeout cap to return before the 2s socket timeout; elapsed={elapsed:?}"
        );
    }

    #[test]
    fn stream_backend_events_truncates_buffered_http_error_body_to_content_length() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 14\r\n\r\nbackend failedextra bytes",
                )
                .unwrap();
        });

        let mut stream = TcpStream::connect(address).unwrap();
        let error = stream_backend_events(&mut stream, Duration::from_secs(30), &mut |_, _| Ok(()))
            .unwrap_err();
        server.join().unwrap();

        assert!(error.contains("backend failed"));
        assert!(!error.contains("extra bytes"));
    }

    #[test]
    fn stream_backend_events_returns_after_done_without_waiting_for_eof() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n\
event: done\ndata: [DONE]\n\n",
                )
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(200));
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let mut events = Vec::new();

        stream_backend_events(
            &mut stream,
            Duration::from_millis(100),
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(events, vec![("done".to_owned(), "[DONE]".to_owned())]);
    }

    #[test]
    fn stream_backend_events_ignores_frames_after_terminal_event() {
        let events = stream_response(
            b"event: done\ndata: [DONE]\n\nevent: delta\ndata: should-not-render\n\n",
        );

        assert_eq!(events, vec![("done".to_owned(), "[DONE]".to_owned())]);
    }

    #[test]
    fn stream_backend_events_stops_after_error_terminal_event() {
        let events = stream_response(
            b"event: error\ndata: backend failed\n\nevent: delta\ndata: should-not-render\n\n",
        );

        assert_eq!(
            events,
            vec![("error".to_owned(), "backend failed".to_owned())]
        );
    }

    #[test]
    fn stream_backend_events_rejects_complete_delta_without_done() {
        let error = stream_response_error(b"event: delta\ndata: hello\n\n");

        assert!(error.contains("backend stream truncated"));
        assert!(error.contains("after SSE events"));
        assert!(error.contains("before done"));
    }

    #[test]
    fn stream_backend_events_does_not_count_comment_frames_as_events() {
        let error = stream_response_error(b": keep-alive\n\n");

        assert!(error.contains("backend stream truncated"));
        assert!(error.contains("EOF before done or error terminal event"));
    }

    #[test]
    fn stream_backend_events_rejects_incomplete_leftover_at_eof() {
        let error = stream_response_error(b"event: delta\ndata: hello\n");

        assert!(error.contains("backend stream truncated"));
        assert!(error.contains("incomplete SSE frame"));
    }
}
