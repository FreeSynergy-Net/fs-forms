//! Every wish that cannot be met is a code, never a silently dropped wish.

mod common;

use common::{builder, month_wish};
use fs_forms_testdata::{
    Anchor, Case, IntervalCase, Period, Relation, SampleBuilder, SampleCode, SampleSpec, SetWish,
};
use fs_types::Zone;

fn code(result: Result<fs_forms_testdata::SampleSet, fs_forms_testdata::Failure>) -> SampleCode {
    result.expect_err("must fail").code()
}

#[test]
fn an_unknown_form_or_version_is_form_unknown() {
    assert_eq!(
        code(builder(1, SetWish::new("nope@^1.0.0", "Event", 1)).build()),
        SampleCode::FormUnknown
    );
    assert_eq!(
        code(builder(1, SetWish::new("bars-in-time@^9.0.0", "Event", 1)).build()),
        SampleCode::FormUnknown
    );
}

#[test]
fn all_day_without_a_flag_channel_is_impossible() {
    let spec = SampleSpec {
        seed: 1,
        forbid: Vec::new(),
        anchor: common::anchor(),
        sets: vec![month_wish()],
    };
    let result = SampleBuilder::new(spec)
        .relation(Relation::order("start", "end", None))
        .build();
    assert_eq!(code(result), SampleCode::CaseImpossible);
    let none = SampleBuilder::new(SampleSpec {
        seed: 1,
        forbid: Vec::new(),
        anchor: common::anchor(),
        sets: vec![month_wish()],
    })
    .build();
    assert_eq!(
        code(none),
        SampleCode::CaseImpossible,
        "interval cases need an Order relation"
    );
}

#[test]
fn too_many_cases_for_the_count() {
    let wish =
        SetWish::new(common::FORM, "Event", 3).case(Case::Interval(IntervalCase::SameDay(4), 1));
    assert_eq!(code(builder(1, wish).build()), SampleCode::CountTooSmall);
}

#[test]
fn a_relation_that_fits_no_form_is_reported() {
    let spec = SampleSpec {
        seed: 1,
        forbid: Vec::new(),
        anchor: common::anchor(),
        sets: vec![SetWish::new(common::FORM, "Event", 2)],
    };
    let result = SampleBuilder::new(spec)
        .relation(Relation::order("start", "end", Some("all-day")))
        .relation(Relation::order("nope", "end", None))
        .build();
    assert_eq!(code(result), SampleCode::RelationUnknown);
}

#[test]
fn bad_anchor_and_unknown_zone() {
    let wish = SetWish::new(common::FORM, "Event", 1);
    let bad = Anchor {
        period: Period::Month(2026, 13),
        zone: Zone::iana("Europe/Berlin").unwrap(),
    };
    let result = SampleBuilder::new(SampleSpec {
        seed: 1,
        forbid: Vec::new(),
        anchor: bad,
        sets: vec![wish.clone()],
    })
    .relation(Relation::order("start", "end", Some("all-day")))
    .build();
    assert_eq!(code(result), SampleCode::AnchorBad);
    let zone = Anchor {
        period: Period::Month(2026, 5),
        zone: Zone::iana("Mars/Olympus").unwrap(),
    };
    let result = SampleBuilder::new(SampleSpec {
        seed: 1,
        forbid: Vec::new(),
        anchor: zone,
        sets: vec![wish],
    })
    .relation(Relation::order("start", "end", Some("all-day")))
    .build();
    assert_eq!(code(result), SampleCode::ZoneUnknown);
}

#[test]
fn distinct_and_empty_need_fitting_channels() {
    let flag = SetWish::new(common::FORM, "Event", 2).distinct("all-day", 2);
    assert_eq!(code(builder(1, flag).build()), SampleCode::CaseImpossible);
    let required = SetWish::new(common::FORM, "Event", 2).empty("label", 1);
    assert_eq!(
        code(builder(1, required).build()),
        SampleCode::CaseImpossible
    );
}

#[test]
fn an_empty_list_is_empty() {
    let set = builder(1, SetWish::new(common::FORM, "Event", 5).case(Case::Empty))
        .build()
        .unwrap();
    assert_eq!(set.objects.len(), 0);
    assert_eq!(set.rows(common::KEY).unwrap().len(), 0);
}
