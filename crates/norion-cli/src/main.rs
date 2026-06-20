use std::{env, process};

use norion_cli::{help_text, parse_cli_args};

fn main() {
    let config = match parse_cli_args(env::args().skip(1)) {
        Ok(config) => config,
        Err(error) if error == help_text() => {
            println!("{error}");
            return;
        }
        Err(error) => {
            eprintln!("{error}");
            eprintln!("{}", help_text());
            process::exit(2);
        }
    };
    for line in config.startup_lines() {
        println!("{line}");
    }
}
