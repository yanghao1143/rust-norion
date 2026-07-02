use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Instant;

use super::StreamProvider;
use super::cleanup_audit::{
    ExperienceCleanupAuditParts, cleanup_audit_endpoint_missing,
    experience_cleanup_audit_response_summary, experience_cleanup_audit_summary,
};
use super::config::ProviderConfig;
use super::event::StreamEvent;
use super::health::{ProviderHealth, parse_provider_health};
use super::hygiene::{experience_hygiene_quarantine_summary, experience_hygiene_report_summary};
use super::model_pool::{
    model_pool_call_request_body, model_pool_call_summary, model_pool_manifest_summary,
    model_pool_route_request_body, model_pool_route_selection, model_pool_route_summary,
    model_pool_status_summary, model_pool_worker_answer_summary,
    model_pool_worker_chat_request_body,
};
use super::repair::experience_repair_summary;
use super::request::StreamRequest;
use super::retrieval::{experience_retrieval_request_body, experience_retrieval_summary};
use super::sse::drain_events;

#[derive(Debug, Clone)]
pub struct ForgeProvider {
    config: ProviderConfig,
}

impl ForgeProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }

    pub fn health(&self) -> Result<ProviderHealth, String> {
        let body = self.request_json_string("GET", "/health", None)?;
        Ok(parse_provider_health(&body))
    }

    pub fn experience_hygiene(&self) -> Result<String, String> {
        let body = self.request_json_string("GET", "/v1/experience-hygiene", None)?;
        experience_hygiene_report_summary(&body)
    }

    pub fn experience_hygiene_quarantine_dry_run(&self, limit: usize) -> Result<String, String> {
        let body = format!("{{\"apply\":false,\"limit\":{}}}", limit.max(1));
        let body =
            self.request_json_string("POST", "/v1/experience-hygiene/quarantine", Some(&body))?;
        let mut summary = experience_hygiene_quarantine_summary(&body)?;
        summary.push_str("\napply=false");
        summary.push_str("\napply_note=Use rust-norion --experience-hygiene-apply or POST apply=true only after explicit confirmation.");
        Ok(summary)
    }

    pub fn experience_repair_dry_run(&self, limit: usize) -> Result<String, String> {
        let body = format!("{{\"apply\":false,\"limit\":{}}}", limit.max(1));
        let body = self.request_json_string("POST", "/v1/experience-repair", Some(&body))?;
        let mut summary = experience_repair_summary(&body)?;
        summary.push_str("\napply=false");
        summary.push_str("\napply_note=Use rust-norion --experience-repair-apply or POST apply=true only after explicit confirmation.");
        Ok(summary)
    }

    pub fn experience_cleanup_audit(&self, limit: usize) -> Result<String, String> {
        let limit = limit.max(1);
        match self.experience_cleanup_audit_endpoint(limit) {
            Ok(summary) => return Ok(summary),
            Err(error) if cleanup_audit_endpoint_missing(&error) => {}
            Err(error) => return Err(error),
        }
        self.experience_cleanup_audit_legacy(limit)
    }

    fn experience_cleanup_audit_endpoint(&self, limit: usize) -> Result<String, String> {
        let body = format!("{{\"limit\":{limit}}}");
        let body = self.request_json_string("POST", "/v1/experience-cleanup-audit", Some(&body))?;
        experience_cleanup_audit_response_summary(&body)
    }

    fn experience_cleanup_audit_legacy(&self, limit: usize) -> Result<String, String> {
        let hygiene = self.experience_hygiene()?;
        let quarantine = self.experience_hygiene_quarantine_dry_run(limit)?;
        let repair = self.experience_repair_dry_run(limit)?;
        Ok(experience_cleanup_audit_summary(
            ExperienceCleanupAuditParts {
                limit,
                hygiene,
                quarantine,
                repair,
            },
        ))
    }

    pub fn experience_retrieval(
        &self,
        prompt: &str,
        profile: &str,
        limit: usize,
    ) -> Result<String, String> {
        self.experience_retrieval_with_index_context(prompt, profile, limit, None)
    }

    pub fn experience_retrieval_with_index_context(
        &self,
        prompt: &str,
        profile: &str,
        limit: usize,
        index_context: Option<&str>,
    ) -> Result<String, String> {
        let body = experience_retrieval_request_body(prompt, profile, limit, index_context);
        let body = self.request_json_string("POST", "/v1/experience-retrieval", Some(&body))?;
        experience_retrieval_summary(&body)
    }

    pub fn model_pool_status(&self) -> Result<String, String> {
        let body = self.request_json_string("GET", "/v1/model-pool/status", None)?;
        model_pool_status_summary(&body)
    }

    pub fn model_pool_manifest(&self) -> Result<String, String> {
        let body = self.request_json_string("GET", "/v1/model-pool/manifest", None)?;
        model_pool_manifest_summary(&body)
    }

    pub fn model_pool_route(&self, task_kind: &str) -> Result<String, String> {
        self.model_pool_route_with_max_tokens(task_kind, None)
    }

    pub fn model_pool_route_with_max_tokens(
        &self,
        task_kind: &str,
        max_tokens: Option<usize>,
    ) -> Result<String, String> {
        let body = model_pool_route_request_body(task_kind, max_tokens);
        let body = self.request_json_string("POST", "/v1/model-pool/route-plan", Some(&body))?;
        model_pool_route_summary(&body)
    }

    pub fn model_pool_call(&self, task_kind: &str, prompt: &str) -> Result<String, String> {
        self.model_pool_call_with_max_tokens(task_kind, prompt, None)
    }

    pub fn model_pool_call_with_max_tokens(
        &self,
        task_kind: &str,
        prompt: &str,
        max_tokens: Option<usize>,
    ) -> Result<String, String> {
        if prompt.trim().is_empty() {
            return Err("model pool call requires a non-empty prompt".to_owned());
        }
        match self.model_pool_call_via_backend(task_kind, prompt, max_tokens) {
            Ok(summary) => return Ok(summary),
            Err(error) if model_pool_call_endpoint_missing(&error) => {}
            Err(error) => return Err(error),
        }
        self.model_pool_call_direct_worker(task_kind, prompt, max_tokens)
    }

    fn model_pool_call_via_backend(
        &self,
        task_kind: &str,
        prompt: &str,
        max_tokens: Option<usize>,
    ) -> Result<String, String> {
        let body = model_pool_call_request_body(task_kind, prompt, max_tokens);
        let body = self.request_json_string_with_response_timeout(
            "POST",
            "/v1/model-pool/call",
            Some(&body),
            self.config.request_timeout,
        )?;
        model_pool_call_summary(&body)
    }

    fn model_pool_call_direct_worker(
        &self,
        task_kind: &str,
        prompt: &str,
        max_tokens: Option<usize>,
    ) -> Result<String, String> {
        let route_body = model_pool_route_request_body(task_kind, max_tokens);
        let route_body =
            self.request_json_string("POST", "/v1/model-pool/route-plan", Some(&route_body))?;
        let route = model_pool_route_selection(&route_body)?;
        let request_body = model_pool_worker_chat_request_body(
            prompt,
            route.effective_max_tokens.or(route.default_max_tokens),
        );
        let response_body = self.request_json_string_to_base_url(
            &route.base_url,
            "POST",
            "/v1/chat/completions",
            Some(&request_body),
        )?;
        model_pool_worker_answer_summary(&route, &response_body)
    }

    fn connect(&self) -> Result<TcpStream, String> {
        self.connect_with_read_timeout(self.config.read_timeout)
    }

    fn connect_with_read_timeout(
        &self,
        read_timeout: std::time::Duration,
    ) -> Result<TcpStream, String> {
        let address = self
            .config
            .backend
            .to_socket_addrs()
            .map_err(|error| {
                format!(
                    "无法解析后端地址 (resolve backend failed) backend={}: {error}",
                    self.config.backend
                )
            })?
            .next()
            .ok_or_else(|| {
                format!(
                    "后端地址没有解析结果 (backend address did not resolve) backend={}",
                    self.config.backend
                )
            })?;
        let stream = TcpStream::connect_timeout(&address, self.config.connect_timeout)
            .map_err(|error| {
                format!(
                    "无法连接 rust-norion 后端 (connect backend failed) backend={} health={}/health: {error}",
                    self.config.backend,
                    backend_base_url(&self.config.backend)
                )
            })?;
        stream
            .set_read_timeout(Some(read_timeout))
            .map_err(|error| format!("set backend read timeout failed: {error}"))?;
        stream
            .set_write_timeout(Some(self.config.read_timeout))
            .map_err(|error| format!("set backend write timeout failed: {error}"))?;
        Ok(stream)
    }

    fn request_json_string(
        &self,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<String, String> {
        self.request_json_string_with_response_timeout(method, path, body, self.config.read_timeout)
    }

    fn request_json_string_with_response_timeout(
        &self,
        method: &str,
        path: &str,
        body: Option<&str>,
        response_timeout: std::time::Duration,
    ) -> Result<String, String> {
        let mut stream = self.connect_with_read_timeout(response_timeout)?;
        let body = body.unwrap_or("");
        let http_request = if method.eq_ignore_ascii_case("GET") {
            format!(
                "GET {path} HTTP/1.1\r\nhost: {}\r\nconnection: close\r\n\r\n",
                self.config.backend
            )
        } else {
            format!(
                "{method} {path} HTTP/1.1\r\nhost: {}\r\ncontent-type: application/json; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                self.config.backend,
                body.len()
            )
        };
        stream
            .write_all(http_request.as_bytes())
            .map_err(|error| format!("write backend {path} request failed: {error}"))?;
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|error| format!("read backend {path} response failed: {error}"))?;
        let (header_end, header_boundary_len) = find_header_boundary(&response)
            .ok_or_else(|| format!("backend {path} response missing HTTP headers"))?;
        let headers = String::from_utf8_lossy(&response[..header_end]);
        let status = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|status| status.parse::<u16>().ok())
            .unwrap_or(0);
        let body = response
            .get(header_end + header_boundary_len..)
            .unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(format!(
                "后端 {path} 返回 HTTP {status} (backend {path} returned HTTP {status}): {}",
                String::from_utf8_lossy(body).trim()
            ));
        }
        std::str::from_utf8(body)
            .map(|body| body.to_owned())
            .map_err(|error| format!("backend {path} body was not UTF-8: {error}"))
    }

    fn request_json_string_to_base_url(
        &self,
        base_url: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<String, String> {
        let endpoint = HttpBaseEndpoint::parse(base_url)?;
        let mut stream = endpoint.connect(self.config.connect_timeout)?;
        stream
            .set_read_timeout(Some(self.config.request_timeout))
            .map_err(|error| format!("set worker read timeout failed: {error}"))?;
        stream
            .set_write_timeout(Some(self.config.read_timeout))
            .map_err(|error| format!("set worker write timeout failed: {error}"))?;
        let body = body.unwrap_or("");
        let request_path = endpoint.request_path(path);
        let http_request = if method.eq_ignore_ascii_case("GET") {
            format!(
                "GET {request_path} HTTP/1.1\r\nhost: {}\r\naccept: application/json\r\nconnection: close\r\n\r\n",
                endpoint.authority
            )
        } else {
            format!(
                "{method} {request_path} HTTP/1.1\r\nhost: {}\r\ncontent-type: application/json; charset=utf-8\r\naccept: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                endpoint.authority,
                body.len()
            )
        };
        stream
            .write_all(http_request.as_bytes())
            .map_err(|error| format!("write worker {request_path} request failed: {error}"))?;
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|error| format!("read worker {request_path} response failed: {error}"))?;
        parse_json_http_response(&response, "worker", &request_path)
    }
}

impl Default for ForgeProvider {
    fn default() -> Self {
        Self::new(ProviderConfig::default())
    }
}

impl StreamProvider for ForgeProvider {
    fn stream(
        &self,
        request: &StreamRequest,
        on_event: &mut dyn FnMut(StreamEvent) -> Result<(), String>,
    ) -> Result<(), String> {
        let body = request.body_json();
        let path = request.endpoint.stream_path();
        let mut stream = self.connect()?;
        let http_request = format!(
            "POST {path} HTTP/1.1\r\nhost: {}\r\ncontent-type: application/json; charset=utf-8\r\naccept: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            self.config.backend,
            body.len()
        );
        stream
            .write_all(http_request.as_bytes())
            .map_err(|error| format!("write backend stream request failed: {error}"))?;

        read_event_stream(&mut stream, self.config.request_timeout, on_event)
    }
}

fn backend_base_url(backend: &str) -> String {
    let trimmed = backend.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_owned()
    } else {
        format!("http://{trimmed}")
    }
}

fn model_pool_call_endpoint_missing(error: &str) -> bool {
    error.contains("HTTP 404")
        || (error.contains("unsupported HTTP path") && error.contains("model-pool/call"))
}

fn read_event_stream(
    stream: &mut TcpStream,
    request_timeout: std::time::Duration,
    on_event: &mut dyn FnMut(StreamEvent) -> Result<(), String>,
) -> Result<(), String> {
    let started = Instant::now();
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let (header_end, header_boundary_len) = loop {
        if started.elapsed() > request_timeout {
            return Err(format!(
                "backend stream timed out after {request_timeout:?}"
            ));
        }
        set_stream_read_timeout(stream, started, request_timeout)
            .map_err(|error| format!("set backend stream header read timeout failed: {error}"))?;
        match stream.read(&mut chunk) {
            Ok(0) => return Err("backend stream closed before headers".to_owned()),
            Ok(read) => {
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(header_boundary) = find_header_boundary(&buffer) {
                    break header_boundary;
                }
            }
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                on_event(StreamEvent::Heartbeat(format!(
                    "waiting for backend headers for {}s",
                    started.elapsed().as_secs()
                )))?;
            }
            Err(error) => return Err(format!("read backend stream headers failed: {error}")),
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
        read_stream_error_body(stream, &mut body, content_length, started, request_timeout)
            .map_err(|error| format!("read backend stream error body failed: {error}"))?;
        return Err(format!(
            "backend stream returned HTTP {status}: {}",
            String::from_utf8_lossy(&body).trim()
        ));
    }

    let mut saw_terminal_event = false;
    let mut saw_any_event = false;
    for event in drain_events(&mut body)? {
        saw_any_event = true;
        relay_stream_event(event, &mut saw_terminal_event, on_event)?;
        if saw_terminal_event {
            return Ok(());
        }
    }

    loop {
        if started.elapsed() > request_timeout {
            return Err(format!(
                "backend stream timed out after {request_timeout:?}"
            ));
        }
        set_stream_read_timeout(stream, started, request_timeout)
            .map_err(|error| format!("set backend stream body read timeout failed: {error}"))?;
        match stream.read(&mut chunk) {
            Ok(0) => {
                if !body.is_empty() {
                    return Err(format!(
                        "backend stream truncated: connection closed with {} byte(s) of incomplete SSE data",
                        body.len()
                    ));
                }
                if saw_terminal_event {
                    return Ok(());
                }
                if saw_any_event {
                    return Err(
                        "backend stream truncated: connection closed before done event after receiving partial events".to_owned(),
                    );
                }
                return Err(
                    "backend stream truncated: connection closed before done event".to_owned(),
                );
            }
            Ok(read) => {
                body.extend_from_slice(&chunk[..read]);
                for event in drain_events(&mut body)? {
                    saw_any_event = true;
                    relay_stream_event(event, &mut saw_terminal_event, on_event)?;
                    if saw_terminal_event {
                        return Ok(());
                    }
                }
            }
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                on_event(StreamEvent::Heartbeat(format!(
                    "waiting for Gemma stream for {}s",
                    started.elapsed().as_secs()
                )))?;
            }
            Err(error) => return Err(format!("read backend stream body failed: {error}")),
        }
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

fn set_stream_read_timeout(
    stream: &TcpStream,
    started: Instant,
    request_timeout: std::time::Duration,
) -> Result<(), std::io::Error> {
    let remaining = request_timeout
        .saturating_sub(started.elapsed())
        .max(std::time::Duration::from_millis(1));
    let timeout = match stream.read_timeout()? {
        Some(current) => current.min(remaining),
        None => remaining,
    };
    stream.set_read_timeout(Some(timeout))
}

fn read_stream_error_body(
    stream: &mut TcpStream,
    body: &mut Vec<u8>,
    content_length: Option<usize>,
    started: Instant,
    request_timeout: std::time::Duration,
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
        if started.elapsed() > request_timeout {
            return Ok(());
        }
        set_stream_read_timeout(stream, started, request_timeout)?;
        let remaining = content_length - body.len();
        match stream.read(&mut chunk[..remaining.min(4096)]) {
            Ok(0) => return Ok(()),
            Ok(read) => body.extend_from_slice(&chunk[..read]),
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

struct HttpBaseEndpoint {
    authority: String,
    base_path: String,
}

impl HttpBaseEndpoint {
    fn parse(base_url: &str) -> Result<Self, String> {
        let normalized = backend_base_url(base_url);
        let without_scheme = normalized
            .strip_prefix("http://")
            .ok_or_else(|| "model pool worker calls require http:// endpoints".to_owned())?;
        let (authority, base_path) = without_scheme
            .split_once('/')
            .map(|(authority, path)| (authority.to_owned(), format!("/{path}")))
            .unwrap_or_else(|| (without_scheme.to_owned(), String::new()));
        if authority.trim().is_empty() {
            return Err("model pool worker endpoint missing authority".to_owned());
        }
        Ok(Self {
            authority,
            base_path,
        })
    }

    fn connect(&self, timeout: std::time::Duration) -> Result<TcpStream, String> {
        let address = self
            .authority
            .to_socket_addrs()
            .map_err(|error| format!("resolve worker {} failed: {error}", self.authority))?
            .next()
            .ok_or_else(|| format!("resolve worker {} returned no address", self.authority))?;
        TcpStream::connect_timeout(&address, timeout)
            .map_err(|error| format!("connect worker {} failed: {error}", self.authority))
    }

    fn request_path(&self, path: &str) -> String {
        let path = if path.starts_with('/') {
            path.to_owned()
        } else {
            format!("/{path}")
        };
        let base_path = self.base_path.trim_end_matches('/');
        if base_path.is_empty() {
            path
        } else if base_path == "/v1" && path.starts_with("/v1/") {
            path
        } else {
            format!("{base_path}{path}")
        }
    }
}

fn parse_json_http_response(response: &[u8], label: &str, path: &str) -> Result<String, String> {
    let (header_end, header_boundary_len) = find_header_boundary(response)
        .ok_or_else(|| format!("{label} {path} response missing HTTP headers"))?;
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .unwrap_or(0);
    let body = response
        .get(header_end + header_boundary_len..)
        .unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(format!(
            "{label} {path} returned HTTP {status}: {}",
            String::from_utf8_lossy(body).trim()
        ));
    }
    std::str::from_utf8(body)
        .map(|body| body.to_owned())
        .map_err(|error| format!("{label} {path} body was not UTF-8: {error}"))
}

fn relay_stream_event(
    event: StreamEvent,
    saw_terminal_event: &mut bool,
    on_event: &mut dyn FnMut(StreamEvent) -> Result<(), String>,
) -> Result<(), String> {
    if event.is_terminal() {
        *saw_terminal_event = true;
    }
    on_event(event)
}

fn find_header_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
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
    use std::io::{ErrorKind, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn cleanup_audit_prefers_combined_backend_endpoint() {
        let (backend, server) = spawn_http_server(1, |request| {
            assert!(request.starts_with("POST /v1/experience-cleanup-audit "));
            assert!(request.contains("\"limit\":7"));
            (200, "OK", cleanup_audit_body())
        });
        let provider = ForgeProvider::new(test_config(backend));

        let summary = provider.experience_cleanup_audit(7).unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["POST /v1/experience-cleanup-audit HTTP/1.1"]);
        assert!(summary.contains("Noiron experience cleanup audit"));
        assert!(summary.contains("sample_limit=7"));
        assert!(summary.contains("writes_experience_state=false"));
        assert!(summary.contains("repairable_legacy_metadata_lessons=1"));
        assert!(summary.contains("index_noisy_records=1"));
        assert!(summary.contains("repairable_index_records=1"));
        assert!(summary.contains("index_retrieval_ready=true"));
    }

    #[test]
    fn cleanup_audit_falls_back_to_legacy_read_only_endpoints() {
        let (backend, server) = spawn_http_server(4, |request| {
            if request.starts_with("POST /v1/experience-cleanup-audit ") {
                return (
                    404,
                    "Not Found",
                    "{\"ok\":false,\"error\":\"unsupported HTTP path: /v1/experience-cleanup-audit\"}".to_owned(),
                );
            }
            if request.starts_with("GET /v1/experience-hygiene ") {
                return (200, "OK", hygiene_body());
            }
            if request.starts_with("POST /v1/experience-hygiene/quarantine ") {
                assert!(request.contains("\"apply\":false"));
                assert!(request.contains("\"limit\":5"));
                return (200, "OK", quarantine_body());
            }
            if request.starts_with("POST /v1/experience-repair ") {
                assert!(request.contains("\"apply\":false"));
                assert!(request.contains("\"limit\":5"));
                return (200, "OK", repair_body());
            }
            (500, "Internal Server Error", "{\"ok\":false}".to_owned())
        });
        let provider = ForgeProvider::new(test_config(backend));

        let summary = provider.experience_cleanup_audit(5).unwrap();
        let requests = server.join().unwrap();

        assert_eq!(
            requests,
            vec![
                "POST /v1/experience-cleanup-audit HTTP/1.1",
                "GET /v1/experience-hygiene HTTP/1.1",
                "POST /v1/experience-hygiene/quarantine HTTP/1.1",
                "POST /v1/experience-repair HTTP/1.1",
            ]
        );
        assert!(summary.contains("sample_limit=5"));
        assert!(summary.contains("## Hygiene"));
        assert!(summary.contains("## Quarantine dry-run"));
        assert!(summary.contains("## Repair dry-run"));
    }

    #[test]
    fn cleanup_audit_does_not_fallback_when_combined_endpoint_is_not_read_only() {
        let (backend, server) = spawn_http_server(1, |request| {
            assert!(request.starts_with("POST /v1/experience-cleanup-audit "));
            (
                200,
                "OK",
                cleanup_audit_body().replace(
                    "\"writes_experience_state\":false",
                    "\"writes_experience_state\":true",
                ),
            )
        });
        let provider = ForgeProvider::new(test_config(backend));

        let error = provider.experience_cleanup_audit(7).unwrap_err();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["POST /v1/experience-cleanup-audit HTTP/1.1"]);
        assert!(error.contains("rejected non-read-only response"));
    }

    #[test]
    fn experience_retrieval_surfaces_runtime_match_diagnostics() {
        let (backend, server) = spawn_recording_http_server(1, |request| {
            assert_no_model_stream_request(request);
            assert_eq!(
                request_line(request),
                "POST /v1/experience-retrieval HTTP/1.1"
            );
            assert!(request.contains("\"prompt\":\"model pool route code\""));
            assert!(request.contains("\"profile\":\"coding\""));
            assert!(request.contains("\"limit\":3"));
            assert!(
                request.contains("\"index_context\":\"model_pool_index:\\nsrc/model_service\"")
            );
            (200, "OK", retrieval_body())
        });
        let provider = ForgeProvider::new(test_config(backend));

        let summary = provider
            .experience_retrieval_with_index_context(
                "model pool route code",
                "coding",
                3,
                Some(" model_pool_index:\nsrc/model_service "),
            )
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["POST /v1/experience-retrieval HTTP/1.1"]);
        assert_no_model_stream_request_lines(&requests);
        assert!(summary.contains("Noiron experience retrieval preview"));
        assert!(summary.contains("index_context_used=true"));
        assert!(summary.contains("runtime_model=gemma-3-12b"));
        assert!(summary.contains("runtime_adapter=llama.cpp"));
        assert!(summary.contains("runtime_device=metal"));
        assert!(summary.contains("runtime_primary_lane=quality"));
        assert!(summary.contains("runtime_kv_influence=0.61"));
        assert!(summary.contains("runtime_uncertainty_perplexity=1.25"));
        assert!(summary.contains("stored_runtime_kv_memory_ids=11,13"));
    }

    #[test]
    fn model_pool_status_uses_only_status_endpoint() {
        let (backend, server) = spawn_recording_http_server(1, |request| {
            assert_no_model_stream_request(request);
            assert_eq!(request_line(request), "GET /v1/model-pool/status HTTP/1.1");
            (200, "OK", model_pool_status_body())
        });
        let provider = ForgeProvider::new(test_config(backend));

        let summary = provider.model_pool_status().unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["GET /v1/model-pool/status HTTP/1.1"]);
        assert_no_model_stream_request_lines(&requests);
        assert!(summary.contains("SmartSteam model pool status"));
        assert!(summary.contains("worker_count=1"));
    }

    #[test]
    fn model_pool_route_posts_review_route_plan_only() {
        let (backend, server) = spawn_recording_http_server(1, |request| {
            assert_no_model_stream_request(request);
            assert_eq!(
                request_line(request),
                "POST /v1/model-pool/route-plan HTTP/1.1"
            );
            assert!(request.contains("\"task_kind\":\"review\""));
            (200, "OK", model_pool_route_body())
        });
        let provider = ForgeProvider::new(test_config(backend));

        let summary = provider.model_pool_route("review").unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["POST /v1/model-pool/route-plan HTTP/1.1"]);
        assert_no_model_stream_request_lines(&requests);
        assert!(summary.contains("SmartSteam model pool route plan"));
        assert!(summary.contains("task_kind=review"));
    }

    #[test]
    fn model_pool_route_with_max_tokens_posts_budget() {
        let (backend, server) = spawn_recording_http_server(1, |request| {
            assert_no_model_stream_request(request);
            assert_eq!(
                request_line(request),
                "POST /v1/model-pool/route-plan HTTP/1.1"
            );
            assert!(request.contains("\"task_kind\":\"quality\""));
            assert!(request.contains("\"max_tokens\":262144"));
            (
                200,
                "OK",
                "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"quality\",\"route_allowed\":true,\"reason\":\"ready\",\"role_candidates\":[\"quality\"],\"selected_role\":\"quality\",\"selected_base_url\":\"http://127.0.0.1:8686\",\"selected_port\":8686,\"selected_default_max_tokens\":262144,\"configured_max_tokens\":262144,\"effective_max_tokens\":262144,\"max_tokens_clamped\":false,\"max_tokens_clamp_reason\":\"quality_worker_request_budget_preserved\",\"candidate_workers\":[]}".to_owned(),
            )
        });
        let provider = ForgeProvider::new(test_config(backend));

        let summary = provider
            .model_pool_route_with_max_tokens("quality", Some(262_144))
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["POST /v1/model-pool/route-plan HTTP/1.1"]);
        assert!(summary.contains("configured_max_tokens=262144"));
        assert!(summary.contains("effective_max_tokens=262144"));
    }

    #[test]
    fn model_pool_call_prefers_backend_call_endpoint() {
        let (backend, backend_server) = spawn_recording_http_server(1, |request| {
            assert_no_model_stream_request(request);
            assert_eq!(request_line(request), "POST /v1/model-pool/call HTTP/1.1");
            assert!(request.contains("\"task_kind\":\"summary\""));
            assert!(request.contains("\"prompt\":\"summarize this log\""));
            assert!(request.contains("\"max_tokens\":4096"));
            (
                200,
                "OK",
                "{\"ok\":true,\"read_only\":false,\"launches_process\":false,\"sends_prompt\":true,\"task_kind\":\"summary\",\"selected_role\":\"summary\",\"selected_base_url\":\"http://127.0.0.1:8687\",\"configured_max_tokens\":4096,\"effective_max_tokens\":768,\"max_tokens_clamped\":true,\"answer\":\"short summary\"}".to_owned(),
            )
        });
        let provider = ForgeProvider::new(test_config(backend));

        let summary = provider
            .model_pool_call_with_max_tokens("summary", "summarize this log", Some(4096))
            .unwrap();
        let backend_requests = backend_server.join().unwrap();

        assert_eq!(backend_requests, vec!["POST /v1/model-pool/call HTTP/1.1"]);
        assert!(summary.contains("SmartSteam model pool call"));
        assert!(summary.contains("selected_role=summary"));
        assert!(summary.contains("configured_max_tokens=4096"));
        assert!(summary.contains("effective_max_tokens=768"));
        assert!(summary.contains("max_tokens_clamped=true"));
        assert!(summary.contains("answer=short summary"));
    }

    #[test]
    fn model_pool_call_uses_request_timeout_not_read_poll_interval() {
        let read_timeout = Duration::from_millis(20);
        let (backend, backend_server) = spawn_recording_http_server(1, move |request| {
            assert_no_model_stream_request(request);
            assert_eq!(request_line(request), "POST /v1/model-pool/call HTTP/1.1");
            thread::sleep(read_timeout + Duration::from_millis(100));
            (
                200,
                "OK",
                "{\"ok\":true,\"read_only\":false,\"launches_process\":false,\"sends_prompt\":true,\"task_kind\":\"summary\",\"selected_role\":\"summary\",\"selected_base_url\":\"http://127.0.0.1:8687\",\"answer\":\"slow summary\"}".to_owned(),
            )
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout,
            request_timeout: Duration::from_secs(2),
        });

        let summary = provider
            .model_pool_call("summary", "summarize this log")
            .unwrap();
        let backend_requests = backend_server.join().unwrap();

        assert_eq!(backend_requests, vec!["POST /v1/model-pool/call HTTP/1.1"]);
        assert!(summary.contains("answer=slow summary"));
    }

    #[test]
    fn backend_connections_set_read_and_write_timeouts() {
        let read_timeout = Duration::from_millis(750);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let _connection = listener.accept().unwrap();
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout,
            request_timeout: Duration::from_secs(5),
        });

        let stream = provider.connect().unwrap();

        assert_eq!(stream.read_timeout().unwrap(), Some(read_timeout));
        assert_eq!(stream.write_timeout().unwrap(), Some(read_timeout));
        drop(stream);
        server.join().unwrap();
    }

    #[test]
    fn model_pool_call_falls_back_for_legacy_backend_endpoint() {
        let (worker, worker_server) = spawn_recording_http_server(1, |request| {
            assert_eq!(request_line(request), "POST /v1/chat/completions HTTP/1.1");
            assert!(request.contains("\"model\":\"smartsteam-pool-worker\""));
            assert!(request.contains("\"content\":\"summarize this log\""));
            assert!(request.contains("\"max_tokens\":768"));
            (
                200,
                "OK",
                "{\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"short summary\"}}]}".to_owned(),
            )
        });
        let route_body =
            model_pool_route_body_for_worker("summary", &format!("http://{worker}"), 768);
        let (backend, backend_server) = spawn_recording_http_server(2, move |request| {
            assert_no_model_stream_request(request);
            match request_line(request) {
                "POST /v1/model-pool/call HTTP/1.1" => (
                    404,
                    "Not Found",
                    "{\"ok\":false,\"error\":\"unsupported HTTP path: /v1/model-pool/call\"}"
                        .to_owned(),
                ),
                "POST /v1/model-pool/route-plan HTTP/1.1" => {
                    assert!(request.contains("\"task_kind\":\"summary\""));
                    (200, "OK", route_body.clone())
                }
                line => panic!("unexpected request line: {line}"),
            }
        });
        let provider = ForgeProvider::new(test_config(backend));

        let summary = provider
            .model_pool_call("summary", "summarize this log")
            .unwrap();
        let backend_requests = backend_server.join().unwrap();
        let worker_requests = worker_server.join().unwrap();

        assert_eq!(
            backend_requests,
            vec![
                "POST /v1/model-pool/call HTTP/1.1",
                "POST /v1/model-pool/route-plan HTTP/1.1"
            ]
        );
        assert_eq!(worker_requests, vec!["POST /v1/chat/completions HTTP/1.1"]);
        assert!(summary.contains("SmartSteam model pool call"));
        assert!(summary.contains("selected_role=summary"));
        assert!(summary.contains("answer=short summary"));
    }

    #[test]
    fn model_pool_call_stops_when_route_is_blocked() {
        let (backend, server) = spawn_recording_http_server(1, |request| {
            assert_no_model_stream_request(request);
            assert_eq!(request_line(request), "POST /v1/model-pool/call HTTP/1.1");
            (
                409,
                "Conflict",
                "{\"ok\":false,\"read_only\":false,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"review\",\"route_allowed\":false,\"reason\":\"quality_worker_down\"}".to_owned(),
            )
        });
        let provider = ForgeProvider::new(test_config(backend));

        let error = provider
            .model_pool_call("review", "review this patch")
            .unwrap_err();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["POST /v1/model-pool/call HTTP/1.1"]);
        assert!(error.contains("quality_worker_down"));
    }

    #[test]
    fn worker_endpoint_does_not_duplicate_v1_prefix() {
        let endpoint = HttpBaseEndpoint::parse("http://127.0.0.1:8687/v1").unwrap();

        assert_eq!(
            endpoint.request_path("/v1/chat/completions"),
            "/v1/chat/completions"
        );
        assert_eq!(endpoint.request_path("/models"), "/v1/models");
    }

    #[test]
    fn model_pool_status_rejects_unsafe_backend_contracts() {
        let cases = [
            ("read_only=false", false, false, false),
            ("launches_process=true", true, true, false),
            ("sends_prompt=true", true, false, true),
        ];

        for (label, read_only, launches_process, sends_prompt) in cases {
            let (backend, server) = spawn_http_server(1, move |request| {
                assert_no_model_stream_request(request);
                assert_eq!(request_line(request), "GET /v1/model-pool/status HTTP/1.1");
                (
                    200,
                    "OK",
                    model_pool_status_body_with_contract(read_only, launches_process, sends_prompt),
                )
            });
            let provider = ForgeProvider::new(test_config(backend));

            let error = provider.model_pool_status().unwrap_err();
            let requests = server.join().unwrap();

            assert_eq!(
                requests,
                vec!["GET /v1/model-pool/status HTTP/1.1"],
                "{label}"
            );
            assert_no_model_stream_request_lines(&requests);
            assert!(error.contains("failed safety contract"), "{label}: {error}");
            assert!(error.contains(label), "{label}: {error}");
        }
    }

    #[test]
    fn stream_rejects_partial_events_when_backend_closes_without_done() {
        let (backend, server) = spawn_http_server(1, |request| {
            assert!(request.starts_with("POST /v1/chat-stream "));
            (
                200,
                "OK",
                "event: delta\ndata: partial answer\n\n".to_owned(),
            )
        });
        let provider = ForgeProvider::new(test_config(backend));
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        let error = provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap_err();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["POST /v1/chat-stream HTTP/1.1"]);
        assert_eq!(
            events,
            vec![StreamEvent::Delta("partial answer".to_owned())]
        );
        assert!(error.contains("backend stream truncated"));
        assert!(error.contains("before done event"));
    }

    #[test]
    fn http_header_boundary_reports_separator_length() {
        assert_eq!(
            find_header_boundary(b"HTTP/1.1 200 OK\r\n\r\nevent: done\n\n"),
            Some((15, 4))
        );
        assert_eq!(
            find_header_boundary(b"HTTP/1.1 200 OK\n\nevent: done\n\n"),
            Some((15, 2))
        );
    }

    #[test]
    fn http_header_boundary_prefers_earliest_separator() {
        assert_eq!(find_header_boundary(b"a\n\nb\r\n\r\nc"), Some((1, 2)));
        assert_eq!(find_header_boundary(b"a\r\n\r\nb\n\nc"), Some((1, 4)));
    }

    #[test]
    fn json_response_parser_accepts_lf_only_http_headers() {
        let body = parse_json_http_response(
            b"HTTP/1.1 200 OK\ncontent-type: application/json\n\n{\"ok\":true}",
            "backend",
            "/health",
        )
        .unwrap();

        assert_eq!(body, "{\"ok\":true}");
    }

    #[test]
    fn json_response_parser_preserves_lf_only_http_error_body() {
        let error = parse_json_http_response(
            b"HTTP/1.1 500 Internal Server Error\ncontent-type: text/plain\n\nbackend failed",
            "backend",
            "/health",
        )
        .unwrap_err();

        assert!(error.contains("backend /health returned HTTP 500"));
        assert!(error.contains("backend failed"));
    }

    #[test]
    fn stream_succeeds_when_backend_sends_done() {
        let (backend, server) = spawn_http_server(1, |request| {
            assert!(request.starts_with("POST /v1/chat-stream "));
            (
                200,
                "OK",
                "event: delta\ndata: complete answer\n\nevent: done\ndata: [DONE]\n\n".to_owned(),
            )
        });
        let provider = ForgeProvider::new(test_config(backend));
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["POST /v1/chat-stream HTTP/1.1"]);
        assert_eq!(
            events,
            vec![
                StreamEvent::Delta("complete answer".to_owned()),
                StreamEvent::Done
            ]
        );
    }

    #[test]
    fn stream_accepts_lf_only_http_headers() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\ncontent-type: text/event-stream\n\nevent: done\ndata: [DONE]\n\n",
                )
                .unwrap();
        });
        let provider = ForgeProvider::new(test_config(backend));
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();
        server.join().unwrap();

        assert_eq!(events, vec![StreamEvent::Done]);
    }

    #[test]
    fn stream_returns_after_done_without_waiting_for_eof() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n\
event: done\ndata: [DONE]\n\n",
                )
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(200));
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_millis(20),
            request_timeout: Duration::from_millis(100),
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();
        server.join().unwrap();

        assert_eq!(events, vec![StreamEvent::Done]);
    }

    #[test]
    fn stream_ignores_frames_after_done_terminal_event() {
        let (backend, server) = spawn_http_server(1, |request| {
            assert!(request.starts_with("POST /v1/chat-stream "));
            (
                200,
                "OK",
                "event: done\ndata: [DONE]\n\nevent: delta\ndata: should-not-render\n\n".to_owned(),
            )
        });
        let provider = ForgeProvider::new(test_config(backend));
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["POST /v1/chat-stream HTTP/1.1"]);
        assert_eq!(events, vec![StreamEvent::Done]);
    }

    #[test]
    fn stream_returns_after_error_without_waiting_for_eof() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n\
event: error\ndata: backend failed\n\n",
                )
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(200));
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_millis(20),
            request_timeout: Duration::from_millis(100),
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();
        server.join().unwrap();

        assert_eq!(
            events,
            vec![StreamEvent::Error("backend failed".to_owned())]
        );
    }

    #[test]
    fn stream_returns_http_error_without_waiting_for_eof() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 14\r\n\r\nbackend failed",
                )
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(200));
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_millis(20),
            request_timeout: Duration::from_millis(100),
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        let error = provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap_err();
        server.join().unwrap();

        assert!(events.is_empty());
        assert!(error.contains("backend stream returned HTTP 500"));
        assert!(error.contains("backend failed"));
    }

    #[test]
    fn stream_returns_http_error_without_content_length_without_waiting_for_eof() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\ncontent-type: text/plain\r\n\r\nbackend failed",
                )
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(200));
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_millis(20),
            request_timeout: Duration::from_millis(100),
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        let error = provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap_err();
        server.join().unwrap();

        assert!(events.is_empty());
        assert!(error.contains("backend stream returned HTTP 500"));
    }

    #[test]
    fn stream_returns_partial_http_error_body_on_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 100\r\n\r\npartial failure",
                )
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(200));
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_millis(20),
            request_timeout: Duration::from_millis(100),
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        let error = provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap_err();
        server.join().unwrap();

        assert!(events.is_empty());
        assert!(error.contains("backend stream returned HTTP 500"));
        assert!(error.contains("partial failure"));
        assert!(!error.contains("read backend stream error body failed"));
    }

    #[test]
    fn stream_caps_http_error_body_wait_to_total_timeout() {
        let request_timeout = Duration::from_millis(80);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 100\r\n\r\npartial failure",
                )
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(300));
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_secs(2),
            request_timeout,
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();
        let started = Instant::now();

        let error = provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap_err();
        let elapsed = started.elapsed();
        server.join().unwrap();

        assert!(events.is_empty());
        assert!(error.contains("backend stream returned HTTP 500"));
        assert!(error.contains("partial failure"));
        assert!(
            elapsed < Duration::from_secs(1),
            "expected total timeout cap to return before the 2s socket timeout; elapsed={elapsed:?}"
        );
    }

    #[test]
    fn stream_truncates_buffered_http_error_body_to_content_length() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 14\r\n\r\nbackend failedextra bytes",
                )
                .unwrap();
        });
        let provider = ForgeProvider::new(test_config(backend));
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        let error = provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap_err();
        server.join().unwrap();

        assert!(events.is_empty());
        assert!(error.contains("backend stream returned HTTP 500"));
        assert!(error.contains("backend failed"));
        assert!(!error.contains("extra bytes"));
    }

    #[test]
    fn stream_heartbeats_while_waiting_for_slow_backend_headers() {
        let read_timeout = Duration::from_millis(25);
        let (backend, server) = spawn_http_server(1, move |request| {
            assert!(request.starts_with("POST /v1/chat-stream "));
            thread::sleep(read_timeout + Duration::from_millis(100));
            (200, "OK", "event: done\ndata: [DONE]\n\n".to_owned())
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout,
            request_timeout: Duration::from_secs(2),
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests, vec!["POST /v1/chat-stream HTTP/1.1"]);
        assert!(
            events.iter().any(|event| matches!(
                event,
                StreamEvent::Heartbeat(message) if message.contains("waiting for backend headers")
            )),
            "expected heartbeat before done, got {events:?}"
        );
        assert_eq!(events.last(), Some(&StreamEvent::Done));
    }

    #[test]
    fn stream_caps_header_wait_to_total_timeout() {
        let request_timeout = Duration::from_millis(80);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            thread::sleep(Duration::from_millis(300));
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\nevent: done\ndata: [DONE]\n\n",
            );
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_secs(2),
            request_timeout,
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();
        let started = Instant::now();

        let error = provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap_err();
        let elapsed = started.elapsed();
        server.join().unwrap();

        assert!(error.contains("backend stream timed out after 80ms"));
        assert!(
            events.iter().any(|event| matches!(
                event,
                StreamEvent::Heartbeat(message) if message.contains("waiting for backend headers")
            )),
            "expected heartbeat before total timeout, got {events:?}"
        );
        assert!(
            elapsed < Duration::from_secs(1),
            "expected total timeout cap to return before the 2s socket timeout; elapsed={elapsed:?}"
        );
    }

    #[test]
    fn stream_heartbeats_while_waiting_for_slow_gemma_body() {
        let read_timeout = Duration::from_millis(25);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n")
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(read_timeout + Duration::from_millis(100));
            stream.write_all(b"event: done\ndata: [DONE]\n\n").unwrap();
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout,
            request_timeout: Duration::from_secs(2),
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap();
        server.join().unwrap();

        assert!(
            events.iter().any(|event| matches!(
                event,
                StreamEvent::Heartbeat(message) if message.contains("waiting for Gemma stream")
            )),
            "expected heartbeat while waiting for stream body, got {events:?}"
        );
        assert_eq!(events.last(), Some(&StreamEvent::Done));
    }

    #[test]
    fn stream_caps_body_wait_to_total_timeout() {
        let request_timeout = Duration::from_millis(80);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n")
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(300));
            let _ = stream.write_all(b"event: done\ndata: [DONE]\n\n");
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_secs(2),
            request_timeout,
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();
        let started = Instant::now();

        let error = provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap_err();
        let elapsed = started.elapsed();
        server.join().unwrap();

        assert!(error.contains("backend stream timed out after 80ms"));
        assert!(
            events.iter().any(|event| matches!(
                event,
                StreamEvent::Heartbeat(message) if message.contains("waiting for Gemma stream")
            )),
            "expected heartbeat before total timeout, got {events:?}"
        );
        assert!(
            elapsed < Duration::from_secs(1),
            "expected total timeout cap to return before the 2s socket timeout; elapsed={elapsed:?}"
        );
    }

    #[test]
    fn stream_uses_request_timeout_as_total_wait_window() {
        let read_timeout = Duration::from_millis(20);
        let request_timeout = Duration::from_millis(80);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat-stream "));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n")
                .unwrap();
            stream.flush().unwrap();
            thread::sleep(request_timeout + Duration::from_millis(80));
            let _ = stream.write_all(b"event: done\ndata: [DONE]\n\n");
        });
        let provider = ForgeProvider::new(ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout,
            request_timeout,
        });
        let request = StreamRequest::chat("hello", Vec::new());
        let mut events = Vec::new();

        let error = provider
            .stream(&request, &mut |event| {
                events.push(event);
                Ok(())
            })
            .unwrap_err();
        server.join().unwrap();

        assert!(error.contains("backend stream timed out after 80ms"));
        assert!(
            events.iter().any(|event| matches!(
                event,
                StreamEvent::Heartbeat(message) if message.contains("waiting for Gemma stream")
            )),
            "expected heartbeat before total timeout, got {events:?}"
        );
        assert!(
            !error.contains("timed out after 20ms"),
            "stream must not treat read_timeout as the total wait window: {error}"
        );
    }

    fn test_config(backend: String) -> ProviderConfig {
        ProviderConfig {
            backend,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_secs(2),
            request_timeout: Duration::from_secs(5),
        }
    }

    fn spawn_http_server<F>(
        expected_requests: usize,
        responder: F,
    ) -> (String, JoinHandle<Vec<String>>)
    where
        F: Fn(&str) -> (u16, &'static str, String) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let handle = thread::spawn(move || {
            let mut request_lines = Vec::new();
            for _ in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_request(&mut stream);
                request_lines.push(request.lines().next().unwrap_or_default().to_owned());
                let (status, reason, body) = responder(&request);
                write_response(&mut stream, status, reason, &body);
            }
            request_lines
        });
        (backend, handle)
    }

    fn spawn_recording_http_server<F>(
        expected_requests: usize,
        responder: F,
    ) -> (String, JoinHandle<Vec<String>>)
    where
        F: Fn(&str) -> (u16, &'static str, String) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let backend = listener.local_addr().unwrap().to_string();
        let handle = thread::spawn(move || {
            let mut request_lines = Vec::new();
            let mut saw_expected_at = None;
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let request = read_request(&mut stream);
                        request_lines.push(request_line(&request).to_owned());
                        let (status, reason, body) = responder(&request);
                        write_response(&mut stream, status, reason, &body);
                        if request_lines.len() >= expected_requests && saw_expected_at.is_none() {
                            saw_expected_at = Some(Instant::now());
                        }
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        if saw_expected_at.is_some_and(|started: Instant| {
                            started.elapsed() >= Duration::from_millis(100)
                        }) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("fake backend accept failed: {error}"),
                }
            }
            request_lines
        });
        (backend, handle)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            bytes.extend_from_slice(&buffer[..read]);
            if request_complete(&bytes) {
                break;
            }
        }
        String::from_utf8(bytes).unwrap()
    }

    fn request_complete(bytes: &[u8]) -> bool {
        let Some((header_end, header_boundary_len)) = find_header_boundary(bytes) else {
            return false;
        };
        let headers = String::from_utf8_lossy(&bytes[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        bytes.len() >= header_end + header_boundary_len + content_length
    }

    #[test]
    fn request_complete_accepts_lf_only_headers_without_body() {
        assert!(request_complete(
            b"GET /health HTTP/1.1\nhost: localhost\n\n"
        ));
    }

    #[test]
    fn request_complete_waits_for_lf_only_content_length_body() {
        assert!(!request_complete(
            b"POST /v1/chat HTTP/1.1\ncontent-length: 5\n\nhell"
        ));
        assert!(request_complete(
            b"POST /v1/chat HTTP/1.1\ncontent-length: 5\n\nhello"
        ));
    }

    fn write_response(stream: &mut TcpStream, status: u16, reason: &str, body: &str) {
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    }

    fn request_line(request: &str) -> &str {
        request.lines().next().unwrap_or_default()
    }

    fn assert_no_model_stream_request(request: &str) {
        assert_no_model_stream_request_line(request_line(request));
    }

    fn assert_no_model_stream_request_lines(requests: &[String]) {
        for request in requests {
            assert_no_model_stream_request_line(request);
        }
    }

    fn assert_no_model_stream_request_line(request: &str) {
        for path in [
            "/v1/chat-stream",
            "/v1/generate-stream",
            "/v1/business-cycle-stream",
        ] {
            assert!(
                !request.contains(path),
                "model pool request must not call {path}: {request}"
            );
        }
    }

    fn cleanup_audit_body() -> String {
        format!(
            "{{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\":false,\"sample_limit\":7,\"error\":null,\"report\":{},\"index_report\":{},\"quarantine_plan\":{},\"repair_plan\":{}}}",
            report_json(),
            index_json(),
            quarantine_plan_json(),
            repair_plan_json()
        )
    }

    fn hygiene_body() -> String {
        format!(
            "{{\"ok\":true,\"request_id\":2,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"error\":null,\"report\":{},\"index_report\":{},\"quarantine_plan\":{}}}",
            report_json(),
            index_json(),
            quarantine_plan_json()
        )
    }

    fn quarantine_body() -> String {
        format!(
            "{{\"ok\":true,\"request_id\":3,\"experience_file\":\"noiron-experience.ndkv\",\"applied\":false,\"backup_file\":null,\"quarantine_file\":null,\"plan\":{}}}",
            quarantine_plan_json()
        )
    }

    fn repair_body() -> String {
        format!(
            "{{\"ok\":true,\"request_id\":4,\"experience_file\":\"noiron-experience.ndkv\",\"applied\":false,\"backup_file\":null,\"plan\":{}}}",
            repair_plan_json()
        )
    }

    fn model_pool_status_body() -> String {
        model_pool_status_body_with_contract(true, false, false)
    }

    fn model_pool_status_body_with_contract(
        read_only: bool,
        launches_process: bool,
        sends_prompt: bool,
    ) -> String {
        format!(
            "{{\"ok\":true,\"contract_version\":\"model-pool.v1\",\"read_only\":{},\"launches_process\":{},\"sends_prompt\":{},\"launch_allowed\":false,\"reason\":\"ready\",\"worker_count\":1,\"healthy_worker_count\":1,\"workers\":[{{\"role\":\"quality\",\"status\":\"ready\",\"ready\":true,\"base_url\":\"http://127.0.0.1:8686\",\"default_context_tokens\":262144,\"default_max_tokens\":262144,\"role_block_reason\":\"none\"}}]}}",
            bool_json(read_only),
            bool_json(launches_process),
            bool_json(sends_prompt)
        )
    }

    fn model_pool_route_body() -> String {
        "{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"review\",\"route_allowed\":true,\"reason\":\"ready\",\"role_candidates\":[\"review\",\"quality\"],\"selected_role\":\"quality\",\"selected_base_url\":\"http://127.0.0.1:8686\",\"selected_port\":8686,\"selected_default_max_tokens\":262144,\"selected_context_window\":8192,\"candidate_workers\":[]}".to_owned()
    }

    fn model_pool_route_body_for_worker(role: &str, base_url: &str, max_tokens: usize) -> String {
        format!(
            "{{\"ok\":true,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"{role}\",\"route_allowed\":true,\"reason\":\"ready\",\"role_candidates\":[\"{role}\"],\"selected_role\":\"{role}\",\"selected_base_url\":\"{base_url}\",\"selected_port\":8687,\"selected_default_max_tokens\":{max_tokens},\"selected_context_window\":8192,\"candidate_workers\":[]}}"
        )
    }

    fn bool_json(value: bool) -> &'static str {
        if value { "true" } else { "false" }
    }

    fn report_json() -> &'static str {
        "{\"total_records\":2,\"findings\":1,\"quarantine_candidates\":1,\"clean\":false,\"listed_findings\":[]}"
    }

    fn index_json() -> &'static str {
        "{\"total_records\":2,\"compacted_records\":1,\"noisy_records\":0,\"max_noise_penalty\":0.0,\"listed_findings\":[]}"
    }

    fn quarantine_plan_json() -> &'static str {
        "{\"applied\":false,\"total_records\":2,\"retained_records\":1,\"quarantine_candidates\":1,\"candidate_ids\":[1],\"listed_findings\":[]}"
    }

    fn repair_plan_json() -> &'static str {
        "{\"total_records\":2,\"legacy_metadata_lessons\":2,\"repairable_legacy_metadata_lessons\":1,\"index_noisy_records\":1,\"index_duplicate_outputs\":1,\"repairable_index_records\":1,\"remaining_legacy_metadata_lessons_after_repair\":1,\"remaining_watch_after_repair\":0,\"remaining_quarantine_candidates_after_repair\":1,\"skipped_quarantine_candidates\":1,\"skipped_missing_clean_gist\":0,\"projected_hygiene_after_repair\":{\"total_records\":2,\"findings\":1,\"watch\":0,\"quarantine_candidates\":1,\"legacy_metadata_lessons\":1,\"legacy_metadata_without_clean_gist\":0,\"index_quality_score\":0.88,\"index_noisy_records\":0,\"index_duplicate_outputs\":0,\"index_retrieval_ready\":true,\"index_risk_level\":\"watch\"},\"listed_repairs\":[]}"
    }

    fn retrieval_body() -> String {
        "{\"ok\":true,\"retrieval\":{\"prompt\":\"model pool route code\",\"profile\":\"coding\",\"index_context_used\":true,\"index_context_chars\":34,\"total_records\":10,\"requested_limit\":3,\"matches\":[{\"experience_id\":7,\"score\":0.9,\"quality\":0.8,\"process_reward\":0.7,\"reward_action\":\"reinforce\",\"lesson_preview\":\"accepted_pattern quality=0.9\",\"usable_hint_preview\":\"route through quality worker\",\"prompt_preview\":\"model pool route code\",\"runtime_model\":\"gemma-3-12b\",\"runtime_adapter\":\"llama.cpp\",\"runtime_device\":\"metal\",\"runtime_primary_lane\":\"quality\",\"runtime_fallback_lane\":\"summary\",\"runtime_memory_mode\":\"kv\",\"runtime_device_execution_source\":\"metal\",\"runtime_forward_energy\":0.72,\"runtime_kv_influence\":0.61,\"runtime_uncertainty_perplexity\":1.25,\"recursive_runtime_calls\":2,\"stored_runtime_kv_memory_ids\":[11,13]}],\"match_count\":1,\"skipped_cross_task_pollution\":0,\"retrieval_noise_penalized_candidates\":0,\"retrieval_noise_filtered_candidates\":0,\"suppressed_prompt_index_candidates\":0,\"max_retrieval_noise_penalty\":0.0,\"max_score\":0.9}}".to_owned()
    }
}
