//! The test data of the board tests: made from a wish (`[sample]`, `tests/fixtures/sample/board-parity.toml`) by `fs-forms-testdata`,
//! served by `SampleModule`, and given to the host as `Records` the way a connected module is (`FormRecords` + the port `Writer`).
//! A test names no card, column or text; it reads them from the data (`card`, `column`).
//!
//! The turn of a module into `Records` for the real host is the adapter of the starter (package 4b, fs-starter); this is the one the
//! tests of this crate use (the module lives on its own thread: `SampleModule` is not `Send`, the port `Writer` is).
//!
//! Pattern: Adapter (`SampleModule` -> `Writer` + `FormData`) behind the existing `FormRecords`; Fake Object inside.

#![allow(dead_code)] // each test file uses a part

mod derive;
#[allow(unused_imports)] // each test file uses a part
pub use derive::Derived;
mod groups;
#[allow(unused_imports)] // each test file uses a part
pub use groups::{groups, rows_of};

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};

use fs_components_kanban::Answer;
use fs_forms_run::{Coded, FormData, FormRecords, FormRow, Invocation, Records, Writer};
use fs_forms_testdata::{SampleModule, SourceSpec};
use fs_interfaces::module::{
    Actor, Caller, Claimed, Context, Invocation as Call, ModuleCore, Page, PageQuery, Value,
};
use fs_interfaces::owner::fake::person;

/// The form of the board (the name the gestures are on).
pub const FORM: &str = "cards-in-columns";
const WISH: &str = include_str!("../fixtures/sample/board-parity.toml");
const PERSON: &str = "urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01";

/// The source of the wish file (`SourceSpec::from_wish_text`: the same defaults a description gets).
fn wish() -> SourceSpec {
    SourceSpec::from_wish_text(WISH).expect("the wish reads")
}

enum Ask {
    Call(Call, Sender<Result<(), Coded>>),
    Read(Sender<Vec<FormData>>),
}

/// The port `Writer` of the module on its thread.
#[derive(Debug)]
struct ModuleWriter(Mutex<Sender<Ask>>);

impl ModuleWriter {
    fn ask<T>(&self, make: impl FnOnce(Sender<T>) -> Ask) -> T {
        let (tx, rx) = channel();
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .send(make(tx))
            .expect("the module thread is there");
        rx.recv().expect("the module thread answers")
    }
}

impl Writer for ModuleWriter {
    fn invoke(&self, call: &Invocation) -> Result<(), Coded> {
        let mut c = Call::default();
        c.command.clone_from(&call.command);
        c.record.clone_from(&call.record);
        c.args = call.args.iter().map(|(p, t)| Value::new(p, t)).collect();
        self.ask(|tx| Ask::Call(c, tx))
    }
    fn reload(&self) -> Result<Vec<FormData>, Coded> {
        Ok(self.ask(Ask::Read))
    }
}

fn context() -> Context {
    let caller = Caller::Connection {
        system_id: "1000".into(),
        system_name: None,
    };
    Context::from_connection(caller, Actor::Person, person(PERSON)).with_claimed(Claimed::new(
        "fs-forms-run",
        "i1",
        "d1",
    ))
}

/// The page of the module as the rows the host reads (the channels of the commands are bound by name: the module binds a command
/// as the channel it is on).
fn data_of(page: &Page, declared: &fs_forms_bind::ModuleForm) -> FormData {
    let mut declared = declared.clone();
    let mut commands: Vec<String> = page.may.clone();
    let rows: Vec<FormRow> = page
        .records
        .iter()
        .map(|r| {
            for m in &r.may {
                if !commands.contains(m) {
                    commands.push(m.clone());
                }
            }
            // The path of a value is a FIELD; the one adapter of the components reads by channel, each channel the values of the
            // fields the map names for it, and keeps the path of every value (a value of no declared field is never seen).
            FormRow::from_values(
                &r.id,
                r.may.clone(),
                &declared.map,
                r.values.iter().map(|v| (v.path.as_str(), v.text.as_str())),
            )
        })
        .collect();
    for c in &commands {
        declared.map.push((c.clone(), vec![c.clone()]));
    }
    let mut columns: Vec<(String, String)> = Vec::new();
    for row in &rows {
        if let Some(key) = row.get("column")
            && !columns.iter().any(|(k, _)| k == key)
        {
            columns.push((key.to_owned(), key.to_owned()));
        }
    }
    FormData {
        source: 0,
        data_type: declared.data_type.clone(),
        declared,
        rows,
        may: page.may.clone(),
        commands,
        columns,
    }
}

fn serve(source: &SourceSpec, rx: Receiver<Ask>) {
    let set = source.build().expect("the wish builds");
    let module = SampleModule::new(&set).expect("the module");
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime");
    let cx = context();
    let read = || -> Vec<FormData> {
        set.slices
            .iter()
            .zip(&set.declared)
            .map(|(slice, declared)| {
                let mut q = PageQuery::default();
                q.form.clone_from(&slice.form);
                q.limit = 500;
                let page = rt.block_on(module.page(&cx, q)).expect("page");
                data_of(&page, declared)
            })
            .collect()
    };
    for ask in rx {
        match ask {
            Ask::Read(back) => {
                let _ = back.send(read());
            }
            Ask::Call(call, back) => {
                let done = rt
                    .block_on(module.apply(&cx, call))
                    .map(|_| ())
                    .map_err(|r| Coded::new(r.detail));
                let _ = back.send(done);
            }
        }
    }
}

/// The records of a module made from `source`: its own module on its own thread, its own memory (a gesture changes nothing
/// else), every form of the source read the way a connected module is.
#[must_use]
pub fn records_of(source: SourceSpec) -> Arc<dyn Records> {
    let (tx, rx) = channel();
    std::thread::spawn(move || serve(&source, rx));
    let writer = Arc::new(ModuleWriter(Mutex::new(tx)));
    let forms = writer.ask(Ask::Read);
    Arc::new(FormRecords {
        provider: fs_forms_testdata::SAMPLE_PROVIDER.to_owned(),
        forms,
        writer: Some(writer),
        ..FormRecords::default()
    })
}

/// The records of a fresh board (the wish `board-parity.toml`).
#[must_use]
pub fn board() -> Arc<dyn Records> {
    records_of(wish())
}

/// The rows of the board now.
#[must_use]
pub fn rows(records: &dyn Records) -> Vec<FormRow> {
    let forms = records.forms();
    forms
        .iter()
        .find(|f| f.declared.form.name == FORM)
        .map(|f| f.rows.clone())
        .unwrap_or_default()
}

/// The key of the `n`-th card (the order of the data).
#[must_use]
pub fn card(n: usize) -> String {
    rows(&*board())[n].key.clone()
}

/// The columns in order of first appearance.
#[must_use]
pub fn columns(records: &dyn Records) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for row in rows(records) {
        if let Some(c) = row.get("column")
            && !out.iter().any(|o| o == c)
        {
            out.push(c.to_owned());
        }
    }
    out
}

/// The `n`-th column of a fresh board.
#[must_use]
pub fn column(n: usize) -> String {
    columns(&*board())[n].clone()
}

/// A column that is not the one `key` is in (on a fresh board).
#[must_use]
pub fn other_column(key: &str) -> String {
    let b = board();
    let own = rows(&*b)
        .iter()
        .find(|r| r.key == key)
        .and_then(|r| r.get("column").map(str::to_owned))
        .expect("the card is there");
    columns(&*b)
        .into_iter()
        .find(|c| *c != own)
        .expect("a second column")
}

fn pair(name: &str, value: &str) -> (String, String) {
    (name.to_owned(), value.to_owned())
}

/// The card `key` dropped into column `to` (the gesture on the channel `move`).
#[must_use]
pub fn move_card(records: &dyn Records, key: &str, to: &str) -> Answer {
    records.invoke(FORM, "move", Some(key), &[pair("column", to)])
}

/// A card titled `title` quick-added to `column` (the gesture on the channel `create`).
#[must_use]
pub fn add_card(records: &dyn Records, column: &str, title: &str) -> Answer {
    records.invoke(
        FORM,
        "create",
        None,
        &[pair("column", column), pair("title", title)],
    )
}

/// The `n`-th card of a fresh board dropped into a column it is not in (the same cards on every fresh board).
#[must_use]
pub fn move_nth(records: &dyn Records, n: usize) -> Answer {
    let key = card(n);
    move_card(records, &key, &other_column(&key))
}

/// The records of sample events (the wish `events-parity.toml`: `bars-in-time`, channels `label` and `lane`, fields `title` and
/// `freesynergy.net:lane`).
#[must_use]
pub fn events() -> Arc<dyn Records> {
    records_of(
        SourceSpec::from_wish_text(include_str!("../fixtures/sample/events-parity.toml"))
            .expect("the wish reads"),
    )
}
