//! The records a block draws: a port the host asks, so no block holds data of its own. A connected module gives
//! [`crate::FormRecords`] (rows by channel); without one the implementation is [`NoRecords`] (nothing: the blocks stay empty and say so).
//! Sample data for pictures and development (`--sample-data`) comes from the host (`Sources::connect_sample`), never from this crate.
//!
//! The port also says which gestures the records take: a channel the module binds to a command ([`Records::command_of`]) is allowed
//! by [`Records::can_channel`] and carried out by [`Records::invoke`]; what is not said is locked.
//!
//! Pattern: Port (`Records`) with a null object and a fixture.
//!
//! History: `history`, `step_back` and `changed` default to "none" (locked), like `can`; `changed` is a counter ([`crate::ChangeCounter`]) a window compares ([`crate::ChangeWatch`]) to read again.

use std::sync::Arc;

use fs_components_kanban::{Answer, Reason};
use fs_objects::UndoState;

use crate::clock::{Clock, SystemClock};
use crate::form_data::FormData;
use crate::view::Coded;

/// One entry of a side list (a project, a label).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideItem {
    /// The key given back on selection.
    pub key: String,
    /// What is shown.
    pub label: String,
}

/// A refusal by code: the host says no code-only reason (the sentence comes from the words of the reader, `Reason::said` falls back to the code).
#[must_use]
pub fn refused(code: &str) -> Answer {
    Answer::Refused(Reason::new(code, ""))
}

/// The answer of records that cannot do an operation.
fn unsupported() -> Answer {
    refused("unsupported")
}

/// What the blocks of the host draw their records from, and which changes they take.
pub trait Records: Send + Sync {
    /// The entries of a side list.
    fn side(&self) -> Vec<SideItem>;
    /// The words the connected module brings (its `catalog/texts/`); `None` = it brings none. `ctx.say` asks them after the words of
    /// the host, so a head such as `person.emails` is a sentence, not the field name.
    fn module_words(&self) -> Option<Arc<dyn crate::module_words::ModuleWords>> {
        None
    }
    /// The forms the connected module offers, with the rows by channel (the blocks tree and table draw from these). Read again
    /// after an accepted call, so a block takes the answer once per drawing, never keeps it.
    fn forms(&self) -> Arc<Vec<FormData>> {
        Arc::default()
    }
    /// The key of the command the channel `channel` of the form `form` is bound to by the module (`move` is `card.move`); `None`
    /// = not bound. The description's `[[action]]` names a command by this key.
    fn command_of(&self, _form: &str, _channel: &str) -> Option<String> {
        None
    }
    /// The channel of the form `form` the module binds the command key `command` to (`card.move` is `move`): the way BACK of
    /// [`Records::command_of`], the one a gesture with arguments takes (the arguments are named as the channels). `None` = no channel of
    /// the form is bound to it. No fixed channel name anywhere in the host.
    fn channel_of(&self, _form: &str, _command: &str) -> Option<String> {
        None
    }
    /// Whether the channel `channel` of the form `form` may be used now on `record` (`None` = without a record, or on any: see
    /// [`crate::write::allowed`]). Default `false`: what is not said is locked.
    fn can_channel(&self, _form: &str, _channel: &str, _record: Option<&str>) -> bool {
        false
    }
    /// A gesture on the channel `channel` of the form `form`, `values` by channel name (what the gesture carries: `column`, `order`);
    /// the arguments are made by the rule of [`crate::write::args_of`], the module is asked, and after an accepted call the form is
    /// read again.
    fn invoke(
        &self,
        _form: &str,
        _channel: &str,
        _record: Option<&str>,
        _values: &[(String, String)],
    ) -> Answer {
        unsupported()
    }
    /// May the command `command` (a key of the contract) be called now on `record`, with no channel and no arguments (a gesture slot
    /// without `args`: `activate`, `menu`)? The module's command set and the `may` of the record decide. Default `false`.
    fn can_command(&self, _form: &str, _command: &str, _record: Option<&str>) -> bool {
        false
    }
    /// The call of the command `command` with the record only (no arguments); after an accepted call the form is read again.
    fn invoke_command(&self, _form: &str, _command: &str, _record: Option<&str>) -> Answer {
        unsupported()
    }
    /// How many undo and redo steps the person has on these records. Default: none (what is not said is locked, like
    /// [`Records::can`]); the window then keeps its undo and redo controls locked.
    fn history(&self) -> UndoState {
        UndoState::default()
    }
    /// One step back (`redo = false`) or forward again (`redo = true`) in that history (only called when [`Records::history`]
    /// shows a step). A conflict (someone else changed the record meanwhile) is a refusal with its code, never an overwrite.
    fn step_back(&self, _redo: bool) -> Answer {
        unsupported()
    }
    /// The change counter: grows whenever the records changed behind the window's back. A window that sees it grow reads the
    /// records again ([`crate::ChangeWatch`]). Default `0` = never changes by itself.
    fn changed(&self) -> u64 {
        0
    }
    /// Read the state of the module again. Called when the `watch` of the module says the record changed (by WHOMEVER, also by the own
    /// window: a view learns of a change only through its own `watch`, never from its own write). Default: nothing to read (records that keep no state).
    ///
    /// Pattern: Observer – a change anywhere means "read again, count the generation".
    ///
    /// # Errors
    /// The code of the failed read (`host-notice-data-reload`); [`Records::stale`] then says the board shows the old state.
    fn reread(&self) -> Result<(), Coded> {
        Ok(())
    }
    /// The clock of this data: what "now" is for a view of them. Default: the system clock (a connected module). Sample data say the
    /// moment they were made (`FixedClock`); the window reads the time only through this port ([`crate::Clock`]).
    fn clock(&self) -> Arc<dyn Clock> {
        Arc::new(SystemClock)
    }
    /// Whether these records are made-up sample data: the window then says so (it never shows invented records as real ones).
    fn is_sample(&self) -> bool {
        false
    }
    /// The code (`host-notice-data-reload`) when the module accepted a call but reading it again failed: the module HAS written and the
    /// board still shows the old state. `None` = nothing stale.
    fn stale(&self) -> Option<String> {
        None
    }
    /// The sources of the description the host could not read, as `(number, code)` (numbers count from 1). Default: none. The window
    /// goes on; what depends on such a source (the heads of lanes) says so, never silently.
    fn source_problems(&self) -> Vec<(u32, Coded)> {
        Vec::new()
    }
    /// The sentence that says why the data source the description names is not there (the window shows it as a problem in its
    /// status line); `None` when the records are what they claim to be.
    fn unavailable(&self) -> Option<String> {
        None
    }
}

/// No provider is connected: nothing to draw.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoRecords;

impl Records for NoRecords {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
}

/// The data source of the description could not be reached: nothing to draw, and the window says why (never silently empty).
#[derive(Clone, Debug)]
pub struct Unavailable(pub String);

impl Records for Unavailable {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn unavailable(&self) -> Option<String> {
        Some(self.0.clone())
    }
}
