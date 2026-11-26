//! The gestures of a place (`FormWrite::fire`, `may_fire`): the way gesture, action, command, channel only for a slot with `args`.
//!
//! Tests: a drop is ONE call of `card.move` straight from the gesture, with column, lane and place as the gesture gives them (no channel
//! name in between); an argument of the slot the gesture gives no value (the end of a column has no `before`, a board without lanes no
//! `lane`) is carried WITHOUT value and never filled from the record, a gesture no
//! action binds at THIS place is `gesture-unbound`, an action at another place does not open it, a slot without args calls the command
//! with the record only (and only with a record), an `open` action is refused with its own code, a card the module does not allow is
//! `locked` and the module is not asked, several actions on one gesture are `gesture-ambiguous`.
//!
//! Pattern: none (tests).

use std::sync::Arc;

use fs_components_core::form_write::{FormWrite, WriteArg, WriteValue};

use super::WriteGate;
use crate::view::ActionView;
use crate::write::fixture::{
    CREATE as CREATE_KEY, FakeWriter, MOVE as MOVE_KEY, REMOVE, data, field, first, other_column,
    records, second,
};

const PLACE: &str = "block:board/main";

fn action(at: Option<&str>, trigger: &str, command: &str) -> ActionView {
    ActionView {
        trigger: trigger.into(),
        at: at.map(str::to_owned),
        command: command.into(),
        open: None,
    }
}

fn gate(actions: Vec<ActionView>, w: &Arc<FakeWriter>) -> WriteGate {
    let mut d = data();
    // the module also allows to delete the first card (a command the form has no channel for here)
    d.commands.push(REMOVE.to_owned());
    d.rows
        .iter_mut()
        .find(|r| r.key == first())
        .expect("the first card")
        .may
        .push(REMOVE.to_owned());
    WriteGate {
        records: Arc::new(records(w, d)),
        actions: Arc::new(actions),
        form: "cards-in-columns".into(),
        block: "board".into(),
        place: PLACE.into(),
        after: crate::after_write::AfterWrite::default(),
    }
}

fn bound() -> Vec<ActionView> {
    vec![
        action(Some(PLACE), "drop", MOVE_KEY),
        action(Some(PLACE), "quick-add", CREATE_KEY),
        action(Some(PLACE), "menu", REMOVE),
    ]
}

fn text(name: &str, value: &str) -> WriteArg {
    WriteArg {
        name: name.into(),
        value: WriteValue::Text(value.into()),
    }
}

fn args_of_call(w: &FakeWriter) -> Vec<(String, String)> {
    w.calls.lock().unwrap()[0].args.clone()
}

#[test]
fn a_drop_calls_the_command_of_the_action_with_what_the_gesture_carries_and_the_lane_of_the_record()
{
    let w = Arc::new(FakeWriter::default());
    let g = gate(bound(), &w);
    let (d, a) = (data(), first());
    let to = other_column(&d, &a);
    g.may_fire("drop", Some(&a)).expect("may");
    g.fire(
        "drop",
        Some(&a),
        &[
            text("column", &to),
            text("lane", "l2"),
            text("after", "c"),
            text("before", "d"),
        ],
    )
    .expect("accepted");
    let calls = w.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command, MOVE_KEY);
    drop(calls);
    let (col, after, before, board) = (
        field(&d, "column"),
        field(&d, "after"),
        field(&d, "before"),
        field(&d, "lane"),
    );
    assert_eq!(
        args_of_call(&w),
        [
            (col, to),
            (after, "c".to_owned()),
            (before, "d".to_owned()),
            (board, "l2".to_owned())
        ]
    );
}

#[test]
fn a_drop_at_the_end_carries_no_place_and_no_lane_it_was_not_given_even_when_the_record_has_them() {
    let w = Arc::new(FakeWriter::default());
    let g = gate(bound(), &w);
    let (d, a) = (data(), first());
    let to = other_column(&d, &a);
    g.fire("drop", Some(&a), &[text("column", &to)])
        .expect("accepted");
    assert_eq!(args_of_call(&w), [(field(&d, "column"), to)]);
}

#[test]
fn a_gesture_no_action_binds_at_this_place_is_unbound_and_nothing_is_sent() {
    let w = Arc::new(FakeWriter::default());
    let a = first();
    let other = vec![
        action(Some("block:other/main"), "drop", MOVE_KEY),
        action(None, "drop", MOVE_KEY),
        action(Some(PLACE), "quick-add", CREATE_KEY),
    ];
    let g = gate(other, &w);
    let refused = g
        .fire("drop", Some(&a), &[text("column", "x")])
        .unwrap_err();
    assert_eq!(refused.code, "gesture-unbound");
    assert_eq!(refused.params, [("trigger".to_owned(), "drop".to_owned())]);
    assert_eq!(
        g.may_fire("drop", Some(&a)).unwrap_err().code,
        "gesture-unbound"
    );
    assert!(w.calls.lock().unwrap().is_empty());
}

#[test]
fn a_card_the_module_does_not_allow_is_locked_and_the_module_is_not_asked() {
    let w = Arc::new(FakeWriter::default());
    let g = gate(bound(), &w);
    let (d, b) = (data(), second());
    let to = other_column(&d, &b);
    let refused = g
        .fire("drop", Some(&b), &[text("column", &to)])
        .unwrap_err();
    assert_eq!(refused.code, "locked");
    assert!(w.calls.lock().unwrap().is_empty());
}

#[test]
fn a_refusal_of_the_module_is_given_back_as_its_code() {
    let w = Arc::new(FakeWriter::default());
    *w.refuse.lock().unwrap() = Some("host-notice-invoke-refused".into());
    let g = gate(bound(), &w);
    let (d, a) = (data(), first());
    let to = other_column(&d, &a);
    let refused = g
        .fire("drop", Some(&a), &[text("column", &to)])
        .unwrap_err();
    assert_eq!(refused.code, "host-notice-invoke-refused");
}

#[test]
fn a_slot_without_args_calls_the_command_with_the_record_only_and_only_with_a_record() {
    let w = Arc::new(FakeWriter::default());
    let g = gate(bound(), &w);
    let a = first();
    // no record: nothing to call
    assert_eq!(g.may_fire("menu", None).unwrap_err().code, "locked");
    g.fire("menu", Some(&a), &[]).expect("accepted");
    let calls = w.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command, REMOVE);
    assert_eq!(calls[0].record.as_deref(), Some(a.as_str()));
    assert_eq!(calls[0].args, Vec::<(String, String)>::new());
}

#[test]
fn a_slot_without_args_is_locked_for_a_card_the_module_does_not_allow() {
    let w = Arc::new(FakeWriter::default());
    let g = gate(bound(), &w);
    let b = second();
    assert_eq!(g.fire("menu", Some(&b), &[]).unwrap_err().code, "locked");
    assert!(w.calls.lock().unwrap().is_empty());
}

#[test]
fn an_action_that_opens_a_form_is_refused_with_its_own_code() {
    let w = Arc::new(FakeWriter::default());
    let mut open = action(Some(PLACE), "activate", "open");
    open.open = Some("detail".into());
    let g = gate(vec![open], &w);
    let refused = g.fire("activate", Some(&first()), &[]).unwrap_err();
    assert_eq!(refused.code, "open-unsupported");
}

#[test]
fn several_actions_on_one_gesture_are_ambiguous_because_the_port_names_no_entry() {
    let w = Arc::new(FakeWriter::default());
    let two = vec![
        action(Some(PLACE), "menu", REMOVE),
        action(Some(PLACE), "menu", "card.claim"),
    ];
    let g = gate(two, &w);
    let refused = g.fire("menu", Some(&first()), &[]).unwrap_err();
    assert_eq!(refused.code, "gesture-ambiguous");
    assert!(w.calls.lock().unwrap().is_empty());
}

#[test]
fn a_drop_through_the_catalog_slot_knows_no_channel_name() {
    // the module binds `card.move` to a channel; an action for a command the module binds to no channel is unbound (no fallback
    // "the name of the channel is the command")
    let w = Arc::new(FakeWriter::default());
    let g = gate(vec![action(Some(PLACE), "drop", "move")], &w);
    let refused = g
        .fire("drop", Some(&first()), &[text("column", "x")])
        .unwrap_err();
    assert_eq!(refused.code, "gesture-unbound");
}
