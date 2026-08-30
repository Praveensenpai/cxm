//! Panic-safe terminal setup and restoration.

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

/// The terminal backend used by CXM's full-screen interface.
pub(super) type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Owns an active terminal session and restores it when dropped.
pub(super) struct TerminalSession {
    terminal: TuiTerminal,
}

impl TerminalSession {
    /// Enters raw mode and the alternate screen after building a terminal backend.
    pub(super) fn start() -> Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;
        enable_raw_mode()?;
        if let Err(error) = execute!(terminal.backend_mut(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error.into());
        }
        Ok(Self { terminal })
    }

    /// Provides mutable access for drawing the active TUI.
    pub(super) fn terminal_mut(&mut self) -> &mut TuiTerminal {
        &mut self.terminal
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
