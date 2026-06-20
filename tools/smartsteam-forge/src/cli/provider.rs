use std::time::Duration;

use smartsteam_forge::ProviderConfig;

use super::CliConfig;

pub(crate) fn provider_config(config: &CliConfig) -> ProviderConfig {
    let mut provider_config = ProviderConfig::rust_norion(config.backend.clone());
    if let Some(seconds) = config.request_timeout_secs {
        provider_config.request_timeout = Duration::from_secs(seconds);
    }
    if let Some(milliseconds) = config.connect_timeout_ms {
        provider_config.connect_timeout = Duration::from_millis(milliseconds);
    }
    if let Some(milliseconds) = config.read_timeout_ms {
        provider_config.read_timeout = Duration::from_millis(milliseconds);
    }
    provider_config
}
