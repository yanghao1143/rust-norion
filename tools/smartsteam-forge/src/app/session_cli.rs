use std::{
    io::{self, Write},
    path::Path,
};

use smartsteam_forge::{
    SessionFilter, SessionStore, list_recent_sessions_filtered, summarize_recent_session,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionCliCommand {
    List { filter: SessionFilter, limit: usize },
    Summary { selector: String },
}

pub fn run_session_cli(command: &SessionCliCommand) -> io::Result<()> {
    let root = SessionStore::default_root().map_err(io::Error::other)?;
    let mut stdout = io::stdout();
    run_session_cli_to(&root, command, &mut stdout)
}

pub fn run_session_cli_to<W: Write>(
    root: &Path,
    command: &SessionCliCommand,
    output: &mut W,
) -> io::Result<()> {
    match command {
        SessionCliCommand::List { filter, limit } => {
            let records =
                list_recent_sessions_filtered(root, *filter, *limit).map_err(io::Error::other)?;
            if records.is_empty() {
                writeln!(
                    output,
                    "no recorded sessions filter={} root={}",
                    filter.label(),
                    root.display()
                )?;
                return Ok(());
            }

            writeln!(
                output,
                "recent sessions filter={} limit={} root={}",
                filter.label(),
                limit,
                root.display()
            )?;
            for (index, record) in records.iter().enumerate() {
                writeln!(output, "{}. {}", index + 1, record.summary_line())?;
            }
            Ok(())
        }
        SessionCliCommand::Summary { selector } => {
            let summary = summarize_recent_session(root, selector).map_err(io::Error::other)?;
            writeln!(output, "{}", summary.summary_line())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use smartsteam_forge::SessionStore;

    use super::*;

    #[test]
    fn lists_sessions_without_creating_a_new_transcript() {
        let root = unique_root("list");
        let store = SessionStore::open(&root).unwrap();
        store.append_message("user", "hello from cli").unwrap();
        store
            .append_event("gate_report", "Business-cycle gate report\noverall: PASS")
            .unwrap();
        let before = jsonl_count(&root);
        let mut output = Vec::new();

        run_session_cli_to(
            &root,
            &SessionCliCommand::List {
                filter: SessionFilter::Passed,
                limit: 10,
            },
            &mut output,
        )
        .unwrap();

        assert_eq!(jsonl_count(&root), before);
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("recent sessions filter=passed"));
        assert!(output.contains("hello from cli"));
        assert!(output.contains("gate=PASS"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn writes_session_summary_from_cli_selector() {
        let root = unique_root("summary");
        let store = SessionStore::open(&root).unwrap();
        store.append_message("user", "summarize from cli").unwrap();
        store.append_message("assistant", "summary answer").unwrap();
        let mut output = Vec::new();

        run_session_cli_to(
            &root,
            &SessionCliCommand::Summary {
                selector: "1".to_owned(),
            },
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("latest_user=\"summarize from cli\""));
        assert!(output.contains(".summary.md"));
        assert!(
            store
                .transcript_path()
                .with_extension("summary.md")
                .exists()
        );

        let _ = fs::remove_dir_all(root);
    }

    fn unique_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "smartsteam_cli_{label}_{}_{}",
            std::process::id(),
            unix_timestamp_millis()
        ))
    }

    fn jsonl_count(root: &Path) -> usize {
        fs::read_dir(root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
            .count()
    }

    fn unix_timestamp_millis() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }
}
