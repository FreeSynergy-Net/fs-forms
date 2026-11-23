//! What the board may do: the connection between its gestures and the records. The host holds no program name and no code of the
//! program: a gesture is allowed only when the DESCRIPTION has an `[[action]]` for it (`quick-add` calling `create`, `drop`
//! calling `move`, a `button` calling `column.create`) AND the records can do the operation. Every other control stays locked and
//! says "Comes later".
//!
//! What is asked of the reader (the name of a new column, a new name, a limit) is asked by the `input-dialog` of `fs-components`
//! (`text.rs`): a refused line keeps the dialog open and shows the refusal. Rename and limit are `button` actions
//! `column.rename` and `column.limit` of the description.
//!
//! Pattern: Adapter (the board's source and events on the port `Records`), gated by the actions of the description.

use std::sync::{Arc, Mutex, PoisonError};

use fs_components_fields::Say;
use fs_components_kanban::{Answer, BoardEvents, BoardSource, Column, Operation};
use fs_components_widgets::dialogs::{InputDialog, Labels};
use fs_components_widgets::live::Live;

use self::text::{Text, Words};
use crate::records::Records;
use crate::view::ActionView;

mod text;

/// The form of the board and the channels of its gestures (`forms.txt`, `cards-in-columns.1.0`).
const FORM: &str = "cards-in-columns";
const MOVE: &str = "move";
const CREATE: &str = "create";

/// The open question to the reader (at most one per window): the input dialog for a column name, and the area that draws it.
#[derive(Default)]
pub(crate) struct Ask {
    dialog: Mutex<Option<InputDialog>>,
    /// The line above the field of the open dialog (what to type, or why the last line was refused); the tests read it.
    #[cfg_attr(not(test), allow(dead_code))]
    message: Mutex<Option<String>>,
    pub live: Live,
}

impl Ask {
    /// The dialog that is open now.
    pub fn current(&self) -> Option<InputDialog> {
        self.dialog
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// What the open dialog says above its field.
    #[cfg(test)]
    pub fn message(&self) -> Option<String> {
        self.message
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn set(&self, dialog: Option<InputDialog>, message: Option<String>) {
        *self.dialog.lock().unwrap_or_else(PoisonError::into_inner) = dialog;
        *self.message.lock().unwrap_or_else(PoisonError::into_inner) = message;
        self.live.refresh_now();
    }
}

/// The source and the events of one board.
pub(crate) struct Wiring {
    pub actions: Arc<Vec<ActionView>>,
    pub records: Arc<dyn Records>,
    pub ask: Arc<Ask>,
    pub say: Arc<dyn Say>,
    pub labels: Labels,
}

impl Wiring {
    fn has(&self, trigger: &str, command: &str) -> bool {
        self.actions
            .iter()
            .any(|a| a.trigger == trigger && a.command == command)
    }

    /// Whether the description connects `op` to a command at all.
    fn described(&self, op: Operation) -> bool {
        match op {
            Operation::AddCard => self.has_for("quick-add", CREATE),
            Operation::Move => self.has_for("drop", MOVE),
            Operation::AddColumn => self.has("button", "column.create"),
            Operation::Rename => self.has("button", "column.rename"),
            Operation::Limit => self.has("button", "column.limit"),
        }
    }

    /// Whether the description has an `[[action]]` with this trigger that calls the command the module binds to the channel of the
    /// form (the key of the contract, `card.move`); records that bind no command are asked the old way, by the name of the channel.
    fn has_for(&self, trigger: &str, channel: &str) -> bool {
        let key = self.records.command_of(FORM, channel);
        self.has(trigger, key.as_deref().unwrap_or(channel))
    }

    /// Whether the channel is a command of the module (a gesture then goes the written way, [`Records::invoke`]).
    fn written(&self, channel: &str) -> bool {
        self.records.command_of(FORM, channel).is_some()
    }

    /// The answer of a written gesture; after an accepted call the board is drawn again from what the module now says.
    fn sent(&self, channel: &str, record: Option<&str>, values: &[(String, String)]) -> Answer {
        let answer = self.records.invoke(FORM, channel, record, values);
        if answer == Answer::Accepted {
            self.ask.live.refresh_now();
        }
        answer
    }

    /// The column `key` as the records show it now.
    fn current(&self, key: &str) -> Option<Column> {
        self.records.columns().into_iter().find(|c| c.key == key)
    }

    /// Ask the reader one line (keys: dialog id, title, placeholder, hint) and give it to `apply`.
    fn ask_text(
        &self,
        (id, title, placeholder, hint): (&'static str, &str, &str, Option<&str>),
        initial: &str,
        apply: impl Fn(&str) -> Result<(), String> + Send + Sync + 'static,
    ) {
        let words = Words {
            id,
            title: self.say.say(title),
            placeholder: self.say.say(placeholder),
            hint: hint.map(|k| self.say.say(k)),
        };
        Text::new(&self.ask, &self.labels, words, apply).open(initial, None);
    }
}

impl BoardSource for Wiring {
    fn columns(&self) -> Vec<Column> {
        self.records.columns()
    }
    fn supports(&self, op: Operation) -> bool {
        let channel = match op {
            Operation::Move => Some(MOVE),
            Operation::AddCard => Some(CREATE),
            _ => None,
        };
        match channel.filter(|c| self.written(c)) {
            // a written gesture: the module says (may), the board asks the whole board (one record is enough, the drop asks exactly)
            Some(c) => self.described(op) && self.records.can_channel(FORM, c, None),
            None => self.described(op) && self.records.can(op),
        }
    }
}

fn locked() -> Answer {
    Answer::Refused("locked".into())
}

impl BoardEvents for Wiring {
    fn move_card_at(&self, key: &str, from: &str, to: &str, index: Option<usize>) -> Answer {
        if self.written(MOVE) {
            if !self.supports(Operation::Move) || !self.records.can_channel(FORM, MOVE, Some(key)) {
                return locked();
            }
            let mut values = vec![("column".to_owned(), to.to_owned())];
            if let Some(i) = index {
                values.push(("order".to_owned(), i.to_string()));
            }
            return self.sent(MOVE, Some(key), &values);
        }
        if from == to {
            return Answer::Refused("unsupported".into());
        }
        self.move_card(key, from, to)
    }

    fn move_card(&self, key: &str, from: &str, to: &str) -> Answer {
        if self.written(MOVE) {
            return self.move_card_at(key, from, to, None);
        }
        if self.supports(Operation::Move) {
            self.records.move_card(key, from, to)
        } else {
            locked()
        }
    }

    fn add_card(&self, column: &str, title: &str) -> Answer {
        if self.written(CREATE) {
            if !self.supports(Operation::AddCard) {
                return locked();
            }
            let values = [
                ("title".to_owned(), title.to_owned()),
                ("column".to_owned(), column.to_owned()),
            ];
            return self.sent(CREATE, None, &values);
        }
        if self.supports(Operation::AddCard) {
            self.records.add_card(column, title)
        } else {
            locked()
        }
    }

    fn add_column(&self) {
        if !self.supports(Operation::AddColumn) {
            return;
        }
        let (records, say) = (self.records.clone(), self.say.clone());
        let keys = (
            "new-column",
            "kanban.new_column",
            "kanban.column_name",
            None,
        );
        self.ask_text(keys, "", move |name| said(&*say, records.add_column(name)));
    }

    fn rename_column(&self, column: &str) {
        if !self.supports(Operation::Rename) {
            return;
        }
        let (records, say, column) = (self.records.clone(), self.say.clone(), column.to_owned());
        let initial = self.current(&column).map(|c| c.title).unwrap_or_default();
        let keys = ("rename-column", "kanban.rename", "kanban.column_name", None);
        self.ask_text(keys, &initial, move |name| {
            said(&*say, records.rename_column(&column, name))
        });
    }

    fn set_limit(&self, column: &str) {
        if !self.supports(Operation::Limit) {
            return;
        }
        let (records, say, column) = (self.records.clone(), self.say.clone(), column.to_owned());
        let initial = self
            .current(&column)
            .and_then(|c| c.limit)
            .map_or_else(String::new, |n| n.to_string());
        let hint = Some("kanban.limit_hint");
        let keys = (
            "column-limit",
            "kanban.limit",
            "kanban.limit_placeholder",
            hint,
        );
        self.ask_text(keys, &initial, move |text| match parse_limit(text) {
            Ok(limit) => said(&*say, records.set_limit(&column, limit)),
            // not a number: the dialog stays open and says so
            Err(_) => Err(say.say("kanban.limit_invalid")),
        });
    }
}

/// A refusal of the records as the sentence the dialog shows (`kanban.refused.<code>`).
fn said(say: &dyn Say, answer: Answer) -> Result<(), String> {
    match answer {
        Answer::Accepted => Ok(()),
        Answer::Refused(code) => Err(say.say(&format!("kanban.refused.{code}"))),
    }
}

/// The limit a line says: a whole number of cards, `0` = no limit; `Err` when the line is not a number.
fn parse_limit(text: &str) -> Result<Option<u32>, std::num::ParseIntError> {
    let n: u32 = text.trim().parse()?;
    Ok((n > 0).then_some(n))
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_edit;
#[cfg(test)]
mod tests_write;
