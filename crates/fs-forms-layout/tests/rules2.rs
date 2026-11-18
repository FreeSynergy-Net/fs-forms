//! Fix round 4 (review of 18.11., 04:40): sources are never renumbered by an overlay (P2), D9 uses catalog and limits (P1, P3), a person's
//! overlay never widens a scheme allowlist (P4), and the tests for the surviving mutants of the second review.
#![allow(clippy::assert_is_empty, clippy::many_single_char_names)] // tests: short names and plain asserts read best
mod support;

use fs_forms_layout::*;
use support::*;

fn source(range: &str) -> SourceDoc {
    SourceDoc {
        range: range.into(),
        data_type: "T".into(),
        scope: None,
    }
}
fn two_sources() -> LayoutDoc {
    let mut d = doc("board");
    d.sources = vec![
        item("source:1", source("tasks >=0.1 <0.2")),
        item("source:2", source("people >=0.1 <0.2")),
    ];
    d.pages[0].v.blocks[0].v.source = Some(1);
    d
}
fn source_ref(n: u32) -> ElemRef {
    ElemRef {
        kind: ElemKind::Source,
        elem: elem(&format!("source:{n}")),
    }
}

fn conflict_of(s: &MemoryLayoutStore, ops: Vec<OpKind>) -> Vec<Cause> {
    let id = block_on(s.list(&LayoutFilter::default(), None, 10))
        .unwrap()
        .items[0]
        .id
        .clone();
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    let r = block_on(s.replace_ops(&o, Rev(1), ops, &who())).unwrap();
    assert!(r.problems.is_empty(), "{:?}", r.problems);
    r.conflicts.iter().map(|c| c.cause).collect()
}
fn resolved(s: &MemoryLayoutStore) -> Resolved {
    block_on(s.resolve(&key("board"))).unwrap().unwrap()
}
fn remove(target: ElemRef) -> OpKind {
    OpKind::Remove { target }
}

#[test]
fn p2_removing_a_source_in_the_middle_is_refused_not_renumbered() {
    let s = store();
    install(&s, two_sources(), "1.0");
    assert_eq!(
        conflict_of(&s, vec![remove(source_ref(1))]),
        vec![Cause::ResultInvalid]
    );
    let r = resolved(&s);
    assert_eq!(r.layout.doc().sources.len(), 2);
    assert_eq!(r.layout.doc().sources[0].v.range, "tasks >=0.1 <0.2"); // the block still points at tasks
    assert_eq!(r.layout.doc().pages[0].v.blocks[0].v.source, Some(1));
}

#[test]
fn p2_the_last_source_can_go_when_nothing_points_at_it() {
    let s = store();
    install(&s, two_sources(), "1.0");
    assert_eq!(
        conflict_of(&s, vec![remove(source_ref(2))]),
        Vec::<Cause>::new()
    );
    assert_eq!(resolved(&s).layout.doc().sources.len(), 1);
}

#[test]
fn p2_the_last_source_stays_when_a_block_points_at_it() {
    let mut d = two_sources();
    d.pages[0].v.blocks[0].v.source = Some(2);
    let s = store();
    install(&s, d, "1.0");
    assert_eq!(
        conflict_of(&s, vec![remove(source_ref(2))]),
        vec![Cause::ResultInvalid]
    );
}

struct BoardOnly;
impl CatalogRules for BoardOnly {
    fn block_known(&self, b: &str) -> bool {
        b == "board"
    }
    fn slot_known(&self, _: &str, _: &str) -> bool {
        true
    }
    fn slot_repeat(&self, _: &str, _: &str) -> u32 {
        16
    }
}

#[test]
fn p1_an_overlay_cannot_set_a_block_the_catalog_does_not_know() {
    let s = MemoryLayoutStore::new(Box::new(BoardOnly), Limits::declared(), who().actor);
    install(&s, doc("board"), "1.0");
    let op = OpKind::Set {
        target: block_ref(),
        field: Field::Block,
        value: Value::Text("evil".into()),
        expect: Value::None,
    };
    assert_eq!(conflict_of(&s, vec![op]), vec![Cause::ResultInvalid]);
    assert_eq!(
        resolved(&s).layout.doc().pages[0].v.blocks[0].v.block,
        "board"
    );
}

#[test]
fn p3_an_overlay_cannot_get_past_the_text_limit() {
    let limits = Limits {
        text_bytes_max: 16,
        ..Limits::declared()
    };
    let s = MemoryLayoutStore::new(Box::new(OpenCatalog), limits, who().actor);
    install(&s, doc("board"), "1.0");
    let target = ElemRef {
        kind: ElemKind::Message,
        elem: elem("message:board-title/de"),
    };
    let long = OpKind::Set {
        target: target.clone(),
        field: Field::Message,
        value: Value::Text("x".repeat(100)),
        expect: Value::None,
    };
    assert_eq!(conflict_of(&s, vec![long]), vec![Cause::ResultInvalid]);
    // Q11: the EFFECTIVE layout keeps the short text too (resolve applies overlays in state `conflict` and must hold the limits)
    assert_eq!(resolved(&s).layout.doc().messages[0].v.message, "Aufgaben");
    let fits = OpKind::Set {
        target,
        field: Field::Message,
        value: Value::Text("short".into()),
        expect: Value::None,
    };
    let id = block_on(s.list(&LayoutFilter::default(), None, 10))
        .unwrap()
        .items[0]
        .id
        .clone();
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    assert!(
        block_on(s.replace_ops(&o, Rev(1), vec![fits], &who()))
            .unwrap()
            .conflicts
            .is_empty()
    );
}

fn schemes_doc() -> LayoutDoc {
    let mut d = doc("board");
    d.pages[0].v.blocks[0].v.bindings[0].v.schemes =
        Some(vec![item("scheme:b/0", "https".to_owned())]);
    d
}
fn set_scheme(to: &str) -> OpKind {
    let target = ElemRef {
        kind: ElemKind::BindingScheme,
        elem: elem("scheme:b/0"),
    };
    OpKind::Set {
        target,
        field: Field::Scheme,
        value: Value::Text(to.into()),
        expect: Value::None,
    }
}

#[test]
fn p4_a_person_overlay_never_widens_the_allowlist() {
    let s = store();
    install(&s, schemes_doc(), "1.0");
    assert_eq!(
        conflict_of(&s, vec![set_scheme("file")]),
        vec![Cause::AllowlistWidened]
    );
    let schemes = resolved(&s).layout.bindings("board", "main")[0]
        .schemes
        .clone();
    assert_eq!(schemes, Some(vec!["https".to_owned()]));
}

/// Q1: not only `file`: every value a person sets is refused, whatever it is.
#[test]
fn p4_every_scheme_value_of_a_person_is_refused() {
    for value in ["http", "ftp", "HTTPS", "", "https", "data", "ssh"] {
        let s = store();
        install(&s, schemes_doc(), "1.0");
        assert_eq!(
            conflict_of(&s, vec![set_scheme(value)]),
            vec![Cause::AllowlistWidened],
            "{value:?}"
        );
        let schemes = resolved(&s).layout.bindings("board", "main")[0]
            .schemes
            .clone();
        assert_eq!(schemes, Some(vec!["https".to_owned()]), "{value:?}");
    }
}

#[test]
fn p4_the_instance_may_widen_but_never_to_a_forbidden_scheme() {
    let s = store().as_instance();
    let id = install(&s, schemes_doc(), "1.0");
    let o = block_on(s.create_instance_overlay(&id, None, None, &who())).unwrap();
    let ok = block_on(s.replace_ops(&o, Rev(1), vec![set_scheme("ftp")], &who())).unwrap();
    assert!(ok.conflicts.is_empty());
    assert_eq!(
        resolved(&s).layout.bindings("board", "main")[0].schemes,
        Some(vec!["ftp".to_owned()])
    );
    let bad = block_on(s.replace_ops(&o, Rev(2), vec![set_scheme("javascript")], &who())).unwrap();
    assert_eq!(bad.conflicts[0].cause, Cause::ResultInvalid);
}
