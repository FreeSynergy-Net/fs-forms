//! The heads of a grouping (lanes): `Grouping{name, heads}`, the digest only when set, the rule `bind-error-heads-form-missing`.
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

/// Facts with the forms the module of a capability declares.
struct WithForms(FakeFacts, Vec<ModuleForm>);

impl ProviderFacts for WithForms {
    fn offer(&self, capability: &str) -> Offer {
        self.0.offer(capability)
    }
    fn forms(&self, _capability: &str) -> Vec<ModuleForm> {
        self.1.clone()
    }
}

fn entry(name: &str, minor: u32, data_type: &str) -> ModuleForm {
    ModuleForm {
        form: FormRef {
            name: name.into(),
            major: 1,
            minor,
        },
        data_type: data_type.into(),
        map: vec![],
        fields: vec![],
    }
}

fn catalog() -> TestCatalog {
    let mut c = TestCatalog::new();
    let board = c.blocks.iter_mut().find(|b| b.name == "board").unwrap();
    board.slots.push(SlotSpec {
        name: "lanes".into(),
        min: 0,
        max: Some(1),
        heads: vec![FormRef {
            name: "rows".into(),
            major: 1,
            minor: 0,
        }],
    });
    c
}

/// The tasks layout with a second source (the lane list) and a `lanes` grouping on the board.
fn lanes_layout(heads: Option<u32>) -> Resolved {
    let mut r = tasks_resolved();
    r.sources.push(SourceIn {
        elem: "s1".into(),
        capability: "planning".into(),
        range: Range {
            min: (0, 0, 0),
            max: (0, 1, 0),
        },
        data_type: "Lane".into(),
    });
    let place = r.pages[0].places[0].clone();
    let NodeIn::Block(mut p) = place else {
        panic!("a block")
    };
    p.bindings.push(BindingIn {
        elem: "b-lanes".into(),
        slot: "lanes".into(),
        nth: 0,
        kind: BindKind::Grouping {
            name: "assignee".into(),
            heads,
        },
    });
    r.pages[0].places[0] = NodeIn::Block(p);
    r
}

fn facts(planning: bool, forms: Vec<ModuleForm>) -> WithForms {
    let mut v = vec![("tasks".to_owned(), Offer::Reached(vec![provider(0, 2)]))];
    if planning {
        v.push(("planning".to_owned(), Offer::Reached(vec![provider(0, 0)])));
    }
    WithForms(FakeFacts(v), forms)
}

fn lanes_slot(p: &Plan) -> SlotFill {
    match &p.place("board", "main").expect("place").status {
        PlaceStatus::Ready(r) => r.slots.iter().find(|s| s.slot == "lanes").cloned().unwrap(),
        PlaceStatus::Locked(l) => panic!("the block must live: {l:?}"),
    }
}

fn build(r: &Resolved, f: &dyn ProviderFacts) -> Plan {
    build_with(r, &catalog(), f, host())
}

#[test]
fn a_form_entry_with_the_type_of_the_head_source_keeps_the_slot_open() {
    let f = facts(
        true,
        vec![entry("rows", 1, "Task"), entry("rows", 1, "Lane")],
    );
    let p = build(&lanes_layout(Some(2)), &f);
    assert!(lanes_slot(&p).lock.is_none(), "{:?}", lanes_slot(&p).lock);
    assert!(matches!(
        lanes_slot(&p).fill,
        Fill::Grouping { heads: Some(2), .. }
    ));
}

#[test]
fn without_an_entry_of_that_type_the_slot_is_locked_and_the_block_lives() {
    // The module declares rows@1.0 only for `Task`: the type decides, not the form name.
    let f = facts(true, vec![entry("rows", 0, "Task")]);
    let p = build(&lanes_layout(Some(2)), &f);
    let l = lanes_slot(&p).lock.expect("slot lock");
    assert_eq!(code_of(&l), "bind-error-heads-form-missing");
    let c = l.cause.coded();
    assert!(c.params.contains(&("form".into(), "rows@1.0".into())));
    assert!(c.params.contains(&("type".into(), "Lane".into())));
    assert!(matches!(l.part, Part::Slot { .. }));
    // The producer (bind) says what the lock is about; the host filters by this field, not by the code text.
    assert_eq!(c.slot_role(), Some(SlotRole::Heads));
    assert!(
        locks_of(&p, "board", "main").is_empty(),
        "the placement lives"
    );
}

#[test]
fn the_form_name_and_the_minor_version_must_fit() {
    let slot_wants_1_0 = facts(true, vec![entry("rows", 0, "Lane")]);
    assert!(
        lanes_slot(&build(&lanes_layout(Some(2)), &slot_wants_1_0))
            .lock
            .is_none()
    );
    let other_name = facts(true, vec![entry("tree", 1, "Lane")]);
    assert!(
        lanes_slot(&build(&lanes_layout(Some(2)), &other_name))
            .lock
            .is_some()
    );
}

#[test]
fn a_locked_head_source_locks_only_the_slot() {
    // `planning` is not installed: the lanes slot shows the cause of the source; the block and the other slots live.
    let f = facts(false, vec![entry("rows", 1, "Lane")]);
    let p = build(&lanes_layout(Some(2)), &f);
    let l = lanes_slot(&p).lock.expect("slot lock");
    assert_eq!(code_of(&l), "accepts-error-contract-missing");
    assert!(matches!(l.part, Part::Slot { .. }));
    assert_eq!(locks_of(&p, "board", "main"), vec![]);
    let PlaceStatus::Ready(r) = &p.place("board", "main").unwrap().status else {
        panic!("ready")
    };
    assert!(
        r.slots
            .iter()
            .filter(|s| s.slot != "lanes")
            .all(|s| s.lock.is_none())
    );
}

#[test]
fn an_unknown_head_source_is_a_slot_lock() {
    let f = facts(true, vec![]);
    let p = build(&lanes_layout(Some(9)), &f);
    let l = lanes_slot(&p).lock.expect("slot lock");
    assert_eq!(code_of(&l), "layout-error-source-unknown");
}

#[test]
fn the_digest_takes_the_heads_only_when_they_are_set() {
    let f = facts(true, vec![entry("rows", 1, "Lane")]);
    let none = build(&lanes_layout(None), &f);
    let some = build(&lanes_layout(Some(2)), &f);
    assert!(lanes_slot(&none).lock.is_none());
    assert_ne!(none.digest_without_style(), some.digest_without_style());
    // Equal inputs, equal plans (the field is part of the digest, nothing else moves).
    assert_eq!(
        some.digest_without_style(),
        build(&lanes_layout(Some(2)), &f).digest_without_style()
    );
}
