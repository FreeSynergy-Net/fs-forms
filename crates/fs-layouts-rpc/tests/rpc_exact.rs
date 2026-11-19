//! Exactness on the wire: a time arrives to the millisecond, every filter operator keeps its word, and the contract `bind` is reachable
//! through this crate (K4a conditions: tests for `bind_capnp`; the mutants write/read time +1 ms and `Not` = `Xor` must be red).
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]
mod support;

use fs_forms_layout::*;
use fs_layouts_rpc::bind_capnp;
use support::*;

#[test]
fn a_time_arrives_to_the_millisecond_in_the_list_and_in_the_log() {
    let d = dir();
    let s = person(&d.path().join("l.sock"));
    run(async {
        let c = client(&s.socket);
        // the same work on the local store (same counting clock) says which times are right
        let local = mem(false, None);
        c.create(doc("a"), &who()).await.unwrap();
        local.create(doc("a"), &who()).await.unwrap();
        let got = c.list(&LayoutFilter::default(), None, 10).await.unwrap();
        let want = local
            .list(&LayoutFilter::default(), None, 10)
            .await
            .unwrap();
        assert_eq!(got.items.len(), 1);
        assert!(got.items[0].updated.0 > 0);
        assert_eq!(got.items[0].updated, want.items[0].updated, "updated");
        let got_log = c.log(None, 10).await.unwrap();
        let want_log = local.log(None, 10).await.unwrap();
        assert!(!got_log.is_empty());
        assert_eq!(
            got_log.iter().map(|e| e.at).collect::<Vec<_>>(),
            want_log.iter().map(|e| e.at).collect::<Vec<_>>(),
            "log times"
        );
    });
}

#[test]
fn every_filter_operator_keeps_its_word_over_the_wire() {
    let d = dir();
    let s = person(&d.path().join("l.sock"));
    run(async {
        let c = client(&s.socket);
        for (n, op) in [FilterOp::And, FilterOp::Or, FilterOp::Not, FilterOp::Xor]
            .into_iter()
            .enumerate()
        {
            let mut layout = rich(&format!("f{n}"));
            let FilterDoc::Op { op: slot, .. } = &mut layout.views[0].v.filter.as_mut().unwrap().v
            else {
                panic!("the rich description has an operator at the root");
            };
            *slot = op;
            let r = c.create(layout.clone(), &who()).await.unwrap();
            assert!(r.problems.is_empty(), "{op:?}: {:?}", r.problems);
            let (back, _) = c.original(&r.layout.unwrap()).await.unwrap().unwrap();
            assert_eq!(back.to_doc(), layout, "{op:?}");
        }
    });
}

#[test]
fn the_contract_bind_is_the_one_of_fs_interfaces_and_carries_its_values() {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut v = msg.init_root::<bind_capnp::verdict::Builder<'_>>();
        v.set_rule("rule-1");
        v.set_pass(true);
        v.set_code("bind-error-locked");
    }
    let r = msg
        .get_root_as_reader::<bind_capnp::verdict::Reader<'_>>()
        .unwrap();
    assert_eq!(r.get_rule().unwrap().to_str().unwrap(), "rule-1");
    assert!(r.get_pass());
    assert_eq!(r.get_code().unwrap().to_str().unwrap(), "bind-error-locked");
}
