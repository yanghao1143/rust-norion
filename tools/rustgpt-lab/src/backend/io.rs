use std::io::{self, ErrorKind};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const BACKEND_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
pub(super) const BACKEND_WRITE_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const BACKEND_STREAM_READ_POLL_INTERVAL: Duration = Duration::from_secs(2);

pub(super) fn connect_backend(
    backend: &str,
    response_timeout: Duration,
) -> Result<TcpStream, String> {
    connect_backend_with_read_timeout(backend, response_timeout)
}

pub(super) fn connect_backend_for_stream(
    backend: &str,
    response_timeout: Duration,
) -> Result<TcpStream, String> {
    connect_backend_with_read_timeout(
        backend,
        std::cmp::min(response_timeout, BACKEND_STREAM_READ_POLL_INTERVAL),
    )
}

fn connect_backend_with_read_timeout(
    backend: &str,
    read_timeout: Duration,
) -> Result<TcpStream, String> {
    let addresses = backend
        .to_socket_addrs()
        .map_err(|error| {
            backend_io_error_message("resolve backend address", error, BACKEND_CONNECT_TIMEOUT)
        })?
        .collect::<Vec<_>>();
    if addresses.is_empty() {
        return Err(format!(
            "resolve backend address failed: {backend} did not resolve to a socket address"
        ));
    }

    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, BACKEND_CONNECT_TIMEOUT) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(read_timeout))
                    .map_err(|error| {
                        backend_io_error_message(
                            "configure backend read timeout",
                            error,
                            read_timeout,
                        )
                    })?;
                stream
                    .set_write_timeout(Some(BACKEND_WRITE_TIMEOUT))
                    .map_err(|error| {
                        backend_io_error_message(
                            "configure backend write timeout",
                            error,
                            BACKEND_WRITE_TIMEOUT,
                        )
                    })?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error
        .map(|error| backend_io_error_message("connect backend", error, BACKEND_CONNECT_TIMEOUT))
        .unwrap_or_else(|| "connect backend failed: no socket addresses available".to_owned()))
}

pub(super) fn backend_io_error_message(
    action: &str,
    error: io::Error,
    timeout: Duration,
) -> String {
    if is_timeout_error(&error) {
        format!(
            "{action} failed: timed out after {}",
            format_timeout_duration(timeout)
        )
    } else {
        format!("{action} failed: {error}")
    }
}

fn format_timeout_duration(timeout: Duration) -> String {
    if timeout.subsec_millis() == 0 && timeout.as_secs() > 0 {
        format!("{}s", timeout.as_secs())
    } else {
        format!("{}ms", timeout.as_millis())
    }
}

fn is_timeout_error(error: &io::Error) -> bool {
    matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)
}
