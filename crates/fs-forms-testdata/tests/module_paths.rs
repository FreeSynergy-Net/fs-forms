//! The paths of the values of `SampleModule` are the FIELDS the module declares (`ModuleForm::map`), never the channel names:
//! a host reads by field, a real module answers by field. The names of channel and field differ here (`label` -> `title`).
//!
//! Pattern: Contract test (the declaration is the oracle: what is read, written and shown is checked against the same map).

mod module_support;

use fs_forms_testdata::SampleSet;
use fs_interfaces::module::ModuleCore;
use module_support::{
    BARS, LANE, TITLE, bars_with_writes, calendar, call, cx, fixed_events, module, page, refused,
    run, text,
};

fn declared_fields(set: &SampleSet) -> Vec<String> {
    set.declared
        .iter()
        .flat_map(|d| d.map.iter().flat_map(|(_, f)| f.clone()))
        .collect()
}

fn every_path_is_a_declared_field(set: &SampleSet) {
    let fields = declared_fields(set);
    assert!(
        fields.iter().any(|f| f == TITLE),
        "the fixture names a field apart from its channel"
    );
    let p = page(&module(set).declare(&bars_with_writes()), BARS);
    assert_ne!(p.total, Some(0), "the page has records");
    for r in &p.records {
        for v in &r.values {
            assert!(
                fields.contains(&v.path),
                "{}: path {:?} is no declared field {fields:?}",
                r.id,
                v.path
            );
        }
    }
    assert!(
        p.records
            .iter()
            .all(|r| r.values.iter().any(|v| v.path == TITLE)),
        "every record shows its title under the field `title`"
    );
}

#[test]
fn every_path_of_every_value_of_the_calendar_set_is_a_declared_field() {
    every_path_is_a_declared_field(&calendar(false, 0, Vec::new()));
}

#[test]
fn every_path_of_every_value_of_the_fixed_events_is_a_declared_field() {
    every_path_is_a_declared_field(&fixed_events(Vec::new()));
}

#[test]
fn a_write_by_field_path_round_trips_and_a_channel_name_is_no_path() {
    let m = module(&calendar(false, 0, Vec::new())).declare(&bars_with_writes());
    let key = page(&m, BARS).records[0].id.clone();
    let when = [
        ("start", "2026-05-12T10:00:00+02:00"),
        ("end", "2026-05-12T11:00:00+02:00"),
    ];
    let args = [when[0], when[1], (LANE, "moved")];
    run(m.apply(&cx("p"), call("move", Some(&key), &args))).expect("move by field");
    assert_eq!(text(&page(&m, BARS), &key, LANE), Some("moved"));
    assert_eq!(text(&page(&m, BARS), &key, "lane"), None);
    let args = [when[0], when[1], ("lane", "x")];
    let r = refused(run(m.apply(&cx("p"), call("move", Some(&key), &args))));
    assert_eq!(r.kind, "bad-arguments", "the channel name is not a field");
    let made = [when[0], when[1], (TITLE, "Made")];
    let made = run(m.apply(&cx("p"), call("create", None, &made))).expect("create by field");
    assert_eq!(text(&page(&m, BARS), &made.record, TITLE), Some("Made"));
    assert_eq!(text(&page(&m, BARS), &made.record, "label"), None);
}
