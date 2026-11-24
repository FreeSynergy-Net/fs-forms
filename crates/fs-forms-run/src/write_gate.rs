//! The write side of the board for the components: the gestures of a view come as channels of the form (`FormWrite`: `move`, `create`
//! and the three gestures on a column), this gate decides and carries them out. A gesture is allowed only when the DESCRIPTION has an
//! `[[action]]` for it (`quick-add` calling `create`, `drop` calling `move`, a `button` calling `column.create`) AND the records can do
//! it: a channel the module binds to a command goes to the module ([`Records::invoke`], the rights are the module's), any other one is
//! asked the old way ([`Records::can`] and the operations of the records). Every other control stays locked and says so.
//!
//! The host knows no layout of a component: what a gesture carries is read from the component itself ([`carried`]). It does still know
//! the gestures of the board by name (the channels `move`, `create`, `column.*` and their operations, "the old way") for records that
//! bind no command; they leave with the column pseudo-channels once the form names them (card form-neu).
//!
//! Pattern: Adapter (`FormWrite` over `Records`), gated by the actions of the description.

use std::sync::{Arc, Mutex, PoisonError};

use fs_components_core::form_write::{FormWrite, WriteArg, WriteDone, WriteRefused, WriteValue};
use fs_components_kanban::{
    Answer, BoardEvents, COLUMN_CREATE, COLUMN_LIMIT, COLUMN_RENAME, CREATE, Card, Column,
    FormEvents, MOVE, MemoryBoard, MoveRequest, NewCard, Operation,
};

use crate::records::{Records, refused};
use crate::view::ActionView;

/// The gate of one form over the records.
pub(crate) struct WriteGate {
    pub records: Arc<dyn Records>,
    pub actions: Arc<Vec<ActionView>>,
    pub form: String,
}

/// The operation of the old way a channel stands for.
fn operation(channel: &str) -> Option<Operation> {
    match channel {
        MOVE => Some(Operation::Move),
        CREATE => Some(Operation::AddCard),
        COLUMN_CREATE => Some(Operation::AddColumn),
        COLUMN_RENAME => Some(Operation::Rename),
        COLUMN_LIMIT => Some(Operation::Limit),
        _ => None,
    }
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

/// A port that only records the channels a gesture names.
struct Probe(Mutex<Vec<String>>);

impl FormWrite for Probe {
    fn can(&self, _channel: &str, _record: Option<&str>) -> bool {
        true
    }
    fn send(
        &self,
        _channel: &str,
        _record: Option<&str>,
        args: &[WriteArg],
    ) -> Result<WriteDone, WriteRefused> {
        let mut names = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        names.extend(args.iter().map(|a| a.name.clone()));
        Ok(WriteDone::default())
    }
}

/// The channels the component carries in its gestures (a move with a place, a new card): read from the component, never listed here.
/// A channel it carries but gives no value (a drop at the end of a column has no `before`) is handed on as "carried, without value"
/// (empty text): the rule of [`crate::write::args_of`] then never fills it from the record.
fn carried() -> Vec<String> {
    let probe = Arc::new(Probe(Mutex::default()));
    // A column with two cards, so a drop between them names both neighbours (`after`, `before`); the probe sees the channels only.
    let mut column = Column::new("", "");
    column.cards = vec![Card::new("a", ""), Card::new("b", "")];
    let board = MemoryBoard::new(vec![column]);
    let events = FormEvents::new(
        probe.clone(),
        Arc::new(board),
        Arc::new(|key: &str| key.to_owned()),
    );
    // after card `a`: both neighbours (`after` = a, `before` = b)
    let _ = events.moved(&MoveRequest::new("", "", "", 1, Some("a".to_owned())));
    let _ = events.new_card(&NewCard::new("", "", false));
    let mut names = probe
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    names.sort();
    names.dedup();
    names
}

fn arg<'a>(args: &'a [WriteArg], name: &str) -> Option<&'a WriteValue> {
    args.iter().find(|a| a.name == name).map(|a| &a.value)
}

fn arg_text(args: &[WriteArg], name: &str) -> String {
    arg(args, name).map(text_of).unwrap_or_default()
}

impl FormWrite for WriteGate {
    fn can(&self, channel: &str, record: Option<&str>) -> bool {
        if !self.described(channel) {
            return false;
        }
        if self.bound(channel).is_some() {
            self.records.can_channel(&self.form, channel, record)
        } else {
            operation(channel).is_some_and(|op| self.records.can(op))
        }
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
        let answer = if self.bound(channel).is_some() {
            let mut values: Vec<(String, String)> = args
                .iter()
                .map(|a| (a.name.clone(), text_of(&a.value)))
                .collect();
            for name in carried() {
                if !values.iter().any(|(c, _)| *c == name) {
                    values.push((name, String::new()));
                }
            }
            self.records.invoke(&self.form, channel, record, &values)
        } else {
            self.send_old(channel, record, args)
        };
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

impl WriteGate {
    /// The gesture asked the old way: the operations of the records.
    fn send_old(&self, channel: &str, record: Option<&str>, args: &[WriteArg]) -> Answer {
        let (name, column) = (arg_text(args, "name"), arg_text(args, "column"));
        let key = record.unwrap_or_default();
        match channel {
            MOVE => self.records.move_card(key, &column),
            CREATE => self.records.add_card(&column, &arg_text(args, "title")),
            COLUMN_CREATE => self.records.add_column(&name),
            COLUMN_RENAME => self.records.rename_column(key, &name),
            COLUMN_LIMIT => {
                let n: u32 = arg_text(args, "limit").parse().unwrap_or(0);
                self.records.set_limit(key, (n > 0).then_some(n))
            }
            _ => refused("unsupported"),
        }
    }
}

#[cfg(test)]
mod ask;
#[cfg(test)]
mod tests;
