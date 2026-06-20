use std::ffi::OsString;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvolutionDaemonAction {
    Start,
    Stop,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EvolutionDaemonStartOptions {
    pub interval_secs: Option<u64>,
    pub max_tokens: Option<u64>,
    pub max_total_tokens: Option<u64>,
    pub max_runtime_secs: Option<u64>,
    pub max_failures: Option<u64>,
    pub max_no_feedback_rounds: Option<u64>,
    pub timeout_secs: Option<u64>,
}

pub(super) fn daemon_control_args(
    action: EvolutionDaemonAction,
    work_dir: &str,
    backend: Option<&str>,
    prompt: Option<&str>,
    check_only: bool,
    start_options: EvolutionDaemonStartOptions,
) -> Vec<OsString> {
    let mut args = vec![OsString::from(match action {
        EvolutionDaemonAction::Start => "-Start",
        EvolutionDaemonAction::Stop => "-Stop",
    })];
    args.push(OsString::from("-WorkDir"));
    args.push(OsString::from(work_dir));
    if check_only {
        args.push(OsString::from("-CheckOnly"));
    }
    if action == EvolutionDaemonAction::Start {
        if let Some(backend) = non_empty_value(backend) {
            args.push(OsString::from("-Backend"));
            args.push(OsString::from(backend));
        }
        if let Some(prompt) = non_empty_value(prompt) {
            args.push(OsString::from("-Prompt"));
            args.push(OsString::from(prompt));
        }
        push_daemon_numeric_arg(&mut args, "-IntervalSecs", start_options.interval_secs);
        push_daemon_numeric_arg(&mut args, "-MaxTokens", start_options.max_tokens);
        push_daemon_numeric_arg(&mut args, "-MaxTotalTokens", start_options.max_total_tokens);
        push_daemon_numeric_arg(&mut args, "-MaxRuntimeSecs", start_options.max_runtime_secs);
        push_daemon_numeric_arg(&mut args, "-MaxFailures", start_options.max_failures);
        push_daemon_numeric_arg(
            &mut args,
            "-MaxNoFeedbackRounds",
            start_options.max_no_feedback_rounds,
        );
        push_daemon_numeric_arg(&mut args, "-TimeoutSecs", start_options.timeout_secs);
    }
    args
}

pub(super) fn evolution_action_name(action: EvolutionDaemonAction) -> &'static str {
    match action {
        EvolutionDaemonAction::Start => "start",
        EvolutionDaemonAction::Stop => "stop",
    }
}

fn push_daemon_numeric_arg(args: &mut Vec<OsString>, flag: &str, value: Option<u64>) {
    if let Some(value) = value {
        args.push(OsString::from(flag));
        args.push(OsString::from(value.to_string()));
    }
}

fn non_empty_value(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_as_strings(args: Vec<OsString>) -> Vec<String> {
        args.iter()
            .map(|value| value.to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn start_check_args_include_backend_and_prompt_when_provided() {
        let args = args_as_strings(daemon_control_args(
            EvolutionDaemonAction::Start,
            "target\\evolution\\daemon",
            Some("127.0.0.1:7979"),
            Some("short daemon check"),
            true,
            EvolutionDaemonStartOptions::default(),
        ));

        assert_eq!(args[0], "-Start");
        assert!(args.contains(&"-CheckOnly".to_owned()));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-WorkDir", "target\\evolution\\daemon"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-Backend", "127.0.0.1:7979"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-Prompt", "short daemon check"])
        );
    }

    #[test]
    fn start_args_do_not_override_backend_by_default() {
        let args = args_as_strings(daemon_control_args(
            EvolutionDaemonAction::Start,
            "target\\evolution\\daemon",
            None,
            None,
            true,
            EvolutionDaemonStartOptions::default(),
        ));

        assert!(!args.contains(&"-Backend".to_owned()));
        assert!(!args.contains(&"-Prompt".to_owned()));
        assert!(args.contains(&"-CheckOnly".to_owned()));
    }

    #[test]
    fn stop_args_never_forward_prompt_or_backend() {
        let args = args_as_strings(daemon_control_args(
            EvolutionDaemonAction::Stop,
            "target\\evolution\\daemon",
            Some("127.0.0.1:7979"),
            Some("ignored"),
            true,
            EvolutionDaemonStartOptions::default(),
        ));

        assert_eq!(args[0], "-Stop");
        assert!(args.contains(&"-CheckOnly".to_owned()));
        assert!(!args.contains(&"-Backend".to_owned()));
        assert!(!args.contains(&"-Prompt".to_owned()));
    }

    #[test]
    fn start_args_forward_budget_overrides() {
        let args = args_as_strings(daemon_control_args(
            EvolutionDaemonAction::Start,
            "target\\evolution\\budget-smoke",
            Some("127.0.0.1:7979"),
            None,
            true,
            EvolutionDaemonStartOptions {
                interval_secs: Some(1),
                max_tokens: Some(64),
                max_total_tokens: Some(96),
                max_runtime_secs: Some(300),
                max_failures: Some(1),
                max_no_feedback_rounds: Some(0),
                timeout_secs: Some(300),
            },
        ));

        for pair in [
            ["-IntervalSecs", "1"],
            ["-MaxTokens", "64"],
            ["-MaxTotalTokens", "96"],
            ["-MaxRuntimeSecs", "300"],
            ["-MaxFailures", "1"],
            ["-MaxNoFeedbackRounds", "0"],
            ["-TimeoutSecs", "300"],
        ] {
            assert!(args.windows(2).any(|window| window == pair));
        }
    }
}
