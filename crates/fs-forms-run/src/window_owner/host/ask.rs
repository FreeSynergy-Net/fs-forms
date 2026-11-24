//! The question of the host when a place is already open with another record (`decide` says `Ask`): the one existing facade of
//! `fs-notification` is used (`Notification::ask`, the answer arrives as `Event::Answered`, ADR 0106); the host has no dialog of its
//! own. The answer (switch here | new window) becomes `when_open` of the same request and the owner opens it.
//!
//! Pattern: Command over Observer – the question carries the request; the answer (an event) runs it with the chosen rule. The owner
//! itself never asks (G4): `open` with `Ask` still says `host-notice-open-unanswered`.

use std::sync::{Arc, Mutex, PoisonError};

use fs_notification::{
    Action, ActionId, Event, Message, NoteId, Notification, Notifications, Subscription,
};

use super::{Decision, HostWindows, OpenFrontend, Opened, WhenOpen, WindowId, WindowOwner, decide};
use crate::view::Coded;

const SWITCH: &str = "switch";
const WINDOW: &str = "window";

/// What [`OpenAsk::open_asking`] did at once.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Asked {
    /// Nothing stood in the way (or the rule was said): opened.
    Done(Opened),
    /// The question is shown under this id; the answer opens the frontend and calls `done`.
    Pending(NoteId),
}

/// Pattern: Command over Observer – see the module. Implemented for a shared [`HostWindows`].
pub trait OpenAsk {
    /// Opens `req`; when the place is open with another record and the rule is `Ask`, asks the person through `notes` first.
    /// `done` gets the result of the open after the answer (also an error), exactly once; a question that is dismissed, expired or
    /// closed without an answer ends with `done(Err(host-notice-open-unanswered))` and lets everything go; an answer that fits nothing opens nothing.
    ///
    /// # Errors
    /// What `open` says; `host-notice-open-ask-failed{cause}` when the question could not be shown.
    fn open_asking(
        &self,
        req: OpenFrontend,
        notes: &Notifications,
        now_ms: u64,
        done: Arc<dyn Fn(Result<Opened, Coded>) + Send + Sync>,
    ) -> Result<Asked, Coded>;
}

fn failed(cause: impl std::fmt::Debug) -> Coded {
    Coded::new("host-notice-open-ask-failed").with("cause", format!("{cause:?}"))
}

fn question(at: &WindowId) -> Result<Notification, Coded> {
    let msg = |key: &str| Message::new(key).map_err(failed);
    let action = |id: &str, key: &str| -> Result<Action, Coded> {
        Ok(Action::new(ActionId::new(id).map_err(failed)?, msg(key)?))
    };
    let title = msg("host-open-ask")?
        .with_text("at", at.as_str())
        .map_err(failed)?;
    Notification::ask(
        title,
        vec![
            action(SWITCH, "host-open-ask-switch")?,
            action(WINDOW, "host-open-ask-window")?,
        ],
    )
    .map_err(failed)
}

impl OpenAsk for Arc<HostWindows> {
    fn open_asking(
        &self,
        req: OpenFrontend,
        notes: &Notifications,
        now_ms: u64,
        done: Arc<dyn Fn(Result<Opened, Coded>) + Send + Sync>,
    ) -> Result<Asked, Coded> {
        let Decision::Ask { at } = decide(&self.open_now(), &req) else {
            return self.open(req).map(Asked::Done);
        };
        let id = notes.ask(question(&at)?, now_ms).map_err(failed)?;
        // the observer is told of every notification: it reacts to the answer of THIS question only, once, and lets itself go
        let keep: Arc<Mutex<Option<Subscription>>> = Arc::default();
        let owner = Arc::clone(self);
        let (mine, slot) = (id.clone(), Arc::clone(&keep));
        let sub = notes.subscribe(Arc::new(move |e: &Event| {
            let rule = match e {
                Event::Answered { id, action } if *id == mine => match action.as_str() {
                    SWITCH => Some(WhenOpen::Switch),
                    WINDOW => Some(WhenOpen::Window),
                    _ => return, // an answer that fits nothing: the `Closed` that follows ends the question
                },
                // dismissed, expired or replaced without an answer: nobody will answer any more
                Event::Closed { id, .. } if *id == mine => None,
                _ => return,
            };
            // once: the subscription lets itself go (it holds this closure and the owner), whichever event came first
            let taken = slot.lock().unwrap_or_else(PoisonError::into_inner).take();
            if taken.is_none() {
                return;
            }
            drop(taken);
            done(match rule {
                Some(rule) => owner.open(req.clone().with_when_open(rule)),
                None => Err(Coded::new("host-notice-open-unanswered").with("at", at.as_str())),
            });
        }));
        *keep.lock().unwrap_or_else(PoisonError::into_inner) = Some(sub);
        Ok(Asked::Pending(id))
    }
}
