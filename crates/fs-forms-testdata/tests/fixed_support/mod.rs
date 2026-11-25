//! Helpers of the tests of the fixed events: the set of a window and the golden reader.

#![allow(dead_code)]

use fs_forms_testdata::{
    Anchor, Period, Relation, SampleBuilder, SampleRow, SampleSet, SampleSpec, SetWish,
};
use fs_types::Zone;

pub const FORM: &str = "bars-in-time@^1.1.0";
pub const KEY: &str = "bars-in-time@1.1.0";
pub const ZONE: &str = "Europe/Berlin";

pub fn anchor(period: Period) -> Anchor {
    Anchor {
        period,
        zone: Zone::iana(ZONE).unwrap(),
    }
}

pub fn wish() -> SetWish {
    SetWish::new(FORM, "Event", 0).fixed("events.sample")
}

pub fn builder(period: Period, wish: SetWish) -> SampleBuilder {
    SampleBuilder::new(SampleSpec {
        seed: 1,
        anchor: anchor(period),
        sets: vec![wish],
        forbid: Vec::new(),
    })
    .relation(Relation::order("start", "end", Some("all-day")))
}

pub fn set(period: Period) -> SampleSet {
    builder(period, wish()).build().expect("builds")
}

pub fn rows(period: Period) -> Vec<SampleRow> {
    set(period).rows(KEY).expect("rows")
}

/// The four windows the pictures use (March, October, November, December 2026).
pub fn standard_windows() -> [Period; 4] {
    [
        Period::Month(2026, 3),
        Period::Month(2026, 10),
        Period::Month(2026, 11),
        Period::Month(2026, 12),
    ]
}

/// The golden file `tests/golden/<name>.txt`; `FS_UPDATE_GOLDEN=1` writes it (a deliberate act, e.g. after a change of the sample).
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
