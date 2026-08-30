//! Full-screen account and session interfaces.

mod accounts;
mod input;
mod sessions;
mod style;
mod terminal;

use crate::account::{AccountName, AccountStore};
use anyhow::Result;
use colored::Colorize;

/// Runs the dashboard and follows transitions between its account and session screens.
pub(crate) fn run_accounts_tui() -> Result<()> {
    let store = AccountStore::for_current_user()?;
    let mut view = View::Accounts;
    loop {
        let next_view = match view {
            View::Accounts => follow_account_action(accounts::run(store.clone())?, &store)?,
            View::Sessions => match sessions::run()? {
                sessions::SessionAction::Quit => None,
                sessions::SessionAction::ShowAccounts => Some(View::Accounts),
            },
        };
        let Some(next_view) = next_view else {
            return Ok(());
        };
        view = next_view;
    }
}

enum View {
    Accounts,
    Sessions,
}

fn follow_account_action(
    action: accounts::AccountAction,
    store: &AccountStore,
) -> Result<Option<View>> {
    match action {
        accounts::AccountAction::Quit => Ok(None),
        accounts::AccountAction::ShowSessions => Ok(Some(View::Sessions)),
        accounts::AccountAction::Switch(name) => {
            let switched = store.switch(&name)?;
            println!(
                "{} Switched to account: {}",
                "✔".green().bold(),
                switched.to_string().cyan().bold()
            );
            Ok(None)
        }
        accounts::AccountAction::Remove(name) => {
            let name = AccountName::parse(&name)?;
            store.remove(&name)?;
            println!(
                "{} Removed account: {}",
                "✔".green().bold(),
                name.to_string().yellow().bold()
            );
            Ok(None)
        }
        accounts::AccountAction::StartNewSession => {
            let saved = store.prepare_new_session()?;
            if let Some(name) = saved {
                println!(
                    "{} Saved active session as '{}'",
                    "✔".green().bold(),
                    name.to_string().cyan().bold()
                );
            }
            println!("{} Prepared fresh login session.", "✨".bold());
            println!(
                "👉 Run {} to log in to your new account.",
                "codex".bold().yellow()
            );
            println!("👉 Run {} when done to save it!", "cxm save".bold().cyan());
            Ok(None)
        }
    }
}
