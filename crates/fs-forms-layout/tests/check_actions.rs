//! The check of the gestures an `[[action]]` binds (`check_actions.rs`): with `at` the trigger is the name of a command slot of the
//! block at that place. One passing case and one counter-test per rule (each goes red when its rule is removed).
//!
//! Pattern: none (tests); the catalog is a small Strategy of its own (`Cat`).

mod support;
use fs_forms_layout::*;
use support::*;

/// A block `board` with the slots `title` (a field), `drop` (a command with args, once), `menu` (a command, twice) and `quick-add`
/// (a command); every other block and slot is unknown.
struct Cat;
impl CatalogRules for Cat {
    fn block_known(&self, b: &str) -> bool {
        b == "board"
    }
    fn slot_known(&self, _: &str, slot: &str) -> bool {
        ["title", "drop", "menu", "quick-add"].contains(&slot)
    }
    fn slot_repeat(&self, _: &str, slot: &str) -> u32 {
        if slot == "menu" { 2 } else { 1 }
    }
    fn command_args(&self, _: &str, slot: &str) -> Option<Vec<String>> {
        match slot {
            "drop" => Some(vec!["column".into()]),
            "menu" | "quick-add" => Some(Vec::new()),
            _ => None,
        }
    }
}

fn action(at: Option<&str>, trigger: Trigger) -> Item<ActionDoc> {
    let mut a = item(
        "action:x",
        ActionDoc {
            trigger,
            at: at.map(|e| ElemId::parse(e).expect("id")),
            target: ActionTarget::Command {
                source: 1,
                command: "card.move".into(),
            },
            confirm: None,
        },
    );
    a.elem = Some(ElemId::parse(&format!("action:{}", trigger.word())).expect("id"));
    a
}

fn causes(actions: Vec<Item<ActionDoc>>) -> Vec<String> {
    let mut d = doc("board");
    d.sources.push(item(
        "source:1",
        SourceDoc {
            range: "planning@>=0.0.1, <0.1.0".into(),
            data_type: "Task".into(),
            scope: None,
        },
    ));
    d.actions = actions;
    d.check(&Cat, &Limits::declared())
        .err()
        .unwrap_or_default()
        .iter()
        .filter(|c| c.code == LayoutCode::ActionInvalid)
        .map(|c| {
            format!(
                "{}:{}",
                c.param("cause").unwrap_or(""),
                c.param("slot")
                    .or(c.param("trigger"))
                    .or(c.param("at"))
                    .unwrap_or("")
            )
        })
        .collect()
}

const MAIN: &str = "block:board/main";

#[test]
fn a_gesture_at_a_place_whose_block_has_that_command_slot_passes() {
    assert_eq!(
        causes(vec![action(Some(MAIN), Trigger::Drop)]),
        Vec::<String>::new()
    );
    assert_eq!(
        causes(vec![action(Some(MAIN), Trigger::QuickAdd)]),
        Vec::<String>::new()
    );
}

#[test]
fn a_page_trigger_needs_no_place() {
    assert_eq!(
        causes(vec![action(None, Trigger::Button)]),
        Vec::<String>::new()
    );
    assert_eq!(
        causes(vec![action(None, Trigger::Submit)]),
        Vec::<String>::new()
    );
}

#[test]
fn a_gesture_without_at_is_refused() {
    assert_eq!(
        causes(vec![action(None, Trigger::Drop)]),
        ["at-missing:drop"]
    );
}

#[test]
fn an_at_that_names_no_place_with_a_block_is_refused() {
    assert_eq!(
        causes(vec![action(Some("block:nowhere/x"), Trigger::Drop)]),
        ["at-unknown:block:nowhere/x"]
    );
}

#[test]
fn a_trigger_that_is_no_slot_of_the_block_is_refused() {
    // `activate` is no slot of this block (slot-invalid in words of the contract)
    assert_eq!(
        causes(vec![action(Some(MAIN), Trigger::Activate)]),
        ["slot-unknown:activate"]
    );
    // a page trigger with `at` names a slot called `button`: there is none
    assert_eq!(
        causes(vec![action(Some(MAIN), Trigger::Button)]),
        ["slot-unknown:button"]
    );
}

#[test]
fn a_slot_that_is_no_command_slot_is_refused() {
    // `title` is a slot of the block, but a field slot; no trigger word is `title`, so the rule is shown with a fake catalog entry
    struct TitleIsMenu;
    impl CatalogRules for TitleIsMenu {
        fn block_known(&self, _: &str) -> bool {
            true
        }
        fn slot_known(&self, _: &str, _: &str) -> bool {
            true
        }
        fn slot_repeat(&self, _: &str, _: &str) -> u32 {
            1
        }
        // knows the slots but none is a command slot
    }
    let mut d = doc("board");
    d.sources.push(item(
        "source:1",
        SourceDoc {
            range: "planning@>=0.0.1, <0.1.0".into(),
            data_type: "Task".into(),
            scope: None,
        },
    ));
    d.actions = vec![action(Some(MAIN), Trigger::Drop)];
    let got = d.check(&TitleIsMenu, &Limits::declared()).err().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].param("cause"), Some("slot-not-command"));
}

#[test]
fn more_actions_than_the_repeat_of_the_slot_are_refused() {
    let a = |n: &str| {
        let mut x = action(Some(MAIN), Trigger::Menu);
        x.elem = Some(ElemId::parse(n).expect("id"));
        x
    };
    assert_eq!(
        causes(vec![a("action:1"), a("action:2")]),
        Vec::<String>::new()
    );
    assert_eq!(
        causes(vec![a("action:1"), a("action:2"), a("action:3")]),
        ["repeat-exceeded:menu"]
    );
    // `drop` takes one
    let d = |n: &str| {
        let mut x = action(Some(MAIN), Trigger::Drop);
        x.elem = Some(ElemId::parse(n).expect("id"));
        x
    };
    assert_eq!(
        causes(vec![d("action:1"), d("action:2")]),
        ["repeat-exceeded:drop"]
    );
}
