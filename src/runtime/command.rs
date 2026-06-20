mod args;
mod diagnostics;
mod mistralrs;
mod process;
mod runtime;
mod types;

pub(in crate::runtime) use diagnostics::populate_static_runtime_diagnostics;
pub use runtime::CommandRuntime;
pub use types::{CommandPromptMode, CommandTextOutputFilter, CommandWireFormat};

#[cfg(test)]
pub(in crate::runtime) use mistralrs::{filter_command_text_output, parse_mistralrs_cli_stats};
