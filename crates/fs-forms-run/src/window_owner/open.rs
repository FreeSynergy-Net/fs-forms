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
    /// Opened new. Its records (and the hold of its module) are the shared ones of its record: a second view of the same record
    /// attaches to them, only the first makes them ([`crate::RecordHub`]).
    New(WindowId),
    /// The very same id (same record) was open: brought forward.
    Raised(WindowId),
    /// The place stays and shows another record: the grip of the hub moves from the records of the old
    /// record to those of the new one (the old ones fall when no other view shows them), and `page(scope)` is asked again. `id` is the
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

/// The place a request lands in: `(program, elem)`. A root is `(program, frontend)`, a child the node of its parent; `None` when the
/// parent is not open.
pub(super) fn place_of(
    open_now: &[OpenedFrontend],
    req: &OpenFrontend,
) -> Option<(String, String)> {
    match &req.parent {
        Some((pid, e)) => {
            let p = open_now.iter().find(|o| &o.id == pid)?;
            Some((p.key.extends.clone(), e.as_str().to_owned()))
        }
        None => Some((req.key.extends.clone(), req.key.frontend.clone())),
    }
}

/// A root is found by its CURRENT `key` (and the record), never by the text of its id: `use_frontend` changes the key, the id stays
/// the name of the place. A child is found by the id of its node.
fn at<'a>(
    open_now: &'a [OpenedFrontend],
    req: &OpenFrontend,
    place: &(String, String),
    same_record: bool,
) -> Option<&'a WindowId> {
    let place_id = WindowId::of(&place.0, &place.1);
    let target = WindowId::of_record(&place.0, &place.1, req.scope.as_deref());
    open_now
        .iter()
        .filter(|o| match (&req.parent, &o.parent) {
            (None, None) => o.key == req.key && (!same_record || o.scope == req.scope),
            (Some(_), _) => {
                if same_record {
                    o.id == target
                } else {
                    o.id.scope_in(&place_id).is_some()
                }
            }
            (None, Some(_)) => false,
        })
        .map(|o| &o.id)
        .min()
}

/// The open frontend that shows exactly this request (same place, same record).
pub(super) fn exact<'a>(
    open_now: &'a [OpenedFrontend],
    req: &OpenFrontend,
    place: &(String, String),
) -> Option<&'a WindowId> {
    at(open_now, req, place, true)
}

/// The open frontend at the place of the request (any record) with the smallest id.
pub(super) fn at_place<'a>(
    open_now: &'a [OpenedFrontend],
    req: &OpenFrontend,
    place: &(String, String),
) -> Option<&'a WindowId> {
    at(open_now, req, place, false)
}

/// Pattern: Policy (Strategy as a pure function) – "is it already open?" is decided from data, testable without a window. The owner
/// only carries it out and never asks itself.
///
/// The target is the place (`program/elem`) of `req`; a root is found by its current key and record, not by the text of an id. Not open: `Open(Window)` (new, nothing to ask). Exactly this record open:
/// `Open(Switch)` (the owner raises it). The place is open with another record: `Switch` and `Window` are taken as said, `Ask` and
/// `Tab` give `Ask { at }` with the open one at the smallest id. A parent that is not open gives `Open(Window)`; the owner then reports it.
#[must_use]
pub fn decide(open_now: &[OpenedFrontend], req: &OpenFrontend) -> Decision {
    let Some(place) = place_of(open_now, req) else {
        return Decision::Open(WhenOpen::Window);
    };
    if exact(open_now, req, &place).is_some() {
        return Decision::Open(WhenOpen::Switch);
    }
    match (at_place(open_now, req, &place), req.when_open) {
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
