mod json_items;
mod json_payload;
mod prompt;

use super::super::{CommandWireFormat, RuntimeRequest};

pub use json_payload::runtime_request_json;
pub(in crate::runtime) use prompt::format_runtime_prompt;

pub(in crate::runtime) fn format_runtime_payload(
    request: &RuntimeRequest,
    wire_format: CommandWireFormat,
) -> String {
    match wire_format {
        CommandWireFormat::Text => format_runtime_prompt(request),
        CommandWireFormat::Json => runtime_request_json(request),
    }
}
