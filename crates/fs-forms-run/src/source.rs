//! Which records a window takes: the contract the description names for its source picks the data port. One small table (contract
//! name to opener), nothing else is known here; a contract the table does not have is said, never ignored.
//!
//! The openers LINK the client crates of the modules (transition for K2/K3, ADR 0104 replaces it by loading modules at run time).
//!
//! Pattern: Registry (table of openers) behind the port `Records`; a failure is a code (`host-notice-data-*`), not a panic.

use std::path::Path;
use std::sync::Arc;

use fs_components_mf2::Mf2Texts;

use crate::args::Config;
use crate::parts::line;
use crate::planning_records::PlanningRecords;
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

/// How a contract name becomes records: the socket of the program (or the default one) and the running program in, records out.
type Opener = fn(Option<&Path>, &Path) -> Result<Arc<dyn Records>, Coded>;

/// The table: the name of the contract (`tasks` of `tasks >=0.2 <0.3`) and the module that serves it.
const SOURCES: &[(&str, Opener)] = &[("tasks", planning)];

fn planning(socket: Option<&Path>, running: &Path) -> Result<Arc<dyn Records>, Coded> {
    let socket = socket
        .map(Path::to_path_buf)
        .or_else(fs_modules_planning_client::default_socket)
        .ok_or_else(|| Coded::new("host-notice-data-no-socket"))?;
    PlanningRecords::connect(&socket, running)
        .map(|r| Arc::new(r) as Arc<dyn Records>)
        .map_err(|e| {
            Coded::new("host-notice-data-unreachable")
                .with("contract", "tasks")
                .with("cause", e.to_string())
        })
}

/// The name of the contract in a range (`tasks >=0.2 <0.3` is `tasks`).
fn name_of(range: &str) -> &str {
    range.split_whitespace().next().unwrap_or("")
}

/// The records for `view`: the sample board when asked for (`--sample-data`), else the module of the first source the table knows;
/// a description without a source gets no records and no notice (nothing is asked for).
#[must_use]
pub fn open(c: &Config, view: &View, texts: &Mf2Texts, running: &Path) -> Opened {
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
    let found = view
        .sources
        .iter()
        .find_map(|s| SOURCES.iter().find(|(n, _)| *n == name_of(s)));
    let failed = |code: Coded| Opened {
        records: Arc::new(Unavailable(line(texts, &code))),
        notice: Some(code),
        connected: false,
    };
    match found {
        None => failed(Coded::new("host-notice-source-unknown").with("contract", name_of(first))),
        Some((_, open)) => match open(c.socket.as_deref(), running) {
            Ok(records) => Opened {
                records,
                notice: None,
                connected: true,
            },
            Err(code) => failed(code),
        },
    }
}
