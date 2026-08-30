//! Persisted quota-cache operations.

use super::model::QuotaInfo;
use crate::paths::CodexPaths;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;

/// Loads persisted quota data, treating a missing cache as empty.
pub(crate) fn load(paths: &CodexPaths) -> Result<HashMap<String, QuotaInfo>> {
    let path = paths.quota_cache_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse quota cache at {path:?}")),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(HashMap::new()),
        Err(error) => Err(error).with_context(|| format!("Failed to read quota cache at {path:?}")),
    }
}

/// Persists quota data for future dashboard loads.
pub(crate) fn save(paths: &CodexPaths, cache: &HashMap<String, QuotaInfo>) -> Result<()> {
    fs::create_dir_all(paths.accounts_dir())?;
    let path = paths.quota_cache_path();
    let content = serde_json::to_string_pretty(cache)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write quota cache at {path:?}"))?;
    Ok(())
}

/// Removes persisted quota data, treating a missing cache as already cleared.
pub(crate) fn clear(paths: &CodexPaths) -> Result<()> {
    let path = paths.quota_cache_path();
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("Failed to clear quota cache at {path:?}"))
        }
    }
}
