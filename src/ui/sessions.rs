//! Session explorer rendering and keyboard interaction.

use super::input::{keep_selection_in_bounds, matches_query, move_selection};
use super::style;
use super::terminal::TerminalSession;
use crate::session::{scan_codex_sessions, CodexSessionInfo};
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState, Wrap},
};
use std::process::Command;
use std::time::Duration;

/// The next action selected from the session explorer.
pub(super) enum SessionAction {
    /// Exit CXM.
    Quit,
    /// Return to the account dashboard.
    ShowAccounts,
}

/// Runs the session explorer until the user returns or resumes a session.
pub(super) fn run() -> Result<SessionAction> {
    let mut terminal = TerminalSession::start()?;
    let sessions = scan_codex_sessions()?;
    let mut state = TableState::default();
    let mut filter = String::new();
    let mut searching = false;
    let mut show_detail = false;
    keep_selection_in_bounds(&mut state, sessions.len());

    loop {
        let filtered = filtered_indices(&sessions, &filter);
        keep_selection_in_bounds(&mut state, filtered.len());
        terminal.terminal_mut().draw(|frame| {
            render(
                frame,
                &sessions,
                &filtered,
                &mut state,
                &filter,
                searching,
                show_detail,
            )
        })?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if searching {
            update_filter(&mut filter, &mut searching, key.code);
            continue;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(SessionAction::Quit),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(SessionAction::Quit)
            }
            KeyCode::Char('/') => searching = true,
            KeyCode::Char(' ') | KeyCode::Char('v') => show_detail = !show_detail,
            KeyCode::Char('a') | KeyCode::Tab => return Ok(SessionAction::ShowAccounts),
            KeyCode::Down | KeyCode::Char('j') => move_selection(&mut state, filtered.len(), true),
            KeyCode::Up | KeyCode::Char('k') => move_selection(&mut state, filtered.len(), false),
            KeyCode::Enter => {
                if let Some(session) = selected_session(&sessions, &filtered, &state) {
                    drop(terminal);
                    resume_session(&session.session_id)?;
                    return Ok(SessionAction::Quit);
                }
            }
            _ => {}
        }
    }
}

fn filtered_indices(sessions: &[CodexSessionInfo], filter: &str) -> Vec<usize> {
    sessions
        .iter()
        .enumerate()
        .filter_map(|(index, session)| {
            (matches_query(filter, &session.session_id) || matches_query(filter, &session.summary))
                .then_some(index)
        })
        .collect()
}

fn selected_session<'a>(
    sessions: &'a [CodexSessionInfo],
    filtered: &[usize],
    state: &TableState,
) -> Option<&'a CodexSessionInfo> {
    sessions.get(*filtered.get(state.selected()?)?)
}

fn resume_session(session_id: &str) -> Result<()> {
    let status = Command::new("codex")
        .args(["resume", session_id])
        .status()
        .context("Failed to launch 'codex resume'")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("'codex resume' exited with {status}")
    }
}

fn update_filter(filter: &mut String, searching: &mut bool, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            filter.clear();
            *searching = false;
        }
        KeyCode::Enter => *searching = false,
        KeyCode::Backspace => {
            filter.pop();
        }
        KeyCode::Char(character) => filter.push(character),
        _ => {}
    }
}

fn render(
    frame: &mut ratatui::Frame,
    sessions: &[CodexSessionInfo],
    filtered: &[usize],
    state: &mut TableState,
    filter: &str,
    searching: bool,
    show_detail: bool,
) {
    let constraints = if show_detail {
        [
            Constraint::Length(3),
            Constraint::Percentage(50),
            Constraint::Min(6),
            Constraint::Length(3),
        ]
    } else {
        [
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(0),
            Constraint::Length(3),
        ]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    let displayed_filter = if filter.is_empty() { "None" } else { filter };
    let header = Paragraph::new(format!(
        " 💬 CXM — Session Explorer ({}) | Filter: {displayed_filter}",
        filtered.len()
    ))
    .style(style::header())
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    let rows = filtered
        .iter()
        .filter_map(|index| sessions.get(*index))
        .map(|session| {
            Row::new([
                session.short_id.clone(),
                session.datetime.clone(),
                session.summary.clone(),
            ])
        })
        .collect::<Vec<_>>();
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(20),
            Constraint::Min(30),
        ],
    )
    .header(Row::new(["Session ID", "Date/Time", "Prompt Summary"]).style(style::header()))
    .block(Block::default().borders(Borders::ALL))
    .row_highlight_style(style::selected());
    frame.render_stateful_widget(table, chunks[1], state);

    if show_detail {
        let detail = selected_session(sessions, filtered, state).map_or_else(
            || "No session selected.".to_owned(),
            |session| {
                format!(
                    "ID: {}\nDate: {}\n\n{}",
                    session.session_id, session.datetime, session.full_prompt
                )
            },
        );
        let preview = Paragraph::new(detail).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(" 🔍 Session Detail Preview ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        frame.render_widget(preview, chunks[2]);
    }

    let footer_text = if searching {
        format!(" Search: {filter} (Press Enter to confirm, Esc to clear)")
    } else {
        " [Enter] Resume | [Space/v] Toggle Preview | [a] Accounts | [/] Filter | [q] Quit"
            .to_owned()
    };
    let footer = Paragraph::new(footer_text)
        .style(style::dimmed())
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[3]);
}
