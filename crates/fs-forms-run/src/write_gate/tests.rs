//! The locks of the write gate for the gestures of a card board (`may_fire`, `fire`) and for the three dialogs on a column (`can`,
//! `send`, the channel way that stays until G8b): a control is unlocked only when the description has an action for it AND the records
//! can do it; the module's own command goes to the module, with exactly what the gesture carries; anything else is refused and
//! nobody is asked. There is no channel way for a card move or a new card any more.
//!
//! Tests: description without action, records that cannot, one action per gesture, the key of the contract against the channel
//! name, a drop and a quick-add as ONE call with what the gesture carries, no old place at the end of a column, a refusal given back
//! as its code, `fire` refuses on its own, records that bind no command (locked, the module is not asked), an action that calls
//! another command than the module bound to its gesture is warned.
//!
//! Pattern: none (tests).

use std::sync::Arc;

use fs_components_core::form_write::{FormWrite, Gesture, WriteArg, WriteValue, trigger};
use fs_components_kanban::{COLUMN_CREATE, COLUMN_LIMIT, COLUMN_RENAME};

use super::WriteGate;
use crate::records::{NoRecords, Records};
use crate::view::ActionView;
use crate::write::fixture::{
    CREATE as CREATE_KEY, FakeWriter, MOVE as MOVE_KEY, column_of, data, field, first, records,
    second,
};

const DROP: Gesture<'static> = Gesture::new(trigger::DROP);
const QUICK_ADD: Gesture<'static> = Gesture::new(trigger::QUICK_ADD);
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
        for gesture in [DROP, QUICK_ADD] {
            assert!(g.may_fire(gesture, Some(&first())).is_err());
        }
        for channel in [COLUMN_CREATE, COLUMN_RENAME, COLUMN_LIMIT] {
            assert!(!g.can(channel, None), "{channel} stays locked");
        }
    }
}

#[test]
fn a_gesture_is_unlocked_by_the_key_of_the_contract_the_module_says_may_be_used() {
    let w = Arc::new(FakeWriter::default());
    let on = module(described(), &w);
    let (a, b) = (first(), second());
    assert!(on.may_fire(DROP, Some(&a)).is_ok() && on.may_fire(QUICK_ADD, None).is_ok());
    // the description names the CHANNEL, not the key of the contract: the module binds no channel to it, the gesture stays locked
    let by_channel = module(
        vec![action("drop", "move"), action("quick-add", "create")],
        &w,
    );
    assert!(by_channel.may_fire(DROP, Some(&a)).is_err());
    assert!(by_channel.may_fire(QUICK_ADD, None).is_err());
    // rename and limit are not channels of this form: locked
    let column = column_of(&data(), &b);
    assert!(!on.can(COLUMN_RENAME, Some(&column)) && !on.can(COLUMN_LIMIT, Some(&column)));
}

#[test]
fn each_gesture_needs_its_own_action() {
    let w = Arc::new(FakeWriter::default());
    let a = first();
    let only_quick = module(vec![action("quick-add", CREATE_KEY)], &w);
    assert!(only_quick.may_fire(QUICK_ADD, None).is_ok());
    assert!(only_quick.may_fire(DROP, Some(&a)).is_err() && !only_quick.can(COLUMN_CREATE, None));
    // an action that names a command no channel of the form is bound to leaves the gesture locked
    let wrong = module(vec![action("quick-add", "delete")], &w);
    assert!(wrong.may_fire(QUICK_ADD, None).is_err());
}

#[test]
fn a_gesture_stays_locked_while_the_module_says_nothing_may_be_done() {
    let w = Arc::new(FakeWriter::default());
    let mut silent = data();
    silent.may.clear();
    silent.rows.iter_mut().for_each(|r| r.may.clear());
    let g = gate(described(), Arc::new(records(&w, silent)));
    assert!(g.may_fire(DROP, Some(&first())).is_err() && g.may_fire(QUICK_ADD, None).is_err());
}

#[test]
fn records_that_cannot_keep_every_control_locked_even_with_all_actions() {
    let all = vec![
        action("quick-add", "create"),
        action("drop", "move"),
        action("button", COLUMN_CREATE),
    ];
    let g = gate(all, Arc::new(NoRecords));
    assert!(g.may_fire(DROP, Some("k")).is_err() && g.may_fire(QUICK_ADD, None).is_err());
    assert!(!g.can(COLUMN_CREATE, None), "column create stays locked");
}

#[test]
fn fire_refuses_on_its_own_when_the_description_has_no_action() {
    let w = Arc::new(FakeWriter::default());
    let g = module(Vec::new(), &w);
    let refused = g.fire(QUICK_ADD, None, &[text("title", "x")]).unwrap_err();
    assert_eq!(refused.code, "gesture-unbound");
    assert!(w.calls.lock().unwrap().is_empty());
    let g = gate(Vec::new(), Arc::new(NoRecords));
    assert!(g.send(COLUMN_CREATE, None, &[text("name", "x")]).is_err());
}

#[test]
fn a_quick_add_creates_with_title_and_column_and_a_refusal_is_given_back_as_its_code() {
    let w = Arc::new(FakeWriter::default());
    let g = module(described(), &w);
    let (d, column) = (data(), column_of(&data(), &first()));
    let (title, field_column) = (field(&d, "title"), field(&d, "column"));
    g.fire(
        QUICK_ADD,
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
        .fire(
            QUICK_ADD,
            None,
            &[text("title", "x"), text("column", &column)],
        )
        .unwrap_err();
    assert_eq!(refused.code, "host-notice-invoke-refused");
}

#[test]
fn records_that_bind_no_command_keep_the_gesture_locked_and_say_so_when_fired() {
    let all = vec![action("drop", MOVE_KEY), action("button", COLUMN_CREATE)];
    let g = gate(all, Arc::new(NoRecords));
    assert!(g.may_fire(DROP, Some("k")).is_err() && !g.can(COLUMN_CREATE, None));
    let refused = g
        .fire(DROP, Some("k"), &[text("column", "done")])
        .unwrap_err();
    assert_eq!(refused.code, "gesture-unbound");
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
