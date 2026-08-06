use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_TTL_SECONDS: u64 = 300; // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub plan_type: String,
    pub used_percent: u32,
    pub limit_reached: bool,
    #[serde(default)]
    pub fetched_at: u64,
    #[serde(skip)]
    pub is_fresh: bool,
}

impl QuotaInfo {
    pub fn remaining_percent(&self) -> u32 {
        if self.used_percent >= 100 {
            0
        } else {
            100 - self.used_percent
        }
    }

    pub fn display_badge(&self) -> String {
        let rem = self.remaining_percent();
        if self.limit_reached || rem == 0 {
            format!("[{} | limit reached]", self.plan_type)
        } else {
            format!("[{} | {}% left]", self.plan_type, rem)
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.fetched_at) > CACHE_TTL_SECONDS
    }

    pub fn formatted_time(&self) -> String {
        let naive = DateTime::from_timestamp(self.fetched_at as i64, 0);
        match naive {
            Some(utc) => {
                let local: DateTime<Local> = DateTime::from(utc);
                local.format("%Y-%m-%d %H:%M:%S").to_string()
            }
            None => "unknown time".to_string(),
        }
    }
}

pub fn get_cache_file_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".codex-accounts").join(".quota_cache.json"))
}

pub fn load_quota_cache() -> HashMap<String, QuotaInfo> {
    let path = match get_cache_file_path() {
        Some(p) => p,
        None => return HashMap::new(),
    };

    if !path.exists() {
        return HashMap::new();
    }

    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_quota_cache(cache: &HashMap<String, QuotaInfo>) {
    if let Some(path) = get_cache_file_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(cache) {
            let _ = fs::write(path, content);
        }
    }
}

pub fn fetch_quota_cached(
    account_key: &str,
    auth_path: &Path,
    no_cache: bool,
) -> Result<QuotaInfo> {
    let mut cache = load_quota_cache();

    if !no_cache {
        if let Some(mut cached) = cache.get(account_key).cloned() {
            if !cached.is_expired() {
                cached.is_fresh = false;
                return Ok(cached);
            }
        }
    }

    let mut quota = fetch_quota_live(auth_path)?;
    quota.is_fresh = true;
    cache.insert(account_key.to_string(), quota.clone());
    save_quota_cache(&cache);

    Ok(quota)
}

#[derive(Deserialize)]
struct WhamResponse {
    plan_type: Option<String>,
    rate_limit: Option<RateLimit>,
}

#[derive(Deserialize)]
struct RateLimit {
    limit_reached: Option<bool>,
    primary_window: Option<PrimaryWindow>,
}

#[derive(Deserialize)]
struct PrimaryWindow {
    used_percent: Option<u32>,
}

fn fetch_quota_live(auth_path: &Path) -> Result<QuotaInfo> {
    let content = fs::read_to_string(auth_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    let access_token = json
        .get("tokens")
        .and_then(|t| t.get("access_token"))
        .and_then(|s| s.as_str())
        .ok_or_else(|| anyhow!("No access token found in auth file"))?;

    let resp = ureq::get("https://chatgpt.com/backend-api/wham/usage")
        .set("Authorization", &format!("Bearer {}", access_token))
        .set("User-Agent", "cxm-cli")
        .timeout(std::time::Duration::from_secs(3))
        .call()?;

    let wham: WhamResponse = resp.into_json()?;
    let plan_type = wham.plan_type.unwrap_or_else(|| "free".to_string());
    let rate_limit = wham.rate_limit;
    let limit_reached = rate_limit.as_ref().and_then(|r| r.limit_reached).unwrap_or(false);
    let used_percent = rate_limit
        .as_ref()
        .and_then(|r| r.primary_window.as_ref())
        .and_then(|w| w.used_percent)
        .unwrap_or(0);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(QuotaInfo {
        plan_type,
        used_percent,
        limit_reached,
        fetched_at: now,
        is_fresh: true,
    })
}
