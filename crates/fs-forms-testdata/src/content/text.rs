//! Text of times and values, shared by the contents.
//!
//! Pattern: Adapter (chrono / fs-jscalendar zone <-> RFC 3339 text).

use chrono::NaiveDateTime;
use fs_jscalendar::time::Zone;

use crate::error::{Failure, SampleCode, fail};
use crate::value::{Moment, Value};

/// The zone of the anchor, looked up in the zone database.
pub(crate) fn zone_of(id: &str) -> Result<Zone, Failure> {
    Zone::parse(id).map_err(|_| fail(SampleCode::ZoneUnknown, "zone", id))
}

/// RFC 3339 `date-time` of a wall-clock time in `zone`, with the offset of the zone at that time.
pub(crate) fn rfc3339(local: NaiveDateTime, zone: &Zone) -> String {
    let offset = (local - zone.to_utc(local).naive_utc()).num_seconds();
    let (sign, abs) = if offset < 0 {
        ('-', -offset)
    } else {
        ('+', offset)
    };
    format!(
        "{}{sign}{:02}:{:02}",
        local.format("%Y-%m-%dT%H:%M:%S"),
        abs / 3600,
        abs % 3600 / 60
    )
}

/// The text of a moment: a whole day is a `full-date`, a wall-clock time an RFC 3339 `date-time` with offset.
///
/// # Errors
/// `zone-unknown`.
pub fn moment_text(moment: Moment, zone_id: &str) -> Result<String, Failure> {
    Ok(match moment {
        Moment::Date(d) => d.format("%Y-%m-%d").to_string(),
        Moment::Local(t) => rfc3339(t, &zone_of(zone_id)?),
    })
}

/// A value as JSON (times as text).
pub(crate) fn value_json(value: &Value, zone_id: &str) -> Result<serde_json::Value, Failure> {
    Ok(match value {
        Value::Text(s) | Value::Reference(s) | Value::Raw(s) => s.clone().into(),
        Value::Flag(b) => (*b).into(),
        Value::Number(n) => (*n).into(),
        Value::Time(m) => moment_text(*m, zone_id)?.into(),
    })
}

/// A JSON scalar as the text a row carries.
pub(crate) fn scalar_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
