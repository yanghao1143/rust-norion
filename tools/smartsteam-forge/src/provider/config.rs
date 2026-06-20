use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub backend: String,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub request_timeout: Duration,
}

impl ProviderConfig {
    pub fn rust_norion(backend: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            connect_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(2),
            request_timeout: Duration::from_secs(900),
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self::rust_norion("127.0.0.1:7878")
    }
}
