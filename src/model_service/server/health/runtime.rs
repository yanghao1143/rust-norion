use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::Args;
use crate::model_service::json::{json_string_field, json_usize_field};

const GEMMA_RUNTIME_CONNECT_TIMEOUT: Duration = Duration::from_millis(120);
const GEMMA_RUNTIME_METADATA_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) struct RuntimeHealthStatus {
    pub(super) mode: &'static str,
    pub(super) gemma_reachable: Option<bool>,
    pub(super) gemma_model: Option<String>,
    pub(super) gemma_context_window: Option<usize>,
    pub(super) gemma_train_context_window: Option<usize>,
    pub(super) gemma_vocab_size: Option<usize>,
    pub(super) gemma_metadata_error: Option<String>,
}

pub(super) fn runtime_health_status(args: &Args) -> RuntimeHealthStatus {
    let gemma_metadata = gemma_runtime_model_status(args.gemma_runtime_server.as_deref());
    let gemma_reachable = if gemma_metadata.proves_reachable() {
        Some(true)
    } else {
        gemma_runtime_reachable(args.gemma_runtime_server.as_deref())
    };

    RuntimeHealthStatus {
        mode: runtime_mode(args),
        gemma_reachable,
        gemma_model: gemma_metadata.model,
        gemma_context_window: gemma_metadata.context_window,
        gemma_train_context_window: gemma_metadata.train_context_window,
        gemma_vocab_size: gemma_metadata.vocab_size,
        gemma_metadata_error: gemma_metadata.error,
    }
}

fn runtime_mode(args: &Args) -> &'static str {
    if args.gemma_runtime_server.is_some() {
        "gemma-http"
    } else if args.gemma_12b_runtime {
        "gemma-command"
    } else if args.runtime_command.is_some() {
        "command"
    } else if args.local_runtime {
        "local"
    } else if args.production_runtime {
        "production"
    } else {
        "built-in"
    }
}

fn gemma_runtime_reachable(server: Option<&str>) -> Option<bool> {
    server
        .and_then(parse_http_authority_socket_addr)
        .map(|address| TcpStream::connect_timeout(&address, GEMMA_RUNTIME_CONNECT_TIMEOUT).is_ok())
}

fn parse_http_authority_socket_addr(server: &str) -> Option<SocketAddr> {
    let trimmed = server.trim().trim_end_matches('/');
    let without_scheme = trimmed.strip_prefix("http://").unwrap_or(trimmed);
    if without_scheme.starts_with("https://") {
        return None;
    }
    let authority = without_scheme
        .split_once('/')
        .map(|(authority, _)| authority)
        .unwrap_or(without_scheme);
    authority.to_socket_addrs().ok()?.next()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GemmaRuntimeModelStatus {
    model: Option<String>,
    context_window: Option<usize>,
    train_context_window: Option<usize>,
    vocab_size: Option<usize>,
    error: Option<String>,
}

impl GemmaRuntimeModelStatus {
    fn proves_reachable(&self) -> bool {
        self.error.is_none()
            && (self.model.is_some()
                || self.context_window.is_some()
                || self.train_context_window.is_some()
                || self.vocab_size.is_some())
    }
}

fn gemma_runtime_model_status(server: Option<&str>) -> GemmaRuntimeModelStatus {
    let Some(server) = server else {
        return GemmaRuntimeModelStatus::default();
    };

    match get_http_json(server, "/v1/models") {
        Ok(body) => parse_gemma_runtime_models(&body),
        Err(error) => GemmaRuntimeModelStatus {
            error: Some(error),
            ..GemmaRuntimeModelStatus::default()
        },
    }
}

fn parse_gemma_runtime_models(body: &str) -> GemmaRuntimeModelStatus {
    let status = GemmaRuntimeModelStatus {
        model: json_string_field(body, "id")
            .or_else(|| json_string_field(body, "model"))
            .or_else(|| json_string_field(body, "name")),
        context_window: json_usize_field(body, "n_ctx"),
        train_context_window: json_usize_field(body, "n_ctx_train"),
        vocab_size: json_usize_field(body, "n_vocab"),
        error: None,
    };

    if status.proves_reachable() {
        status
    } else {
        GemmaRuntimeModelStatus {
            error: Some(
                "Gemma runtime /v1/models response did not include model metadata".to_owned(),
            ),
            ..status
        }
    }
}

fn get_http_json(server: &str, path: &str) -> Result<String, String> {
    let endpoint = parse_http_endpoint(server)?;
    let mut stream = TcpStream::connect_timeout(&endpoint.address, GEMMA_RUNTIME_CONNECT_TIMEOUT)
        .map_err(|error| {
        format!(
            "connect Gemma runtime metadata endpoint {} failed: {error}",
            endpoint.authority
        )
    })?;
    stream
        .set_read_timeout(Some(GEMMA_RUNTIME_METADATA_TIMEOUT))
        .map_err(|error| format!("set Gemma metadata read timeout failed: {error}"))?;
    stream
        .set_write_timeout(Some(GEMMA_RUNTIME_METADATA_TIMEOUT))
        .map_err(|error| format!("set Gemma metadata write timeout failed: {error}"))?;

    let request_path = endpoint.request_path(path);
    let request = format!(
        "GET {request_path} HTTP/1.1\r\nhost: {}\r\naccept: application/json\r\nconnection: close\r\n\r\n",
        endpoint.authority
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write Gemma metadata request failed: {error}"))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| format!("read Gemma metadata response failed: {error}"))?;
    parse_http_json_response(&response)
}

#[derive(Debug, Clone)]
struct HttpHealthEndpoint {
    authority: String,
    address: SocketAddr,
    base_path: String,
}

impl HttpHealthEndpoint {
    fn request_path(&self, path: &str) -> String {
        let path = if path.starts_with('/') {
            path.to_owned()
        } else {
            format!("/{path}")
        };
        if self.base_path.is_empty() {
            path
        } else {
            format!("{}{}", self.base_path, path)
        }
    }
}

fn parse_http_endpoint(server: &str) -> Result<HttpHealthEndpoint, String> {
    let trimmed = server.trim().trim_end_matches('/');
    let without_scheme = trimmed.strip_prefix("http://").unwrap_or(trimmed);
    if without_scheme.starts_with("https://") {
        return Err("Gemma runtime metadata only supports local http:// endpoints".to_owned());
    }
    let (authority, base_path) = without_scheme
        .split_once('/')
        .map(|(authority, path)| (authority, normalize_base_path(path)))
        .unwrap_or_else(|| (without_scheme, String::new()));
    let address = authority
        .to_socket_addrs()
        .map_err(|error| format!("resolve Gemma runtime metadata endpoint failed: {error}"))?
        .next()
        .ok_or_else(|| {
            "resolve Gemma runtime metadata endpoint failed: no socket address".to_owned()
        })?;
    Ok(HttpHealthEndpoint {
        authority: authority.to_owned(),
        address,
        base_path,
    })
}

fn normalize_base_path(path: &str) -> String {
    let path = path.trim_matches('/');
    if path.is_empty() {
        String::new()
    } else {
        format!("/{path}")
    }
}

fn parse_http_json_response(response: &[u8]) -> Result<String, String> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "Gemma metadata response did not include HTTP headers".to_owned())?;
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .unwrap_or(0);
    let body = String::from_utf8_lossy(&response[header_end + 4..]).to_string();
    if !(200..300).contains(&status) {
        return Err(format!(
            "Gemma metadata endpoint returned HTTP {status}: {}",
            body.trim()
        ));
    }
    Ok(body)
}

pub(super) fn option_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub(super) fn option_usize_json(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Args;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn runtime_status_reports_unreachable_gemma_http_runtime() {
        let args = Args::parse(vec![
            "--gemma-runtime-server".to_owned(),
            "http://127.0.0.1:9".to_owned(),
        ]);

        let status = runtime_health_status(&args);

        assert_eq!(status.mode, "gemma-http");
        assert_eq!(status.gemma_reachable, Some(false));
        assert!(status.gemma_context_window.is_none());
        assert!(status.gemma_metadata_error.is_some());
    }

    #[test]
    fn runtime_status_reports_gemma_http_model_context() {
        let (server, handle) = spawn_models_server(
            "{\"data\":[{\"id\":\"gemma-4-12b-it-Q8_0.gguf\",\"meta\":{\"n_vocab\":262144,\"n_ctx\":262144,\"n_ctx_train\":262144}}]}",
        );
        let args = Args::parse(vec!["--gemma-runtime-server".to_owned(), server]);

        let status = runtime_health_status(&args);

        handle.join().unwrap();
        assert_eq!(status.mode, "gemma-http");
        assert_eq!(status.gemma_reachable, Some(true));
        assert_eq!(
            status.gemma_model.as_deref(),
            Some("gemma-4-12b-it-Q8_0.gguf")
        );
        assert_eq!(status.gemma_context_window, Some(262_144));
        assert_eq!(status.gemma_train_context_window, Some(262_144));
        assert_eq!(status.gemma_vocab_size, Some(262_144));
        assert!(status.gemma_metadata_error.is_none());
    }

    #[test]
    fn option_bool_json_serializes_nullable_booleans() {
        assert_eq!(option_bool_json(Some(true)), "true");
        assert_eq!(option_bool_json(Some(false)), "false");
        assert_eq!(option_bool_json(None), "null");
    }

    #[test]
    fn option_usize_json_serializes_nullable_numbers() {
        assert_eq!(option_usize_json(Some(262_144)), "262144");
        assert_eq!(option_usize_json(None), "null");
    }

    fn spawn_models_server(body: &'static str) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_request(&mut stream);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        (format!("http://{address}"), handle)
    }

    fn read_request(stream: &mut TcpStream) {
        let mut buffer = [0_u8; 1024];
        let _ = stream.read(&mut buffer).unwrap();
    }
}
