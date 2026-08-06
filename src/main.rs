mod account;

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
#[command(version = "0.1.1")]
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
    /// List all saved Codex accounts
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
            Err(e) => {
                println!("{} {}", "✘ Could not auto-save current account:".red(), e);
                println!("Log in via Codex first, then run 'cxm save'");
            }
        }
        return Ok(());
    }

    let mut options: Vec<String> = accounts
        .iter()
        .map(|acc| {
            if acc.is_active {
                format!("{} {}", acc.name, "(active)".green().bold())
            } else {
                acc.name.clone()
            }
        })
        .collect();

    options.push(" Save Current Account".blue().to_string());

    let ans = Select::new("Select Codex Account:", options).prompt();

    match ans {
        Ok(choice) => {
            if choice.contains("Save Current Account") {
                let name = save_current_account(None)?;
                println!("{} Saved active account as '{}'", "✔".green().bold(), name.bold().cyan());
            } else {
                let clean_name = choice.replace(" (active)", "").trim().to_string();
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
        let email_str = acc.email.as_deref().unwrap_or(&acc.name);
        if acc.is_active {
            println!("  {} {} ({}) {}", "*".green().bold(), acc.name.bold().cyan(), email_str, "(active)".green());
        } else {
            println!("    {} ({})", acc.name, email_str);
        }
    }

    Ok(())
}
