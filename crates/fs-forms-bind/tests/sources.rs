//! H-K3: a missing, mismatching or unreachable source locks every placement and command that uses it, visibly.
mod common;
use common::*;
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn plan_with(offer: Offer) -> Plan {
    let facts = FakeFacts(vec![("tasks".into(), offer)]);
    build_with(&tasks_resolved(), &TestCatalog::new(), &facts, host())
}

fn check(plan: &Plan, cause: &str, params: &[(&str, &str)]) {
    for (page, place) in [("board", "main"), ("detail", "props")] {
        let l = locks_of(plan, page, place);
        assert_eq!(l.len(), 1, "{page}/{place}");
        assert_eq!(code_of(&l[0]), cause);
        for (k, v) in params {
            assert_eq!(l[0].cause.coded().param(k), Some(*v), "{k}");
        }
    }
    assert!(
        matches!(
            plan.place("detail", "empty").unwrap().status,
            PlaceStatus::Ready(_)
        ),
        "no source = drawn"
    );
    assert_eq!(plan.state, PlanState::Partial);
    let create = plan
        .menu
        .commands()
        .into_iter()
        .find(|c| c.name == "create")
        .unwrap();
    assert_eq!(
        command_state(create, None),
        Shown::Locked(create.fixed.clone().unwrap())
    );
    assert_eq!(
        code_of(create.fixed.as_ref().unwrap()),
        cause,
        "menu command has the same cause"
    );
}

#[test]
fn not_installed() {
    let p = plan_with(Offer::NotInstalled);
    check(
        &p,
        "accepts-error-contract-missing",
        &[
            ("capability", "tasks"),
            ("range", "tasks@>=0.2, <0.3"),
            ("have", ""),
        ],
    );
}

#[test]
fn version_out_of_range() {
    let p = plan_with(Offer::Reached(vec![provider(0, 3)]));
    check(&p, "accepts-error-contract-missing", &[("have", "0.3")]);
}

#[test]
fn unreachable() {
    let cause = BindCode::ProviderUnreachable
        .coded()
        .with("capability", "tasks");
    let p = plan_with(Offer::Unreachable { cause });
    check(
        &p,
        "bind-error-provider-unreachable",
        &[("capability", "tasks")],
    );
}

#[test]
fn highest_version_in_range_wins() {
    let p = plan_with(Offer::Reached(vec![
        provider(0, 1),
        provider(0, 2),
        provider(0, 3),
    ]));
    assert_eq!(p.sources[0].status.as_ref().unwrap().version, (0, 2));
    assert_eq!(p.state, PlanState::Ready);
}

#[test]
fn coded_lock_has_the_one_window_code() {
    let p = plan_with(Offer::NotInstalled);
    let c = locks_of(&p, "board", "main")[0].coded();
    assert_eq!(c.code, "bind-error-locked");
    assert_eq!(c.param("part"), Some("place:board/main"));
    assert_eq!(c.param("cause"), Some("accepts-error-contract-missing"));
    assert_eq!(c.param("cause-capability"), Some("tasks"));
}
