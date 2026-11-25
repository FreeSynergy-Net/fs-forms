//! What a test asks for: the wishes, the anchor in time and the relations between channels.
//!
//! Pattern: Value object, with a fluent way to write it (the `SetWish` methods) for tests and later for `[sample]` of a description.

use chrono::NaiveDate;
use fs_types::Zone;

use crate::case::Case;
use crate::error::{Failure, SampleCode, fail};

/// What the builder shall make. Never reads the clock: everything is relative to the anchor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleSpec {
    /// The seed; the same spec and seed give the same set on every platform.
    pub seed: u64,
    /// The period and the zone the times are placed in.
    pub anchor: Anchor,
    /// One wish per form.
    pub sets: Vec<SetWish>,
}

/// The period and the zone of a set (never "now").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anchor {
    /// The period.
    pub period: Period,
    /// The IANA zone the people of the view live in; times carry its offset.
    pub zone: Zone,
}

/// A period of the calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    /// A month: year, month 1..=12.
    Month(i32, u32),
    /// An ISO 8601 week: ISO year, week 1..=53 (starts on Monday).
    Week(i32, u32),
    /// A day: year, month, day.
    Day(i32, u32, u32),
}

/// A period as first day and number of days.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// The first day.
    pub first: NaiveDate,
    /// How many days (at least 1).
    pub days: i64,
}

impl Period {
    /// The first day and the length.
    ///
    /// # Errors
    /// `anchor-bad` when the numbers name no month, week or day.
    pub fn span(self) -> Result<Span, Failure> {
        let bad = || fail(SampleCode::AnchorBad, "what", format!("{self:?}"));
        let (first, days) = match self {
            Self::Month(y, m) => {
                let first = NaiveDate::from_ymd_opt(y, m, 1).ok_or_else(bad)?;
                let next = if m == 12 {
                    NaiveDate::from_ymd_opt(y + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(y, m + 1, 1)
                };
                (first, (next.ok_or_else(bad)? - first).num_days())
            }
            Self::Week(y, w) => (
                NaiveDate::from_isoywd_opt(y, w, chrono::Weekday::Mon).ok_or_else(bad)?,
                7,
            ),
            Self::Day(y, m, d) => (NaiveDate::from_ymd_opt(y, m, d).ok_or_else(bad)?, 1),
        };
        Ok(Span { first, days })
    }
}

/// What one form shall get.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetWish {
    /// The form with a version requirement (`bars-in-time@^1.0.0`), looked up with `fs_interfaces::forms`.
    pub form: String,
    /// The ActivityStreams 2 type of the records (the `type` of a `[[source]]`); it chooses the content standard.
    pub data_type: String,
    /// How many records, cases included.
    pub count: u32,
    /// Channel and how many different values it takes (a text channel such as a lane or a column).
    pub distinct: Vec<(String, u32)>,
    /// Optional channel and how many records leave it empty.
    pub empty: Vec<(String, u32)>,
    /// The edge cases, made first; they count into `count`.
    pub cases: Vec<Case>,
    /// Extra records (not in `count`) whose time pair is broken on purpose, so a view has to show a partial reading.
    pub invalid: u32,
}

impl SetWish {
    /// A wish for `count` records of `data_type` in `form`.
    #[must_use]
    pub fn new(form: &str, data_type: &str, count: u32) -> Self {
        Self {
            form: form.to_owned(),
            data_type: data_type.to_owned(),
            count,
            distinct: Vec::new(),
            empty: Vec::new(),
            cases: Vec::new(),
            invalid: 0,
        }
    }

    /// `channel` takes `n` different values.
    #[must_use]
    pub fn distinct(mut self, channel: &str, n: u32) -> Self {
        self.distinct.push((channel.to_owned(), n));
        self
    }

    /// `n` records leave the optional `channel` empty.
    #[must_use]
    pub fn empty(mut self, channel: &str, n: u32) -> Self {
        self.empty.push((channel.to_owned(), n));
        self
    }

    /// One more edge case.
    #[must_use]
    pub fn case(mut self, case: Case) -> Self {
        self.cases.push(case);
        self
    }

    /// `n` extra records with a broken time pair.
    #[must_use]
    pub fn invalid(mut self, n: u32) -> Self {
        self.invalid = n;
        self
    }
}

/// How channels of a form belong together, as long as the catalog does not carry it (forms 0.3.0 will; then this goes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Relation {
    /// ISO 8601-1 interval `[from, to)` of two `time` channels; `all_day` names the `flag` channel that says "whole days"
    /// (then both are `full-date`).
    Order {
        /// The channel of the start.
        from: String,
        /// The channel of the end.
        to: String,
        /// The flag channel for whole days, if the form has one.
        all_day: Option<String>,
    },
    /// The `reference` channel `child` points at the parent record of the same form (a tree).
    Parent {
        /// The channel that holds the reference.
        child: String,
    },
}

impl Relation {
    /// An order relation.
    #[must_use]
    pub fn order(from: &str, to: &str, all_day: Option<&str>) -> Self {
        Self::Order {
            from: from.to_owned(),
            to: to.to_owned(),
            all_day: all_day.map(str::to_owned),
        }
    }
}
