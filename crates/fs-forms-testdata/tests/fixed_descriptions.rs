//! The four calendar descriptions (month, week, agenda, timeline) ask for the fixed events: read with the exchange reader, built like the host does.

mod described_support;

use described_support::{Table, read, specs};
use fs_forms_testdata::SampleSpec;

const KEY: &str = "bars-in-time@1.1.0";

fn layout_of(name: &str) -> (fs_forms_layout::Layout, String) {
    let path = format!("{}/../../forms/calendar/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    (read(&text), text)
}

#[test]
fn month_week_agenda_and_timeline_show_the_fixed_events_of_their_window() {
    // (file, the kind of anchor it writes)
    for (file, anchor) in [
        ("C-F1-monat.toml", "month"),
        ("C-F2-woche.toml", "week"),
        ("C-F3-agenda.toml", "month"),
        ("C-F4-zeitleiste.toml", "month"),
    ] {
        let (layout, text) = layout_of(file);
        assert!(text.contains(anchor), "{file}");
        let got = specs(&layout, Some(&text));
        assert_eq!(got.len(), 1, "{file}: one source");
        let wish = &got[0].spec.sets[0];
        assert_eq!(wish.fixed.as_deref(), Some("events.sample"), "{file}");
        assert_eq!(wish.data_type, "Event", "{file}");
        let set = got[0].build().unwrap_or_else(|e| panic!("{file}: {e:?}"));
        let rows = set.rows(KEY).unwrap_or_else(|e| panic!("{file}: {e:?}"));
        assert!(!rows.is_empty(), "{file}: the window shows events");
        assert!(
            set.now.is_some(),
            "{file}: the fixed now comes with the data"
        );
    }
}

#[test]
fn the_week_of_the_picture_holds_the_events_of_16_to_22_november() {
    let (layout, text) = layout_of("C-F2-woche.toml");
    let set = SampleSpec::from_description(&layout, &Table, Some(&text)).expect("spec")[0]
        .build()
        .expect("builds");
    let rows = set.rows(KEY).expect("rows");
    let titles: Vec<&str> = rows.iter().filter_map(|r| r.get("label")).collect();
    for title in ["Teamrunde", "Zahnarzt", "Call New York", "Buß- und Bettag"] {
        assert!(titles.contains(&title), "{title} is missing: {titles:?}");
    }
}
