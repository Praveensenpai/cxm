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
#[command(version = "0.6.7")]
#[command(about = "Codex Account Manager & Instant Switcher", long_about = None)]
struct Cli {
    /// Account name or email to switch to directly
    account: Option<String>,

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
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "cxm", &mut io::stdout());
        }
        None => ui::run_accounts_tui()?,
    }

    Ok(())
}
