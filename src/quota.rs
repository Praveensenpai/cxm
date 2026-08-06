use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct QuotaInfo {
    pub plan_type: String,
    pub used_percent: u32,
    pub limit_reached: bool,
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

pub fn fetch_quota_for_auth_file(auth_path: &Path) -> Result<QuotaInfo> {
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

    Ok(QuotaInfo {
        plan_type,
        used_percent,
        limit_reached,
    })
}
