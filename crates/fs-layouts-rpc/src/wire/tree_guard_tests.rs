//! Tests of the guard of the tree of places (depth, nodes) with its counter-proofs: over the limit the message is refused with
//! `too-many` BEFORE anything is unpacked (a message poisoned for the reader shows it). Helpers from `tree_tests`.
//!
//! Pattern: Composite (tests of the guard over the wire); the cases are a table, one function checks them all.

use fs_forms_layout::{LayoutCode, LayoutDoc};

use super::tree_guard::TreeLimits;
use super::tree_tests::*;
use crate::layouts_capnp as w;

/// Another word is put into the message AFTER the writer (the page's navigation): whoever unpacks the page meets it as
/// `format-invalid{found=bogus}`; the guard must refuse BEFORE that, so the error of an over-limit message is `too-many`.
fn poisoned(d: &LayoutDoc) -> capnp::message::Builder<capnp::message::HeapAllocator> {
    let mut m = wire(d);
    m.get_root::<w::layout_doc::Builder<'_>>()
        .unwrap()
        .get_pages()
        .unwrap()
        .get(0)
        .set_navigation("bogus");
    m
}

fn found(e: &fs_forms_layout::Coded<LayoutCode>, k: &str) -> String {
    e.param(k).unwrap_or_default().to_owned()
}

#[test]
fn the_poison_is_met_when_the_limits_allow_it() {
    let e = read(&poisoned(&doc(chain(2))), &TreeLimits::declared()).unwrap_err();
    assert_eq!(
        (e.code, found(&e, "found").as_str()),
        (LayoutCode::FormatInvalid, "bogus")
    );
}

#[test]
fn the_depth_at_the_limit_is_read_one_more_is_refused_before_unpacking() {
    let lim = TreeLimits {
        depth_max: 5,
        nodes_max: 1000,
        ..TreeLimits::declared()
    };
    assert_eq!(read(&wire(&doc(chain(5))), &lim).unwrap(), doc(chain(5)));
    let e = read(&wire(&doc(chain(6))), &lim).unwrap_err();
    assert_eq!(e.code, LayoutCode::TooMany);
    assert_eq!(
        (found(&e, "what").as_str(), found(&e, "max").as_str()),
        ("place-depth", "5")
    );
    // before unpacking: the poisoned page is refused for its depth, its bogus word is never read
    let e = read(&poisoned(&doc(chain(6))), &lim).unwrap_err();
    assert_eq!(
        (e.code, found(&e, "what").as_str()),
        (LayoutCode::TooMany, "place-depth")
    );
}

#[test]
fn the_declared_depth_is_the_one_of_the_checker() {
    let max = fs_forms_layout::Limits::declared().place_depth_max;
    let lim = TreeLimits::declared();
    assert_eq!(lim.depth_max, max);
    assert!(read(&wire(&doc(chain(max))), &lim).is_ok());
    let e = read(&wire(&doc(chain(max + 1))), &lim).unwrap_err();
    assert_eq!(found(&e, "what"), "place-depth");
}

#[test]
fn the_nodes_at_the_limit_are_read_one_more_is_refused_before_unpacking() {
    let flat = |n: usize| doc((0..n).map(|_| block(None, "a", vec![])).collect());
    // the page itself counts 1: 1 page + 9 nodes = 10
    let lim = TreeLimits {
        nodes_max: 10,
        ..TreeLimits::declared()
    };
    assert_eq!(read(&wire(&flat(9)), &lim).unwrap(), flat(9));
    let e = read(&wire(&flat(10)), &lim).unwrap_err();
    assert_eq!(
        (
            e.code,
            found(&e, "what").as_str(),
            found(&e, "max").as_str()
        ),
        (LayoutCode::TooMany, "place-nodes", "10")
    );
    let e = read(&poisoned(&flat(10)), &lim).unwrap_err();
    assert_eq!(
        (e.code, found(&e, "what").as_str()),
        (LayoutCode::TooMany, "place-nodes")
    );
}

#[test]
fn attributes_count_against_the_nodes() {
    let many = (0..20).map(|i| attr(None, &format!("k{i}"), "v")).collect();
    let d = doc(vec![block(None, "a", many)]);
    let lim = TreeLimits {
        nodes_max: 10,
        ..TreeLimits::declared()
    };
    assert_eq!(
        found(&read(&poisoned(&d), &lim).unwrap_err(), "what"),
        "place-nodes"
    );
}

/// A bomb that is a wide list: one list of 100000 nodes is refused by its length alone, whatever its entries are.
#[test]
fn a_wide_list_is_refused_by_its_length() {
    let d = doc((0..100_000).map(|_| block(None, "a", vec![])).collect());
    let e = read(&poisoned(&d), &TreeLimits::declared()).unwrap_err();
    assert_eq!(
        (e.code, found(&e, "what").as_str(), found(&e, "by").as_str()),
        (LayoutCode::TooMany, "place-nodes", "default")
    );
}
