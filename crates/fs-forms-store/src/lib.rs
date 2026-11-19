//! `fs-forms-store`: the domain `forms` of fs-forms (probe F1b, first cut): the 20 typed tables `forms_*` with their closed word
//! lists, the four migration steps, and the mapping between a checked layout and its rows (`to_rows`, `from_rows`).
//!
//! What is NOT here yet: the database adapter itself (`DbLayoutStore`), because the edge to fs-objects v5 (`Store`, `inspect`,
//! `Error::Newer`, `spec`, `migrate`, `registry`) is not pinned. `spec.rs` holds stand-ins with the same shapes; the port
//! `LayoutStore` is already served by `MemoryLayoutStore` of `fs-forms-layout`, which is the mock for the first tests.
//!
//! Pattern: Facade: this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod from_rows;
mod from_rows_parts;
mod rows;
pub mod spec;
/// The table specs are self-describing data: name, columns and constraints are the documentation.
#[allow(missing_docs)]
pub mod tables;
#[allow(missing_docs)]
pub mod tables_body;
#[allow(missing_docs)]
pub mod tables_overlay;
mod to_rows;
mod to_rows_parts;

pub use from_rows::from_rows;
pub use rows::{Cell, Row, Rows, validate};
pub use tables_overlay::{ALL, STEPS};
pub use to_rows::{LayoutMeta, to_rows};
