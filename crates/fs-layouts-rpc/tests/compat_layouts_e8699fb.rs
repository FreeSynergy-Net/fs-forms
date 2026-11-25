//! L2b: the contract `layouts` of fs-interfaces e8699fb4 (before L2a, kept as generated code in `compat/`, NOT a second fs-interfaces in
//! the graph) against a395979b (appends `Binding.source @9`, `hasSource @10`; `Source.scope` carries a free token). Both directions:
//! an old reader reads what the new schema writes (the rest intact, the new fields ignored), the new schema reads what the old one
//! wrote (no source = `hasSource` false). A scope token is plain text on both sides.
//!
//! Pattern: Golden master / compat fixture (frozen generated code of the old schema, as in fs-interfaces `tests/compat_0_0_20.rs`).

use fs_interfaces::layouts_capnp as new;

#[allow(unused_imports)]
// the frozen generated code below refers to `crate::common_capnp` and `crate::bind_capnp`
use fs_interfaces::{bind_capnp, common_capnp};

#[path = "compat/layouts_e8699fb_capnp.rs"]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
mod layouts_capnp;
use layouts_capnp as old;

fn reader_of(bytes: &[u8]) -> capnp::message::Reader<impl capnp::message::ReaderSegments + '_> {
    capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap()
}

/// A layout with one page, one block and one `grouping` binding, written by the NEW schema.
fn new_bytes(source: Option<u32>) -> Vec<u8> {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut doc = msg.init_root::<new::layout_doc::Builder<'_>>();
        doc.set_format(4);
        let mut srcs = doc.reborrow().init_sources(2);
        srcs.reborrow().get(0).set_scope("selected-list");
        srcs.reborrow().get(1).set_scope("lanes:assignee");
        srcs.reborrow().get(1).set_n(2);
        let mut pages = doc.init_pages(1);
        let mut blocks = pages.reborrow().get(0).init_blocks(1);
        let mut binds = blocks.reborrow().get(0).init_binds(1);
        let mut b = binds.reborrow().get(0);
        b.set_slot("lanes");
        b.set_from("grouping");
        b.set_name("assignee");
        b.set_has_source(source.is_some());
        b.set_source(source.unwrap_or(0));
    }
    capnp::serialize::write_message_to_words(&msg)
}

/// The same layout written by the OLD schema (no `source`).
fn old_bytes() -> Vec<u8> {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut doc = msg.init_root::<old::layout_doc::Builder<'_>>();
        doc.set_format(3);
        let mut srcs = doc.reborrow().init_sources(2);
        srcs.reborrow().get(0).set_scope("selected-list");
        srcs.reborrow().get(1).set_scope("all");
        let mut pages = doc.init_pages(1);
        let mut blocks = pages.reborrow().get(0).init_blocks(1);
        let mut binds = blocks.reborrow().get(0).init_binds(1);
        let mut b = binds.reborrow().get(0);
        b.set_slot("lanes");
        b.set_from("grouping");
        b.set_name("assignee");
    }
    capnp::serialize::write_message_to_words(&msg)
}

#[test]
fn the_old_reader_reads_the_new_writer_and_ignores_the_source() {
    for source in [Some(2), Some(0), None] {
        let bytes = new_bytes(source);
        let reader = reader_of(&bytes);
        let doc = reader.get_root::<old::layout_doc::Reader<'_>>().unwrap();
        assert_eq!(doc.get_format(), 4);
        let b = doc
            .get_pages()
            .unwrap()
            .get(0)
            .get_blocks()
            .unwrap()
            .get(0)
            .get_binds()
            .unwrap()
            .get(0);
        assert_eq!(b.get_slot().unwrap().to_str().unwrap(), "lanes");
        assert_eq!(b.get_from().unwrap().to_str().unwrap(), "grouping");
        assert_eq!(b.get_name().unwrap().to_str().unwrap(), "assignee");
        assert!(
            !b.get_has_schemes(),
            "the neighbours of the new slots are intact"
        );
    }
}

#[test]
fn a_free_scope_token_is_plain_text_for_the_old_reader() {
    let bytes = new_bytes(Some(2));
    let reader = reader_of(&bytes);
    let doc = reader.get_root::<old::layout_doc::Reader<'_>>().unwrap();
    let s = doc.get_sources().unwrap();
    assert_eq!(
        s.get(0).get_scope().unwrap().to_str().unwrap(),
        "selected-list"
    );
    assert_eq!(
        s.get(1).get_scope().unwrap().to_str().unwrap(),
        "lanes:assignee"
    );
    assert_eq!(s.get(1).get_n(), 2);
}

#[test]
fn the_new_schema_reads_the_old_writer_as_no_source() {
    let bytes = old_bytes();
    let reader = reader_of(&bytes);
    let doc = reader.get_root::<new::layout_doc::Reader<'_>>().unwrap();
    assert_eq!(doc.get_format(), 3);
    let b = doc
        .get_pages()
        .unwrap()
        .get(0)
        .get_blocks()
        .unwrap()
        .get(0)
        .get_binds()
        .unwrap()
        .get(0);
    assert_eq!(b.get_name().unwrap().to_str().unwrap(), "assignee");
    assert!(!b.get_has_source(), "an old writer leaves hasSource false");
    assert_eq!(b.get_source(), 0);
    assert_eq!(
        doc.get_sources()
            .unwrap()
            .get(1)
            .get_scope()
            .unwrap()
            .to_str()
            .unwrap(),
        "all"
    );
}

#[test]
fn the_new_schema_keeps_source_0_apart_from_no_source() {
    for (given, has, n) in [(Some(0), true, 0), (Some(2), true, 2), (None, false, 0)] {
        let bytes = new_bytes(given);
        let reader = reader_of(&bytes);
        let doc = reader.get_root::<new::layout_doc::Reader<'_>>().unwrap();
        let b = doc
            .get_pages()
            .unwrap()
            .get(0)
            .get_blocks()
            .unwrap()
            .get(0)
            .get_binds()
            .unwrap()
            .get(0);
        assert_eq!((b.get_has_source(), b.get_source()), (has, n), "{given:?}");
    }
}

#[test]
fn the_old_fields_of_the_binding_are_at_the_same_places_in_both_writers() {
    // the same bytes read by both schemas: the old fields of the binding are at the same places
    let new_b = new_bytes(Some(2));
    let old_b = old_bytes();
    let (rn, ro) = (reader_of(&new_b), reader_of(&old_b));
    let n = rn.get_root::<old::layout_doc::Reader<'_>>().unwrap();
    let o = ro.get_root::<old::layout_doc::Reader<'_>>().unwrap();
    let bn = n
        .get_pages()
        .unwrap()
        .get(0)
        .get_blocks()
        .unwrap()
        .get(0)
        .get_binds()
        .unwrap()
        .get(0);
    let bo = o
        .get_pages()
        .unwrap()
        .get(0)
        .get_blocks()
        .unwrap()
        .get(0)
        .get_binds()
        .unwrap()
        .get(0);
    assert_eq!(
        bn.get_name().unwrap().to_str().unwrap(),
        bo.get_name().unwrap().to_str().unwrap()
    );
    assert_eq!(
        bn.get_from().unwrap().to_str().unwrap(),
        bo.get_from().unwrap().to_str().unwrap()
    );
}
