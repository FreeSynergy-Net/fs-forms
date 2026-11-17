//! K54, K55, K79, K80, K81, K82, K84 and overlay rules on the memory store.
mod support;

use fs_forms_layout::*;
use support::*;

fn text_key(r: &Resolved) -> Option<String> {
    r.layout.doc().pages[0].v.blocks[0].v.text_key.clone()
}
fn with_schemes(frontend: &str) -> LayoutDoc {
    let mut d = doc(frontend);
    d.pages[0].v.blocks[0].v.bindings[0].v.schemes =
        Some(vec![item("scheme:b/0", "https".to_owned())]);
    d
}

#[test]
fn k54_overlay_acts_and_the_original_stays() {
    let s = store();
    let id = install(&s, doc("board"), "1.0");
    let digest = block_on(s.original(&id)).unwrap().unwrap().0.digest();
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    let r = block_on(s.replace_ops(&o, Rev(1), vec![set_text(Field::TextKey, "mine")], &who()))
        .unwrap();
    assert!(r.problems.is_empty() && r.conflicts.is_empty());
    let res = block_on(s.resolve(&key("board"))).unwrap().unwrap();
    assert_eq!(text_key(&res), Some("mine".to_owned()));
    assert_eq!(res.provenance.len(), 1);
    assert_ne!(res.effective_digest, digest);
    assert_eq!(
        block_on(s.original(&id)).unwrap().unwrap().0.digest(),
        digest
    );
}

#[test]
fn k80_the_store_sets_expect_from_the_original() {
    let s = store();
    let id = install(&s, doc("board"), "1.0");
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    block_on(s.replace_ops(&o, Rev(1), vec![set_text(Field::TextKey, "mine")], &who())).unwrap();
    let ov = block_on(s.overlay(&o)).unwrap().unwrap();
    assert!(matches!(
        &ov.ops[0].kind,
        OpKind::Set {
            expect: Value::None,
            ..
        }
    ));
}

#[test]
fn k55_changed_original_gives_a_kept_conflict() {
    let s = store();
    let id = install(&s, doc("board"), "1.0");
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    block_on(s.replace_ops(&o, Rev(1), vec![set_text(Field::TextKey, "mine")], &who())).unwrap();
    let mut v2 = doc("board");
    v2.pages[0].v.blocks[0].v.text_key = Some("theirs".into());
    let r = block_on(s.install(v2, &pkg("2.0"), Mode::Apply, &who())).unwrap();
    assert_eq!(r.conflicts[0].cause, Cause::TargetChanged);
    let res = block_on(s.resolve(&key("board"))).unwrap().unwrap();
    assert_eq!(text_key(&res), Some("theirs".to_owned())); // the conflicting op has no effect
    assert_eq!(res.conflicts.len(), 1);
    assert_eq!(block_on(s.overlay(&o)).unwrap().unwrap().ops.len(), 1); // op stays stored
    // k84: keep follows the new value
    block_on(s.resolve_conflict(&o, 0, "2.0", Resolution::Keep, &who())).unwrap();
    assert!(block_on(s.conflicts(Some(&id), true)).unwrap().is_empty());
    let o2 = block_on(s.overlay(&o)).unwrap().unwrap();
    assert!(matches!(&o2.ops[0].kind, OpKind::Set { expect: Value::Text(t), .. } if t == "theirs"));
}

#[test]
fn k84_drop_removes_the_op_and_is_logged() {
    let s = store();
    let id = install(&s, doc("board"), "1.0");
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    block_on(s.replace_ops(&o, Rev(1), vec![set_text(Field::TextKey, "mine")], &who())).unwrap();
    let mut v2 = doc("board");
    v2.pages[0].v.blocks[0].v.text_key = Some("theirs".into());
    block_on(s.install(v2, &pkg("2.0"), Mode::Apply, &who())).unwrap();
    block_on(s.resolve_conflict(&o, 0, "2.0", Resolution::Drop, &who())).unwrap();
    assert!(block_on(s.overlay(&o)).unwrap().unwrap().ops.is_empty());
    assert_eq!(
        block_on(s.log(None, 100)).unwrap().last().unwrap().action,
        LogAction::ConflictResolve
    );
}

#[test]
fn k81_removing_the_last_scheme_is_result_invalid() {
    let s = store();
    let id = install(&s, with_schemes("board"), "1.0");
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    let gone = OpKind::Remove {
        target: ElemRef {
            kind: ElemKind::BindingScheme,
            elem: elem("scheme:b/0"),
        },
    };
    let r = block_on(s.replace_ops(&o, Rev(1), vec![gone], &who())).unwrap();
    assert_eq!(r.conflicts[0].cause, Cause::ResultInvalid);
    let res = block_on(s.resolve(&key("board"))).unwrap().unwrap();
    assert_eq!(
        res.layout.bindings("board", "main")[0].schemes,
        Some(vec!["https".to_owned()])
    );
}

#[test]
fn unknown_target_and_field_are_refused_and_nothing_is_stored() {
    let s = store();
    let id = install(&s, doc("board"), "1.0");
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    let ghost = OpKind::Remove {
        target: ElemRef {
            kind: ElemKind::Block,
            elem: elem("block:nope"),
        },
    };
    let r = block_on(s.replace_ops(&o, Rev(1), vec![ghost], &who())).unwrap();
    assert_eq!(r.problems[0].code, LayoutCode::ElemUnknown);
    let r = block_on(s.replace_ops(&o, Rev(1), vec![set_text(Field::Style, "x")], &who())).unwrap();
    assert_eq!(r.problems[0].param("cause"), Some("field-unknown"));
    assert_eq!(block_on(s.overlay(&o)).unwrap().unwrap().rev, Rev(1));
}

#[test]
fn overlays_only_on_package_originals() {
    let s = store();
    let id = block_on(s.create(doc("board"), &who()))
        .unwrap()
        .layout
        .unwrap();
    assert_eq!(
        block_on(s.create_overlay(&id, None, None, &who()))
            .unwrap_err()
            .code,
        LayoutCode::NotOverlayable
    );
}

#[test]
fn overlay_count_follows_the_limit() {
    let t = MemoryLayoutStore::new(
        Box::new(OpenCatalog),
        Limits {
            overlays_max: 1,
            ..Limits::declared()
        },
        who().actor,
    );
    let id = install(&t, doc("board"), "1.0");
    block_on(t.create_overlay(&id, None, None, &who())).unwrap();
    let e = block_on(t.create_overlay(&id, None, None, &who())).unwrap_err();
    assert_eq!(
        (e.code, e.param("by")),
        (LayoutCode::TooMany, Some("default"))
    );
}

#[test]
fn k79_instance_before_person_whatever_the_creation_order() {
    let s = store().as_instance();
    let id = install(&s, doc("board"), "1.0");
    let person = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    block_on(s.replace_ops(
        &person,
        Rev(1),
        vec![set_text(Field::TextKey, "person")],
        &who(),
    ))
    .unwrap();
    let inst = block_on(s.create_instance_overlay(&id, None, None, &who())).unwrap();
    block_on(s.replace_ops(
        &inst,
        Rev(1),
        vec![set_text(Field::TextKey, "instance")],
        &who(),
    ))
    .unwrap();
    // the person op expects None but the instance op acts first -> target-changed: the instance value stays
    let res = block_on(s.resolve(&key("board"))).unwrap().unwrap();
    assert_eq!(text_key(&res), Some("instance".to_owned()));
    assert_eq!(res.layers.len(), 2);
}

#[test]
fn k82_locks_need_the_instance_service_and_a_reason() {
    let person_only = store();
    let id = install(&person_only, doc("board"), "1.0");
    let e = block_on(person_only.lock(&id, None, "why", &who())).unwrap_err();
    assert_eq!(
        (e.code, e.param("what")),
        (LayoutCode::Unsupported, Some("admin"))
    );
    let s = store().as_instance();
    let id = install(&s, doc("board"), "1.0");
    let b = block_ref();
    assert_eq!(
        block_on(s.lock(&id, Some(&b), "  ", &who()))
            .unwrap_err()
            .code,
        LayoutCode::ReasonMissing
    );
    assert!(block_on(s.lock(&id, Some(&b), " frozen ", &who())).unwrap());
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    let r =
        block_on(s.replace_ops(&o, Rev(1), vec![set_text(Field::TextKey, "x")], &who())).unwrap();
    assert_eq!(
        (r.problems[0].code, r.problems[0].param("reason")),
        (LayoutCode::Locked, Some("frozen"))
    );
    assert_eq!(
        block_on(s.unlock(&id, Some(&b), "", &who()))
            .unwrap_err()
            .code,
        LayoutCode::ReasonMissing
    );
    assert!(block_on(s.unlock(&id, Some(&b), "done", &who())).unwrap());
    assert!(
        block_on(s.replace_ops(&o, Rev(1), vec![set_text(Field::TextKey, "x")], &who()))
            .unwrap()
            .problems
            .is_empty()
    );
}

#[test]
fn apply_is_pure_and_off_overlays_do_nothing() {
    let layout = doc("board")
        .check(&OpenCatalog, &Limits::declared())
        .unwrap();
    let a = apply(&layout, &[], &[], "1.0");
    assert_eq!(a.layout, layout);
    assert!(a.effects.is_empty());
}
