use std::io::{Read, Write};

use crate::http::split_http_head_body;
use crate::request::{ChatRequest, LabEndpoint};
use std::time::Duration;

mod body;
mod gate;
mod io;
mod parse;
mod stream;
mod types;

use body::backend_request_body;
pub(crate) use gate::backend_prompt_block_reason;
use io::{
    BACKEND_WRITE_TIMEOUT, backend_io_error_message, connect_backend, connect_backend_for_stream,
};
use parse::{parse_backend_health, parse_backend_result};
use stream::stream_backend_events;
pub(crate) use types::{
    BackendActiveRequest, BackendExperienceHygiene, BackendExperienceIndex, BackendHealth,
    BackendLastInference, BackendResult,
};

#[cfg(test)]
use io::BACKEND_STREAM_READ_POLL_INTERVAL;
#[cfg(test)]
use stream::relay_sse_frame;

pub(crate) fn call_backend_health(
    backend: &str,
    response_timeout: Duration,
) -> Result<BackendHealth, String> {
    let mut stream = connect_backend(backend, response_timeout)?;
    let http_request =
        format!("GET /health HTTP/1.1\r\nhost: {backend}\r\nconnection: close\r\n\r\n");
    stream.write_all(http_request.as_bytes()).map_err(|error| {
        backend_io_error_message("write backend health request", error, BACKEND_WRITE_TIMEOUT)
    })?;
    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|error| {
        backend_io_error_message("read backend health response", error, response_timeout)
    })?;
    let (_, response_body) = split_http_head_body(&response);
    Ok(parse_backend_health(response_body))
}

pub(crate) fn call_backend_model_pool_status(
    backend: &str,
    response_timeout: Duration,
) -> Result<String, String> {
    let mut stream = connect_backend(backend, response_timeout)?;
    let http_request = format!(
        "GET /v1/model-pool/status HTTP/1.1\r\nhost: {backend}\r\nconnection: close\r\n\r\n"
    );
    stream.write_all(http_request.as_bytes()).map_err(|error| {
        backend_io_error_message(
            "write backend model pool status request",
            error,
            BACKEND_WRITE_TIMEOUT,
        )
    })?;
    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|error| {
        backend_io_error_message(
            "read backend model pool status response",
            error,
            response_timeout,
        )
    })?;
    let (_, response_body) = split_http_head_body(&response);
    let trimmed = response_body.trim();
    if trimmed.is_empty() {
        return Err("backend model pool status returned an empty body".to_owned());
    }
    if !trimmed.starts_with('{') {
        return Err(format!(
            "backend model pool status returned non-json body: {}",
            trimmed.chars().take(120).collect::<String>()
        ));
    }
    Ok(trimmed.to_owned())
}

pub(crate) fn call_backend(
    backend: &str,
    request: &ChatRequest,
    response_timeout: Duration,
) -> Result<BackendResult, String> {
    let path = match request.endpoint {
        LabEndpoint::Chat => "/v1/chat",
        LabEndpoint::Generate => "/v1/generate",
        LabEndpoint::BusinessCycle => "/v1/business-cycle",
    };
    let body = backend_request_body(request);

    let mut stream = connect_backend(backend, response_timeout)?;
    let http_request = format!(
        "POST {path} HTTP/1.1\r\nhost: {backend}\r\ncontent-type: application/json; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(http_request.as_bytes()).map_err(|error| {
        backend_io_error_message("write backend request", error, BACKEND_WRITE_TIMEOUT)
    })?;
    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|error| {
        backend_io_error_message("read backend response", error, response_timeout)
    })?;
    let (_, response_body) = split_http_head_body(&response);
    parse_backend_result(response_body)
}

pub(crate) fn call_backend_event_stream(
    backend: &str,
    request: &ChatRequest,
    response_timeout: Duration,
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), String>,
) -> Result<(), String> {
    let path = match request.endpoint {
        LabEndpoint::Chat => "/v1/chat-stream",
        LabEndpoint::Generate => "/v1/generate-stream",
        LabEndpoint::BusinessCycle => "/v1/business-cycle-stream",
    };
    let body = backend_request_body(request);
    let mut stream = connect_backend_for_stream(backend, response_timeout)?;
    let http_request = format!(
        "POST {path} HTTP/1.1\r\nhost: {backend}\r\ncontent-type: application/json; charset=utf-8\r\naccept: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(http_request.as_bytes()).map_err(|error| {
        backend_io_error_message("write backend stream request", error, BACKEND_WRITE_TIMEOUT)
    })?;
    stream_backend_events(&mut stream, response_timeout, on_event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::parse_chat_request;

    #[test]
    fn business_cycle_body_includes_feedback_and_rust_check() {
        let request = parse_chat_request(
            "{\"prompt\":\"业务\",\"endpoint\":\"business-cycle\",\"feedback_amount\":0.4,\"rust_check_code\":\"pub fn ok() {}\",\"self_improve\":false}",
        )
        .unwrap();
        let body = backend_request_body(&request);
        assert!(body.contains("\"feedback_amount\":0.4"));
        assert!(body.contains("\"max_tokens\":262144"));
        assert!(body.contains("\"self_improve\":false"));
        assert!(body.contains("\"rust_check_code\":\"pub fn ok() {}\""));
        assert!(body.contains("\"gate\":\"business_cycle\""));
    }

    #[test]
    fn chat_body_preserves_history_messages() {
        let request = parse_chat_request(
            "{\"prompt\":\"第二问\",\"endpoint\":\"chat\",\"messages\":[{\"role\":\"user\",\"content\":\"第一问\"},{\"role\":\"assistant\",\"content\":\"第一答\"},{\"role\":\"user\",\"content\":\"第二问\"}]}",
        )
        .unwrap();

        let body = backend_request_body(&request);

        assert!(body.contains("\"role\":\"assistant\",\"content\":\"第一答\""));
        assert!(body.contains("\"role\":\"user\",\"content\":\"第二问\""));
        assert!(body.contains("\"max_tokens\":262144"));
        assert!(body.contains("\"case\":\"rustgpt-lab-chat\""));
    }

    #[test]
    fn generate_body_uses_prompt_contract() {
        let request = parse_chat_request(
            "{\"prompt\":\"写一个摘要\",\"endpoint\":\"generate\",\"profile\":\"writing\",\"output\":\"enhanced\"}",
        )
        .unwrap();

        let body = backend_request_body(&request);

        assert!(body.contains("\"prompt\":\"写一个摘要\""));
        assert!(body.contains("\"profile\":\"writing\""));
        assert!(body.contains("\"output\":\"enhanced\""));
        assert!(body.contains("\"max_tokens\":262144"));
        assert!(body.contains("\"case\":\"rustgpt-lab-generate\""));
        assert!(!body.contains("\"messages\""));
    }

    #[test]
    fn relays_sse_frame_event_and_data() {
        let mut events = Vec::new();
        relay_sse_frame(b"event: delta\ndata: hello\n\n", &mut |event, data| {
            events.push((event.to_owned(), data.to_owned()));
            Ok(())
        })
        .unwrap();

        assert_eq!(events, vec![("delta".to_owned(), "hello".to_owned())]);
    }

    #[test]
    fn relays_multiline_sse_frame_data() {
        let mut events = Vec::new();
        relay_sse_frame(
            b"event: final\r\ndata: line1\r\ndata: line2\r\n\r\n",
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(
            events,
            vec![("final".to_owned(), "line1\nline2".to_owned())]
        );
    }

    #[test]
    fn ignores_comment_only_sse_frame() {
        let mut events = Vec::new();
        let terminal = relay_sse_frame(b": keep-alive\n\n", &mut |event, data| {
            events.push((event.to_owned(), data.to_owned()));
            Ok(())
        })
        .unwrap();

        assert!(!terminal);
        assert!(events.is_empty());
    }

    #[test]
    fn relays_sse_field_without_colon_as_empty_value() {
        let mut events = Vec::new();
        relay_sse_frame(b"data\n\n", &mut |event, data| {
            events.push((event.to_owned(), data.to_owned()));
            Ok(())
        })
        .unwrap();

        assert_eq!(events, vec![("message".to_owned(), String::new())]);
    }

    #[test]
    fn relays_empty_sse_event_field_as_message_event() {
        let mut events = Vec::new();
        relay_sse_frame(b"event:\ndata: hello\n\n", &mut |event, data| {
            events.push((event.to_owned(), data.to_owned()));
            Ok(())
        })
        .unwrap();

        assert_eq!(events, vec![("message".to_owned(), "hello".to_owned())]);
    }

    #[test]
    fn relays_empty_sse_event_field_without_colon_as_message_event() {
        let mut events = Vec::new();
        relay_sse_frame(b"event\ndata: hello\n\n", &mut |event, data| {
            events.push((event.to_owned(), data.to_owned()));
            Ok(())
        })
        .unwrap();

        assert_eq!(events, vec![("message".to_owned(), "hello".to_owned())]);
    }

    #[test]
    fn preserves_sse_data_indentation_after_optional_space() {
        let mut events = Vec::new();
        relay_sse_frame(b"data:   indented\n\n", &mut |event, data| {
            events.push((event.to_owned(), data.to_owned()));
            Ok(())
        })
        .unwrap();

        assert_eq!(
            events,
            vec![("message".to_owned(), "  indented".to_owned())]
        );
    }

    #[test]
    fn backend_io_error_message_names_timeouts() {
        let timed_out = backend_io_error_message(
            "read backend response",
            std::io::Error::new(std::io::ErrorKind::TimedOut, "deadline"),
            Duration::from_secs(9),
        );
        let would_block = backend_io_error_message(
            "read backend stream body",
            std::io::Error::new(std::io::ErrorKind::WouldBlock, "deadline"),
            Duration::from_secs(11),
        );
        let short_poll = backend_io_error_message(
            "read backend stream headers",
            std::io::Error::new(std::io::ErrorKind::TimedOut, "deadline"),
            Duration::from_millis(250),
        );
        let mixed_seconds = backend_io_error_message(
            "read backend stream body",
            std::io::Error::new(std::io::ErrorKind::TimedOut, "deadline"),
            Duration::from_millis(1500),
        );
        let refused = backend_io_error_message(
            "connect backend",
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused"),
            Duration::from_secs(3),
        );

        assert_eq!(
            timed_out,
            "read backend response failed: timed out after 9s"
        );
        assert_eq!(
            would_block,
            "read backend stream body failed: timed out after 11s"
        );
        assert_eq!(
            short_poll,
            "read backend stream headers failed: timed out after 250ms"
        );
        assert_eq!(
            mixed_seconds,
            "read backend stream body failed: timed out after 1500ms"
        );
        assert_eq!(refused, "connect backend failed: refused");
    }

    #[test]
    fn connect_backend_sets_socket_timeouts() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || listener.accept().unwrap());

        let stream = connect_backend(&address.to_string(), Duration::from_secs(17)).unwrap();

        assert_eq!(
            stream.read_timeout().unwrap(),
            Some(Duration::from_secs(17))
        );
        assert_eq!(stream.write_timeout().unwrap(), Some(BACKEND_WRITE_TIMEOUT));
        drop(stream);
        let _ = accepted.join().unwrap();
    }

    #[test]
    fn connect_backend_for_stream_uses_short_polling_read_timeout() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || listener.accept().unwrap());

        let stream =
            connect_backend_for_stream(&address.to_string(), Duration::from_secs(900)).unwrap();

        assert_eq!(
            stream.read_timeout().unwrap(),
            Some(BACKEND_STREAM_READ_POLL_INTERVAL)
        );
        assert_eq!(stream.write_timeout().unwrap(), Some(BACKEND_WRITE_TIMEOUT));
        drop(stream);
        let _ = accepted.join().unwrap();
    }

    #[test]
    fn connect_backend_for_stream_caps_read_timeout_to_total_timeout() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || listener.accept().unwrap());

        let stream =
            connect_backend_for_stream(&address.to_string(), Duration::from_millis(250)).unwrap();

        assert_eq!(
            stream.read_timeout().unwrap(),
            Some(Duration::from_millis(250))
        );
        assert_eq!(stream.write_timeout().unwrap(), Some(BACKEND_WRITE_TIMEOUT));
        drop(stream);
        let _ = accepted.join().unwrap();
    }

    #[test]
    fn call_backend_event_stream_heartbeats_during_slow_backend_headers() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("POST /v1/chat-stream HTTP/1.1"));
            std::thread::sleep(BACKEND_STREAM_READ_POLL_INTERVAL + Duration::from_secs(1));
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n\
event: done\ndata: [DONE]\n\n",
                )
                .unwrap();
        });
        let request = parse_chat_request("{\"prompt\":\"hello\",\"endpoint\":\"chat\"}").unwrap();
        let mut events = Vec::new();

        call_backend_event_stream(
            &address.to_string(),
            &request,
            Duration::from_secs(5),
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap();

        assert!(
            events
                .iter()
                .any(|(event, data)| event == "heartbeat" && data.contains("本地后端"))
        );
        assert_eq!(
            events.last(),
            Some(&("done".to_owned(), "[DONE]".to_owned()))
        );
        accepted.join().unwrap();
    }

    #[test]
    fn call_backend_event_stream_heartbeats_during_slow_backend_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("POST /v1/chat-stream HTTP/1.1"));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n")
                .unwrap();
            stream.flush().unwrap();
            std::thread::sleep(BACKEND_STREAM_READ_POLL_INTERVAL + Duration::from_secs(1));
            stream.write_all(b"event: done\ndata: [DONE]\n\n").unwrap();
        });
        let request = parse_chat_request("{\"prompt\":\"hello\",\"endpoint\":\"chat\"}").unwrap();
        let mut events = Vec::new();

        call_backend_event_stream(
            &address.to_string(),
            &request,
            Duration::from_secs(5),
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap();

        assert!(
            events
                .iter()
                .any(|(event, data)| event == "heartbeat" && data.contains("本地后端"))
        );
        assert_eq!(
            events.last(),
            Some(&("done".to_owned(), "[DONE]".to_owned()))
        );
        accepted.join().unwrap();
    }

    #[test]
    fn call_backend_event_stream_times_out_during_slow_backend_headers() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("POST /v1/chat-stream HTTP/1.1"));
            std::thread::sleep(Duration::from_millis(400));
        });
        let request = parse_chat_request("{\"prompt\":\"hello\",\"endpoint\":\"chat\"}").unwrap();
        let mut events = Vec::new();

        let error = call_backend_event_stream(
            &address.to_string(),
            &request,
            Duration::from_millis(150),
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap_err();

        assert!(error.contains("backend stream timed out after 150ms while waiting for headers"));
        assert!(
            events
                .iter()
                .any(|(event, data)| event == "heartbeat" && data.contains("本地后端"))
        );
        accepted.join().unwrap();
    }

    #[test]
    fn call_backend_event_stream_times_out_during_slow_backend_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("POST /v1/chat-stream HTTP/1.1"));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n")
                .unwrap();
            stream.flush().unwrap();
            std::thread::sleep(Duration::from_millis(400));
        });
        let request = parse_chat_request("{\"prompt\":\"hello\",\"endpoint\":\"chat\"}").unwrap();
        let mut events = Vec::new();

        let error = call_backend_event_stream(
            &address.to_string(),
            &request,
            Duration::from_millis(150),
            &mut |event, data| {
                events.push((event.to_owned(), data.to_owned()));
                Ok(())
            },
        )
        .unwrap_err();

        assert!(error.contains("backend stream timed out after 150ms while waiting for body"));
        assert!(
            events
                .iter()
                .any(|(event, data)| event == "heartbeat" && data.contains("本地后端"))
        );
        accepted.join().unwrap();
    }

    #[test]
    fn proxies_model_pool_status_as_raw_json() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 512];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("GET /v1/model-pool/status HTTP/1.1"));
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 87\r\nconnection: close\r\n\r\n{\"ok\":true,\"worker_count\":2,\"healthy_worker_count\":1,\"workers\":[],\"route_metrics\":{}}",
                )
                .unwrap();
        });

        let body =
            call_backend_model_pool_status(&address.to_string(), Duration::from_secs(3)).unwrap();

        assert!(body.contains("\"worker_count\":2"));
        assert!(body.contains("\"healthy_worker_count\":1"));
        accepted.join().unwrap();
    }

    #[test]
    fn model_pool_status_rejects_non_json_backend_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 512];
            let _ = stream.read(&mut request).unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 404 Error\r\ncontent-type: text/plain\r\ncontent-length: 11\r\nconnection: close\r\n\r\nunsupported",
                )
                .unwrap();
        });

        let error = call_backend_model_pool_status(&address.to_string(), Duration::from_secs(3))
            .unwrap_err();

        assert!(error.contains("non-json body"));
        accepted.join().unwrap();
    }

    #[test]
    fn parses_backend_health_busy_state() {
        let health = parse_backend_health(
            "{\"ok\":true,\"service\":\"rust-norion\",\"requests_seen\":3,\"active_engine_requests\":1,\"engine_busy\":true,\"active_requests\":[{\"request_id\":42,\"endpoint\":\"chat-stream\",\"elapsed_ms\":1234,\"prompt_preview\":\"hello\"}],\"runtime_mode\":\"gemma-http\",\"gemma_runtime_server\":\"http://127.0.0.1:8686\",\"gemma_runtime_reachable\":false,\"gemma_runtime_model\":\"gemma-4-12b-it-Q8_0.gguf\",\"gemma_runtime_context_window\":262144,\"gemma_runtime_train_context_window\":262144,\"gemma_runtime_vocab_size\":262144,\"gemma_runtime_metadata_error\":null,\"readiness_ok\":false,\"safe_device_ok\":false,\"readiness_failures\":[\"engine_busy\"],\"safe_device_failures\":[\"cpu-first\"],\"device_primary_lane\":\"cpu-vector\",\"device_memory_mode\":\"disk-backed-streaming\",\"experience_hygiene\":{\"experience_file\":\"D:\\\\rust-norion\\\\target\\\\state\\\\experience.ndkv\",\"checked\":true,\"clean\":false,\"findings\":2,\"quarantine_candidates\":1,\"repair\":{\"repairable_legacy_metadata_lessons\":3,\"repairable_index_records\":1},\"index\":{\"total_records\":863,\"noisy_records\":1,\"duplicate_outputs\":1,\"quality_score\":0.58,\"retrieval_ready\":false,\"risk_level\":\"blocked\"}}}",
        );
        assert!(health.ok);
        assert_eq!(health.service.as_deref(), Some("rust-norion"));
        assert_eq!(health.requests_seen.as_deref(), Some("3"));
        assert_eq!(health.active_engine_requests.as_deref(), Some("1"));
        assert_eq!(health.engine_busy, Some(true));
        assert_eq!(health.active_requests.len(), 1);
        assert_eq!(health.active_requests[0].request_id.as_deref(), Some("42"));
        assert_eq!(
            health.active_requests[0].endpoint.as_deref(),
            Some("chat-stream")
        );
        assert_eq!(
            health.active_requests[0].prompt_preview.as_deref(),
            Some("hello")
        );
        assert_eq!(health.runtime_mode.as_deref(), Some("gemma-http"));
        assert_eq!(
            health.gemma_runtime_server.as_deref(),
            Some("http://127.0.0.1:8686")
        );
        assert_eq!(health.gemma_runtime_reachable, Some(false));
        assert_eq!(
            health.gemma_runtime_model.as_deref(),
            Some("gemma-4-12b-it-Q8_0.gguf")
        );
        assert_eq!(
            health.gemma_runtime_context_window.as_deref(),
            Some("262144")
        );
        assert_eq!(
            health.gemma_runtime_train_context_window.as_deref(),
            Some("262144")
        );
        assert_eq!(health.gemma_runtime_vocab_size.as_deref(), Some("262144"));
        assert!(health.gemma_runtime_metadata_error.is_none());
        assert_eq!(health.readiness_ok, Some(false));
        assert_eq!(health.safe_device_ok, Some(false));
        assert_eq!(health.readiness_failures, vec!["engine_busy"]);
        assert_eq!(health.safe_device_failures, vec!["cpu-first"]);
        assert_eq!(health.device_primary_lane.as_deref(), Some("cpu-vector"));
        assert_eq!(
            health.device_memory_mode.as_deref(),
            Some("disk-backed-streaming")
        );
        let hygiene = health.experience_hygiene.unwrap();
        assert_eq!(
            hygiene.experience_file.as_deref(),
            Some("D:\\rust-norion\\target\\state\\experience.ndkv")
        );
        assert_eq!(hygiene.checked, Some(true));
        assert_eq!(hygiene.clean, Some(false));
        assert_eq!(hygiene.quarantine_candidates.as_deref(), Some("1"));
        assert_eq!(
            hygiene.repairable_legacy_metadata_lessons.as_deref(),
            Some("3")
        );
        assert_eq!(hygiene.repairable_index_records.as_deref(), Some("1"));
        let index = hygiene.index.unwrap();
        assert_eq!(index.total_records.as_deref(), Some("863"));
        assert_eq!(index.noisy_records.as_deref(), Some("1"));
        assert_eq!(index.duplicate_outputs.as_deref(), Some("1"));
        assert_eq!(index.quality_score.as_deref(), Some("0.58"));
        assert_eq!(index.retrieval_ready, Some(false));
        assert_eq!(index.risk_level.as_deref(), Some("blocked"));
    }

    #[test]
    fn backend_health_rejects_trailing_garbage_after_nested_objects() {
        let health = parse_backend_health(
            "{\"ok\":true,\"experience_hygiene\":{\"checked\":true}x,\"last_inference\":{\"request_id\":7}x}",
        );

        assert!(health.experience_hygiene.is_none());
        assert!(health.last_inference.is_none());
    }

    #[test]
    fn backend_health_ignores_object_keys_inside_string_values() {
        let health = parse_backend_health(
            r#"{"ok":true,"note":"\"experience_hygiene\":{\"checked\":false},\"last_inference\":{\"request_id\":999},","experience_hygiene":{"checked":true,"index":{"risk_level":"watch"}},"last_inference":{"request_id":7}}"#,
        );

        let hygiene = health.experience_hygiene.unwrap();
        assert_eq!(hygiene.checked, Some(true));
        assert_eq!(hygiene.index.unwrap().risk_level.as_deref(), Some("watch"));
        assert_eq!(
            health.last_inference.unwrap().request_id.as_deref(),
            Some("7")
        );
    }

    #[test]
    fn backend_result_ignores_answer_key_inside_string_values() {
        let result = parse_backend_result(
            r#"{"ok":true,"note":"\"answer\":\"poison\"","answer":"real answer","runtime_model":"gemma"}"#,
        )
        .unwrap();

        assert_eq!(result.answer, "real answer");
        assert_eq!(result.runtime_model.as_deref(), Some("gemma"));
    }

    #[test]
    fn backend_result_discards_raw_answer_side_channels() {
        let result = parse_backend_result(
            r#"{"ok":true,"answer":"selected answer","raw_answer":"raw side channel","enhanced_answer":"enhanced side channel"}"#,
        )
        .unwrap();

        let debug = format!("{result:?}");
        assert_eq!(result.answer, "selected answer");
        assert!(!debug.contains("raw side channel"));
        assert!(!debug.contains("enhanced side channel"));
    }

    #[test]
    fn backend_health_rejects_malformed_readiness_failure_arrays() {
        let health = parse_backend_health(
            "{\"ok\":true,\"readiness_failures\":[\"engine_busy\",\"bad\\q\"],\"safe_device_failures\":[\"cpu-first\"]x}",
        );

        assert!(health.readiness_failures.is_empty());
        assert!(health.safe_device_failures.is_empty());
    }

    #[test]
    fn parses_backend_health_last_inference() {
        let health = parse_backend_health(
            "{\"ok\":true,\"last_inference\":{\"request_id\":7,\"endpoint\":\"generate\",\"elapsed_ms\":1234,\"runtime_model\":\"gemma\",\"runtime_token_count\":19,\"quality\":0.9,\"process_reward\":0.8,\"action\":\"reinforce\",\"error\":null}}",
        );

        let last = health.last_inference.unwrap();
        assert_eq!(last.request_id.as_deref(), Some("7"));
        assert_eq!(last.endpoint.as_deref(), Some("generate"));
        assert_eq!(last.elapsed_ms.as_deref(), Some("1234"));
        assert_eq!(last.runtime_model.as_deref(), Some("gemma"));
        assert_eq!(last.runtime_token_count.as_deref(), Some("19"));
        assert_eq!(last.action.as_deref(), Some("reinforce"));
    }
}
