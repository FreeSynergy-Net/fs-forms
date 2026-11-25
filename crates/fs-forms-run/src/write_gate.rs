//! The write side of the board for the components: the gestures of a view come as channels of the form (`FormWrite`: `move`, `create`
//! and the three gestures on a column), this gate decides and carries them out. A gesture is allowed only when the DESCRIPTION has an
//! `[[action]]` for it (`quick-add` calling `create`, `drop` calling `move`, a `button` calling `column.create`) AND the records can do
//! it: a channel the module binds to a command goes to the module ([`Records::invoke`], the rights are the module's). Every other
//! control stays locked and says so.
//!
//! The host knows no layout of a component: what a gesture carries is asked of the component ([`carried_channels`]). It does still
//! know the channels of the board by name (`move`, `create`, `column.*`) to see which action of the description a gesture needs;
//! records that bind no command for a channel leave it locked (no second, older way).
//!
//! Pattern: Adapter (`FormWrite` over `Records`), gated by the actions of the description.

use std::sync::Arc;

use fs_components_core::form_write::{FormWrite, WriteArg, WriteDone, WriteRefused, WriteValue};
use fs_components_kanban::{Answer, CREATE, MOVE, carried_channels};

use crate::records::Records;
use crate::view::ActionView;

/// The gate of one form over the records.
pub(crate) struct WriteGate {
    pub records: Arc<dyn Records>,
    pub actions: Arc<Vec<ActionView>>,
    pub form: String,
}

impl WriteGate {
    fn has(&self, trigger: &str, command: &str) -> bool {
        self.actions
            .iter()
            .any(|a| a.trigger == trigger && a.command == command)
    }

    /// The command the module binds to the channel (the key of the contract, `card.move`); none = not bound.
    fn bound(&self, channel: &str) -> Option<String> {
        self.records.command_of(&self.form, channel)
    }

    /// Whether the description connects the channel to a gesture at all. Records that bind no command are asked by the channel's name.
    fn described(&self, channel: &str) -> bool {
        let key = self.bound(channel);
        let key = key.as_deref().unwrap_or(channel);
        match channel {
            MOVE => self.has("drop", key),
            CREATE => self.has("quick-add", key),
            _ => self.has("button", channel),
        }
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
        let mut values: Vec<(String, String)> = args
            .iter()
            .map(|a| (a.name.clone(), text_of(&a.value)))
            .collect();
        // a channel the gesture carries but gives no value (a drop at the end of a column has no `before`) is handed on as "carried,
        // without value" (empty text): the rule of [`crate::write::args_of`] then never fills it from the record
        for name in carried_channels() {
            if !values.iter().any(|(c, _)| *c == name) {
                values.push((name, String::new()));
            }
        }
        let answer = self.records.invoke(&self.form, channel, record, &values);
        match answer {
            Answer::Accepted => Ok(WriteDone::default()),
            Answer::Refused(reason) => Err(WriteRefused::new(&reason.code)),
            // the gate asks no question of the reader: a question is a refusal with its own code, never a silent "unsupported"
            Answer::Ask(_) => Err(WriteRefused::new(
                fs_components_core::form_write::ASK_UNSUPPORTED,
            )),
            // an answer of a later version of the board is not carried out
            _ => Err(WriteRefused::new("unsupported")),
        }
    }
}

#[cfg(test)]
mod ask;
#[cfg(test)]
mod tests;
