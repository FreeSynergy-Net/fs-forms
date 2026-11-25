//! The call of the written way on the fake module (SW-4a): an accepted call reads the form again, a refused one changes nothing, a
//! locked gesture never reaches the module, a failed reading after an accepted call is no refusal, `create` comes from the page only.
//! The data come from the generator (`fixture`); no test names a card or a column.
//!
//! Pattern: none (tests).

use std::sync::Arc;

use super::allowed;
use super::fixture::{
    CREATE, FakeWriter, MOVE, column_of, data, field, first, lane_of, moved, other_column, records,
    second,
};
use super::tests::{gesture, pairs};
use crate::form_data::FormRecords;
use crate::records::Records;

#[test]
fn an_accepted_call_reads_the_form_again_and_the_board_shows_the_new_state() {
    let w = Arc::new(FakeWriter::default());
    let r = records(&w, data());
    let d = data();
    let (a, from, to) = (first(), column_of(&d, &first()), other_column(&d, &first()));
    let (before, board) = (field(&d, "before"), field(&d, "lane"));
    let lane = lane_of(&d, &a);
    assert_eq!(
        columns(&r)
            .iter()
            .find(|c| c.key == from)
            .map(|c| c.cards.len()),
        Some(1)
    );
    let mut after = data();
    let at = after.rows.iter().position(|r| r.key == a).unwrap();
    after.rows[at] = moved(&after.rows[at], &to);
    *w.after.lock().unwrap() = vec![after];
    let answer = r.invoke(
        "cards-in-columns",
        "move",
        Some(&a),
        &gesture(&[("column", &to), ("before", "c")]),
    );
    assert_eq!(answer, fs_components_kanban::Answer::Accepted);
    let call = w.calls.lock().unwrap()[0].clone();
    assert_eq!(call.command, MOVE);
    assert_eq!(call.record.as_deref(), Some(a.as_str()));
    assert_eq!(
        pairs(&call.args),
        [
            ("column", to.as_str()),
            (before.as_str(), "c"),
            (board.as_str(), lane.as_str())
        ]
    );
    assert_eq!(*w.reloads.lock().unwrap(), 1);
    let cols = columns(&r);
    assert!(
        cols.iter().all(|c| c.key != from),
        "the card is no longer in its old column"
    );
    assert_eq!(
        cols.iter().find(|c| c.key == to).map(|c| c.cards.len()),
        Some(2)
    );
}

#[test]
fn a_refused_call_changes_nothing_and_is_not_read_again() {
    let w = Arc::new(FakeWriter::default());
    *w.refuse.lock().unwrap() = Some("host-notice-invoke-refused".into());
    let r = records(&w, data());
    let d = data();
    let (a, from, to) = (first(), column_of(&d, &first()), other_column(&d, &first()));
    let answer = r.invoke(
        "cards-in-columns",
        "move",
        Some(&a),
        &gesture(&[("column", &to)]),
    );
    assert_eq!(
        answer,
        crate::records::refused("host-notice-invoke-refused")
    );
    assert_eq!(*w.reloads.lock().unwrap(), 0);
    assert_eq!(
        columns(&r)
            .iter()
            .find(|c| c.key == from)
            .map(|c| c.cards.len()),
        Some(1)
    );
}

#[test]
fn a_locked_gesture_never_reaches_the_module_and_records_without_a_writer_cannot_write() {
    let writer = Arc::new(FakeWriter::default());
    let rec = records(&writer, data());
    let d = data();
    let (key, other) = (first(), second());
    let to = other_column(&d, &other);
    let answer = rec.invoke(
        "cards-in-columns",
        "move",
        Some(&other),
        &gesture(&[("column", &to)]),
    );
    assert_eq!(answer, crate::records::refused("locked"));
    assert!(writer.calls.lock().unwrap().is_empty());
    let none = FormRecords {
        forms: vec![data()],
        ..FormRecords::default()
    };
    assert!(!none.can_channel("cards-in-columns", "move", Some(&key)));
    assert_eq!(
        none.invoke("cards-in-columns", "move", Some(&key), &[]),
        crate::records::refused("unsupported")
    );
}

#[test]
fn a_failed_reading_after_an_accepted_call_is_no_refusal_and_says_so() {
    // r1 SOLL 3: the module HAS written; the answer is `Accepted`, the old state stays, the failure is kept as a code
    let w = Arc::new(FakeWriter::default());
    *w.reload_fails.lock().unwrap() = true;
    let r = records(&w, data());
    let d = data();
    let (a, to) = (first(), other_column(&d, &first()));
    let answer = r.invoke(
        "cards-in-columns",
        "move",
        Some(&a),
        &gesture(&[("column", &to)]),
    );
    assert_eq!(answer, fs_components_kanban::Answer::Accepted);
    assert_eq!(r.stale().as_deref(), Some("host-notice-data-reload"));
    assert_eq!(r.forms().len(), 1, "the old state stays");
    *w.reload_fails.lock().unwrap() = false;
    r.invoke(
        "cards-in-columns",
        "move",
        Some(&a),
        &gesture(&[("column", &to)]),
    );
    assert_eq!(r.stale(), None, "a good reading clears it");
}

#[test]
fn create_comes_only_from_the_page_never_from_the_may_of_a_row() {
    let mut d = data();
    d.may.clear();
    d.rows[0].may.push(CREATE.into());
    assert!(
        !allowed(&d, "create", None),
        "a row that names create does not open the quick add"
    );
    assert!(allowed(&d, "move", None), "move: one record allows it");
    let mut d = data();
    d.rows.iter_mut().for_each(|r| r.may.clear());
    assert!(allowed(&d, "create", None), "the page says create");
}

/// The columns of the board of the form `cards-in-columns` as the component reads them from the records.
fn columns(r: &FormRecords) -> Vec<fs_components_kanban::Column> {
    use crate::records::Records as _;
    let forms = r.forms();
    let data = forms
        .iter()
        .find(|d| d.declared.form.name == "cards-in-columns");
    data.map_or_else(Vec::new, |d| {
        fs_components_kanban::columns_of(&crate::form_data::input_of(d))
    })
}
