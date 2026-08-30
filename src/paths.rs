//! Filesystem locations used by Codex and CXM.

use anyhow::{anyhow, Result};
use std::path::PathBuf;

/// Resolves the files CXM reads from and writes to for the current user.
#[derive(Debug, Clone)]
pub(crate) struct CodexPaths {
    codex_dir: PathBuf,
    accounts_dir: PathBuf,
}

impl CodexPaths {
    /// Builds the standard Codex and CXM storage locations from the user's home directory.
    pub(crate) fn for_current_user() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not resolve home directory"))?;

        Ok(Self {
            codex_dir: home.join(".codex"),
            accounts_dir: home.join(".codex-accounts"),
        })
    }

    /// Returns the directory holding Codex configuration and history.
    pub(crate) fn codex_dir(&self) -> &PathBuf {
        &self.codex_dir
    }

    /// Returns the directory holding saved CXM account profiles.
    pub(crate) fn accounts_dir(&self) -> &PathBuf {
        &self.accounts_dir
    }

    /// Returns the active Codex authentication file.
    pub(crate) fn active_auth_path(&self) -> PathBuf {
        self.codex_dir.join("auth.json")
    }

    /// Returns the account profile selected by CXM.
    pub(crate) fn current_account_path(&self) -> PathBuf {
        self.accounts_dir.join(".current")
    }

    /// Returns CXM's persisted quota cache.
    pub(crate) fn quota_cache_path(&self) -> PathBuf {
        self.accounts_dir.join(".quota_cache.json")
    }

    /// Returns Codex's session history file.
    pub(crate) fn history_path(&self) -> PathBuf {
        self.codex_dir.join("history.jsonl")
    }
}
