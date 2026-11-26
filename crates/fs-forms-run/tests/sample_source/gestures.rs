//! The gestures of the board tests on the records of the sample module (move, quick-add), as the host gives them to the module.
//!
//! Pattern: Command (a gesture is an invocation of a channel of the form), over the `Records` port.

use fs_components_kanban::Answer;
use fs_forms_run::Records;

use super::{FORM, card, other_column};

fn pair(name: &str, value: &str) -> (String, String) {
    (name.to_owned(), value.to_owned())
}

/// The card `key` dropped into column `to` (the gesture on the channel `move`).
#[must_use]
pub fn move_card(records: &dyn Records, key: &str, to: &str) -> Answer {
    records.invoke(FORM, "move", Some(key), &[pair("column", to)])
}

/// A card titled `title` quick-added to `column` (the gesture on the channel `create`).
#[must_use]
pub fn add_card(records: &dyn Records, column: &str, title: &str) -> Answer {
    records.invoke(
        FORM,
        "create",
        None,
        &[pair("column", column), pair("title", title)],
    )
}

/// The `n`-th card of a fresh board dropped into a column it is not in (the same cards on every fresh board).
#[must_use]
pub fn move_nth(records: &dyn Records, n: usize) -> Answer {
    let key = card(n);
    move_card(records, &key, &other_column(&key))
}
