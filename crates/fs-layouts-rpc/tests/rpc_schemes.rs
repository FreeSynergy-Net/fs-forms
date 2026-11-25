//! The scheme policy of the instance over the wire: what the service accepted with its policy is read back by the client, never
//! refused again with the default list (the client asks only grammar and the fixed floor).
//!
//! Pattern: Specification asked through the real path (socket, service, client), with the instance as the only source of a wider list.
mod support;

use fs_forms_layout::*;
use support::*;

fn with_scheme(name: &str) -> LayoutDoc {
    let mut d = doc("board");
    d.pages[0].v.blocks[0].v.bindings[0].v.schemes =
        Some(vec![item("scheme:a/0", name.to_owned())]);
    d
}

fn instance_limits() -> Limits {
    let ceil = CeilingSet {
        schemes: Some(SchemePolicy::listed(["https", "mailto"]).unwrap()),
        ..CeilingSet::default()
    };
    Limits::effective(&Limits::declared(), &ceil)
}

#[test]
fn a_scheme_the_instance_allows_comes_back_through_the_client() {
    let d = dir();
    let s = person_with(&d.path().join("l.sock"), instance_limits());
    run(async {
        let c = client(&s.socket);
        let r = c.create(with_scheme("mailto"), &who()).await.unwrap();
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        let id = r.layout.unwrap();
        let (layout, _) = c.original(&id).await.unwrap().unwrap();
        assert_eq!(layout.to_doc(), with_scheme("mailto"));
        let res = c.resolve(&key("board")).await.unwrap().unwrap();
        assert_eq!(res.layout.to_doc(), with_scheme("mailto"));
    });
}

#[test]
fn the_service_still_refuses_what_its_policy_does_not_list() {
    let tmp = dir();
    let svc = person(&tmp.path().join("l.sock"));
    run(async {
        let cli = client(&svc.socket);
        let reply = cli.create(with_scheme("mailto"), &who()).await;
        let blocked = match reply {
            Ok(rep) => rep
                .problems
                .iter()
                .any(|p| p.code == LayoutCode::SchemeNotAllowed),
            Err(err) => err.code == LayoutCode::SchemeNotAllowed,
        };
        assert!(blocked);
    });
}
