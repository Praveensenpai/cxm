//! Filesystem-backed saved-account operations.

use super::auth::email_from_auth_file;
use super::name::AccountName;
use super::profile::AccountProfile;
use crate::paths::CodexPaths;
use crate::quota::{clear_quota_cache, fetch_quota_cached, load_quota_cache, QuotaInfo};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Owns saved Codex account profiles and the active-profile marker.
#[derive(Debug, Clone)]
pub(crate) struct AccountStore {
    paths: CodexPaths,
}

impl AccountStore {
    /// Creates an account store using the standard user locations.
    pub(crate) fn for_current_user() -> Result<Self> {
        Ok(Self {
            paths: CodexPaths::for_current_user()?,
        })
    }

    /// Lists profiles with their non-expired cached quota data.
    pub(crate) fn list_cached(&self) -> Result<Vec<AccountProfile>> {
        let cache = load_quota_cache(&self.paths)?;
        self.list_with_quota(|name, _| {
            Ok(cache
                .get(name.as_str())
                .filter(|quota| !quota.is_expired())
                .cloned())
        })
    }

    /// Lists profiles, fetching quota data as needed.
    pub(crate) fn list_with_quota_refresh(
        &self,
        bypass_cache: bool,
    ) -> Result<Vec<AccountProfile>> {
        self.list_with_quota(|name, path| {
            fetch_quota_cached(&self.paths, name, path, bypass_cache).map(Some)
        })
    }

    /// Saves the active Codex authentication file under an alias or its email claim.
    pub(crate) fn save_active(&self, alias: Option<&str>) -> Result<AccountName> {
        let auth_path = self.paths.active_auth_path();
        if !auth_path.exists() {
            return Err(anyhow!("Codex auth file not found at {auth_path:?}"));
        }

        let name = alias
            .map(AccountName::parse)
            .transpose()?
            .or_else(|| email_from_auth_file(&auth_path).and_then(|email| AccountName::parse(&email).ok()))
            .ok_or_else(|| anyhow!("Could not extract email from id_token. Provide an alias with 'cxm save <alias>'"))?;

        fs::create_dir_all(self.paths.accounts_dir())?;
        let target = self.profile_path(&name);
        fs::copy(&auth_path, &target)
            .with_context(|| format!("Failed to save account file to {target:?}"))?;
        self.set_current(&name)?;
        clear_quota_cache(&self.paths)?;
        Ok(name)
    }

    /// Copies a saved profile into Codex's active authentication location.
    pub(crate) fn switch(&self, query: &str) -> Result<AccountName> {
        let target = self.find_matching_name(query)?;
        let source = self.profile_path(&target);
        if !source.exists() {
            return Err(anyhow!("Account '{query}' not found in saved accounts"));
        }

        fs::create_dir_all(self.paths.codex_dir())?;
        let active_auth = self.paths.active_auth_path();
        fs::copy(&source, &active_auth)
            .with_context(|| format!("Failed to copy account auth to {active_auth:?}"))?;
        self.set_current(&target)?;
        Ok(target)
    }

    /// Deletes a saved profile by its exact validated name.
    pub(crate) fn remove(&self, name: &AccountName) -> Result<()> {
        let path = self.profile_path(name);
        if !path.exists() {
            return Err(anyhow!("Account '{name}' not found"));
        }
        fs::remove_file(path)?;
        Ok(())
    }

    /// Saves the active profile when possible, then clears auth for a fresh Codex login.
    pub(crate) fn prepare_new_session(&self) -> Result<Option<AccountName>> {
        let auth_path = self.paths.active_auth_path();
        let saved = if auth_path.exists() {
            let name = self.save_active(None)?;
            fs::remove_file(&auth_path)?;
            Some(name)
        } else {
            None
        };

        let current_path = self.paths.current_account_path();
        if current_path.exists() {
            fs::remove_file(current_path)?;
        }
        Ok(saved)
    }

    fn list_with_quota<F>(&self, mut quota_for: F) -> Result<Vec<AccountProfile>>
    where
        F: FnMut(&AccountName, &Path) -> Result<Option<QuotaInfo>>,
    {
        let accounts_dir = self.paths.accounts_dir();
        if !accounts_dir.exists() {
            return Ok(Vec::new());
        }

        let active = self.current_name()?;
        let mut profiles = Vec::new();
        for entry in fs::read_dir(accounts_dir)? {
            let path = entry?.path();
            if let Some(name) = Self::profile_name_from_path(&path)? {
                let quota = quota_for(&name, &path)?;
                profiles.push(AccountProfile {
                    is_active: active.as_ref() == Some(&name),
                    name,
                    quota,
                });
            }
        }

        profiles.sort_by(|left, right| {
            let left_remaining = left.quota.as_ref().map_or(0, QuotaInfo::remaining_percent);
            let right_remaining = right.quota.as_ref().map_or(0, QuotaInfo::remaining_percent);
            right_remaining
                .cmp(&left_remaining)
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(profiles)
    }

    fn current_name(&self) -> Result<Option<AccountName>> {
        let current_path = self.paths.current_account_path();
        if current_path.exists() {
            let value = fs::read_to_string(current_path)?;
            return AccountName::parse(&value).map(Some);
        }

        Ok(email_from_auth_file(&self.paths.active_auth_path())
            .and_then(|email| AccountName::parse(&email).ok()))
    }

    fn set_current(&self, name: &AccountName) -> Result<()> {
        fs::create_dir_all(self.paths.accounts_dir())?;
        fs::write(self.paths.current_account_path(), name.as_str())?;
        Ok(())
    }

    fn find_matching_name(&self, query: &str) -> Result<AccountName> {
        let query_lowercase = query.to_lowercase();
        if let Some(profile) = self.list_cached()?.into_iter().find(|profile| {
            profile
                .name
                .as_str()
                .to_lowercase()
                .contains(&query_lowercase)
        }) {
            return Ok(profile.name);
        }

        AccountName::parse(query)
    }

    fn profile_path(&self, name: &AccountName) -> PathBuf {
        self.paths
            .accounts_dir()
            .join(format!("{}.json", name.as_str()))
    }

    fn profile_name_from_path(path: &Path) -> Result<Option<AccountName>> {
        if !path.is_file()
            || path.extension().and_then(|extension| extension.to_str()) != Some("json")
        {
            return Ok(None);
        }

        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            return Ok(None);
        };
        if stem.starts_with('.') {
            return Ok(None);
        }

        AccountName::parse(stem).map(Some)
    }
}
