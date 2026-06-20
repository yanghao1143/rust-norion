use std::env;
use std::time::Duration;

mod app;
mod cli;
mod text_width;
mod ui;

use cli::{CliConfig, provider_config, usage};

fn main() -> std::io::Result<()> {
    let config = match CliConfig::parse(env::args().skip(1)) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}\n\n{}", usage());
            std::process::exit(2);
        }
    };

    if config.help {
        println!("{}", usage());
        return Ok(());
    }

    if let Some(command) = &config.session_command {
        return app::run_session_cli(command);
    }

    if config.evolution_start_check_json {
        let backend = config.backend_overridden.then_some(config.backend.as_str());
        return app::run_evolution_start_check_json(
            &config.evolution_work_dir,
            backend,
            config.prompt.as_deref(),
            config.evolution_candidates_backlog.as_deref(),
            app::EvolutionDaemonStartOptions {
                interval_secs: config.evolution_interval_secs,
                max_tokens: config.evolution_max_tokens,
                max_total_tokens: config.evolution_max_total_tokens,
                max_runtime_secs: config.evolution_max_runtime_secs,
                max_failures: config.evolution_max_failures,
                max_no_feedback_rounds: config.evolution_max_no_feedback_rounds,
                timeout_secs: config.evolution_timeout_secs,
            },
        );
    }

    if config.evolution_start || config.evolution_stop {
        let action = if config.evolution_start {
            app::EvolutionDaemonAction::Start
        } else {
            app::EvolutionDaemonAction::Stop
        };
        let backend = config.backend_overridden.then_some(config.backend.as_str());
        return app::run_evolution_daemon_control(
            action,
            &config.evolution_work_dir,
            backend,
            config.prompt.as_deref(),
            config.evolution_check_only,
            config.evolution_candidates_backlog.as_deref(),
            app::EvolutionDaemonStartOptions {
                interval_secs: config.evolution_interval_secs,
                max_tokens: config.evolution_max_tokens,
                max_total_tokens: config.evolution_max_total_tokens,
                max_runtime_secs: config.evolution_max_runtime_secs,
                max_failures: config.evolution_max_failures,
                max_no_feedback_rounds: config.evolution_max_no_feedback_rounds,
                timeout_secs: config.evolution_timeout_secs,
            },
        );
    }

    if let Some(watch) = config.evolution_watch {
        let backend = config.backend_overridden.then_some(config.backend.as_str());
        return app::run_evolution_status_watch(
            &config.evolution_work_dir,
            backend,
            Duration::from_secs(watch.interval_secs),
            watch.max_iterations,
        );
    }

    if let Some(candidate_id) = &config.evolution_candidate_mark {
        return app::run_evolution_candidate_mark(
            &config.evolution_work_dir,
            config.evolution_candidates_backlog.as_deref(),
            candidate_id,
            config.evolution_candidate_status.as_deref().unwrap_or(""),
            config.evolution_candidate_note.as_deref(),
        );
    }

    if let Some(candidate_id) = &config.evolution_candidate_apply_check {
        return app::run_evolution_candidate_apply_check(
            &config.evolution_work_dir,
            config.evolution_candidates_backlog.as_deref(),
            candidate_id,
        );
    }

    if let Some(candidate_id) = &config.evolution_candidate_validate {
        return app::run_evolution_candidate_validate(
            &config.evolution_work_dir,
            config.evolution_candidates_backlog.as_deref(),
            candidate_id,
            config
                .evolution_candidate_validation_command
                .as_deref()
                .unwrap_or(""),
            config
                .evolution_candidate_validation_status
                .as_deref()
                .unwrap_or(""),
            config.evolution_candidate_note.as_deref(),
        );
    }

    if config.evolution_candidate_gate {
        return app::run_evolution_candidate_gate(
            &config.evolution_work_dir,
            config.evolution_candidates_backlog.as_deref(),
        );
    }

    if config.evolution_candidate_list {
        return app::run_evolution_candidate_list(
            &config.evolution_work_dir,
            config.evolution_candidates_backlog.as_deref(),
            config.evolution_candidate_status.as_deref(),
            config.evolution_candidates_limit,
        );
    }

    if config.evolution_candidates {
        return app::run_evolution_candidates(
            &config.evolution_work_dir,
            config.evolution_candidates_limit,
            config.evolution_candidates_save.then_some(
                config
                    .evolution_candidates_backlog
                    .as_deref()
                    .unwrap_or_default(),
            ),
        );
    }

    if config.evolution_status {
        let backend = config.backend_overridden.then_some(config.backend.as_str());
        return app::run_evolution_status(
            &config.evolution_work_dir,
            config.evolution_status_json,
            backend,
        );
    }

    if config.evolution_strict_summary {
        return app::run_evolution_strict_summary(
            config.evolution_strict_summary_path.as_deref(),
            config.evolution_strict_summary_json,
        );
    }

    let provider: Box<dyn app::ChatProvider> = if config.mock {
        Box::new(app::MockProvider::default())
    } else {
        Box::new(app::RuntimeProvider::new(provider_config(&config)))
    };

    apply_startup_settings(provider.as_ref(), &config)?;

    if config.doctor {
        return app::run_diagnostic(provider.as_ref());
    }

    if config.health_check {
        return app::run_health_check(provider.as_ref());
    }

    if config.experience_hygiene_quarantine {
        return app::run_experience_hygiene_quarantine_dry_run(
            provider.as_ref(),
            config.experience_hygiene_limit,
        );
    }

    if config.experience_hygiene {
        return app::run_experience_hygiene_check(provider.as_ref());
    }

    if config.experience_repair {
        return app::run_experience_repair_dry_run(
            provider.as_ref(),
            config.experience_repair_limit,
        );
    }

    if config.experience_cleanup_audit {
        return app::run_experience_cleanup_audit(
            provider.as_ref(),
            config.experience_cleanup_audit_limit,
        );
    }

    if let Some(watch) = config.model_pool_watch {
        return app::run_model_pool_watch(
            provider.as_ref(),
            Duration::from_secs(watch.interval_secs),
            watch.max_iterations,
        );
    }

    if config.model_pool_status {
        return app::run_model_pool_status(provider.as_ref());
    }

    if config.model_pool_manifest {
        return app::run_model_pool_manifest(provider.as_ref());
    }

    if config.model_pool_advice {
        return app::run_model_pool_advice(provider.as_ref());
    }

    if config.model_pool_smoke {
        return app::run_model_pool_smoke(provider.as_ref());
    }

    if let Some(task_kind) = &config.model_pool_route {
        return app::run_model_pool_route(provider.as_ref(), task_kind);
    }

    if let Some(task_kind) = &config.model_pool_call {
        let prompt = config
            .prompt
            .clone()
            .ok_or_else(|| std::io::Error::other("--pool-call requires --prompt <text>"))?;
        return app::run_model_pool_call(provider.as_ref(), task_kind, &prompt);
    }

    if config.preflight_check {
        return app::run_preflight_check(provider.as_ref(), config.require_safe_device);
    }

    if let Some(prompt) = config.prompt {
        if config.require_health || config.require_safe_device {
            let summary =
                app::require_prompt_preflight(provider.as_ref(), config.require_safe_device)?;
            eprintln!("preflight: {summary}");
        }
        return app::run_once(provider.as_ref(), prompt);
    }

    if config.require_health || config.require_safe_device {
        let summary = app::require_prompt_preflight(provider.as_ref(), config.require_safe_device)?;
        eprintln!("preflight: {summary}");
    }

    let mut app = if config.require_health || config.require_safe_device {
        app::App::with_guards(provider, true, config.require_safe_device)
    } else {
        app::App::new(provider)
    };
    ui::run(&mut app)
}

fn apply_startup_settings(
    provider: &dyn app::ChatProvider,
    config: &CliConfig,
) -> std::io::Result<()> {
    if let Some(endpoint) = config.endpoint {
        provider
            .set_endpoint(endpoint)
            .map_err(|error| std::io::Error::other(format!("set mode failed: {error}")))?;
    }
    if let Some(max_messages) = config.context_messages {
        provider.set_context_window(max_messages).map_err(|error| {
            std::io::Error::other(format!("set context window failed: {error}"))
        })?;
    }
    if let Some(max_tokens) = config.max_tokens {
        provider
            .set_max_tokens(max_tokens)
            .map_err(|error| std::io::Error::other(format!("set max tokens failed: {error}")))?;
    }
    Ok(())
}
