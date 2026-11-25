//! Probe of the consumer path of `--sample-data`: a real description file goes through `fetch`, `open` and a `Sources` whose
//! `connect_sample` derives the test data from what it is told (`Described`: the layout and the text of `[sample]`) with the real
//! catalog adapter (`CatalogBlockForms`). It proves the wiring: layout and `[sample]` ARRIVE and have an effect. The turn of the
//! module into `Records` is the next package.
//!
//! Pattern: Spy (the test `Sources` writes down what it derived from what it was given).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use fs_forms_testdata::{SampleModule, SampleSpec};

use crate::args::Config;
use crate::block_forms::CatalogBlockForms;
use crate::described::Described;
use crate::feed::Want;
use crate::records::{NoRecords, Records};
use crate::run::{Fetched, fetch};
use crate::sources::{Sources, open};
use crate::view::Coded;
use crate::words::words;

/// What the spy derived: (source, form, count of wished records) per source, rows really built for the first form, the seed.
#[derive(Default)]
struct Seen {
    asked: Option<Asked>,
}

#[derive(Debug)]
struct Asked {
    had_layout: bool,
    had_sample: bool,
    wished: Vec<(u32, String, u32)>,
    rows: usize,
    seed: u64,
    module_ok: bool,
}

#[derive(Default)]
struct Probe(Mutex<Seen>);

fn probe_fail(code: &str, why: impl std::fmt::Debug) -> Coded {
    Coded::new(code).with("why", format!("{why:?}"))
}

impl Sources for Probe {
    fn connect(&self, range: &str, _: Option<&Path>, _: &Path) -> Result<Arc<dyn Records>, Coded> {
        Err(Coded::new("host-notice-source-unknown").with("contract", range))
    }

    fn connect_sample(
        &self,
        _wants: &[Want],
        described: &Described<'_>,
        _: Option<&Path>,
        _: &Path,
    ) -> Result<Arc<dyn Records>, Coded> {
        let layout = described
            .layout()
            .ok_or_else(|| Coded::new("probe-no-layout"))?;
        let specs = SampleSpec::from_description(layout, &CatalogBlockForms, described.sample())
            .map_err(|e| probe_fail("probe-spec", e))?;
        let first = specs.first().ok_or_else(|| Coded::new("probe-no-source"))?;
        let set = first.build().map_err(|e| probe_fail("probe-build", e))?;
        let slice = set
            .slices
            .first()
            .ok_or_else(|| Coded::new("probe-no-set"))?;
        let rows = set
            .rows(&slice.form)
            .map_err(|e| probe_fail("probe-rows", e))?;
        let module_ok =
            SampleModule::from_description(layout, &CatalogBlockForms, described.sample(), 1)
                .is_ok();
        self.0.lock().expect("lock").asked = Some(Asked {
            had_layout: true,
            had_sample: described.sample().is_some(),
            wished: specs
                .iter()
                .flat_map(|s| {
                    s.spec
                        .sets
                        .iter()
                        .map(|w| (s.source, w.form.clone(), w.count))
                })
                .collect(),
            rows: rows.len(),
            seed: first.spec.seed,
            module_ok,
        });
        Ok(Arc::new(NoRecords))
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/probe")
        .join(name)
}

/// The whole consumer path for one file: `fetch`, then `open` with the flag.
fn through(path: PathBuf) -> Asked {
    let c = Config {
        description: Some(path),
        sample_data: true,
        ..Config::default()
    };
    let Fetched { view, layout, .. } = fetch(&c).expect("the description is read");
    let texts = words(&fs_components_words::Locale::english());
    let spy = Probe::default();
    let described = Described::of(layout.as_ref());
    let o = open(&c, &view, &described, &texts, Path::new("x"), &spy);
    assert!(o.notice.is_none(), "{:?}", o.notice);
    let got = spy.0.lock().expect("lock").asked.take();
    got.expect("connect_sample was asked")
}

#[test]
fn the_layout_and_the_sample_section_of_a_real_file_reach_the_sources_and_decide_the_data() {
    let a = through(fixture("board-sample.toml"));
    assert!(a.had_layout && a.had_sample && a.module_ok, "{a:?}");
    // the section says 5 records and seed 7: the default would be 8 records
    assert_eq!(
        a.wished,
        [(1, "cards-in-columns@^1.0.0".to_owned(), 5)],
        "{a:?}"
    );
    assert_eq!((a.rows, a.seed), (5, 7), "{a:?}");
}

#[test]
fn without_the_section_the_same_path_gives_the_defaults_of_the_declaration() {
    // counter-test: the same file with the section cut off; nothing else differs
    let text = std::fs::read_to_string(fixture("board-sample.toml")).expect("fixture");
    let cut = text
        .find("\n[sample]")
        .expect("the fixture has the section");
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("board.toml");
    std::fs::write(&path, &text[..cut]).expect("write");
    let a = through(path);
    assert!(a.had_layout && !a.had_sample && a.module_ok, "{a:?}");
    assert_eq!(a.wished[0].2, 8, "{a:?}");
    assert_eq!(a.rows, 8, "{a:?}");
}
