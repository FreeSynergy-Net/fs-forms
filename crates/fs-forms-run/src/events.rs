//! What the board may do: the connection between its gestures and the records. The host holds no program name and no code of the
//! program: a gesture is allowed only when the DESCRIPTION has an `[[action]]` for it (`quick-add` calling `create`, `drop`
//! calling `move`, a `button` calling `column.create`) AND the records can do the operation. Every other control stays locked and
//! says "Comes later".
//!
//! The one thing asked of the reader, the name of a new column, is asked by the `input-dialog` of `fs-components`.
//!
//! Pattern: Adapter (the board's source and events on the port `Records`), gated by the actions of the description.

use std::sync::{Arc, Mutex, PoisonError};

use fs_components_fields::Say;
use fs_components_kanban::{Answer, BoardEvents, BoardSource, Column, Operation};
use fs_components_widgets::dialogs::{InputDialog, Labels};
use fs_components_widgets::live::Live;

use crate::records::Records;
use crate::view::ActionView;

/// The open question to the reader (at most one per window): the input dialog for a column name, and the area that draws it.
#[derive(Default)]
pub(crate) struct Ask {
    dialog: Mutex<Option<InputDialog>>,
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

    fn set(&self, dialog: Option<InputDialog>) {
        *self.dialog.lock().unwrap_or_else(PoisonError::into_inner) = dialog;
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
            Operation::Rename | Operation::Limit => false,
        }
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
        let (records, done, left) = (
            self.records.clone(),
            self.ask.clone(),
            self.ask.clone(),
        );
        let dialog = InputDialog::new(
            &self.labels,
            &self.say.say("kanban.new_column"),
            &self.say.say("kanban.column_name"),
        )
        .id("new-column")
        .on_input(move |name| {
            // a refusal (the name exists) keeps the dialog open so the reader can type another
            if records.add_column(name) == Answer::Accepted {
                done.set(None);
            }
        })
        .on_cancel(move || left.set(None));
        self.ask.set(Some(dialog));
    }
}

#[cfg(test)]
mod tests;
