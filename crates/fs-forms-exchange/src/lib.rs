//! `fs-forms-exchange`: the only place that reads or writes TOML (and, with feature `json`, JSON) of a forms description.
//! A file is import and export, Git and review, never a runtime source; the host never links this crate.
//!
//! First cut: layouts in format 1 and 2 (filters of views and overlay files are not part of it yet). Reading collects ALL
//! problems with code and line; writing is always format 2 and canonical (TOML: fixed order, LF; JSON: RFC 8785).
//!
//! Pattern: Adapter (file to model and back).

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod api;
mod emit;
mod fields;
mod read;
mod tree;
mod write;

pub use api::{Format, read_layout, write_layout};
pub use fs_forms_layout::{Coded, Layout, LayoutCode, LayoutDoc};
