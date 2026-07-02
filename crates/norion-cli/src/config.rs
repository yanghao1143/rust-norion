use norion_service::{
    ChatSessionConfig, ModelEndpoint, ModelEndpointSelectionKind, ModelRole, RoutingIntent,
    RoutingPreference,
};

use crate::CliInputConfig;
use crate::input::InputRouteOptionsSnapshot;

const LOCAL_COMMANDS: &str = "/status|/state /workers|/worker-status|/endpoints /role ROLE /prefer fast|quality|balanced /endpoint WORKER|auto /worker WORKER|auto /model ROLE [PREFERENCE] [ENDPOINT|auto] /max-tokens N|auto|off /history-limit N";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliRuntimeConfig {
    pub input: CliInputConfig,
    pub session: ChatSessionConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliStartupSnapshot {
    pub banner: String,
    pub route: String,
    pub routing_intent: RoutingIntent,
    pub model_role_label: String,
    pub routing_preference_label: String,
    pub endpoint_label: String,
    pub endpoint_pinned: bool,
    pub endpoint_kind: ModelEndpointSelectionKind,
    pub endpoint_kind_label: String,
    pub endpoint_auto: bool,
    pub endpoint_built_in: bool,
    pub endpoint_custom: bool,
    pub wire_model_role_label: String,
    pub wire_routing_preference_label: String,
    pub wire_prefer_fast: bool,
    pub wire_prefer_quality: bool,
    pub wire_sends_max_tokens: bool,
    pub wire_max_tokens: Option<usize>,
    pub wire_endpoint_pinned: bool,
    pub wire_endpoint_kind_label: String,
    pub wire_sends_model_endpoint: bool,
    pub wire_model_endpoint_label: Option<String>,
    pub history_limit: usize,
    pub max_tokens: Option<usize>,
    pub max_tokens_label: String,
    pub route_options: InputRouteOptionsSnapshot,
    pub local_commands: String,
}

impl Default for CliRuntimeConfig {
    fn default() -> Self {
        Self {
            input: CliInputConfig::default(),
            session: ChatSessionConfig::default(),
        }
    }
}

impl CliRuntimeConfig {
    pub fn startup_snapshot(&self) -> CliStartupSnapshot {
        let routing_intent = self.input.routing_intent();
        let route_options = InputRouteOptionsSnapshot::from_intent(&routing_intent);
        let endpoint_kind = routing_intent.endpoint_kind();
        let route_wire = routing_intent.wire_snapshot();
        let max_tokens = self.session.default_max_tokens;
        CliStartupSnapshot {
            banner: "norion-cli protocol shell".to_owned(),
            route: routing_intent.summary(),
            model_role_label: routing_intent.model_role_label().to_owned(),
            routing_preference_label: routing_intent.routing_preference_label().to_owned(),
            endpoint_label: routing_intent.endpoint_label().to_owned(),
            endpoint_pinned: routing_intent.endpoint_pinned,
            endpoint_kind,
            endpoint_kind_label: routing_intent.endpoint_kind_label().to_owned(),
            endpoint_auto: routing_intent.endpoint_auto(),
            endpoint_built_in: routing_intent.endpoint_built_in(),
            endpoint_custom: routing_intent.endpoint_custom(),
            wire_model_role_label: route_wire.model_role_label,
            wire_routing_preference_label: route_wire.routing_preference_label,
            wire_prefer_fast: route_wire.prefer_fast,
            wire_prefer_quality: route_wire.prefer_quality,
            wire_sends_max_tokens: max_tokens.is_some(),
            wire_max_tokens: max_tokens,
            wire_endpoint_pinned: route_wire.endpoint_pinned,
            wire_endpoint_kind_label: route_wire.endpoint_kind_label,
            wire_sends_model_endpoint: route_wire.sends_model_endpoint,
            wire_model_endpoint_label: route_wire.model_endpoint_label,
            routing_intent,
            history_limit: self.session.history_limit,
            max_tokens,
            max_tokens_label: max_tokens
                .map(|value| value.to_string())
                .unwrap_or_else(|| "backend-default".to_owned()),
            route_options,
            local_commands: LOCAL_COMMANDS.to_owned(),
        }
    }

    pub fn startup_lines(&self) -> Vec<String> {
        self.startup_snapshot().lines()
    }
}

impl CliStartupSnapshot {
    pub fn lines(&self) -> Vec<String> {
        vec![
            self.banner.clone(),
            self.route.clone(),
            format!(
                "history_limit={} max_tokens={}",
                self.history_limit, self.max_tokens_label
            ),
            format!("local_commands={}", self.local_commands),
        ]
    }
}

pub fn parse_cli_args<I, S>(args: I) -> Result<CliRuntimeConfig, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut config = CliRuntimeConfig::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        let arg = arg.as_ref().to_owned();
        let (name, inline_value) = split_option(&arg)?;
        match name {
            "--role" => {
                let value = option_value(name, inline_value, &mut args)?;
                config.input.model_role = ModelRole::from_label(&value)
                    .ok_or_else(|| format!("unknown model role: {value}"))?;
            }
            "--prefer" | "--preference" => {
                let value = option_value(name, inline_value, &mut args)?;
                config.input.routing_preference = RoutingPreference::from_label(&value)
                    .ok_or_else(|| format!("unknown routing preference: {value}"))?;
            }
            "--endpoint" | "--worker" => {
                let value = option_value(name, inline_value, &mut args)?;
                config.input.model_endpoint = ModelEndpoint::from_label(&value);
            }
            "--max-tokens" => {
                let value = option_value(name, inline_value, &mut args)?;
                config.session.default_max_tokens = parse_optional_max_tokens(name, &value)?;
            }
            "--history-limit" => {
                let value = option_value(name, inline_value, &mut args)?;
                config.session.history_limit = parse_positive_usize(name, &value)?;
            }
            "--help" | "-h" => return Err(help_text()),
            _ => return Err(format!("unknown option: {name}")),
        }
    }

    Ok(config)
}

pub fn help_text() -> String {
    [
        "usage: norion-cli [--role ROLE] [--prefer fast|quality|balanced]",
        "                  [--endpoint WORKER|auto] [--worker WORKER|auto]",
        "                  [--max-tokens N|auto|off]",
        "                  [--history-limit N]",
        "       norion-cli evidence-packet --issue N --commit SHA --command CMD --gate passed|failed|blocked --input PATH [--git-worktree PATH] [--release-review-input PATH] [--issue-state-input PATH] [--demo-proof-input PATH] [--roundtrip-proof-input PATH] [--issue30-context-input PATH] [--output PATH] [--require TEXT] [--reject TEXT]",
        "",
        "no-backend mode: prints a local protocol snapshot only; it does not start Gemma, connect to a backend, or submit prompts",
        "evidence-packet prints or writes a deterministic redacted GitHub issue comment and can require or reject packet fields",
        "routing defaults to endpoint=auto pinned=false unless --endpoint or --worker selects a worker",
        "roles: assistant|reviewer|summarizer|tester",
        "preferences: balanced|fast|quality; endpoint auto|default|none clears a worker pin",
        "local commands: /status|/state, /workers|/worker-status|/endpoints, /role ROLE, /prefer fast|quality|balanced, /endpoint WORKER|auto, /worker WORKER|auto, /model ROLE [PREFERENCE] [ENDPOINT|auto], /max-tokens N|auto|off, /history-limit N",
    ]
    .join("\n")
}

fn split_option(arg: &str) -> Result<(&str, Option<String>), String> {
    if !arg.starts_with('-') {
        return Err(format!("unexpected positional argument: {arg}"));
    }
    Ok(arg
        .split_once('=')
        .map(|(name, value)| (name, Some(value.to_owned())))
        .unwrap_or((arg, None)))
}

fn option_value<I, S>(
    name: &str,
    inline_value: Option<String>,
    args: &mut I,
) -> Result<String, String>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    if let Some(value) = inline_value {
        return Ok(value);
    }
    args.next()
        .map(|value| value.as_ref().to_owned())
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| format!("missing value for {name}"))
}

fn parse_positive_usize(name: &str, value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map(|value| value.max(1))
        .map_err(|_| format!("{name} must be a positive integer"))
}

fn parse_optional_max_tokens(name: &str, value: &str) -> Result<Option<usize>, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" | "default" | "backend" | "none" | "off" => Ok(None),
        _ => parse_positive_usize(name, value).map(Some),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CliInput, InputAction, KeyInput};
    use norion_service::{ChatRole, ChatSession, request_json};

    #[test]
    fn default_cli_args_do_not_pin_worker() {
        let config = parse_cli_args(Vec::<&str>::new()).expect("expected default config");

        assert_eq!(config.input.model_role, ModelRole::Assistant);
        assert_eq!(config.input.routing_preference, RoutingPreference::Balanced);
        assert_eq!(config.input.model_endpoint, None);
        assert_eq!(config.session.default_max_tokens, None);
    }

    #[test]
    fn cli_args_set_routing_and_session_limits() {
        let config = parse_cli_args([
            "--role",
            "reviewer",
            "--prefer=fast",
            "--endpoint",
            "fast-reviewer",
            "--max-tokens",
            "8192",
            "--history-limit=32",
        ])
        .expect("expected parsed config");

        assert_eq!(config.input.model_role, ModelRole::Reviewer);
        assert_eq!(
            config.input.routing_preference,
            RoutingPreference::PreferFast
        );
        assert_eq!(
            config.input.model_endpoint,
            Some(ModelEndpoint::FastReviewer)
        );
        assert_eq!(config.session.default_max_tokens, Some(8192));
        assert_eq!(config.session.history_limit, 32);
    }

    #[test]
    fn prefer_quality_does_not_pin_endpoint_without_operator_endpoint() {
        let config = parse_cli_args(["--prefer", "quality"]).expect("expected parsed preference");

        assert_eq!(
            config.input.routing_preference,
            RoutingPreference::PreferQuality
        );
        assert_eq!(config.input.model_endpoint, None);
    }

    #[test]
    fn cli_flags_flow_into_enter_request_without_pinning_auto_route() {
        let config = parse_cli_args([
            "--role",
            "reviewer",
            "--prefer",
            "fast",
            "--endpoint",
            "auto",
            "--max-tokens",
            "8192",
            "--history-limit",
            "4",
        ])
        .expect("expected parsed config");
        let mut session = ChatSession::new("cli", config.session);
        session.record_user("first");
        session.record_assistant("answer");
        let mut input = CliInput::new(config.input);
        for ch in "review this patch".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let action = input.handle_key(KeyInput::Enter, &session);

        let InputAction::Send(request) = action else {
            panic!("expected send action");
        };
        let roles = request
            .messages
            .iter()
            .map(|message| message.role)
            .collect::<Vec<_>>();
        let contents = request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            roles,
            vec![ChatRole::User, ChatRole::Assistant, ChatRole::User]
        );
        assert_eq!(contents, vec!["first", "answer", "review this patch"]);
        assert_eq!(request.max_tokens, Some(8192));
        assert_eq!(request.model_role, ModelRole::Reviewer);
        assert_eq!(request.routing_preference, RoutingPreference::PreferFast);
        assert_eq!(request.model_endpoint, None);
        assert!(!request.endpoint_pinned());
        assert!(input.buffer().is_empty());
    }

    #[test]
    fn cli_flags_flow_into_request_json_with_explicit_route_metadata() {
        let config = parse_cli_args([
            "--role",
            "reviewer",
            "--prefer",
            "fast",
            "--endpoint",
            "auto",
            "--max-tokens",
            "8192",
        ])
        .expect("expected parsed config");
        let session = ChatSession::new("cli", config.session.clone());
        let mut input = CliInput::new(config.input);
        for ch in "review this patch".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let InputAction::Send(auto_request) = input.handle_key(KeyInput::Enter, &session) else {
            panic!("expected send action");
        };
        let auto_json = request_json(&auto_request);

        assert!(auto_json.contains("\"model_role\":\"reviewer\""));
        assert!(auto_json.contains("\"routing_preference\":\"prefer_fast\""));
        assert!(auto_json.contains("\"prefer_fast\":true"));
        assert!(auto_json.contains("\"max_tokens\":8192"));
        assert!(auto_json.contains("\"endpoint_pinned\":false"));
        assert!(auto_json.contains("\"endpoint_kind\":\"auto\""));
        assert!(!auto_json.contains("\"model_endpoint\""));

        let pinned = parse_cli_args([
            "--role",
            "reviewer",
            "--prefer",
            "fast",
            "--worker",
            "mlx-reviewer-8b",
            "--max-tokens",
            "8192",
            "--max-tokens",
            "off",
        ])
        .expect("expected pinned config");
        let session = ChatSession::new("cli", pinned.session.clone());
        let mut input = CliInput::new(pinned.input);
        for ch in "review this patch".chars() {
            input.handle_key(KeyInput::Char(ch), &session);
        }

        let InputAction::Send(pinned_request) = input.handle_key(KeyInput::Enter, &session) else {
            panic!("expected pinned send action");
        };
        let pinned_json = request_json(&pinned_request);

        assert!(pinned_json.contains("\"endpoint_pinned\":true"));
        assert!(pinned_json.contains("\"endpoint_kind\":\"custom\""));
        assert!(pinned_json.contains("\"model_endpoint\":\"mlx-reviewer-8b\""));
        assert!(!pinned_json.contains("\"max_tokens\""));
    }

    #[test]
    fn endpoint_auto_clears_pinned_worker_from_flags() {
        let config = parse_cli_args(["--endpoint", "auto"]).expect("expected auto endpoint");

        assert_eq!(config.input.model_endpoint, None);
    }

    #[test]
    fn worker_auto_clears_pinned_worker_from_flags() {
        let config = parse_cli_args(["--endpoint", "fast-reviewer", "--worker", "auto"])
            .expect("expected worker auto endpoint");

        assert_eq!(config.input.model_endpoint, None);
        assert_eq!(
            config.input.routing_summary(),
            "role=assistant preference=balanced endpoint=auto pinned=false"
        );
    }

    #[test]
    fn endpoint_flag_accepts_custom_worker_label() {
        let config =
            parse_cli_args(["--endpoint", "mlx-reviewer-8b"]).expect("expected custom endpoint");

        assert_eq!(
            config.input.model_endpoint,
            Some(ModelEndpoint::Worker("mlx-reviewer-8b".to_owned()))
        );
        assert_eq!(
            config.input.routing_summary(),
            "role=assistant preference=balanced endpoint=mlx-reviewer-8b pinned=true"
        );
    }

    #[test]
    fn max_tokens_auto_clears_cli_token_budget() {
        let config = parse_cli_args(["--max-tokens", "8192", "--max-tokens=off"])
            .expect("expected backend-default max tokens");

        assert_eq!(config.session.default_max_tokens, None);
        assert!(config.startup_lines()[2].contains("max_tokens=backend-default"));
    }

    #[test]
    fn startup_lines_explain_auto_route_and_local_commands() {
        let config = parse_cli_args(["--prefer", "quality"]).expect("expected config");
        let startup = config.startup_snapshot();

        assert_eq!(startup.banner, "norion-cli protocol shell");
        assert_eq!(startup.model_role_label, "assistant");
        assert_eq!(startup.routing_preference_label, "prefer_quality");
        assert_eq!(startup.endpoint_label, "auto");
        assert!(!startup.endpoint_pinned);
        assert_eq!(startup.endpoint_kind, ModelEndpointSelectionKind::Auto);
        assert_eq!(startup.endpoint_kind_label, "auto");
        assert!(startup.endpoint_auto);
        assert!(!startup.endpoint_built_in);
        assert!(!startup.endpoint_custom);
        assert_eq!(startup.wire_model_role_label, "assistant");
        assert_eq!(startup.wire_routing_preference_label, "prefer_quality");
        assert!(!startup.wire_prefer_fast);
        assert!(startup.wire_prefer_quality);
        assert!(!startup.wire_sends_max_tokens);
        assert_eq!(startup.wire_max_tokens, None);
        assert!(!startup.wire_endpoint_pinned);
        assert_eq!(startup.wire_endpoint_kind_label, "auto");
        assert!(!startup.wire_sends_model_endpoint);
        assert_eq!(startup.wire_model_endpoint_label, None);
        assert_eq!(startup.history_limit, 64);
        assert_eq!(startup.max_tokens, None);
        assert_eq!(startup.max_tokens_label, "backend-default");
        assert_eq!(startup.route_options.selected_role_label, "assistant");
        assert_eq!(
            startup.route_options.selected_preference_label,
            "prefer_quality"
        );
        assert_eq!(startup.route_options.selected_endpoint_label, "auto");
        assert_eq!(startup.route_options.selected_endpoint_kind_label, "auto");
        assert!(startup.route_options.selected_endpoint_auto);
        assert!(!startup.route_options.selected_endpoint_built_in);
        assert!(!startup.route_options.selected_endpoint_custom);
        assert!(!startup.route_options.endpoint_pinned);
        assert_eq!(
            startup.route_options.selected_wire_model_role_label,
            "assistant"
        );
        assert_eq!(
            startup.route_options.selected_wire_routing_preference_label,
            "prefer_quality"
        );
        assert!(!startup.route_options.selected_wire_prefer_fast);
        assert!(startup.route_options.selected_wire_prefer_quality);
        assert!(!startup.route_options.selected_wire_endpoint_pinned);
        assert_eq!(
            startup.route_options.selected_wire_endpoint_kind_label,
            "auto"
        );
        assert!(!startup.route_options.selected_wire_sends_model_endpoint);
        assert_eq!(
            startup.route_options.selected_wire_model_endpoint_label,
            None
        );
        assert_eq!(
            startup.route_options.role_labels,
            vec!["assistant", "reviewer", "summarizer", "tester"]
        );
        assert_eq!(
            startup
                .route_options
                .role_options
                .iter()
                .filter(|option| option.selected)
                .map(|option| option.role_label.as_str())
                .collect::<Vec<_>>(),
            vec!["assistant"]
        );
        let reviewer_option = startup
            .route_options
            .role_options
            .iter()
            .find(|option| option.role == ModelRole::Reviewer)
            .expect("reviewer role option should be present");
        assert_eq!(
            reviewer_option.selection_summary,
            "role=reviewer preference=prefer_quality endpoint=auto pinned=false"
        );
        assert_eq!(reviewer_option.selection_wire_model_role_label, "reviewer");
        assert!(reviewer_option.selection_wire_prefer_quality);
        assert!(!reviewer_option.selection_wire_endpoint_pinned);
        assert!(!reviewer_option.selection_wire_sends_model_endpoint);
        assert_eq!(
            startup.route_options.preference_labels,
            vec!["balanced", "prefer_fast", "prefer_quality"]
        );
        assert_eq!(
            startup
                .route_options
                .preference_options
                .iter()
                .filter(|option| option.selected)
                .map(|option| option.preference_label.as_str())
                .collect::<Vec<_>>(),
            vec!["prefer_quality"]
        );
        let fast_option = startup
            .route_options
            .preference_options
            .iter()
            .find(|option| option.preference == RoutingPreference::PreferFast)
            .expect("prefer-fast option should be present");
        assert_eq!(
            fast_option.selection_summary,
            "role=assistant preference=prefer_fast endpoint=auto pinned=false"
        );
        assert!(fast_option.selection_wire_prefer_fast);
        assert!(!fast_option.selection_wire_prefer_quality);
        assert!(!fast_option.selection_wire_endpoint_pinned);
        assert!(!fast_option.selection_wire_sends_model_endpoint);
        assert_eq!(
            startup.route_options.built_in_endpoint_labels,
            vec!["quality-12b", "fast-reviewer", "summary-tester"]
        );
        assert_eq!(startup.route_options.auto_endpoint_label, "auto");
        assert!(startup.local_commands.contains("/model ROLE"));
        assert_eq!(startup.lines(), config.startup_lines());
        assert_eq!(
            config.startup_lines(),
            vec![
                "norion-cli protocol shell".to_owned(),
                "role=assistant preference=prefer_quality endpoint=auto pinned=false".to_owned(),
                "history_limit=64 max_tokens=backend-default".to_owned(),
                format!("local_commands={LOCAL_COMMANDS}"),
            ]
        );
    }

    #[test]
    fn startup_lines_show_operator_worker_pin_and_session_policy() {
        let config = parse_cli_args([
            "--role",
            "reviewer",
            "--prefer",
            "fast",
            "--worker",
            "mlx-reviewer-8b",
            "--max-tokens",
            "8192",
            "--history-limit",
            "16",
        ])
        .expect("expected pinned worker config");
        let startup = config.startup_snapshot();

        assert_eq!(startup.model_role_label, "reviewer");
        assert_eq!(startup.routing_preference_label, "prefer_fast");
        assert_eq!(startup.endpoint_label, "mlx-reviewer-8b");
        assert!(startup.endpoint_pinned);
        assert_eq!(startup.endpoint_kind, ModelEndpointSelectionKind::Custom);
        assert_eq!(startup.endpoint_kind_label, "custom");
        assert!(!startup.endpoint_auto);
        assert!(!startup.endpoint_built_in);
        assert!(startup.endpoint_custom);
        assert_eq!(startup.wire_model_role_label, "reviewer");
        assert_eq!(startup.wire_routing_preference_label, "prefer_fast");
        assert!(startup.wire_prefer_fast);
        assert!(!startup.wire_prefer_quality);
        assert!(startup.wire_sends_max_tokens);
        assert_eq!(startup.wire_max_tokens, Some(8192));
        assert!(startup.wire_endpoint_pinned);
        assert_eq!(startup.wire_endpoint_kind_label, "custom");
        assert!(startup.wire_sends_model_endpoint);
        assert_eq!(
            startup.wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-8b")
        );
        assert_eq!(startup.history_limit, 16);
        assert_eq!(startup.max_tokens, Some(8192));
        assert_eq!(startup.max_tokens_label, "8192");
        assert_eq!(
            startup.route_options.selected_endpoint_label,
            "mlx-reviewer-8b"
        );
        assert_eq!(startup.route_options.selected_endpoint_kind_label, "custom");
        assert!(!startup.route_options.selected_endpoint_auto);
        assert!(!startup.route_options.selected_endpoint_built_in);
        assert!(startup.route_options.selected_endpoint_custom);
        assert!(startup.route_options.endpoint_pinned);
        assert_eq!(
            startup.route_options.selected_wire_model_role_label,
            "reviewer"
        );
        assert_eq!(
            startup.route_options.selected_wire_routing_preference_label,
            "prefer_fast"
        );
        assert!(startup.route_options.selected_wire_prefer_fast);
        assert!(!startup.route_options.selected_wire_prefer_quality);
        assert!(startup.route_options.selected_wire_endpoint_pinned);
        assert_eq!(
            startup.route_options.selected_wire_endpoint_kind_label,
            "custom"
        );
        assert!(startup.route_options.selected_wire_sends_model_endpoint);
        assert_eq!(
            startup
                .route_options
                .selected_wire_model_endpoint_label
                .as_deref(),
            Some("mlx-reviewer-8b")
        );
        let tester_option = startup
            .route_options
            .role_options
            .iter()
            .find(|option| option.role == ModelRole::Tester)
            .expect("tester role option should be present");
        assert_eq!(
            tester_option.selection_summary,
            "role=tester preference=prefer_fast endpoint=mlx-reviewer-8b pinned=true"
        );
        assert_eq!(tester_option.selection_wire_model_role_label, "tester");
        assert!(tester_option.selection_wire_prefer_fast);
        assert!(tester_option.selection_wire_endpoint_pinned);
        assert_eq!(tester_option.selection_wire_endpoint_kind_label, "custom");
        assert!(tester_option.selection_wire_sends_model_endpoint);
        assert_eq!(
            tester_option.selection_wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-8b")
        );
        let quality_option = startup
            .route_options
            .preference_options
            .iter()
            .find(|option| option.preference == RoutingPreference::PreferQuality)
            .expect("prefer-quality option should be present");
        assert_eq!(
            quality_option.selection_summary,
            "role=reviewer preference=prefer_quality endpoint=mlx-reviewer-8b pinned=true"
        );
        assert_eq!(
            quality_option.selection_wire_routing_preference_label,
            "prefer_quality"
        );
        assert!(quality_option.selection_wire_prefer_quality);
        assert!(quality_option.selection_wire_endpoint_pinned);
        assert_eq!(quality_option.selection_wire_endpoint_kind_label, "custom");
        assert!(quality_option.selection_wire_sends_model_endpoint);
        assert_eq!(
            quality_option
                .selection_wire_model_endpoint_label
                .as_deref(),
            Some("mlx-reviewer-8b")
        );
        assert_eq!(
            config.startup_lines(),
            vec![
                "norion-cli protocol shell".to_owned(),
                "role=reviewer preference=prefer_fast endpoint=mlx-reviewer-8b pinned=true"
                    .to_owned(),
                "history_limit=16 max_tokens=8192".to_owned(),
                format!("local_commands={LOCAL_COMMANDS}"),
            ]
        );
    }

    #[test]
    fn startup_snapshot_keeps_worker_pin_distinct_from_backend_default_tokens() {
        let config = parse_cli_args([
            "--role",
            "reviewer",
            "--prefer",
            "fast",
            "--worker",
            "mlx-reviewer-8b",
            "--max-tokens",
            "8192",
            "--max-tokens",
            "off",
        ])
        .expect("expected pinned worker with backend-default tokens");
        let startup = config.startup_snapshot();

        assert_eq!(
            startup.route,
            "role=reviewer preference=prefer_fast endpoint=mlx-reviewer-8b pinned=true"
        );
        assert_eq!(startup.endpoint_label, "mlx-reviewer-8b");
        assert!(startup.endpoint_pinned);
        assert_eq!(startup.endpoint_kind, ModelEndpointSelectionKind::Custom);
        assert_eq!(startup.endpoint_kind_label, "custom");
        assert!(startup.wire_endpoint_pinned);
        assert_eq!(startup.wire_endpoint_kind_label, "custom");
        assert!(startup.wire_sends_model_endpoint);
        assert_eq!(
            startup.wire_model_endpoint_label.as_deref(),
            Some("mlx-reviewer-8b")
        );
        assert_eq!(startup.max_tokens, None);
        assert_eq!(startup.max_tokens_label, "backend-default");
        assert!(!startup.wire_sends_max_tokens);
        assert_eq!(startup.wire_max_tokens, None);
        assert_eq!(startup.history_limit, 64);
        assert_eq!(
            startup
                .route_options
                .selected_wire_model_endpoint_label
                .as_deref(),
            Some("mlx-reviewer-8b")
        );
        assert!(startup.route_options.selected_wire_endpoint_pinned);
        assert!(startup.route_options.selected_wire_sends_model_endpoint);
        assert_eq!(
            config.startup_lines(),
            vec![
                "norion-cli protocol shell".to_owned(),
                "role=reviewer preference=prefer_fast endpoint=mlx-reviewer-8b pinned=true"
                    .to_owned(),
                "history_limit=64 max_tokens=backend-default".to_owned(),
                format!("local_commands={LOCAL_COMMANDS}"),
            ]
        );
    }

    #[test]
    fn help_text_documents_worker_pinning_boundary() {
        let help = help_text();

        assert!(help.contains("--endpoint WORKER|auto"));
        assert!(help.contains("--worker WORKER|auto"));
        assert!(help.contains("--max-tokens N|auto|off"));
        assert!(help.contains("no-backend mode"));
        assert!(help.contains("does not start Gemma"));
        assert!(help.contains("connect to a backend"));
        assert!(help.contains("submit prompts"));
        assert!(help.contains("endpoint=auto pinned=false unless --endpoint or --worker"));
        assert!(help.contains("roles: assistant|reviewer|summarizer|tester"));
        assert!(help.contains("preferences: balanced|fast|quality"));
        assert!(help.contains("endpoint auto|default|none clears a worker pin"));
        assert!(help.contains("/status|/state"));
        assert!(help.contains("/workers"));
        assert!(help.contains("/worker-status"));
        assert!(help.contains("/endpoints"));
        assert!(help.contains("/role ROLE"));
        assert!(help.contains("/prefer fast|quality|balanced"));
        assert!(help.contains("/worker WORKER|auto"));
        assert!(help.contains("/model ROLE [PREFERENCE] [ENDPOINT|auto]"));
        assert!(help.contains("/max-tokens N|auto|off"));
        assert!(help.contains("/history-limit N"));
    }
}
