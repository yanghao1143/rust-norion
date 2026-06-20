use std::time::Duration;

pub(crate) const DEFAULT_BACKEND_RESPONSE_TIMEOUT: Duration = Duration::from_secs(900);

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) bind: String,
    pub(crate) backend: String,
    pub(crate) mode: RunMode,
    pub(crate) backend_response_timeout: Duration,
    pub(crate) context_messages: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunMode {
    Server,
    Repl,
    Help,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8787".to_owned(),
            backend: "127.0.0.1:7878".to_owned(),
            mode: RunMode::Server,
            backend_response_timeout: DEFAULT_BACKEND_RESPONSE_TIMEOUT,
            context_messages: 64,
        }
    }
}

pub(crate) fn parse_args<I>(args: I) -> Config
where
    I: IntoIterator<Item = String>,
{
    let mut config = Config::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                config.mode = RunMode::Help;
                return config;
            }
            "--bind" => {
                if let Some(value) = args.next() {
                    config.bind = value;
                }
            }
            "--backend" => {
                if let Some(value) = args.next() {
                    config.backend = trim_http_prefix(&value).to_owned();
                }
            }
            "--repl" | "--interactive" => {
                config.mode = RunMode::Repl;
            }
            "--backend-timeout-secs" | "--timeout-secs" => {
                if let Some(value) = args.next() {
                    if let Some(timeout) = parse_timeout_secs(&value) {
                        config.backend_response_timeout = timeout;
                    }
                }
            }
            "--context-messages" | "--context-window" | "--max-context-messages" => {
                if let Some(value) = args.next() {
                    if let Some(context_messages) = parse_context_messages(&value) {
                        config.context_messages = context_messages;
                    }
                }
            }
            _ => {}
        }
    }
    config
}

pub(crate) fn help_text() -> &'static str {
    "rustgpt-lab: browser Web Lab, streaming proxy, and REPL for rust-norion\n\
\n\
Usage:\n\
  rustgpt-lab [--bind 127.0.0.1:8787] [--backend 127.0.0.1:7878]\n\
  rustgpt-lab --repl --backend 127.0.0.1:7878\n\
\n\
Default server mode starts only the 8787 Web Lab UI/proxy; it expects the 7878\n\
rust-norion backend to already be listening and never starts Gemma/mistralrs.\n\
\n\
Options:\n\
  --bind <addr>                  Web Lab UI/proxy listen address, default 127.0.0.1:8787\n\
  --backend <addr|url>           rust-norion backend, default 127.0.0.1:7878\n\
  --repl, --interactive          run the terminal REPL instead of the Web Lab server\n\
  --backend-timeout-secs <n>     total backend streaming window, clamped to 30..7200s\n\
  --timeout-secs <n>             alias for --backend-timeout-secs\n\
  --context-messages <2..256>    short chat history message count, not tokens, default 64\n\
  --context-window <2..256>      alias for --context-messages\n\
  --max-context-messages <n>     alias for --context-messages\n\
  --help, -h                     print this help and exit without starting services\n\
\n\
Token terms:\n\
  context flags count short chat history messages, not tokens\n\
  max_tokens is the request generation budget; Gemma n_ctx is runtime context length\n\
\n\
Port map:\n\
  127.0.0.1:7878  rust-norion model-service backend\n\
  127.0.0.1:8787  rustgpt-lab Web UI and local SSE proxy\n\
  127.0.0.1:8686  optional Gemma/mistralrs runtime behind rust-norion\n"
}

fn parse_timeout_secs(value: &str) -> Option<Duration> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .map(|seconds| Duration::from_secs(seconds.clamp(30, 7200)))
}

fn parse_context_messages(value: &str) -> Option<usize> {
    value
        .trim()
        .parse::<usize>()
        .ok()
        .map(|messages| messages.clamp(2, 256))
}

fn trim_http_prefix(value: &str) -> &str {
    value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_backend_url_without_scheme() {
        let config = parse_args(["--backend".to_owned(), "http://127.0.0.1:7878".to_owned()]);
        assert_eq!(config.backend, "127.0.0.1:7878");
    }

    #[test]
    fn parses_repl_mode() {
        let config = parse_args(["--repl".to_owned()]);

        assert_eq!(config.mode, RunMode::Repl);
    }

    #[test]
    fn parses_help_mode() {
        let config = parse_args(["--help".to_owned()]);

        assert_eq!(config.mode, RunMode::Help);
    }

    #[test]
    fn help_mode_overrides_later_starting_flags() {
        let config = parse_args([
            "--help".to_owned(),
            "--repl".to_owned(),
            "--backend".to_owned(),
            "127.0.0.1:7878".to_owned(),
        ]);

        assert_eq!(config.mode, RunMode::Help);
    }

    #[test]
    fn help_mode_overrides_earlier_starting_flags() {
        let config = parse_args(["--repl".to_owned(), "--help".to_owned()]);

        assert_eq!(config.mode, RunMode::Help);
    }

    #[test]
    fn help_text_documents_non_starting_mode() {
        let help = help_text();

        assert!(help.contains("--help, -h"));
        assert!(help.contains("exit without starting services"));
        assert!(help.contains("starts only the 8787 Web Lab UI/proxy"));
        assert!(help.contains("never starts Gemma/mistralrs"));
        assert!(help.contains("short chat history message count, not tokens"));
        assert!(help.contains("context flags count short chat history messages, not tokens"));
        assert!(help.contains("max_tokens is the request generation budget"));
        assert!(help.contains("Gemma n_ctx is runtime context length"));
        assert!(help.contains("--bind 127.0.0.1:8787"));
        assert!(help.contains("--backend 127.0.0.1:7878"));
    }

    #[test]
    fn parses_backend_timeout_secs() {
        let config = parse_args(["--backend-timeout-secs".to_owned(), "1800".to_owned()]);

        assert_eq!(config.backend_response_timeout, Duration::from_secs(1800));
    }

    #[test]
    fn parses_backend_timeout_secs_alias() {
        let config = parse_args(["--timeout-secs".to_owned(), "1800".to_owned()]);

        assert_eq!(config.backend_response_timeout, Duration::from_secs(1800));
    }

    #[test]
    fn clamps_backend_timeout_secs_to_safe_range() {
        let short = parse_args(["--timeout-secs".to_owned(), "1".to_owned()]);
        let long = parse_args(["--timeout-secs".to_owned(), "99999".to_owned()]);

        assert_eq!(short.backend_response_timeout, Duration::from_secs(30));
        assert_eq!(long.backend_response_timeout, Duration::from_secs(7200));
    }

    #[test]
    fn parses_context_messages() {
        let config = parse_args(["--context-messages".to_owned(), "128".to_owned()]);

        assert_eq!(config.context_messages, 128);
    }

    #[test]
    fn parses_context_message_alias() {
        let config = parse_args(["--max-context-messages".to_owned(), "128".to_owned()]);

        assert_eq!(config.context_messages, 128);
    }

    #[test]
    fn clamps_context_messages_to_lab_range() {
        let short = parse_args(["--context-window".to_owned(), "1".to_owned()]);
        let long = parse_args(["--context-window".to_owned(), "999".to_owned()]);

        assert_eq!(short.context_messages, 2);
        assert_eq!(long.context_messages, 256);
    }
}
