//! Saved account-profile data used by the account list.

use super::name::AccountName;
use crate::quota::QuotaInfo;

/// An account profile together with the data shown in the account screen.
#[derive(Debug, Clone)]
pub(crate) struct AccountProfile {
    /// The stable saved-profile identifier.
    pub(crate) name: AccountName,
    /// Whether this is the currently active Codex profile.
    pub(crate) is_active: bool,
    /// The latest cached or live quota information, when available.
    pub(crate) quota: Option<QuotaInfo>,
}
