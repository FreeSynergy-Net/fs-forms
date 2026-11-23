//! The gestures of the board on a module that writes (SW-4a): the description names the command by the key of the contract
//! (`card.move`), the module says what may be done, a drop is ONE call with the args the form says, and after `Accepted` the board
//! shows what the module now says. Mutants (by hand, all red): `can` true by default (the unsaid control unlocks); the key
//! bypassed (the channel name `move` taken as the command); no new reading after `Accepted`; the context `lane` not filled.
//!
//! Pattern: none (tests).

use std::sync::Arc;

use fs_components_kanban::{Answer, BoardEvents, BoardSource, Operation};
use fs_components_widgets::dialogs::Labels;

use super::{Ask, Wiring};
use crate::view::ActionView;
use crate::write::fixture::{FakeWriter, data, records};

fn action(trigger: &str, command: &str) -> ActionView {
    ActionView {
        trigger: trigger.into(),
        command: command.into(),
    }
}

fn wiring(actions: Vec<ActionView>, w: &Arc<FakeWriter>) -> Wiring {
    Wiring {
        actions: Arc::new(actions),
        records: Arc::new(records(w, data())),
        ask: Arc::new(Ask::default()),
        say: Arc::new(|k: &str| k.to_owned()),
        labels: Labels::default(),
    }
}

fn described() -> Vec<ActionView> {
    vec![
        action("drop", "card.move"),
        action("quick-add", "task.create"),
    ]
}

#[test]
fn a_control_is_unlocked_by_the_key_of_the_contract_the_module_says_may_be_used() {
    let w = Arc::new(FakeWriter::default());
    let on = wiring(described(), &w);
    assert!(on.supports(Operation::Move) && on.supports(Operation::AddCard));
    // the description names the CHANNEL, not the key of the contract: no action for this command, the control stays locked
    let by_channel = wiring(
        vec![action("drop", "move"), action("quick-add", "create")],
        &w,
    );
    assert!(!by_channel.supports(Operation::Move) && !by_channel.supports(Operation::AddCard));
    let none = wiring(Vec::new(), &w);
    assert!(!none.supports(Operation::Move));
    // rename and limit are not channels of this form: still locked
    assert!(!on.supports(Operation::Rename) && !on.supports(Operation::Limit));
}

#[test]
fn a_drop_is_one_call_with_the_column_the_place_and_the_lane_of_the_record() {
    let w = Arc::new(FakeWriter::default());
    let on = wiring(described(), &w);
    assert_eq!(
        on.move_card_at("a", "todo", "doing", Some(1)),
        Answer::Accepted
    );
    let calls = w.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command, "card.move");
    let args: Vec<_> = calls[0]
        .args
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(args, [("column", "doing"), ("index", "1"), ("board", "b1")]);
    drop(calls);
    // in the own column: a reorder, it is a move like any other (the place is the whole point)
    assert_eq!(
        on.move_card_at("a", "todo", "todo", Some(0)),
        Answer::Accepted
    );
    assert_eq!(w.calls.lock().unwrap().len(), 2);
}

#[test]
fn a_card_the_module_does_not_allow_is_refused_locked_and_the_module_is_not_asked() {
    let w = Arc::new(FakeWriter::default());
    let on = wiring(described(), &w);
    assert_eq!(
        on.move_card_at("b", "doing", "todo", None),
        Answer::Refused("locked".into())
    );
    assert!(w.calls.lock().unwrap().is_empty());
}

#[test]
fn a_quick_add_creates_with_title_and_column_and_a_refusal_is_given_back_as_its_code() {
    let w = Arc::new(FakeWriter::default());
    let on = wiring(described(), &w);
    assert_eq!(on.add_card("todo", "Ship it"), Answer::Accepted);
    let call = w.calls.lock().unwrap()[0].clone();
    assert_eq!(call.command, "task.create");
    assert_eq!(call.record, None);
    let args: Vec<_> = call
        .args
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(args, [("title", "Ship it"), ("column", "todo")]);
    *w.refuse.lock().unwrap() = Some("host-notice-invoke-refused".into());
    assert_eq!(
        on.add_card("todo", "x"),
        Answer::Refused("host-notice-invoke-refused".into())
    );
}

#[test]
fn an_action_that_calls_another_command_than_the_module_bound_to_the_gesture_is_warned() {
    let w = Arc::new(FakeWriter::default());
    let r = records(&w, data());
    assert_eq!(crate::action_report(&described(), &r), Vec::new());
    let wrong = [
        action("drop", "move"),
        action("quick-add", "task.create"),
        action("button", "anything"),
    ];
    let said = crate::action_report(&wrong, &r);
    assert_eq!(said.len(), 1);
    assert_eq!(said[0].code.code, "action-channel-mismatch");
    assert_eq!(
        said[0].code.params,
        [
            ("trigger".to_owned(), "drop".to_owned()),
            ("command".to_owned(), "move".to_owned())
        ]
    );
    assert!(!said[0].red);
}

#[test]
fn a_control_stays_locked_while_the_module_says_nothing_may_be_done() {
    let w = Arc::new(FakeWriter::default());
    let mut silent = data();
    silent.may.clear();
    silent.rows.iter_mut().for_each(|r| r.may.clear());
    let on = Wiring {
        records: Arc::new(records(&w, silent)),
        ..wiring(described(), &w)
    };
    assert!(!on.supports(Operation::Move) && !on.supports(Operation::AddCard));
}

#[test]
fn a_drop_at_the_end_of_another_column_sends_no_place_even_when_the_record_has_one() {
    // r1 MUSS 1: the fixture card `a` has `order = 3`; a drop without an index (the end) must not send the old place
    let w = Arc::new(FakeWriter::default());
    let on = wiring(described(), &w);
    assert_eq!(
        on.move_card_at("a", "todo", "doing", None),
        Answer::Accepted
    );
    let calls = w.calls.lock().unwrap();
    let args: Vec<_> = calls[0]
        .args
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(args, [("column", "doing"), ("board", "b1")]);
}

#[test]
fn the_board_uses_the_write_port_of_the_component_not_a_copy_of_its_gestures() {
    // r1 MUSS 3: the gesture reaches the module through `FormEvents`/`FormSupports` (fs-components-kanban) over the port
    let src = include_str!("../events.rs");
    assert!(src.contains("FormEvents::new(") && src.contains("FormSupports::new("));
    for literal in ["\"column\"", "\"order\"", "\"title\""] {
        assert!(!src.contains(literal), "the host names {literal}");
    }
}
