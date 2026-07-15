use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

pub(crate) const MODEL_POOL_CALL_CANCEL_MARKER: &[u8] = b"\r\nnorion-model-pool-cancel\r\n";

pub(crate) fn read_http_request(stream: &mut TcpStream) -> std::io::Result<String> {
    const MAX_REQUEST_BYTES: usize = 1_048_576;
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;
    let mut request_len = None;

    loop {
        let read_limit = if let Some(request_len) = request_len {
            if buffer.len() >= request_len {
                break;
            }
            request_len.saturating_sub(buffer.len()).min(chunk.len())
        } else if let Some(header_end) = header_end {
            if buffer.len() < header_end {
                header_end.saturating_sub(buffer.len()).min(chunk.len())
            } else {
                let headers = String::from_utf8_lossy(&buffer[..header_end]);
                let total = header_end
                    .checked_add(parse_content_length(&headers).unwrap_or(0))
                    .filter(|total| *total <= MAX_REQUEST_BYTES)
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "HTTP request exceeds 1 MiB limit",
                        )
                    })?;
                request_len = Some(total);
                continue;
            }
        } else {
            let peeked = stream.peek(&mut chunk)?;
            if peeked == 0 {
                break;
            }
            if let Some(found_header_end) = find_http_header_end_after(&buffer, &chunk[..peeked]) {
                header_end = Some(found_header_end);
                found_header_end.saturating_sub(buffer.len())
            } else {
                if buffer.len().saturating_add(peeked) > MAX_REQUEST_BYTES {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "HTTP request exceeds 1 MiB limit",
                    ));
                }
                peeked
            }
        };
        if read_limit == 0 {
            break;
        }
        let read = stream.read(&mut chunk[..read_limit])?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }

    String::from_utf8(buffer)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

pub(crate) fn split_http_head_body(raw: &str) -> (&str, &str) {
    raw.split_once("\r\n\r\n")
        .or_else(|| raw.split_once("\n\n"))
        .unwrap_or((raw, ""))
}

pub(crate) fn reserve_model_service_loopback_addr() -> std::io::Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    drop(listener);
    Ok(addr.to_string())
}

pub(crate) fn wait_for_model_service_http_response(
    addr: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> std::io::Result<String> {
    let mut last_error = None;
    for _ in 0..100 {
        match try_model_service_http_request(addr, method, path, body) {
            Ok(response) => return Ok(response),
            Err(error) => {
                last_error = Some(error);
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!(
            "model service did not respond at {addr}: {}",
            last_error
                .map(|error| error.to_string())
                .unwrap_or_else(|| "no connection attempt completed".to_owned())
        ),
    ))
}

pub(crate) fn model_service_http_request(
    addr: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> std::io::Result<String> {
    try_model_service_http_request(addr, method, path, body)
}

pub(crate) fn try_model_service_http_request(
    addr: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> std::io::Result<String> {
    let body = body.unwrap_or("");
    let request = if method.eq_ignore_ascii_case("GET") {
        format!("GET {path} HTTP/1.1\r\nhost: {addr}\r\nconnection: close\r\n\r\n")
    } else {
        format!(
            "{method} {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-type: application/json; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
    };
    let mut stream = TcpStream::connect(addr)?;
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

pub(crate) fn model_service_http_body(response: &str) -> &str {
    split_http_head_body(response).1
}

fn find_http_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .or_else(|| {
            buffer
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|index| index + 2)
        })
}

fn find_http_header_end_after(buffer: &[u8], next: &[u8]) -> Option<usize> {
    let suffix_len = buffer.len().min(3);
    let suffix_start = buffer.len().saturating_sub(suffix_len);
    let mut preview = [0_u8; 1027];
    preview[..suffix_len].copy_from_slice(&buffer[suffix_start..]);
    preview[suffix_len..suffix_len + next.len()].copy_from_slice(next);
    find_http_header_end(&preview[..suffix_len + next.len()])
        .map(|header_end| suffix_start + header_end)
}

fn parse_content_length(headers: &str) -> Option<usize> {
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_reader_preserves_bytes_after_content_length() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let client = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            let request = format!(
                "POST /v1/model-pool/call HTTP/1.1\r\nhost: {address}\r\ncontent-length: 2\r\n\r\n{{}}{}",
                String::from_utf8_lossy(MODEL_POOL_CALL_CANCEL_MARKER)
            );
            stream.write_all(request.as_bytes()).unwrap();
        });
        let (mut stream, _) = listener.accept().unwrap();

        let request = read_http_request(&mut stream).unwrap();
        let mut marker = vec![0_u8; MODEL_POOL_CALL_CANCEL_MARKER.len()];
        stream.read_exact(&mut marker).unwrap();

        client.join().unwrap();
        assert!(request.ends_with("\r\n\r\n{}"));
        assert_eq!(marker, MODEL_POOL_CALL_CANCEL_MARKER);
    }
}
