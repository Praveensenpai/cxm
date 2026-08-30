//! Shared keyboard and filtering helpers for TUI tables.

use ratatui::widgets::TableState;

/// Moves table selection one row, wrapping around the available rows.
pub(super) fn move_selection(state: &mut TableState, row_count: usize, forward: bool) {
    if row_count == 0 {
        state.select(None);
        return;
    }

    let selected = state.selected().unwrap_or_default();
    let next = if forward {
        (selected + 1) % row_count
    } else {
        selected.checked_sub(1).unwrap_or(row_count - 1)
    };
    state.select(Some(next));
}

/// Clears a selection whose index no longer exists after filtering.
pub(super) fn keep_selection_in_bounds(state: &mut TableState, row_count: usize) {
    if row_count == 0 {
        state.select(None);
    } else if state
        .selected()
        .is_none_or(|selected| selected >= row_count)
    {
        state.select(Some(0));
    }
}

/// Returns whether a query is empty or occurs case-insensitively in a candidate string.
pub(super) fn matches_query(query: &str, candidate: &str) -> bool {
    query.is_empty() || candidate.to_lowercase().contains(&query.to_lowercase())
}
