//! Live quota-service client.

use super::model::{current_unix_seconds, QuotaInfo};
use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// The usage endpoint consumed by the official Codex authentication token.
const USAGE_ENDPOINT: &str = "https://chatgpt.com/backend-api/wham/usage";
/// The application identifier sent with quota requests.
const USER_AGENT: &str = "cxm-cli";
/// A quota request must not block the TUI refresh thread indefinitely.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
/// Quota-service default plan when the response leaves it unspecified.
const DEFAULT_PLAN_TYPE: &str = "free";

/// Fetches the primary-window quota associated with an authentication file.
pub(crate) fn fetch(auth_path: &Path) -> Result<QuotaInfo> {
    let content = fs::read_to_string(auth_path)?;
    let auth: AuthFile = serde_json::from_str(&content)?;
    let response = ureq::get(USAGE_ENDPOINT)
        .set(
            "Authorization",
            &format!("Bearer {}", auth.tokens.access_token),
        )
        .set("User-Agent", USER_AGENT)
        .timeout(REQUEST_TIMEOUT)
        .call()?;
    let usage: UsageResponse = response.into_json()?;
    Ok(QuotaInfo {
        plan_type: usage
            .plan_type
            .unwrap_or_else(|| DEFAULT_PLAN_TYPE.to_owned()),
        used_percent: usage
            .rate_limit
            .as_ref()
            .and_then(|limit| limit.primary_window.as_ref())
            .and_then(|window| window.used_percent)
            .unwrap_or_default(),
        limit_reached: usage
            .rate_limit
            .as_ref()
            .and_then(|limit| limit.limit_reached)
            .unwrap_or(false),
        fetched_at: current_unix_seconds()?,
        is_fresh: true,
    })
}

#[derive(Deserialize)]
struct AuthFile {
    tokens: AuthTokens,
}

#[derive(Deserialize)]
struct AuthTokens {
    access_token: String,
}

#[derive(Deserialize)]
struct UsageResponse {
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
