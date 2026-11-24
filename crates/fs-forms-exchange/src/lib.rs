//! `fs-forms-exchange`: the only place that reads or writes TOML (and, with feature `json`, JSON) of a forms description.
//! A file is import and export, Git and review. The host `fs-forms-run` reads a description file through this crate when it is given a path (`--description`, K5 H4); ADR 0096 point 6 (the description lives in `forms.db`) is open for that case, see the CHANGELOG of fs-forms.
//!
//! First cut: layouts in format 3 (a tree of places; an older format is refused with `format-old`, it is not converted; filters of views and overlay files are not part of it yet). Reading collects ALL
//! problems with code and line; writing is always format 3 and canonical (TOML: fixed order, LF; JSON: RFC 8785).
//!
//! Pattern: Adapter (file to model and back).

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod api;
mod emit;
mod fields;
mod read;
mod read_place;
mod scaffold;
mod tree;
mod write;
mod write_place;

pub use api::{Format, LAYOUT_BYTES_MAX, read_layout, write_layout};
pub use fs_forms_layout::{Coded, Layout, LayoutCode, LayoutDoc};
pub use scaffold::scaffold_layout;
