//! The values of a record by channel, before they are stored in a standard, and the strategy that makes them.
//!
//! Pattern: Strategy (`ValueMaker`, one per channel kind) + Value object (`Value`, `Moment`, `MakeCx`).

use chrono::{NaiveDate, NaiveDateTime};
use fs_interfaces::forms::{Art, Channel};
use rand::rngs::ChaCha8Rng;

use crate::spec::Span;

/// A point in time as the standard of the content wants it: a whole day or a wall-clock time in the zone of the anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Moment {
    /// A whole day (`full-date`).
    Date(NaiveDate),
    /// Wall-clock time, interpreted in the zone of the anchor (JSCalendar `LocalDateTime`).
    Local(NaiveDateTime),
}

impl Moment {
    /// The wall-clock reading (a whole day is its midnight), for sorting.
    #[must_use]
    pub fn wall(self) -> NaiveDateTime {
        match self {
            Self::Date(d) => d.and_hms_opt(0, 0, 0).unwrap_or_default(),
            Self::Local(t) => t,
        }
    }
}

/// One value of one channel.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Art `text`.
    Text(String),
    /// Art `flag`.
    Flag(bool),
    /// Art `number`.
    Number(i64),
    /// Art `time`.
    Time(Moment),
    /// Art `reference`: the id of another record.
    Reference(String),
    /// A value that is no valid value of its channel (made on purpose by `invalid`); kept as the text the module "sent".
    Raw(String),
}

/// How the text of a record looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    /// A few short Latin words.
    Plain,
    /// Several paragraphs.
    Long,
    /// Words in a script that is not Latin.
    NonLatin,
}

/// What a strategy needs to make one value; it never reads the clock.
pub struct MakeCx<'a> {
    /// The stream of this set (ChaCha8, seeded from the seed and the form).
    pub rng: &'a mut ChaCha8Rng,
    /// The place of the record in the set, from 0.
    pub index: u32,
    /// The period of the anchor.
    pub span: Span,
    /// The values the channel may take when the wish said `distinct` (empty = free).
    pub pool: &'a [String],
    /// The ids of the records made before this one (for references).
    pub earlier: &'a [String],
    /// The look of the text.
    pub flavor: Flavor,
}

/// Pattern: Strategy – one implementation per `Art`; the builder picks it from the declaration of the channel and knows no name.
///
/// Makes the values of one kind of channel (`Art`). Records and commands make no values and have no strategy.
pub trait ValueMaker {
    /// The kind of channel this strategy serves.
    fn art(&self) -> Art;

    /// A value for `channel`; `None` when there is none to give (a reference with no earlier record).
    fn make(&self, cx: &mut MakeCx<'_>, channel: &Channel) -> Option<Value>;
}

/// The values of one record by channel name (a channel the record leaves empty is not in the map).
pub type Record = std::collections::BTreeMap<String, Value>;
