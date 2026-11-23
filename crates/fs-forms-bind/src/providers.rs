//! Provider facts: synchronous snapshots the host gathers (asynchronously) before it calls `Bind::build`.
//!
//! Pattern: Port (`ProviderFacts`) + Snapshot values.

use crate::coded::Coded;
use fs_components_commands::Platform;

use crate::menu_std::StandardMenus;
use crate::types::{Limits, SearchKind};

/// One installed provider version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Provider {
    /// `major.minor.patch` (`SemVer`, three parts).
    pub version: (u32, u32, u32),
}

/// What the host learned about one capability for the asking person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Offer {
    /// Reached; one entry per installed version.
    Reached(Vec<Provider>),
    /// No package offers the capability.
    NotInstalled,
    /// Installed but not reachable now.
    Unreachable {
        /// The cause (a `bind-error-*` code).
        cause: Coded,
    },
}

/// Snapshot port: the host fills it, bind reads it.
pub trait ProviderFacts {
    /// The offer for a capability.
    fn offer(&self, capability: &str) -> Offer;
}

/// Health of the settings store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsHealth {
    /// Read from the store.
    Ok,
    /// Not reachable: defaults apply, visibly.
    Defaults {
        /// Code key of the setting error.
        cause: String,
    },
}

/// The settings bind uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsFacts {
    /// Style of the person (wins over the layout's style).
    pub style: Option<String>,
    /// Record limits.
    pub limits: Limits,
    /// Search kind of the person.
    pub search: Option<SearchKind>,
    /// Store health.
    pub health: SettingsHealth,
}

/// What this host build can do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostCaps {
    /// Writing available; else write commands are locked `write-later`.
    pub write: bool,
    /// Workflow runtime available.
    pub workflow: bool,
}

/// Everything about the providers in one value.
pub struct Providers<'a> {
    /// Capability offers.
    pub records: &'a dyn ProviderFacts,
    /// Settings.
    pub settings: &'a SettingsFacts,
    /// Host abilities.
    pub host: HostCaps,
    /// The platform `Primary` resolves for (shortcut conflicts).
    pub platform: Platform,
    /// The standard menus of the shell, read by the host (never copied into the plan).
    pub standard: &'a StandardMenus,
}
