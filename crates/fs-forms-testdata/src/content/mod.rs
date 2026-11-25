//! Content by standard: how the values of a record are stored in the object of its type, and read back.
//!
//! Pattern: Strategy (`Content`, chosen from the AS2 type by `for_type`: a small table "standard -> content", no program is named)
//! + Adapter (channel values <-> the properties of the standard).

mod as2;
mod jscalendar;
mod text;

use chrono::{DateTime, Utc};
use fs_forms_bind::Basic;
use fs_interfaces::forms::{Art, Form};

use crate::error::Failure;
use crate::value::Record;

pub use text::moment_text;

/// The channels that belong together, as far as content needs them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentCtx {
    /// The IANA id of the zone of the anchor.
    pub zone: String,
    /// The channel that is the name of a record (the first required text channel).
    pub label: Option<String>,
    /// The channels of the `Order` relation, if the form has one.
    pub order: Option<Order>,
}

/// The channels of an `Order` relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    /// The start channel.
    pub from: String,
    /// The end channel.
    pub to: String,
    /// The flag channel of whole days.
    pub all_day: Option<String>,
}

/// What a content strategy makes of one record.
pub struct Parts {
    /// The display name of the object (never empty, ADR 0032).
    pub name: String,
    /// The content in its standard.
    pub json: serde_json::Value,
    /// Start and end as UTC instants, where the standard has them (AS2 `startTime`, `endTime`).
    pub instants: Option<(DateTime<Utc>, Option<DateTime<Utc>>)>,
}

/// The map of a form onto the fields of a content: channel to field names, and the basic type of each field.
pub type FieldMap = (Vec<(String, Vec<String>)>, Vec<(String, Basic)>);

/// Pattern: Strategy – one per standard (JSCalendar for `Event`, plain AS2 for everything else).
///
/// A content standard.
pub trait Content {
    /// The media type of the content.
    fn content_type(&self) -> &'static str;

    /// Stores a record; `uid` is the identity the standard asks for (JSCalendar `uid`).
    ///
    /// # Errors
    /// `unsupported` or `zone-unknown` when the record cannot be stored in this standard.
    fn store(&self, ctx: &ContentCtx, uid: &str, record: &Record) -> Result<Parts, Failure>;

    /// The values of a stored record as text by channel, as a module would send them in `page`.
    fn read(&self, ctx: &ContentCtx, json: &serde_json::Value) -> Vec<(String, Vec<String>)>;

    /// Which field of the standard fills which channel of `form`.
    fn map(&self, ctx: &ContentCtx, form: &Form) -> FieldMap;
}

/// The content of an AS2 type: JSCalendar for `Event`, plain AS2 for the rest.
#[must_use]
pub fn for_type(data_type: &str) -> Box<dyn Content> {
    match data_type {
        "Event" => Box::new(jscalendar::JsEvent),
        _ => Box::new(as2::Plain),
    }
}

/// The basic type of a field that fills a channel of `art`.
pub(crate) fn basic_of(art: Art) -> Basic {
    match art {
        Art::Flag => Basic::Bool,
        Art::Number => Basic::Number,
        Art::Time => Basic::DateTime,
        Art::Reference => Basic::Reference,
        _ => Basic::Text,
    }
}
