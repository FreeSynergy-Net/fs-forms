//! Which source feeds a block: the one rule, asked by every reader of a description (the host, the test data).
//!
//! Pattern: Information expert (the description knows its numbered sources, so it answers; no reader repeats the rule).

use crate::doc::{BindFrom, BlockDoc, LayoutDoc};

impl LayoutDoc {
    /// The source `block` reads: the one it names, else the first source (`0` = the description has no source at all).
    #[must_use]
    pub fn source_of(&self, block: &BlockDoc) -> u32 {
        block.source.unwrap_or(u32::from(!self.sources.is_empty()))
    }

    /// The data type of source `n` (counts from 1), or `None` for a number the description does not have.
    #[must_use]
    pub fn source_type(&self, n: u32) -> Option<&str> {
        let i = usize::try_from(n).ok()?.checked_sub(1)?;
        self.sources.get(i).map(|s| s.v.data_type.as_str())
    }

    /// The first grouping of `block` with a source of its own: (the source of the heads, the channel it groups by).
    #[must_use]
    pub fn heads_of<'a>(&self, block: &'a BlockDoc) -> Option<(u32, &'a str)> {
        block
            .bindings
            .iter()
            .find_map(|x| match (&x.v.from, x.v.source) {
                (BindFrom::Grouping(g), Some(h)) => Some((h, g.as_str())),
                _ => None,
            })
    }
}
