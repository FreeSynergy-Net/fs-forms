//! The "now" of a view of sample data: the whole road from the words of the command line through `open` to the context the blocks
//! are drawn with. The data are the fixed events of the sample (their own "now" is 2026-11-16 in Berlin); the window of the test
//! reads the time ONLY through the port, as every window does.
//!
//! Pattern: Spy-free example tests over the port `Clock` (the same `Sources` a host gives, nothing special for tests in the crate).

use std::path::Path;
use std::sync::Arc;

use fs_components_words::Locale;
use fs_forms_testdata::{Anchor, Period, Relation, SampleBuilder, SampleSpec, SetWish};
use fs_types::Zone;

use crate::after_write::AfterWrite;
use crate::args::{Command, parse};
use crate::clock::{Clock, FixedClock, Moment, SystemClock};
use crate::described::Described;
use crate::feed::Want;
use crate::form_data::FormRecords;
use crate::records::Records;
use crate::sources::{Sources, open};
use crate::style::StyleCell;
use crate::view::{Coded, View};
use crate::window::ctx_of;
use crate::words::words;

/// What the starter's adapter does with a sample set: the clock of the records is the "now" of the set, in UTC.
struct Events;

impl Sources for Events {
    fn connect(&self, range: &str, _: Option<&Path>, _: &Path) -> Result<Arc<dyn Records>, Coded> {
        Err(Coded::new("host-notice-source-unknown").with("contract", range))
    }

    fn connect_sample(
        &self,
        _: &[Want],
        _: &Described<'_>,
        _: Option<&Path>,
        _: &Path,
    ) -> Result<Arc<dyn Records>, Coded> {
        let zone = Zone::iana("Europe/Berlin").expect("zone");
        let spec = SampleSpec {
            seed: 1,
            anchor: Anchor {
                period: Period::Week(2026, 47),
                zone: zone.clone(),
            },
            sets: vec![SetWish::new("bars-in-time@^1.1.0", "Event", 0).fixed("events.sample")],
            forbid: Vec::new(),
        };
        let set = SampleBuilder::new(spec)
            .relation(Relation::order("start", "end", Some("all-day")))
            .build()
            .expect("the fixed events build");
        let now = set
            .now_utc(&zone)
            .expect("zone")
            .expect("a fixed set has a now");
        Ok(Arc::new(FormRecords {
            provider: "fake".to_owned(),
            clock: Some(Arc::new(FixedClock::new(now))),
            ..FormRecords::default()
        }))
    }
}

fn moment(text: &str) -> Moment {
    text.parse().expect("rfc3339")
}

/// The "now" the context of a window shows for these words of the command line.
fn now_of(words_text: &str) -> Moment {
    let mut all = vec!["--description".to_owned(), "x.toml".to_owned()];
    all.extend(words_text.split_whitespace().map(str::to_owned));
    let Ok(Command::Window(c)) = parse(&all) else {
        panic!("not a window: {words_text}");
    };
    let view = View::default();
    let texts = words(&Locale::english());
    let opened = open(
        &c,
        &view,
        &Described::none(),
        &texts,
        Path::new("x"),
        &Events,
    );
    let ctx = ctx_of(
        &texts,
        &Locale::english(),
        &view,
        opened.records,
        Arc::new(StyleCell::default()),
        (AfterWrite::default(), opened.clock),
    );
    ctx.clock.now()
}

#[test]
fn sample_data_without_the_switch_show_the_now_of_the_sample_set() {
    // 09:00 in Berlin on 16 November 2026 (winter time, +01:00)
    assert_eq!(now_of("--sample-data"), moment("2026-11-16T08:00:00Z"));
}

#[test]
fn the_switch_gives_the_view_another_now_than_the_sample() {
    let want = moment("2026-11-25T09:00:00Z");
    assert_eq!(
        now_of("--sample-data --sample-now 2026-11-25T09:00:00Z"),
        want
    );
    assert_ne!(
        now_of("--sample-data"),
        want,
        "the switch is the difference"
    );
}

#[test]
fn an_offset_of_the_switch_is_an_instant_like_any_other() {
    assert_eq!(
        now_of("--sample-data --sample-now 2026-11-25T10:00:00+01:00"),
        moment("2026-11-25T09:00:00Z")
    );
}

#[test]
fn without_sample_data_the_view_reads_the_system_clock() {
    let got = now_of("");
    assert!(got > moment("2026-01-01T00:00:00Z"), "{got}");
    assert!(
        got <= SystemClock.now(),
        "the system clock does not run backwards"
    );
}
