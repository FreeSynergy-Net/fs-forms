//! Guarantees 1 and 2: anonymous answers leave no trace, and results change only in batches.

mod common;

use std::sync::{Arc, Mutex};

use common::*;
use fs_forms_core::{ALL_STAGES, Anonymity, SurveyChange, SurveyFeed, SurveyListener};

#[derive(Default)]
struct Seen(Mutex<Vec<SurveyChange>>);

impl SurveyListener for Seen {
    fn changed(&self, c: &SurveyChange) {
        self.0.lock().unwrap().push(c.clone());
    }
}

async fn small(e: &Env, a: Anonymity) -> (fs_forms_core::Survey, fs_objects::ObjectId) {
    let s = e.repo.create("Lunch", a, &[]).await.unwrap();
    e.repo.edit(s.id(), "Lunch", a, 3).await.unwrap();
    let (_, q) = e
        .repo
        .add_question(s.id(), pick().required())
        .await
        .unwrap();
    (e.repo.publish(s.id()).await.unwrap(), q)
}

/// True when `w` (36 bytes) has the shape of a UUID: 8-4-4-4-12 hex digits.
fn is_uuid(w: &[u8]) -> bool {
    w.len() == 36
        && w.iter().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                *c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

/// Counts the dates in a raw text: (the epoch day `1970-01-01`, any other date). First every real
/// UUID (8-4-4-4-12 hex digits, the ids of the box) is blanked out, because the first hex digits of
/// a UUIDv7 are the clock and can read like `2026-`. In the rest, every `dddd-d` (four digits, a
/// dash, a digit) starts a date in ANY spelling: padded or not (`2026-11-5`), year and month only
/// (`2026-11`), ordinal (`2026-322`), with or without time or zone, alone or inside a longer
/// string. Each one counts as the epoch day only if it reads `1970-01-01`, otherwise as another
/// date. This is never weaker than the old search for `dddd-dd-dd` and replaces the search for the
/// current year, which said nothing in any other year and could hit a random id.
///
/// Not found (the old test did not see these either): a time without a date (`T10:00:00`), Unix
/// seconds (`1790000000`), the compact ISO form (`20261118T100000Z`).
fn dates_in(text: &str) -> (usize, usize) {
    let mut b = text.as_bytes().to_vec();
    let mut i = 0;
    while i + 36 <= b.len() {
        if is_uuid(&b[i..i + 36]) {
            b[i..i + 36].fill(b'_');
            i += 36;
        } else {
            i += 1;
        }
    }
    let mut counts = (0, 0);
    for i in 0..b.len().saturating_sub(5) {
        let w = &b[i..i + 6];
        if w[..4].iter().all(u8::is_ascii_digit) && w[4] == b'-' && w[5].is_ascii_digit() {
            if b[i..].starts_with(b"1970-01-01") {
                counts.0 += 1;
            } else {
                counts.1 += 1;
            }
        }
    }
    counts
}

#[test]
fn dates_in_sees_every_form_of_a_date_in_any_year_and_no_random_id() {
    // ids: real UUIDs (8-4-4-4-12), not a date, whatever digits they hold
    for id in [
        "01a02026-2026-7000-8000-202620262026", // UUIDv7: group 1 ends in the year, group 2 is the year
        "0a1b2c3d-2026-1234-5678-123456789012",
        "20260101-2026-0101-2026-202601012026",
        "00002026-2026-2026-2026-202620262026",
        "ABCDEF12-2026-7ABC-8DEF-2026ABCDEF12", // upper case
    ] {
        assert_eq!(dates_in(&format!("\"{id}\"")), (0, 0), "{id}");
        assert_eq!(
            dates_in(&format!("{{\"id\":\"{id}\",\"n\":1}}")),
            (0, 0),
            "{id}"
        );
    }
    for year in [2025, 2026, 2027, 2040] {
        for form in [
            format!("\"{year}-05-05T10:00:00Z\""), // time with zone
            format!("\"{year}-05-05T10:00:00\""),  // time without zone
            format!("\"{year}-05-05\""),           // plain date
            format!("\"seen at x{year}-05-05T10:00:00Zy\""), // inside a longer string
            format!("{{\"at\":{year}-05-05}}"),    // bare in the JSON
            format!("\"{year}-11-5\""),            // not padded
            format!("\"{year}-11\""),              // year and month
            format!("\"{year}-322\""),             // ordinal date
        ] {
            assert_eq!(dates_in(&form), (0, 1), "{form}");
        }
    }
    // a date right next to a real id is still seen, and the id is still not a date
    assert_eq!(
        dates_in("\"01a02026-2026-7000-8000-202620262026\" \"2026-11-5\""),
        (0, 1)
    );
    assert_eq!(dates_in("\"1970-01-01T00:00:00Z\""), (1, 0));
    assert_eq!(dates_in("\"1970-01-01\" \"1970-01-02\""), (1, 1));
    assert_eq!(dates_in("\"1970-01\" \"1970-1-1\""), (0, 2));
    // a date at the very end of the text is seen too
    assert_eq!(dates_in("x2026-1"), (0, 1));
    assert_eq!(dates_in("x1970-01-01"), (1, 0));
}

#[tokio::test]
async fn submits_leave_nothing_in_main_store_log_or_feed() {
    let e = env().await;
    let (s, q) = small(&e, Anonymity::AnonymousOpen).await;
    let main_before = raw(&e.main).await;
    let seen = Arc::new(Seen::default());
    let _sub = e.repo.subscribe(seen.clone());
    submit_open(&e, &s, q, "a", 4).await;
    assert_eq!(
        raw(&e.main).await,
        main_before,
        "main store untouched by anonymous submits"
    );
    assert!(seen.0.lock().unwrap().is_empty(), "the feed gets nothing");
    assert_eq!(
        log_len(&e.ballots).await,
        0,
        "the ballot store has no activity at all"
    );
}

#[tokio::test]
async fn box_holds_no_person_time_or_ballot_id() {
    let e = env().await;
    let (s, q) = small(&e, Anonymity::AnonymousOpen).await;
    submit_open(&e, &s, q, "a", 2).await;
    submit_open(&e, &s, q, "b", 2).await;
    let content = box_json(&e, &s).await;
    let keys: Vec<_> = content.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        ["ai", "people", "voluntary"],
        "the lane of AIs is its own key; voluntary details are tied to no ballot"
    );
    for ballot in content["people"]["pending"].as_array().unwrap() {
        let answers = ballot.as_array().unwrap();
        assert_eq!(answers.len(), 1);
        let mut k: Vec<_> = answers[0].as_object().unwrap().keys().cloned().collect();
        k.sort();
        assert_eq!(
            k,
            ["choice", "question"],
            "a ballot is its answers and nothing else"
        );
    }
    let all = raw_box(&e, &s).await;
    assert!(!all.contains("people/") && !all.contains(ME));
    let (epoch, other) = dates_in(&all);
    assert_eq!(other, 0, "no real date in the ballot store: {all}");
    assert!(epoch > 0, "the box carries the epoch: {all}");
}

#[tokio::test]
async fn reading_and_reopening_change_nothing() {
    let e = env().await;
    let (s, q) = small(&e, Anonymity::AnonymousOpen).await;
    submit_open(&e, &s, q, "a", 3).await;
    e.repo.close(s.id()).await.unwrap(); // releases a batch of 3
    e.repo.reopen(s.id()).await.unwrap();
    let first = e.repo.summary(s.id(), ALL_STAGES).await.unwrap();
    assert_eq!((first.ballots, count(&first, q, "a")), (3, 3));
    let second = e.repo.summary(s.id(), ALL_STAGES).await.unwrap();
    assert_eq!(first, second, "reading twice shows no change");
    submit_open(&e, &s, q, "b", 2).await;
    let held = e.repo.summary(s.id(), ALL_STAGES).await.unwrap();
    assert_eq!(
        (held.ballots, held.held_back),
        (3, 0),
        "new ballots are not shown, and not counted while open"
    );
    assert_eq!(count(&held, q, "b"), 0);
    e.repo.close(s.id()).await.unwrap();
    let closed = e.repo.summary(s.id(), ALL_STAGES).await.unwrap();
    assert_eq!(
        (closed.ballots, closed.held_back),
        (3, 2),
        "after close the number is real"
    );
    assert_eq!(count(&closed, q, "b"), 0);
    e.repo.reopen(s.id()).await.unwrap();
    assert_eq!(
        e.repo.summary(s.id(), ALL_STAGES).await.unwrap(),
        held,
        "2 < minGroup: no change"
    );
}

#[tokio::test]
async fn a_release_is_never_smaller_than_min_group() {
    let e = env().await;
    let (s, q) = small(&e, Anonymity::AnonymousOpen).await;
    submit_open(&e, &s, q, "a", 2).await;
    e.repo.close(s.id()).await.unwrap();
    let sum = e.repo.summary(s.id(), ALL_STAGES).await.unwrap();
    assert!(sum.hidden && sum.ballots == 0 && sum.held_back == 2);
    assert!(sum.questions[0].hidden && sum.questions[0].counts.is_empty());
    e.repo.reopen(s.id()).await.unwrap();
    submit_open(&e, &s, q, "b", 1).await;
    e.repo.close(s.id()).await.unwrap();
    let sum = e.repo.summary(s.id(), ALL_STAGES).await.unwrap();
    assert_eq!((sum.ballots, sum.held_back, sum.hidden), (3, 0, false));
    let batches = box_json(&e, &s).await["people"]["batches"]
        .as_array()
        .unwrap()
        .clone();
    assert!(
        batches
            .iter()
            .all(|b| b["ballots"].as_array().unwrap().len() >= 3)
    );
}

#[tokio::test]
async fn export_gives_totals_only() {
    let e = env().await;
    let (s, q) = small(&e, Anonymity::AnonymousOpen).await;
    submit_open(&e, &s, q, "a", 3).await;
    e.repo.close(s.id()).await.unwrap();
    let csv = String::from_utf8(e.repo.export(s.id(), "text/csv").await.unwrap()).unwrap();
    assert!(csv.starts_with("question,value,count\r\n") && csv.contains("Which?,a,3"));
    assert!(!csv.contains("person"));
}
