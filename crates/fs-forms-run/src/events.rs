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
            Operation::AddCard => self.has("quick-add", "create"),
            Operation::Move => self.has("drop", "move"),
            Operation::AddColumn => self.has("button", "column.create"),
            Operation::Rename => self.has("button", "column.rename"),
            Operation::Limit => self.has("button", "column.limit"),
        }
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
        self.described(op) && self.records.can(op)
    }
}

fn locked() -> Answer {
    Answer::Refused("locked".into())
}

impl BoardEvents for Wiring {
    fn move_card(&self, key: &str, from: &str, to: &str) -> Answer {
        if self.supports(Operation::Move) {
            self.records.move_card(key, from, to)
        } else {
            locked()
        }
    }

    fn add_card(&self, column: &str, title: &str) -> Answer {
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
            Some(limit) => said(&*say, records.set_limit(&column, limit)),
            // not a number: the dialog stays open and says so
            None => Err(say.say("kanban.limit_invalid")),
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

/// The limit a line says: a whole number of cards, `0` = no limit; `None` when the line is not a number.
fn parse_limit(text: &str) -> Option<Option<u32>> {
    let n: u32 = text.trim().parse().ok()?;
    Some((n > 0).then_some(n))
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_edit;
