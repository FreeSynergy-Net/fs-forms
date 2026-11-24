//! The pure decision `decide` and "the owner never asks" (O1, W11, W12), against `FakeWindows` as the source of `open_now`.
//!
//! Tests: W11 the table of `decide` (not open, same record, other record with each `WhenOpen`; `Tab` asks until phase 2);
//! W12 `Ask` calls the owner never (G4): the counters stand until an answer comes, and the fake refuses an unanswered `Ask`.
//! The setting `fs.open.when-open` ("always so") is O3 (fs-settings) and not part of this repo yet.
//!
//! Mutants (each run red, see BERICHT-s6-o1): M12 `Ask` is taken as `Switch` -> W11, W12 (G4); M15 the fake opens on an unanswered `Ask` -> W12.

mod window_owner_common;

use fs_forms_run::{
    Decision, OpenFrontend, Placement, WhenOpen, WindowId, WindowOpened, WindowOwner, decide,
};
use window_owner_common::{fake, id, tasks};

fn req(scope: Option<&str>, when: WhenOpen) -> OpenFrontend {
    let r = OpenFrontend::new(tasks(), Placement::Window).with_when_open(when);
    match scope {
        Some(s) => r.with_scope(s),
        None => r,
    }
}

#[test]
fn w11_decide_is_a_table_over_open_now() {
    let owner = fake();
    // nothing open: a new one, nothing to ask, whatever the rule says
    for w in [
        WhenOpen::Ask,
        WhenOpen::Switch,
        WhenOpen::Tab,
        WhenOpen::Window,
    ] {
        assert_eq!(
            decide(&owner.open_now(), &req(Some("urn:a"), w)),
            Decision::Open(WhenOpen::Window)
        );
    }
    let a = id(owner.open(req(Some("urn:a"), WhenOpen::Ask)).unwrap());
    let now = owner.open_now();
    // the very same record: raised, no question
    for w in [WhenOpen::Ask, WhenOpen::Window] {
        assert_eq!(
            decide(&now, &req(Some("urn:a"), w)),
            Decision::Open(WhenOpen::Switch)
        );
    }
    // another record: the rule decides; Ask and (until the frame `dock` exists) Tab ask, with the open one named
    let ask = Decision::Ask { at: a };
    assert_eq!(decide(&now, &req(Some("urn:b"), WhenOpen::Ask)), ask);
    assert_eq!(decide(&now, &req(Some("urn:b"), WhenOpen::Tab)), ask);
    assert_eq!(
        decide(&now, &req(Some("urn:b"), WhenOpen::Switch)),
        Decision::Open(WhenOpen::Switch)
    );
    assert_eq!(
        decide(&now, &req(Some("urn:b"), WhenOpen::Window)),
        Decision::Open(WhenOpen::Window)
    );
    // no record against an open record is "another record" too
    assert!(matches!(
        decide(&now, &req(None, WhenOpen::Ask)),
        Decision::Ask { .. }
    ));
    assert_eq!(WhenOpen::default(), WhenOpen::Ask);
}

#[test]
fn w12_ask_calls_the_owner_never_the_counters_stand_until_an_answer() {
    let owner = fake();
    id(owner.open(req(Some("urn:a"), WhenOpen::Ask)).unwrap());
    let (holds, now) = (owner.holds(), owner.open_now());
    // the host decides: Ask. It does not call `open`.
    let d = decide(&owner.open_now(), &req(Some("urn:b"), WhenOpen::Ask));
    assert!(matches!(d, Decision::Ask { .. }));
    assert_eq!(
        (owner.holds(), owner.open_now()),
        (holds, now.clone()),
        "decide changes nothing"
    );
    // an owner that is called with the unanswered Ask refuses, with its code, and changes nothing
    let e = owner.open(req(Some("urn:b"), WhenOpen::Ask)).unwrap_err();
    assert_eq!(e.code, "host-notice-open-unanswered");
    assert_eq!((owner.holds(), owner.open_now()), (holds, now));
    // the answer arrives: now it opens
    let r = owner.open(req(Some("urn:b"), WhenOpen::Window)).unwrap();
    assert_eq!(
        r,
        WindowOpened::New(WindowId::of_record(
            "net.freesynergy.Tasks",
            "board",
            Some("urn:b")
        ))
    );
    assert_eq!(owner.holds(), (2, 0));
}
