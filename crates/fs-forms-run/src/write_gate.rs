//! The write side of the board for the components. A GESTURE of a view (`fire`: `drop`, `menu`, `quick-add`, `activate`, named as the
//! command slots of the contract) is allowed only when the DESCRIPTION has an `[[action]]` at this place for it, and goes the way
//! gesture, action, command ([`fire`]): no channel name is known to the host, and card moves and new cards have NO channel way any
//! more (the board says `drop` and `quick-add` by `fire`; G8). Only the three dialogs on a column (`can`, `send`: column create,
//! rename, limit) still go by channel until G8b; they too need an action AT THIS PLACE and records that bind a command. Every other
//! control stays locked and says so.
//!
//! The host knows no layout of a component: what a gesture carries is the `args` of the slot in the catalog.
//!
//! Pattern: Adapter (`FormWrite` over `Records`), gated by the actions of the description; one gate per PLACE.

use std::sync::Arc;

use fs_components_core::form_write::{
    FormWrite, Gesture, WriteArg, WriteDone, WriteRefused, WriteValue,
};

use crate::after_write::AfterWrite;
use crate::records::Records;
use crate::view::ActionView;

mod fire;

/// The gate of one place of a form over the records.
pub(crate) struct WriteGate {
    pub records: Arc<dyn Records>,
    pub actions: Arc<Vec<ActionView>>,
    pub form: String,
    /// The block of the place (its command slots tell which arguments a gesture carries).
    pub block: String,
    /// The element id of the place: an `[[action]]` binds a gesture of THIS place with `at`.
    pub place: String,
    /// Fired after an accepted write (the menu "Edit" follows the new depth of the history).
    pub after: AfterWrite,
}

impl WriteGate {
    /// The command the module binds to the channel (the key of the contract, `card.move`); none = not bound.
    fn bound(&self, channel: &str) -> Option<String> {
        self.records.command_of(&self.form, channel)
    }

    /// Whether the description connects the channel to a button of the page (the dialogs on a column). The gestures of a place never
    /// come here: they go by `fire`.
    fn described(&self, channel: &str) -> bool {
        self.actions
            .iter()
            .any(|a| a.trigger == "button" && a.command == channel)
    }
}

/// The text of a value as the module takes it (a whole number without a point).
fn text_of(value: &WriteValue) -> String {
    match value {
        WriteValue::Text(t) | WriteValue::Time(t) => t.clone(),
        WriteValue::Number(n) if n.fract() == 0.0 => format!("{n:.0}"),
        WriteValue::Number(n) => n.to_string(),
        WriteValue::Flag(f) => f.to_string(),
    }
}

impl FormWrite for WriteGate {
    fn can(&self, channel: &str, record: Option<&str>) -> bool {
        if !self.described(channel) {
            return false;
        }
        // records that bind no command for the channel leave it locked
        self.bound(channel).is_some() && self.records.can_channel(&self.form, channel, record)
    }

    fn send(
        &self,
        channel: &str,
        record: Option<&str>,
        args: &[WriteArg],
    ) -> Result<WriteDone, WriteRefused> {
        // the gate decides on its own: a caller that did not ask `can` first still gets `locked`
        if !self.can(channel, record) {
            return Err(WriteRefused::new("locked"));
        }
        let values: Vec<(String, String)> = args
            .iter()
            .map(|a| (a.name.clone(), text_of(&a.value)))
            .collect();
        self.answer(self.records.invoke(&self.form, channel, record, &values))
    }

    fn may_fire(&self, gesture: Gesture<'_>, record: Option<&str>) -> Result<(), WriteRefused> {
        self.may_gesture(gesture, record)
    }

    fn fire(
        &self,
        gesture: Gesture<'_>,
        record: Option<&str>,
        args: &[WriteArg],
    ) -> Result<WriteDone, WriteRefused> {
        self.gesture(gesture, record, args)
    }
}

#[cfg(test)]
mod ask;
#[cfg(test)]
mod fire_confirm_tests;
#[cfg(test)]
mod fire_tests;
#[cfg(test)]
mod tests;
