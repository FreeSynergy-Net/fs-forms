//! The memory of the sample module: its tables and the log of changes (what `apply` and `revert` work on).
//!
//! Pattern: Memento (an entry keeps the values before and after, so a change can be taken back) + Idempotent Receiver (the same
//! non-empty run and step act once). Nothing here leaves the process.

use std::collections::HashMap;

use fs_interfaces::module::{ChangeKind, ChangeRef, Origin};
use fs_types::Utc;

use crate::project::{Rec, Table, Values};

/// The base of the logical clock (2023-11-14, in ms): the module reads no clock, so the same calls give the same log.
const EPOCH_MS: i64 = 1_700_000_000_000;

/// One change in the log.
#[derive(Clone, Debug)]
pub struct Entry {
    /// The id (a UUID v7 as number: time, then a counter).
    pub id: u128,
    /// The table.
    pub table: usize,
    /// The record key.
    pub object: String,
    /// Create, Update or Undo.
    pub kind: ChangeKind,
    /// The revision after the change and before it (`None` at create).
    pub rev: u64,
    /// The revision before the change.
    pub before_rev: Option<u64>,
    /// The values before (`None` = the record did not exist) and after (`None` = it does not exist).
    pub before: Option<Values>,
    /// The values after.
    pub after: Option<Values>,
    /// Who asked.
    pub origin: Origin,
    /// The run of the step the change was made in.
    pub run: String,
    /// The change this one took back.
    pub undoes: Option<u128>,
    /// The time, logical (ms).
    pub at: i64,
}

impl Entry {
    /// The change as the contract gives it.
    #[must_use]
    pub fn change_ref(&self) -> ChangeRef {
        let c = ChangeRef::new(
            &uuid_text(self.id),
            &self.object,
            &self.rev.to_string(),
            &self.before_rev.map(|r| r.to_string()).unwrap_or_default(),
            self.origin.clone(),
            self.kind,
            Utc::from_millis(self.at),
        );
        match self.undoes {
            Some(u) => c.undoing(&uuid_text(u)),
            None => c,
        }
    }
}

/// The text of a UUID (8-4-4-4-12, lower case).
#[must_use]
pub fn uuid_text(id: u128) -> String {
    let h = format!("{id:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

/// The number behind a UUID text.
#[must_use]
pub fn uuid_number(text: &str) -> Option<u128> {
    let h: String = text.chars().filter(|c| *c != '-').collect();
    (h.len() == 32)
        .then(|| u128::from_str_radix(&h, 16).ok())
        .flatten()
}

/// The tables, the log and the steps already done.
#[derive(Debug)]
pub struct Store {
    /// One table per form.
    pub tables: Vec<Table>,
    /// The log, oldest first.
    pub log: Vec<Entry>,
    /// (run, step, is a revert) -> index in the log.
    pub done: HashMap<(String, String, bool), usize>,
}

impl Store {
    /// A store over `tables`, empty log.
    #[must_use]
    pub fn new(tables: Vec<Table>) -> Self {
        Self {
            tables,
            log: Vec::new(),
            done: HashMap::new(),
        }
    }

    /// The next id and time: a UUID v7 (version 7, variant 10) whose time part is the logical clock.
    #[must_use]
    pub fn next_id(&self) -> (u128, i64) {
        let n = self.log.len() as u64 + 1;
        let at = EPOCH_MS + i64::try_from(n).unwrap_or(0) * 1000;
        let ms = u128::try_from(at).unwrap_or(0);
        ((ms << 80) | (7 << 76) | (0b10 << 62) | u128::from(n), at)
    }

    /// The table and index of a living record.
    #[must_use]
    pub fn find(&self, record: &str) -> Option<(usize, usize)> {
        self.tables.iter().enumerate().find_map(|(t, table)| {
            let r = table.recs.iter().position(|r| r.alive && r.id == record)?;
            Some((t, r))
        })
    }

    /// The newest change of a record.
    #[must_use]
    pub fn latest(&self, object: &str) -> Option<&Entry> {
        self.log.iter().rev().find(|e| e.object == object)
    }

    /// Writes `entry` into its record (values, revision, life), then into the log, and remembers the step. Returns its index.
    pub fn record(&mut self, entry: Entry, step: (&str, &str), revert: bool) -> usize {
        let table = &mut self.tables[entry.table];
        let rec = table.recs.iter_mut().find(|r| r.id == entry.object);
        match (rec, &entry.after) {
            (Some(r), after) => {
                r.rev = entry.rev;
                r.alive = after.is_some();
                if let Some(v) = after {
                    r.values.clone_from(v);
                }
            }
            (None, Some(v)) => table.recs.push(Rec {
                id: entry.object.clone(),
                rev: entry.rev,
                values: v.clone(),
                alive: true,
            }),
            (None, None) => {}
        }
        self.log.push(entry);
        let at = self.log.len() - 1;
        if !step.0.is_empty() && !step.1.is_empty() {
            self.done
                .insert((step.0.to_owned(), step.1.to_owned(), revert), at);
        }
        at
    }
}
