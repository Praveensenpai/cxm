//! Codex session-history discovery and presentation.

mod history;
mod text;

use history::{format_local_datetime, SessionHistory};
use text::NEW_CONVERSATION_SUMMARY;

use crate::paths::CodexPaths;
use anyhow::Result;

pub(crate) use text::{clean_user_text, sanitize_summary};

/// A resumable Codex session with display-ready metadata.
#[derive(Debug, Clone)]
pub(crate) struct CodexSessionInfo {
    /// The complete Codex session identifier passed to `codex resume`.
    pub(crate) session_id: String,
    /// A compact identifier suitable for a table row.
    pub(crate) short_id: String,
    /// The latest interaction time in the local timezone.
    pub(crate) datetime: String,
    /// The latest interaction time as Unix seconds, used for sorting.
    pub(crate) timestamp: u64,
    /// A concise summary of the first meaningful prompt.
    pub(crate) summary: String,
    /// The full cleaned prompt shown in the preview panel.
    pub(crate) full_prompt: String,
}

impl CodexSessionInfo {
    fn from_history(session_id: String, history: SessionHistory) -> Self {
        let short_id = session_id.chars().take(8).collect();
        let full_prompt = clean_user_text(&history.first_prompt);
        Self {
            short_id,
            datetime: format_local_datetime(history.latest_timestamp),
            timestamp: history.latest_timestamp,
            summary: sanitize_summary(&history.first_prompt),
            full_prompt: if full_prompt.is_empty() {
                NEW_CONVERSATION_SUMMARY.to_owned()
            } else {
                full_prompt
            },
            session_id,
        }
    }
}

/// Reads Codex session history for the current user.
pub(crate) fn scan_codex_sessions() -> Result<Vec<CodexSessionInfo>> {
    history::scan(&CodexPaths::for_current_user()?)
}
