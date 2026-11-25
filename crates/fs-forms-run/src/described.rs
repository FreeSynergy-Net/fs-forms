//! What the description tells a source besides the contracts (the checked layout and its `[sample]` text).
//!
//! Pattern: Parameter object.

use fs_forms_layout::Layout;

/// What the description says that a source may need besides the contracts: the checked description itself, and in it the section
/// `[sample]` as text (as the exchange reader gives it). One value for the one question "what does this window show"; there is no
/// second reader of the file in the host. `None` for a view that came without a description (the socket cannot deliver one yet).
///
/// Pattern: Parameter object (what `connect_sample` is told about the description, so the port can grow without a new argument).
#[derive(Clone, Copy, Debug, Default)]
pub struct Described<'a> {
    layout: Option<&'a Layout>,
}

impl<'a> Described<'a> {
    /// The description a view was made from, if there is one.
    #[must_use]
    pub fn of(layout: Option<&'a Layout>) -> Self {
        Self { layout }
    }
    /// No description (a view from memory).
    #[must_use]
    pub fn none() -> Self {
        Self { layout: None }
    }
    /// The checked description.
    #[must_use]
    pub fn layout(&self) -> Option<&'a Layout> {
        self.layout
    }
    /// The text of `[sample]` (TOML with the top-level key `sample`), `None` when the description has none. It is NOT checked here:
    /// its reader is `fs-forms-testdata`.
    #[must_use]
    pub fn sample(&self) -> Option<&'a str> {
        self.layout.and_then(|l| l.doc().sample.as_deref())
    }
}
