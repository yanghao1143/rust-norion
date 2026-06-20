use std::env;

mod app;
mod backend;
mod chunk;
mod config;
mod http;
mod json;
mod model_pool_advice;
mod repl;
mod request;
mod sse;
mod status;

fn main() -> std::io::Result<()> {
    let config = config::parse_args(env::args().skip(1));
    match config.mode {
        config::RunMode::Server => app::run(config),
        config::RunMode::Repl => repl::run(config),
        config::RunMode::Help => {
            print!("{}", config::help_text());
            Ok(())
        }
    }
}
