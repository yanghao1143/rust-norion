use std::{
    io::{self, Write},
    sync::mpsc,
};

use super::provider::{ChatProvider, ProviderEvent};

pub fn run_once(provider: &dyn ChatProvider, prompt: String) -> io::Result<()> {
    let stream = provider.send(prompt);
    let mut stdout = io::stdout();
    drain_stream_to(stream, &mut stdout)
}

fn drain_stream_to<W: Write>(
    stream: mpsc::Receiver<ProviderEvent>,
    output: &mut W,
) -> io::Result<()> {
    let mut wrote_delta = false;

    loop {
        let event = stream.recv().map_err(|error| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("provider stream closed before done: {error}"),
            )
        })?;

        match event {
            ProviderEvent::Delta(delta) => {
                wrote_delta = true;
                write!(output, "{delta}")?;
                output.flush()?;
            }
            ProviderEvent::ReplaceAssistant(answer) => {
                if wrote_delta {
                    writeln!(output)?;
                }
                writeln!(output, "{answer}")?;
                output.flush()?;
                wrote_delta = true;
            }
            ProviderEvent::GateReport(report) => {
                finish_delta_line(output, &mut wrote_delta)?;
                print_labeled_block(output, "gate_report", &report)?;
            }
            ProviderEvent::Stage(stage) => {
                finish_delta_line(output, &mut wrote_delta)?;
                print_labeled_line(output, "stage", &stage)?;
            }
            ProviderEvent::Meta(meta) => {
                finish_delta_line(output, &mut wrote_delta)?;
                print_labeled_line(output, "meta", &meta)?;
            }
            ProviderEvent::Status(status) => {
                finish_delta_line(output, &mut wrote_delta)?;
                print_labeled_line(output, "status", &status)?;
            }
            ProviderEvent::Heartbeat(heartbeat) => {
                finish_delta_line(output, &mut wrote_delta)?;
                print_labeled_line(output, "heartbeat", &heartbeat)?;
            }
            ProviderEvent::Done => {
                finish_delta_line(output, &mut wrote_delta)?;
                print_labeled_line(output, "status", "done")?;
                return Ok(());
            }
            ProviderEvent::Error(error) => {
                finish_delta_line(output, &mut wrote_delta)?;
                return Err(io::Error::other(format!("provider error: {error}")));
            }
        }
    }
}

fn finish_delta_line<W: Write>(output: &mut W, wrote_delta: &mut bool) -> io::Result<()> {
    if *wrote_delta {
        writeln!(output)?;
        *wrote_delta = false;
    }
    Ok(())
}

fn print_labeled_line<W: Write>(output: &mut W, label: &str, value: &str) -> io::Result<()> {
    writeln!(output, "{label}: {value}")?;
    output.flush()
}

fn print_labeled_block<W: Write>(output: &mut W, label: &str, value: &str) -> io::Result<()> {
    writeln!(output, "{label}:")?;
    writeln!(output, "{value}")?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn drains_done_stream() {
        let (tx, rx) = mpsc::channel();
        tx.send(ProviderEvent::Delta("ok".to_owned())).unwrap();
        tx.send(ProviderEvent::Done).unwrap();
        let mut output = Vec::new();

        assert!(drain_stream_to(rx, &mut output).is_ok());
        assert_eq!(String::from_utf8(output).unwrap(), "ok\nstatus: done\n");
    }

    #[test]
    fn returns_error_for_provider_error() {
        let (tx, rx) = mpsc::channel();
        tx.send(ProviderEvent::Error("boom".to_owned())).unwrap();
        let mut output = Vec::new();

        let error = drain_stream_to(rx, &mut output).unwrap_err();

        assert!(error.to_string().contains("provider error: boom"));
        assert!(output.is_empty());
    }

    #[test]
    fn labeled_events_start_after_streamed_delta_line() {
        let (tx, rx) = mpsc::channel();
        tx.send(ProviderEvent::Delta("draft".to_owned())).unwrap();
        tx.send(ProviderEvent::Status("final ok=true".to_owned()))
            .unwrap();
        tx.send(ProviderEvent::Done).unwrap();
        let mut output = Vec::new();

        drain_stream_to(rx, &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "draft\nstatus: final ok=true\nstatus: done\n"
        );
    }
}
