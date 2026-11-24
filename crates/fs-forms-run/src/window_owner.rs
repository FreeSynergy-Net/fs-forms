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
mod open;

pub use open::{Decision, Opened, WhenOpen, decide, notice_of};

use fs_forms_layout::{ElemId, FrontendKey};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};

use crate::view::Coded;

/// Pattern: Strategy – how a child frontend appears (in the frame of the parent, or in a window of its own) is decided at the parent node; the child is the same either way.
///
/// Abweichung: it is a closed pair for now (`non_exhaustive`: floating, N6 stage 3, may follow); this is the mirror of the
/// `Placement` the layout node will carry (`fs-forms-layout` `doc.rs`, S6-3), joined to it when that node exists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Placement {
    /// Embedded in a frame of the parent window ("tab" is a panel inside a frame `tabs`, no mode of its own). A panel is any node of the
    /// tree, any arrangeable unit that can be resolved; `InFrame` is the one way of appearing inside a frame, which frame (`flow`,
    /// `split`, `tabs`, `dock`) the tree says. Was `Panel` until stage 6 P1 (N.1: when everything is a panel, "Panel" names nothing).
    #[default]
    InFrame,
    /// A window of its own (of the operating system).
    Window,
}

/// RFC 3986 "unreserved" stays as it is, everything else (also `/`, `#`, `%` and every non-ASCII byte) is encoded.
const SCOPE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Pattern: Value Object – the stable name of one open frontend, `<program>/<elem>` (ADR 0069: stable, never a counter).
/// With a record (`scope`) the id is `<program>/<elem>/<scope>`, the scope percent-encoded; without one it is as before.
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

    /// The id of the node `elem` of `program` showing the record `scope`: `<program>/<elem>/<scope>`, the scope percent-encoded
    /// (RFC 3986, crate `percent-encoding`). Without a scope it is exactly [`WindowId::of`] (old ids never change). Two different
    /// scopes never give the same id, and [`WindowId::scope_in`] reads the scope back.
    #[must_use]
    pub fn of_record(program: &str, elem: &str, scope: Option<&str>) -> Self {
        match scope {
            None => Self::of(program, elem),
            Some(s) => Self(format!(
                "{program}/{elem}/{}",
                utf8_percent_encode(s, SCOPE)
            )),
        }
    }

    /// The reverse of [`WindowId::of_record`]: `Some(scope)` when this id names the place `place` (made by `of`), where `scope` is
    /// `None` for the place itself and `Some(record)` for a record shown there; `None` when the id belongs to another place.
    #[must_use]
    pub fn scope_in(&self, place: &WindowId) -> Option<Option<String>> {
        let rest = self.0.strip_prefix(place.as_str())?;
        if rest.is_empty() {
            return Some(None);
        }
        let enc = rest.strip_prefix('/')?;
        let dec = percent_decode_str(enc).decode_utf8().ok()?;
        Some(Some(dec.into_owned()))
    }

    /// The text of the id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Pattern: Value Object – the request to open one frontend (what the parent node says, nothing else).
///
/// `non_exhaustive`: build it with [`OpenFrontend::new`] and the `with_*` setters, so later data (a floating `rect`) only adds.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
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
    /// What to do when the place is already open with another record (rule of the program, overwritten by the person); see [`decide`].
    pub when_open: WhenOpen,
}

impl OpenFrontend {
    /// A root frontend `key` in `mode`: locked, no record, `when_open` = ask.
    #[must_use]
    pub fn new(key: FrontendKey, mode: Placement) -> Self {
        Self {
            key,
            mode,
            parent: None,
            switchable: false,
            scope: None,
            when_open: WhenOpen::default(),
        }
    }

    /// Placed in the node `elem` of the open window `parent`.
    #[must_use]
    pub fn with_parent(mut self, parent: WindowId, elem: ElemId) -> Self {
        self.parent = Some((parent, elem));
        self
    }

    /// The person may switch panel and window later.
    #[must_use]
    pub fn with_switchable(mut self, switchable: bool) -> Self {
        self.switchable = switchable;
        self
    }

    /// The record (an AS2 id) the child shows.
    #[must_use]
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// What to do when the place is already open with another record.
    #[must_use]
    pub fn with_when_open(mut self, when_open: WhenOpen) -> Self {
        self.when_open = when_open;
        self
    }
}

/// Pattern: Value Object – one open frontend as data (for the scaffold form "open views"; it draws nothing).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedFrontend {
    /// The record it shows (the `scope` of its id), `None` = none.
    pub scope: Option<String>,
    /// Its stable id.
    pub id: WindowId,
    /// What is open NOW: the frontend of the surface. [`WindowOwner::use_frontend`] changes it; the `id` stays the name of the place.
    /// Roots are found by this key and the record (`scope`), never by the text of the id ([`decide`]).
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
/// composition, a grip on the read cache of the records and its OWN held connection to the module. Dropping them is signing off.
pub trait WindowOwner: Send + Sync {
    /// Opens `req.key` in `req.mode` and says what it did ([`Opened`]). The same [`WindowId`] already open: [`Opened::Raised`], no
    /// second frontend (the circle people, calendar, people ends here). The same place open with another record: `req.when_open`
    /// decides ([`decide`]): `Switch` keeps the place, the module and the hold (the own connection of the view) and lets the grip show the new record ([`Opened::Switched`]), `Window` opens a
    /// second one. The owner never asks: `Ask` (and `Tab`, phase 2) is answered by the host before it calls.
    ///
    /// # Errors
    /// `host-notice-frontend-missing{extends,frontend}` (no such description), `host-notice-frontend-refused` (the module said no),
    /// `host-notice-window-unknown` (the parent is not open), `host-notice-open-unanswered{at}` (the place is open with another
    /// record and `when_open` is still `Ask`). The part stays visible and locked with the code, never empty.
    fn open(&self, req: OpenFrontend) -> Result<Opened, Coded>;

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

    /// The person changes the frontend of the open surface `id` (same program): the surface stays, only what draws it is exchanged.
    /// The [`WindowId`] (the name of the place, given on `open`), the `scope`, the records and the held connection stay the same (no
    /// new hold, none dropped); the composition is built from the new description and the key in [`OpenedFrontend`] is the new one.
    /// Anything that is not the same is refused and nothing changes.
    ///
    /// Pattern: Bridge – the surface is the abstraction (id, record, records, hold), the frontend its implementation; only the
    /// implementation is exchanged, while the program runs.
    ///
    /// Where the state of the person lives today (checked, stage 6 P2): the selection of a board sits in the mounted component of
    /// the page (`draw.rs` `MountSlot`, `Ctx.mounts`; `fs-components-kanban` `state.rs` `selected`), kept over a rebuild by the key
    /// (block, look) and dropped with `slots.retain` when the block is gone; it is not a datum of the surface yet. A rule for
    /// open input (ask first) does not exist in the host. Both come with the real owner (`HostWindows`, O4a, C1), not here.
    ///
    /// # Errors
    /// `host-notice-window-unknown{id}`; `host-notice-frontend-unknown{id,available}` (no frontend of that name in any program);
    /// `host-notice-frontend-other-program{id,frontend}` (the name belongs to another program); `host-notice-frontend-refused`
    /// (the module says no); `host-notice-frontend-open-elsewhere{id}` (that frontend already shows this record in another place).
    fn use_frontend(&self, id: &WindowId, frontend: &str) -> Result<(), Coded>;

    /// What is open, as data.
    fn open_now(&self) -> Vec<OpenedFrontend>;
}
