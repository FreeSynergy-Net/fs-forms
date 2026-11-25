//! The closed list of edge cases a wish can ask for. They apply to kinds of channels, never to names of channels or programs.
//!
//! Pattern: Value object (a closed enum; a new case is a new variant plus its strategy in the builder).

/// A hand-made edge case; every case counts into the `count` of its wish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    /// An interval case on the `Order` relation (`times` groups of it); see [`IntervalCase`] for how many records a group is.
    Interval(IntervalCase, u32),
    /// `n` records whose text is very long (several paragraphs).
    LongText(u32),
    /// `n` records whose text is not in Latin script.
    NonLatin(u32),
    /// `roots` chains of `depth` records each; each record points at the one before (needs a `Relation::Parent`).
    Tree {
        /// Levels of each chain (at least 1).
        depth: u8,
        /// Number of chains (at least 1).
        roots: u8,
    },
    /// The list is empty (no record at all; no other case may be given).
    Empty,
}

/// How a pair of times of an `Order` relation is placed relative to the period of the anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalCase {
    /// One whole day (RFC 3339 `full-date`, half open: the end is the next day). One record per group.
    AllDay,
    /// `n` whole days in a row, with times. One record per group.
    MultiDay(u8),
    /// `n` records on ONE day, at different hours. `n` records per group.
    SameDay(u8),
    /// From the last evening of the period into the first morning of the next one. One record per group.
    CrossPeriod,
    /// Start and end are the same instant (`PT0S`). One record per group.
    ZeroLength,
}

impl IntervalCase {
    /// How many records one group of this case makes.
    #[must_use]
    pub fn records(self) -> u32 {
        match self {
            Self::SameDay(n) => u32::from(n),
            _ => 1,
        }
    }
}

impl Case {
    /// How many records the case needs.
    #[must_use]
    pub fn records(self) -> u32 {
        match self {
            Self::Interval(case, times) => case.records() * times,
            Self::LongText(n) | Self::NonLatin(n) => n,
            Self::Tree { depth, roots } => u32::from(depth) * u32::from(roots),
            Self::Empty => 0,
        }
    }
}
