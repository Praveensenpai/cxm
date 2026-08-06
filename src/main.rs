mod account;
mod quota;
mod session;
mod ui;

use account::*;
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use std::io;

#[derive(Parser)]
#[command(name = "cxm")]
#[command(author = "Praveensenpai")]
#[command(version = "0.6.3")]
#[command(about = "Codex Account Manager & Instant Switcher", long_about = None)]
struct Cli {
    /// Account name or email to switch to directly
    account: Option<String>,

    /// Bypass quota cache and fetch live quota from backend API
    #[arg(short = 'n', long = "no-cache", global = true)]
    no_cache: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Save current Codex auth state as a named account profile
    Save {
        /// Optional alias for account (defaults to JWT email)
        alias: Option<String>,
    },
    /// Back up active account and prepare a fresh session to log in to a new account
    #[command(alias = "add")]
    New,
    /// List all saved Codex accounts with usage quota
    List {
        /// Bypass quota cache and fetch live quota from backend API
        #[arg(short = 'n', long = "no-cache")]
        no_cache: bool,
    },
    /// Generate shell autocompletion scripts (bash, zsh, fish, powershell, elvish)
    Completions {
        /// Target shell for autocompletions
        shell: Shell,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(target) = cli.account {
        return switch_account(&target);
    }

    match cli.command {
        Some(Commands::Save { alias }) => {
            let name = save_current_account(alias.as_deref())?;
            println!("{} Saved current Codex account as '{}'", "✔".green().bold(), name.bold().cyan());
        }
        Some(Commands::New) => prepare_new_session()?,
        Some(Commands::List { no_cache }) => list_all_accounts(cli.no_cache || no_cache)?,
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "cxm", &mut io::stdout());
        }
        None => ui::run_accounts_tui(cli.no_cache)?,
    }

    Ok(())
}

fn list_all_accounts(no_cache: bool) -> Result<()> {
    if no_cache {
        println!("{}", "⏳ Fetching live account quotas from backend API...".yellow());
    }

    let accounts = list_accounts(no_cache)?;

    if accounts.is_empty() {
        println!("{}", "No saved accounts.".yellow());
        return Ok(());
    }

    println!("{}", "Saved Codex Accounts:".bold().underline());
    let mut latest_fetch_time: Option<String> = None;
    let mut is_any_fresh = false;

    for acc in &accounts {
        let quota_badge = acc
            .quota
            .as_ref()
            .map(|q| {
                if q.is_fresh {
                    is_any_fresh = true;
                }
                if latest_fetch_time.is_none() || q.is_fresh {
                    latest_fetch_time = Some(q.formatted_time());
                }
                q.display_badge().cyan().to_string()
            })
            .unwrap_or_else(|| "[quota unavailable]".dimmed().to_string());

        if acc.is_active {
            println!(
                "  {} {} {} {}",
                "*".green().bold(),
                acc.name.bold().magenta(),
                "(active)".green(),
                quota_badge
            );
        } else {
            println!("    {} {}", acc.name, quota_badge);
        }
    }

    if let Some(timestamp) = latest_fetch_time {
        println!();
        if is_any_fresh {
            println!(
                "{} Quota data: {} • Updated: {}",
                "ℹ".blue().bold(),
                "fresh (live)".green().bold(),
                timestamp.bold()
            );
        } else {
            println!(
                "{} Quota data: {} • Last updated: {}",
                "ℹ".blue().bold(),
                "cached (5-min TTL)".yellow(),
                timestamp.dimmed()
            );
        }
    }

    Ok(())
}
