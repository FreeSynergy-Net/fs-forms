//! First use (ENTWURF section 7): the calendar month against the declaration of `bars-in-time@1.1.0`, as the catalog of the pinned
//! `fs-interfaces` has it. The checks read the declaration; no channel name of the form is written here except the ones of the
//! wish itself (start, end, lane, all-day).

mod common;

use chrono::{DateTime, Datelike, NaiveDate};
use common::{FORM, KEY, builder, month_wish};
use fs_forms_testdata::{SampleRow, SampleSet};
use fs_interfaces::forms::{self, Art};

fn set(seed: u64) -> SampleSet {
    builder(seed, month_wish()).build().expect("builds")
}

fn rows() -> Vec<SampleRow> {
    set(1).rows(KEY).expect("rows")
}

fn instant(text: &str) -> Option<DateTime<chrono::FixedOffset>> {
    DateTime::parse_from_rfc3339(text).ok()
}

fn day(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .ok()
        .filter(|_| text.len() == 10)
}

#[test]
fn the_pinned_catalog_knows_the_form() {
    let version = forms::parse_version("1.1.0").unwrap();
    let form = forms::find("bars-in-time", &version)
        .expect("bars-in-time@1.1.0 is in the pinned fs-interfaces");
    assert_eq!(form.key(), KEY);
    assert!(FORM.starts_with("bars-in-time@"));
}

#[test]
fn eight_events_and_one_broken_row_in_the_period() {
    let rows = rows();
    assert_eq!(rows.len(), 9);
    let broken: Vec<_> = rows
        .iter()
        .filter(|r| {
            r.get("end")
                .is_some_and(|e| instant(e).is_none() && day(e).is_none())
        })
        .collect();
    assert_eq!(broken.len(), 1, "exactly one row with a broken end");
    for row in rows.iter().filter(|r| !broken.contains(r)) {
        let start = row.get("start").expect("start");
        let first = instant(start).map_or_else(|| day(start).expect("a date"), |t| t.date_naive());
        assert!(
            first.year() == 2026 && first.month() == 5,
            "{start} is not in May 2026"
        );
    }
}

#[test]
fn every_row_keeps_the_declaration() {
    let version = forms::parse_version("1.1.0").unwrap();
    let form = forms::find("bars-in-time", &version).unwrap();
    for row in rows() {
        for (channel, values) in &row.values {
            let declared = form
                .channel(channel)
                .unwrap_or_else(|| panic!("{channel} is no channel of the form"));
            assert!(declared.count.allows(values.len()), "{channel}");
        }
        for required in form.required().filter(|c| !c.arts.contains(&Art::Records)) {
            assert!(
                row.get(&required.name).is_some(),
                "{} missing in {}",
                required.name,
                row.key
            );
        }
    }
}

#[test]
fn times_carry_the_offset_of_the_zone_and_whole_days_are_dates() {
    let rows = rows();
    let mut whole = 0;
    for row in &rows {
        let (Some(start), Some(end)) = (row.get("start"), row.get("end")) else {
            continue;
        };
        if row.get("all-day") == Some("true") {
            whole += 1;
            let (a, b) = (day(start).expect("full-date"), day(end).expect("full-date"));
            assert_eq!((b - a).num_days(), 1, "half open: the end is the next day");
        } else if let (Some(a), Some(b)) = (instant(start), instant(end)) {
            assert_eq!(
                a.offset().local_minus_utc(),
                7200,
                "May in Berlin is +02:00"
            );
            assert!(b >= a, "the end is not before the start");
        }
    }
    assert_eq!(whole, 2);
}

#[test]
fn the_cases_are_there() {
    let rows = rows();
    let timed: Vec<_> = rows
        .iter()
        .filter_map(|r| Some((instant(r.get("start")?)?, instant(r.get("end")?)?)))
        .collect();
    // one over 3 days: the end is on the third day
    assert_eq!(
        timed
            .iter()
            .filter(|(a, b)| (b.date_naive() - a.date_naive()).num_days() == 2)
            .count(),
        1
    );
    // one over the end of the month
    assert_eq!(
        timed
            .iter()
            .filter(|(a, b)| a.month() == 5 && b.month() == 6)
            .count(),
        1
    );
    // four on one day (the day of the group may take ordinary records too)
    let mut per_day = std::collections::BTreeMap::new();
    for (a, b) in &timed {
        if a.date_naive() == b.date_naive() {
            *per_day.entry(a.date_naive()).or_insert(0) += 1;
        }
    }
    assert!(per_day.values().any(|n| *n >= 4), "{per_day:?}");
    // two lanes
    let lanes: std::collections::BTreeSet<_> = rows.iter().filter_map(|r| r.get("lane")).collect();
    assert_eq!(lanes.len(), 2, "{lanes:?}");
}

#[test]
fn objects_are_as2_with_jscalendar_content() {
    let set = set(1);
    assert_eq!(set.objects.len(), 9);
    for object in &set.objects {
        assert_eq!(object.envelope.object_type, "Event");
        assert_eq!(object.content_type, "application/jscalendar+json");
        assert_eq!(object.content["@type"], "Event");
        assert_eq!(object.content["timeZone"], "Europe/Berlin");
        assert!(
            object
                .envelope
                .iri
                .as_deref()
                .is_some_and(|i| i.starts_with("urn:fs-sample:bars-in-time@1.1.0:"))
        );
    }
    assert_eq!(
        set.objects
            .iter()
            .filter(|o| o.content["showWithoutTime"] == true)
            .count(),
        2
    );
    let declared = &set.declared[0];
    assert_eq!(declared.data_type, "Event");
    assert!(
        declared
            .map
            .iter()
            .any(|(c, f)| c == "start" && f == &["start"])
    );
}

#[test]
fn same_seed_same_set_other_seed_other_set() {
    assert_eq!(set(1).render(), set(1).render());
    assert_ne!(set(1).render(), set(2).render());
}

#[test]
fn golden_text_of_the_calendar_month() {
    common::golden("calendar-month", &set(1).render());
}
