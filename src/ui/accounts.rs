//! Account dashboard rendering and keyboard interaction.

use super::input::{keep_selection_in_bounds, matches_query, move_selection};
use super::style;
use super::terminal::TerminalSession;
use crate::account::{AccountProfile, AccountStore};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

/// The next action selected from the account dashboard.
pub(super) enum AccountAction {
    /// Exit CXM.
    Quit,
    /// Open the session explorer.
    ShowSessions,
    /// Switch to the named profile.
    Switch(String),
    /// Remove the named profile.
    Remove(String),
    /// Start a fresh Codex login flow.
    StartNewSession,
}

/// Runs the account dashboard until the user chooses an action.
pub(super) fn run(store: AccountStore) -> Result<AccountAction> {
    let mut terminal = TerminalSession::start()?;
    let mut state = TableState::default();
    let mut filter = String::new();
    let mut searching = false;
    let mut accounts = store.list_cached()?;
    let has_valid_cache = has_complete_cache(&accounts);
    let (sender, receiver) = channel();
    let mut refresh = RefreshState::new(!has_valid_cache);

    if !has_valid_cache {
        start_refresh(store.clone(), sender.clone(), false);
    }
    keep_selection_in_bounds(&mut state, accounts.len());

    loop {
        receive_refresh(&receiver, &mut accounts, &mut refresh, &mut state);
        let filtered = filtered_indices(&accounts, &filter);
        keep_selection_in_bounds(&mut state, filtered.len());
        terminal.terminal_mut().draw(|frame| {
            render(
                frame, &accounts, &filtered, &mut state, &filter, searching, &refresh,
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
            KeyCode::Char('q') | KeyCode::Esc => return Ok(AccountAction::Quit),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(AccountAction::Quit)
            }
            KeyCode::Char('/') => searching = true,
            KeyCode::Char('s') | KeyCode::Tab => return Ok(AccountAction::ShowSessions),
            KeyCode::Char('n') => return Ok(AccountAction::StartNewSession),
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(name) = selected_account_name(&accounts, &filtered, &state) {
                    return Ok(AccountAction::Remove(name));
                }
            }
            KeyCode::Enter => {
                if let Some(name) = selected_account_name(&accounts, &filtered, &state) {
                    return Ok(AccountAction::Switch(name));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => move_selection(&mut state, filtered.len(), true),
            KeyCode::Up | KeyCode::Char('k') => move_selection(&mut state, filtered.len(), false),
            KeyCode::Char('r') => request_refresh(&store, &sender, &mut refresh),
            _ => {}
        }
    }
}

type RefreshMessage = Result<Vec<AccountProfile>, String>;

struct RefreshState {
    is_running: bool,
    last_started: Option<Instant>,
    message: Option<(String, Instant)>,
}

impl RefreshState {
    fn new(is_running: bool) -> Self {
        Self {
            is_running,
            last_started: None,
            message: None,
        }
    }
}

fn has_complete_cache(accounts: &[AccountProfile]) -> bool {
    !accounts.is_empty()
        && accounts.iter().all(|account| {
            account
                .quota
                .as_ref()
                .is_some_and(|quota| !quota.is_expired())
        })
}

fn start_refresh(store: AccountStore, sender: Sender<RefreshMessage>, bypass_cache: bool) {
    std::thread::spawn(move || {
        let result = store
            .list_with_quota_refresh(bypass_cache)
            .map_err(|error| error.to_string());
        let _ = sender.send(result);
    });
}

fn receive_refresh(
    receiver: &Receiver<RefreshMessage>,
    accounts: &mut Vec<AccountProfile>,
    refresh: &mut RefreshState,
    state: &mut TableState,
) {
    if let Ok(result) = receiver.try_recv() {
        refresh.is_running = false;
        match result {
            Ok(fresh_accounts) => {
                *accounts = fresh_accounts;
                keep_selection_in_bounds(state, accounts.len());
            }
            Err(error) => {
                refresh.message = Some((
                    format!(" Refresh failed: {error}"),
                    Instant::now() + Duration::from_secs(3),
                ));
            }
        }
    }
}

fn request_refresh(
    store: &AccountStore,
    sender: &Sender<RefreshMessage>,
    refresh: &mut RefreshState,
) {
    let now = Instant::now();
    if refresh.is_running {
        refresh.message = Some((
            " Refresh is already running in background...".to_owned(),
            now + Duration::from_secs(3),
        ));
        return;
    }
    if let Some(last_started) = refresh.last_started {
        let cooldown = Duration::from_secs(15);
        if last_started.elapsed() < cooldown {
            let remaining = cooldown.saturating_sub(last_started.elapsed()).as_secs();
            refresh.message = Some((
                format!(" Refresh is on cooldown. Please wait {remaining}s."),
                now + Duration::from_secs(3),
            ));
            return;
        }
    }

    refresh.is_running = true;
    refresh.last_started = Some(now);
    start_refresh(store.clone(), sender.clone(), true);
}

fn filtered_indices(accounts: &[AccountProfile], filter: &str) -> Vec<usize> {
    accounts
        .iter()
        .enumerate()
        .filter_map(|(index, account)| {
            matches_query(filter, account.name.as_str()).then_some(index)
        })
        .collect()
}

fn selected_account_name(
    accounts: &[AccountProfile],
    filtered: &[usize],
    state: &TableState,
) -> Option<String> {
    let selected = state.selected()?;
    let account_index = *filtered.get(selected)?;
    Some(accounts.get(account_index)?.name.as_str().to_owned())
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
    accounts: &[AccountProfile],
    filtered: &[usize],
    state: &mut TableState,
    filter: &str,
    searching: bool,
    refresh: &RefreshState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(frame.area());
    let active_account = accounts
        .iter()
        .find(|account| account.is_active)
        .map(|account| account.name.as_str())
        .unwrap_or("None");
    let loading = if refresh.is_running {
        " | Refreshing... ⏳"
    } else {
        ""
    };
    let header = Paragraph::new(format!(
        " ⚡ CXM — Codex Accounts ({}) | Active: {active_account}{loading}",
        accounts.len()
    ))
    .style(style::header())
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    let rows = account_rows(accounts, filtered);
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(30),
            Constraint::Min(30),
        ],
    )
    .header(Row::new(["Status", "Account Profile", "Quota Metrics"]).style(style::header()))
    .block(Block::default().borders(Borders::ALL))
    .row_highlight_style(style::selected());
    frame.render_stateful_widget(table, chunks[1], state);

    let (footer_text, footer_style) = footer(filter, searching, refresh);
    let footer = Paragraph::new(footer_text)
        .style(footer_style)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

fn account_rows(accounts: &[AccountProfile], filtered: &[usize]) -> Vec<Row<'static>> {
    if accounts.is_empty() {
        return vec![Row::new([
            "",
            "No saved accounts",
            "Run cxm save <alias> after logging in",
        ])];
    }
    filtered
        .iter()
        .filter_map(|index| accounts.get(*index))
        .map(|account| {
            let status = if account.is_active {
                "* ACTIVE"
            } else {
                "  INACTIVE"
            };
            let quota = account.quota.as_ref().map_or_else(
                || "[quota unavailable]".to_owned(),
                |quota| quota.display_badge(),
            );
            Row::new([status.to_owned(), account.name.as_str().to_owned(), quota])
        })
        .collect()
}

fn footer(filter: &str, searching: bool, refresh: &RefreshState) -> (String, Style) {
    if let Some((message, until)) = &refresh.message {
        if Instant::now() < *until {
            return (
                message.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }
    if refresh.is_running {
        return (
            " ⏳ Fetching live account quotas in background... Navigate freely with [↑/↓]"
                .to_owned(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    }
    if searching {
        return (
            format!(" Search: {filter} (Press Enter to confirm, Esc to clear)"),
            style::header(),
        );
    }
    (
        " [Enter] Switch | [s] Sessions | [n] New Login | [d] Delete | [r] Refresh | [/] Filter | [q] Quit".to_owned(),
        style::dimmed(),
    )
}
