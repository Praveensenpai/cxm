use anyhow::Result;
use chrono::{DateTime, Local};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Terminal,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::stdout;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct HistoryLine {
    session_id: Option<String>,
    ts: Option<u64>,
    text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CodexSession {
    pub session_id: String,
    pub prompt: String,
    pub timestamp: u64,
}

impl CodexSession {
    pub fn formatted_time(&self) -> String {
        if self.timestamp == 0 {
            return "unknown time".to_string();
        }
        let naive = DateTime::from_timestamp(self.timestamp as i64, 0);
        match naive {
            Some(utc) => {
                let local: DateTime<Local> = DateTime::from(utc);
                local.format("%Y-%m-%d %H:%M").to_string()
            }
            None => "unknown time".to_string(),
        }
    }

    pub fn short_id(&self) -> String {
        if self.session_id.len() >= 8 {
            self.session_id[..8].to_string()
        } else {
            self.session_id.clone()
        }
    }
}

pub fn get_history_file_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".codex").join("history.jsonl"))
}

pub fn sanitize_prompt(raw: &str) -> String {
    let cleaned = raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("[Image #"))
        .collect::<Vec<&str>>()
        .join(" ");

    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "New Conversation".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn scan_codex_sessions() -> Result<Vec<CodexSession>> {
    let history_path = match get_history_file_path() {
        Some(p) if p.exists() => p,
        _ => return Ok(Vec::new()),
    };

    let content = fs::read_to_string(&history_path)?;
    let mut session_map: HashMap<String, (String, u64)> = HashMap::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(item) = serde_json::from_str::<HistoryLine>(line) {
            if let (Some(sid), Some(ts), Some(text)) = (item.session_id, item.ts, item.text) {
                let entry = session_map.entry(sid).or_insert_with(|| (text.clone(), ts));
                if entry.1 < ts {
                    entry.1 = ts;
                }
                if entry.0 == "." || entry.0.starts_with("[Image #") {
                    let sanitized = sanitize_prompt(&text);
                    if sanitized != "New Conversation" {
                        entry.0 = text;
                    }
                }
            }
        }
    }

    let mut sessions: Vec<CodexSession> = session_map
        .into_iter()
        .map(|(session_id, (raw_prompt, timestamp))| CodexSession {
            session_id,
            prompt: sanitize_prompt(&raw_prompt),
            timestamp,
        })
        .collect();

    sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(sessions)
}

pub fn pick_and_resume_session() -> Result<()> {
    let sessions = scan_codex_sessions()?;
    if sessions.is_empty() {
        println!("No Codex session history found.");
        return Ok(());
    }

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut selected_index = 0;
    let mut search_query = String::new();
    let mut is_searching = false;

    let selected_session_id = loop {
        let filtered: Vec<&CodexSession> = sessions
            .iter()
            .filter(|s| {
                if search_query.is_empty() {
                    true
                } else {
                    let q = search_query.to_lowercase();
                    s.prompt.to_lowercase().contains(&q)
                        || s.short_id().to_lowercase().contains(&q)
                        || s.formatted_time().to_lowercase().contains(&q)
                }
            })
            .collect();

        if selected_index >= filtered.len() && !filtered.is_empty() {
            selected_index = filtered.len() - 1;
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(5),
                    Constraint::Length(if is_searching { 3 } else { 1 }),
                ])
                .split(f.area());

            let header_cells = ["ID", "TIMESTAMP", "PROMPT / TITLE"]
                .iter()
                .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
            let header = Row::new(header_cells)
                .style(Style::default().bg(Color::Rgb(40, 42, 54)))
                .height(1);

            let rows: Vec<Row> = filtered
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let style = if i == selected_index {
                        Style::default()
                            .fg(Color::Rgb(255, 255, 255))
                            .bg(Color::Rgb(98, 114, 164))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(248, 248, 242))
                    };

                    Row::new(vec![
                        Cell::from(Span::styled(s.short_id(), Style::default().fg(Color::Rgb(189, 147, 249)).add_modifier(Modifier::BOLD))),
                        Cell::from(Span::styled(s.formatted_time(), Style::default().fg(Color::Rgb(241, 250, 140)))),
                        Cell::from(s.prompt.clone()),
                    ])
                    .style(style)
                })
                .collect();

            let title = format!(" 💬 Codex Session Explorer ({} sessions) ", filtered.len());
            let table = Table::new(
                rows,
                [
                    Constraint::Length(10),
                    Constraint::Length(18),
                    Constraint::Min(30),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .title_style(Style::default().fg(Color::Rgb(80, 250, 123)).add_modifier(Modifier::BOLD)),
            );

            f.render_widget(table, chunks[0]);

            if is_searching {
                let search_bar = Paragraph::new(format!("Search: {}_", search_query))
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::default().borders(Borders::ALL).title(" Filter Sessions "));
                f.render_widget(search_bar, chunks[1]);
            } else {
                let help_text = if search_query.is_empty() {
                    " [↑/↓/j/k] Navigate • [/] Filter • [Enter] Resume • [Esc/q] Quit "
                } else {
                    " [↑/↓/j/k] Navigate • [/] Edit Filter • [Backspace] Clear • [Enter] Resume • [Esc] Quit "
                };
                let footer = Paragraph::new(help_text)
                    .style(Style::default().fg(Color::DarkGray));
                f.render_widget(footer, chunks[1]);
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if is_searching {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                is_searching = false;
                            }
                            KeyCode::Backspace => {
                                search_query.pop();
                                selected_index = 0;
                            }
                            KeyCode::Char(c) => {
                                search_query.push(c);
                                selected_index = 0;
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                break None;
                            }
                            KeyCode::Char('/') => {
                                is_searching = true;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if selected_index > 0 {
                                    selected_index -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if !filtered.is_empty() && selected_index < filtered.len() - 1 {
                                    selected_index += 1;
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(session) = filtered.get(selected_index) {
                                    break Some(session.session_id.clone());
                                }
                            }
                            KeyCode::Backspace => {
                                search_query.clear();
                                selected_index = 0;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    };

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    if let Some(session_id) = selected_session_id {
        println!("🚀 Resuming Codex session {}...", session_id);
        let mut child = Command::new("codex")
            .args(["resume", &session_id])
            .spawn()?;
        let _ = child.wait();
    }

    Ok(())
}
