use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::event;

use crate::app::{App, ChatProvider};

mod input;
mod render;
mod terminal;

pub fn run<P: ChatProvider>(app: &mut App<P>) -> io::Result<()> {
    let mut session = terminal::start_terminal()?;
    run_loop(session.terminal_mut(), app)
}

fn run_loop<P: ChatProvider>(
    terminal: &mut terminal::ForgeTerminal,
    app: &mut App<P>,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let mut key_filter = input::KeyInputFilter::default();

    while !app.should_quit {
        terminal.draw(|frame| render::draw(frame, app))?;

        let timeout = Duration::from_millis(50)
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();

        if event::poll(timeout)? {
            input::handle_event(app, event::read()?, &mut key_filter, Instant::now());
        }

        if last_tick.elapsed() >= Duration::from_millis(50) {
            app.tick();
            last_tick = Instant::now();
        }
    }

    Ok(())
}
