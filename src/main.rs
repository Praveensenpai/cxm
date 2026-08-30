mod account;
mod paths;
mod quota;
mod session;
mod ui;

use account::AccountStore;
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use std::io;

#[derive(Parser)]
#[command(name = "cxm")]
#[command(author = "Praveensenpai")]
#[command(disable_version_flag = true)]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Codex Account Manager & Instant Switcher", long_about = None)]
struct Cli {
    /// Show the application version
    #[arg(
        short = 'v',
        long = "version",
        action = clap::ArgAction::Version
    )]
    version: Option<bool>,

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
        let account = AccountStore::for_current_user()?.switch(&target)?;
        println!(
            "{} Switched to account: {}",
            "✔".green().bold(),
            account.to_string().cyan().bold()
        );
        return Ok(());
    }

    match cli.command {
        Some(Commands::Save { alias }) => {
            let name = AccountStore::for_current_user()?.save_active(alias.as_deref())?;
            println!(
                "{} Saved current Codex account as '{}'",
                "✔".green().bold(),
                name.to_string().bold().cyan()
            );
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "cxm", &mut io::stdout());
        }
        None => ui::run_accounts_tui()?,
    }

    Ok(())
}
