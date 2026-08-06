use anyhow::Result;
use chrono::{DateTime, Local};
use colored::*;
use inquire::Select;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
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
    } else if trimmed.chars().count() > 60 {
        format!("{}...", trimmed.chars().take(57).collect::<String>())
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
        println!("{}", "No Codex session history found.".yellow());
        return Ok(());
    }

    let options: Vec<String> = sessions
        .iter()
        .map(|s| {
            format!(
                "{} [{}] {}",
                s.short_id().cyan(),
                s.formatted_time().dimmed(),
                s.prompt.bold()
            )
        })
        .collect();

    let ans = Select::new("💬 Select Codex Session to Resume:", options).prompt();

    match ans {
        Ok(choice) => {
            let short_id = choice.split_whitespace().next().unwrap_or("").trim();
            if let Some(target) = sessions.iter().find(|s| s.short_id() == short_id) {
                println!("{} Resuming Codex session {}...", "🚀".bold(), target.session_id.cyan());
                let mut child = Command::new("codex")
                    .args(["resume", &target.session_id])
                    .spawn()?;
                let _ = child.wait();
            }
        }
        Err(_) => {
            println!("Operation cancelled.");
        }
    }

    Ok(())
}
