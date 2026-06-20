use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use crate::runtime::RuntimeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::runtime) struct HttpEndpoint {
    host: String,
    port: u16,
    base_path: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::runtime) struct HttpStreamTimeouts {
    pub(in crate::runtime) total_ms: Option<u64>,
    pub(in crate::runtime) idle_ms: Option<u64>,
}

impl HttpStreamTimeouts {
    pub(in crate::runtime) fn new(total_ms: Option<u64>, idle_ms: Option<u64>) -> Self {
        Self {
            total_ms: total_ms.map(|timeout| timeout.max(1)),
            idle_ms: idle_ms.map(|timeout| timeout.max(1)),
        }
    }
}

impl HttpEndpoint {
    pub(in crate::runtime) fn parse(base_url: &str) -> Result<Self, RuntimeError> {
        let trimmed = base_url.trim().trim_end_matches('/');
        let without_scheme = trimmed.strip_prefix("http://").unwrap_or(trimmed);
        if without_scheme.starts_with("https://") {
            return Err(RuntimeError::new(
                "mistralrs HTTP runtime only supports local http:// endpoints",
            ));
        }

        let (authority, path) = without_scheme
            .split_once('/')
            .map(|(authority, path)| (authority, format!("/{path}")))
            .unwrap_or_else(|| (without_scheme, String::new()));
        if authority.is_empty() {
            return Err(RuntimeError::new(
                "mistralrs HTTP runtime endpoint must include host:port",
            ));
        }

        let (host, port) = parse_authority(authority)?;
        Ok(Self {
            host,
            port,
            base_path: normalize_base_path(&path),
        })
    }

    pub(in crate::runtime) fn post_json(
        &self,
        path: &str,
        body: &str,
        timeout_ms: Option<u64>,
    ) -> Result<String, RuntimeError> {
        let mut stream = TcpStream::connect((self.host.as_str(), self.port)).map_err(|error| {
            RuntimeError::new(format!(
                "failed to connect mistralrs HTTP runtime at {}:{}: {error}",
                self.host, self.port
            ))
        })?;
        if let Some(timeout_ms) = timeout_ms {
            let timeout = Duration::from_millis(timeout_ms.max(1));
            stream.set_read_timeout(Some(timeout)).map_err(|error| {
                RuntimeError::new(format!("failed to set HTTP read timeout: {error}"))
            })?;
            stream.set_write_timeout(Some(timeout)).map_err(|error| {
                RuntimeError::new(format!("failed to set HTTP write timeout: {error}"))
            })?;
        }

        let request_path = self.request_path(path);
        let request = format!(
            "POST {request_path} HTTP/1.1\r\n\
             Host: {}:{}\r\n\
             Content-Type: application/json; charset=utf-8\r\n\
             Accept: application/json\r\n\
             Connection: close\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            self.host,
            self.port,
            body.len(),
            body
        );
        stream.write_all(request.as_bytes()).map_err(|error| {
            RuntimeError::new(format!("failed to write mistralrs HTTP request: {error}"))
        })?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response).map_err(|error| {
            RuntimeError::new(format!("failed to read mistralrs HTTP response: {error}"))
        })?;
        parse_http_response(&response)
    }

    pub(in crate::runtime) fn post_json_stream(
        &self,
        path: &str,
        body: &str,
        timeouts: HttpStreamTimeouts,
        on_body_chunk: &mut dyn FnMut(&[u8]) -> Result<(), RuntimeError>,
    ) -> Result<Vec<u8>, RuntimeError> {
        let mut stream = TcpStream::connect((self.host.as_str(), self.port)).map_err(|error| {
            RuntimeError::new(format!(
                "failed to connect mistralrs HTTP runtime at {}:{}: {error}",
                self.host, self.port
            ))
        })?;
        let mut read_budget = HttpStreamReadBudget::new(timeouts);
        if let Some(write_timeout_ms) = timeouts.total_ms.or(timeouts.idle_ms) {
            let timeout = Duration::from_millis(write_timeout_ms.max(1));
            stream.set_write_timeout(Some(timeout)).map_err(|error| {
                RuntimeError::new(format!("failed to set HTTP write timeout: {error}"))
            })?;
        }

        let request_path = self.request_path(path);
        let request = format!(
            "POST {request_path} HTTP/1.1\r\n\
             Host: {}:{}\r\n\
             Content-Type: application/json; charset=utf-8\r\n\
             Accept: text/event-stream, application/json\r\n\
             Connection: close\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            self.host,
            self.port,
            body.len(),
            body
        );
        stream.write_all(request.as_bytes()).map_err(|error| {
            RuntimeError::new(format!("failed to write mistralrs HTTP request: {error}"))
        })?;

        let (headers, initial_body) = read_http_headers(&mut stream, &mut read_budget)?;
        let status_code = parse_http_status_code(&headers)?;
        if !(200..300).contains(&status_code) {
            let mut body = initial_body;
            stream.read_to_end(&mut body).map_err(|error| {
                RuntimeError::new(format!("failed to read mistralrs HTTP error body: {error}"))
            })?;
            return Err(RuntimeError::new(format!(
                "mistralrs HTTP runtime returned status {status_code}: {}",
                String::from_utf8_lossy(&body).trim()
            )));
        }

        if headers.lines().any(|line| {
            line.to_ascii_lowercase()
                .contains("transfer-encoding: chunked")
        }) {
            stream_chunked_body(initial_body, &mut stream, &mut read_budget, on_body_chunk)
        } else {
            stream_plain_body(initial_body, &mut stream, &mut read_budget, on_body_chunk)
        }
    }

    fn request_path(&self, path: &str) -> String {
        let path = if path.starts_with('/') {
            path.to_owned()
        } else {
            format!("/{path}")
        };
        if self.base_path.is_empty() {
            path
        } else if path == "/" {
            self.base_path.clone()
        } else {
            format!("{}{}", self.base_path, path)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamTimeoutKind {
    Idle,
    Total,
}

#[derive(Debug)]
struct HttpStreamReadBudget {
    started_at: Instant,
    total_ms: Option<u64>,
    idle_ms: Option<u64>,
    response_bytes: usize,
    body_bytes: usize,
    body_chunks: usize,
}

impl HttpStreamReadBudget {
    fn new(timeouts: HttpStreamTimeouts) -> Self {
        Self {
            started_at: Instant::now(),
            total_ms: timeouts.total_ms,
            idle_ms: timeouts.idle_ms,
            response_bytes: 0,
            body_bytes: 0,
            body_chunks: 0,
        }
    }

    fn record_body_chunk(&mut self, len: usize) {
        if len > 0 {
            self.body_bytes += len;
            self.body_chunks += 1;
        }
    }

    fn read(
        &mut self,
        stream: &mut TcpStream,
        phase: &'static str,
        buffer: &mut [u8],
    ) -> Result<usize, RuntimeError> {
        let timeout_kind = self.apply_read_timeout(stream, phase)?;
        match stream.read(buffer) {
            Ok(read) => {
                self.response_bytes += read;
                Ok(read)
            }
            Err(error)
                if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)
                    && timeout_kind.is_some() =>
            {
                Err(self.timeout_error(phase, timeout_kind.unwrap()))
            }
            Err(error) => Err(RuntimeError::new(format!(
                "failed to read mistralrs HTTP {phase}: {error}"
            ))),
        }
    }

    fn apply_read_timeout(
        &self,
        stream: &mut TcpStream,
        phase: &'static str,
    ) -> Result<Option<StreamTimeoutKind>, RuntimeError> {
        let timeout = self.next_read_timeout(phase)?;
        stream
            .set_read_timeout(timeout.map(|(_, duration)| duration))
            .map_err(|error| {
                RuntimeError::new(format!("failed to set HTTP stream read timeout: {error}"))
            })?;
        Ok(timeout.map(|(kind, _)| kind))
    }

    fn next_read_timeout(
        &self,
        phase: &'static str,
    ) -> Result<Option<(StreamTimeoutKind, Duration)>, RuntimeError> {
        let total_remaining = if let Some(total_ms) = self.total_ms {
            let elapsed_ms = self.started_at.elapsed().as_millis() as u64;
            if elapsed_ms >= total_ms {
                return Err(self.timeout_error(phase, StreamTimeoutKind::Total));
            }
            Some(total_ms - elapsed_ms)
        } else {
            None
        };

        match (self.idle_ms, total_remaining) {
            (Some(idle_ms), Some(total_ms)) if total_ms <= idle_ms => Ok(Some((
                StreamTimeoutKind::Total,
                Duration::from_millis(total_ms.max(1)),
            ))),
            (Some(idle_ms), _) => Ok(Some((
                StreamTimeoutKind::Idle,
                Duration::from_millis(idle_ms.max(1)),
            ))),
            (None, Some(total_ms)) => Ok(Some((
                StreamTimeoutKind::Total,
                Duration::from_millis(total_ms.max(1)),
            ))),
            (None, None) => Ok(None),
        }
    }

    fn timeout_error(&self, phase: &'static str, kind: StreamTimeoutKind) -> RuntimeError {
        match kind {
            StreamTimeoutKind::Idle => RuntimeError::new(format!(
                "mistralrs HTTP stream idle timeout after {} ms while reading {phase}; received {} response bytes, emitted {} body bytes in {} chunks",
                self.idle_ms.unwrap_or_default(),
                self.response_bytes,
                self.body_bytes,
                self.body_chunks
            )),
            StreamTimeoutKind::Total => RuntimeError::new(format!(
                "mistralrs HTTP stream total timeout after {} ms while reading {phase}; received {} response bytes, emitted {} body bytes in {} chunks",
                self.total_ms.unwrap_or_default(),
                self.response_bytes,
                self.body_bytes,
                self.body_chunks
            )),
        }
    }
}

fn read_http_headers(
    stream: &mut TcpStream,
    read_budget: &mut HttpStreamReadBudget,
) -> Result<(String, Vec<u8>), RuntimeError> {
    let mut response = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let read = read_budget.read(stream, "headers", &mut chunk)?;
        if read == 0 {
            return Err(RuntimeError::new(
                "mistralrs HTTP response did not include headers",
            ));
        }
        response.extend_from_slice(&chunk[..read]);
        if let Some(header_end) = find_header_end(&response) {
            let headers = String::from_utf8_lossy(&response[..header_end]).to_string();
            let body_start = header_end + 4;
            let body = response.get(body_start..).unwrap_or_default().to_vec();
            return Ok((headers, body));
        }
    }
}

fn parse_http_status_code(headers: &str) -> Result<u16, RuntimeError> {
    headers
        .lines()
        .next()
        .and_then(|status| status.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| RuntimeError::new("mistralrs HTTP response had invalid status line"))
}

fn stream_plain_body(
    initial_body: Vec<u8>,
    stream: &mut TcpStream,
    read_budget: &mut HttpStreamReadBudget,
    on_body_chunk: &mut dyn FnMut(&[u8]) -> Result<(), RuntimeError>,
) -> Result<Vec<u8>, RuntimeError> {
    let mut body = Vec::new();
    if !initial_body.is_empty() {
        on_body_chunk(&initial_body)?;
        read_budget.record_body_chunk(initial_body.len());
        body.extend_from_slice(&initial_body);
    }

    let mut chunk = [0_u8; 4096];
    loop {
        let read = read_budget.read(stream, "response body", &mut chunk)?;
        if read == 0 {
            return Ok(body);
        }
        on_body_chunk(&chunk[..read])?;
        read_budget.record_body_chunk(read);
        body.extend_from_slice(&chunk[..read]);
    }
}

fn stream_chunked_body(
    initial_body: Vec<u8>,
    stream: &mut TcpStream,
    read_budget: &mut HttpStreamReadBudget,
    on_body_chunk: &mut dyn FnMut(&[u8]) -> Result<(), RuntimeError>,
) -> Result<Vec<u8>, RuntimeError> {
    let mut buffer = initial_body;
    let mut body = Vec::new();
    loop {
        let line_end = loop {
            if let Some(line_end) = find_crlf(&buffer) {
                break line_end;
            }
            read_more_body(stream, read_budget, &mut buffer)?;
        };
        let size_line = String::from_utf8_lossy(&buffer[..line_end]);
        let size = usize::from_str_radix(size_line.trim(), 16)
            .map_err(|_| RuntimeError::new("chunked HTTP body had invalid chunk size"))?;
        let chunk_start = line_end + 2;
        if size == 0 {
            return Ok(body);
        }
        while buffer.len() < chunk_start + size + 2 {
            read_more_body(stream, read_budget, &mut buffer)?;
        }
        let chunk = buffer[chunk_start..chunk_start + size].to_vec();
        on_body_chunk(&chunk)?;
        read_budget.record_body_chunk(chunk.len());
        body.extend_from_slice(&chunk);
        buffer.drain(..chunk_start + size + 2);
    }
}

fn read_more_body(
    stream: &mut TcpStream,
    read_budget: &mut HttpStreamReadBudget,
    buffer: &mut Vec<u8>,
) -> Result<(), RuntimeError> {
    let mut chunk = [0_u8; 4096];
    let read = read_budget.read(stream, "chunked response body", &mut chunk)?;
    if read == 0 {
        return Err(RuntimeError::new(
            "chunked HTTP body ended before terminating chunk",
        ));
    }
    buffer.extend_from_slice(&chunk[..read]);
    Ok(())
}

fn parse_authority(authority: &str) -> Result<(String, u16), RuntimeError> {
    let (host, port) = authority.rsplit_once(':').ok_or_else(|| {
        RuntimeError::new("mistralrs HTTP runtime endpoint must include an explicit port")
    })?;
    if host.is_empty() {
        return Err(RuntimeError::new(
            "mistralrs HTTP runtime endpoint host must not be empty",
        ));
    }
    let port = port.parse::<u16>().map_err(|_| {
        RuntimeError::new("mistralrs HTTP runtime endpoint port must be a valid u16")
    })?;
    Ok((host.to_owned(), port))
}

fn normalize_base_path(path: &str) -> String {
    let path = path.trim_end_matches('/');
    if path.is_empty() || path == "/" {
        String::new()
    } else if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    }
}

fn parse_http_response(response: &[u8]) -> Result<String, RuntimeError> {
    let Some(header_end) = find_header_end(response) else {
        return Err(RuntimeError::new(
            "mistralrs HTTP response did not include headers",
        ));
    };
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let mut lines = headers.lines();
    let status = lines
        .next()
        .ok_or_else(|| RuntimeError::new("mistralrs HTTP response missing status line"))?;
    let status_code = status
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| RuntimeError::new("mistralrs HTTP response had invalid status line"))?;

    let body = &response[header_end + 4..];
    let body = if headers.lines().any(|line| {
        line.to_ascii_lowercase()
            .contains("transfer-encoding: chunked")
    }) {
        decode_chunked_body(body)?
    } else {
        body.to_vec()
    };
    let body = String::from_utf8(body).map_err(|error| {
        RuntimeError::new(format!("mistralrs HTTP response was not UTF-8: {error}"))
    })?;

    if !(200..300).contains(&status_code) {
        return Err(RuntimeError::new(format!(
            "mistralrs HTTP runtime returned status {status_code}: {}",
            body.trim()
        )));
    }

    Ok(body)
}

fn find_header_end(response: &[u8]) -> Option<usize> {
    response.windows(4).position(|window| window == b"\r\n\r\n")
}

fn decode_chunked_body(body: &[u8]) -> Result<Vec<u8>, RuntimeError> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < body.len() {
        let Some(line_end) = find_crlf(&body[index..]) else {
            return Err(RuntimeError::new(
                "chunked HTTP body had incomplete chunk size",
            ));
        };
        let size_line = String::from_utf8_lossy(&body[index..index + line_end]);
        let size = usize::from_str_radix(size_line.trim(), 16)
            .map_err(|_| RuntimeError::new("chunked HTTP body had invalid chunk size"))?;
        index += line_end + 2;
        if size == 0 {
            return Ok(out);
        }
        if index + size + 2 > body.len() {
            return Err(RuntimeError::new(
                "chunked HTTP body had incomplete chunk data",
            ));
        }
        out.extend_from_slice(&body[index..index + size]);
        index += size + 2;
    }
    Err(RuntimeError::new("chunked HTTP body did not terminate"))
}

fn find_crlf(bytes: &[u8]) -> Option<usize> {
    bytes.windows(2).position(|window| window == b"\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_parse_accepts_local_http_url() {
        let endpoint = HttpEndpoint::parse("http://127.0.0.1:8686").unwrap();

        assert_eq!(
            endpoint,
            HttpEndpoint {
                host: "127.0.0.1".to_owned(),
                port: 8686,
                base_path: String::new(),
            }
        );
    }

    #[test]
    fn endpoint_parse_keeps_base_path() {
        let endpoint = HttpEndpoint::parse("127.0.0.1:8686/proxy").unwrap();

        assert_eq!(
            endpoint.request_path("/v1/chat/completions"),
            "/proxy/v1/chat/completions"
        );
    }

    #[test]
    fn http_response_parser_decodes_chunked_body() {
        let raw = b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n4\r\nrust\r\n7\r\n-norion\r\n0\r\n\r\n";

        assert_eq!(parse_http_response(raw).unwrap(), "rust-norion");
    }

    #[test]
    fn streaming_chunked_decoder_emits_each_decoded_chunk() {
        let initial = b"4\r\nrust\r\n7\r\n-norion\r\n0\r\n\r\n".to_vec();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut sink = [0_u8; 1];
            let _ = stream.read(&mut sink);
        });
        let mut stream = TcpStream::connect(addr).unwrap();
        let mut chunks = Vec::new();
        let mut read_budget = HttpStreamReadBudget::new(HttpStreamTimeouts::default());

        let body = stream_chunked_body(initial, &mut stream, &mut read_budget, &mut |chunk| {
            chunks.push(String::from_utf8(chunk.to_vec()).unwrap());
            Ok(())
        })
        .unwrap();

        assert_eq!(String::from_utf8(body).unwrap(), "rust-norion");
        assert_eq!(chunks, vec!["rust".to_owned(), "-norion".to_owned()]);
        drop(stream);
        handle.join().unwrap();
    }

    #[test]
    fn post_json_stream_reports_idle_timeout_after_partial_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut sink = [0_u8; 1024];
            let _ = stream.read(&mut sink);
            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/event-stream\r\n",
                "Connection: close\r\n",
                "\r\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"rust\"}}]}\n\n"
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
            std::thread::sleep(Duration::from_millis(120));
        });
        let endpoint = HttpEndpoint::parse(&format!("http://{addr}")).unwrap();
        let mut chunks = Vec::new();

        let error = endpoint
            .post_json_stream(
                "/v1/chat/completions",
                "{}",
                HttpStreamTimeouts::new(Some(1_000), Some(20)),
                &mut |chunk| {
                    chunks.push(String::from_utf8(chunk.to_vec()).unwrap());
                    Ok(())
                },
            )
            .unwrap_err();

        assert!(error.message().contains("stream idle timeout after 20 ms"));
        assert!(error.message().contains("response body"));
        assert!(error.message().contains("emitted"));
        assert!(chunks.iter().any(|chunk| chunk.contains("\"rust\"")));
        handle.join().unwrap();
    }
}
