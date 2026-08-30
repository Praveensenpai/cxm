//! Quota data shown in the account dashboard.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// The maximum percentage that can be consumed from a quota window.
const FULL_QUOTA_PERCENT: u32 = 100;
/// How long a persisted quota result can be shown without a refresh.
const CACHE_TTL_SECONDS: u64 = 5 * 60;

/// The live or cached quota status for one Codex account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QuotaInfo {
    /// The account's plan label returned by the usage service.
    pub(crate) plan_type: String,
    /// The percentage used in the primary quota window.
    pub(crate) used_percent: u32,
    /// Whether the service reports that this window is exhausted.
    pub(crate) limit_reached: bool,
    /// The Unix timestamp at which this data was fetched.
    #[serde(default)]
    pub(crate) fetched_at: u64,
    /// Whether this screen load fetched the value from the service.
    #[serde(skip)]
    pub(crate) is_fresh: bool,
}

impl QuotaInfo {
    /// Returns the remaining quota percentage, clamped at zero for malformed responses.
    pub(crate) fn remaining_percent(&self) -> u32 {
        FULL_QUOTA_PERCENT.saturating_sub(self.used_percent)
    }

    /// Builds the compact quota label displayed in account rows.
    pub(crate) fn display_badge(&self) -> String {
        let remaining = self.remaining_percent();
        if self.limit_reached || remaining == 0 {
            format!("[{} | limit reached]", self.plan_type)
        } else {
            format!("[{} | {remaining}% left]", self.plan_type)
        }
    }

    /// Returns whether the cached value is older than the refresh interval.
    pub(crate) fn is_expired(&self) -> bool {
        current_unix_seconds()
            .map(|now| now.saturating_sub(self.fetched_at) > CACHE_TTL_SECONDS)
            .unwrap_or(true)
    }
}

/// Returns the current Unix timestamp, reporting clock configuration errors to callers.
pub(crate) fn current_unix_seconds() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System clock is before the Unix epoch")
        .map(|duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::QuotaInfo;

    #[test]
    fn remaining_quota_is_clamped_for_invalid_service_values() {
        let quota = QuotaInfo {
            plan_type: "free".to_owned(),
            used_percent: 140,
            limit_reached: false,
            fetched_at: 0,
            is_fresh: false,
        };

        assert_eq!(quota.remaining_percent(), 0);
        assert_eq!(quota.display_badge(), "[free | limit reached]");
    }
}
