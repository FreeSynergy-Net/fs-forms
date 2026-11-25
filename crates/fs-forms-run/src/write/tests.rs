//! The way of a gesture to the module on a fake module (SW-4a): the rule for the arguments (only what the form says it writes, the
//! context from the record, the channel the gesture must deliver), the conditions of `can` (the binding, the command set, `may`:
//! what is not said is locked), and the call itself (accepted: read again; refused: nothing changes; never asked without need).
//!
//! Mutants (run by hand, all red): `allowed` true by default; the context of the record dropped; a channel without binding passed by
//! its own name; no new reading after `Accepted`; the `!` check dropped (the module is asked).
//!
//! Pattern: none (tests).

use std::sync::Arc;

use super::fixture::{
    FakeWriter, MOVE, column_of, data, field, first, lane_of, other_column, records, second,
};
use super::{allowed, args_of, command_key};
use crate::records::Records;

pub(super) fn gesture(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(c, v)| ((*c).to_owned(), (*v).to_owned()))
        .collect()
}

pub(super) fn pairs(v: &[(String, String)]) -> Vec<(&str, &str)> {
    v.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()
}

#[test]
fn the_fields_of_the_module_are_not_named_like_the_channels() {
    // otherwise a call that writes the channel name instead of the bound field would pass every test of the arguments
    let d = data();
    for channel in ["after", "before", "lane", "column", "title"] {
        assert_ne!(field(&d, channel), channel, "{channel}");
    }
}

#[test]
fn the_arguments_are_what_the_form_says_the_command_writes_by_the_fields_of_the_module() {
    let d = data();
    let key = first();
    let a = d.rows.iter().find(|r| r.key == key);
    let (to, lane) = (other_column(&d, &key), lane_of(&d, &key));
    let (after, before, board) = (field(&d, "after"), field(&d, "before"), field(&d, "lane"));
    let col = field(&d, "column");
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
            (col.as_str(), to.as_str()),
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
        [(col.as_str(), to.as_str()), (board.as_str(), lane.as_str())]
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
