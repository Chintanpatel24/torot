use crate::core::event::EventBus;
use crate::core::state::AppState;
use crate::tui::app::TuiApp;
use anyhow::Result;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use std::io;
use std::sync::Arc;

pub fn run() -> Result<()> {
    let state = Arc::new(AppState::new()?);
    let (bus, rx) = EventBus::new();

    let mut app = TuiApp::new(state, bus, rx);

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = ratatui::init();

    let result = app.run(&mut terminal);

    ratatui::restore();
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}
