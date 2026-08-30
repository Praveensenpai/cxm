//! Live quota retrieval and cached quota storage.

mod cache;
mod client;
mod model;

use crate::account::AccountName;
use crate::paths::CodexPaths;
use anyhow::Result;
use std::path::Path;

pub(crate) use model::QuotaInfo;

/// Removes all persisted quota data.
pub(crate) fn clear_quota_cache(paths: &CodexPaths) -> Result<()> {
    cache::clear(paths)
}

/// Loads all persisted quota data.
pub(crate) fn load_quota_cache(
    paths: &CodexPaths,
) -> Result<std::collections::HashMap<String, QuotaInfo>> {
    cache::load(paths)
}

/// Loads a cached quota value or fetches and persists a live replacement.
pub(crate) fn fetch_quota_cached(
    paths: &CodexPaths,
    account: &AccountName,
    auth_path: &Path,
    bypass_cache: bool,
) -> Result<QuotaInfo> {
    let mut cached = cache::load(paths)?;
    if !bypass_cache {
        if let Some(mut quota) = cached
            .get(account.as_str())
            .cloned()
            .filter(|quota| !quota.is_expired())
        {
            quota.is_fresh = false;
            return Ok(quota);
        }
    }

    let quota = client::fetch(auth_path)?;
    cached.insert(account.as_str().to_owned(), quota.clone());
    cache::save(paths, &cached)?;
    Ok(quota)
}
