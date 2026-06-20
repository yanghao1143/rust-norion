use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::sse::{frame_boundary, relay_sse_frame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) body: String,
}

pub(crate) fn get(backend: &str, path: &str, timeout_secs: u64) -> Result<HttpResponse, String> {
    let mut stream = connect(backend, timeout_secs)?;
    let request = format!("GET {path} HTTP/1.1\r\nhost: {backend}\r\nconnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write GET {path} failed: {error}"))?;
    read_http_response(&mut stream, path)
}

pub(crate) fn post_json(
    backend: &str,
    path: &str,
    body: &str,
    timeout_secs: u64,
) -> Result<HttpResponse, String> {
    let mut stream = connect(backend, timeout_secs)?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nhost: {backend}\r\ncontent-type: application/json; charset=utf-8\r\naccept: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write POST {path} failed: {error}"))?;
    read_http_response(&mut stream, path)
}

pub(crate) fn post_event_stream(
    backend: &str,
    path: &str,
    body: &str,
    timeout_secs: u64,
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), String>,
) -> Result<(), String> {
    let mut stream = connect(backend, timeout_secs)?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nhost: {backend}\r\ncontent-type: application/json; charset=utf-8\r\naccept: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write POST {path} failed: {error}"))?;
    stream_event_response(&mut stream, path, on_event)
}

fn connect(backend: &str, timeout_secs: u64) -> Result<TcpStream, String> {
    let stream = TcpStream::connect(backend)
        .map_err(|error| format!("connect {backend} failed: {error}"))?;
    let timeout = Some(Duration::from_secs(timeout_secs));
    stream
        .set_read_timeout(timeout)
        .map_err(|error| format!("set read timeout failed: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|error| format!("set write timeout failed: {error}"))?;
    Ok(stream)
}

fn read_http_response(stream: &mut TcpStream, path: &str) -> Result<HttpResponse, String> {
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("read {path} response failed: {error}"))?;
    let (head, body) = split_http_head_body(&response);
    Ok(HttpResponse {
        status: http_status(head),
        body: body.to_owned(),
    })
}

fn stream_event_response<R: Read>(
    stream: &mut R,
    path: &str,
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), String>,
) -> Result<(), String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let mut saw_terminal_event = false;
    let header_end = loop {
        let read = stream
            .read(&mut chunk)
            .map_err(|error| format!("read {path} stream headers failed: {error}"))?;
        if read == 0 {
            return Err(format!("{path} stream closed before headers"));
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(header_end) = find_header_end(&buffer) {
            break header_end;
        }
    };

    let headers = String::from_utf8_lossy(&buffer[..header_end]);
    let status = http_status(&headers);
    let mut body = buffer.get(header_end + 4..).unwrap_or_default().to_vec();
    if !(200..300).contains(&status) {
        stream
            .read_to_end(&mut body)
            .map_err(|error| format!("read {path} error body failed: {error}"))?;
        return Err(format!(
            "{path} returned HTTP {status}: {}",
            String::from_utf8_lossy(&body).trim()
        ));
    }

    loop {
        while let Some((frame_end, boundary_len)) = frame_boundary(&body) {
            let frame = body[..frame_end].to_vec();
            body.drain(..frame_end + boundary_len);
            relay_terminal_sse_frame(&frame, on_event, &mut saw_terminal_event)?;
        }
        let read = stream
            .read(&mut chunk)
            .map_err(|error| format!("read {path} stream body failed: {error}"))?;
        if read == 0 {
            if !body.is_empty() {
                return Err(format!("{path} stream truncated before SSE frame boundary"));
            }
            return if saw_terminal_event {
                Ok(())
            } else {
                Err(format!("{path} stream truncated before terminal event"))
            };
        }
        body.extend_from_slice(&chunk[..read]);
    }
}

fn relay_terminal_sse_frame(
    frame: &[u8],
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), String>,
    saw_terminal_event: &mut bool,
) -> Result<(), String> {
    relay_sse_frame(frame, &mut |event, data| {
        if matches!(event, "done" | "error") {
            *saw_terminal_event = true;
        }
        on_event(event, data)
    })
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn split_http_head_body(response: &str) -> (&str, &str) {
    response
        .split_once("\r\n\r\n")
        .or_else(|| response.split_once("\n\n"))
        .unwrap_or((response, ""))
}

fn http_status(head: &str) -> u16 {
    head.lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn splits_http_response() {
        let (head, body) = split_http_head_body("HTTP/1.1 200 OK\r\nx: y\r\n\r\nbody");

        assert_eq!(http_status(head), 200);
        assert_eq!(body, "body");
    }

    #[test]
    fn parses_status_without_body_separator() {
        let (head, body) = split_http_head_body("HTTP/1.1 204 No Content\r\nx: y");

        assert_eq!(http_status(head), 204);
        assert_eq!(body, "");
    }

    #[test]
    fn stream_event_response_requires_done_terminal_event() {
        let response = concat!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n",
            "event: delta\r\n",
            "data: hello\r\n\r\n",
            "event: done\r\n",
            "data: [DONE]\r\n\r\n"
        );
        let mut events = Vec::new();
        let mut stream = Cursor::new(response.as_bytes());

        stream_event_response(
            &mut stream,
            "/v1/business-cycle-stream",
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(
            events,
            vec![
                ("delta".to_owned(), "hello".to_owned()),
                ("done".to_owned(), "[DONE]".to_owned())
            ]
        );
    }

    #[test]
    fn stream_event_response_accepts_error_as_terminal_event() {
        let response = concat!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n",
            "event: error\r\n",
            "data: backend failed\r\n\r\n"
        );
        let mut events = Vec::new();
        let mut stream = Cursor::new(response.as_bytes());

        stream_event_response(
            &mut stream,
            "/v1/business-cycle-stream",
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(
            events,
            vec![("error".to_owned(), "backend failed".to_owned())]
        );
    }

    #[test]
    fn stream_event_response_rejects_delta_only_eof() {
        let response = concat!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n",
            "event: delta\r\n",
            "data: partial\r\n\r\n"
        );
        let mut stream = Cursor::new(response.as_bytes());

        let error =
            stream_event_response(&mut stream, "/v1/business-cycle-stream", &mut |_, _| Ok(()))
                .unwrap_err();

        assert!(error.contains("terminal event"), "{error}");
    }

    #[test]
    fn stream_event_response_rejects_incomplete_leftover() {
        let response = concat!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n",
            "event: delta\r\n",
            "data: partial"
        );
        let mut stream = Cursor::new(response.as_bytes());

        let error =
            stream_event_response(&mut stream, "/v1/business-cycle-stream", &mut |_, _| Ok(()))
                .unwrap_err();

        assert!(error.contains("SSE frame boundary"), "{error}");
    }
}
