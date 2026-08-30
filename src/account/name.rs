//! Account-profile identifiers.

use anyhow::{anyhow, Result};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// A validated account-profile name that is safe to use in a profile filename.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct AccountName(String);

impl AccountName {
    /// Validates and stores an account-profile name.
    pub(crate) fn parse(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Account name cannot be empty"));
        }

        if trimmed == "." || trimmed == ".." || trimmed.contains(['/', '\\', '\0']) {
            return Err(anyhow!("Account name must not contain a path separator"));
        }

        Ok(Self(trimmed.to_owned()))
    }

    /// Returns the account-profile name as text.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for AccountName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for AccountName {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::parse(value)
    }
}

#[cfg(test)]
mod tests {
    use super::AccountName;

    #[test]
    fn accepts_an_email_profile_name() {
        let name = AccountName::parse(" user@example.com ").expect("email names are valid");

        assert_eq!(name.as_str(), "user@example.com");
    }

    #[test]
    fn rejects_path_traversal_names() {
        assert!(AccountName::parse("../other-profile").is_err());
        assert!(AccountName::parse("nested/profile").is_err());
    }
}
