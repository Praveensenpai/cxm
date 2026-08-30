//! Shared visual styles for the TUI.

use ratatui::style::{Color, Modifier, Style};

/// Returns the standard heading style.
pub(super) fn header() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

/// Returns the selected-table-row style.
pub(super) fn selected() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

/// Returns the subdued footer style.
pub(super) fn dimmed() -> Style {
    Style::default().fg(Color::DarkGray)
}
