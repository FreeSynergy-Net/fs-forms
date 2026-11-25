//! The digest (A-P2): fixed bytes (golden values), and every part of the plan takes part in it.
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn hex(d: [u8; 32]) -> String {
    use std::fmt::Write;
    d.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").unwrap();
        s
    })
}

/// A plan with notices, menu locks, a locked source and a locked placement.
fn rich() -> Plan {
    let mut r = tasks_resolved();
    r.warnings
        .push(problem("pl-board-main", "layout-error-warn"));
    r.problems.push(problem("m-note", "layout-error-menu"));
    // A code without a known prefix (K8): its digest name `unknown` is part of the golden value.
    r.problems.push(problem_in(
        "pl-detail-props",
        "oops-no-prefix",
        Family::Unknown,
    ));
    // `type_may = false` (no type right) and an unsupported, writing, record command: every flag of a command takes part.
    let mut c = cmd("dup", false, true, true);
    c.desc.type_may = false;
    r.menu[1].children.push(MenuIn {
        elem: "m-dup".into(),
        kind: MenuKind::Command {
            command: c,
            on_selected: true,
            shortcut: Some("ctrl+n".into()),
        },
        children: vec![],
    });
    build_with(&r, &TestCatalog::new(), &FakeFacts(vec![]), host())
}

#[test]
fn golden_digest_of_the_tasks_plan() {
    // Source numbers count from 1 now: the fixture input changed (source 0 -> 1), the canon encoding did not. A5 F6: the digest changed ON PURPOSE (a node is a variant `block`/`frame`, `set` is a field); tests/tree.rs holds the counter-proofs.
    assert_eq!(
        hex(tasks_plan().digest_without_style()),
        "76b6800491b1a33e81d9d754eccf1fa01e56094ab2e2b7d065cd22d88f657c4e"
    );
}

#[test]
fn golden_digest_of_a_plan_with_notices_and_locks() {
    let p = rich();
    assert!(
        p.locks()
            .iter()
            .any(|l| matches!(l.cause, Cause::Unknown(_))),
        "K8 code in the plan"
    );
    assert!(!p.notices.is_empty() && !p.menu.locks.is_empty() && p.sources[0].status.is_err());
    assert_eq!(
        hex(p.digest_without_style()),
        "6dd09f9f0ec37cbaa7de81dbb7202d44754f8f42db9d01556e29084d4a2fc21f"
    );
}

/// The wire format of A-P2: `label (u16 BE) + length (u32 BE) + payload`, numbers u64 BE, in declaration order.
#[test]
fn first_fields_follow_the_a_p2_encoding() {
    let p = tasks_plan();
    let b = p.canonical_bytes_without_style();
    let key = p.key.as_bytes();
    assert_eq!(&b[0..2], &[0, 0]);
    assert_eq!(&b[2..6], &u32::try_from(key.len()).unwrap().to_be_bytes());
    assert_eq!(&b[6..6 + key.len()], key);
    let at = 6 + key.len();
    assert_eq!(&b[at..at + 6], &[0, 1, 0, 0, 0, 8]);
    assert_eq!(&b[at + 6..at + 14], &p.stamp.to_be_bytes());
}

#[test]
fn every_part_changes_the_digest() {
    let base = rich();
    let d = base.digest_without_style();
    let mut p = base.clone();
    p.notices.pop();
    assert_ne!(p.digest_without_style(), d, "notices");
    let mut p = base.clone();
    p.menu.locks.pop();
    assert_ne!(p.digest_without_style(), d, "menu locks");
    let mut p = base.clone();
    p.state = PlanState::Refused;
    assert_ne!(p.digest_without_style(), d, "state");
    let mut p = base.clone();
    p.keys.reserved.pop();
    assert_ne!(p.digest_without_style(), d, "reserved keys");
    let mut p = base.clone();
    p.stamp += 1;
    assert_ne!(p.digest_without_style(), d, "stamp");
    let mut p = base.clone();
    p.style = Some("x".into());
    assert_eq!(p.digest_without_style(), d, "style is not part of it");
    assert_ne!(tasks_plan().digest_without_style(), d);
}
