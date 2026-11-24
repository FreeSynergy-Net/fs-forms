//! The owner of the open frontends (stage 6 of `PLAN-basis-vertraege.md`, contract S6-1): a frontend of one program placed inside
//! another one, as a panel (a place in the tree of the parent) or as a window of its own. The parent description names only a
//! [`FrontendKey`]; the owner opens it, holds what decides its lifetime (composition, records, the held connection of its module) and
//! lets all of it fall when the frontend is closed. It draws nothing itself.
//!
//! This file is the contract only (trait and value objects, appended to the crate; nothing existing changes). The fake with the
//! conformance tests is [`fake`]; the real owner (`HostWindows`, S6-7) comes after the layout node `PlaceNode::Frontend` (A5 tree).
//!
//! Pattern: Facade: this file holds the contract; the double lives in the submodule.

pub mod fake;

use fs_forms_layout::{ElemId, FrontendKey};

use crate::view::Coded;

/// Pattern: Strategy – how a child frontend appears (panel or window) is decided at the parent node; the child is the same either way.
///
/// Abweichung: it is a closed pair for now (`non_exhaustive`: floating, N6 stage 3, may follow); this is the mirror of the
/// `Placement` the layout node will carry (`fs-forms-layout` `doc.rs`, S6-3), joined to it when that node exists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Placement {
    /// A place in the tree of the parent window ("tab" is a panel inside a frame `tabs`, no mode of its own).
    #[default]
    Panel,
    /// A window of the operating system.
    Window,
}

/// Pattern: Value Object – the stable name of one open frontend, `<program>/<elem>` (ADR 0069: stable, never a counter).
///
/// The program part is the FULL `AppStream` id (`net.freesynergy.Tasks/settings-embed`); the display name is separate. The root frontend of a program is `<id>/<frontend>`; an embedded one is `<id of the parent program>/<elem of the node>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub String);

impl WindowId {
    /// The id of the node `elem` of a frontend of the program `program`.
    #[must_use]
    pub fn of(program: &str, elem: &str) -> Self {
        Self(format!("{program}/{elem}"))
    }

    /// The text of the id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Pattern: Value Object – the request to open one frontend (what the parent node says, nothing else).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenFrontend {
    /// The frontend to open: `(AppStream id of the program, frontend name)`, never a module.
    pub key: FrontendKey,
    /// How it appears.
    pub mode: Placement,
    /// The window and the node it is placed in; `None` = a root frontend of its program.
    pub parent: Option<(WindowId, ElemId)>,
    /// Whether the person may switch panel and window later (`place`); the flag `window` of the node. Absent = locked.
    pub switchable: bool,
    /// The record chosen in a neighbour node (an AS2 id), given as `scope` to the child; only a reference, no data goes with it.
    pub scope: Option<String>,
}

/// Pattern: Value Object – one open frontend as data (for the scaffold form "open views"; it draws nothing).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedFrontend {
    /// Its stable id.
    pub id: WindowId,
    /// What was opened.
    pub key: FrontendKey,
    /// How it appears.
    pub mode: Placement,
    /// The window it sits in, `None` for a root.
    pub parent: Option<WindowId>,
}

/// Pattern: Strategy – the owner of the open frontends: `open` is told the [`Placement`] by the parent node and the owner acts by it. `place` is the one command that changes it later (S6-9).
///
/// Abweichung: it is also a registry (one entry per [`WindowId`], like a Multiton), and it draws nothing: a panel goes into the
/// composition as a child node, a window goes through `fs-components-app`. It holds only what decides the lifetime: the
/// composition, the records and the held connection of the module. Dropping them is signing off.
pub trait WindowOwner: Send + Sync {
    /// Opens `req.key` in `req.mode`. When the same [`WindowId`] is already open, that id comes back and no second frontend is made
    /// (the circle people, calendar, people ends here).
    ///
    /// # Errors
    /// `host-notice-frontend-missing{extends,frontend}` (no such description), `host-notice-frontend-refused` (the module said no),
    /// `host-notice-window-unknown` (the parent is not open). The part stays visible and locked with the code, never empty.
    fn open(&self, req: OpenFrontend) -> Result<WindowId, Coded>;

    /// Closes the frontend and every child inside it; the connections of their modules fall (= sign off).
    ///
    /// # Errors
    /// `host-notice-window-unknown` when `id` is not open.
    fn close(&self, id: &WindowId) -> Result<(), Coded>;

    /// The person switches the open frontend `id` between panel and window. It is the same frontend afterwards (same composition,
    /// records and held connection; the state of its form stays); only the place changes. Placing it where it already is is no error.
    ///
    /// # Errors
    /// `host-notice-window-unknown` when `id` is not open; `host-notice-placement-locked{id}` when the program locked the switch
    /// (no flag `window` on the node), the frontend stays as it is.
    fn place(&self, id: &WindowId, placement: Placement) -> Result<(), Coded>;

    /// What is open, as data.
    fn open_now(&self) -> Vec<OpenedFrontend>;
}
