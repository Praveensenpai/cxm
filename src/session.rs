use anyhow::Result;
use chrono::{DateTime, Local};
use colored::*;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct HistoryLine {
    session_id: Option<String>,
    ts: Option<u64>,
    text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CodexSessionInfo {
    pub session_id: String,
    pub short_id: String,
    pub datetime: String,
    pub timestamp: u64,
    pub summary: String,
    pub full_prompt: String,
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

pub fn scan_codex_sessions() -> Result<Vec<CodexSessionInfo>> {
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

    let mut sessions: Vec<CodexSessionInfo> = session_map
        .into_iter()
        .map(|(session_id, (raw_prompt, timestamp))| {
            let short_id = if session_id.len() >= 8 {
                session_id[..8].to_string()
            } else {
                session_id.clone()
            };

            let datetime = if timestamp > 0 {
                DateTime::from_timestamp(timestamp as i64, 0)
                    .map(|t| t.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            } else {
                "Unknown".to_string()
            };

            CodexSessionInfo {
                session_id,
                short_id,
                datetime,
                timestamp,
                summary: sanitize_prompt(&raw_prompt),
                full_prompt: raw_prompt,
            }
        })
        .collect();

    sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(sessions)
}

pub fn pick_and_resume_session() -> Result<()> {
    let sessions = scan_codex_sessions()?;
    if sessions.is_empty() {
        println!("{}", "No Codex session history found.".yellow());
        return Ok(());
    }

    enable_raw_mode()?;
    let mut out = stdout();
    let _ = execute!(out, EnterAlternateScreen, cursor::Hide);

    let mut selected_idx = 0;
    let mut search_query = String::new();
    let mut expanded_id: Option<String> = None;

    let result_session_opt = loop {
        let (cols, term_rows) = crossterm::terminal::size().unwrap_or((80, 24));
        let cols_usize = cols as usize;

        let filtered: Vec<&CodexSessionInfo> = sessions
            .iter()
            .filter(|s| {
                if search_query.is_empty() {
                    true
                } else {
                    let q = search_query.to_lowercase();
                    s.summary.to_lowercase().contains(&q)
                        || s.session_id.to_lowercase().contains(&q)
                        || s.datetime.contains(&q)
                }
            })
            .collect();

        if selected_idx >= filtered.len() && !filtered.is_empty() {
            selected_idx = filtered.len() - 1;
        }

        let _ = execute!(
            out,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        );

        let count_str = format!("({}/{} sessions)", filtered.len(), sessions.len());
        let sep = "─".repeat(cols_usize.min(120));
        print!("\x1b[38;2;189;147;249m{}\x1b[0m\r\n", sep);
        print!(
            "\x1b[1m\x1b[38;2;139;233;253m🔍 Search Codex Session \x1b[38;2;98;114;164m{}\x1b[38;2;139;233;253m > \x1b[38;2;80;250;123m{}\x1b[0m\r\n",
            count_str, search_query
        );
        print!("\x1b[38;2;98;114;164m[ ↑↓: Move | Space/v: Details | Enter: Resume | Esc: Exit ]\x1b[0m\r\n");
        print!("\x1b[38;2;189;147;249m{}\x1b[0m\r\n\r\n", sep);

        if filtered.is_empty() {
            print!("  \x1b[38;2;255;85;85mNo matching sessions found.\x1b[0m\r\n");
        } else {
            let avail_height = (term_rows as usize).saturating_sub(6).max(3);

            let get_item_height = |idx: usize| -> usize {
                let s = filtered[idx];
                if expanded_id.as_ref() == Some(&s.session_id) {
                    let p_lines = s.full_prompt.lines().take(6).count();
                    1 + 3 + p_lines + 1
                } else {
                    1
                }
            };

            let mut start_idx = selected_idx;
            let mut h_acc = get_item_height(selected_idx);
            while start_idx > 0 {
                let prev_h = get_item_height(start_idx - 1);
                if h_acc + prev_h > avail_height {
                    break;
                }
                start_idx -= 1;
                h_acc += prev_h;
            }

            let avail_prompt_width = cols_usize.saturating_sub(34).max(15);
            let mut rendered_height = 0;

            for idx in start_idx..filtered.len() {
                let item_h = get_item_height(idx);
                if rendered_height > 0 && rendered_height + item_h > avail_height {
                    break;
                }
                rendered_height += item_h;

                let s = filtered[idx];
                let is_selected = idx == selected_idx;
                let is_expanded = expanded_id.as_ref() == Some(&s.session_id);

                let trunc_summary = if s.summary.chars().count() > avail_prompt_width {
                    let text: String = s.summary.chars().take(avail_prompt_width.saturating_sub(3)).collect();
                    format!("{}...", text)
                } else {
                    s.summary.clone()
                };

                if is_selected {
                    print!(
                        " \x1b[38;2;80;250;123m▶\x1b[0m \x1b[1m\x1b[38;2;139;233;253m{}\x1b[0m │ \x1b[38;2;255;121;198m{}\x1b[0m │ \x1b[1m\x1b[38;2;248;248;242m{}\x1b[0m\r\n",
                        s.datetime, s.short_id, trunc_summary
                    );
                } else {
                    print!(
                        "   \x1b[38;2;98;114;164m{}\x1b[0m │ \x1b[38;2;98;114;164m{}\x1b[0m │ \x1b[38;2;98;114;164m{}\x1b[0m\r\n",
                        s.datetime, s.short_id, trunc_summary
                    );
                }

                if is_expanded {
                    let box_w = cols_usize.saturating_sub(6).min(100);
                    let top_bar = format!("┌─ 🔍 FULL SESSION DETAILS {}", "─".repeat(box_w.saturating_sub(27)));
                    print!("    \x1b[38;2;255;184;108m{}\x1b[0m\r\n", top_bar);
                    print!("    \x1b[38;2;255;184;108m│\x1b[0m \x1b[1mFull Session ID:\x1b[0m \x1b[38;2;255;121;198m{}\x1b[0m\r\n", s.session_id);
                    print!("    \x1b[38;2;255;184;108m│\x1b[0m \x1b[1mDate:\x1b[0m {}\r\n", s.datetime);
                    print!("    \x1b[38;2;255;184;108m│\x1b[0m \x1b[1mPrompt:\x1b[0m\r\n");
                    for p_line in s.full_prompt.lines().take(6) {
                        print!("    \x1b[38;2;255;184;108m│\x1b[0m   {}\r\n", p_line);
                    }
                    let bot_bar = "└".to_string() + &"─".repeat(box_w.saturating_sub(1));
                    print!("    \x1b[38;2;255;184;108m{}\x1b[0m\r\n", bot_bar);
                }
            }
        }

        let _ = out.flush();

        if let Ok(Event::Key(key_event)) = event::read() {
            match key_event.code {
                KeyCode::Esc => break None,
                KeyCode::Char('q') if key_event.modifiers.contains(KeyModifiers::CONTROL) => break None,
                KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => break None,
                KeyCode::Enter => {
                    if !filtered.is_empty() {
                        break Some(filtered[selected_idx].clone());
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected_idx > 0 {
                        selected_idx -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !filtered.is_empty() && selected_idx + 1 < filtered.len() {
                        selected_idx += 1;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Tab | KeyCode::Char('v') => {
                    if !filtered.is_empty() {
                        let cur_id = &filtered[selected_idx].session_id;
                        if expanded_id.as_ref() == Some(cur_id) {
                            expanded_id = None;
                        } else {
                            expanded_id = Some(cur_id.clone());
                        }
                    }
                }
                KeyCode::Backspace => {
                    search_query.pop();
                    selected_idx = 0;
                }
                KeyCode::Char(c) => {
                    search_query.push(c);
                    selected_idx = 0;
                }
                _ => {}
            }
        }
    };

    let _ = execute!(out, cursor::Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();

    if let Some(selected_session) = result_session_opt {
        println!("🚀 Resuming Codex session {}...", selected_session.session_id.cyan());
        let mut child = Command::new("codex")
            .args(["resume", &selected_session.session_id])
            .spawn()?;
        let _ = child.wait();
    }

    Ok(())
}
