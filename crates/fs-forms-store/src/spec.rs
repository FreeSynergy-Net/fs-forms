//! The table description of fs-objects v5 (`spec::{Col, Logical, Table}`, `migrate::Step`), re-exported so the tables in `tables*.rs`
//! and the tests name one path. The renderer of fs-objects does NOT quote column names, so no column may be an SQL keyword.
//!
//! Pattern: Facade (re-exports only; the types live in fs-objects).

pub use fs_objects::v5::migrate::Step;
pub use fs_objects::v5::spec::{Col, Logical, Table};

/// All statements of a step, in order (tables first, then their indexes). A code step has no SQL text of its own: none.
#[must_use]
pub fn statements(step: &Step) -> Vec<String> {
    match step {
        Step::Create(tables) => tables
            .iter()
            .flat_map(|t| std::iter::once(t.ddl()).chain(t.indexes.iter().map(|i| (*i).to_owned())))
            .collect(),
        Step::Sql(s) => vec![(*s).to_owned()],
        Step::Code(..) => Vec::new(),
    }
}
