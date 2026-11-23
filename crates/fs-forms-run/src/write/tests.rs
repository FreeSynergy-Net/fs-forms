//! The way of a gesture to the module on a fake module (SW-4a): the rule for the arguments (only what the form says it writes, the
//! context from the record, the channel the gesture must deliver), the conditions of `can` (the binding, the command set, `may`:
//! what is not said is locked), and the call itself (accepted: read again; refused: nothing changes; never asked without need).
//!
//! Mutants (run by hand, all red): `allowed` true by default; the context of the record dropped; a channel without binding passed by
//! its own name; no new reading after `Accepted`; the `!` check dropped (the module is asked).
//!
//! Pattern: none (tests).

use std::sync::Arc;

use super::fixture::{FakeWriter, data, records, row};
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
    let a = d.rows.first();
    let args = args_of(
        &d,
        "move",
        a,
        &gesture(&[("column", "doing"), ("order", "1")]),
    )
    .unwrap();
    // `lane` is not in the gesture: the context of the record (`board` of the module) fills it
    assert_eq!(
        pairs(&args),
        [("column", "doing"), ("index", "1"), ("board", "b1")]
    );
    // without a place: `order` is optional and left out
    let args = args_of(&d, "move", a, &gesture(&[("column", "doing")])).unwrap();
    assert_eq!(pairs(&args), [("column", "doing"), ("board", "b1")]);
    // what the gesture carries beyond the writes is not passed on
    let args = args_of(&d, "move", a, &gesture(&[("column", "x"), ("title", "no")])).unwrap();
    assert!(!pairs(&args).iter().any(|(f, _)| *f == "title"));
}

#[test]
fn a_channel_the_gesture_must_deliver_ends_the_call_before_it_is_sent() {
    let d = data();
    let e = args_of(&d, "create", None, &gesture(&[("column", "todo")])).unwrap_err();
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
        &gesture(&[("column", "todo")]),
    );
    assert_eq!(
        answer,
        fs_components_kanban::Answer::Refused("form-write-missing".into())
    );
    assert!(
        w.calls.lock().unwrap().is_empty(),
        "the module is never asked"
    );
}

#[test]
fn a_channel_without_a_binding_in_the_map_is_locked_not_left_out() {
    let mut d = data();
    d.declared.map.retain(|(c, _)| c != "column");
    let e = args_of(&d, "move", d.rows.first(), &gesture(&[("column", "doing")])).unwrap_err();
    assert_eq!(e.code, "form-write-unmapped");
    let mut d = data();
    d.declared.map.retain(|(c, _)| c != "move");
    assert!(command_key(&d, "move").is_none());
    assert!(
        !allowed(&d, "move", Some("a")),
        "no command key: the channel name is never used as a path"
    );
}

#[test]
fn what_is_not_said_is_locked() {
    let d = data();
    assert!(allowed(&d, "move", Some("a")));
    assert!(!allowed(&d, "move", Some("b")), "no `may` on the record");
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
    d.commands.retain(|c| c != "card.move");
    assert!(
        !allowed(&d, "move", Some("a")),
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
    assert!(!allowed(&data(), "nothing", Some("a")));
    assert!(command_key(&data(), "title").is_none() && command_key(&data(), "column").is_none());
    assert_eq!(command_key(&data(), "move"), Some("card.move"));
}

#[test]
fn an_accepted_call_reads_the_form_again_and_the_board_shows_the_new_state() {
    let w = Arc::new(FakeWriter::default());
    let r = records(&w, data());
    assert_eq!(
        r.columns()
            .iter()
            .find(|c| c.key == "todo")
            .map(|c| c.cards.len()),
        Some(1)
    );
    let mut moved = data();
    moved.rows[0] = row("a", "doing", "In progress", &["card.move"]);
    *w.after.lock().unwrap() = vec![moved];
    let answer = r.invoke(
        "cards-in-columns",
        "move",
        Some("a"),
        &gesture(&[("column", "doing"), ("order", "0")]),
    );
    assert_eq!(answer, fs_components_kanban::Answer::Accepted);
    let call = w.calls.lock().unwrap()[0].clone();
    assert_eq!(call.command, "card.move");
    assert_eq!(call.record.as_deref(), Some("a"));
    assert_eq!(
        pairs(&call.args),
        [("column", "doing"), ("index", "0"), ("board", "b1")]
    );
    assert_eq!(*w.reloads.lock().unwrap(), 1);
    let cols = r.columns();
    assert!(
        cols.iter().all(|c| c.key != "todo"),
        "a is no longer in todo"
    );
    assert_eq!(
        cols.iter()
            .find(|c| c.key == "doing")
            .map(|c| c.cards.len()),
        Some(2)
    );
}

#[test]
fn a_refused_call_changes_nothing_and_is_not_read_again() {
    let w = Arc::new(FakeWriter::default());
    *w.refuse.lock().unwrap() = Some("host-notice-invoke-refused".into());
    let r = records(&w, data());
    let answer = r.invoke(
        "cards-in-columns",
        "move",
        Some("a"),
        &gesture(&[("column", "doing")]),
    );
    assert_eq!(
        answer,
        fs_components_kanban::Answer::Refused("host-notice-invoke-refused".into())
    );
    assert_eq!(*w.reloads.lock().unwrap(), 0);
    assert_eq!(
        r.columns()
            .iter()
            .find(|c| c.key == "todo")
            .map(|c| c.cards.len()),
        Some(1)
    );
}

#[test]
fn a_locked_gesture_never_reaches_the_module_and_records_without_a_writer_cannot_write() {
    let w = Arc::new(FakeWriter::default());
    let r = records(&w, data());
    let answer = r.invoke(
        "cards-in-columns",
        "move",
        Some("b"),
        &gesture(&[("column", "todo")]),
    );
    assert_eq!(
        answer,
        fs_components_kanban::Answer::Refused("locked".into())
    );
    assert!(w.calls.lock().unwrap().is_empty());
    let none = FormRecords {
        forms: vec![data()],
        ..FormRecords::default()
    };
    assert!(!none.can_channel("cards-in-columns", "move", Some("a")));
    assert_eq!(
        none.invoke("cards-in-columns", "move", Some("a"), &[]),
        fs_components_kanban::Answer::Refused("unsupported".into())
    );
}
