//! The fake module of the tests of the written way: its data come from the generator of `fs-forms-testdata`, made from a wish file
//! (`tests/fixtures/sample/write-parity.toml`): two cards in two columns of one lane. A test names no key, column or lane: it reads
//! them from the data (`first`, `second`, `column_of`, `other_column`, `lane_of`). What is the test's own input stays here: the keys
//! of the contract the module binds its commands to, and the rights (the card with a place may be moved, the other may not). The
//! writer is a spy: it records the calls and answers what it was told.
//!
//! Pattern: Test Data Builder (the generator) + Adapter (`SampleSet` -> `FormData`) + Spy (`FakeWriter`), Object Mother for the rights.

use std::sync::{Arc, Mutex};

use fs_forms_bind::ModuleForm;
use fs_forms_testdata::{SampleSet, SourceSpec};

use super::{Invocation, Writer};
use crate::form_data::{FormData, FormRecords, FormRow};
use crate::view::Coded;

/// The wish: what the generator makes (data), not what the test may do (rights).
const WISH: &str = include_str!("../../tests/fixtures/sample/write-parity.toml");

/// The key of the contract the module binds the channel `move` to.
pub(crate) const MOVE: &str = "card.move";
/// The key of the contract the module binds the channel `create` to.
pub(crate) const CREATE: &str = "task.create";
/// The key of the contract the module binds the channel `remove` to.
pub(crate) const REMOVE: &str = "task.delete";

/// The set the wish makes (the same on every call: the seed is in the wish).
pub(crate) fn set() -> SampleSet {
    SourceSpec::from_wish_text(WISH)
        .expect("the wish reads")
        .build()
        .expect("the wish builds")
}

/// The declaration of the module: the generated map plus the binding of its commands to the keys of its contract.
pub(crate) fn declared() -> ModuleForm {
    let mut declared = set().declared.remove(0);
    for (channel, key) in [("move", MOVE), ("create", CREATE), ("remove", REMOVE)] {
        declared
            .map
            .push((channel.to_owned(), vec![key.to_owned()]));
    }
    declared
}

/// The data of the fake module: the card with a place may be moved, the other may not, the page may create.
pub(crate) fn data() -> FormData {
    let set = set();
    let slice = &set.slices[0];
    let mut rows: Vec<FormRow> = set
        .rows(&slice.form)
        .expect("the form is in the set")
        .into_iter()
        .map(|r| FormRow {
            key: r.key,
            values: r.values,
            may: Vec::new(),
        })
        .collect();
    let at = movable_at(&rows);
    rows[at].may.push(MOVE.to_owned());
    FormData {
        source: 0,
        data_type: String::new(),
        declared: declared(),
        rows,
        may: vec![CREATE.to_owned()],
        commands: vec![MOVE.to_owned(), CREATE.to_owned()],
        columns: Vec::new(),
    }
}

/// The card that may be moved is the one with a place of its own (`order`): the tests ask what a drop does with an old place.
fn movable_at(rows: &[FormRow]) -> usize {
    rows.iter()
        .position(|r| r.get("order").is_some())
        .expect("the wish makes a card with a place")
}

/// The key of the card that may be moved (it has a place of its own).
pub(crate) fn first() -> String {
    let d = data();
    d.rows[movable_at(&d.rows)].key.clone()
}

/// The key of the card that may not be moved.
pub(crate) fn second() -> String {
    let d = data();
    d.rows[1 - movable_at(&d.rows)].key.clone()
}

/// The value of `channel` on the record `key`.
pub(crate) fn value_of(d: &FormData, key: &str, channel: &str) -> String {
    d.rows
        .iter()
        .find(|r| r.key == key)
        .and_then(|r| r.get(channel))
        .expect("the record has the channel")
        .to_owned()
}

/// The column the card `key` is in.
pub(crate) fn column_of(d: &FormData, key: &str) -> String {
    value_of(d, key, "column")
}

/// The lane the card `key` is in (the context a drop carries).
pub(crate) fn lane_of(d: &FormData, key: &str) -> String {
    value_of(d, key, "lane")
}

/// A column that is not the one the card `key` is in.
pub(crate) fn other_column(d: &FormData, key: &str) -> String {
    let own = column_of(d, key);
    d.rows
        .iter()
        .filter_map(|r| r.get("column"))
        .find(|c| *c != own)
        .expect("the wish makes two columns")
        .to_owned()
}

/// The field of the module that holds `channel` (what a call carries as the name of its argument).
pub(crate) fn field(d: &FormData, channel: &str) -> String {
    d.declared
        .map
        .iter()
        .find(|(c, _)| c == channel)
        .and_then(|(_, f)| f.first())
        .expect("the channel is mapped")
        .clone()
}

/// The row `of` moved into `column` (its rights stay).
pub(crate) fn moved(of: &FormRow, column: &str) -> FormRow {
    let mut row = of.clone();
    if let Some((_, v)) = row.values.iter_mut().find(|(c, _)| c == "column") {
        *v = vec![column.to_owned()];
    }
    row
}

/// A module that records the calls and answers what it was told.
#[derive(Debug, Default)]
pub(crate) struct FakeWriter {
    pub calls: Mutex<Vec<Invocation>>,
    pub refuse: Mutex<Option<String>>,
    pub reloads: Mutex<usize>,
    pub after: Mutex<Vec<FormData>>,
    pub reload_fails: Mutex<bool>,
}

impl Writer for FakeWriter {
    fn invoke(&self, call: &Invocation) -> Result<(), Coded> {
        self.calls.lock().unwrap().push(call.clone());
        match self.refuse.lock().unwrap().clone() {
            Some(code) => Err(Coded::new(code)),
            None => Ok(()),
        }
    }
    fn reload(&self) -> Result<Vec<FormData>, Coded> {
        *self.reloads.lock().unwrap() += 1;
        if *self.reload_fails.lock().unwrap() {
            return Err(Coded::new("host-notice-data-unreachable"));
        }
        let after = self.after.lock().unwrap().clone();
        Ok(if after.is_empty() {
            vec![data()]
        } else {
            after
        })
    }
}

pub(crate) fn records(writer: &Arc<FakeWriter>, d: FormData) -> FormRecords {
    FormRecords {
        provider: "tasks".into(),
        forms: vec![d],
        writer: Some(writer.clone()),
        ..FormRecords::default()
    }
}
