use std::{env, process};

use norion_cli::{help_text, parse_cli_args, parse_evidence_packet_args, run_evidence_packet};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("evidence-packet") {
        let config = match parse_evidence_packet_args(args.iter().skip(1)) {
            Ok(config) => config,
            Err(error) => {
                eprintln!("{error}");
                process::exit(2);
            }
        };
        match run_evidence_packet(&config) {
            Ok(packet) => print!("{packet}"),
            Err(error) => {
                eprintln!("{error}");
                process::exit(2);
            }
        }
        return;
    }

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
