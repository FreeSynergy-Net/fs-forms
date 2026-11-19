//! `fs-forms-run`: the forms host, read only, first frame (H3 of the host plan, `PLAN-forms-host.md` section 1).
//!
//! The process asks the layouts of `fs-forms serve` for the resolved description, lets the catalog say which block is built,
//! puts the window together from `fs-components` blocks only and shows every problem where it happened (a locked part says why,
//! with its code; nothing is left out without a word). It has no widget of its own, no table "type to block" and no program name.
//!
//! Stand of what it reads: the plan types of the probe H1 `unreviewed/probe-forms-h1-bind` (`fs-forms-bind`, commit `7dde60c`
//! plus unpushed work of the builder): [`view::View`] is the narrow projection of its `Plan`; the crate joins when it is pinned.
//!
//! Pattern: Facade: this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod args;
mod blocks;
mod catalog;
mod compose;
mod error;
mod file;
mod from_layout;
mod link;
mod run;
mod view;
mod window;
mod words;

pub use args::{Command, Config, HELP, parse};
pub use blocks::Blocks;
pub use catalog::{Catalog, TestCatalog};
pub use compose::{Composition, HostState, Node, ScreenTree, compose};
pub use error::{EXIT_CONTRACT, HostError};
pub use file::FileLink;
pub use from_layout::view_of;
pub use link::{FixtureLink, LayoutLink, SocketLink};
pub use run::run;
pub use view::{Coded, MenuItem, MenuState, PageView, PlaceView, View};
pub use window::app;
pub use words::{TEXTS, session_locale, words};
