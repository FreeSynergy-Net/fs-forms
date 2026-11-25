//! The types of a content: the channels it needs, what it makes of a record, and the `Content` trait.
//!
//! Pattern: Strategy (the trait `Content`, one strategy per standard) + Value Object (`ContentCtx`, `Order`, `Parts`).

use chrono::{DateTime, Utc};
use fs_forms_bind::Basic;
use fs_interfaces::forms::Form;

use crate::error::Failure;
use crate::value::Record;

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
