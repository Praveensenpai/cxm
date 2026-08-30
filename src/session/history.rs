//! Codex history-file parsing.

use super::CodexSessionInfo;
use crate::paths::CodexPaths;
use anyhow::Result;
use chrono::{DateTime, Local};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Scans Codex history and returns its sessions in newest-first order.
pub(crate) fn scan(paths: &CodexPaths) -> Result<Vec<CodexSessionInfo>> {
    let path = paths.history_path();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    let mut sessions = HashMap::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<HistoryLine>(&line) {
            update_session(&mut sessions, entry);
        }
    }

    let mut sessions = sessions
        .into_iter()
        .map(|(session_id, data)| CodexSessionInfo::from_history(session_id, data))
        .collect::<Vec<_>>();
    sessions.sort_by_key(|session| std::cmp::Reverse(session.timestamp));
    Ok(sessions)
}

#[derive(Debug, Deserialize)]
struct HistoryLine {
    session_id: Option<String>,
    ts: Option<u64>,
    text: Option<String>,
}

#[derive(Debug)]
pub(super) struct SessionHistory {
    pub(super) first_prompt: String,
    pub(super) latest_timestamp: u64,
}

fn update_session(sessions: &mut HashMap<String, SessionHistory>, entry: HistoryLine) {
    let (Some(session_id), Some(timestamp), Some(text)) = (entry.session_id, entry.ts, entry.text)
    else {
        return;
    };

    let session = sessions
        .entry(session_id)
        .or_insert_with(|| SessionHistory {
            first_prompt: text.clone(),
            latest_timestamp: timestamp,
        });
    session.latest_timestamp = session.latest_timestamp.max(timestamp);
    if session.first_prompt == "." || session.first_prompt.starts_with("[Image #") {
        let summary = super::sanitize_summary(&text);
        if summary != super::NEW_CONVERSATION_SUMMARY {
            session.first_prompt = text;
        }
    }
}

pub(super) fn format_local_datetime(timestamp: u64) -> String {
    let Ok(timestamp) = i64::try_from(timestamp) else {
        return "Unknown".to_owned();
    };

    DateTime::from_timestamp(timestamp, 0)
        .map(|datetime| {
            datetime
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "Unknown".to_owned())
}
