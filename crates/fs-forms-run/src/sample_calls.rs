//! The three gestures of the sample board as calls of the bound commands ([`Records::invoke`]), for pictures and tests: what a
//! drop, a quick-add and a column button send, without making the arguments by hand each time.
//!
//! Pattern: Facade over `Records::invoke` (the one way a gesture reaches records; no second, older way beside it).

use fs_components_kanban::{Answer, COLUMN_CREATE, CREATE, MOVE};

use crate::records::Records;
use crate::sample::BOARD_FORM;

fn pair(name: &str, value: &str) -> (String, String) {
    (name.to_owned(), value.to_owned())
}

/// The card `key` dropped into column `to` (the command bound to `move`).
#[must_use]
pub fn sample_move_card(records: &dyn Records, key: &str, to: &str) -> Answer {
    records.invoke(BOARD_FORM, MOVE, Some(key), &[pair("column", to)])
}

/// A card titled `title` quick-added to `column` (the command bound to `create`).
#[must_use]
pub fn sample_add_card(records: &dyn Records, column: &str, title: &str) -> Answer {
    records.invoke(
        BOARD_FORM,
        CREATE,
        None,
        &[pair("column", column), pair("title", title)],
    )
}

/// A column named `name` (the command bound to `column.create`).
#[must_use]
pub fn sample_add_column(records: &dyn Records, name: &str) -> Answer {
    records.invoke(BOARD_FORM, COLUMN_CREATE, None, &[pair("name", name)])
}
