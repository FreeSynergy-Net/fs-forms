//! Rename and limit of a column: asked by the input dialog, sent to the records, a refusal keeps the dialog open and says why.
//!
//! Tests: the dialog starts with the current name / limit and what is confirmed reaches the records (`0` = no limit); a line that
//! is no number never reaches them and the dialog says so; a refusal (a conflict of the revision) shows its sentence and the dialog
//! stays until a try works; without the capability of the records or without the action of the description the control stays
//! locked and no dialog opens. Mutants (run by hand, red): `supports` without `records.can` (the lock test); `parse_limit` taking
//! `abc` as 0 (the invalid-line test); a refusal that closes the dialog (the conflict test).
//!
//! Pattern: none (tests; a recording stand-in for the records).

use std::sync::{Arc, Mutex, PoisonError};

use fs_components_fields::Look;
use fs_components_fields::Say;
use fs_components_kanban::{Answer, BoardSource, Column, KanbanBoard, Operation};
use fs_components_styles::StyleId;
use fs_components_widgets::dialogs::Labels;

use super::{Ask, Wiring};
use crate::records::{Records, SideItem};
use crate::view::ActionView;

/// Records that note what they are asked and can be told to refuse.
struct Edits {
    can: Vec<Operation>,
    refuse: Mutex<Option<String>>,
    calls: Mutex<Vec<String>>,
}

impl Edits {
    fn new(can: &[Operation]) -> Arc<Self> {
        Arc::new(Self {
            can: can.to_vec(),
            refuse: Mutex::new(None),
            calls: Mutex::new(Vec::new()),
        })
    }
    fn refuse_with(&self, code: Option<&str>) {
        *self.refuse.lock().unwrap_or_else(PoisonError::into_inner) = code.map(str::to_owned);
    }
    fn calls(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
    fn answer(&self, call: String) -> Answer {
        let refuse = self
            .refuse
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(code) = refuse {
            return Answer::Refused(code);
        }
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
        Answer::Accepted
    }
}

impl Records for Edits {
    fn columns(&self) -> Vec<Column> {
        let mut todo = Column::new("todo", "To do");
        todo.limit = Some(4);
        vec![todo, Column::new("done", "Done")]
    }
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn can(&self, op: Operation) -> bool {
        self.can.contains(&op)
    }
    fn rename_column(&self, column: &str, name: &str) -> Answer {
        self.answer(format!("rename {column} {name}"))
    }
    fn set_limit(&self, column: &str, limit: Option<u32>) -> Answer {
        self.answer(format!("limit {column} {limit:?}"))
    }
}

fn action(command: &str) -> ActionView {
    ActionView {
        trigger: "button".into(),
        command: command.into(),
    }
}

fn wiring(actions: &[&str], records: Arc<dyn Records>) -> Arc<Wiring> {
    Arc::new(Wiring {
        actions: Arc::new(actions.iter().map(|c| action(c)).collect()),
        records,
        ask: Arc::new(Ask::default()),
        say: Arc::new(|k: &str| k.to_owned()),
        labels: Labels::default(),
    })
}

fn board(w: &Arc<Wiring>) -> KanbanBoard {
    let say: Arc<dyn Say> = Arc::new(|k: &str| k.to_owned());
    KanbanBoard::new(w.clone(), say, Look::new(StyleId::Admin, false)).events(w.clone())
}

const BOTH: [Operation; 2] = [Operation::Rename, Operation::Limit];
const ACTIONS: [&str; 2] = ["column.rename", "column.limit"];

/// Type `line` in the open dialog and confirm it.
fn type_in(w: &Wiring, line: &str) -> bool {
    let dialog = w.ask.current().expect("a dialog is open");
    let _ = dialog.clone().initial(line);
    dialog.confirm()
}

#[test]
fn rename_starts_with_the_current_name_and_sends_the_new_one() {
    let records = Edits::new(&BOTH);
    let w = wiring(&ACTIONS, records.clone());
    board(&w).request(Operation::Rename, Some("todo"));
    assert_eq!(w.ask.current().expect("open").text(), "To do");
    assert!(type_in(&w, "  Backlog "));
    assert_eq!(records.calls(), ["rename todo Backlog"]);
    assert!(w.ask.current().is_none(), "taken: the dialog closes");
}

#[test]
fn the_limit_dialog_starts_with_the_limit_and_zero_means_no_limit() {
    let records = Edits::new(&BOTH);
    let w = wiring(&ACTIONS, records.clone());
    let b = board(&w);
    b.request(Operation::Limit, Some("todo"));
    assert_eq!(w.ask.current().expect("open").text(), "4");
    assert_eq!(w.ask.message().as_deref(), Some("kanban.limit_hint"));
    assert!(type_in(&w, "7"));
    b.request(Operation::Limit, Some("todo"));
    assert!(type_in(&w, "0"));
    assert_eq!(records.calls(), ["limit todo Some(7)", "limit todo None"]);
    // a column without a limit starts with an empty field
    b.request(Operation::Limit, Some("done"));
    assert_eq!(w.ask.current().expect("open").text(), "");
}

#[test]
fn a_line_that_is_no_number_never_reaches_the_records_and_the_dialog_says_so() {
    let records = Edits::new(&BOTH);
    let w = wiring(&ACTIONS, records.clone());
    board(&w).request(Operation::Limit, Some("todo"));
    for bad in ["abc", "-3", "2.5", "99999999999"] {
        assert!(type_in(&w, bad));
        assert!(w.ask.current().is_some(), "{bad}: still open");
        assert_eq!(w.ask.message().as_deref(), Some("kanban.limit_invalid"));
        assert_eq!(
            w.ask.current().expect("open").text(),
            bad,
            "what was typed stays"
        );
    }
    assert!(records.calls().is_empty());
    assert!(type_in(&w, "3"));
    assert_eq!(records.calls(), ["limit todo Some(3)"]);
    assert!(w.ask.current().is_none());
}

#[test]
fn a_conflict_shows_its_sentence_and_the_dialog_stays_until_a_try_works() {
    let records = Edits::new(&BOTH);
    let w = wiring(&ACTIONS, records.clone());
    board(&w).request(Operation::Rename, Some("todo"));
    records.refuse_with(Some("conflict"));
    assert!(type_in(&w, "Backlog"));
    assert!(
        w.ask.current().is_some(),
        "a refusal is not silence and not the end"
    );
    assert_eq!(w.ask.message().as_deref(), Some("kanban.refused.conflict"));
    assert!(records.calls().is_empty());
    records.refuse_with(None);
    assert!(type_in(&w, "Backlog"));
    assert_eq!(records.calls(), ["rename todo Backlog"]);
    assert!(w.ask.current().is_none());
    assert_eq!(w.ask.message(), None);
}

#[test]
fn cancel_leaves_without_a_change() {
    let records = Edits::new(&BOTH);
    let w = wiring(&ACTIONS, records.clone());
    board(&w).request(Operation::Rename, Some("todo"));
    w.ask.current().expect("open").cancel();
    assert!(w.ask.current().is_none());
    assert!(records.calls().is_empty());
}

#[test]
fn without_the_capability_or_the_action_the_control_stays_locked() {
    // the description has both actions, the records can only rename: the limit stays locked
    let w = wiring(&ACTIONS, Edits::new(&[Operation::Rename]));
    assert!(w.supports(Operation::Rename));
    assert!(!w.supports(Operation::Limit));
    board(&w).request(Operation::Limit, Some("todo"));
    assert!(w.ask.current().is_none(), "no dialog for a locked control");
    // the event called on its own does not open one either
    use fs_components_kanban::BoardEvents;
    w.set_limit("todo");
    assert!(w.ask.current().is_none());
    // the records can both, the description names only the limit: rename stays locked
    let w = wiring(&["column.limit"], Edits::new(&BOTH));
    assert!(!w.supports(Operation::Rename));
    assert!(w.supports(Operation::Limit));
    w.rename_column("todo");
    assert!(w.ask.current().is_none());
    // neither: nothing is unlocked
    let w = wiring(&[], Edits::new(&[]));
    for op in BOTH {
        assert!(!w.supports(op), "{op:?}");
    }
}
