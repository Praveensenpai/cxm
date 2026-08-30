//! Presentation cleanup for raw Codex prompt text.

/// The fallback shown for a session with no user prompt.
pub(super) const NEW_CONVERSATION_SUMMARY: &str = "New Conversation";

/// Removes Codex metadata and shell-artifact lines from a user prompt.
pub(crate) fn clean_user_text(raw: &str) -> String {
    let without_tags = remove_metadata_tags(raw);
    without_tags
        .lines()
        .map(str::trim)
        .filter(is_user_content)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Produces a compact, single-line prompt summary.
pub(crate) fn sanitize_summary(raw: &str) -> String {
    let summary = clean_user_text(raw)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if summary.is_empty() {
        NEW_CONVERSATION_SUMMARY.to_owned()
    } else {
        summary
    }
}

fn remove_metadata_tags(raw: &str) -> String {
    const TAGS: [&str; 8] = [
        "<USER_REQUEST>",
        "</USER_REQUEST>",
        "<USER_SETTINGS_CHANGE>",
        "</USER_SETTINGS_CHANGE>",
        "<ADDITIONAL_METADATA>",
        "</ADDITIONAL_METADATA>",
        "<EPHEMERAL_MESSAGE>",
        "</EPHEMERAL_MESSAGE>",
    ];

    TAGS.into_iter()
        .fold(raw.to_owned(), |text, tag| text.replace(tag, ""))
}

fn is_user_content(line: &&str) -> bool {
    !line.is_empty()
        && !line.starts_with('<')
        && !line.starts_with("The current local time is:")
        && !line.starts_with("The user changed setting")
        && !line.starts_with("The user has uploaded")
        && !line.starts_with("┌─")
        && !line.starts_with("└─")
        && !line.starts_with('│')
        && !line.starts_with("~ ❯")
        && !line.starts_with("~ ✗")
}

#[cfg(test)]
mod tests {
    use super::{clean_user_text, sanitize_summary, NEW_CONVERSATION_SUMMARY};

    #[test]
    fn removes_metadata_and_shell_artifacts() {
        let raw = "<USER_REQUEST>\n  Build this\n</USER_REQUEST>\n~ ❯ command\n";

        assert_eq!(clean_user_text(raw), "Build this");
        assert_eq!(sanitize_summary(raw), "Build this");
    }

    #[test]
    fn uses_a_stable_fallback_for_empty_prompts() {
        assert_eq!(
            sanitize_summary("<EPHEMERAL_MESSAGE></EPHEMERAL_MESSAGE>"),
            NEW_CONVERSATION_SUMMARY
        );
    }
}
