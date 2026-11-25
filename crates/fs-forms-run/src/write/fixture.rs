//! The fake module of the tests of the written way: its declaration, its data and a writer that records the calls.
//!
//! Pattern: none (test fixture).

use std::sync::{Arc, Mutex};

use fs_forms_bind::{Basic, FormRef, ModuleForm};

use super::{Invocation, Writer};
use crate::form_data::{FormData, FormRecords, FormRow};
use crate::view::Coded;

/// The declaration of a task module for `cards-in-columns@1.2.0` (`move` writes `after`/`before`, never `order`), as its `catalog/forms.json` says it.
pub(crate) fn declared() -> ModuleForm {
    let pair = |c: &str, f: &[&str]| (c.to_owned(), f.iter().map(|x| (*x).to_owned()).collect());
    ModuleForm {
        data_type: String::new(),
        form: FormRef {
            name: "cards-in-columns".into(),
            major: 1,
            minor: 2,
        },
        map: vec![
            pair("cards", &["Task"]),
            pair("column", &["column"]),
            pair("title", &["title"]),
            pair("lane", &["board"]),
            pair("order", &["index"]),
            pair("after", &["after_id"]),
            pair("before", &["before_id"]),
            pair("column-label", &["column_name"]),
            pair("progress", &["percent"]),
            pair("facts", &["due", "priority"]),
            pair("move", &["card.move"]),
            pair("create", &["task.create"]),
            pair("remove", &["task.delete"]),
        ],
        fields: vec![
            ("due".into(), Basic::DateTime),
            ("priority".into(), Basic::Text),
            ("column".into(), Basic::Text),
            ("title".into(), Basic::Text),
            ("board".into(), Basic::Text),
            ("index".into(), Basic::Number),
            ("after_id".into(), Basic::Reference),
            ("before_id".into(), Basic::Reference),
            ("column_name".into(), Basic::Text),
            ("percent".into(), Basic::Number),
        ],
    }
}

pub(crate) fn row(key: &str, column: &str, label: &str, may: &[&str]) -> FormRow {
    FormRow::of(
        key,
        &[
            ("title", key),
            ("column", column),
            ("column-label", label),
            ("lane", "b1"),
            ("order", "3"),
        ],
    )
    .allowing(may)
}

/// The data of the fake module: card `a` may be moved, card `b` may not, the page may create.
pub(crate) fn data() -> FormData {
    FormData {
        declared: declared(),
        rows: vec![
            row("a", "todo", "To do", &["card.move"]),
            row("b", "doing", "In progress", &[]),
        ],
        may: vec!["task.create".into()],
        commands: vec!["card.move".into(), "task.create".into()],
        columns: Vec::new(),
    }
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
