//! Which records a window takes: the contract the description names for its source is handed to the PORT [`Sources`], which the
//! host that runs `fs-forms-run` gives (the starter finds, checks and connects the module service). This crate holds no table of
//! modules and links no module client; a contract nobody serves is said, never ignored.
//!
//! Pattern: Port (`Sources`) with a null object (`NoSources`); a failure is a code (`host-notice-data-*`), not a panic.

use std::path::Path;
use std::sync::Arc;

use fs_components_mf2::Mf2Texts;

use crate::args::Config;
use crate::described::Described;
use crate::feed::Want;
use crate::parts::line;
use crate::records::{NoRecords, Records, Unavailable};
use crate::view::{Coded, View};

/// What the window draws from, and what it must say about it.
pub struct Opened {
    /// The records.
    pub records: Arc<dyn Records>,
    /// A notice for the whole window (a data source that is not there, or not known); `None` when all is as described.
    pub notice: Option<Coded>,
    /// Whether real records are connected (the notice "data sources are not connected" then does not hold).
    pub connected: bool,
}

/// The data sources a host can offer: it connects the module service that speaks a contract.
pub trait Sources: Send + Sync {
    /// The records of the module that serves `range` (the contract and its range as the description writes it: `tasks@>=0.2, <0.3`;
    /// the host compares the version of the module with it), or the code that says why there are none. `socket` is the one the
    /// command line gave (a host may ignore it), `running` the running program.
    ///
    /// # Errors
    /// A code (`host-notice-data-*`, `host-notice-source-unknown`) for a service that is not known, not there or does not fit.
    fn connect(
        &self,
        range: &str,
        socket: Option<&Path>,
        running: &Path,
    ) -> Result<Arc<dyn Records>, Coded>;

    /// The records for ALL sources of the description: one [`Want`] per `[[source]]` with the range, the data type and the scope the
    /// description gives. The host reads each source with its own scope (`page(form, scope)`), chooses the `forms.json` entry of the
    /// module by the TYPE, and numbers the [`crate::FormData`] it returns (`source`, `data_type`); a source it cannot read is a code in
    /// the `problems` of the records, not a failure of the whole window. Default: the first source only, as before.
    ///
    /// # Errors
    /// As [`Sources::connect`], for the first source (without it there is nothing to draw).
    fn connect_wanted(
        &self,
        wants: &[Want],
        socket: Option<&Path>,
        running: &Path,
    ) -> Result<Arc<dyn Records>, Coded> {
        match wants.first() {
            Some(first) => self.connect(&first.range, socket, running),
            None => Err(Coded::new("host-notice-source-unknown")),
        }
    }
    /// The sample records for `--sample-data`: the SAME question as [`Sources::connect_wanted`] (the host decides what sample means, not
    /// the caller). Default: none (`host-notice-source-unknown`, the contract is `sample`); the host derives the test data from the
    /// description (`fs-forms-testdata`, the section `[sample]`), as the starter does. This crate holds no sample data of its own.
    ///
    /// # Errors
    /// As [`Sources::connect_wanted`].
    fn connect_sample(
        &self,
        _wants: &[Want],
        _described: &Described<'_>,
        _socket: Option<&Path>,
        _running: &Path,
    ) -> Result<Arc<dyn Records>, Coded> {
        Err(Coded::new("host-notice-source-unknown").with("contract", "sample"))
    }
}

/// A host that offers no data source: every contract is unknown.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSources;

impl Sources for NoSources {
    fn connect(&self, range: &str, _: Option<&Path>, _: &Path) -> Result<Arc<dyn Records>, Coded> {
        Err(Coded::new("host-notice-source-unknown").with("contract", name_of(range)))
    }
}

/// The name of the contract in a range (`tasks@>=0.2, <0.3` is `tasks`).
#[must_use]
pub fn name_of(range: &str) -> &str {
    fs_forms_layout::split_range(range).0
}

/// The records for `view`: what `sources` connects for the sources of the description (`--sample-data` asks the SAME port
/// ([`Sources::connect_sample`]), the flag is read here once); without the flag, a description without a source gets no records and no notice (nothing is asked for).
#[must_use]
pub fn open(
    c: &Config,
    view: &View,
    described: &Described<'_>,
    texts: &Mf2Texts,
    running: &Path,
    sources: &dyn Sources,
) -> Opened {
    let wants = view.wants_or_ranges();
    let sample = c.sample_data;
    if !sample && view.sources.is_empty() {
        return Opened {
            records: Arc::new(NoRecords),
            notice: None,
            connected: false,
        };
    }
    let asked = if sample {
        sources.connect_sample(&wants, described, c.socket.as_deref(), running)
    } else {
        sources.connect_wanted(&wants, c.socket.as_deref(), running)
    };
    match asked {
        Ok(records) => Opened {
            records,
            notice: None,
            connected: !sample,
        },
        Err(code) => Opened {
            records: Arc::new(Unavailable(line(texts, &code))),
            notice: Some(code),
            connected: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words::words;

    #[test]
    fn a_host_without_sources_says_the_contract_is_unknown_and_shows_it_as_a_problem() {
        let texts = words(&fs_components_words::Locale::english());
        let view = View {
            sources: vec!["groups@>=0.1, <0.2".into()],
            ..View::default()
        };
        let c = crate::args::Config::default();
        let o = open(
            &c,
            &view,
            &Described::none(),
            &texts,
            Path::new("x"),
            &NoSources,
        );
        assert!(!o.connected);
        let code = o.notice.expect("a notice");
        assert_eq!(code.code, "host-notice-source-unknown");
        assert_eq!(
            code.params,
            vec![("contract".to_owned(), "groups".to_owned())]
        );
        assert!(o.records.unavailable().is_some());
    }

    #[test]
    fn no_source_in_the_description_asks_for_nothing() {
        let texts = words(&fs_components_words::Locale::english());
        let c = crate::args::Config::default();
        let o = open(
            &c,
            &View::default(),
            &Described::none(),
            &texts,
            Path::new("x"),
            &NoSources,
        );
        assert!(o.notice.is_none() && !o.connected);
    }
}

#[cfg(test)]
mod sample_tests {
    use std::sync::Mutex;

    use super::*;
    use crate::words::words;

    /// Spy: says which question the host asked.
    #[derive(Default)]
    struct Spy(Mutex<Vec<&'static str>>);

    impl Sources for Spy {
        fn connect(&self, _: &str, _: Option<&Path>, _: &Path) -> Result<Arc<dyn Records>, Coded> {
            self.0.lock().expect("lock").push("connect");
            Ok(Arc::new(NoRecords))
        }

        fn connect_sample(
            &self,
            wants: &[Want],
            _: &Described<'_>,
            _: Option<&Path>,
            _: &Path,
        ) -> Result<Arc<dyn Records>, Coded> {
            let n = wants.len();
            self.0
                .lock()
                .expect("lock")
                .push(if n == 1 { "sample-1" } else { "sample-n" });
            Ok(Arc::new(NoRecords))
        }
    }

    #[test]
    fn with_sample_data_the_sources_are_asked_for_sample_and_never_for_real_data() {
        let texts = words(&fs_components_words::Locale::english());
        let view = View {
            sources: vec!["groups@>=0.1, <0.2".into()],
            ..View::default()
        };
        let c = Config {
            sample_data: true,
            ..Config::default()
        };
        let spy = Spy::default();
        let o = open(&c, &view, &Described::none(), &texts, Path::new("x"), &spy);
        assert_eq!(*spy.0.lock().expect("lock"), vec!["sample-1"]);
        assert!(o.notice.is_none() && !o.connected);
    }

    #[test]
    fn a_host_without_sample_data_says_so_and_the_crate_makes_none_up() {
        let texts = words(&fs_components_words::Locale::english());
        let c = Config {
            sample_data: true,
            ..Config::default()
        };
        let o = open(
            &c,
            &View::default(),
            &Described::none(),
            &texts,
            Path::new("x"),
            &NoSources,
        );
        assert!(!o.records.is_sample() && !o.connected);
        let notice = o.notice.expect("a visible notice, not an empty window");
        assert_eq!(notice.code, "host-notice-source-unknown");
    }

    #[test]
    fn without_the_flag_the_real_question_is_asked() {
        let texts = words(&fs_components_words::Locale::english());
        let view = View {
            sources: vec!["groups@>=0.1, <0.2".into()],
            ..View::default()
        };
        let spy = Spy::default();
        let o = open(
            &Config::default(),
            &view,
            &Described::none(),
            &texts,
            Path::new("x"),
            &spy,
        );
        assert_eq!(*spy.0.lock().expect("lock"), vec!["connect"]);
        assert!(o.connected);
    }
}
