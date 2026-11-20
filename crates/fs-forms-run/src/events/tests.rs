//! The gestures of the board on the sample records: what the description allows works, everything else stays locked.
//!
//! Tests: a card added in the quick-add row shows in "To do"; a card moved by the buttons changes column; a column is asked by the
//! input dialog and appears; without an action in the description, without records that can, and for rename and limit nothing is
//! unlocked and nothing changes. Mutants (run by hand, all red): `supports` always true (4 tests); `supports` without
//! `records.can` (1 test); the lock check dropped from the event `add_card` (1 test: the board asks first, so only a direct call of
//! the event sees it).
//!
//! Pattern: none (tests).

use std::sync::Arc;

use fs_components_fields::{Look, Say};
use fs_components_kanban::{BoardSource, Column, KanbanBoard, Operation};
use fs_components_styles::StyleId;
use fs_components_widgets::dialogs::Labels;

use super::{Ask, Wiring};
use crate::records::{NoRecords, Records};
use crate::sample::SampleBoard;
use crate::view::ActionView;

fn action(trigger: &str, command: &str) -> ActionView {
    ActionView {
        trigger: trigger.into(),
        command: command.into(),
    }
}

fn all_actions() -> Vec<ActionView> {
    vec![
        action("quick-add", "create"),
        action("drop", "move"),
        action("button", "column.create"),
    ]
}

fn wiring(actions: Vec<ActionView>, records: Arc<dyn Records>) -> Arc<Wiring> {
    Arc::new(Wiring {
        actions: Arc::new(actions),
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

fn titles(cols: &[Column], key: &str) -> Vec<String> {
    cols.iter()
        .find(|c| c.key == key)
        .map(|c| c.cards.iter().map(|x| x.title.clone()).collect())
        .unwrap_or_default()
}

fn keys(cols: &[Column], key: &str) -> Vec<String> {
    cols.iter()
        .find(|c| c.key == key)
        .map(|c| c.cards.iter().map(|x| x.key.clone()).collect())
        .unwrap_or_default()
}

#[test]
fn a_card_typed_in_the_quick_add_row_shows_in_to_do_and_the_row_empties() {
    let w = wiring(all_actions(), Arc::new(SampleBoard::default()));
    let b = board(&w);
    b.set_quick_text("todo", "  Ship it ");
    assert!(b.submit_quick("todo"));
    let cols = w.records.columns();
    assert_eq!(
        titles(&cols, "todo"),
        ["Write the spec", "Review the plan", "Ship it"]
    );
    assert_eq!(keys(&cols, "todo").last().map(String::as_str), Some("FS-6"));
    assert_eq!(b.quick_text("todo"), "");
    assert_eq!(b.snapshot().notice, None);
}

#[test]
fn a_card_moved_by_the_button_changes_its_column() {
    let w = wiring(all_actions(), Arc::new(SampleBoard::default()));
    let b = board(&w);
    b.step_over("FS-1", 1);
    let cols = w.records.columns();
    assert_eq!(keys(&cols, "todo"), ["FS-2"]);
    assert_eq!(keys(&cols, "doing"), ["FS-3", "FS-4", "FS-1"]);
    // grab, hover, drop: the drop gesture ends in the same request
    b.grab("FS-5");
    b.hover("todo");
    b.drop_held();
    assert_eq!(keys(&w.records.columns(), "todo"), ["FS-2", "FS-5"]);
}

#[test]
fn a_column_is_asked_by_the_input_dialog_and_appears_when_confirmed() {
    let w = wiring(all_actions(), Arc::new(SampleBoard::default()));
    let b = board(&w);
    assert!(w.ask.current().is_none());
    b.request(Operation::AddColumn, None);
    let dialog = w.ask.current().expect("the dialog is open");
    assert_eq!(w.records.columns().len(), 3, "nothing yet");
    let _ = dialog.clone().initial("  Review ");
    assert!(dialog.confirm());
    let cols = w.records.columns();
    assert_eq!(cols.len(), 4);
    assert_eq!(cols[3].title, "Review");
    assert!(w.ask.current().is_none(), "the dialog closes");
}

#[test]
fn a_name_that_exists_keeps_the_dialog_open_and_cancel_leaves() {
    let w = wiring(all_actions(), Arc::new(SampleBoard::default()));
    let b = board(&w);
    b.request(Operation::AddColumn, None);
    let dialog = w.ask.current().unwrap();
    let _ = dialog.clone().initial("To do");
    assert!(dialog.confirm());
    assert_eq!(w.records.columns().len(), 3);
    assert!(w.ask.current().is_some(), "still open to type another name");
    dialog.cancel();
    assert!(w.ask.current().is_none());
}

#[test]
fn without_an_action_in_the_description_everything_stays_locked() {
    for actions in [Vec::new(), vec![action("menu", "delete")]] {
        let w = wiring(actions, Arc::new(SampleBoard::default()));
        for op in Operation::ALL {
            assert!(!w.supports(op), "{op:?} stays locked");
        }
        let b = board(&w);
        b.set_quick_text("todo", "Sneaky");
        assert!(!b.submit_quick("todo"));
        assert_eq!(b.snapshot().notice.as_deref(), Some("locked"));
        b.step_over("FS-1", 1);
        b.request(Operation::AddColumn, None);
        assert!(w.ask.current().is_none());
        let cols = w.records.columns();
        assert_eq!(keys(&cols, "todo"), ["FS-1", "FS-2"]);
        assert_eq!(cols.len(), 3);
    }
}

#[test]
fn the_events_refuse_on_their_own_when_called_without_the_board_asking_first() {
    use fs_components_kanban::{Answer, BoardEvents};
    let w = wiring(Vec::new(), Arc::new(SampleBoard::default()));
    assert_eq!(w.add_card("todo", "x"), Answer::Refused("locked".into()));
    assert_eq!(
        w.move_card("FS-1", "todo", "done"),
        Answer::Refused("locked".into())
    );
    w.add_column();
    assert!(w.ask.current().is_none());
    let cols = w.records.columns();
    assert_eq!(keys(&cols, "todo"), ["FS-1", "FS-2"]);
    assert_eq!(keys(&cols, "done"), ["FS-5"]);
}

#[test]
fn records_that_cannot_keep_every_control_locked_even_with_all_actions() {
    let w = wiring(all_actions(), Arc::new(NoRecords));
    for op in Operation::ALL {
        assert!(!w.supports(op), "{op:?} stays locked");
    }
    assert!(!board(&w).add_card("todo", "x"));
}

#[test]
fn rename_and_limit_stay_locked_because_nothing_can_do_them() {
    let w = wiring(all_actions(), Arc::new(SampleBoard::default()));
    assert!(!w.supports(Operation::Rename));
    assert!(!w.supports(Operation::Limit));
    for op in [Operation::AddCard, Operation::Move, Operation::AddColumn] {
        assert!(w.supports(op), "{op:?} is unlocked");
    }
}

#[test]
fn each_operation_needs_its_own_action() {
    let only_quick = wiring(
        vec![action("quick-add", "create")],
        Arc::new(SampleBoard::default()),
    );
    assert!(only_quick.supports(Operation::AddCard));
    assert!(!only_quick.supports(Operation::Move));
    assert!(!only_quick.supports(Operation::AddColumn));
    let wrong_command = wiring(
        vec![action("quick-add", "delete"), action("drop", "create")],
        Arc::new(SampleBoard::default()),
    );
    assert!(!wrong_command.supports(Operation::AddCard));
    assert!(!wrong_command.supports(Operation::Move));
}
