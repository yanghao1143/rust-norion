use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::backend::{
    BackendResult, backend_prompt_block_reason, call_backend, call_backend_event_stream,
    call_backend_health, call_backend_model_pool_status,
};
use crate::chunk::answer_chunks;
use crate::config::Config;
use crate::http::{read_http_request, split_http_head_body, write_json};
use crate::model_pool_advice::model_pool_advice_json;
use crate::request::{parse_chat_request, request_context_preview};
use crate::sse::{send_sse, write_sse_headers};
use crate::status::{backend_error_hint, wait_status_message};

mod assets;
mod health_json;

use health_json::backend_health_json;

const JSON_FALLBACK_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
const JSON_FALLBACK_TIMEOUT_GRACE: Duration = Duration::from_secs(5);

pub(crate) fn run(config: Config) -> std::io::Result<()> {
    let listener = TcpListener::bind(&config.bind)?;
    println!("rustgpt-lab listening on http://{}", config.bind);
    println!("backend: http://{}", config.backend);
    println!(
        "backend response timeout: {}s",
        config.backend_response_timeout.as_secs()
    );

    for stream in listener.incoming() {
        let config = config.clone();
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    if let Err(error) = handle_connection(stream, &config) {
                        eprintln!("request failed: {error}");
                    }
                });
            }
            Err(error) => eprintln!("connection failed: {error}"),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream, config: &Config) -> std::io::Result<()> {
    let raw = read_http_request(&mut stream)?;
    let (head, body) = split_http_head_body(&raw);
    let Some(request_line) = head.lines().next() else {
        return write_json(
            &mut stream,
            400,
            "{\"ok\":false,\"error\":\"missing request line\"}",
        );
    };
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();

    match (method, path) {
        ("GET", "/") => assets::write_index(&mut stream),
        ("GET", "/app.css") => assets::write_css(&mut stream),
        ("GET", "/app.js") => assets::write_js(&mut stream),
        ("GET", "/health") => write_json(
            &mut stream,
            200,
            &format!(
                "{{\"ok\":true,\"service\":\"rustgpt-lab\",\"backend\":{},\"backend_response_timeout_secs\":{}}}",
                crate::json::json_string(&config.backend),
                config.backend_response_timeout.as_secs()
            ),
        ),
        ("GET", "/api/backend-health") => handle_backend_health(stream, config),
        ("GET", "/api/model-pool-status") => handle_model_pool_status(stream, config),
        ("GET", "/api/model-pool-advice") => handle_model_pool_advice(stream, config),
        ("POST", "/api/chat-stream") => handle_chat_stream(stream, config.clone(), body.to_owned()),
        _ => write_json(
            &mut stream,
            404,
            "{\"ok\":false,\"error\":\"unsupported route\"}",
        ),
    }
}

fn handle_backend_health(mut stream: TcpStream, config: &Config) -> std::io::Result<()> {
    let body = match call_backend_health(&config.backend, config.backend_response_timeout) {
        Ok(health) => backend_health_json(&health),
        Err(error) => format!(
            "{{\"ok\":false,\"error\":{}}}",
            crate::json::json_string(&backend_error_hint(&error))
        ),
    };
    write_json(&mut stream, 200, &body)
}

fn handle_model_pool_status(mut stream: TcpStream, config: &Config) -> std::io::Result<()> {
    let body =
        match call_backend_model_pool_status(&config.backend, config.backend_response_timeout) {
            Ok(body) => body,
            Err(error) => format!(
                "{{\"ok\":false,\"error\":{}}}",
                crate::json::json_string(&backend_error_hint(&error))
            ),
        };
    write_json(&mut stream, 200, &body)
}

fn handle_model_pool_advice(mut stream: TcpStream, config: &Config) -> std::io::Result<()> {
    let body = match call_backend_model_pool_status(
        &config.backend,
        config.backend_response_timeout,
    ) {
        Ok(status_body) => model_pool_advice_json(&status_body),
        Err(error) => format!(
            "{{\"ok\":false,\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"error\":{}}}",
            crate::json::json_string(&backend_error_hint(&error))
        ),
    };
    write_json(&mut stream, 200, &body)
}

fn handle_chat_stream(mut stream: TcpStream, config: Config, body: String) -> std::io::Result<()> {
    let request = match parse_chat_request(&body) {
        Ok(request) => request,
        Err(error) => {
            return write_json(
                &mut stream,
                400,
                &format!(
                    "{{\"ok\":false,\"error\":{}}}",
                    crate::json::json_string(&error)
                ),
            );
        }
    };

    write_sse_headers(&mut stream)?;
    send_sse(&mut stream, "status", "proxy connected")?;
    send_sse(
        &mut stream,
        "status",
        "checking backend prompt gate before forwarding request",
    )?;
    if let Some(block_reason) = backend_prompt_gate_block_reason(&config) {
        send_sse(&mut stream, "error", &block_reason)?;
        send_sse(&mut stream, "done", "[DONE]")?;
        return Ok(());
    }
    send_sse(
        &mut stream,
        "status",
        "local rust-norion backend is connected; real Gemma 12B first load or long answers may still take minutes",
    )?;
    send_sse(
        &mut stream,
        "status",
        &format!(
            "calling rust-norion {} with output={}",
            request.endpoint.as_label(),
            request.output
        ),
    )?;
    send_sse(&mut stream, "request", &request_context_preview(&request))?;

    if request.endpoint.supports_token_stream() {
        send_sse(
            &mut stream,
            "status",
            "using backend token stream; deltas are relayed as rust-norion emits them",
        )?;
        match call_backend_event_stream(
            &config.backend,
            &request,
            config.backend_response_timeout,
            &mut |event, data| {
                send_sse(&mut stream, event, data).map_err(|error| error.to_string())
            },
        ) {
            Ok(()) => return Ok(()),
            Err(error) => {
                if !backend_stream_error_can_fallback(&error) {
                    send_sse(&mut stream, "error", &backend_error_hint(&error))?;
                    send_sse(&mut stream, "done", "[DONE]")?;
                    return Ok(());
                }
                send_sse(
                    &mut stream,
                    "status",
                    &format!(
                        "backend token stream unavailable; falling back to JSON proxy: {}; JSON fallback waits for the full backend response before chunking the final answer",
                        backend_error_hint(&error)
                    ),
                )?;
            }
        }
    }

    let (tx, rx) = mpsc::channel();
    let backend = config.backend.clone();
    let response_timeout = config.backend_response_timeout;
    let request_for_worker = request.clone();
    thread::spawn(move || {
        let result = call_backend(&backend, &request_for_worker, response_timeout);
        let _ = tx.send(result);
    });

    let started = Instant::now();
    let fallback_timeout = json_fallback_timeout(config.backend_response_timeout);
    send_sse(
        &mut stream,
        "status",
        &format!(
            "JSON fallback active; waiting up to {}s for the complete backend response before sending answer chunks",
            fallback_timeout.as_secs()
        ),
    )?;
    loop {
        if started.elapsed() >= fallback_timeout {
            send_json_fallback_timeout(&mut stream, started.elapsed(), fallback_timeout)?;
            break;
        }

        let wait = JSON_FALLBACK_HEARTBEAT_INTERVAL
            .min(fallback_timeout.saturating_sub(started.elapsed()));
        match rx.recv_timeout(wait) {
            Ok(result) => {
                match result {
                    Ok(result) if result.ok => stream_backend_result(&mut stream, result)?,
                    Ok(result) => {
                        let error = result
                            .error
                            .as_deref()
                            .unwrap_or("backend returned ok=false");
                        send_sse(&mut stream, "error", &backend_error_hint(error))?;
                    }
                    Err(error) => send_sse(&mut stream, "error", &backend_error_hint(&error))?,
                }
                send_sse(&mut stream, "done", "[DONE]")?;
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let elapsed = started.elapsed();
                if elapsed >= fallback_timeout {
                    send_json_fallback_timeout(&mut stream, elapsed, fallback_timeout)?;
                    break;
                }
                send_sse(
                    &mut stream,
                    "heartbeat",
                    &wait_status_message(elapsed.as_secs()),
                )?;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                send_sse(&mut stream, "error", "backend worker disconnected")?;
                send_sse(&mut stream, "done", "[DONE]")?;
                break;
            }
        }
    }

    Ok(())
}

fn backend_prompt_gate_block_reason(config: &Config) -> Option<String> {
    match call_backend_health(&config.backend, config.backend_response_timeout) {
        Ok(health) => backend_prompt_block_reason(&health),
        Err(error) => Some(format!(
            "backend prompt gate failed: {}",
            backend_error_hint(&error)
        )),
    }
}

fn json_fallback_timeout(response_timeout: Duration) -> Duration {
    response_timeout + JSON_FALLBACK_TIMEOUT_GRACE
}

fn backend_stream_error_can_fallback(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("returned http 404")
        || normalized.contains("returned http 405")
        || normalized.contains("returned http 501")
        || normalized.contains("unsupported")
}

fn send_json_fallback_timeout(
    stream: &mut TcpStream,
    elapsed: Duration,
    limit: Duration,
) -> std::io::Result<()> {
    send_sse(
        stream,
        "status",
        &json_fallback_timeout_status(elapsed, limit),
    )?;
    send_sse(stream, "error", &json_fallback_timeout_error(limit))?;
    send_sse(stream, "done", "[DONE]")
}

fn json_fallback_timeout_status(elapsed: Duration, limit: Duration) -> String {
    format!(
        "backend JSON fallback timed out after {}s (limit {}s); the backend request may still be running server-side, but this SSE stream is closing so the page can recover",
        elapsed.as_secs(),
        limit.as_secs()
    )
}

fn json_fallback_timeout_error(limit: Duration) -> String {
    format!(
        "backend JSON fallback timed out after {}s; check backend health or retry with token streaming once the backend is ready",
        limit.as_secs()
    )
}

fn stream_backend_result(stream: &mut TcpStream, result: BackendResult) -> std::io::Result<()> {
    if let Some(runtime_model) = result.runtime_model {
        send_sse(stream, "meta", &format!("runtime_model={runtime_model}"))?;
    }
    if let Some(elapsed_ms) = result.elapsed_ms {
        send_sse(stream, "meta", &format!("backend_elapsed_ms={elapsed_ms}"))?;
    }
    if let Some(passed) = result.business_cycle_passed {
        send_sse(stream, "meta", &format!("business_cycle_passed={passed}"))?;
    }
    if let Some(applied) = result.feedback_applied {
        send_sse(stream, "meta", &format!("feedback_applied={applied}"))?;
    }
    if let Some(passed) = result.rust_check_passed {
        send_sse(stream, "meta", &format!("rust_check_passed={passed}"))?;
    }
    if let Some(passed) = result.self_improve_passed {
        send_sse(stream, "meta", &format!("self_improve_passed={passed}"))?;
    }
    if let Some(raw_answer) = result.raw_answer {
        send_sse(stream, "raw", &raw_answer)?;
    }
    if let Some(enhanced_answer) = result.enhanced_answer {
        send_sse(stream, "enhanced", &enhanced_answer)?;
    }

    for chunk in answer_chunks(&result.answer, 18) {
        send_sse(stream, "delta", &chunk)?;
        thread::sleep(Duration::from_millis(35));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::Shutdown;

    #[test]
    fn json_fallback_timeout_messages_are_actionable() {
        let limit = Duration::from_secs(5);
        let status = json_fallback_timeout_status(Duration::from_secs(7), limit);
        let error = json_fallback_timeout_error(limit);

        assert!(status.contains("backend JSON fallback timed out after 7s"));
        assert!(status.contains("request may still be running server-side"));
        assert!(status.contains("SSE stream is closing"));
        assert!(error.contains("backend JSON fallback timed out after 5s"));
        assert!(error.contains("check backend health"));
    }

    #[test]
    fn backend_stream_runtime_failures_do_not_fallback_to_json() {
        assert!(!backend_stream_error_can_fallback(
            "backend stream truncated: EOF before done or error terminal event"
        ));
        assert!(!backend_stream_error_can_fallback(
            "read backend stream body failed: timed out after 300s"
        ));
        assert!(!backend_stream_error_can_fallback(
            "connect backend failed: connection refused"
        ));
        assert!(backend_stream_error_can_fallback(
            "backend stream returned HTTP 404: unsupported HTTP path"
        ));
    }

    #[test]
    fn chat_stream_marks_backend_eof_after_delta_as_truncated() {
        let backend = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend_addr = backend.local_addr().unwrap();
        let backend_thread = thread::spawn(move || {
            {
                let (mut health_stream, _) = backend.accept().unwrap();
                let health_request = crate::http::read_http_request(&mut health_stream).unwrap();
                assert!(health_request.starts_with("GET /health HTTP/1.1"));
                write_backend_health_ok(&mut health_stream);
                health_stream.shutdown(Shutdown::Both).unwrap();
            }

            let (mut stream, _) = backend.accept().unwrap();
            let request = crate::http::read_http_request(&mut stream).unwrap();
            assert!(request.starts_with("POST /v1/chat-stream HTTP/1.1"));
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n\
event: delta\ndata: partial\n\n",
                )
                .unwrap();
        });

        let lab = TcpListener::bind("127.0.0.1:0").unwrap();
        let lab_addr = lab.local_addr().unwrap();
        let config = Config {
            backend: backend_addr.to_string(),
            ..Config::default()
        };
        let lab_thread = thread::spawn(move || {
            let (stream, _) = lab.accept().unwrap();
            handle_connection(stream, &config).unwrap();
        });

        let body = r#"{"prompt":"hello","endpoint":"chat","messages":[{"role":"user","content":"hello"}]}"#;
        let mut client = TcpStream::connect(lab_addr).unwrap();
        write!(
            client,
            "POST /api/chat-stream HTTP/1.1\r\nhost: {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            lab_addr,
            body.len(),
            body
        )
        .unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        backend_thread.join().unwrap();
        lab_thread.join().unwrap();

        assert!(response.contains("event: delta\ndata: partial"));
        assert!(response.contains("event: error"));
        assert!(response.contains("backend stream truncated"));
        assert!(response.contains("after SSE events"));
        assert!(response.contains("event: done\ndata: [DONE]"));
        assert!(!response.contains("backend stream ended before done; keeping received events"));
    }

    #[test]
    fn chat_stream_blocks_when_backend_prompt_gate_is_busy() {
        let backend = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend_addr = backend.local_addr().unwrap();
        let backend_thread = thread::spawn(move || {
            let (mut stream, _) = backend.accept().unwrap();
            let request = crate::http::read_http_request(&mut stream).unwrap();
            assert!(request.starts_with("GET /health HTTP/1.1"));
            let body = "{\"ok\":true,\"engine_busy\":true,\"active_requests\":[{\"request_id\":99,\"endpoint\":\"chat-stream\",\"elapsed_ms\":45000,\"prompt_preview\":\"already running\"}],\"runtime_mode\":\"gemma-http\",\"gemma_runtime_server\":\"http://127.0.0.1:8686\",\"gemma_runtime_reachable\":true,\"readiness_ok\":true,\"safe_device_ok\":true}";
            write_backend_json(&mut stream, body);
        });

        let lab = TcpListener::bind("127.0.0.1:0").unwrap();
        let lab_addr = lab.local_addr().unwrap();
        let config = Config {
            backend: backend_addr.to_string(),
            ..Config::default()
        };
        let lab_thread = thread::spawn(move || {
            let (stream, _) = lab.accept().unwrap();
            handle_connection(stream, &config).unwrap();
        });

        let body = r#"{"prompt":"hello","endpoint":"chat","messages":[{"role":"user","content":"hello"}]}"#;
        let mut client = TcpStream::connect(lab_addr).unwrap();
        write!(
            client,
            "POST /api/chat-stream HTTP/1.1\r\nhost: {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            lab_addr,
            body.len(),
            body
        )
        .unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        backend_thread.join().unwrap();
        lab_thread.join().unwrap();

        assert!(response.contains("event: status\ndata: proxy connected"));
        assert!(response.contains(
            "event: status\ndata: checking backend prompt gate before forwarding request"
        ));
        assert!(response.contains("event: error"));
        assert!(response.contains("backend engine is busy"));
        assert!(response.contains("#99 chat-stream 45000ms"));
        assert!(response.contains("prompt=\"already running\""));
        assert!(response.contains("event: done\ndata: [DONE]"));
        assert!(!response.contains("calling rust-norion chat"));
    }

    fn write_backend_health_ok(stream: &mut TcpStream) {
        write_backend_json(
            stream,
            "{\"ok\":true,\"engine_busy\":false,\"runtime_mode\":\"gemma-http\",\"gemma_runtime_server\":\"http://127.0.0.1:8686\",\"gemma_runtime_reachable\":true,\"readiness_ok\":true,\"safe_device_ok\":true}",
        );
    }

    fn write_backend_json(stream: &mut TcpStream, body: &str) {
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    }
}
