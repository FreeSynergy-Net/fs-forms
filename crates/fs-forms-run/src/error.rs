//! What can go wrong in the host, each with a code (`host-error-*`, MF2 keys in `texts/`), parameters and the exit code it earns.
//!
//! Pattern: Value objects + closed code list (append only). A fatal error ends the process with 3 so that the starter shows its
//! message and starts the standard frontend (ADR 0094 4); the others are shown in the window, never silently.

use std::fmt;

/// The exit code for "the layouts or their contract are not there": the starter reads it.
pub const EXIT_CONTRACT: u8 = 3;

/// An error of the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostError {
    /// No socket was named and there is no default (no `$XDG_RUNTIME_DIR`).
    NoSocket,
    /// Nothing answers at the socket of `fs-forms serve`.
    LayoutsUnreachable {
        /// The socket.
        path: String,
    },
    /// The server answers, but does not offer the layouts contract this host speaks.
    LayoutsContractMissing {
        /// The socket.
        path: String,
    },
    /// The program has no layout for this frontend.
    LayoutMissing {
        /// The program.
        extends: String,
        /// The frontend.
        frontend: String,
    },
    /// The store of layouts is in emergency mode (`layout-error-store-broken` or `-store-newer`, from `health`).
    LayoutStore {
        /// The code of the store.
        code: String,
    },
}

impl HostError {
    /// The code without prefix: the MF2 key is `host-error-<code>`.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoSocket => "no-socket",
            Self::LayoutsUnreachable { .. } => "layouts-unreachable",
            Self::LayoutsContractMissing { .. } => "layouts-contract-missing",
            Self::LayoutMissing { .. } => "layout-missing",
            Self::LayoutStore { .. } => "layout-store",
        }
    }

    /// The named parameters.
    #[must_use]
    pub fn params(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::NoSocket => Vec::new(),
            Self::LayoutsUnreachable { path } | Self::LayoutsContractMissing { path } => {
                vec![("path", path.clone())]
            }
            Self::LayoutMissing { extends, frontend } => {
                vec![("extends", extends.clone()), ("frontend", frontend.clone())]
            }
            Self::LayoutStore { code } => vec![("code", code.clone())],
        }
    }

    /// Whether the host ends (exit 3, the starter falls back) instead of showing the problem in a window.
    #[must_use]
    pub fn fatal(&self) -> bool {
        !matches!(self, Self::LayoutMissing { .. } | Self::LayoutStore { .. })
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "host-error-{}", self.code())?;
        for (k, v) in self.params() {
            write!(f, " {k}={v}")?;
        }
        Ok(())
    }
}

impl std::error::Error for HostError {}
