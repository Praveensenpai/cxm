use crate::quota::{fetch_quota_cached, load_quota_cache, QuotaInfo};
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use colored::Colorize;
use inquire::Select;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub name: String,
    pub _email: Option<String>,
    pub is_active: bool,
    pub quota: Option<QuotaInfo>,
    pub _file_path: PathBuf,
}

pub fn get_codex_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not resolve home directory"))?;
    Ok(home.join(".codex"))
}

pub fn get_accounts_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not resolve home directory"))?;
    Ok(home.join(".codex-accounts"))
}

pub fn get_active_auth_path() -> Result<PathBuf> {
    Ok(get_codex_dir()?.join("auth.json"))
}

pub fn decode_jwt_email(id_token: &str) -> Option<String> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    let mut payload_b64 = parts[1].to_string();
    while payload_b64.len() % 4 != 0 {
        payload_b64.push('=');
    }

    let decoded = URL_SAFE_NO_PAD
        .decode(payload_b64.as_bytes())
        .ok()
        .or_else(|| STANDARD.decode(payload_b64.as_bytes()).ok())?;

    let json: Value = serde_json::from_slice(&decoded).ok()?;
    json.get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub fn extract_email_from_auth_file(auth_path: &PathBuf) -> Option<String> {
    let content = fs::read_to_string(auth_path).ok()?;
    let json: Value = serde_json::from_str(&content).ok()?;
    let id_token = json.get("tokens")?.get("id_token")?.as_str()?;
    decode_jwt_email(id_token)
}

pub fn get_current_active_account() -> Option<String> {
    let accounts_dir = get_accounts_dir().ok()?;
    let current_file = accounts_dir.join(".current");
    if current_file.exists() {
        if let Ok(name) = fs::read_to_string(current_file) {
            let name = name.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }

    let auth_path = get_active_auth_path().ok()?;
    extract_email_from_auth_file(&auth_path)
}

pub fn set_current_active_account(name: &str) -> Result<()> {
    let accounts_dir = get_accounts_dir()?;
    fs::create_dir_all(&accounts_dir)?;
    fs::write(accounts_dir.join(".current"), name)?;
    Ok(())
}

pub fn save_current_account(alias: Option<&str>) -> Result<String> {
    let auth_path = get_active_auth_path()?;
    if !auth_path.exists() {
        return Err(anyhow!("Codex auth file not found at {:?}", auth_path));
    }

    let account_name = match alias {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        _ => extract_email_from_auth_file(&auth_path).ok_or_else(|| {
            anyhow!(
                "Could not extract email from id_token. Provide an alias with 'cxm save <alias>'"
            )
        })?,
    };

    let accounts_dir = get_accounts_dir()?;
    fs::create_dir_all(&accounts_dir)?;

    let target_path = accounts_dir.join(format!("{}.json", account_name));
    fs::copy(&auth_path, &target_path)
        .with_context(|| format!("Failed to save account file to {:?}", target_path))?;

    set_current_active_account(&account_name)?;
    Ok(account_name)
}

fn list_accounts_with<F>(mut quota_for: F) -> Result<Vec<AccountInfo>>
where
    F: FnMut(&str, &PathBuf) -> Option<QuotaInfo>,
{
    let accounts_dir = get_accounts_dir()?;
    if !accounts_dir.exists() {
        return Ok(Vec::new());
    }

    let active_account = get_current_active_account();
    let mut accounts = Vec::new();

    for entry in fs::read_dir(&accounts_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.starts_with('.') {
                    continue;
                }
                let email = extract_email_from_auth_file(&path);
                let is_active = active_account.as_deref() == Some(stem);
                let quota = quota_for(stem, &path);
                accounts.push(AccountInfo {
                    name: stem.to_string(),
                    _email: email,
                    is_active,
                    quota,
                    _file_path: path,
                });
            }
        }
    }

    accounts.sort_by(|a, b| {
        let rem_a = a.quota.as_ref().map(|q| q.remaining_percent()).unwrap_or(0);
        let rem_b = b.quota.as_ref().map(|q| q.remaining_percent()).unwrap_or(0);
        rem_b.cmp(&rem_a).then_with(|| a.name.cmp(&b.name))
    });
    Ok(accounts)
}

pub fn list_accounts(no_cache: bool) -> Result<Vec<AccountInfo>> {
    list_accounts_with(|stem, path| fetch_quota_cached(stem, path, no_cache).ok())
}

/// Load account rows and only non-expired quota data from disk.
pub fn list_accounts_cached() -> Result<Vec<AccountInfo>> {
    let cache = load_quota_cache();
    list_accounts_with(|stem, _| cache.get(stem).filter(|quota| !quota.is_expired()).cloned())
}

pub fn switch_account(account_name: &str) -> Result<()> {
    let accounts = list_accounts(false)?;
    let target = accounts.iter().find(|acc| {
        acc.name
            .to_lowercase()
            .contains(&account_name.to_lowercase())
    });

    let target_name = match target {
        Some(acc) => &acc.name,
        None => account_name,
    };

    let accounts_dir = get_accounts_dir()?;
    let target_path = accounts_dir.join(format!("{}.json", target_name));

    if !target_path.exists() {
        return Err(anyhow!(
            "Account '{}' not found in saved accounts",
            account_name
        ));
    }

    let codex_dir = get_codex_dir()?;
    fs::create_dir_all(&codex_dir)?;

    let auth_path = get_active_auth_path()?;
    fs::copy(&target_path, &auth_path)
        .with_context(|| format!("Failed to copy account auth to {:?}", auth_path))?;

    set_current_active_account(target_name)?;
    println!(
        "{} Switched to account: {}",
        "✔".green().bold(),
        target_name.bold().cyan()
    );
    Ok(())
}

pub fn remove_account(account_name: &str) -> Result<()> {
    let accounts_dir = get_accounts_dir()?;
    let target_path = accounts_dir.join(format!("{}.json", account_name));

    if !target_path.exists() {
        return Err(anyhow!("Account '{}' not found", account_name));
    }

    fs::remove_file(&target_path)?;
    println!(
        "{} Removed account: {}",
        "✔".green().bold(),
        account_name.bold().yellow()
    );
    Ok(())
}

pub fn interactive_remove_account() -> Result<()> {
    let accounts = list_accounts(false)?;
    if accounts.is_empty() {
        println!("{}", "No saved accounts to remove.".yellow());
        return Ok(());
    }

    let options: Vec<String> = accounts.iter().map(|acc| acc.name.clone()).collect();
    let ans = Select::new("🗑️ Select Account to Delete:", options).prompt();

    match ans {
        Ok(selected) => {
            remove_account(&selected)?;
        }
        Err(_) => {
            println!("Operation cancelled.");
        }
    }

    Ok(())
}

pub fn prepare_new_session() -> Result<()> {
    let auth_path = get_active_auth_path()?;
    if auth_path.exists() {
        if let Ok(saved_name) = save_current_account(None) {
            println!(
                "{} Saved active session as '{}'",
                "✔".green().bold(),
                saved_name.bold().cyan()
            );
        }
        let _ = fs::remove_file(&auth_path);
    }

    let accounts_dir = get_accounts_dir()?;
    let current_file = accounts_dir.join(".current");
    if current_file.exists() {
        let _ = fs::remove_file(current_file);
    }

    println!("{} Prepared fresh login session.", "✨".bold());
    println!(
        "👉 Run {} to log in to your new account.",
        "codex".bold().yellow()
    );
    println!(
        "👉 Run {} (or {}) when done to save it!",
        "cxm save".bold().cyan(),
        "cxm".bold().cyan()
    );

    Ok(())
}
