//! Fix round 4, part 2: the tests for the surviving mutants of the second review (menus, source bounds, owners, provider questions,
//! canonical message order, op order).
#![allow(clippy::assert_is_empty, clippy::many_single_char_names)] // tests: short names and plain asserts read best
mod support;

use fs_forms_layout::*;
use support::*;

fn check(d: LayoutDoc) -> Result<Layout, Vec<Coded<LayoutCode>>> {
    d.check(&OpenCatalog, &Limits::declared())
}
fn codes(r: Result<Layout, Vec<Coded<LayoutCode>>>) -> Vec<LayoutCode> {
    r.expect_err("must fail").iter().map(|c| c.code).collect()
}
fn source(range: &str) -> SourceDoc {
    SourceDoc {
        range: range.into(),
        data_type: "T".into(),
        scope: None,
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
fn r1c_nested_menu_entries_are_checked() {
    let entry = |kind, label: Option<&str>, kids| MenuDoc {
        kind,
        label_key: label.map(str::to_owned),
        command: None,
        on: None,
        shortcut: None,
        view: None,
        children: kids,
    };
    let mut d = doc("board");
    let bad_child = item("menu:0.0", entry(MenuKind::Command, None, vec![]));
    d.menu = vec![item("menu:0", entry(MenuKind::Menu, None, vec![bad_child]))];
    assert_eq!(codes(check(d)), vec![LayoutCode::MenuInvalid]);
    let mut d = doc("board");
    let sep = item("menu:0.0", entry(MenuKind::Separator, Some("x"), vec![]));
    d.menu = vec![item("menu:0", entry(MenuKind::Menu, None, vec![sep]))];
    assert_eq!(codes(check(d)), vec![LayoutCode::MenuInvalid]);
}

#[test]
fn r1d_r1e_source_numbers_have_exact_bounds() {
    for n in [0_u32, 2] {
        let mut d = doc("board");
        d.sources = vec![item("source:1", source("tasks >=0.1 <0.2"))];
        d.pages[0].v.blocks[0].v.source = Some(n);
        assert_eq!(
            codes(check(d)),
            vec![LayoutCode::SourceUnknown],
            "source {n}"
        );
    }
    let mut d = doc("board");
    d.sources = vec![item("source:1", source("tasks >=0.1 <0.2"))];
    d.pages[0].v.blocks[0].v.source = Some(1);
    assert!(check(d).is_ok());
}

#[test]
fn r1h_a_view_owner_is_not_empty() {
    let mut d = doc("board");
    let view = ViewDoc {
        view: "v".into(),
        owner: Some(Reference(String::new())),
        grouping: None,
        orders: vec![],
        filter: None,
        search_kind: None,
    };
    d.views = vec![item("view:v", view)];
    assert_eq!(codes(check(d)), vec![LayoutCode::KeyInvalid]);
}

#[test]
fn r2c_only_blocks_with_a_field_binding_ask_the_provider() {
    let mut d = doc("board");
    d.sources = vec![item("source:1", source("tasks >=0.1 <0.2"))];
    d.pages[0].v.blocks[0].v.source = Some(1);
    d.pages[0].v.blocks[0].v.bindings[0].v.from = BindFrom::Records; // no field: nothing to ask
    let s = store();
    install(&s, d, "1.0");
    assert!(resolved(&s).warnings.is_empty());
}

#[test]
fn m35m_check_puts_messages_in_key_lang_order() {
    let mut d = doc("board");
    let m = |k: &str, l: &str| MessageDoc {
        key: k.into(),
        lang: l.into(),
        message: "x".into(),
    };
    d.messages = vec![
        item("message:b/de", m("b", "de")),
        item("message:a/en", m("a", "en")),
        item("message:a/de", m("a", "de")),
    ];
    let l = check(d).unwrap();
    let order: Vec<(String, String)> = l
        .doc()
        .messages
        .iter()
        .map(|x| (x.v.key.clone(), x.v.lang.clone()))
        .collect();
    let want = [("a", "de"), ("a", "en"), ("b", "de")].map(|(k, l)| (k.to_owned(), l.to_owned()));
    assert_eq!(order, want);
}

#[test]
fn o1_the_ops_of_one_overlay_run_in_seq_order() {
    let s = store();
    install(&s, doc("board"), "1.0");
    // forward: set the text, then remove the block (both act); reversed, the set would hit a removed block and conflict
    let ops = vec![set_text(Field::TextKey, "x"), remove(block_ref())];
    assert_eq!(conflict_of(&s, ops), Vec::<Cause>::new());
    assert!(resolved(&s).layout.doc().pages[0].v.blocks.is_empty());
}

fn three_sources(block_on_source: u32) -> LayoutDoc {
    let mut d = doc("board");
    d.sources = vec![
        item("source:1", source("tasks >=0.1 <0.2")),
        item("source:2", source("people >=0.1 <0.2")),
        item("source:3", source("groups >=0.1 <0.2")),
    ];
    d.pages[0].v.blocks[0].v.source = Some(block_on_source);
    d
}
fn source_elem(n: u32) -> ElemRef {
    ElemRef {
        kind: ElemKind::Source,
        elem: elem(&format!("source:{n}")),
    }
}

/// Q3: removing a source in front of the one a block uses would renumber it, although the removed source itself is not referenced.
#[test]
fn q3_a_source_before_the_used_one_cannot_go_either() {
    for (used, removed) in [(2, 1), (3, 1), (3, 2), (1, 2), (1, 3), (2, 3)] {
        let s = store();
        install(&s, three_sources(used), "1.0");
        let want = if removed == 3 && used != 3 {
            Vec::<Cause>::new()
        } else {
            vec![Cause::ResultInvalid]
        };
        assert_eq!(
            conflict_of(&s, vec![remove(source_elem(removed))]),
            want,
            "block on {used}, remove {removed}"
        );
        let r = resolved(&s);
        let doc = r.layout.doc();
        let range = &doc.sources[usize::try_from(used - 1).unwrap()].v.range;
        let name = ["tasks", "people", "groups"][usize::try_from(used - 1).unwrap()];
        assert!(
            range.starts_with(name),
            "block on {used} still sees {name}, got {range}"
        );
        assert_eq!(doc.pages[0].v.blocks[0].v.source, Some(used));
    }
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

fn set_block(to: &str) -> OpKind {
    OpKind::Set {
        target: block_ref(),
        field: Field::Block,
        value: Value::Text(to.into()),
        expect: Value::None,
    }
}

/// Q9: a new package version re-applies every overlay; the catalog is asked again. The overlay set a block the catalog does not
/// know: it is a conflict at writing and again, under the new base version, at the new install.
#[test]
fn q9_a_new_package_version_asks_the_catalog_again() {
    let s = MemoryLayoutStore::new(Box::new(BoardOnly), Limits::declared(), who().actor);
    install(&s, doc("board"), "1.0");
    assert_eq!(
        conflict_of(&s, vec![set_block("other")]),
        vec![Cause::ResultInvalid]
    );
    let r = block_on(s.install(doc("board"), &pkg("2.0"), Mode::Apply, &who())).unwrap();
    assert_eq!(
        r.conflicts
            .iter()
            .map(|c| (c.cause, c.base_version.as_str()))
            .collect::<Vec<_>>(),
        vec![(Cause::ResultInvalid, "2.0")]
    );
}

/// Q14: the same for the limits: an overlay text over the limit is a conflict again at the new install.
#[test]
fn q14_a_new_package_version_holds_the_limits_again() {
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
        target,
        field: Field::Message,
        value: Value::Text("x".repeat(100)),
        expect: Value::None,
    };
    assert_eq!(conflict_of(&s, vec![long]), vec![Cause::ResultInvalid]);
    let r = block_on(s.install(doc("board"), &pkg("2.0"), Mode::Apply, &who())).unwrap();
    assert_eq!(
        r.conflicts
            .iter()
            .map(|c| (c.cause, c.base_version.as_str()))
            .collect::<Vec<_>>(),
        vec![(Cause::ResultInvalid, "2.0")]
    );
    assert_eq!(resolved(&s).layout.doc().messages[0].v.message, "Aufgaben");
}
