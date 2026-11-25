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
mod changes;
mod compose;
mod draw;
mod error;
mod file;
mod form_check;
mod form_data;
mod from_layout;
mod from_places;
mod frontend_cell;
pub mod history;
mod link;
mod menus;
mod page;
mod panel;
mod parts;
mod record_hub;
mod records;
mod repaint;
mod rules_adapter;
mod run;
mod sample;
mod sample_calls;
mod sample_groups;
mod screens;
mod sources;
mod status;
mod steps;
mod style;
mod unfinished;
mod view;
mod window;
mod window_owner;
mod words;
pub mod write;
mod write_gate;

pub use args::{Command, Config, HELP, parse};
pub use blocks::Blocks;
pub use catalog::{Catalog, TestCatalog};
pub use changes::{ChangeCounter, ChangeWatch};
pub use compose::{Composition, HostState, Node, ScreenTree, compose};
pub use error::{EXIT_CONTRACT, HostError};
pub use file::{FileLink, Loaded};
pub use form_check::{Finding, action_report, problems as form_problems, report as form_report};
pub use form_data::{FormData, FormRecords, FormRow};
pub use from_layout::view_of;
pub use fs_objects::UndoState;
pub use link::{FixtureLink, LayoutLink, SocketLink};
pub use record_hub::fake::{FakeModule, HoldCounts};
pub use record_hub::{ChangeSink, Link, Module, RecordGrip, RecordHub, RecordKey};
pub use records::{NoRecords, Records, SideItem, Unavailable, refused};
pub use repaint::{BlincRepaint, Repaint};
pub use rules_adapter::ContractRules;
pub use run::{run, run_code, run_code_with, run_with};
pub use sample::SampleBoard;
pub use sample_calls::{sample_add_card, sample_add_column, sample_move_card};
pub use sample_groups::groups as sample_groups;
pub use sources::{NoSources, Opened, Sources, name_of, open};
pub use style::{DEFAULT_STYLE, StyleCell, StyleSource, resolve, unknown};
pub use view::{ActionView, Coded, MenuItem, MenuState, PageView, PlaceView, View};
pub use window::{app, app_styled, app_with};
pub use window_owner::fake::{CountingRecords, FakeHold, FakeWindows};
pub use window_owner::host::{
    Asked, BlincSurfaces, Descriptions, HostWindows, Modules, OpenAsk, Surfaces,
};
pub use window_owner::{
    Decision, OpenFrontend, Opened as WindowOpened, OpenedFrontend, Placement, WhenOpen, WindowId,
    WindowOwner, decide, notice_of,
};
pub use words::{TEXTS, session_locale, words};
pub use write::{Invocation, Writer, allowed, args_of, command_key};
