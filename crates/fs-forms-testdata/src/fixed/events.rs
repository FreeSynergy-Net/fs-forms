//! `events.sample`: the fixed sample of the contract `events` (`fs_events::sample()`) as `Event` records of a form with an interval.
//!
//! Pattern: Object Mother behind a Registry (the Adapter holds no data: it asks `fs_events::sample()` and the occurrence logic of
//! fs-events, `instances::in_window`, so a series is expanded the way the real module does it, never by a rule of ours). Departures: the
//! times are shown in the zone of the anchor (the zone of the view); the calendar is the lane (its IRI is stored, the name is in `names`).

use std::collections::HashMap;

use fs_events::draft::Appointment;
use fs_events::ids::person;
use fs_events::instances::in_window;
use fs_events::sample::{CUSTOM_OFFSET, CUSTOM_ZONE, sample};
use fs_interfaces::forms::Form;
use fs_types::Zone;

use super::events_record::{Entry, Frame, names, record, window_utc};
use super::{FixedSlice, FixedSource};
use crate::assemble::{label_of, module_form, stored};
use crate::content::{self, ContentCtx, Order};
use crate::defaults;
use crate::error::{Failure, SampleCode, fail};
use crate::spec::{Relation, Span};
use crate::value::Moment;

/// The name of this source.
const NAME: &str = "events.sample";

/// The adapter on the fixed sample of the contract `events`.
#[derive(Clone, Copy, Debug, Default)]
pub struct EventsSample;

fn unsupported(what: impl Into<String>) -> Failure {
    fail(SampleCode::Unsupported, "what", what)
}

/// The interval channels of the form (the relation `interval`, one place for them: the declaration).
fn order_of(form: &Form) -> Result<Order, Failure> {
    defaults::relations(form)?
        .into_iter()
        .find_map(|r| match r {
            Relation::Order { from, to, all_day } => Some(Order { from, to, all_day }),
            Relation::Parent { .. } => None,
        })
        .ok_or_else(|| unsupported(format!("{} has no interval channels", form.key())))
}

impl FixedSource for EventsSample {
    fn name(&self) -> &str {
        NAME
    }

    #[allow(clippy::unnecessary_literal_bound)] // the port says `&str`: another source may build its type at run time
    fn data_type(&self) -> &str {
        "Event"
    }

    fn now(&self, zone: &Zone) -> Result<Moment, Failure> {
        // The sample stamps every appointment with its "now" (no clock is read); the first one tells it.
        let stamp = sample()
            .appointments
            .first()
            .map(|(_, a): &(_, Appointment)| a.updated)
            .ok_or_else(|| unsupported("the sample has no appointments"))?;
        let utc = fs_events::zone::from_utc_ms(stamp).and_utc();
        Ok(Moment::Local(content::zone_of(zone.id())?.to_local(utc)))
    }

    fn fill(&self, form: &Form, window: &Span, zone: &Zone) -> Result<FixedSlice, Failure> {
        let ctx = ContentCtx {
            zone: zone.id().to_owned(),
            label: label_of(form).map(|c| c.name.clone()),
            order: Some(order_of(form)?),
        };
        let rule = content::zone_of(zone.id())?;
        let (from, to) = window_utc(window, &rule);
        let data = sample();
        let customs = HashMap::from([(CUSTOM_ZONE.to_owned(), CUSTOM_OFFSET)]);
        let viewer = person(1);
        let mut found: Vec<Entry> = Vec::new();
        for (slug, a) in &data.appointments {
            let calendar = data.calendars.iter().find(|c| c.id == a.calendar);
            if calendar.is_none_or(|c| c.trash.is_some()) {
                continue; // a calendar in the trash shows nothing (the window of the contract does the same)
            }
            let (instances, damaged) = in_window(a, &customs, Some(&viewer), zone, from, to);
            if let Some(d) = damaged.first() {
                return Err(unsupported(format!("{slug}: {}", d.reason)));
            }
            let color = a
                .color
                .clone()
                .or_else(|| calendar.and_then(|c| c.color.clone()));
            found.extend(instances.into_iter().map(|instance| Entry {
                instance,
                uid: a.uid.clone(),
                color: color.clone(),
            }));
        }
        found.sort_by_cached_key(Entry::order);
        let frame = Frame {
            ctx: &ctx,
            rule: &rule,
            form,
        };
        let content = content::for_type("Event");
        let form_key = form.key();
        let mut objects = Vec::with_capacity(found.len());
        for (n, entry) in found.iter().enumerate() {
            let (key, rec) = record(&frame, entry)?;
            let kind = (form_key.as_str(), "Event");
            objects.push(stored(
                &*content,
                &ctx,
                kind,
                *window,
                None,
                (n, &key),
                &rec,
            )?);
        }
        Ok(FixedSlice {
            objects,
            declared: module_form(form, &*content, &ctx, "Event"),
            ctx,
            names: names(&data),
        })
    }
}
