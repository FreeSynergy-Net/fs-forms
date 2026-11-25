//! JSCalendar (RFC 8984) content for the AS2 type `Event`: `start` as `LocalDateTime` with `timeZone`, the length as `duration`,
//! whole days as `showWithoutTime`. The projection to the channels (RFC 3339 with the offset of the zone, `full-date`) is what a
//! real module does in `page`.
//!
//! Pattern: Strategy (`Content`) + Template method (`calendar_object!` of fs-jscalendar gives the shared members; the type adds `duration`).

// The members mirror the JSCalendar property names one to one; the macro writes them.
#![allow(missing_docs)]

use chrono::{Duration, NaiveDateTime};
use fs_interfaces::forms::{Art, Form};
use fs_jscalendar::model::VENDOR;
use fs_jscalendar::time::Dur;

use super::text::{rfc3339, scalar_text, value_json, zone_of};
use super::{Content, ContentCtx, FieldMap, Parts, basic_of};
use crate::error::{Failure, SampleCode, fail};
use crate::value::{Moment, Record, Value};
use fs_forms_bind::Basic;

fs_jscalendar::calendar_object!(
    /// A JSCalendar `Event` (only what a sample needs: the owner crate of the real type is not in this graph).
    SampleEvent, "Event" {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub duration: Option<String>,
    }
);

const LOCAL: &str = "%Y-%m-%dT%H:%M:%S";

/// Events in JSCalendar. Needs an `Order` relation: it says which channels are start and end.
pub struct JsEvent;

fn unsupported(what: &str) -> Failure {
    fail(SampleCode::Unsupported, "what", what)
}

/// `P<d>D` / `PT<s>S` of a wall-clock difference (days nominal, the rest exact seconds).
fn duration_text(from: NaiveDateTime, to: NaiveDateTime) -> String {
    let diff = to - from;
    let days = diff.num_days();
    let secs = (diff - Duration::days(days)).num_seconds();
    match (days, secs) {
        (0, 0) => "PT0S".to_owned(),
        (d, 0) => format!("P{d}D"),
        (0, s) => format!("PT{s}S"),
        (d, s) => format!("P{d}DT{s}S"),
    }
}

impl Content for JsEvent {
    fn content_type(&self) -> &'static str {
        "application/jscalendar+json"
    }

    fn store(&self, ctx: &ContentCtx, uid: &str, record: &Record) -> Result<Parts, Failure> {
        let order = ctx
            .order
            .as_ref()
            .ok_or_else(|| unsupported("an Event needs an Order relation"))?;
        let zone = zone_of(&ctx.zone)?;
        let Some(Value::Time(from)) = record.get(&order.from) else {
            return Err(unsupported("the start of an Event is no time"));
        };
        let start = match *from {
            Moment::Date(d) => d.and_hms_opt(0, 0, 0).unwrap_or_default(),
            Moment::Local(t) => t,
        };
        let duration = match record.get(&order.to) {
            Some(Value::Time(Moment::Date(e))) => Some(duration_text(
                start,
                e.and_hms_opt(0, 0, 0).unwrap_or_default(),
            )),
            Some(Value::Time(Moment::Local(e))) => Some(duration_text(start, *e)),
            Some(Value::Raw(raw)) => Some(raw.clone()),
            _ => None,
        };
        let mut event = SampleEvent {
            uid: uid.to_owned(),
            title: match ctx.label.as_ref().and_then(|l| record.get(l)) {
                Some(Value::Text(t)) => Some(t.clone()),
                _ => None,
            },
            start: Some(start.format(LOCAL).to_string()),
            time_zone: Some(ctx.zone.clone()),
            duration: duration.clone(),
            ..Default::default()
        };
        event.show_without_time = order.all_day.as_ref().and_then(|c| match record.get(c) {
            Some(Value::Flag(b)) => Some(*b),
            _ => None,
        });
        event.normalise();
        let own = [
            Some(&order.from),
            Some(&order.to),
            order.all_day.as_ref(),
            ctx.label.as_ref(),
        ];
        for (channel, value) in record.iter().filter(|(c, _)| !own.contains(&Some(c))) {
            event.set_vendor(&format!("{VENDOR}{channel}"), value_json(value, &ctx.zone)?);
        }
        let end = duration
            .as_deref()
            .and_then(|d| Dur::parse(d).ok())
            .map(|d| d.end_utc(&zone, start));
        let name = event.display_title();
        Ok(Parts {
            name,
            json: event.to_value(),
            instants: Some((zone.to_utc(start), end)),
        })
    }

    fn read(&self, ctx: &ContentCtx, json: &serde_json::Value) -> Vec<(String, Vec<String>)> {
        let (Some(order), Ok(event)) = (ctx.order.as_ref(), SampleEvent::from_value(json.clone()))
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if let (Some(label), Some(title)) = (&ctx.label, &event.title) {
            out.push((label.clone(), vec![title.clone()]));
        }
        let whole = event.show_without_time == Some(true);
        let start = event
            .start
            .as_deref()
            .and_then(|s| fs_jscalendar::time::parse_local(s).ok());
        let zone = zone_of(&ctx.zone).ok();
        if let (Some(start), Some(zone)) = (start, zone.as_ref()) {
            let shown = |t: NaiveDateTime| {
                if whole {
                    t.format("%Y-%m-%d").to_string()
                } else {
                    rfc3339(t, zone)
                }
            };
            out.push((order.from.clone(), vec![shown(start)]));
            if let Some(d) = event.duration.as_deref() {
                let end = Dur::parse(d)
                    .ok()
                    .map(|dur| zone.to_local(dur.end_utc(zone, start)));
                out.push((
                    order.to.clone(),
                    vec![end.map_or_else(|| d.to_owned(), shown)],
                ));
            }
        }
        if let (Some(c), Some(b)) = (&order.all_day, event.show_without_time) {
            out.push((c.clone(), vec![b.to_string()]));
        }
        for (key, value) in &event.extra {
            if let Some(channel) = key.strip_prefix(VENDOR) {
                out.push((channel.to_owned(), vec![scalar_text(value)]));
            }
        }
        out
    }

    fn map(&self, ctx: &ContentCtx, form: &Form) -> FieldMap {
        let mut map = Vec::new();
        let mut fields = Vec::new();
        let mut add = |channel: &str, field: String, basic: Basic| {
            map.push((channel.to_owned(), vec![field.clone()]));
            fields.push((field, basic));
        };
        for channel in &form.channels {
            let Some(art) = channel.arts.first().copied() else {
                continue;
            };
            let name = channel.name.as_str();
            match (&ctx.order, art) {
                _ if matches!(art, Art::Records | Art::Command) => {}
                _ if ctx.label.as_deref() == Some(name) => {
                    add(name, "title".to_owned(), Basic::Text);
                }
                (Some(o), _) if o.from == name => add(name, "start".to_owned(), Basic::DateTime),
                // A derived field: the end is start + duration.
                (Some(o), _) if o.to == name => add(name, "end".to_owned(), Basic::DateTime),
                (Some(o), _) if o.all_day.as_deref() == Some(name) => {
                    add(name, "showWithoutTime".to_owned(), Basic::Bool);
                }
                _ => add(name, format!("{VENDOR}{name}"), basic_of(art)),
            }
        }
        (map, fields)
    }
}
