//! L2b on the wire: the scope token as text (host word or RFC 6749 section 3.3 token) and the binding source (`hasSource`, index 0 is
//! valid). Real writer, real reader. A word that breaks the grammar is `format-invalid`, never a silent "no scope".
//!
//! Pattern: Round trip (writer and reader against each other) plus counter-tests on the raw message.

use fs_forms_layout::{
    BindFrom, BindingDoc, BlockDoc, ElemId, Item, LayoutCode, PageDoc, PlaceNode, ScopeToken,
    SourceDoc, SourceScope,
};

use super::tree_guard::TreeLimits;
use super::tree_tests::{doc, it, read, wire};
use crate::layouts_capnp as w;

fn binding(n: u32, source: Option<u32>) -> Item<BindingDoc> {
    Item {
        elem: Some(ElemId::parse(&format!("binding:b/main/lanes/{n}")).unwrap()),
        line: 0,
        v: BindingDoc {
            slot: "lanes".into(),
            nth: n,
            from: BindFrom::Grouping("assignee".into()),
            schemes: None,
            source,
        },
    }
}

fn lanes_doc() -> fs_forms_layout::LayoutDoc {
    let block = BlockDoc {
        place: "main".into(),
        block: "cards-in-columns".into(),
        source: Some(1),
        title_key: None,
        text_key: None,
        bindings: vec![binding(0, Some(2)), binding(1, Some(0)), binding(2, None)],
        set: vec![],
    };
    let mut d = doc(vec![it(None, PlaceNode::Block(block.clone()))]);
    let page: &mut PageDoc = &mut d.pages[0].v;
    page.blocks = vec![it(Some("block:b/main"), block)];
    d.sources = ["selected-list", "lanes:assignee"]
        .iter()
        .enumerate()
        .map(|(i, t)| {
            it(
                Some(&format!("source:{}", i + 1)),
                SourceDoc {
                    range: "planning@>=0.0.1, <0.1.0".into(),
                    data_type: "Lane".into(),
                    scope: SourceScope::from_text(t),
                },
            )
        })
        .collect();
    d
}

#[test]
fn scope_token_and_binding_source_survive_the_wire() {
    let d = lanes_doc();
    let back = read(&wire(&d), &TreeLimits::declared()).unwrap();
    assert_eq!(back, d);
    assert_eq!(
        back.sources[1].v.scope,
        ScopeToken::parse("lanes:assignee").map(SourceScope::Module)
    );
    let sources: Vec<_> = back.pages[0].v.blocks[0]
        .v
        .bindings
        .iter()
        .map(|b| b.v.source)
        .collect();
    assert_eq!(
        sources,
        [Some(2), Some(0), None],
        "0 is a source, not 'none'"
    );
}

#[test]
fn has_source_is_false_for_none_and_true_for_zero() {
    let m = wire(&lanes_doc());
    let r = m.get_root_as_reader::<w::layout_doc::Reader<'_>>().unwrap();
    let binds = r
        .get_pages()
        .unwrap()
        .get(0)
        .get_blocks()
        .unwrap()
        .get(0)
        .get_binds()
        .unwrap();
    let flags: Vec<_> = binds
        .iter()
        .map(|b| (b.get_has_source(), b.get_source()))
        .collect();
    assert_eq!(flags, [(true, 2), (true, 0), (false, 0)]);
}

#[test]
fn a_scope_that_breaks_the_grammar_on_the_wire_is_format_invalid() {
    for bad in ["a b", "x\"y", "x\\y", "caf\u{e9}", &"a".repeat(257)] {
        let mut m = wire(&lanes_doc());
        m.get_root::<w::layout_doc::Builder<'_>>()
            .unwrap()
            .get_sources()
            .unwrap()
            .get(1)
            .set_scope(bad);
        let e = read(&m, &TreeLimits::declared()).expect_err(bad);
        assert_eq!(e.code, LayoutCode::FormatInvalid, "{bad:?}");
    }
}
