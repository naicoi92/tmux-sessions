use std::io;

use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{prelude::CrosstermBackend, Terminal};

pub fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, io::Error> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

pub fn restore_terminal() -> Result<(), io::Error> {
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()
}
