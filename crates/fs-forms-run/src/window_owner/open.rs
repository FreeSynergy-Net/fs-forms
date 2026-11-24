//! What happens when a frontend is opened whose place is already open (NACHTRAG stage 6, O1): the rule as data ([`WhenOpen`]),
//! the answer of the owner ([`Opened`]) and the pure decision ([`decide`]). Nothing here opens or draws anything.
//!
//! Pattern: Policy – the rule (`WhenOpen`, `decide`) is data and a pure function; the owner only carries it out.

use super::{OpenFrontend, OpenedFrontend, WindowId};

/// Pattern: Value Object – what to do when the target place is already open with another record. The program names a default on the
/// layout node (`when-open`), the person overwrites it (setting `fs.open.when-open`); the default is `Ask`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WhenOpen {
    /// Ask the person (`notification.ask`); the answer comes back as one of the other values.
    #[default]
    Ask,
    /// Same place, same connection, only the record changes.
    Switch,
    /// A tab beside the open one. Phase 2 (frame `dock`): the value exists, but until then [`decide`] turns it into `Ask`, which
    /// offers two answers (switch here | new window). Keep that comment until the frame exists.
    Tab,
    /// A second window for the other record.
    Window,
}

/// Pattern: Value Object – what `open` did, so the host can show and report it (not only the id).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Opened {
    /// Opened new, with a hold of its own.
    New(WindowId),
    /// The very same id (same record) was open: brought forward.
    Raised(WindowId),
    /// The place stays and shows another record: records and hold stay (same module), only `page(scope)` is asked again. `id` is the
    /// id of the record now shown, `from` the record shown before.
    Switched {
        /// The id of the record now shown.
        id: WindowId,
        /// The record shown before.
        from: Option<String>,
    },
}

/// The outcome of [`decide`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Call `open` with this value; it is never `Ask` and never `Tab`.
    Open(WhenOpen),
    /// The host asks first (`notification.ask`: switch | window, later also tab, and "always so" writes the setting), then calls `open`.
    Ask {
        /// The open frontend that stands in the way.
        at: WindowId,
    },
}

/// The open frontend at `place` (any record) with the smallest id.
pub(super) fn at_place<'a>(
    open_now: &'a [OpenedFrontend],
    place: &WindowId,
) -> Option<&'a WindowId> {
    open_now
        .iter()
        .filter(|o| o.id.scope_in(place).is_some())
        .map(|o| &o.id)
        .min()
}

/// Pattern: Policy (Strategy as a pure function) – "is it already open?" is decided from data, testable without a window. The owner
/// only carries it out and never asks itself.
///
/// The target is the place (`program/elem`) of `req`. Not open: `Open(Window)` (new, nothing to ask). Exactly this record open:
/// `Open(Switch)` (the owner raises it). The place is open with another record: `Switch` and `Window` are taken as said, `Ask` and
/// `Tab` give `Ask { at }` with the open one at the smallest id. A parent that is not open gives `Open(Window)`; the owner then reports it.
#[must_use]
pub fn decide(open_now: &[OpenedFrontend], req: &OpenFrontend) -> Decision {
    let program = match &req.parent {
        Some((pid, _)) => match open_now.iter().find(|o| &o.id == pid) {
            Some(p) => p.key.extends.clone(),
            None => return Decision::Open(WhenOpen::Window),
        },
        None => req.key.extends.clone(),
    };
    let elem = match &req.parent {
        Some((_, e)) => e.as_str().to_owned(),
        None => req.key.frontend.clone(),
    };
    let place = WindowId::of(&program, &elem);
    let target = WindowId::of_record(&program, &elem, req.scope.as_deref());
    if open_now.iter().any(|o| o.id == target) {
        return Decision::Open(WhenOpen::Switch);
    }
    match (at_place(open_now, &place), req.when_open) {
        (None, _) => Decision::Open(WhenOpen::Window),
        (Some(_), w @ (WhenOpen::Switch | WhenOpen::Window)) => Decision::Open(w),
        // `Tab` waits for the frame `dock` (phase 2): until then it asks, with two answers.
        (Some(at), _) => Decision::Ask { at: at.clone() },
    }
}

/// The status line (yellow, a [`Coded`]) the host shows for what `open` did, or none. `Raised` is `host-notice-raised{id}`: Blinc cannot
/// bring a window forward by itself yet, so the host says so instead of staying silent.
///
/// Pattern: Policy – a pure function from the answer to the line, testable without a window.
#[must_use]
pub fn notice_of(opened: &Opened) -> Option<crate::view::Coded> {
    match opened {
        Opened::Raised(id) => {
            Some(crate::view::Coded::new("host-notice-raised").with("id", id.as_str()))
        }
        Opened::New(_) | Opened::Switched { .. } => None,
    }
}
