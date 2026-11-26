//! The fixed events (`events.sample`): the same on every run, the same as the contract says, never a raw identifier.

mod fixed_support;

use chrono::{DateTime, NaiveDate};
use fixed_support::{KEY, ZONE, anchor, builder, golden, rows, set, standard_windows, wish};
use fs_events::draft::{Appointments, When};
use fs_events::ids::calendar_iri;
use fs_events::sample::sample;
use fs_forms_testdata::{Period, SampleCode, SampleRow, SetWish, standard_fixed};
use fs_interfaces::forms::{self, Art};
use fs_types::BasicValue;

#[test]
fn the_same_description_twice_gives_the_same_set() {
    for period in standard_windows() {
        assert_eq!(set(period).render(), set(period).render(), "{period:?}");
    }
}

#[test]
fn golden_month_2026_11() {
    golden(
        "events-sample-2026-11",
        &set(Period::Month(2026, 11)).render(),
    );
}

#[test]
fn golden_week_2026_w47() {
    golden(
        "events-sample-2026-W47",
        &set(Period::Week(2026, 47)).render(),
    );
}

#[test]
fn the_fixed_now_is_the_one_of_the_sample() {
    let s = set(Period::Month(2026, 11));
    let now = s.now.expect("a fixed set carries its now");
    assert_eq!(
        now.wall().to_string(),
        "2026-11-16 09:00:00",
        "08:00Z in Berlin"
    );
    assert!(
        builder(
            Period::Month(2026, 11),
            SetWish::new(fixed_support::FORM, "Event", 3)
        )
        .build()
        .expect("generated")
        .now
        .is_none(),
        "generated data carry no now"
    );
}

/// What the fixture says about one row, in the terms of the contract: key, title, instants (or whole days), lane name.
type Shown = (String, String, String, String, bool, String);

fn shown(rows: &[SampleRow]) -> Vec<Shown> {
    let at = |t: &str| {
        DateTime::parse_from_rfc3339(t)
            .map_or_else(|_| t.to_owned(), |d| d.timestamp_millis().to_string())
    };
    let mut out: Vec<Shown> = rows
        .iter()
        .map(|r| {
            (
                r.key.clone(),
                r.get("label").unwrap_or_default().to_owned(),
                at(r.get("start").unwrap_or_default()),
                at(r.get("end").unwrap_or_default()),
                r.get("all-day") == Some("true"),
                r.get("lane").unwrap_or_default().to_owned(),
            )
        })
        .collect();
    out.sort();
    out
}

/// What the contract (the fake of fs-events over the same sample) answers for the window: derived, no list written by hand.
async fn from_contract(period: Period) -> Vec<Shown> {
    let data = sample();
    let fake = data.fake();
    let span = period.span().expect("span");
    let zone = anchor(period).zone;
    let rule = fs_jscalendar::time::Zone::parse(ZONE).expect("zone");
    let midnight = |d: NaiveDate| {
        let utc = rule.to_utc(d.and_hms_opt(0, 0, 0).unwrap());
        fs_events::draft::Utc::from_millis(utc.timestamp_millis())
    };
    let window = fake
        .window(
            midnight(span.first),
            midnight(span.first + chrono::Duration::days(span.days)),
            &zone,
            &[],
        )
        .await
        .expect("window");
    let mut out: Vec<Shown> = window
        .instances
        .iter()
        .map(|i| {
            let appointment = data
                .appointments
                .iter()
                .map(|(_, a)| a)
                .find(|a| a.id == i.appointment)
                .expect("appointment");
            let key = i.recurrence_id.map_or_else(
                || appointment.uid.clone(),
                |r| format!("{}#{}", appointment.uid, r.canonical()),
            );
            let lane = data
                .calendars
                .iter()
                .find(|c| c.id == i.calendar)
                .map(|c| c.name.clone())
                .expect("calendar");
            let (start, end, whole) = match &i.when {
                When::AllDay { first, days } => {
                    let first = fs_events::zone::naive_date(*first).expect("date");
                    let end = first + chrono::Duration::days(i64::from(*days));
                    (first.to_string(), end.to_string(), true)
                }
                _ => (
                    i.start.millis().to_string(),
                    i.end.millis().to_string(),
                    false,
                ),
            };
            (key, i.title.clone(), start, end, whole, lane)
        })
        .collect();
    out.sort();
    out
}

#[tokio::test(flavor = "current_thread")]
async fn the_rows_are_what_the_contract_answers_for_the_window() {
    for period in [
        Period::Month(2026, 11),
        Period::Week(2026, 47),
        Period::Month(2026, 3),
    ] {
        let want = from_contract(period).await;
        let got = shown(&rows(period));
        assert!(!want.is_empty(), "{period:?}: the window has events");
        assert_eq!(got, want, "{period:?}");
    }
}

#[test]
fn every_appointment_of_the_sample_shows_in_the_month_it_starts_in_or_is_excused() {
    use chrono::Datelike as _;
    let data = sample();
    let mut missing: Vec<&str> = Vec::new();
    for (slug, a) in &data.appointments {
        // The one excuse: the calendar is in the trash (derived from the sample, not listed here).
        let trashed = data
            .calendars
            .iter()
            .any(|c| c.id == a.calendar && c.trash.is_some());
        let first = fs_events::instances::series_start(&a.when).expect("a start");
        let month = Period::Month(first.year(), first.month());
        let shown = rows(month)
            .iter()
            .any(|r| r.key.split('#').next() == Some(a.uid.as_str()));
        if !trashed && !shown {
            missing.push(slug);
        }
    }
    assert!(
        missing.is_empty(),
        "not in the window of its first month: {missing:?}"
    );
    // And the windows of the pictures are not empty.
    for period in standard_windows() {
        assert!(!rows(period).is_empty(), "{period:?}");
    }
}

#[test]
fn the_lane_is_the_name_of_the_calendar_never_its_iri() {
    let data = sample();
    let names: Vec<&str> = data.calendars.iter().map(|c| c.name.as_str()).collect();
    let rows = rows(Period::Month(2026, 11));
    for row in &rows {
        let lane = row.get("lane").expect("every event has its calendar");
        assert!(names.contains(&lane), "{}: lane `{lane}`", row.key);
    }
    let stored = set(Period::Month(2026, 11));
    let iri = calendar_iri(data.calendars[1].id).canonical();
    assert!(
        stored.render().contains(&iri),
        "the stored object keeps the reference"
    );
}

/// Whether a text looks like a raw identifier: an IRI, a UUID, or the hex of an object id.
fn raw(text: &str) -> bool {
    text.starts_with("urn:")
        || text.starts_with("http://")
        || text.starts_with("https://")
        || uuid::Uuid::parse_str(text).is_ok()
        || (text.len() == 32 && text.chars().all(|c| c.is_ascii_hexdigit()))
}

#[test]
fn no_text_channel_shows_a_raw_identifier() {
    let form = forms::find_key(KEY).expect("form");
    let texts: Vec<&str> = form
        .channels
        .iter()
        .filter(|c| c.arts.contains(&Art::Text))
        .map(|c| c.name.as_str())
        .collect();
    for period in standard_windows() {
        for row in rows(period) {
            for (channel, values) in row
                .values
                .iter()
                .filter(|(c, _)| texts.contains(&c.as_str()))
            {
                for value in values {
                    assert!(!raw(value), "{period:?} {}: {channel} = {value}", row.key);
                }
            }
        }
    }
}

#[test]
fn an_iri_without_a_name_is_an_error_not_an_iri_on_the_screen() {
    let mut s = set(Period::Month(2026, 11));
    s.names.clear();
    let err = s.rows(KEY).expect_err("no names");
    assert_eq!(err.code(), SampleCode::NameMissing);
    let text = format!("{err:?}");
    assert!(text.contains("lane"), "names the channel: {text}");
    assert!(text.contains("urn:fs-testdata:"), "names the key: {text}");
}

#[test]
fn the_registry_names_the_adapters_of_the_contracts() {
    assert_eq!(
        standard_fixed().names(),
        vec!["events.sample", "person-directory.sample"]
    );
}

#[test]
fn an_unknown_name_is_fixed_unknown() {
    let err = builder(Period::Month(2026, 11), wish().fixed("calendars.sample"))
        .build()
        .expect_err("unknown");
    assert_eq!(err.code(), SampleCode::FixedUnknown);
}

#[test]
fn a_fixed_set_with_wishes_of_the_generator_is_fixed_conflict() {
    for conflict in [
        SetWish::new(fixed_support::FORM, "Event", 5).fixed("events.sample"),
        wish().distinct("lane", 2),
        wish().empty("lane", 1),
        wish().invalid(1),
    ] {
        let err = builder(Period::Month(2026, 11), conflict)
            .build()
            .expect_err("conflict");
        assert_eq!(err.code(), SampleCode::FixedConflict);
    }
}

#[test]
fn the_type_of_the_wish_must_be_the_type_of_the_source() {
    let wrong = SetWish::new(fixed_support::FORM, "Task", 0).fixed("events.sample");
    let err = builder(Period::Month(2026, 11), wrong)
        .build()
        .expect_err("type");
    assert_eq!(err.code(), SampleCode::FixedType);
}
