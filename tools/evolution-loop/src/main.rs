mod args;
mod clean_room_batch_status;
mod clean_room_handoff;
mod helper_feedback;
mod helper_stage_repair;
mod http;
mod inference_backend;
mod json;
mod ledger;
mod model_policy;
mod model_registry;
mod outcome_log;
mod pool_artifacts;
mod pool_dispatch;
mod pool_lease;
mod pool_request;
mod pool_stage;
mod pool_stage_call;
mod profile_scoring;
mod prompts;
mod remote_chain;
mod report;
mod routing_rules;
mod runner;
mod self_improve_proposal_artifact;
mod sse;
mod validation;
mod worker_window_status;

fn main() {
    match args::parse_env() {
        Ok(args::ParseOutcome::Help) => {
            print!("{}", args::help_text());
        }
        Ok(args::ParseOutcome::ListModels) => match model_registry::default_model_registry() {
            Ok(registry) => println!("{}", registry.render_model_list()),
            Err(error) => {
                eprintln!("evolution-loop model registry error: {error}");
                std::process::exit(1);
            }
        },
        Ok(args::ParseOutcome::Report(config)) => {
            if let Err(error) = report::run(config) {
                eprintln!("evolution-loop report error: {error}");
                std::process::exit(1);
            }
        }
        Ok(args::ParseOutcome::Run(config)) => {
            if let Err(error) = runner::run(config) {
                eprintln!("evolution-loop error: {error}");
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            eprintln!();
            eprintln!("{}", args::help_text());
            std::process::exit(2);
        }
    }
}
