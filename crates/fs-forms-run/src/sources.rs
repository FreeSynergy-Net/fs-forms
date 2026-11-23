//! Which records a window takes: the contract the description names for its source is handed to the PORT [`Sources`], which the
//! host that runs `fs-forms-run` gives (the starter finds, checks and connects the module service). This crate holds no table of
//! modules and links no module client; a contract nobody serves is said, never ignored.
//!
//! Pattern: Port (`Sources`) with a null object (`NoSources`); a failure is a code (`host-notice-data-*`), not a panic.

use std::path::Path;
use std::sync::Arc;

use fs_components_mf2::Mf2Texts;

use crate::args::Config;
use crate::parts::line;
use crate::records::{NoRecords, Records, Unavailable};
use crate::sample::SampleBoard;
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
    /// The records of the module that serves `range` (the contract and its range as the description writes it: `tasks >=0.2, <0.3`;
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
}

/// A host that offers no data source: every contract is unknown.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSources;

impl Sources for NoSources {
    fn connect(&self, range: &str, _: Option<&Path>, _: &Path) -> Result<Arc<dyn Records>, Coded> {
        Err(Coded::new("host-notice-source-unknown").with("contract", name_of(range)))
    }
}

/// The name of the contract in a range (`tasks >=0.2, <0.3` is `tasks`).
#[must_use]
pub fn name_of(range: &str) -> &str {
    range.split_whitespace().next().unwrap_or("")
}

/// The records for `view`: the sample data when asked for (`--sample-data`), else what `sources` connects for the first source of
/// the description; a description without a source gets no records and no notice (nothing is asked for).
#[must_use]
pub fn open(
    c: &Config,
    view: &View,
    texts: &Mf2Texts,
    running: &Path,
    sources: &dyn Sources,
) -> Opened {
    if c.sample_data {
        return Opened {
            records: Arc::new(SampleBoard::default()),
            notice: None,
            connected: false,
        };
    }
    let Some(first) = view.sources.first() else {
        return Opened {
            records: Arc::new(NoRecords),
            notice: None,
            connected: false,
        };
    };
    match sources.connect(first, c.socket.as_deref(), running) {
        Ok(records) => Opened {
            records,
            notice: None,
            connected: true,
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
            sources: vec!["groups >=0.1, <0.2".into()],
            ..View::default()
        };
        let c = crate::args::Config::default();
        let o = open(&c, &view, &texts, Path::new("x"), &NoSources);
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
        let o = open(&c, &View::default(), &texts, Path::new("x"), &NoSources);
        assert!(o.notice.is_none() && !o.connected);
    }
}
