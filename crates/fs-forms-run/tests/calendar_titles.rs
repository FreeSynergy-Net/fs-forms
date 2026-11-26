//! The calendar descriptions (month, week, agenda, timeline) give the host rows whose column `label` holds the TITLE of the event,
//! never the key (`urn:fs-testdata:...`): the host reads the rows by the field the module declares, the same way for every frontend.
//!
//! Pattern: Characterization test (the facts of the fixed events, stated once) of an Adapter; the source is the `[sample]` section of
//! the shipped description itself, so a description that breaks its data breaks here.

mod sample_source;

use fs_forms_testdata::SourceSpec;
use sample_source::records_of;

/// The shipped descriptions and their `[sample]` section, as text.
const FILES: [(&str, &str); 4] = [
    (
        "C-F1",
        include_str!("../../../forms/calendar/C-F1-monat.toml"),
    ),
    (
        "C-F2",
        include_str!("../../../forms/calendar/C-F2-woche.toml"),
    ),
    (
        "C-F3",
        include_str!("../../../forms/calendar/C-F3-agenda.toml"),
    ),
    (
        "C-F4",
        include_str!("../../../forms/calendar/C-F4-zeitleiste.toml"),
    ),
];

#[test]
fn every_calendar_description_shows_titles_not_keys_in_the_label_column() {
    for (name, text) in FILES {
        let records = records_of(SourceSpec::from_wish_text(text).expect("the wish reads"));
        let forms = records.forms();
        let found = &forms.first().expect("a form of events").rows;
        assert!(!found.is_empty(), "{name}: the events are there");
        for row in found {
            let title = row.get("label").unwrap_or_default();
            assert!(!title.is_empty(), "{name} {}: the title is empty", row.key);
            assert!(
                !title.starts_with("urn:"),
                "{name} {}: the title is the key ({title})",
                row.key
            );
            assert_ne!(title, row.key, "{name}: the title is the key");
        }
        let titles: Vec<&str> = found.iter().filter_map(|r| r.get("label")).collect();
        assert!(titles.contains(&"Teamrunde"), "{name}: {titles:?}");
    }
}
