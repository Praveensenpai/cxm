mod account;
mod quota;

use account::*;
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use inquire::Select;
use std::io;

#[derive(Parser)]
#[command(name = "cxm")]
#[command(author = "Praveensenpai")]
#[command(version = "0.2.0")]
#[command(about = "Codex Account Manager & Instant Switcher", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Switch to a Codex account interactively or by name
    Switch {
        /// Account name or email to switch to
        account: Option<String>,
    },
    /// Save current Codex auth state as a named account profile
    Save {
        /// Optional alias for account (defaults to JWT email)
        alias: Option<String>,
    },
    /// Back up active account and prepare a fresh session to log in to a new account
    #[command(alias = "add")]
    New,
    /// List all saved Codex accounts with usage quota
    List,
    /// Remove a saved Codex account
    Remove {
        /// Account name or email to remove
        account: String,
    },
    /// Generate shell autocompletion scripts (bash, zsh, fish, powershell, elvish)
    Completions {
        /// Target shell for autocompletions
        shell: Shell,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Switch { account }) => match account {
            Some(name) => switch_account(&name)?,
            None => interactive_switch()?,
        },
        Some(Commands::Save { alias }) => {
            let name = save_current_account(alias.as_deref())?;
            println!("{} Saved current Codex account as '{}'", "✔".green().bold(), name.bold().cyan());
        }
        Some(Commands::New) => prepare_new_session()?,
        Some(Commands::List) => list_all_accounts()?,
        Some(Commands::Remove { account }) => remove_account(&account)?,
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "cxm", &mut io::stdout());
        }
        None => interactive_switch()?,
    }

    Ok(())
}

fn interactive_switch() -> Result<()> {
    let accounts = list_accounts()?;

    if accounts.is_empty() {
        println!("{}", "No saved Codex accounts found.".yellow());
        println!("Saving your current Codex auth state...");
        match save_current_account(None) {
            Ok(name) => {
                println!("{} Saved active account as '{}'", "✔".green().bold(), name.bold().cyan());
            }
            Err(_) => {
                println!("No active Codex auth session found. Preparing fresh session...");
                prepare_new_session()?;
            }
        }
        return Ok(());
    }

    let mut options: Vec<String> = accounts
        .iter()
        .map(|acc| {
            let quota_badge = acc
                .quota
                .as_ref()
                .map(|q| q.display_badge())
                .unwrap_or_default();

            if acc.is_active {
                format!("{} {} {}", acc.name, "(active)".green().bold(), quota_badge.dimmed())
            } else {
                format!("{} {}", acc.name, quota_badge.dimmed())
            }
        })
        .collect();

    options.push("💾 Save Current Account".blue().to_string());
    options.push("➕ New Session (Log into new account)".yellow().to_string());

    let ans = Select::new("Select Codex Account:", options).prompt();

    match ans {
        Ok(choice) => {
            if choice.contains("Save Current Account") {
                let name = save_current_account(None)?;
                println!("{} Saved active account as '{}'", "✔".green().bold(), name.bold().cyan());
            } else if choice.contains("New Session") {
                prepare_new_session()?;
            } else {
                let clean_name = choice
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                switch_account(&clean_name)?;
            }
        }
        Err(_) => {
            println!("Operation cancelled.");
        }
    }

    Ok(())
}

fn list_all_accounts() -> Result<()> {
    let accounts = list_accounts()?;

    if accounts.is_empty() {
        println!("{}", "No saved accounts.".yellow());
        return Ok(());
    }

    println!("{}", "Saved Codex Accounts:".bold().underline());
    for acc in accounts {
        let quota_badge = acc
            .quota
            .as_ref()
            .map(|q| q.display_badge().cyan().to_string())
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

    Ok(())
}
