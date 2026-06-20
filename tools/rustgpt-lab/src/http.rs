use std::io::{Read, Write};
use std::net::TcpStream;

pub(crate) fn read_http_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut buffer = [0_u8; 8192];
    let mut data = Vec::new();
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..read]);
        if let Some((head_end, boundary_len)) = http_head_body_boundary(&data) {
            let head = String::from_utf8_lossy(&data[..head_end]);
            let content_length = content_length(&head).unwrap_or(0);
            if data.len() >= head_end + boundary_len + content_length {
                break;
            }
        }
        if data.len() > 2_000_000 {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&data).into_owned())
}

pub(crate) fn split_http_head_body(raw: &str) -> (&str, &str) {
    if let Some((head_end, boundary_len)) = http_head_body_boundary(raw.as_bytes()) {
        (&raw[..head_end], &raw[head_end + boundary_len..])
    } else {
        (raw, "")
    }
}

pub(crate) fn write_html(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    write_static(stream, "text/html; charset=utf-8", body)
}

pub(crate) fn write_static(
    stream: &mut TcpStream,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\ncache-control: no-store\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())
}

pub(crate) fn write_json(stream: &mut TcpStream, status: u16, body: &str) -> std::io::Result<()> {
    let reason = if status == 200 { "OK" } else { "Error" };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())
}

fn content_length(head: &str) -> Option<usize> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    })
}

fn http_head_body_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
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
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    fn capture_http_write(write: impl FnOnce(&mut TcpStream) + Send + 'static) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            write(&mut stream);
        });

        let mut client = TcpStream::connect(address).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        server.join().unwrap();
        response
    }

    #[test]
    fn split_http_head_body_accepts_lf_only_separator() {
        let (head, body) = split_http_head_body("HTTP/1.1 200 OK\ncontent-length: 2\n\nok");

        assert_eq!(head, "HTTP/1.1 200 OK\ncontent-length: 2");
        assert_eq!(body, "ok");
    }

    #[test]
    fn split_http_head_body_preserves_crlf_separator_behavior() {
        let (head, body) = split_http_head_body("HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok");

        assert_eq!(head, "HTTP/1.1 200 OK\r\ncontent-length: 2");
        assert_eq!(body, "ok");
    }

    #[test]
    fn http_head_body_boundary_prefers_earliest_separator() {
        assert_eq!(http_head_body_boundary(b"a\n\nb\r\n\r\nc"), Some((1, 2)));
        assert_eq!(http_head_body_boundary(b"a\r\n\r\nb\n\nc"), Some((1, 4)));
    }

    #[test]
    fn read_http_request_accepts_lf_only_headers_without_client_close() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(300)))
                .unwrap();
            read_http_request(&mut stream).unwrap()
        });

        let mut client = TcpStream::connect(address).unwrap();
        client
            .write_all(b"POST /api/chat HTTP/1.1\nhost: local\ncontent-length: 5\n\nhello")
            .unwrap();

        let request = server.join().unwrap();
        assert!(request.starts_with("POST /api/chat HTTP/1.1\n"));
        assert!(request.ends_with("\n\nhello"));
    }

    #[test]
    fn read_http_request_waits_for_lf_only_content_length_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(500)))
                .unwrap();
            read_http_request(&mut stream).unwrap()
        });

        let mut client = TcpStream::connect(address).unwrap();
        client
            .write_all(b"POST /api/chat HTTP/1.1\ncontent-length: 5\n\nhe")
            .unwrap();
        thread::sleep(Duration::from_millis(40));
        client.write_all(b"llo").unwrap();

        let request = server.join().unwrap();
        assert!(request.ends_with("\n\nhello"));
    }

    #[test]
    fn read_http_request_counts_content_length_as_utf8_bytes() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(500)))
                .unwrap();
            read_http_request(&mut stream).unwrap()
        });

        let body = "你好";
        let mut client = TcpStream::connect(address).unwrap();
        let request_head = format!(
            "POST /api/chat HTTP/1.1\ncontent-length: {}\n\n",
            body.len()
        );
        client.write_all(request_head.as_bytes()).unwrap();
        let body_bytes = body.as_bytes();
        client.write_all(&body_bytes[..3]).unwrap();
        thread::sleep(Duration::from_millis(40));
        client.write_all(&body_bytes[3..]).unwrap();

        let request = server.join().unwrap();
        assert!(request.ends_with("\n\n你好"));
    }

    #[test]
    fn write_json_counts_content_length_as_utf8_bytes() {
        let body = "{\"message\":\"你好\"}";
        let response = capture_http_write(move |stream| {
            write_json(stream, 200, body).unwrap();
        });

        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.contains(&format!("content-length: {}\r\n", body.len())));
        assert!(response.ends_with(body));
    }

    #[test]
    fn write_static_counts_content_length_as_utf8_bytes() {
        let body = "<p>你好</p>";
        let response = capture_http_write(move |stream| {
            write_static(stream, "text/html; charset=utf-8", body).unwrap();
        });

        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.contains("content-type: text/html; charset=utf-8\r\n"));
        assert!(response.contains(&format!("content-length: {}\r\n", body.len())));
        assert!(response.ends_with(body));
    }
}
