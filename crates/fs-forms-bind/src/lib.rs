//! `fs-forms-bind`: the pure build plan of the forms host (probe H1 "host-bind").
//!
//! `Bind::build(&Resolved, &dyn Catalog, &Providers) -> Plan` reads only snapshots, does no I/O, needs no clock and never
//! panics. Every problem becomes a [`PartLock`] on the smallest part concerned (code `bind-error-locked{part, cause}`) or a
//! window [`Notice`]; nothing is dropped silently. The host (H3) only interprets the plan.
//!
//! The key chain (Ime, field, composite, workbench, window) is not here: it is an additive extension of
//! `fs-components-commands`. This crate has no dependencies (H-K17: no toml/JSON at run time).
//!
//! Interface status: draft (ADR 0045). Written with Claude (AI) as a co-author.
//!
//! Pattern: Facade: this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod build;
mod build_menu;
mod build_place;
mod catalog;
mod coded;
mod codes;
mod commands;
mod keys;
mod menu;
mod model;
mod plan;
mod plan_query;
mod providers;
mod runtime;
mod types;

#[cfg(feature = "testing")]
mod fixtures;
#[cfg(feature = "conformance")]
pub mod suite;
#[cfg(feature = "testing")]
pub mod testing;

pub use build::Bind;
pub use catalog::{BlockSpec, Catalog, SlotSpec, check_bindings};
pub use coded::{Coded, Family};
pub use codes::{BindCode, Cause, Notice, Part, PartLock, Severity};
pub use commands::{Shown, command_id, command_plan, command_state};
pub use keys::{Focus, KeyPlan, Keymap, Outcome, Shortcut};
pub use menu::{Arranged, MenuEntry, MenuNode, MenuPlan, standard_entries};
pub use model::*;
pub use plan::*;
pub use providers::{
    HostCaps, Offer, Provider, ProviderFacts, Providers, SettingsFacts, SettingsHealth,
};
pub use runtime::{Admitted, FieldShown, admit_page, field_shown};
pub use types::*;
