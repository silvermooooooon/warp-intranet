//! Immutable policy for the terminal-only intranet distribution.
//!
//! No environment variable, feature flag, saved account, proxy, URL or release channel can
//! authorize application-owned networking. Shell processes and Unix-domain IPC are not governed
//! by this policy; the user's shell remains free to use SSH, Git and other intranet services.

use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetworkDisabled;

impl fmt::Display for NetworkDisabled {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("application networking is disabled in Warp Intranet")
    }
}

impl std::error::Error for NetworkDisabled {}

/// Whether application-owned network services may be started.
pub const fn is_network_enabled() -> bool {
    false
}

/// Reject an operation before constructing or invoking its network transport.
pub const fn require_network() -> Result<(), NetworkDisabled> {
    Err(NetworkDisabled)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
