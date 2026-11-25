//! The way of a gesture to the module on a fake module (SW-4a): the rule for the arguments (only what the form says it writes, the
//! context from the record, the channel the gesture must deliver), the conditions of `can` (the binding, the command set, `may`:
//! what is not said is locked), and the call itself (accepted: read again; refused: nothing changes; never asked without need).
//!
//! Mutants (run by hand, all red): `allowed` true by default; the context of the record dropped; a channel without binding passed by
//! its own name; no new reading after `Accepted`; the `!` check dropped (the module is asked).
//!
//! Pattern: none (tests).

use std::sync::Arc;

use super::fixture::{FakeWriter, records};
use super::generated::{
    CREATE, MOVE, column_of, data, field, first, lane_of, moved, other_column, second,
};
use super::{allowed, args_of, command_key};
use crate::form_data::FormRecords;
use crate::records::Records;

fn gesture(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(c, v)| ((*c).to_owned(), (*v).to_owned()))
        .collect()
}

fn pairs(v: &[(String, String)]) -> Vec<(&str, &str)> {
    v.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()
}

#[test]
fn the_arguments_are_what_the_form_says_the_command_writes_by_the_fields_of_the_module() {
    let d = data();
    let key = first();
    let a = d.rows.iter().find(|r| r.key == key);
    let (to, lane) = (other_column(&d, &key), lane_of(&d, &key));
    let (after, before, board) = (field(&d, "after"), field(&d, "before"), field(&d, "lane"));
    let args = args_of(
        &d,
        "move",
        a,
        &gesture(&[("column", &to), ("after", "c"), ("before", "d")]),
    )
    .unwrap();
    // `lane` is not in the gesture: the context of the record (the field of the lane in the module) fills it
    assert_eq!(
        pairs(&args),
        [
            ("column", to.as_str()),
            (after.as_str(), "c"),
            (before.as_str(), "d"),
            (board.as_str(), lane.as_str())
        ]
    );
    // without a place (named, no value): no place is written although the record has one (its `order` in the data)
    assert!(
        a.is_some_and(|r| r.get("order").is_some()),
        "the card has a place"
    );
    let args = args_of(
        &d,
        "move",
        a,
        &gesture(&[("column", &to), ("after", ""), ("before", "")]),
    )
    .unwrap();
    assert_eq!(
        pairs(&args),
        [("column", to.as_str()), (board.as_str(), lane.as_str())]
    );
    // a place the gesture does not name at all is not a place of the gesture: only the scope comes from the record
    let args = args_of(&d, "move", a, &gesture(&[("column", &to)])).unwrap();
    let order = field(&d, "order");
    assert!(
        !args.iter().any(|(f, _)| *f == order),
        "order is never written"
    );
    // what the gesture carries beyond the writes is not passed on
    let args = args_of(&d, "move", a, &gesture(&[("column", "x"), ("title", "no")])).unwrap();
    let title = field(&d, "title");
    assert!(!pairs(&args).iter().any(|(f, _)| *f == title));
}

#[test]
fn a_channel_the_gesture_must_deliver_ends_the_call_before_it_is_sent() {
    let d = data();
    let column = column_of(&d, &first());
    let e = args_of(&d, "create", None, &gesture(&[("column", &column)])).unwrap_err();
    assert_eq!(
        (e.code.as_str(), e.params[0].1.as_str()),
        ("form-write-missing", "title")
    );
    let w = Arc::new(FakeWriter::default());
    let r = records(&w, d);
    let answer = r.invoke(
        "cards-in-columns",
        "create",
        None,
        &gesture(&[("column", &column)]),
    );
    assert_eq!(answer, crate::records::refused("form-write-missing"));
    assert!(
        w.calls.lock().unwrap().is_empty(),
        "the module is never asked"
    );
}

#[test]
fn a_channel_without_a_binding_in_the_map_is_locked_not_left_out() {
    let mut d = data();
    d.declared.map.retain(|(c, _)| c != "column");
    let key = first();
    let to = other_column(&d, &key);
    let a = d.rows.iter().find(|r| r.key == key);
    let e = args_of(&d, "move", a, &gesture(&[("column", &to)])).unwrap_err();
    assert_eq!(e.code, "form-write-unmapped");
    let mut d = data();
    d.declared.map.retain(|(c, _)| c != "move");
    assert!(command_key(&d, "move").is_none());
    assert!(
        !allowed(&d, "move", Some(&key)),
        "no command key: the channel name is never used as a path"
    );
}

#[test]
fn what_is_not_said_is_locked() {
    let d = data();
    let (a, b) = (first(), second());
    assert!(allowed(&d, "move", Some(&a)));
    assert!(!allowed(&d, "move", Some(&b)), "no `may` on the record");
    assert!(!allowed(&d, "move", Some("zzz")), "no such record");
    assert!(
        allowed(&d, "move", None),
        "one record allows it: the control of the board is not locked"
    );
    assert!(allowed(&d, "create", None), "the page says create");
    assert!(
        !allowed(&d, "remove", None),
        "bound, but not in the command set and not in `may`"
    );
    let mut d = data();
    d.commands.retain(|c| c != MOVE);
    assert!(
        !allowed(&d, "move", Some(&a)),
        "not in the command set of the module"
    );
    let mut d = data();
    d.rows.iter_mut().for_each(|r| r.may.clear());
    d.may.clear();
    assert!(!allowed(&d, "move", None) && !allowed(&d, "create", None));
    assert!(
        !allowed(&data(), "title", None),
        "a data channel is no command"
    );
    assert!(!allowed(&data(), "nothing", Some(&a)));
    assert!(command_key(&data(), "title").is_none() && command_key(&data(), "column").is_none());
    assert_eq!(command_key(&data(), "move"), Some(MOVE));
}

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
    let w = Arc::new(FakeWriter::default());
    let r = records(&w, data());
    let d = data();
    let (a, b) = (first(), second());
    let to = other_column(&d, &b);
    let answer = r.invoke(
        "cards-in-columns",
        "move",
        Some(&b),
        &gesture(&[("column", &to)]),
    );
    assert_eq!(answer, crate::records::refused("locked"));
    assert!(w.calls.lock().unwrap().is_empty());
    let none = FormRecords {
        forms: vec![data()],
        ..FormRecords::default()
    };
    assert!(!none.can_channel("cards-in-columns", "move", Some(&a)));
    assert_eq!(
        none.invoke("cards-in-columns", "move", Some(&a), &[]),
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
