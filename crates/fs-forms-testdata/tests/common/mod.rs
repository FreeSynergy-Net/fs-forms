//! Shared helpers of the tests: the spec of the calendar month (ENTWURF section 7) and the golden file reader.

#![allow(dead_code)]

use fs_forms_testdata::{
    Anchor, Case, IntervalCase, Period, Relation, SampleBuilder, SampleSpec, SetWish,
};
use fs_types::Zone;

pub const FORM: &str = "bars-in-time@^1.0.0";
pub const KEY: &str = "bars-in-time@1.0.0";

pub fn anchor() -> Anchor {
    Anchor {
        period: Period::Month(2026, 5),
        zone: Zone::iana("Europe/Berlin").unwrap(),
    }
}

/// 8 events: 2 whole days, 1 over 3 days, 4 on one day, 1 over the end of the month, 2 lanes; plus 1 broken row.
pub fn month_wish() -> SetWish {
    SetWish::new(FORM, "Event", 8)
        .distinct("lane", 2)
        .case(Case::Interval(IntervalCase::AllDay, 2))
        .case(Case::Interval(IntervalCase::MultiDay(3), 1))
        .case(Case::Interval(IntervalCase::SameDay(4), 1))
        .case(Case::Interval(IntervalCase::CrossPeriod, 1))
        .invalid(1)
}

pub fn builder(seed: u64, wish: SetWish) -> SampleBuilder {
    SampleBuilder::new(SampleSpec {
        seed,
        anchor: anchor(),
        sets: vec![wish],
    })
    .relation(Relation::order("start", "end", Some("all-day")))
}

/// The golden file `tests/golden/<name>.txt`; `FS_UPDATE_GOLDEN=1` writes it (a deliberate act, e.g. after a `fake` update).
pub fn golden(name: &str, got: &str) {
    let path = format!("{}/tests/golden/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("FS_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, got).unwrap();
    }
    let want = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        want == got,
        "golden file {path} differs; renew it on purpose with FS_UPDATE_GOLDEN=1"
    );
}
