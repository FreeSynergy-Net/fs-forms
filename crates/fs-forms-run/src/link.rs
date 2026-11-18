//! The connection to the layouts of `fs-forms serve`: only the interface the host needs (resolve), no hand-written logic.
//!
//! Pattern: Port (`LayoutLink`) with an adapter over the socket and a fixture. `SocketLink` cannot fetch a layout yet: the
//! contract `layouts.capnp` (F1b) is not offered by `fs-forms serve` and `fs-forms-client` has no `Layouts` client. It says so
//! with a code (`host-error-layouts-contract-missing`, exit 3), it does not invent a layout. The real client replaces its body.

use std::path::{Path, PathBuf};

use crate::error::HostError;
use crate::view::View;

/// Where the resolved description comes from.
pub trait LayoutLink {
    /// `Layouts.resolve(extends, frontend)`: the plan of the effective description.
    ///
    /// # Errors
    /// `layout-missing` (no layout for this pair), `layout-store` (emergency mode) or a fatal connection error.
    fn resolve(&self, extends: &str, frontend: &str) -> Result<View, HostError>;
}

/// The socket of `fs-forms serve`.
#[derive(Clone, Debug)]
pub struct SocketLink {
    path: PathBuf,
}

impl SocketLink {
    /// Connect to the socket (the `fs-forms-client` default when none is named).
    ///
    /// # Errors
    /// `no-socket` (no default), `layouts-unreachable` (nothing at the path).
    pub fn open(named: Option<&Path>) -> Result<Self, HostError> {
        let path = named
            .map(Path::to_path_buf)
            .or_else(fs_forms_client::default_socket)
            .ok_or(HostError::NoSocket)?;
        if path.exists() {
            Ok(Self { path })
        } else {
            Err(HostError::LayoutsUnreachable {
                path: path.display().to_string(),
            })
        }
    }
}

impl LayoutLink for SocketLink {
    fn resolve(&self, _extends: &str, _frontend: &str) -> Result<View, HostError> {
        Err(HostError::LayoutsContractMissing {
            path: self.path.display().to_string(),
        })
    }
}

/// A link that answers from memory (tests, pictures).
#[derive(Clone, Debug)]
pub struct FixtureLink(pub Result<View, HostError>);

impl LayoutLink for FixtureLink {
    fn resolve(&self, _extends: &str, _frontend: &str) -> Result<View, HostError> {
        self.0.clone()
    }
}
