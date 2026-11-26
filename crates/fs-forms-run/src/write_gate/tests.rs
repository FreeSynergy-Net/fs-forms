//! The locks and the rights of the write gate (moved from the tests of the old `events/` wiring, now on `WriteGate::can` and `send`):
//! a control is unlocked only when the description has an action for it AND the records can do it; the module's own command goes to
//! the module, with exactly what the gesture carries; anything else is refused `locked` and nobody is asked.
//!
//! Tests: description without action, records that cannot, one action per operation, the key of the contract against the channel
//! name, a card the module does not allow, one call per drop with column/place/lane, no old place at the end of a column, a refusal
//! given back as its code, `send` refuses on its own, records that bind no command (locked, the module is not asked), an action that calls another command
//! than the module bound to its gesture is warned.
//!
//! Pattern: none (tests).

use std::sync::Arc;

use fs_components_core::form_write::{FormWrite, WriteArg, WriteValue};
use fs_components_kanban::{COLUMN_CREATE, COLUMN_LIMIT, COLUMN_RENAME, CREATE, MOVE};

use super::WriteGate;
use crate::records::{NoRecords, Records};
use crate::view::ActionView;
use crate::write::fixture::{
    CREATE as CREATE_KEY, FakeWriter, MOVE as MOVE_KEY, column_of, data, field, first, lane_of,
    other_column, records, second,
};

/// The element id of the place the gate stands at.
const PLACE: &str = "block:board/main";

fn action(trigger: &str, command: &str) -> ActionView {
    ActionView {
        trigger: trigger.into(),
        at: Some(PLACE.into()),
        command: command.into(),
        open: None,
        confirm: false,
    }
}

/// The actions of a description that names the commands by the key of the contract.
fn described() -> Vec<ActionView> {
    vec![action("drop", MOVE_KEY), action("quick-add", CREATE_KEY)]
}

fn gate(actions: Vec<ActionView>, records: Arc<dyn Records>) -> WriteGate {
    WriteGate {
        records,
        actions: Arc::new(actions),
        form: "cards-in-columns".into(),
        block: "board".into(),
        place: PLACE.into(),
        after: crate::after_write::AfterWrite::default(),
    }
}

fn module(actions: Vec<ActionView>, w: &Arc<FakeWriter>) -> WriteGate {
    gate(actions, Arc::new(records(w, data())))
}

fn text(name: &str, value: &str) -> WriteArg {
    WriteArg {
        name: name.into(),
        value: WriteValue::Text(value.into()),
    }
}

#[test]
fn without_an_action_in_the_description_everything_stays_locked() {
    let w = Arc::new(FakeWriter::default());
    for actions in [Vec::new(), vec![action("menu", "delete")]] {
        let g = module(actions, &w);
        for channel in [MOVE, CREATE, COLUMN_CREATE, COLUMN_RENAME, COLUMN_LIMIT] {
            assert!(!g.can(channel, None), "{channel} stays locked");
        }
    }
}

#[test]
fn a_control_is_unlocked_by_the_key_of_the_contract_the_module_says_may_be_used() {
    let w = Arc::new(FakeWriter::default());
    let on = module(described(), &w);
    let (a, b) = (first(), second());
    assert!(on.can(MOVE, Some(&a)) && on.can(CREATE, None));
    // the description names the CHANNEL, not the key of the contract: no action for this command, the control stays locked
    let by_channel = module(
        vec![action("drop", "move"), action("quick-add", "create")],
        &w,
    );
    assert!(!by_channel.can(MOVE, Some(&a)) && !by_channel.can(CREATE, None));
    // rename and limit are not channels of this form: locked
    let column = column_of(&data(), &b);
    assert!(!on.can(COLUMN_RENAME, Some(&column)) && !on.can(COLUMN_LIMIT, Some(&column)));
}

#[test]
fn each_operation_needs_its_own_action() {
    let w = Arc::new(FakeWriter::default());
    let a = first();
    let only_quick = module(vec![action("quick-add", CREATE_KEY)], &w);
    assert!(only_quick.can(CREATE, None));
    assert!(!only_quick.can(MOVE, Some(&a)) && !only_quick.can(COLUMN_CREATE, None));
    let wrong = module(
        vec![action("quick-add", "delete"), action("drop", CREATE_KEY)],
        &w,
    );
    assert!(!wrong.can(CREATE, None) && !wrong.can(MOVE, Some(&a)));
}

#[test]
fn a_control_stays_locked_while_the_module_says_nothing_may_be_done() {
    let w = Arc::new(FakeWriter::default());
    let mut silent = data();
    silent.may.clear();
    silent.rows.iter_mut().for_each(|r| r.may.clear());
    let g = gate(described(), Arc::new(records(&w, silent)));
    assert!(!g.can(MOVE, Some(&first())) && !g.can(CREATE, None));
}

#[test]
fn records_that_cannot_keep_every_control_locked_even_with_all_actions() {
    let all = vec![
        action("quick-add", "create"),
        action("drop", "move"),
        action("button", COLUMN_CREATE),
    ];
    let g = gate(all, Arc::new(NoRecords));
    for channel in [MOVE, CREATE, COLUMN_CREATE] {
        assert!(!g.can(channel, None), "{channel} stays locked");
    }
}

#[test]
fn a_card_the_module_does_not_allow_is_refused_locked_and_the_module_is_not_asked() {
    let w = Arc::new(FakeWriter::default());
    let g = module(described(), &w);
    let (b, to) = (second(), other_column(&data(), &second()));
    let refused = g.send(MOVE, Some(&b), &[text("column", &to)]).unwrap_err();
    assert_eq!(refused.code, "locked");
    assert!(w.calls.lock().unwrap().is_empty());
}

#[test]
fn send_refuses_on_its_own_when_the_description_has_no_action() {
    let w = Arc::new(FakeWriter::default());
    let g = module(Vec::new(), &w);
    let refused = g.send(CREATE, None, &[text("title", "x")]).unwrap_err();
    assert_eq!(refused.code, "locked");
    assert!(w.calls.lock().unwrap().is_empty());
    let g = gate(Vec::new(), Arc::new(NoRecords));
    assert!(g.send(COLUMN_CREATE, None, &[text("name", "x")]).is_err());
}

#[test]
fn a_drop_is_one_call_with_the_column_the_place_and_the_lane_of_the_record() {
    let w = Arc::new(FakeWriter::default());
    let g = module(described(), &w);
    let (d, a) = (data(), first());
    let (to, lane) = (other_column(&d, &a), lane_of(&d, &a));
    let (after, before, board) = (field(&d, "after"), field(&d, "before"), field(&d, "lane"));
    let col = field(&d, "column");
    g.send(
        MOVE,
        Some(&a),
        &[text("column", &to), text("after", "c"), text("before", "d")],
    )
    .expect("accepted");
    let calls = w.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command, MOVE_KEY);
    let args: Vec<_> = calls[0]
        .args
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(
        args,
        [
            (col.as_str(), to.as_str()),
            (after.as_str(), "c"),
            (before.as_str(), "d"),
            (board.as_str(), lane.as_str())
        ]
    );
}

#[test]
fn a_drop_at_the_end_of_another_column_sends_no_place_even_when_the_record_has_one() {
    // the fixture card has an order of its own; a drop without a place (the end) must not send the old one
    let w = Arc::new(FakeWriter::default());
    let g = module(described(), &w);
    let (d, a) = (data(), first());
    let (to, lane, board) = (other_column(&d, &a), lane_of(&d, &a), field(&d, "lane"));
    let col = field(&d, "column");
    g.send(MOVE, Some(&a), &[text("column", &to)])
        .expect("accepted");
    let calls = w.calls.lock().unwrap();
    let args: Vec<_> = calls[0]
        .args
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(
        args,
        [(col.as_str(), to.as_str()), (board.as_str(), lane.as_str())]
    );
}

#[test]
fn a_quick_add_creates_with_title_and_column_and_a_refusal_is_given_back_as_its_code() {
    let w = Arc::new(FakeWriter::default());
    let g = module(described(), &w);
    let (d, column) = (data(), column_of(&data(), &first()));
    let (title, field_column) = (field(&d, "title"), field(&d, "column"));
    g.send(
        CREATE,
        None,
        &[text("title", "Ship it"), text("column", &column)],
    )
    .expect("accepted");
    let call = w.calls.lock().unwrap()[0].clone();
    assert_eq!((call.command.as_str(), call.record), (CREATE_KEY, None));
    let args: Vec<_> = call
        .args
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(
        args,
        [
            (title.as_str(), "Ship it"),
            (field_column.as_str(), column.as_str())
        ]
    );
    *w.refuse.lock().unwrap() = Some("host-notice-invoke-refused".into());
    let refused = g
        .send(CREATE, None, &[text("title", "x"), text("column", &column)])
        .unwrap_err();
    assert_eq!(refused.code, "host-notice-invoke-refused");
}

#[test]
fn records_that_bind_no_command_keep_the_gesture_locked_and_say_so_when_sent() {
    let all = vec![action("drop", MOVE), action("button", COLUMN_CREATE)];
    let g = gate(all, Arc::new(NoRecords));
    assert!(!g.can(MOVE, Some("k")) && !g.can(COLUMN_CREATE, None));
    let refused = g
        .send(MOVE, Some("k"), &[text("column", "done")])
        .unwrap_err();
    assert_eq!(refused.code, "locked");
}

#[test]
fn an_action_that_calls_another_command_than_the_module_bound_to_the_gesture_is_warned() {
    let w = Arc::new(FakeWriter::default());
    let r = records(&w, data());
    assert_eq!(crate::action_report(&view_of(described()), &r), Vec::new());
    let wrong = vec![
        action("drop", "move"),
        action("quick-add", CREATE_KEY),
        action("button", "anything"),
    ];
    let said = crate::action_report(&view_of(wrong), &r);
    assert_eq!(said.len(), 1);
    assert_eq!(said[0].code.code, "action-channel-mismatch");
    assert!(!said[0].red);
}

/// A view with one page whose place `PLACE` holds a board and the given actions.
fn view_of(actions: Vec<ActionView>) -> crate::view::View {
    crate::view::View {
        actions,
        pages: vec![crate::view::PageView {
            page: "board".into(),
            seq: 0,
            frame: "workbench".into(),
            locks: Vec::new(),
            places: vec![crate::view::PlaceView {
                place: "main".into(),
                seq: 0,
                block: "board".into(),
                title: None,
                locks: Vec::new(),
                slot_locks: Vec::new(),
                info: crate::place_info::PlaceInfo::default(),
                elem: PLACE.into(),
                embed: None,
            }],
        }],
        ..crate::view::View::default()
    }
}
