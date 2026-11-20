//! The records a block draws: a port the host asks, so no block holds data of its own. In this version no provider is
//! connected, so the real implementation is [`NoRecords`] (nothing: the blocks stay empty and say so); [`crate::SampleBoard`] is
//! built-in, writable sample data for pictures and development (`--sample-data`), never used by a starter.
//!
//! The port also says which changes the records can take ([`Records::can`]) and takes them (`add_card`, `move_card`,
//! `add_column`, `rename_column`, `set_limit`); every one defaults to "cannot", so a provider that does not name an operation leaves its control locked.
//!
//! Pattern: Port (`Records`) with a null object and a fixture.

use fs_components_kanban::{Answer, Column, Operation};

/// One entry of a side list (a project, a label).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideItem {
    /// The key given back on selection.
    pub key: String,
    /// What is shown.
    pub label: String,
}

/// The answer of records that cannot do an operation.
fn unsupported() -> Answer {
    Answer::Refused("unsupported".into())
}

/// What the blocks of the host draw their records from, and which changes they take.
pub trait Records: Send + Sync {
    /// The columns with their cards of a board.
    fn columns(&self) -> Vec<Column>;
    /// The entries of a side list.
    fn side(&self) -> Vec<SideItem>;
    /// Whether these records are made-up sample data: the window then says so (it never shows invented records as real ones).
    fn is_sample(&self) -> bool {
        false
    }
    /// The sentence that says why the data source the description names is not there (the window shows it as a problem in its
    /// status line); `None` when the records are what they claim to be.
    fn unavailable(&self) -> Option<String> {
        None
    }
    /// Whether these records can do `op` now. A control of an operation they cannot do stays locked ("Comes later").
    fn can(&self, _op: Operation) -> bool {
        false
    }
    /// A card titled `title` in `column` (only called when [`Records::can`] says [`Operation::AddCard`]).
    fn add_card(&self, _column: &str, _title: &str) -> Answer {
        unsupported()
    }
    /// The card `key` from column `from` to column `to` (only called when [`Records::can`] says [`Operation::Move`]).
    fn move_card(&self, _key: &str, _from: &str, _to: &str) -> Answer {
        unsupported()
    }
    /// A column named `title` (only called when [`Records::can`] says [`Operation::AddColumn`]).
    fn add_column(&self, _title: &str) -> Answer {
        unsupported()
    }
    /// The column `column` named `name` (only called when [`Records::can`] says [`Operation::Rename`]).
    fn rename_column(&self, _column: &str, _name: &str) -> Answer {
        unsupported()
    }
    /// The column `column` limited to `limit` cards, `None` = no limit (only called when [`Records::can`] says
    /// [`Operation::Limit`]).
    fn set_limit(&self, _column: &str, _limit: Option<u32>) -> Answer {
        unsupported()
    }
}

/// No provider is connected: nothing to draw.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoRecords;

impl Records for NoRecords {
    fn columns(&self) -> Vec<Column> {
        Vec::new()
    }
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
}

/// The data source of the description could not be reached: nothing to draw, and the window says why (never silently empty).
#[derive(Clone, Debug)]
pub struct Unavailable(pub String);

impl Records for Unavailable {
    fn columns(&self) -> Vec<Column> {
        Vec::new()
    }
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn unavailable(&self) -> Option<String> {
        Some(self.0.clone())
    }
}
