mod cleanup_audit;
mod config;
mod endpoint;
mod event;
mod final_payload;
mod health;
mod http;
mod hygiene;
pub(crate) mod json;
mod model_pool;
mod repair;
mod request;
mod retrieval;
mod sse;

pub use config::ProviderConfig;
pub use endpoint::StreamEndpoint;
pub use event::StreamEvent;
pub use final_payload::FinalPayloadSummary;
pub use health::ProviderHealth;
pub use http::ForgeProvider;
pub use request::{ChatMessage, StreamRequest};

pub trait StreamProvider {
    fn stream(
        &self,
        request: &StreamRequest,
        on_event: &mut dyn FnMut(StreamEvent) -> Result<(), String>,
    ) -> Result<(), String>;
}
