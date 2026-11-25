//! One occurrence of the events sample as the values of the channels of a form.
//!
//! Pattern: Projection (occurrence -> `Record`: the shape every strategy of the builder makes, so the stored object is made by the
//! same `Content` as a generated one). No channel is invented: a channel the form does not have gets no value.

use chrono::{Duration, NaiveDate, NaiveDateTime};
use fs_events::Sample;
use fs_events::draft::{Instance, ParticipationStatus, When};
use fs_events::ids::calendar_iri;
use fs_events::zone::{from_utc_ms, naive, naive_date};
use fs_interfaces::forms::{Art, Form};
use fs_jscalendar::time::Zone as Rule;
use fs_types::BasicValue;

use super::Names;
use crate::content::ContentCtx;
use crate::error::{Failure, SampleCode, fail};
use crate::spec::Span;
use crate::value::{Moment, Record, Value};

/// One occurrence with what the appointment it belongs to adds.
pub(super) struct Entry {
    pub(super) instance: Instance,
    pub(super) uid: String,
    pub(super) color: Option<String>,
}

/// What a record is made against: the channels of the content, the zone rule of the view and the form.
pub(super) struct Frame<'a> {
    pub(super) ctx: &'a ContentCtx,
    pub(super) rule: &'a Rule,
    pub(super) form: &'a Form,
}

impl Entry {
    /// The order of the set: first time, end, title, key (the order the window of the contract answers in).
    pub(super) fn order(&self) -> (i64, i64, String, String) {
        let i = &self.instance;
        (
            i.start.millis(),
            i.end.millis(),
            i.title.clone(),
            self.key(),
        )
    }

    /// The key of the record: the `uid` of the appointment, and for an occurrence of a series the recurrence id as well.
    pub(super) fn key(&self) -> String {
        match self.instance.recurrence_id {
            Some(at) => format!("{}#{}", self.uid, at.canonical()),
            None => self.uid.clone(),
        }
    }
}

/// `[first day, first day + days)` in the zone of the view as two instants.
pub(super) fn window_utc(
    window: &Span,
    rule: &Rule,
) -> (fs_events::draft::Utc, fs_events::draft::Utc) {
    let midnight = |d: NaiveDate| {
        let utc = rule.to_utc(d.and_hms_opt(0, 0, 0).unwrap_or_default());
        fs_events::draft::Utc::from_millis(utc.timestamp_millis())
    };
    (
        midnight(window.first),
        midnight(window.first + Duration::days(window.days)),
    )
}

fn local(rule: &Rule, at: fs_events::draft::Utc) -> NaiveDateTime {
    rule.to_local(from_utc_ms(at).and_utc())
}

/// The words of JSCalendar `participationStatus` (RFC 8984 section 4.4.6): the code of `state`, never a sentence.
fn status(s: ParticipationStatus) -> &'static str {
    match s {
        ParticipationStatus::NeedsAction => "needs-action",
        ParticipationStatus::Accepted => "accepted",
        ParticipationStatus::Declined => "declined",
        ParticipationStatus::Tentative => "tentative",
        ParticipationStatus::Delegated => "delegated",
    }
}

/// Start and end as the content wants them: whole days as dates (end exclusive), a time as wall clock in the zone of the view.
fn moments(frame: &Frame<'_>, i: &Instance) -> Result<(Moment, Moment, bool), Failure> {
    let bad = || {
        fail(
            SampleCode::Unsupported,
            "what",
            "a time of the sample is no date",
        )
    };
    Ok(match &i.when {
        When::AllDay { first, days } => {
            let first = naive_date(*first).ok_or_else(bad)?;
            let end = first + Duration::days(i64::from(*days));
            (Moment::Date(first), Moment::Date(end), true)
        }
        When::Floating { start, length } => {
            let start = naive(*start).ok_or_else(bad)?;
            let end = start
                + Duration::days(i64::from(length.days))
                + Duration::seconds(i64::from(length.seconds));
            (Moment::Local(start), Moment::Local(end), false)
        }
        When::Timed { .. } => (
            Moment::Local(local(frame.rule, i.start)),
            Moment::Local(local(frame.rule, i.end)),
            false,
        ),
    })
}

/// A channel of the form of this art, by name.
fn has(form: &Form, name: &str, art: Art) -> bool {
    form.channel(name)
        .is_some_and(|c| c.arts.first() == Some(&art))
}

/// The record of one occurrence, with its key.
///
/// # Errors
/// `unsupported` for a time that is no date.
pub(super) fn record(frame: &Frame<'_>, entry: &Entry) -> Result<(String, Record), Failure> {
    let i = &entry.instance;
    let (start, end, whole) = moments(frame, i)?;
    let mut rec = Record::new();
    if let Some(order) = &frame.ctx.order {
        rec.insert(order.from.clone(), Value::Time(start));
        rec.insert(order.to.clone(), Value::Time(end));
        if let Some(flag) = &order.all_day {
            rec.insert(flag.clone(), Value::Flag(whole));
        }
    }
    if let Some(label) = &frame.ctx.label {
        rec.insert(label.clone(), Value::Text(i.title.clone()));
    }
    let mut text = |channel: &str, value: String| {
        if has(frame.form, channel, Art::Text) {
            rec.insert(channel.to_owned(), Value::Text(value));
        }
    };
    text("lane", calendar_iri(i.calendar).canonical());
    if let Some(color) = &entry.color {
        text("color", color.clone());
    }
    if let Some(mine) = i.my_status {
        text("state", status(mine).to_owned());
    }
    Ok((entry.key(), rec))
}

/// The display names of what the records point at: every calendar, every known person (the reference holds the IRI, the name is here).
pub(super) fn names(data: &Sample) -> Names {
    let calendars = data
        .calendars
        .iter()
        .map(|c| (calendar_iri(c.id).canonical(), c.name.clone()));
    let people = data
        .people
        .iter()
        .map(|k| (k.person.id.canonical(), k.name.clone()));
    calendars.chain(people).collect()
}
