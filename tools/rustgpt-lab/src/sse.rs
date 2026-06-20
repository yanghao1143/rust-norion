use std::io::Write;
use std::net::TcpStream;

pub(crate) fn write_sse_headers(stream: &mut TcpStream) -> std::io::Result<()> {
    stream.write_all(
        b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream; charset=utf-8\r\ncache-control: no-cache\r\nconnection: close\r\n\r\n",
    )?;
    stream.flush()
}

pub(crate) fn send_sse(stream: &mut TcpStream, event: &str, data: &str) -> std::io::Result<()> {
    stream.write_all(format!("event: {event}\n").as_bytes())?;
    let data = normalize_sse_newlines(data);
    for line in data.split('\n') {
        stream.write_all(format!("data: {line}\n").as_bytes())?;
    }
    stream.write_all(b"\n")?;
    stream.flush()
}

fn normalize_sse_newlines(data: &str) -> String {
    data.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;
    use std::thread;

    fn capture_sse_write(write: impl FnOnce(&mut TcpStream) + Send + 'static) -> String {
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
    fn send_sse_preserves_trailing_newline_data() {
        let response = capture_sse_write(|stream| {
            send_sse(stream, "delta", "line\n").unwrap();
        });

        assert_eq!(response, "event: delta\ndata: line\ndata: \n\n");
    }

    #[test]
    fn send_sse_normalizes_crlf_data_lines() {
        let response = capture_sse_write(|stream| {
            send_sse(stream, "delta", "first\r\nsecond\rthird").unwrap();
        });

        assert_eq!(
            response,
            "event: delta\ndata: first\ndata: second\ndata: third\n\n"
        );
    }

    #[test]
    fn send_sse_preserves_empty_data() {
        let response = capture_sse_write(|stream| {
            send_sse(stream, "done", "").unwrap();
        });

        assert_eq!(response, "event: done\ndata: \n\n");
    }

    #[test]
    fn write_sse_headers_emits_event_stream_response() {
        let response = capture_sse_write(|stream| {
            write_sse_headers(stream).unwrap();
        });

        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.contains("content-type: text/event-stream; charset=utf-8\r\n"));
        assert!(response.ends_with("\r\n\r\n"));
    }
}
