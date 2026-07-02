use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::development_pollution::DefenseSpacerActivationGate;
use crate::kv_exchange::RuntimeKvBlock;
use crate::reflection::ReasoningStep;
use crate::runtime_manifest::{
    TransformerRuntimeArchitecture, default_transformer_runtime_architecture,
};

use super::{
    args::expand_command_arg,
    diagnostics::populate_static_runtime_diagnostics,
    mistralrs::{
        filter_command_text_output, mistralrs_cli_reported_tokens, mistralrs_cli_stats_trace,
        parse_mistralrs_cli_stats,
    },
    process::wait_for_command_output,
    types::{CommandPromptMode, CommandTextOutputFilter, CommandWireFormat},
};
use crate::runtime::{
    ModelRuntime, RuntimeError, RuntimeMetadata, RuntimeRequest, RuntimeResponse,
    format_runtime_payload, parse_runtime_response_json,
};

#[derive(Debug, Clone)]
pub struct CommandRuntime {
    program: PathBuf,
    args: Vec<String>,
    prompt_mode: CommandPromptMode,
    wire_format: CommandWireFormat,
    text_output_filter: CommandTextOutputFilter,
    timeout_ms: Option<u64>,
    metadata: RuntimeMetadata,
    architecture: Option<TransformerRuntimeArchitecture>,
    imported_kv_blocks: Vec<RuntimeKvBlock>,
    exported_kv_blocks: Vec<RuntimeKvBlock>,
    activation_gate: Option<DefenseSpacerActivationGate>,
}

impl CommandRuntime {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            prompt_mode: CommandPromptMode::Stdin,
            wire_format: CommandWireFormat::Text,
            text_output_filter: CommandTextOutputFilter::None,
            timeout_ms: None,
            metadata: RuntimeMetadata::default(),
            architecture: None,
            imported_kv_blocks: Vec::new(),
            exported_kv_blocks: Vec::new(),
            activation_gate: None,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn prompt_mode(mut self, prompt_mode: CommandPromptMode) -> Self {
        self.prompt_mode = prompt_mode;
        self
    }

    pub fn wire_format(mut self, wire_format: CommandWireFormat) -> Self {
        self.wire_format = wire_format;
        self
    }

    pub fn text_output_filter(mut self, filter: CommandTextOutputFilter) -> Self {
        self.text_output_filter = filter;
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms.max(1));
        self
    }

    pub fn with_metadata(mut self, metadata: RuntimeMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_architecture(mut self, architecture: TransformerRuntimeArchitecture) -> Self {
        self.architecture = Some(architecture);
        self
    }

    pub fn with_activation_gate(mut self, gate: DefenseSpacerActivationGate) -> Self {
        self.activation_gate = Some(gate);
        self
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn command_args(&self) -> &[String] {
        &self.args
    }

    pub fn command_text_output_filter(&self) -> CommandTextOutputFilter {
        self.text_output_filter
    }

    pub fn timeout_ms(&self) -> Option<u64> {
        self.timeout_ms
    }

    pub(in crate::runtime) fn expanded_args(
        &self,
        request: &RuntimeRequest,
        payload: &str,
    ) -> Vec<String> {
        self.args
            .iter()
            .map(|arg| expand_command_arg(arg, request, payload, self.wire_format))
            .collect()
    }
}

impl ModelRuntime for CommandRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        self.metadata.clone()
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        self.architecture.unwrap_or_else(|| {
            default_transformer_runtime_architecture(
                self.metadata.native_context_window,
                self.metadata.embedding_dimensions,
            )
        })
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        self.imported_kv_blocks.clear();
        self.imported_kv_blocks.extend(blocks.iter().cloned());
        Ok(self.imported_kv_blocks.len())
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(self.exported_kv_blocks.clone())
    }

    fn generate(&mut self, mut request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        self.exported_kv_blocks.clear();
        if let Some(gate) = self.activation_gate.as_ref().filter(|gate| !gate.allowed) {
            return Err(RuntimeError::new(format!(
                "runtime command activation blocked: {}",
                gate.summary_line()
            )));
        }
        if request.imported_kv_blocks.is_empty() && !self.imported_kv_blocks.is_empty() {
            request.imported_kv_blocks = std::mem::take(&mut self.imported_kv_blocks);
        } else {
            self.imported_kv_blocks.clear();
        }
        let payload = format_runtime_payload(&request, self.wire_format);
        let mut command = Command::new(&self.program);
        command.args(self.expanded_args(&request, &payload));

        if self.prompt_mode == CommandPromptMode::Stdin {
            command.stdin(Stdio::piped());
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|error| {
            RuntimeError::new(format!(
                "failed to spawn runtime command {}: {error}",
                self.program.display()
            ))
        })?;

        if self.prompt_mode == CommandPromptMode::Stdin {
            let Some(mut stdin) = child.stdin.take() else {
                return Err(RuntimeError::new("runtime command did not expose stdin"));
            };
            stdin.write_all(payload.as_bytes()).map_err(|error| {
                RuntimeError::new(format!("failed to write runtime prompt to stdin: {error}"))
            })?;
        }

        let output = wait_for_command_output(
            child,
            self.timeout_ms.map(Duration::from_millis),
            &self.program,
        )?;
        if !output.status.success() {
            return Err(RuntimeError::new(format!(
                "runtime command exited with status {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut response = match self.wire_format {
            CommandWireFormat::Text => {
                let mut response = RuntimeResponse::new(filter_command_text_output(
                    &stdout,
                    self.text_output_filter,
                ));
                if self.text_output_filter == CommandTextOutputFilter::MistralRsCli
                    && let Some(stats) = parse_mistralrs_cli_stats(&stdout)
                {
                    response.tokens = mistralrs_cli_reported_tokens(stats);
                    response.trace.push(mistralrs_cli_stats_trace(stats));
                }
                response
            }
            CommandWireFormat::Json => parse_runtime_response_json(&stdout)?,
        };
        let architecture = self.architecture();
        populate_static_runtime_diagnostics(
            &mut response.diagnostics,
            &self.metadata,
            architecture,
        );
        self.exported_kv_blocks = response.exported_kv_blocks.clone();
        response.trace.push(ReasoningStep::new(
            "command_runtime",
            format!("executed {}", self.program.display()),
            0.72,
        ));
        Ok(response)
    }
}
