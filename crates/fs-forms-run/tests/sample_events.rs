//! The host reads sample events by channel, the values it finds under the FIELDS the module declares: the title of an event is its
//! title (not the key), the lane is there. The adapter of `tests/sample_source` reads the way `fs-starter-host` does.
//!
//! Pattern: Characterization test (the facts of the generated events, stated once) of an Adapter.

mod sample_source;

use sample_source::events;

#[test]
fn an_event_shows_its_title_and_its_lane_by_channel() {
    let records = events();
    let forms = records.forms();
    let form = forms.first().expect("a form of events");
    assert_eq!(form.declared.form.name, "bars-in-time");
    assert!(!form.rows.is_empty(), "the events are there");
    for row in &form.rows {
        let title = row.get("label").unwrap_or_default();
        assert!(!title.is_empty(), "{}: the title is empty", row.key);
        assert_ne!(title, row.key, "the title is not the key");
        assert!(
            row.get("lane").is_some_and(|l| !l.is_empty()),
            "{}: the lane is missing",
            row.key
        );
    }
}
