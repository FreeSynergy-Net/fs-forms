//! Tests of `Action.at` on the wire: a document with a place round-trips, an old message (no `at` written) reads as `None`.
//! The wire is built with the real writer and read with the real reader.
//!
//! Pattern: Adapter (tests of the action over the wire, both directions); the old bytes are a message whose `at` was never set.

use fs_forms_layout::{ActionDoc, ActionTarget, ElemId, Item, LayoutDoc, Reference, Trigger};

use super::doc_in::read_doc;
use super::doc_out::write_doc;
use super::tree_guard::TreeLimits;
use super::tree_tests::{chain, doc, it};
use crate::layouts_capnp as w;

fn action(at: Option<&str>) -> Item<ActionDoc> {
    it(
        Some("action:go"),
        ActionDoc {
            trigger: Trigger::Button,
            at: at.map(|e| ElemId::parse(e).unwrap()),
            target: ActionTarget::Workflow(Reference("https://example.org/wf/1".into())),
            confirm: None,
        },
    )
}

fn through(d: &LayoutDoc) -> LayoutDoc {
    let mut m = capnp::message::Builder::new_default();
    write_doc(m.init_root::<w::layout_doc::Builder<'_>>(), d).unwrap();
    read_doc(
        m.get_root_as_reader::<w::layout_doc::Reader<'_>>().unwrap(),
        &TreeLimits::declared(),
    )
    .unwrap()
}

#[test]
fn an_action_with_a_place_survives_the_wire() {
    let mut d = doc(chain(1));
    d.actions = vec![action(Some("block:main"))];
    let got = through(&d);
    assert_eq!(
        got.actions[0].v.at.as_ref().map(ElemId::as_str),
        Some("block:main")
    );
    assert_eq!(got.actions[0].v, d.actions[0].v);
}

#[test]
fn an_action_without_a_place_stays_without_one() {
    let mut d = doc(chain(1));
    d.actions = vec![action(None)];
    assert!(through(&d).actions[0].v.at.is_none());
}

#[test]
fn old_bytes_without_at_read_as_none() {
    // a message from a writer that knew no `at`: the member was never set
    let mut m = capnp::message::Builder::new_default();
    let mut root = m.init_root::<w::layout_doc::Builder<'_>>();
    let mut with = doc(chain(1));
    with.actions = vec![action(None)];
    write_doc(root.reborrow(), &with).unwrap();
    let r = m.get_root_as_reader::<w::layout_doc::Reader<'_>>().unwrap();
    let a = r.get_actions().unwrap().get(0);
    assert!(a.get_at().unwrap().is_empty());
    assert!(
        read_doc(r, &TreeLimits::declared()).unwrap().actions[0]
            .v
            .at
            .is_none()
    );
}
