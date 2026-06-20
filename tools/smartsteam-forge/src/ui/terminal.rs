use std::io;

use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

pub(super) type ForgeTerminal = Terminal<CrosstermBackend<io::Stdout>>;

pub(super) fn start_terminal() -> io::Result<TerminalSession> {
    enable_raw_mode()?;
    let raw_mode = RawModeGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, DisableBracketedPaste, LeaveAlternateScreen);
            return Err(error);
        }
    };
    raw_mode.disarm();
    Ok(TerminalSession::new(terminal))
}

struct RawModeGuard;

impl RawModeGuard {
    fn disarm(self) {
        std::mem::forget(self);
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

pub(super) struct TerminalSession {
    terminal: ForgeTerminal,
}

impl TerminalSession {
    fn new(terminal: ForgeTerminal) -> Self {
        Self { terminal }
    }

    pub(super) fn terminal_mut(&mut self) -> &mut ForgeTerminal {
        &mut self.terminal
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            DisableBracketedPaste,
            LeaveAlternateScreen
        );
        let _ = self.terminal.show_cursor();
    }
}
