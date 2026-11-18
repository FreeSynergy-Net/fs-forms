//! Review fixes of 18.11.: overlays obey the local rules (D9), `check` is at least as strict as the tables, `resolve` is never silent,
//! and the tests that the first review found missing (dry run, overlay rev, ranges, off overlays, ops limit).
#![allow(clippy::assert_is_empty, clippy::many_single_char_names)] // tests: short names and plain asserts read best
mod support;

use fs_forms_layout::*;
use support::*;

fn schemes_doc() -> LayoutDoc {
    let mut d = doc("board");
    let list = vec![item("scheme:b/0", "https".to_owned())];
    d.pages[0].v.blocks[0].v.bindings[0].v.schemes = Some(list);
    d
}
fn scheme_ref() -> ElemRef {
    ElemRef {
        kind: ElemKind::BindingScheme,
        elem: elem("scheme:b/0"),
    }
}
fn codes(r: Result<Layout, Vec<Coded<LayoutCode>>>) -> Vec<LayoutCode> {
    r.expect_err("must fail").iter().map(|c| c.code).collect()
}
fn check(d: LayoutDoc) -> Result<Layout, Vec<Coded<LayoutCode>>> {
    d.check(&OpenCatalog, &Limits::declared())
}

/// An overlay op that sets one field; the result must be refused with `result-invalid` and the effective layout stays as it was.
fn refused_set(target: ElemRef, field: Field, value: &str, d: LayoutDoc) -> Resolved {
    let s = store();
    let id = install(&s, d, "1.0");
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    let op = OpKind::Set {
        target,
        field,
        value: Value::Text(value.into()),
        expect: Value::None,
    };
    let r = block_on(s.replace_ops(&o, Rev(1), vec![op], &who())).unwrap();
    assert_eq!(r.conflicts[0].cause, Cause::ResultInvalid, "{value}");
    block_on(s.resolve(&key("board"))).unwrap().unwrap()
}

#[test]
fn r1_an_overlay_cannot_set_a_forbidden_scheme() {
    for bad in ["javascript", "vbscript", "SSH", "1http", "ht tp"] {
        let r = refused_set(scheme_ref(), Field::Scheme, bad, schemes_doc());
        assert_eq!(
            r.layout.bindings("board", "main")[0].schemes,
            Some(vec!["https".to_owned()]),
            "{bad}"
        );
    }
}

#[test]
fn r6_an_overlay_cannot_break_a_message() {
    let msg = ElemRef {
        kind: ElemKind::Message,
        elem: elem("message:board-title/de"),
    };
    let r = refused_set(msg, Field::Message, "{open", doc("board"));
    assert_eq!(r.layout.doc().messages[0].v.message, "Aufgaben");
}

#[test]
fn check_is_at_least_as_strict_as_the_tables() {
    let mut d = schemes_doc();
    d.pages[0].v.blocks[0].v.bindings[0].v.schemes =
        Some(vec![item("scheme:b/0", "SSH".to_owned())]);
    assert_eq!(codes(check(d)), vec![LayoutCode::SchemeBad]);
    let mut d = schemes_doc();
    d.pages[0].v.blocks[0].v.bindings[0].v.schemes =
        Some(vec![item("scheme:b/0", "1http".to_owned())]);
    assert_eq!(codes(check(d)), vec![LayoutCode::SchemeBad]); // N4c: a digit never starts a scheme
    let mut d = doc("board");
    d.pages[0].v.blocks[0].v.source = Some(9);
    assert_eq!(codes(check(d)), vec![LayoutCode::SourceUnknown]);
    let mut d = doc("board");
    let target = ActionTarget::Command {
        source: 9,
        command: "create".into(),
    };
    d.actions = vec![item(
        "action:1",
        ActionDoc {
            trigger: Trigger::Button,
            target,
            confirm: None,
        },
    )];
    assert_eq!(codes(check(d)), vec![LayoutCode::SourceUnknown]);
}

#[test]
fn menu_pair_rules_are_checked() {
    let entry =
        |kind, label: Option<&str>, command: Option<(u32, String)>, view: Option<&str>| MenuDoc {
            kind,
            label_key: label.map(str::to_owned),
            command,
            on: None,
            shortcut: None,
            view: view.map(str::to_owned),
            children: vec![],
        };
    let one = |m: MenuDoc| {
        let mut d = doc("board");
        d.menu = vec![item("menu:0", m)];
        codes(check(d))
    };
    assert_eq!(
        one(entry(MenuKind::Command, None, None, None)),
        vec![LayoutCode::MenuInvalid]
    );
    assert_eq!(
        one(entry(MenuKind::View, None, None, None)),
        vec![LayoutCode::MenuInvalid]
    );
    assert_eq!(
        one(entry(MenuKind::Separator, Some("x"), None, None)),
        vec![LayoutCode::MenuInvalid]
    );
    assert_eq!(
        one(entry(MenuKind::Menu, None, None, Some("v"))),
        vec![LayoutCode::MenuInvalid]
    );
    let with_source_9 = entry(MenuKind::Command, None, Some((9, "create".into())), None);
    assert_eq!(one(with_source_9), vec![LayoutCode::SourceUnknown]);
}

#[test]
fn n17_a_range_needs_both_bounds() {
    for text in [
        "tasks >=0.2",
        "tasks <0.3",
        "tasks",
        "tasks >=0.3 <0.2",
        "tasks >0.2",
    ] {
        let mut d = doc("board");
        d.requires = vec![item("requires:t", RequiresDoc { range: text.into() })];
        assert_eq!(codes(check(d)), vec![LayoutCode::RangeInvalid], "{text}");
        let mut d = doc("board");
        d.sources = vec![item(
            "source:1",
            SourceDoc {
                range: text.into(),
                data_type: "T".into(),
                scope: None,
            },
        )];
        assert_eq!(codes(check(d)), vec![LayoutCode::RangeInvalid], "{text}");
    }
}

#[test]
fn m34_a_dry_run_leaves_no_trace() {
    let s = store();
    block_on(s.import(doc("board"), Mode::DryRun, &who())).unwrap();
    assert!(
        block_on(s.list(&LayoutFilter::default(), None, 10))
            .unwrap()
            .items
            .is_empty()
    );
    let r = block_on(s.import(doc("board"), Mode::Apply, &who())).unwrap();
    assert!(r.problems.is_empty() && r.layout.is_some()); // the key is still free
}

#[test]
fn m47b_overlay_rev_is_compared() {
    let s = store();
    let id = install(&s, doc("board"), "1.0");
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    block_on(s.replace_ops(&o, Rev(1), vec![set_text(Field::TextKey, "a")], &who())).unwrap();
    let stamp = block_on(s.stamp()).unwrap();
    let e = block_on(s.replace_ops(&o, Rev(1), vec![set_text(Field::TextKey, "b")], &who()))
        .unwrap_err();
    assert_eq!(
        (e.code, e.param("expected"), e.param("found")),
        (LayoutCode::RevChanged, Some("1"), Some("2"))
    );
    assert_eq!(block_on(s.stamp()).unwrap(), stamp);
    let e = block_on(s.set_state(&o, Rev(1), false, &who())).unwrap_err();
    assert_eq!(e.code, LayoutCode::RevChanged);
}

#[test]
fn n21_ops_limit_has_an_exact_boundary() {
    let limits = Limits {
        ops_max: 2,
        ..Limits::declared()
    };
    let s = MemoryLayoutStore::new(Box::new(OpenCatalog), limits, who().actor);
    let id = install(&s, doc("board"), "1.0");
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    let op = || set_text(Field::TextKey, "x");
    assert!(
        block_on(s.replace_ops(&o, Rev(1), vec![op(), op()], &who()))
            .unwrap()
            .problems
            .is_empty()
    );
    let r = block_on(s.replace_ops(&o, Rev(2), vec![op(), op(), op()], &who())).unwrap();
    assert_eq!(
        (r.problems[0].code, r.problems[0].param("max")),
        (LayoutCode::TooMany, Some("2"))
    );
}

fn manual_overlay(state: OverlayState, scope: OverlayScope, text: &str) -> Overlay {
    let op = Op {
        seq: 0,
        kind: set_text(Field::TextKey, text),
        who: who(),
        at: Utc(1),
    };
    let op = Op {
        kind: OpKind::Set {
            target: block_ref(),
            field: Field::TextKey,
            value: Value::Text(text.into()),
            expect: Value::None,
        },
        ..op
    };
    Overlay {
        id: OverlayId(ObjectId("o".into())),
        layout: LayoutId(ObjectId("l".into())),
        scope,
        title_key: None,
        lang: None,
        who: who(),
        base: pkg("1.0"),
        state,
        rev: Rev(1),
        ops: vec![op],
    }
}

#[test]
fn n25_off_and_orphaned_overlays_do_nothing() {
    let layout = check(doc("board")).unwrap();
    for state in [OverlayState::Off, OverlayState::Orphaned] {
        let o = manual_overlay(state, OverlayScope::Instance, "x");
        assert_eq!(apply(&layout, &[o], &[], "1.0").layout, layout, "{state:?}");
    }
    let on = manual_overlay(OverlayState::Applied, OverlayScope::Instance, "x");
    assert_ne!(apply(&layout, &[on], &[], "1.0").layout, layout);
}

#[test]
fn resolve_is_never_silent_about_providers() {
    let mut d = doc("board");
    let src = SourceDoc {
        range: "tasks >=0.2 <0.3".into(),
        data_type: "Task".into(),
        scope: None,
    };
    d.sources = vec![item("source:1", src)];
    d.pages[0].v.blocks[0].v.source = Some(1);
    let s = store(); // no provider connected
    install(&s, d, "1.0");
    let r = block_on(s.resolve(&key("board"))).unwrap().unwrap();
    assert_eq!(r.warnings.len(), 1);
    assert_eq!(
        (r.warnings[0].code, r.warnings[0].param("capability")),
        (LayoutCode::ProviderUnchecked, Some("tasks"))
    );
    assert!(r.problems.is_empty());
}
