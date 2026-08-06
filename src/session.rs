use anyhow::Result;
use chrono::{DateTime, Local};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

pub fn clean_user_text(raw: &str) -> String {
    let mut s = raw.to_string();
    let tags = [
        "<USER_REQUEST>",
        "</USER_REQUEST>",
        "<USER_SETTINGS_CHANGE>",
        "</USER_SETTINGS_CHANGE>",
        "<ADDITIONAL_METADATA>",
        "</ADDITIONAL_METADATA>",
        "<EPHEMERAL_MESSAGE>",
        "</EPHEMERAL_MESSAGE>",
    ];
    for tag in tags {
        s = s.replace(tag, "");
    }

    let cleaned = s
        .lines()
        .map(|l| l.trim())
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with('<')
                && !l.starts_with("The current local time is:")
                && !l.starts_with("The user changed setting")
                && !l.starts_with("The user has uploaded")
                && !l.starts_with("┌─")
                && !l.starts_with("└─")
                && !l.starts_with('│')
                && !l.starts_with("~ ❯")
                && !l.starts_with("~ ✗")
        })
        .collect::<Vec<&str>>()
        .join("\n");

    cleaned.trim().to_string()
}

pub fn sanitize_summary(raw: &str) -> String {
    let cleaned = clean_user_text(raw);
    let single_line = cleaned
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<&str>>()
        .join(" ");

    let trimmed = single_line.trim();
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
                    let sanitized = sanitize_summary(&text);
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

            let full_prompt = clean_user_text(&raw_prompt);

            CodexSessionInfo {
                session_id,
                short_id,
                datetime,
                timestamp,
                summary: sanitize_summary(&raw_prompt),
                full_prompt: if full_prompt.is_empty() { "New Conversation".to_string() } else { full_prompt },
            }
        })
        .collect();

    sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(sessions)
}
